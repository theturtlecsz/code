//! ACE learning integration for post-execution feedback
//!
//! Collects execution outcomes (compile, tests, lints) and sends
//! feedback to ACE for learning.

use super::ace_client::{self, AceResult};
use super::ace_route_selector::DiffStat;
use codex_core::config_types::AceConfig;
use serde::{Deserialize, Serialize};
use std::time::Instant;
use tracing::{debug, info, warn};

/// Maximum total length for stack traces (to keep feedback compact)
const MAX_STACK_TRACE_TOTAL: usize = 2000;

/// Compact execution feedback for ACE learning
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecutionFeedback {
    pub compile_ok: bool,
    pub tests_passed: bool,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub failing_tests: Vec<String>,
    pub lint_issues: usize,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub stack_traces: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub diff_stat: Option<DiffStatSummary>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiffStatSummary {
    pub files: usize,
    pub insertions: usize,
    pub deletions: usize,
}

impl From<&DiffStat> for DiffStatSummary {
    fn from(stat: &DiffStat) -> Self {
        Self {
            files: stat.files_changed,
            insertions: stat.insertions,
            deletions: stat.deletions,
        }
    }
}

impl ExecutionFeedback {
    /// Create new feedback with default values
    pub fn new() -> Self {
        Self {
            compile_ok: true,
            tests_passed: true,
            failing_tests: Vec::new(),
            lint_issues: 0,
            stack_traces: Vec::new(),
            diff_stat: None,
        }
    }

    /// Set compile status
    pub fn with_compile_ok(mut self, ok: bool) -> Self {
        self.compile_ok = ok;
        self
    }

    /// Set test status
    pub fn with_tests_passed(mut self, passed: bool) -> Self {
        self.tests_passed = passed;
        self
    }

    /// Add failing test names
    pub fn with_failing_tests(mut self, tests: Vec<String>) -> Self {
        self.failing_tests = tests;
        self
    }

    /// Set lint issue count
    pub fn with_lint_issues(mut self, count: usize) -> Self {
        self.lint_issues = count;
        self
    }

    /// Add stack traces (will be trimmed to MAX_STACK_TRACE_TOTAL)
    pub fn with_stack_traces(mut self, traces: Vec<String>) -> Self {
        self.stack_traces = trim_stack_traces(traces, MAX_STACK_TRACE_TOTAL);
        self
    }

    /// Set diff statistics
    pub fn with_diff_stat(mut self, stat: DiffStat) -> Self {
        self.diff_stat = Some(DiffStatSummary::from(&stat));
        self
    }

    /// Check if execution was successful
    pub fn is_successful(&self) -> bool {
        self.compile_ok && self.tests_passed && self.lint_issues == 0
    }

    /// Format as JSON string for ACE
    pub fn to_json_string(&self) -> Result<String, serde_json::Error> {
        serde_json::to_string(self)
    }
}

impl Default for ExecutionFeedback {
    fn default() -> Self {
        Self::new()
    }
}

/// Trim stack traces to fit within total character limit
fn trim_stack_traces(traces: Vec<String>, max_total: usize) -> Vec<String> {
    let mut result = Vec::new();
    let mut total_len = 0;

    for trace in traces {
        if total_len + trace.len() > max_total {
            // Take what we can from this trace
            let remaining = max_total - total_len;
            if remaining > 100 {
                // Only include if we have meaningful space
                let trimmed = format!("{}... [truncated]", &trace[..remaining.min(trace.len())]);
                result.push(trimmed);
            }
            break;
        }

        total_len += trace.len();
        result.push(trace);
    }

    result
}

/// Create a patch summary from task description and files changed
fn create_patch_summary(task_title: &str, diff_stat: Option<&DiffStat>) -> String {
    let mut summary = task_title.to_string();

    if let Some(stat) = diff_stat {
        summary.push_str(&format!(
            " ({} files, +{} -{} lines)",
            stat.files_changed, stat.insertions, stat.deletions
        ));
    }

    summary
}

/// Send learning feedback to ACE
///
/// This should be called after build/test/lint validation completes.
/// It sends compact feedback to ACE for learning from outcomes.
pub async fn send_learning_feedback(
    config: &AceConfig,
    repo_root: String,
    branch: String,
    scope: &str,
    task_title: &str,
    feedback: ExecutionFeedback,
    diff_stat: Option<DiffStat>,
) {
    // Check if ACE is enabled
    if !config.enabled {
        debug!("ACE learning skipped: disabled");
        return;
    }

    let start = Instant::now();

    // Create patch summary
    let attempt = create_patch_summary(task_title, diff_stat.as_ref());

    // Format feedback as JSON
    let feedback_json = match feedback.to_json_string() {
        Ok(json) => json,
        Err(e) => {
            warn!("Failed to serialize feedback for ACE learning: {}", e);
            return;
        }
    };

    // Call ACE learn
    // TODO: Track bullet IDs from injection and pass here
    let bullet_ids_used = Vec::new();

    let result = ace_client::learn(
        repo_root,
        branch,
        scope.to_string(),
        task_title.to_string(),
        attempt,
        feedback_json,
        bullet_ids_used,
    )
    .await;

    let elapsed = start.elapsed();

    match result {
        AceResult::Ok(response) => {
            info!(
                "ACE learn {}ms scope={} added={} demoted={} promoted={}",
                elapsed.as_millis(),
                scope,
                response.updated_bullets.added,
                response.updated_bullets.demoted,
                response.updated_bullets.promoted
            );
        }
        AceResult::Disabled => {
            debug!("ACE learning skipped: ACE disabled");
        }
        AceResult::Error(e) => {
            warn!("ACE learning failed ({}ms): {}", elapsed.as_millis(), e);
        }
    }
}

/// Synchronous wrapper for send_learning_feedback
///
/// Uses tokio runtime handle to bridge sync/async boundary.
/// Safe to call from synchronous contexts on a tokio runtime.
pub fn send_learning_feedback_sync(
    config: &AceConfig,
    repo_root: String,
    branch: String,
    scope: &str,
    task_title: &str,
    feedback: ExecutionFeedback,
    diff_stat: Option<DiffStat>,
) {
    // Clone data for move into async task
    let config = config.clone();
    let scope = scope.to_string();
    let task_title = task_title.to_string();

    // Check if we're on a tokio runtime
    match tokio::runtime::Handle::try_current() {
        Ok(handle) => {
            // Spawn task to avoid blocking
            handle.spawn(async move {
                send_learning_feedback(
                    &config,
                    repo_root,
                    branch,
                    &scope,
                    &task_title,
                    feedback,
                    diff_stat,
                )
                .await;
            });
        }
        Err(_) => {
            debug!("ACE learning skipped: not on tokio runtime");
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_execution_feedback_success() {
        let feedback = ExecutionFeedback::new()
            .with_compile_ok(true)
            .with_tests_passed(true)
            .with_lint_issues(0);

        assert!(feedback.is_successful());
    }

    #[test]
    fn test_execution_feedback_compile_failure() {
        let feedback = ExecutionFeedback::new()
            .with_compile_ok(false)
            .with_tests_passed(true);

        assert!(!feedback.is_successful());
    }

    #[test]
    fn test_execution_feedback_test_failure() {
        let feedback = ExecutionFeedback::new()
            .with_compile_ok(true)
            .with_tests_passed(false)
            .with_failing_tests(vec!["test_foo".to_string(), "test_bar".to_string()]);

        assert!(!feedback.is_successful());
        assert_eq!(feedback.failing_tests.len(), 2);
    }

    #[test]
    fn test_execution_feedback_lint_issues() {
        let feedback = ExecutionFeedback::new()
            .with_compile_ok(true)
            .with_tests_passed(true)
            .with_lint_issues(5);

        assert!(!feedback.is_successful());
    }

    #[test]
    fn test_trim_stack_traces() {
        let traces = vec![
            "Error: foo\n  at bar.rs:10".to_string(),
            "Error: baz\n  at qux.rs:20".to_string(),
            "Error: very long trace that will be truncated".repeat(100),
        ];

        let trimmed = trim_stack_traces(traces, 200);

        // Should have first two, third truncated or omitted
        assert!(trimmed.len() <= 3);

        let total_len: usize = trimmed.iter().map(|s| s.len()).sum();
        // Allow some slack for truncation suffix
        assert!(total_len <= 220, "total_len was {}", total_len);
    }

    #[test]
    fn test_feedback_serialization() {
        let feedback = ExecutionFeedback::new()
            .with_compile_ok(false)
            .with_tests_passed(false)
            .with_failing_tests(vec!["test_foo".to_string()])
            .with_lint_issues(3)
            .with_stack_traces(vec!["Error at line 10".to_string()])
            .with_diff_stat(DiffStat::new(5, 100, 50));

        let json = feedback.to_json_string().unwrap();

        // Should contain key fields
        assert!(json.contains("compile_ok"));
        assert!(json.contains("tests_passed"));
        assert!(json.contains("failing_tests"));
        assert!(json.contains("lint_issues"));
        assert!(json.contains("diff_stat"));
    }

    #[test]
    fn test_create_patch_summary() {
        let summary = create_patch_summary("Add authentication", None);
        assert_eq!(summary, "Add authentication");

        let summary_with_diff =
            create_patch_summary("Add authentication", Some(&DiffStat::new(3, 150, 20)));
        assert!(summary_with_diff.contains("3 files"));
        assert!(summary_with_diff.contains("+150"));
        assert!(summary_with_diff.contains("-20"));
    }
}
