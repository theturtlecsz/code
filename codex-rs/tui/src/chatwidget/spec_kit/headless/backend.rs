//! Mockable agent execution backend for headless mode (SPEC-KIT-900)
//!
//! Provides a trait-based abstraction for agent execution that enables:
//! - Real execution via AGENT_MANAGER for production
//! - Mock execution for deterministic testing

use std::time::Duration;

use codex_core::agent_tool::{AGENT_MANAGER, AgentStatus};
use codex_core::config_types::AgentConfig;

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

/// Resolve a user-facing agent name (from prompts.json) to a configured agent name.
///
/// SPEC-KIT-981: Uses shared resolver for TUI/headless parity.
/// Falls back to agent_name if resolution fails (lets downstream produce a clear error).
fn resolve_agent_config_name(agent_name: &str, agent_configs: &[AgentConfig]) -> String {
    // SPEC-KIT-981: Use shared resolver, fall back to agent_name on error
    super::super::agent_resolver::resolve_agent_config_name(agent_name, agent_configs)
        .unwrap_or_else(|_| agent_name.to_string())
}

/// Real backend using codex_core::agent_tool::AGENT_MANAGER
///
/// This implementation spawns real agents via AGENT_MANAGER and polls
/// for completion using a dedicated tokio runtime.
pub struct DefaultAgentBackend {
    /// Agent configurations loaded from config.toml
    agent_configs: Vec<AgentConfig>,
}

impl DefaultAgentBackend {
    /// Create a new backend with agent configurations
    pub fn new(agent_configs: Vec<AgentConfig>) -> Self {
        Self { agent_configs }
    }
}

impl AgentBackend for DefaultAgentBackend {
    fn run_stage_agent(
        &self,
        agent_name: &str,
        prompt: String,
        spec_id: &str,
        stage: &str,
        timeout: Duration,
    ) -> Result<String, HeadlessError> {
        let config_name = resolve_agent_config_name(agent_name, &self.agent_configs);
        let batch_id = format!("headless-{}-{}", spec_id, stage);
        let agent_configs = self.agent_configs.clone();

        tracing::info!(
            agent = %agent_name,
            config_name = %config_name.as_str(),
            spec_id = %spec_id,
            stage = %stage,
            "DefaultAgentBackend: spawning real agent"
        );

        // Execute the async agent creation and polling
        // Handle both cases: inside an existing runtime (CLI) or outside (tests)
        let async_block = async {
            // Step 1: Create the agent via AGENT_MANAGER
            let agent_id = {
                let mut manager = AGENT_MANAGER.write().await;
                manager
                    .create_agent_from_config_name(
                        &config_name,
                        &agent_configs,
                        prompt,
                        false, // read_only
                        Some(batch_id),
                    )
                    .await
                    .map_err(|e| HeadlessError::InfraError(e))?
            };

            tracing::info!(
                agent_id = %agent_id,
                agent = %agent_name,
                "Agent created, polling for completion"
            );

            // Step 2: Poll until terminal status
            let start = std::time::Instant::now();
            let poll_interval = Duration::from_millis(500);

            loop {
                if start.elapsed() > timeout {
                    tracing::warn!(
                        agent_id = %agent_id,
                        elapsed_ms = start.elapsed().as_millis(),
                        "Agent timeout"
                    );
                    return Err(HeadlessError::Timeout {
                        expected: 1,
                        completed: 0,
                        elapsed_ms: start.elapsed().as_millis() as u64,
                    });
                }

                let manager = AGENT_MANAGER.read().await;
                if let Some(agent) = manager.get_agent(&agent_id) {
                    match agent.status {
                        AgentStatus::Completed => {
                            tracing::info!(
                                agent_id = %agent_id,
                                elapsed_ms = start.elapsed().as_millis(),
                                "Agent completed successfully"
                            );
                            return agent.result.clone().ok_or_else(|| {
                                HeadlessError::InfraError(
                                    "Agent completed but no result available".to_string(),
                                )
                            });
                        }
                        AgentStatus::Failed => {
                            let error_msg = agent
                                .error
                                .clone()
                                .unwrap_or_else(|| "Unknown agent error".to_string());
                            tracing::error!(
                                agent_id = %agent_id,
                                error = %error_msg,
                                "Agent failed"
                            );
                            return Err(HeadlessError::AgentFailed {
                                agent: agent_name.to_string(),
                                reason: error_msg,
                            });
                        }
                        AgentStatus::Cancelled => {
                            tracing::warn!(agent_id = %agent_id, "Agent cancelled");
                            return Err(HeadlessError::AgentFailed {
                                agent: agent_name.to_string(),
                                reason: "cancelled".to_string(),
                            });
                        }
                        AgentStatus::Pending | AgentStatus::Running => {
                            // Still in progress, continue polling
                        }
                    }
                }
                drop(manager);

                tokio::time::sleep(poll_interval).await;
            }
        };

        // Execute the async block, handling both runtime contexts
        if let Ok(handle) = tokio::runtime::Handle::try_current() {
            // We're inside an existing tokio runtime (e.g., CLI)
            // Use block_in_place to allow blocking within the async context
            tokio::task::block_in_place(|| handle.block_on(async_block))
        } else {
            // No existing runtime (e.g., synchronous test context)
            // Create a new runtime
            let rt = tokio::runtime::Runtime::new().map_err(|e| {
                HeadlessError::InfraError(format!("Failed to create tokio runtime: {}", e))
            })?;
            rt.block_on(async_block)
        }
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
