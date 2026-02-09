//! Bot run artifact types (SPEC-PM-002 / SPEC-PM-003).
//!
//! These types define the capsule artifacts produced by bot runs:
//! - `BotRunState` — run lifecycle state machine
//! - `BotRunLog` — terminal record for a completed/cancelled/failed run
//! - `BotRunCheckpoint` — periodic progress snapshot
//! - `BotRunResult` — discriminated outcome (research or review)
//!
//! ## Decision References
//! - PM-D11: Capsule-backed run queue
//! - PM-D13: Cancellation persists partial artifacts
//! - PM-D15: Checkpoint: hybrid event-driven + 30min floor
//! - PM-D16: Reject capture=none (exit 10)

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

/// Bot run lifecycle state (PM-D11).
#[derive(Debug, Clone, Copy, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum BotRunState {
    /// Persisted to capsule, waiting for execution slot.
    Queued,
    /// Engine is actively processing.
    Running,
    /// Completed successfully.
    Succeeded,
    /// Completed with issues requiring human attention.
    NeedsAttention,
    /// Run failed due to infrastructure or logic error.
    Failed,
    /// Run was cancelled by user (PM-D13: partial artifacts preserved).
    Cancelled,
}

impl BotRunState {
    /// Whether this is a terminal state (no further transitions).
    pub fn is_terminal(self) -> bool {
        matches!(
            self,
            BotRunState::Succeeded
                | BotRunState::NeedsAttention
                | BotRunState::Failed
                | BotRunState::Cancelled
        )
    }
}

/// A checkpoint artifact written periodically during a run (PM-D15).
///
/// Written every 30min or on significant events. Content depth
/// varies by capture mode (PM-D16 compliance table in brief).
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
pub struct BotRunCheckpoint {
    pub schema_version: String,
    pub run_id: String,
    pub work_item_id: String,
    pub seq: u32,
    pub state: BotRunState,
    pub timestamp: String,
    /// Human-readable progress summary.
    pub summary: String,
    /// Optional progress percentage (0-100).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub percent: Option<u8>,
    /// Phase/stage the engine is currently in.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub phase: Option<String>,
}

impl BotRunCheckpoint {
    pub const SCHEMA_VERSION: &'static str = "bot_run_checkpoint@1.0";
}

/// Terminal log for a completed bot run.
///
/// Written once when the run reaches a terminal state.
/// Always includes the final state and timing.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
pub struct BotRunLog {
    pub schema_version: String,
    pub run_id: String,
    pub work_item_id: String,
    pub state: BotRunState,
    /// RFC3339 timestamp.
    pub started_at: String,
    /// RFC3339 timestamp.
    pub finished_at: String,
    /// Duration in seconds.
    pub duration_s: u64,
    /// Exit code (0 = success, 10 = needs_input, etc.).
    pub exit_code: i32,
    /// Human-readable summary.
    pub summary: String,
    /// Whether the run was partial (cancelled with preserved artifacts, PM-D13).
    #[serde(default)]
    pub partial: bool,
    /// Number of checkpoints written during the run.
    #[serde(default)]
    pub checkpoint_count: u32,
    /// Error message if failed.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}

impl BotRunLog {
    pub const SCHEMA_VERSION: &'static str = "bot_run_log@1.0";
}

/// Research report produced by the research engine.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
pub struct ResearchReport {
    pub schema_version: String,
    pub run_id: String,
    pub work_item_id: String,
    pub timestamp: String,
    /// Structured findings from the research run.
    pub findings: Vec<ResearchFinding>,
    /// Overall summary.
    pub summary: String,
}

impl ResearchReport {
    pub const SCHEMA_VERSION: &'static str = "research_report@1.0";
}

/// A single finding from a research run.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
pub struct ResearchFinding {
    pub title: String,
    pub body: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub source: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub confidence: Option<String>,
}

/// Review report produced by the review engine.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
pub struct ReviewReport {
    pub schema_version: String,
    pub run_id: String,
    pub work_item_id: String,
    pub timestamp: String,
    /// Review findings (issues, suggestions).
    pub findings: Vec<ReviewFinding>,
    /// Whether the review produced patches.
    #[serde(default)]
    pub has_patches: bool,
    /// Overall quality assessment.
    pub summary: String,
}

impl ReviewReport {
    pub const SCHEMA_VERSION: &'static str = "review_report@1.0";
}

/// A single finding from a review run.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
pub struct ReviewFinding {
    pub severity: ReviewSeverity,
    pub title: String,
    pub body: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub file_path: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub line: Option<u32>,
}

/// Severity level for review findings.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ReviewSeverity {
    Info,
    Warning,
    Error,
    Critical,
}

/// Discriminated union of bot run results.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
#[serde(tag = "kind")]
pub enum BotRunResult {
    #[serde(rename = "research")]
    Research { report: ResearchReport },
    #[serde(rename = "review")]
    Review { report: ReviewReport },
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bot_run_state_terminal() {
        assert!(!BotRunState::Queued.is_terminal());
        assert!(!BotRunState::Running.is_terminal());
        assert!(BotRunState::Succeeded.is_terminal());
        assert!(BotRunState::NeedsAttention.is_terminal());
        assert!(BotRunState::Failed.is_terminal());
        assert!(BotRunState::Cancelled.is_terminal());
    }

    #[test]
    fn bot_run_log_roundtrip() {
        let now = "2026-02-09T00:00:00Z".to_string();
        let log = BotRunLog {
            schema_version: BotRunLog::SCHEMA_VERSION.to_string(),
            run_id: "run-001".to_string(),
            work_item_id: "SPEC-TEST-001".to_string(),
            state: BotRunState::Succeeded,
            started_at: now.clone(),
            finished_at: now,
            duration_s: 42,
            exit_code: 0,
            summary: "Stub research completed".to_string(),
            partial: false,
            checkpoint_count: 0,
            error: None,
        };

        let json = serde_json::to_string(&log).unwrap_or_else(|e| panic!("serialize: {e}"));
        let back: BotRunLog =
            serde_json::from_str(&json).unwrap_or_else(|e| panic!("deserialize: {e}"));
        assert_eq!(log.run_id, back.run_id);
        assert_eq!(log.state, back.state);
    }

    #[test]
    fn checkpoint_roundtrip() {
        let cp = BotRunCheckpoint {
            schema_version: BotRunCheckpoint::SCHEMA_VERSION.to_string(),
            run_id: "run-001".to_string(),
            work_item_id: "SPEC-TEST-001".to_string(),
            seq: 0,
            state: BotRunState::Running,
            timestamp: "2026-02-09T00:00:00Z".to_string(),
            summary: "Processing...".to_string(),
            percent: Some(50),
            phase: Some("analysis".to_string()),
        };

        let json = serde_json::to_string(&cp).unwrap_or_else(|e| panic!("serialize: {e}"));
        let back: BotRunCheckpoint =
            serde_json::from_str(&json).unwrap_or_else(|e| panic!("deserialize: {e}"));
        assert_eq!(cp.seq, back.seq);
        assert_eq!(cp.percent, back.percent);
    }

    #[test]
    fn bot_run_result_research_roundtrip() {
        let result = BotRunResult::Research {
            report: ResearchReport {
                schema_version: ResearchReport::SCHEMA_VERSION.to_string(),
                run_id: "run-001".to_string(),
                work_item_id: "SPEC-TEST-001".to_string(),
                timestamp: "2026-02-09T00:00:00Z".to_string(),
                findings: vec![ResearchFinding {
                    title: "Finding 1".to_string(),
                    body: "Details".to_string(),
                    source: None,
                    confidence: None,
                }],
                summary: "One finding".to_string(),
            },
        };

        let json = serde_json::to_string(&result).unwrap_or_else(|e| panic!("serialize: {e}"));
        let back: BotRunResult =
            serde_json::from_str(&json).unwrap_or_else(|e| panic!("deserialize: {e}"));
        assert_eq!(result, back);
    }
}
