//! Headless output formatting (SPEC-KIT-900)
//!
//! Formats headless execution results as JSON for CLI consumption.

use serde::{Deserialize, Serialize};

/// Headless execution output for JSON serialization
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HeadlessOutput {
    /// Schema version (bump only on breaking changes)
    pub schema_version: u32,
    /// Tool version for debugging
    pub tool_version: String,
    /// Execution mode (always "execute" for headless runner)
    pub mode: String,
    /// SPEC ID
    pub spec_id: String,
    /// Starting stage
    pub from_stage: String,
    /// Ending stage
    pub to_stage: String,
    /// Exit code
    pub exit_code: i32,
    /// Exit reason (semantic string)
    pub exit_reason: String,
    /// Stages that completed successfully
    pub stages_completed: Vec<String>,
    /// Error message if any
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
    /// Stage0 execution info
    #[serde(skip_serializing_if = "Option::is_none")]
    pub stage0: Option<Stage0Info>,
}

/// Stage0 execution info for output
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Stage0Info {
    /// Whether Stage0 completed
    pub completed: bool,
    /// Execution time in milliseconds
    pub duration_ms: u64,
    /// Whether Tier2 (NotebookLM) was used
    pub tier2_used: bool,
}

impl HeadlessOutput {
    /// Create output for successful execution
    pub fn success(
        spec_id: String,
        from_stage: String,
        to_stage: String,
        stages_completed: Vec<String>,
        stage0: Option<Stage0Info>,
    ) -> Self {
        Self {
            schema_version: 1,
            tool_version: tool_version(),
            mode: "execute".to_string(),
            spec_id,
            from_stage,
            to_stage,
            exit_code: 0,
            exit_reason: "success".to_string(),
            stages_completed,
            error: None,
            stage0,
        }
    }

    /// Create output for error
    pub fn error(
        spec_id: String,
        from_stage: String,
        to_stage: String,
        exit_code: i32,
        exit_reason: &str,
        error: String,
        stages_completed: Vec<String>,
    ) -> Self {
        Self {
            schema_version: 1,
            tool_version: tool_version(),
            mode: "execute".to_string(),
            spec_id,
            from_stage,
            to_stage,
            exit_code,
            exit_reason: exit_reason.to_string(),
            stages_completed,
            error: Some(error),
            stage0: None,
        }
    }
}

/// Format HeadlessResult to JSON string
pub fn format_result_json(output: &HeadlessOutput) -> String {
    serde_json::to_string_pretty(output)
        .unwrap_or_else(|e| format!(r#"{{"error": "Failed to serialize output: {}"}}"#, e))
}

/// Get tool version string with git sha for debugging
fn tool_version() -> String {
    let base_version = env!("CARGO_PKG_VERSION");
    let git_sha = option_env!("SPECKIT_GIT_SHA")
        .or(option_env!("GIT_SHA"))
        .unwrap_or("");

    if git_sha.is_empty() {
        base_version.to_string()
    } else {
        format!("{base_version}+{git_sha}")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_success_output() {
        let output = HeadlessOutput::success(
            "SPEC-KIT-900".to_string(),
            "plan".to_string(),
            "validate".to_string(),
            vec!["plan".to_string(), "tasks".to_string()],
            None,
        );

        assert_eq!(output.exit_code, 0);
        assert_eq!(output.mode, "execute");
        assert_eq!(output.stages_completed.len(), 2);
    }

    #[test]
    fn test_error_output() {
        let output = HeadlessOutput::error(
            "SPEC-KIT-900".to_string(),
            "plan".to_string(),
            "validate".to_string(),
            10,
            "needs_input",
            "Missing maieutic".to_string(),
            vec![],
        );

        assert_eq!(output.exit_code, 10);
        assert_eq!(output.exit_reason, "needs_input");
        assert!(output.error.is_some());
    }

    #[test]
    fn test_json_format() {
        let output = HeadlessOutput::success(
            "TEST".to_string(),
            "plan".to_string(),
            "tasks".to_string(),
            vec![],
            None,
        );

        let json = format_result_json(&output);
        assert!(json.contains("\"mode\": \"execute\""));
    }
}
