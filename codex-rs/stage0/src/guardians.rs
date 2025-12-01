//! Guardians for Stage0 memory ingestion
//!
//! Implements MetadataGuardian and TemplateGuardian per STAGE0_GUARDIANS_AND_ORCHESTRATION.md.
//! These guardians validate and structure memories before they are written to local-memory.

use crate::config::Stage0Config;
use crate::errors::{Result, Stage0Error};
use async_trait::async_trait;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

// ─────────────────────────────────────────────────────────────────────────────
// Core Types
// ─────────────────────────────────────────────────────────────────────────────

/// Classification of memory content
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub enum MemoryKind {
    /// Reusable solution or approach
    Pattern,
    /// Architectural or design choice with rationale
    Decision,
    /// Bug, issue, or failure mode
    Problem,
    /// Learning or observation
    Insight,
    /// Uncategorized or ambiguous content
    Other,
}

impl MemoryKind {
    /// Parse from string (case-insensitive)
    pub fn parse(s: &str) -> Self {
        match s.to_ascii_uppercase().as_str() {
            "PATTERN" => Self::Pattern,
            "DECISION" => Self::Decision,
            "PROBLEM" => Self::Problem,
            "INSIGHT" => Self::Insight,
            _ => Self::Other,
        }
    }

    /// Convert to a tag string for local-memory
    pub fn as_tag(&self) -> &'static str {
        match self {
            Self::Pattern => "type:pattern",
            Self::Decision => "type:decision",
            Self::Problem => "type:problem",
            Self::Insight => "type:insight",
            Self::Other => "type:other",
        }
    }

    /// Human-readable label for templates
    pub fn as_label(&self) -> &'static str {
        match self {
            Self::Pattern => "PATTERN",
            Self::Decision => "DECISION",
            Self::Problem => "PROBLEM",
            Self::Insight => "INSIGHT",
            Self::Other => "OTHER",
        }
    }
}

impl std::fmt::Display for MemoryKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_label())
    }
}

/// Input from codex-rs before guardian processing
#[derive(Debug, Clone, Default)]
pub struct MemoryDraft {
    /// Raw content to be structured
    pub raw_content: String,
    /// Tags to apply (may include agent: tags)
    pub tags: Vec<String>,
    /// When the memory was created (optional, validated by MetadataGuardian)
    pub created_at: Option<DateTime<Utc>>,
    /// Agent type tag (e.g., "agent:llm_claude", "agent:human")
    pub agent_type_tag: Option<String>,
    /// Initial priority override (1-10)
    pub initial_priority: Option<i32>,
}

impl MemoryDraft {
    /// Create a new draft with raw content
    pub fn new(raw_content: impl Into<String>) -> Self {
        Self {
            raw_content: raw_content.into(),
            ..Default::default()
        }
    }

    /// Builder: set tags
    pub fn with_tags(mut self, tags: Vec<String>) -> Self {
        self.tags = tags;
        self
    }

    /// Builder: set created_at
    pub fn with_created_at(mut self, ts: DateTime<Utc>) -> Self {
        self.created_at = Some(ts);
        self
    }

    /// Builder: set agent type
    pub fn with_agent_type(mut self, agent: impl Into<String>) -> Self {
        self.agent_type_tag = Some(agent.into());
        self
    }

    /// Builder: set priority
    pub fn with_priority(mut self, priority: i32) -> Self {
        self.initial_priority = Some(priority);
        self
    }
}

/// Output after guardian processing, ready for local-memory
#[derive(Debug, Clone)]
pub struct GuardedMemory {
    /// Structured content (template-formatted by TemplateGuardian)
    pub content_structured: String,
    /// Original raw content (preserved for overlay DB)
    pub content_raw: String,
    /// Classified memory kind
    pub kind: MemoryKind,
    /// Validated/normalized creation timestamp
    pub created_at: DateTime<Utc>,
    /// Agent type tag
    pub agent_type_tag: String,
    /// Initial priority (1-10)
    pub initial_priority: i32,
    /// Original tags from draft (with kind tag added)
    pub tags: Vec<String>,
}

impl GuardedMemory {
    /// Get all tags including the kind tag
    pub fn all_tags(&self) -> Vec<String> {
        let mut tags = self.tags.clone();
        let kind_tag = self.kind.as_tag().to_string();
        if !tags.contains(&kind_tag) {
            tags.push(kind_tag);
        }
        if !tags.contains(&self.agent_type_tag) {
            tags.push(self.agent_type_tag.clone());
        }
        tags
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// LLM Client Trait
// ─────────────────────────────────────────────────────────────────────────────

/// Trait for LLM operations used by TemplateGuardian
///
/// Implementations should be provided by codex-rs using codex-ollama or similar.
/// Stage0 only depends on this trait, not on specific LLM crates.
#[async_trait]
pub trait LlmClient: Send + Sync {
    /// Classify content into a MemoryKind
    ///
    /// Should return `MemoryKind::Other` on ambiguous input.
    async fn classify_kind(&self, input: &str) -> Result<MemoryKind>;

    /// Restructure content into the standard template format
    ///
    /// The template format is defined in STAGE0_CONFIG_AND_PROMPTS.md:
    /// ```text
    /// [KIND]: <One-line Summary>
    ///
    /// CONTEXT: <situation/trigger>
    ///
    /// REASONING: <WHY, alternatives considered>
    ///
    /// OUTCOME: <result/impact>
    /// ```
    async fn restructure_template(&self, input: &str, kind: MemoryKind) -> Result<String>;
}

// ─────────────────────────────────────────────────────────────────────────────
// Metadata Guardian
// ─────────────────────────────────────────────────────────────────────────────

/// Apply metadata validation and normalization to a memory draft
///
/// Validates:
/// - `created_at`: required if strict_metadata, defaults to `now` otherwise
/// - `agent_type_tag`: required if strict_metadata, inferred from tags otherwise
/// - `initial_priority`: clamped to 1-10, defaults to 7
///
/// Returns a partially-filled `GuardedMemory` (kind and content_structured
/// will be set by TemplateGuardian).
pub fn apply_metadata_guardian(
    cfg: &Stage0Config,
    draft: &MemoryDraft,
    now: DateTime<Utc>,
) -> Result<GuardedMemory> {
    let strict = cfg.ingestion.strict_metadata;

    // 1. Validate/default created_at
    let created_at = match draft.created_at {
        Some(ts) => ts,
        None => {
            if strict {
                return Err(Stage0Error::config(
                    "missing created_at for memory draft (strict_metadata=true)",
                ));
            }
            tracing::warn!("Memory draft missing created_at, defaulting to now");
            now
        }
    };

    // 2. Validate/infer agent_type_tag
    let agent_type_tag = match &draft.agent_type_tag {
        Some(tag) if !tag.is_empty() => tag.clone(),
        _ => {
            if strict {
                return Err(Stage0Error::config(
                    "missing agent_type_tag for memory draft (strict_metadata=true)",
                ));
            }
            // Try to infer from tags
            draft
                .tags
                .iter()
                .find(|t| t.starts_with("agent:"))
                .cloned()
                .unwrap_or_else(|| {
                    tracing::warn!(
                        "Memory draft missing agent_type_tag, defaulting to agent:unknown"
                    );
                    "agent:unknown".to_string()
                })
        }
    };

    // 3. Normalize initial_priority (clamp to 1-10, default 7)
    let priority = draft.initial_priority.unwrap_or(7).clamp(1, 10);

    Ok(GuardedMemory {
        content_structured: draft.raw_content.clone(), // Will be overwritten by TemplateGuardian
        content_raw: draft.raw_content.clone(),
        kind: MemoryKind::Other, // Will be set by TemplateGuardian
        created_at,
        agent_type_tag,
        initial_priority: priority,
        tags: draft.tags.clone(),
    })
}

// ─────────────────────────────────────────────────────────────────────────────
// Template Guardian
// ─────────────────────────────────────────────────────────────────────────────

/// Apply template restructuring to a guarded memory using an LLM
///
/// 1. Classifies the content into a MemoryKind
/// 2. Restructures the content into the standard template
/// 3. Performs basic sanity checks on the output
///
/// Falls back to OTHER and preserves raw content on LLM errors.
pub async fn apply_template_guardian<L: LlmClient>(
    llm: &L,
    mut guarded: GuardedMemory,
) -> Result<GuardedMemory> {
    // 1. Classify kind (fallback to Other on error)
    let kind = match llm.classify_kind(&guarded.content_raw).await {
        Ok(k) => k,
        Err(e) => {
            tracing::warn!(error = %e, "LLM classification failed, defaulting to OTHER");
            MemoryKind::Other
        }
    };

    // 2. Restructure content according to template
    let structured = match llm.restructure_template(&guarded.content_raw, kind).await {
        Ok(s) => s,
        Err(e) => {
            tracing::warn!(error = %e, "LLM restructuring failed, using raw content");
            // Fallback: create a minimal template with raw content
            format!(
                "[{kind}]: Memory (restructuring failed)\n\n\
                 CONTEXT: (automatic fallback)\n\n\
                 REASONING: LLM restructuring unavailable\n\n\
                 OUTCOME: Raw content preserved below\n\n\
                 ---\n{raw}",
                kind = kind.as_label(),
                raw = guarded.content_raw
            )
        }
    };

    // 3. Basic sanity check: warn if template seems malformed
    let first_line = structured.lines().next().unwrap_or("");
    if !first_line.starts_with('[') {
        tracing::warn!(
            first_line = first_line,
            "Template output may be malformed (expected [KIND]: ...)"
        );
    }

    guarded.kind = kind;
    guarded.content_structured = structured;

    Ok(guarded)
}

/// Skip LLM processing and use raw content as-is
///
/// Useful when LLM is unavailable or for testing.
/// Creates a minimal template wrapper around the raw content.
pub fn apply_template_guardian_passthrough(mut guarded: GuardedMemory) -> GuardedMemory {
    let kind = MemoryKind::Other;
    let structured = format!(
        "[{kind}]: Unstructured memory\n\n\
         CONTEXT: (no LLM processing)\n\n\
         REASONING: Content preserved as-is\n\n\
         OUTCOME: See raw content below\n\n\
         ---\n{raw}",
        kind = kind.as_label(),
        raw = guarded.content_raw
    );

    guarded.kind = kind;
    guarded.content_structured = structured;
    guarded
}

// ─────────────────────────────────────────────────────────────────────────────
// Tests
// ─────────────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::{IngestionConfig, Stage0Config};

    /// Mock LLM client for testing
    pub struct MockLlmClient {
        /// Kind to return for classify_kind
        pub classify_result: Result<MemoryKind>,
        /// Content to return for restructure_template
        pub restructure_result: Result<String>,
    }

    impl MockLlmClient {
        pub fn success(kind: MemoryKind, structured: impl Into<String>) -> Self {
            Self {
                classify_result: Ok(kind),
                restructure_result: Ok(structured.into()),
            }
        }

        pub fn classify_fails() -> Self {
            Self {
                classify_result: Err(Stage0Error::prompt("mock classify error")),
                restructure_result: Ok("[OTHER]: Fallback".to_string()),
            }
        }

        pub fn restructure_fails() -> Self {
            Self {
                classify_result: Ok(MemoryKind::Decision),
                restructure_result: Err(Stage0Error::prompt("mock restructure error")),
            }
        }
    }

    #[async_trait]
    impl LlmClient for MockLlmClient {
        async fn classify_kind(&self, _input: &str) -> Result<MemoryKind> {
            self.classify_result.clone()
        }

        async fn restructure_template(&self, _input: &str, kind: MemoryKind) -> Result<String> {
            match &self.restructure_result {
                Ok(s) => {
                    // Replace placeholder kind with actual
                    Ok(s.replace("[KIND]", &format!("[{}]", kind.as_label())))
                }
                Err(e) => Err(Stage0Error::prompt(e.to_string())),
            }
        }
    }

    fn strict_config() -> Stage0Config {
        let mut cfg = Stage0Config::default();
        cfg.ingestion = IngestionConfig {
            strict_metadata: true,
        };
        cfg
    }

    fn lenient_config() -> Stage0Config {
        let mut cfg = Stage0Config::default();
        cfg.ingestion = IngestionConfig {
            strict_metadata: false,
        };
        cfg
    }

    // ─────────────────────────────────────────────────────────────────────────
    // MemoryKind tests
    // ─────────────────────────────────────────────────────────────────────────

    #[test]
    fn test_memory_kind_parse() {
        assert_eq!(MemoryKind::parse("pattern"), MemoryKind::Pattern);
        assert_eq!(MemoryKind::parse("PATTERN"), MemoryKind::Pattern);
        assert_eq!(MemoryKind::parse("Pattern"), MemoryKind::Pattern);
        assert_eq!(MemoryKind::parse("DECISION"), MemoryKind::Decision);
        assert_eq!(MemoryKind::parse("problem"), MemoryKind::Problem);
        assert_eq!(MemoryKind::parse("insight"), MemoryKind::Insight);
        assert_eq!(MemoryKind::parse("other"), MemoryKind::Other);
        assert_eq!(MemoryKind::parse("unknown"), MemoryKind::Other);
        assert_eq!(MemoryKind::parse(""), MemoryKind::Other);
    }

    #[test]
    fn test_memory_kind_as_tag() {
        assert_eq!(MemoryKind::Pattern.as_tag(), "type:pattern");
        assert_eq!(MemoryKind::Decision.as_tag(), "type:decision");
        assert_eq!(MemoryKind::Problem.as_tag(), "type:problem");
        assert_eq!(MemoryKind::Insight.as_tag(), "type:insight");
        assert_eq!(MemoryKind::Other.as_tag(), "type:other");
    }

    // ─────────────────────────────────────────────────────────────────────────
    // MetadataGuardian tests
    // ─────────────────────────────────────────────────────────────────────────

    #[test]
    fn test_metadata_guardian_strict_missing_created_at_fails() {
        let cfg = strict_config();
        let draft = MemoryDraft::new("test content").with_agent_type("agent:test");
        let now = Utc::now();

        let result = apply_metadata_guardian(&cfg, &draft, now);
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(err.to_string().contains("created_at"));
    }

    #[test]
    fn test_metadata_guardian_strict_missing_agent_type_fails() {
        let cfg = strict_config();
        let draft = MemoryDraft::new("test content").with_created_at(Utc::now());
        let now = Utc::now();

        let result = apply_metadata_guardian(&cfg, &draft, now);
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(err.to_string().contains("agent_type"));
    }

    #[test]
    fn test_metadata_guardian_strict_success() {
        let cfg = strict_config();
        let ts = Utc::now();
        let draft = MemoryDraft::new("test content")
            .with_created_at(ts)
            .with_agent_type("agent:claude")
            .with_priority(8);
        let now = Utc::now();

        let result = apply_metadata_guardian(&cfg, &draft, now);
        assert!(result.is_ok());
        let guarded = result.unwrap();
        assert_eq!(guarded.created_at, ts);
        assert_eq!(guarded.agent_type_tag, "agent:claude");
        assert_eq!(guarded.initial_priority, 8);
    }

    #[test]
    fn test_metadata_guardian_lenient_defaults_created_at() {
        let cfg = lenient_config();
        let draft = MemoryDraft::new("test content").with_agent_type("agent:test");
        let now = Utc::now();

        let result = apply_metadata_guardian(&cfg, &draft, now);
        assert!(result.is_ok());
        let guarded = result.unwrap();
        assert_eq!(guarded.created_at, now);
    }

    #[test]
    fn test_metadata_guardian_lenient_infers_agent_from_tags() {
        let cfg = lenient_config();
        let draft = MemoryDraft::new("test content")
            .with_created_at(Utc::now())
            .with_tags(vec!["project:foo".to_string(), "agent:human".to_string()]);
        let now = Utc::now();

        let result = apply_metadata_guardian(&cfg, &draft, now);
        assert!(result.is_ok());
        let guarded = result.unwrap();
        assert_eq!(guarded.agent_type_tag, "agent:human");
    }

    #[test]
    fn test_metadata_guardian_lenient_defaults_agent_to_unknown() {
        let cfg = lenient_config();
        let draft = MemoryDraft::new("test content").with_created_at(Utc::now());
        let now = Utc::now();

        let result = apply_metadata_guardian(&cfg, &draft, now);
        assert!(result.is_ok());
        let guarded = result.unwrap();
        assert_eq!(guarded.agent_type_tag, "agent:unknown");
    }

    #[test]
    fn test_metadata_guardian_clamps_priority() {
        let cfg = lenient_config();

        // Test priority too low
        let draft = MemoryDraft::new("test")
            .with_created_at(Utc::now())
            .with_agent_type("agent:test")
            .with_priority(-5);
        let guarded = apply_metadata_guardian(&cfg, &draft, Utc::now()).unwrap();
        assert_eq!(guarded.initial_priority, 1);

        // Test priority too high
        let draft = MemoryDraft::new("test")
            .with_created_at(Utc::now())
            .with_agent_type("agent:test")
            .with_priority(99);
        let guarded = apply_metadata_guardian(&cfg, &draft, Utc::now()).unwrap();
        assert_eq!(guarded.initial_priority, 10);

        // Test default priority
        let draft = MemoryDraft::new("test")
            .with_created_at(Utc::now())
            .with_agent_type("agent:test");
        let guarded = apply_metadata_guardian(&cfg, &draft, Utc::now()).unwrap();
        assert_eq!(guarded.initial_priority, 7);
    }

    // ─────────────────────────────────────────────────────────────────────────
    // TemplateGuardian tests
    // ─────────────────────────────────────────────────────────────────────────

    #[tokio::test]
    async fn test_template_guardian_success() {
        let cfg = lenient_config();
        let draft = MemoryDraft::new("We decided to use SQLite for the overlay DB")
            .with_created_at(Utc::now())
            .with_agent_type("agent:claude");

        let guarded = apply_metadata_guardian(&cfg, &draft, Utc::now()).unwrap();

        let llm = MockLlmClient::success(
            MemoryKind::Decision,
            "[KIND]: Use SQLite for overlay DB\n\nCONTEXT: Need persistent storage\n\nREASONING: Simple, embedded\n\nOUTCOME: Fast development",
        );

        let result = apply_template_guardian(&llm, guarded).await;
        assert!(result.is_ok());
        let guarded = result.unwrap();
        assert_eq!(guarded.kind, MemoryKind::Decision);
        assert!(guarded.content_structured.contains("[DECISION]"));
        assert!(guarded.content_structured.contains("CONTEXT:"));
    }

    #[tokio::test]
    async fn test_template_guardian_classify_fails_defaults_to_other() {
        let cfg = lenient_config();
        let draft = MemoryDraft::new("some content")
            .with_created_at(Utc::now())
            .with_agent_type("agent:test");

        let guarded = apply_metadata_guardian(&cfg, &draft, Utc::now()).unwrap();
        let llm = MockLlmClient::classify_fails();

        let result = apply_template_guardian(&llm, guarded).await;
        assert!(result.is_ok());
        let guarded = result.unwrap();
        assert_eq!(guarded.kind, MemoryKind::Other);
    }

    #[tokio::test]
    async fn test_template_guardian_restructure_fails_creates_fallback() {
        let cfg = lenient_config();
        let draft = MemoryDraft::new("original raw content")
            .with_created_at(Utc::now())
            .with_agent_type("agent:test");

        let guarded = apply_metadata_guardian(&cfg, &draft, Utc::now()).unwrap();
        let llm = MockLlmClient::restructure_fails();

        let result = apply_template_guardian(&llm, guarded).await;
        assert!(result.is_ok());
        let guarded = result.unwrap();
        assert_eq!(guarded.kind, MemoryKind::Decision);
        assert!(guarded.content_structured.contains("original raw content"));
        assert!(guarded.content_structured.contains("restructuring failed"));
    }

    #[test]
    fn test_template_guardian_passthrough() {
        let cfg = lenient_config();
        let draft = MemoryDraft::new("raw content here")
            .with_created_at(Utc::now())
            .with_agent_type("agent:test");

        let guarded = apply_metadata_guardian(&cfg, &draft, Utc::now()).unwrap();
        let guarded = apply_template_guardian_passthrough(guarded);

        assert_eq!(guarded.kind, MemoryKind::Other);
        assert!(guarded.content_structured.contains("[OTHER]"));
        assert!(guarded.content_structured.contains("raw content here"));
    }

    // ─────────────────────────────────────────────────────────────────────────
    // Integration tests
    // ─────────────────────────────────────────────────────────────────────────

    #[tokio::test]
    async fn test_full_guardian_pipeline() {
        let cfg = lenient_config();
        let draft = MemoryDraft::new("Found a bug: the cache TTL wasn't being respected")
            .with_tags(vec!["spec:SPEC-KIT-102".to_string()])
            .with_priority(9);

        let now = Utc::now();
        let guarded = apply_metadata_guardian(&cfg, &draft, now).unwrap();

        assert_eq!(guarded.created_at, now);
        assert_eq!(guarded.agent_type_tag, "agent:unknown");
        assert_eq!(guarded.initial_priority, 9);

        let llm = MockLlmClient::success(
            MemoryKind::Problem,
            "[KIND]: Cache TTL not respected\n\nCONTEXT: Testing Tier2 cache\n\nREASONING: TTL check missing\n\nOUTCOME: Fixed in PR #123",
        );

        let guarded = apply_template_guardian(&llm, guarded).await.unwrap();

        assert_eq!(guarded.kind, MemoryKind::Problem);
        assert!(guarded.content_structured.contains("[PROBLEM]"));

        let all_tags = guarded.all_tags();
        assert!(all_tags.contains(&"type:problem".to_string()));
        assert!(all_tags.contains(&"spec:SPEC-KIT-102".to_string()));
        assert!(all_tags.contains(&"agent:unknown".to_string()));
    }

    #[test]
    fn test_guarded_memory_all_tags() {
        let guarded = GuardedMemory {
            content_structured: "test".to_string(),
            content_raw: "test".to_string(),
            kind: MemoryKind::Pattern,
            created_at: Utc::now(),
            agent_type_tag: "agent:claude".to_string(),
            initial_priority: 5,
            tags: vec!["project:foo".to_string(), "domain:rust".to_string()],
        };

        let all_tags = guarded.all_tags();
        assert!(all_tags.contains(&"type:pattern".to_string()));
        assert!(all_tags.contains(&"agent:claude".to_string()));
        assert!(all_tags.contains(&"project:foo".to_string()));
        assert!(all_tags.contains(&"domain:rust".to_string()));
    }
}
