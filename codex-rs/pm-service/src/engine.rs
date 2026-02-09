//! Stub bot engines for Phase-0 walking skeleton.
//!
//! These engines immediately succeed, producing minimal artifacts.
//! Real engines (research with NotebookLM, review with validation)
//! will be implemented in Phase 2.

use codex_core::pm::artifacts::{
    BotRunLog, BotRunState, ResearchFinding, ResearchReport, ReviewReport,
};
use codex_core::pm::bot::BotKind;

/// Result of a stub engine execution.
pub struct EngineResult {
    pub state: BotRunState,
    pub exit_code: i32,
    pub summary: String,
    pub log: BotRunLog,
    /// Serialized report artifact (JSON).
    pub report_json: String,
}

/// Execute a stub research engine.
///
/// Phase-0: immediately succeeds with a placeholder report.
pub fn run_research_stub(run_id: &str, work_item_id: &str) -> EngineResult {
    let now = chrono::Utc::now().to_rfc3339();
    let report = ResearchReport {
        schema_version: ResearchReport::SCHEMA_VERSION.to_string(),
        run_id: run_id.to_string(),
        work_item_id: work_item_id.to_string(),
        timestamp: now.clone(),
        findings: vec![ResearchFinding {
            title: "Stub research".to_string(),
            body: "Phase-0 stub: no real research performed.".to_string(),
            source: None,
            confidence: Some("n/a".to_string()),
        }],
        summary: "Phase-0 stub research completed.".to_string(),
    };

    let report_json = serde_json::to_string_pretty(&report).unwrap_or_else(|_| "{}".to_string());

    let log = BotRunLog {
        schema_version: BotRunLog::SCHEMA_VERSION.to_string(),
        run_id: run_id.to_string(),
        work_item_id: work_item_id.to_string(),
        state: BotRunState::Succeeded,
        started_at: now.clone(),
        finished_at: now,
        duration_s: 0,
        exit_code: 0,
        summary: "Stub research completed".to_string(),
        partial: false,
        checkpoint_count: 0,
        error: None,
    };

    EngineResult {
        state: BotRunState::Succeeded,
        exit_code: 0,
        summary: "Stub research completed".to_string(),
        log,
        report_json,
    }
}

/// Execute a stub review engine.
///
/// Phase-0: immediately succeeds with a placeholder report.
pub fn run_review_stub(run_id: &str, work_item_id: &str) -> EngineResult {
    let now = chrono::Utc::now().to_rfc3339();
    let report = ReviewReport {
        schema_version: ReviewReport::SCHEMA_VERSION.to_string(),
        run_id: run_id.to_string(),
        work_item_id: work_item_id.to_string(),
        timestamp: now.clone(),
        findings: vec![],
        has_patches: false,
        summary: "Phase-0 stub review completed.".to_string(),
    };

    let report_json = serde_json::to_string_pretty(&report).unwrap_or_else(|_| "{}".to_string());

    let log = BotRunLog {
        schema_version: BotRunLog::SCHEMA_VERSION.to_string(),
        run_id: run_id.to_string(),
        work_item_id: work_item_id.to_string(),
        state: BotRunState::Succeeded,
        started_at: now.clone(),
        finished_at: now,
        duration_s: 0,
        exit_code: 0,
        summary: "Stub review completed".to_string(),
        partial: false,
        checkpoint_count: 0,
        error: None,
    };

    EngineResult {
        state: BotRunState::Succeeded,
        exit_code: 0,
        summary: "Stub review completed".to_string(),
        log,
        report_json,
    }
}

/// Dispatch to the appropriate stub engine.
pub fn run_stub(kind: BotKind, run_id: &str, work_item_id: &str) -> EngineResult {
    match kind {
        BotKind::Research => run_research_stub(run_id, work_item_id),
        BotKind::Review => run_review_stub(run_id, work_item_id),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn research_stub_succeeds() {
        let result = run_research_stub("run-001", "SPEC-TEST-001");
        assert_eq!(result.state, BotRunState::Succeeded);
        assert_eq!(result.exit_code, 0);
        assert!(!result.report_json.is_empty());
    }

    #[test]
    fn review_stub_succeeds() {
        let result = run_review_stub("run-001", "SPEC-TEST-001");
        assert_eq!(result.state, BotRunState::Succeeded);
        assert_eq!(result.exit_code, 0);
    }
}
