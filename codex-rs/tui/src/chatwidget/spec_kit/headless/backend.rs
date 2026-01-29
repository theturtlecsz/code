//! Mockable agent execution backend for headless mode (SPEC-KIT-900)
//!
//! Provides a trait-based abstraction for agent execution that enables:
//! - Real execution via AGENT_MANAGER for production
//! - Mock execution for deterministic testing

use std::time::Duration;

use super::runner::HeadlessError;

/// Trait for agent execution backends (mockable for tests)
///
/// This abstraction allows `HeadlessPipelineRunner` to work with both:
/// - Real LLM agents via AGENT_MANAGER
/// - Mock agents that return canned outputs for testing
pub trait AgentBackend: Send + Sync {
    /// Execute an agent and return its output
    ///
    /// # Arguments
    /// - `agent_name`: Name of the agent (e.g., "gemini", "claude", "gpt_pro")
    /// - `prompt`: The full prompt to send to the agent
    /// - `spec_id`: SPEC identifier for logging
    /// - `stage`: Stage name (e.g., "plan", "tasks")
    /// - `timeout`: Maximum time to wait for agent completion
    ///
    /// # Returns
    /// - `Ok(String)`: Agent's output (typically JSON)
    /// - `Err(HeadlessError)`: On timeout, infra error, or agent failure
    fn run_stage_agent(
        &self,
        agent_name: &str,
        prompt: String,
        spec_id: &str,
        stage: &str,
        timeout: Duration,
    ) -> Result<String, HeadlessError>;
}

/// Real backend using codex_core::agent_tool::AGENT_MANAGER
///
/// This implementation is a placeholder that returns an error because:
/// - AGENT_MANAGER uses async locks requiring tokio runtime
/// - Agent configs need to be loaded from ChatWidget context
///
/// For production use, this needs proper async integration (SPEC-KIT-930).
/// For tests, use MockAgentBackend instead.
pub struct DefaultAgentBackend;

impl DefaultAgentBackend {
    /// Create a new backend (placeholder)
    pub fn new(_agent_configs: Vec<codex_core::config_types::AgentConfig>) -> Self {
        Self
    }
}

impl AgentBackend for DefaultAgentBackend {
    fn run_stage_agent(
        &self,
        agent_name: &str,
        _prompt: String,
        spec_id: &str,
        stage: &str,
        _timeout: Duration,
    ) -> Result<String, HeadlessError> {
        // ESCALATED: Real agent spawning requires async context and proper integration
        // with AGENT_MANAGER's tokio-based RwLock.
        //
        // For now, return error to prevent false-green tests.
        // Tracked in SPEC-KIT-930 (widget-independent infrastructure).
        tracing::warn!(
            agent = %agent_name,
            spec_id = %spec_id,
            stage = %stage,
            "DefaultAgentBackend: real agent spawning not implemented"
        );
        Err(HeadlessError::InfraError(
            "Real agent spawning requires async context - use MockAgentBackend for tests (SPEC-KIT-930)"
                .to_string(),
        ))
    }
}

/// Mock backend for testing (returns canned outputs)
///
/// Use this in unit tests to avoid real LLM calls and ensure deterministic results.
#[cfg(test)]
pub struct MockAgentBackend {
    /// Map from stage name to canned output
    pub responses: std::collections::HashMap<String, String>,
}

#[cfg(test)]
impl MockAgentBackend {
    /// Create a mock backend with pre-defined responses
    ///
    /// # Arguments
    /// - `responses`: Map from stage name (e.g., "plan", "tasks") to JSON output
    pub fn with_responses(responses: std::collections::HashMap<String, String>) -> Self {
        Self { responses }
    }

    /// Create a mock backend with default test responses
    pub fn with_default_responses() -> Self {
        let mut responses = std::collections::HashMap::new();

        // Default plan stage response
        responses.insert(
            "plan".to_string(),
            serde_json::json!({
                "stage": "spec-plan",
                "agent": "mock",
                "research_summary": [
                    {"topic": "Test topic", "details": "Mock research details"}
                ],
                "questions": []
            })
            .to_string(),
        );

        // Default tasks stage response
        responses.insert(
            "tasks".to_string(),
            serde_json::json!({
                "stage": "spec-tasks",
                "agent": "mock",
                "tasks": [
                    {
                        "order": 1,
                        "task_id": "TASK-001",
                        "title": "Mock task",
                        "status": "pending",
                        "validation_step": "Run tests",
                        "artifact": "test_file.rs",
                        "notes": "Mock task note"
                    }
                ],
                "acceptance_coverage": ["Tests pass"],
                "followups": []
            })
            .to_string(),
        );

        Self { responses }
    }
}

#[cfg(test)]
impl AgentBackend for MockAgentBackend {
    fn run_stage_agent(
        &self,
        agent_name: &str,
        _prompt: String,
        spec_id: &str,
        stage: &str,
        _timeout: Duration,
    ) -> Result<String, HeadlessError> {
        tracing::debug!(
            agent = %agent_name,
            spec_id = %spec_id,
            stage = %stage,
            "MockAgentBackend: returning canned response"
        );

        self.responses.get(stage).cloned().ok_or_else(|| {
            HeadlessError::InfraError(format!(
                "MockAgentBackend: no response configured for stage '{}'",
                stage
            ))
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    #[test]
    fn test_mock_backend_returns_configured_response() {
        let mut responses = HashMap::new();
        responses.insert("plan".to_string(), r#"{"stage":"plan"}"#.to_string());
        let backend = MockAgentBackend::with_responses(responses);

        let result = backend.run_stage_agent(
            "test-agent",
            "test prompt".to_string(),
            "TEST-001",
            "plan",
            Duration::from_secs(30),
        );

        assert!(result.is_ok());
        assert_eq!(result.unwrap(), r#"{"stage":"plan"}"#);
    }

    #[test]
    fn test_mock_backend_returns_error_for_unknown_stage() {
        let backend = MockAgentBackend::with_responses(HashMap::new());

        let result = backend.run_stage_agent(
            "test-agent",
            "test prompt".to_string(),
            "TEST-001",
            "unknown",
            Duration::from_secs(30),
        );

        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), HeadlessError::InfraError(_)));
    }

    #[test]
    fn test_mock_backend_with_default_responses() {
        let backend = MockAgentBackend::with_default_responses();

        // Plan stage should work
        let plan_result = backend.run_stage_agent(
            "test-agent",
            "test prompt".to_string(),
            "TEST-001",
            "plan",
            Duration::from_secs(30),
        );
        assert!(plan_result.is_ok());
        assert!(plan_result.unwrap().contains("spec-plan"));

        // Tasks stage should work
        let tasks_result = backend.run_stage_agent(
            "test-agent",
            "test prompt".to_string(),
            "TEST-001",
            "tasks",
            Duration::from_secs(30),
        );
        assert!(tasks_result.is_ok());
        assert!(tasks_result.unwrap().contains("spec-tasks"));
    }
}
