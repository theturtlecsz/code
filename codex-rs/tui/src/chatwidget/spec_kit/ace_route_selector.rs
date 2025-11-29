//! ACE route selection logic for spec-kit commands
//!
//! Decides whether to use direct LLM or route through ace.generate
//! based on task complexity, rerun detection, and other heuristics.
//!
//! Note: This module is planned infrastructure for intelligent routing
//! decisions. Integration pending ACE framework completion.

#![allow(dead_code)] // Planned infrastructure, integration pending

use blake3::Hasher;
use codex_core::config_types::AceConfig;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::Path;
use std::sync::{Mutex, OnceLock};
use std::time::{SystemTime, UNIX_EPOCH};
use tracing::{debug, warn};

/// Task signature for rerun detection
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct TaskSignature(String);

impl TaskSignature {
    /// Create a new task signature from components
    ///
    /// Format: blake3("<command>|<branch>|<spec_or_title>|<sorted_files>")
    pub fn new(command: &str, branch: &str, spec_or_title: &str, primary_files: &[String]) -> Self {
        let mut hasher = Hasher::new();

        // Canonicalize components
        hasher.update(command.trim().to_lowercase().as_bytes());
        hasher.update(b"|");
        hasher.update(branch.trim().as_bytes());
        hasher.update(b"|");
        hasher.update(canonicalize_text(spec_or_title).as_bytes());
        hasher.update(b"|");

        // Sort and canonicalize files
        let mut sorted_files = primary_files.to_vec();
        sorted_files.sort();
        for (i, file) in sorted_files.iter().enumerate() {
            if i > 0 {
                hasher.update(b",");
            }
            hasher.update(canonicalize_path(file).as_bytes());
        }

        let hash = hasher.finalize();
        Self(hash.to_hex().to_string())
    }

    /// Get the hash string
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

/// Canonicalize text for stable hashing
fn canonicalize_text(text: &str) -> String {
    text.trim()
        .to_lowercase()
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
}

/// Canonicalize path for stable hashing
fn canonicalize_path(path: &str) -> String {
    Path::new(path)
        .components()
        .map(|c| c.as_os_str().to_string_lossy().to_string())
        .collect::<Vec<_>>()
        .join("/")
        .to_lowercase()
}

/// Metadata about a previous task run
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskRunMetadata {
    pub signature: String,
    pub timestamp: u64,
    pub command: String,
    pub branch: String,
    pub success: Option<bool>,
    pub files_changed: usize,
}

/// In-memory cache of recent task runs
static TASK_RUN_CACHE: OnceLock<Mutex<HashMap<String, TaskRunMetadata>>> = OnceLock::new();

/// Get or initialize the task run cache
fn get_cache() -> &'static Mutex<HashMap<String, TaskRunMetadata>> {
    TASK_RUN_CACHE.get_or_init(|| Mutex::new(HashMap::new()))
}

/// Check if this is a rerun of a recent task
pub fn is_rerun(signature: &TaskSignature, branch: &str, rerun_window_minutes: u64) -> bool {
    let cache = get_cache();
    let Ok(guard) = cache.lock() else {
        warn!("Failed to lock task run cache");
        return false;
    };

    let cache_key = format!("{}:{}", branch, signature.as_str());

    if let Some(metadata) = guard.get(&cache_key) {
        let now = current_timestamp();
        let window_seconds = rerun_window_minutes * 60;
        let elapsed = now.saturating_sub(metadata.timestamp);

        if elapsed <= window_seconds {
            debug!(
                "Rerun detected: {} (elapsed: {}s, window: {}s)",
                signature.as_str(),
                elapsed,
                window_seconds
            );
            return true;
        }
    }

    false
}

/// Record a task run for rerun detection
pub fn record_task_run(
    signature: TaskSignature,
    command: String,
    branch: String,
    success: Option<bool>,
    files_changed: usize,
) {
    let cache = get_cache();
    let Ok(mut guard) = cache.lock() else {
        warn!("Failed to lock task run cache");
        return;
    };

    let cache_key = format!("{}:{}", branch, signature.as_str());
    let metadata = TaskRunMetadata {
        signature: signature.0.clone(),
        timestamp: current_timestamp(),
        command,
        branch,
        success,
        files_changed,
    };

    guard.insert(cache_key, metadata);

    // Cleanup old entries (keep last 100)
    if guard.len() > 100 {
        let mut entries: Vec<_> = guard
            .iter()
            .map(|(k, v)| (k.clone(), v.timestamp))
            .collect();
        entries.sort_by_key(|(_, ts)| *ts);

        // Remove oldest 20
        for (key, _) in entries.iter().take(20) {
            guard.remove(key);
        }
    }

    debug!(
        "Recorded task run: {} (success: {:?}, files: {})",
        signature.as_str(),
        success,
        files_changed
    );
}

/// Get current Unix timestamp
fn current_timestamp() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
}

/// Check if task title contains cross-cutting keywords
pub fn has_cross_cutting_keywords(title: &str) -> bool {
    let lower = title.to_lowercase();
    let keywords = [
        "refactor",
        "migrate",
        "rename",
        "cross-cutting",
        "cross cutting",
        "monorepo",
        "multi-module",
        "multi module",
        "restructure",
    ];

    keywords.iter().any(|&kw| lower.contains(kw))
}

/// Diff statistics for complexity assessment
#[derive(Debug, Clone, Default)]
pub struct DiffStat {
    pub files_changed: usize,
    pub insertions: usize,
    pub deletions: usize,
}

impl DiffStat {
    pub fn new(files_changed: usize, insertions: usize, deletions: usize) -> Self {
        Self {
            files_changed,
            insertions,
            deletions,
        }
    }
}

/// Route selection decision
///
/// NOTE: ACE is data-only (SQLite storage). This decision determines whether to
/// increase ACE context (more bullets, enhanced prompts), NOT whether to call
/// an LLM through ACE (ACE doesn't call LLMs).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RouteDecision {
    /// Use standard prompt with normal ACE context
    Direct,
    /// Use enhanced ACE context (more bullets, detailed heuristics)
    EnhancedContext { reason: String },
}

impl RouteDecision {
    pub fn should_use_enhanced_context(&self) -> bool {
        matches!(self, RouteDecision::EnhancedContext { .. })
    }

    /// Alias for should_use_enhanced_context (for backward compatibility with tests)
    pub fn should_use_ace(&self) -> bool {
        self.should_use_enhanced_context()
    }

    pub fn reason(&self) -> Option<&str> {
        match self {
            RouteDecision::EnhancedContext { reason } => Some(reason),
            RouteDecision::Direct => None,
        }
    }
}

/// Aggregator effort levels understood by the Codex CLI
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AggregatorEffort {
    Minimal,
    Low,
    Medium,
    High,
}

impl AggregatorEffort {
    pub fn as_str(&self) -> &'static str {
        match self {
            AggregatorEffort::Minimal => "minimal",
            AggregatorEffort::Low => "low",
            AggregatorEffort::Medium => "medium",
            AggregatorEffort::High => "high",
        }
    }
}

/// Routing decision for a spec stage (model/effort policy)
#[derive(Debug, Clone)]
pub struct StageRoutingDecision {
    /// Aggregator effort to apply (Codex flag `-c model_reasoning_effort=…`)
    pub aggregator_effort: AggregatorEffort,
    /// Optional reason for escalation (stored in evidence)
    pub escalation_reason: Option<String>,
}

/// Compute stage routing defaults and escalation based on simple signals.
///
/// Signals:
/// - `stage`: current stage in the pipeline
/// - `prompt_char_len`: approximate prompt size (proxy for input tokens)
/// - `prior_conflict_retry`: true when we are retrying due to prior conflict
pub fn decide_stage_routing(
    stage: crate::spec_prompts::SpecStage,
    prompt_char_len: usize,
    prior_conflict_retry: bool,
) -> StageRoutingDecision {
    // Baseline per stage
    let mut effort = match stage {
        // SPEC-KIT-957: Specify uses Tier 1 (single agent, minimal effort)
        crate::spec_prompts::SpecStage::Specify => AggregatorEffort::Minimal,
        crate::spec_prompts::SpecStage::Validate | crate::spec_prompts::SpecStage::Unlock => {
            AggregatorEffort::Minimal
        }
        crate::spec_prompts::SpecStage::Plan
        | crate::spec_prompts::SpecStage::Tasks
        | crate::spec_prompts::SpecStage::Audit
        | crate::spec_prompts::SpecStage::Implement => AggregatorEffort::Medium,
        // Quality commands use minimal aggregation (no complex synthesis needed)
        crate::spec_prompts::SpecStage::Clarify
        | crate::spec_prompts::SpecStage::Analyze
        | crate::spec_prompts::SpecStage::Checklist => AggregatorEffort::Minimal,
    };

    let mut reason: Option<String> = None;

    // Escalate on conflict retry
    if prior_conflict_retry {
        effort = AggregatorEffort::High;
        reason = Some("retry_after_conflict".to_string());
    }

    // Size-aware hint (we still keep cheap contributors; aggregator can stay baseline)
    // Rough 4 chars/token heuristic; ~6k tokens ≈ 24k chars.
    if prompt_char_len >= 24_000 {
        // Only annotate reason if we didn't already escalate for conflicts.
        if reason.is_none() {
            reason = Some("large_input_context".to_string());
        }
    }

    StageRoutingDecision {
        aggregator_effort: effort,
        escalation_reason: reason,
    }
}

/// Decide whether to use enhanced ACE context based on complexity heuristics
///
/// NOTE: This does NOT route to an ACE LLM endpoint. ACE is data-only storage.
/// Enhanced context means: more bullets, increased slice_size, richer heuristics.
/// The CODE orchestrator still calls the LLM using the client's API keys.
pub fn select_route(
    config: &AceConfig,
    command: &str,
    branch: &str,
    spec_or_title: &str,
    primary_files: &[String],
    diff_stat: Option<&DiffStat>,
    prior_failure: bool,
) -> RouteDecision {
    // Only applies to implement command
    if !command.contains("implement") {
        debug!("Route selection: Direct (not an implement command)");
        return RouteDecision::Direct;
    }

    // Check if ACE is enabled and mode allows usage
    if !config.enabled {
        debug!("Route selection: Direct (ACE disabled)");
        return RouteDecision::Direct;
    }

    // Check mode setting
    match config.mode {
        codex_core::config_types::AceMode::Never => {
            debug!("Route selection: Direct (mode=never)");
            return RouteDecision::Direct;
        }
        _ => {} // Auto and Always modes proceed to heuristics
    }

    // Generate task signature
    let signature = TaskSignature::new(command, branch, spec_or_title, primary_files);

    // Heuristic 1: Rerun detection
    if is_rerun(&signature, branch, config.rerun_window_minutes) {
        return RouteDecision::EnhancedContext {
            reason: "rerun detected within window".to_string(),
        };
    }

    // Heuristic 2: Prior failure
    if prior_failure {
        return RouteDecision::EnhancedContext {
            reason: "prior attempt failed".to_string(),
        };
    }

    // Heuristic 3: High file count (complexity)
    if let Some(stat) = diff_stat
        && stat.files_changed > config.complex_task_files_threshold
    {
        return RouteDecision::EnhancedContext {
            reason: format!(
                "high file count ({} > {})",
                stat.files_changed, config.complex_task_files_threshold
            ),
        };
    }

    // Heuristic 4: Cross-cutting keywords
    if has_cross_cutting_keywords(spec_or_title) {
        return RouteDecision::EnhancedContext {
            reason: "cross-cutting keywords detected".to_string(),
        };
    }

    // Default to direct
    debug!("Route selection: Direct (no triggers matched)");
    RouteDecision::Direct
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_task_signature_deterministic() {
        let sig1 = TaskSignature::new(
            "speckit.implement",
            "main",
            "Add user auth",
            &["src/auth.rs".to_string(), "src/lib.rs".to_string()],
        );

        let sig2 = TaskSignature::new(
            "speckit.implement",
            "main",
            "Add user auth",
            &["src/lib.rs".to_string(), "src/auth.rs".to_string()], // Different order
        );

        // Should be identical (files sorted)
        assert_eq!(sig1, sig2);
    }

    #[test]
    fn test_task_signature_case_insensitive() {
        let sig1 = TaskSignature::new(
            "speckit.implement",
            "main",
            "Add User Auth",
            &["src/Auth.rs".to_string()],
        );

        let sig2 = TaskSignature::new(
            "SPECKIT.IMPLEMENT",
            "main",
            "add user auth",
            &["src/auth.rs".to_string()],
        );

        // Should be identical (normalized)
        assert_eq!(sig1, sig2);
    }

    #[test]
    fn test_task_signature_different() {
        let sig1 = TaskSignature::new(
            "speckit.implement",
            "main",
            "Add user auth",
            &["src/auth.rs".to_string()],
        );

        let sig2 = TaskSignature::new(
            "speckit.implement",
            "main",
            "Add user auth",
            &["src/auth.rs".to_string(), "src/db.rs".to_string()], // Different files
        );

        assert_ne!(sig1, sig2);
    }

    #[test]
    fn test_canonicalize_text() {
        assert_eq!(canonicalize_text("  Add  User  Auth  "), "add user auth");
        assert_eq!(canonicalize_text("Add\nUser\tAuth"), "add user auth");
    }

    #[test]
    fn test_has_cross_cutting_keywords() {
        assert!(has_cross_cutting_keywords("Refactor authentication module"));
        assert!(has_cross_cutting_keywords("Migrate to new API"));
        assert!(has_cross_cutting_keywords(
            "Rename variables across codebase"
        ));
        assert!(has_cross_cutting_keywords("Cross-cutting concern"));
        assert!(has_cross_cutting_keywords("Monorepo restructure"));
        assert!(!has_cross_cutting_keywords("Add new feature"));
        assert!(!has_cross_cutting_keywords("Fix bug in login"));
    }

    #[test]
    fn test_rerun_detection() {
        let sig = TaskSignature::new(
            "speckit.implement",
            "main",
            "Test task",
            &["src/test.rs".to_string()],
        );

        // Record a task
        record_task_run(
            sig.clone(),
            "speckit.implement".to_string(),
            "main".to_string(),
            Some(true),
            1,
        );

        // Should detect rerun immediately
        assert!(is_rerun(&sig, "main", 30));

        // Different branch should not match
        assert!(!is_rerun(&sig, "feature", 30));
    }

    #[test]
    fn test_route_selection_rerun() {
        let config = AceConfig {
            enabled: true,
            mode: codex_core::config_types::AceMode::Auto,
            slice_size: 8,
            db_path: "test.db".to_string(),
            use_for: vec![],
            complex_task_files_threshold: 4,
            rerun_window_minutes: 30,
        };

        let sig = TaskSignature::new(
            "speckit.implement",
            "main",
            "Test rerun",
            &["src/test.rs".to_string()],
        );

        // Record a prior run
        record_task_run(
            sig.clone(),
            "speckit.implement".to_string(),
            "main".to_string(),
            Some(true),
            1,
        );

        // Should select ACE due to rerun
        let decision = select_route(
            &config,
            "speckit.implement",
            "main",
            "Test rerun",
            &["src/test.rs".to_string()],
            None,
            false,
        );

        assert!(decision.should_use_ace());
        assert!(decision.reason().unwrap().contains("rerun"));
    }

    #[test]
    fn test_route_selection_failure() {
        let config = AceConfig {
            enabled: true,
            mode: codex_core::config_types::AceMode::Auto,
            slice_size: 8,
            db_path: "test.db".to_string(),
            use_for: vec![],
            complex_task_files_threshold: 4,
            rerun_window_minutes: 30,
        };

        // Should select ACE due to prior failure
        let decision = select_route(
            &config,
            "speckit.implement",
            "main",
            "Test failure",
            &["src/test.rs".to_string()],
            None,
            true, // prior_failure = true
        );

        assert!(decision.should_use_ace());
        assert!(decision.reason().unwrap().contains("failed"));
    }

    #[test]
    fn test_route_selection_high_file_count() {
        let config = AceConfig {
            enabled: true,
            mode: codex_core::config_types::AceMode::Auto,
            slice_size: 8,
            db_path: "test.db".to_string(),
            use_for: vec![],
            complex_task_files_threshold: 4,
            rerun_window_minutes: 30,
        };

        let diff_stat = DiffStat::new(10, 500, 200);

        // Should select ACE due to high file count
        let decision = select_route(
            &config,
            "speckit.implement",
            "main",
            "Large refactor",
            &[],
            Some(&diff_stat),
            false,
        );

        assert!(decision.should_use_ace());
        assert!(decision.reason().unwrap().contains("high file count"));
    }

    #[test]
    fn test_route_selection_keywords() {
        let config = AceConfig {
            enabled: true,
            mode: codex_core::config_types::AceMode::Auto,
            slice_size: 8,
            db_path: "test.db".to_string(),
            use_for: vec![],
            complex_task_files_threshold: 4,
            rerun_window_minutes: 30,
        };

        // Should select ACE due to keywords
        let decision = select_route(
            &config,
            "speckit.implement",
            "main",
            "Refactor authentication module",
            &[],
            None,
            false,
        );

        assert!(decision.should_use_ace());
        assert!(decision.reason().unwrap().contains("cross-cutting"));
    }

    #[test]
    fn test_route_selection_direct() {
        let config = AceConfig {
            enabled: true,
            mode: codex_core::config_types::AceMode::Auto,
            slice_size: 8,
            db_path: "test.db".to_string(),
            use_for: vec![],
            complex_task_files_threshold: 4,
            rerun_window_minutes: 30,
        };

        let diff_stat = DiffStat::new(2, 50, 10);

        // Should use direct (no triggers)
        let decision = select_route(
            &config,
            "speckit.implement",
            "main",
            "Add small feature",
            &["src/feature.rs".to_string()],
            Some(&diff_stat),
            false,
        );

        assert!(!decision.should_use_ace());
    }

    #[test]
    fn test_route_selection_ace_disabled() {
        let config = AceConfig {
            enabled: false, // Disabled
            mode: codex_core::config_types::AceMode::Auto,
            slice_size: 8,
            db_path: "test.db".to_string(),
            use_for: vec![],
            complex_task_files_threshold: 4,
            rerun_window_minutes: 30,
        };

        // Should use direct even with triggers
        let decision = select_route(
            &config,
            "speckit.implement",
            "main",
            "Refactor everything",
            &[],
            None,
            true,
        );

        assert!(!decision.should_use_ace());
    }

    /// Integration test: Route selection respects all ACE config settings
    #[test]
    fn test_integration_route_selection_disabled() {
        let config = AceConfig {
            enabled: false,
            mode: codex_core::config_types::AceMode::Auto,
            slice_size: 8,
            db_path: "test.db".to_string(),
            use_for: vec![],
            complex_task_files_threshold: 4,
            rerun_window_minutes: 30,
        };

        // Even with all triggers, should use Direct when disabled
        let decision = select_route(
            &config,
            "speckit.implement",
            "main",
            "Refactor monorepo structure", // Cross-cutting keyword
            &[],
            Some(&DiffStat::new(10, 500, 200)), // High file count
            true,                               // Prior failure
        );

        assert!(!decision.should_use_ace());
    }

    /// Integration test: Mode=never behaves like disabled
    #[test]
    fn test_integration_mode_never_like_disabled() {
        let disabled = AceConfig {
            enabled: false,
            ..Default::default()
        };

        let never = AceConfig {
            enabled: true,
            mode: codex_core::config_types::AceMode::Never,
            ..Default::default()
        };

        let diff = DiffStat::new(10, 100, 50);

        // Both should make same decision
        let decision1 = select_route(
            &disabled,
            "speckit.implement",
            "main",
            "Test",
            &[],
            Some(&diff),
            true,
        );
        let decision2 = select_route(
            &never,
            "speckit.implement",
            "main",
            "Test",
            &[],
            Some(&diff),
            true,
        );

        assert!(!decision1.should_use_ace());
        assert!(!decision2.should_use_ace());
    }

    #[test]
    fn test_route_selection_non_implement() {
        let config = AceConfig {
            enabled: true,
            mode: codex_core::config_types::AceMode::Auto,
            slice_size: 8,
            db_path: "test.db".to_string(),
            use_for: vec![],
            complex_task_files_threshold: 4,
            rerun_window_minutes: 30,
        };

        // Should use direct for non-implement commands
        let decision = select_route(
            &config,
            "speckit.tasks",
            "main",
            "Refactor everything",
            &[],
            None,
            true,
        );

        assert!(!decision.should_use_ace());
    }
}
