//! Claude CLI Provider Implementation (SPEC-KIT-952)
//!
//! Routes prompts through the `claude` CLI for Anthropic models.
//! Uses `--output-format json` for structured responses.

#![allow(dead_code)] // CLI provider helpers pending integration

use crate::cli_executor::{CliError, CliExecutor};
use crate::providers::{ProviderError, ProviderResponse, ProviderResult, TokenUsage};
use serde::Deserialize;
use serde_json::Value;
use std::time::Duration;

/// Event in the Claude CLI JSON output stream
#[derive(Debug, Deserialize)]
struct ClaudeEvent {
    /// Event type: "system", "assistant", "result"
    #[serde(rename = "type")]
    event_type: String,
    /// Result text (only present in "result" events)
    #[serde(default)]
    result: Option<String>,
    /// Usage information (only present in "result" events)
    #[serde(default)]
    usage: Option<ClaudeUsage>,
    /// Model usage breakdown
    #[serde(rename = "modelUsage", default)]
    model_usage: Option<Value>,
}

/// Usage information from Claude CLI
#[derive(Debug, Deserialize)]
struct ClaudeUsage {
    /// Input tokens consumed
    #[serde(default)]
    input_tokens: Option<u32>,
    /// Output tokens generated
    #[serde(default)]
    output_tokens: Option<u32>,
}

/// Claude CLI provider for executing prompts
#[derive(Debug)]
pub struct ClaudeProvider {
    executor: CliExecutor,
}

impl ClaudeProvider {
    /// Create a new Claude provider
    ///
    /// Detects the Claude CLI in PATH and verifies it's available.
    pub fn new() -> ProviderResult<Self> {
        let cli_path = CliExecutor::detect("claude")?;
        let executor = CliExecutor::with_default_timeout(cli_path, "claude".to_string());
        Ok(Self { executor })
    }

    /// Create a new Claude provider with custom timeout
    pub fn with_timeout(timeout_secs: u64) -> ProviderResult<Self> {
        let cli_path = CliExecutor::detect("claude")?;
        let executor = CliExecutor::new(
            cli_path,
            "claude".to_string(),
            Duration::from_secs(timeout_secs),
        );
        Ok(Self { executor })
    }

    /// Execute a prompt using the Claude CLI
    ///
    /// # Arguments
    /// * `prompt` - The prompt text to send to Claude
    ///
    /// # Returns
    /// The provider response with generated content
    pub async fn execute_prompt(&self, prompt: &str) -> ProviderResult<ProviderResponse> {
        // Build command arguments
        let args = vec!["-p", prompt, "--output-format", "json"];

        // Execute CLI command
        let stdout = self.executor.execute_for_stdout(&args).await?;

        // Parse JSON array of events
        let events: Vec<ClaudeEvent> =
            serde_json::from_str(&stdout).map_err(|e| ProviderError::Provider {
                provider: "Claude".to_string(),
                message: format!("Failed to parse JSON response: {}", e),
            })?;

        // Find the "result" event which contains the actual response
        let result_event = events
            .iter()
            .find(|e| e.event_type == "result")
            .ok_or_else(|| ProviderError::Provider {
                provider: "Claude".to_string(),
                message: "No result event found in CLI output".to_string(),
            })?;

        // Extract content from result
        let content = result_event.result.clone().unwrap_or_default();

        // Try to extract model name from modelUsage
        let model = result_event
            .model_usage
            .as_ref()
            .and_then(|mu| mu.as_object())
            .and_then(|obj| obj.keys().next())
            .map(|s| s.to_string())
            .unwrap_or_else(|| "claude".to_string());

        // Extract usage
        let usage = result_event.usage.as_ref().map(|u| TokenUsage {
            input_tokens: u.input_tokens,
            output_tokens: u.output_tokens,
        });

        Ok(ProviderResponse {
            content,
            model,
            usage,
        })
    }

    /// Execute a prompt with CLI routing settings
    ///
    /// # Arguments
    /// * `prompt` - The prompt text to send to Claude
    /// * `settings` - CLI routing settings from TUI config
    ///
    /// # Returns
    /// The provider response with generated content
    pub async fn execute_prompt_with_settings(
        &self,
        prompt: &str,
        settings: &crate::providers::CliRoutingSettings,
    ) -> ProviderResult<ProviderResponse> {
        // Build command arguments
        let mut args = vec!["-p", prompt, "--output-format", "json"];

        // Map sandbox policy to CLI flags
        if let Some(ref sandbox) = settings.sandbox_policy {
            match sandbox {
                codex_core::protocol::SandboxPolicy::DangerFullAccess => {
                    args.push("--dangerously-skip-permissions");
                }
                codex_core::protocol::SandboxPolicy::ReadOnly => {
                    // Read-only is a safe default, no special flag needed
                }
                codex_core::protocol::SandboxPolicy::WorkspaceWrite { .. } => {
                    // Workspace write is handled by Claude's default behavior
                }
            }
        }

        // Map approval policy to permission mode
        if let Some(ref approval) = settings.approval_policy {
            match approval {
                codex_core::protocol::AskForApproval::Never => {
                    args.push("--permission-mode");
                    args.push("bypassPermissions");
                }
                codex_core::protocol::AskForApproval::OnFailure => {
                    args.push("--permission-mode");
                    args.push("acceptEdits");
                }
                codex_core::protocol::AskForApproval::OnRequest
                | codex_core::protocol::AskForApproval::UnlessTrusted => {
                    // Default behavior, no flag needed
                }
            }
        }

        // Execute CLI command
        let stdout = self.executor.execute_for_stdout(&args).await?;

        // Parse JSON array of events
        let events: Vec<ClaudeEvent> =
            serde_json::from_str(&stdout).map_err(|e| ProviderError::Provider {
                provider: "Claude".to_string(),
                message: format!("Failed to parse JSON response: {}", e),
            })?;

        // Find the "result" event which contains the actual response
        let result_event = events
            .iter()
            .find(|e| e.event_type == "result")
            .ok_or_else(|| ProviderError::Provider {
                provider: "Claude".to_string(),
                message: "No result event found in CLI output".to_string(),
            })?;

        // Extract content from result
        let content = result_event.result.clone().unwrap_or_default();

        // Try to extract model name from modelUsage
        let model = result_event
            .model_usage
            .as_ref()
            .and_then(|mu| mu.as_object())
            .and_then(|obj| obj.keys().next())
            .map(|s| s.to_string())
            .unwrap_or_else(|| "claude".to_string());

        // Extract usage
        let usage = result_event.usage.as_ref().map(|u| TokenUsage {
            input_tokens: u.input_tokens,
            output_tokens: u.output_tokens,
        });

        Ok(ProviderResponse {
            content,
            model,
            usage,
        })
    }

    /// Execute a prompt with additional options
    ///
    /// # Arguments
    /// * `prompt` - The prompt text to send to Claude
    /// * `system_prompt` - Optional system prompt
    /// * `model` - Optional specific model to use
    pub async fn execute_with_options(
        &self,
        prompt: &str,
        system_prompt: Option<&str>,
        model: Option<&str>,
    ) -> ProviderResult<ProviderResponse> {
        // Build command arguments
        let mut args = vec!["-p", prompt, "--output-format", "json"];

        // Add system prompt if provided
        let system_arg;
        if let Some(sys) = system_prompt {
            system_arg = sys.to_string();
            args.push("--system-prompt");
            args.push(&system_arg);
        }

        // Add model if provided
        let model_arg;
        if let Some(m) = model {
            model_arg = m.to_string();
            args.push("--model");
            args.push(&model_arg);
        }

        // Execute CLI command
        let stdout = self.executor.execute_for_stdout(&args).await?;

        // Parse JSON array of events
        let events: Vec<ClaudeEvent> =
            serde_json::from_str(&stdout).map_err(|e| ProviderError::Provider {
                provider: "Claude".to_string(),
                message: format!("Failed to parse JSON response: {}", e),
            })?;

        // Find the "result" event
        let result_event = events
            .iter()
            .find(|e| e.event_type == "result")
            .ok_or_else(|| ProviderError::Provider {
                provider: "Claude".to_string(),
                message: "No result event found in CLI output".to_string(),
            })?;

        // Extract content from result
        let content = result_event.result.clone().unwrap_or_default();

        // Try to extract model name from modelUsage, fall back to provided model
        let response_model = result_event
            .model_usage
            .as_ref()
            .and_then(|mu| mu.as_object())
            .and_then(|obj| obj.keys().next())
            .map(|s| s.to_string())
            .unwrap_or_else(|| model.unwrap_or("claude").to_string());

        // Extract usage
        let usage = result_event.usage.as_ref().map(|u| TokenUsage {
            input_tokens: u.input_tokens,
            output_tokens: u.output_tokens,
        });

        Ok(ProviderResponse {
            content,
            model: response_model,
            usage,
        })
    }

    /// Check if Claude CLI is authenticated
    ///
    /// Attempts a simple test command to verify authentication status.
    pub async fn check_auth(&self) -> ProviderResult<bool> {
        // Try a minimal prompt to check auth
        let result = self
            .executor
            .execute(&["-p", "test", "--output-format", "json"])
            .await;

        match result {
            Ok(output) => Ok(output.success),
            Err(CliError::NotAuthenticated { .. }) => Ok(false),
            Err(e) => Err(e.into()),
        }
    }
}

/// Check if Claude CLI is available in PATH
pub fn is_available() -> bool {
    CliExecutor::detect("claude").is_ok()
}

/// Get installation instructions for Claude CLI
pub fn install_instructions() -> &'static str {
    "Install Claude Code CLI from:\n  \
     https://claude.ai/download\n\n\
     Then authenticate by running:\n  \
     claude\n\n\
     Follow the prompts to complete authentication."
}

/// Get authentication instructions for Claude CLI
pub fn auth_instructions() -> &'static str {
    "Please authenticate the Claude CLI:\n  \
     claude\n\n\
     Follow the prompts to complete authentication,\n\
     then retry your command."
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_claude_events() {
        // Actual format from Claude CLI --output-format json
        let json = r#"[
            {"type":"system","subtype":"init","session_id":"test"},
            {"type":"assistant","message":{"content":[{"type":"text","text":"Hello!"}]}},
            {"type":"result","subtype":"success","result":"Hello, world!","usage":{"input_tokens":10,"output_tokens":5},"modelUsage":{"claude-sonnet-4-5":{"inputTokens":10,"outputTokens":5}}}
        ]"#;

        let events: Vec<ClaudeEvent> = serde_json::from_str(json).unwrap();
        assert_eq!(events.len(), 3);

        // Find result event
        let result = events.iter().find(|e| e.event_type == "result").unwrap();
        assert_eq!(result.result, Some("Hello, world!".to_string()));
        assert!(result.usage.is_some());
        let usage = result.usage.as_ref().unwrap();
        assert_eq!(usage.input_tokens, Some(10));
        assert_eq!(usage.output_tokens, Some(5));
    }

    #[test]
    fn test_parse_result_event_only() {
        let json = r#"[
            {"type":"result","subtype":"success","result":"Test response"}
        ]"#;

        let events: Vec<ClaudeEvent> = serde_json::from_str(json).unwrap();
        assert_eq!(events.len(), 1);
        assert_eq!(events[0].event_type, "result");
        assert_eq!(events[0].result, Some("Test response".to_string()));
    }

    #[test]
    fn test_parse_empty_result() {
        let json = r#"[
            {"type":"result","subtype":"success"}
        ]"#;

        let events: Vec<ClaudeEvent> = serde_json::from_str(json).unwrap();
        let result = &events[0];
        assert!(result.result.is_none());
    }

    #[test]
    fn test_install_instructions() {
        let instructions = install_instructions();
        assert!(instructions.contains("claude.ai/download"));
        assert!(instructions.contains("claude"));
    }
}
