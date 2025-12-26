//! Convergence Acceptance Tests for Stage0
//!
//! These tests verify the convergence golden path behavior:
//! - Tier1 (DCC) excludes system:true memories by default
//! - Tier2 fail-closed semantics (skip gracefully when unavailable)
//! - System pointer memory storage is best-effort
//!
//! Uses mock implementations to avoid external dependencies.

use async_trait::async_trait;
use codex_stage0::{
    dcc::{EnvCtx, Iqo, LocalMemoryClient, LocalMemorySearchParams, LocalMemorySummary},
    guardians::{LlmClient, MemoryKind},
    tier2::{Tier2Client, Tier2Response},
    NoopVectorBackend, Stage0Config, Stage0Engine, Stage0Error,
};
use std::sync::atomic::{AtomicU32, Ordering};

// ─────────────────────────────────────────────────────────────────────────────
// Mock Implementations
// ─────────────────────────────────────────────────────────────────────────────

/// Mock local-memory client that returns configurable memories
struct MockLocalMemoryClient {
    memories: Vec<LocalMemorySummary>,
}

impl MockLocalMemoryClient {
    fn new(memories: Vec<LocalMemorySummary>) -> Self {
        Self { memories }
    }

    fn with_sample_memories() -> Self {
        Self::new(vec![
            LocalMemorySummary {
                id: "mem-001".to_string(),
                domain: Some("spec-kit".to_string()),
                tags: vec!["type:pattern".to_string()],
                created_at: Some(chrono::Utc::now()),
                snippet: "Sample pattern memory".to_string(),
                similarity_score: 0.9,
            },
            LocalMemorySummary {
                id: "mem-002".to_string(),
                domain: Some("spec-kit".to_string()),
                tags: vec!["type:decision".to_string()],
                created_at: Some(chrono::Utc::now()),
                snippet: "Sample decision memory".to_string(),
                similarity_score: 0.8,
            },
        ])
    }

    fn with_system_memories() -> Self {
        Self::new(vec![
            LocalMemorySummary {
                id: "mem-sys-001".to_string(),
                domain: Some("spec-tracker".to_string()),
                tags: vec!["system:true".to_string(), "stage:0".to_string()],
                created_at: Some(chrono::Utc::now()),
                snippet: "System pointer memory (should be excluded)".to_string(),
                similarity_score: 0.95,
            },
            LocalMemorySummary {
                id: "mem-normal-001".to_string(),
                domain: Some("spec-kit".to_string()),
                tags: vec!["type:pattern".to_string()],
                created_at: Some(chrono::Utc::now()),
                snippet: "Normal memory".to_string(),
                similarity_score: 0.85,
            },
        ])
    }
}

#[async_trait]
impl LocalMemoryClient for MockLocalMemoryClient {
    async fn search_memories(
        &self,
        params: LocalMemorySearchParams,
    ) -> codex_stage0::Result<Vec<LocalMemorySummary>> {
        // Filter by exclude_tags if provided
        let exclude_set: std::collections::HashSet<&str> = params
            .iqo
            .exclude_tags
            .iter()
            .map(String::as_str)
            .collect();

        let filtered: Vec<LocalMemorySummary> = self
            .memories
            .iter()
            .filter(|m| {
                // Skip memories that have any excluded tag
                !m.tags.iter().any(|t| exclude_set.contains(t.as_str()))
            })
            .take(params.max_results)
            .cloned()
            .collect();

        Ok(filtered)
    }
}

/// Mock LLM client
struct MockLlmClient;

#[async_trait]
impl LlmClient for MockLlmClient {
    async fn classify_kind(&self, _input: &str) -> codex_stage0::Result<MemoryKind> {
        Ok(MemoryKind::Other)
    }

    async fn restructure_template(
        &self,
        input: &str,
        _kind: MemoryKind,
    ) -> codex_stage0::Result<String> {
        Ok(input.to_string())
    }

    async fn generate_iqo(
        &self,
        _spec_content: &str,
        _env: &EnvCtx,
    ) -> codex_stage0::Result<Iqo> {
        Ok(Iqo {
            domains: vec!["spec-kit".to_string()],
            keywords: vec!["test".to_string()],
            max_candidates: 50,
            ..Default::default()
        })
    }
}

/// Mock Tier 2 client with configurable behavior
struct MockTier2Client {
    call_count: AtomicU32,
    should_fail: bool,
}

impl MockTier2Client {
    fn success() -> Self {
        Self {
            call_count: AtomicU32::new(0),
            should_fail: false,
        }
    }

    fn failing() -> Self {
        Self {
            call_count: AtomicU32::new(0),
            should_fail: true,
        }
    }

    fn get_call_count(&self) -> u32 {
        self.call_count.load(Ordering::SeqCst)
    }
}

/// Noop Tier2 client that always returns an error (simulates Tier2 not configured)
///
/// In real usage, the TUI layer checks if notebook is configured and passes
/// this noop client when not configured.
struct NoopTier2Client;

#[async_trait]
impl Tier2Client for NoopTier2Client {
    async fn generate_divine_truth(
        &self,
        _spec_id: &str,
        _spec_content: &str,
        _task_brief_md: &str,
    ) -> codex_stage0::Result<Tier2Response> {
        Err(Stage0Error::tier2("Tier2 not configured"))
    }
}

#[async_trait]
impl Tier2Client for MockTier2Client {
    async fn generate_divine_truth(
        &self,
        _spec_id: &str,
        _spec_content: &str,
        _task_brief_md: &str,
    ) -> codex_stage0::Result<Tier2Response> {
        self.call_count.fetch_add(1, Ordering::SeqCst);

        if self.should_fail {
            Err(Stage0Error::tier2("Mock Tier 2 failure"))
        } else {
            Ok(Tier2Response {
                divine_truth_md: r#"# Divine Truth Brief: SPEC-TEST

## 1. Executive Summary
- Test summary point 1
- Test summary point 2

## 2. Architectural Guardrails
- Follow existing patterns

## 3. Historical Context & Lessons
- Previous implementations worked well

## 4. Risks & Open Questions
- Low risk test case
"#
                .to_string(),
                suggested_links: vec![],
            })
        }
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Convergence Acceptance Tests
// ─────────────────────────────────────────────────────────────────────────────

/// Helper to create a test engine with temp database
#[allow(clippy::expect_used)]
fn create_test_engine() -> (Stage0Engine, tempfile::TempDir) {
    let temp = tempfile::tempdir().expect("tempdir");
    let cfg = Stage0Config {
        db_path: temp
            .path()
            .join("stage0-overlay.db")
            .to_string_lossy()
            .into_owned(),
        ..Default::default()
    };
    let engine = Stage0Engine::with_config(cfg).expect("create engine");
    (engine, temp)
}

/// Test: Tier1 excludes system:true memories by default
///
/// CONVERGENCE: Per MEMO_codex-rs.md Section 3, system pointer memories
/// (tagged with system:true) must be excluded from normal retrieval.
#[tokio::test]
async fn test_tier1_excludes_system_memories() {
    let (engine, _temp) = create_test_engine();
    let local_mem = MockLocalMemoryClient::with_system_memories();
    let llm = MockLlmClient;
    let noop_vector: Option<&NoopVectorBackend> = None;

    // Run DCC (compile_context)
    let result = engine
        .compile_context(
            &local_mem,
            &llm,
            noop_vector,
            "SPEC-TEST",
            "Test spec content",
            &EnvCtx::default(),
            false,
        )
        .await
        .expect("compile_context should succeed");

    // Verify: system:true memory was excluded
    assert!(
        !result.memories_used.contains(&"mem-sys-001".to_string()),
        "System memory should be excluded from retrieval"
    );

    // Verify: normal memory was included
    assert!(
        result.memories_used.contains(&"mem-normal-001".to_string()),
        "Normal memory should be included in retrieval"
    );
}

/// Test: Tier2 skipped when not configured (fail-closed)
///
/// CONVERGENCE: Per MEMO_codex-rs.md Section 1, when Tier2 is not configured
/// (no notebook mapping), Stage0 should skip Tier2 and continue with Tier1 only.
///
/// NOTE: The check for notebook configuration happens at the TUI layer
/// (stage0_integration.rs). When notebook is not configured, the TUI passes
/// a NoopTier2Client that always returns an error. This test simulates that behavior.
#[tokio::test]
async fn test_tier2_skipped_when_not_configured() {
    let (engine, _temp) = create_test_engine();
    let local_mem = MockLocalMemoryClient::with_sample_memories();
    let llm = MockLlmClient;
    // When notebook is not configured, TUI passes NoopTier2Client
    let tier2 = NoopTier2Client;
    let noop_vector: Option<&codex_stage0::NoopVectorBackend> = None;

    let result = engine
        .run_stage0(
            &local_mem,
            &llm,
            noop_vector,
            &tier2,
            "SPEC-TEST",
            "Test spec content",
            &EnvCtx::default(),
            false,
        )
        .await
        .expect("run_stage0 should succeed");

    // Verify: Stage0 completed with Tier1 results and fallback Divine Truth
    assert!(!result.tier2_used, "Tier2 should not be marked as used");
    assert!(result.divine_truth.is_fallback(), "Should use fallback");
    assert!(
        !result.task_brief_md.is_empty(),
        "Task brief should be generated"
    );
}

/// Test: Tier2 runs when properly configured
///
/// CONVERGENCE: When Tier2 is enabled AND has a notebook configured,
/// it should be called and return synthesized Divine Truth.
#[tokio::test]
async fn test_tier2_runs_when_configured() {
    let (engine, _temp) = create_test_engine();
    let local_mem = MockLocalMemoryClient::with_sample_memories();
    let llm = MockLlmClient;
    let tier2 = MockTier2Client::success();
    let noop_vector: Option<&NoopVectorBackend> = None;

    let result = engine
        .run_stage0(
            &local_mem,
            &llm,
            noop_vector,
            &tier2,
            "SPEC-TEST",
            "Test spec content",
            &EnvCtx::default(),
            false,
        )
        .await
        .expect("run_stage0 should succeed");

    // Verify: Tier2 was called
    assert_eq!(tier2.get_call_count(), 1, "Tier2 should be called once");

    // Verify: Result includes Tier2 content
    assert!(result.tier2_used, "Tier2 should be marked as used");
    assert!(
        !result.divine_truth.is_fallback(),
        "Should not be fallback"
    );
    assert!(
        result
            .divine_truth
            .raw_markdown
            .contains("Executive Summary"),
        "Divine Truth should contain synthesized content"
    );
}

/// Test: Tier2 failure is graceful (soft failure)
///
/// CONVERGENCE: If Tier2 fails at runtime, Stage0 should continue
/// with a fallback Divine Truth and not propagate the error.
#[tokio::test]
async fn test_tier2_failure_graceful() {
    let (engine, _temp) = create_test_engine();
    let local_mem = MockLocalMemoryClient::with_sample_memories();
    let llm = MockLlmClient;
    let tier2 = MockTier2Client::failing();
    let noop_vector: Option<&NoopVectorBackend> = None;

    let result = engine
        .run_stage0(
            &local_mem,
            &llm,
            noop_vector,
            &tier2,
            "SPEC-TEST",
            "Test spec content",
            &EnvCtx::default(),
            false,
        )
        .await
        .expect("run_stage0 should succeed despite Tier2 failure");

    // Verify: Tier2 was called but failed
    assert_eq!(tier2.get_call_count(), 1, "Tier2 should be called");

    // Verify: Stage0 completed with fallback
    assert!(!result.tier2_used, "Tier2 should not be marked as used");
    assert!(result.divine_truth.is_fallback(), "Should use fallback");
    assert!(
        !result.task_brief_md.is_empty(),
        "Task brief should still be generated"
    );
}

/// Test: Store system pointers config defaults to true
///
/// CONVERGENCE: The store_system_pointers config option should default
/// to true to enable pointer memory traceability.
#[test]
fn test_store_system_pointers_default() {
    let cfg = Stage0Config::default();
    assert!(
        cfg.store_system_pointers,
        "store_system_pointers should default to true"
    );
}

/// Test: Stage0 config loads with store_system_pointers option
#[test]
fn test_store_system_pointers_config_parsing() {
    let toml = r#"
        enabled = true
        store_system_pointers = false
    "#;

    let cfg = Stage0Config::parse(toml).expect("should parse");
    assert!(!cfg.store_system_pointers, "Should parse as false");

    let toml_default = r#"
        enabled = true
    "#;

    let cfg_default = Stage0Config::parse(toml_default).expect("should parse");
    assert!(cfg_default.store_system_pointers, "Should default to true");
}
