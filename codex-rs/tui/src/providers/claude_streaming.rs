//! Claude CLI Provider with Streaming Support (SPEC-KIT-952 Phase 1)
//!
//! Uses codex_core::cli_executor for streaming CLI routing.
//! Provides fallback/alternative to native API approach.

use codex_core::cli_executor::{
    ClaudeCliConfig, ClaudeCliExecutor, CliError, CliExecutor, Conversation, Message, Role,
    StreamEvent,
};
use futures::StreamExt;
use std::time::Duration;
use tokio::sync::mpsc;

use crate::app_event_sender::AppEventSender;
use crate::providers::{ProviderError, ProviderResult};

/// Claude CLI provider with streaming support
pub struct ClaudeStreamingProvider {
    executor: ClaudeCliExecutor,
}

impl ClaudeStreamingProvider {
    /// Create a new Claude streaming provider
    pub fn new() -> ProviderResult<Self> {
        // Check if claude CLI is available
        if !Self::is_available() {
            return Err(ProviderError::Provider {
                provider: "Claude".to_string(),
                message: "Claude CLI not found. Install from https://claude.ai/download"
                    .to_string(),
            });
        }

        let config = ClaudeCliConfig::default();
        let executor = ClaudeCliExecutor::new(config);

        Ok(Self { executor })
    }

    /// Create provider with custom timeout
    pub fn with_timeout(timeout_secs: u64) -> ProviderResult<Self> {
        if !Self::is_available() {
            return Err(ProviderError::Provider {
                provider: "Claude".to_string(),
                message: "Claude CLI not found".to_string(),
            });
        }

        let config = ClaudeCliConfig {
            timeout_secs,
            ..Default::default()
        };
        let executor = ClaudeCliExecutor::new(config);

        Ok(Self { executor })
    }

    /// Execute prompt with streaming to AppEventSender
    ///
    /// Streams response deltas in real-time to the TUI via event sender.
    /// Accumulates full response text for conversation history.
    pub async fn execute_streaming(
        &self,
        messages: &[codex_core::context_manager::Message],
        model: &str,
        tx: AppEventSender,
    ) -> ProviderResult<String> {
        // Convert messages to cli_executor format
        let conversation = Self::convert_messages(messages, model);

        // Get last message as current user message (extract text from ContentBlocks)
        let user_message = messages
            .last()
            .map(|m| {
                m.content
                    .iter()
                    .filter_map(|block| {
                        if let codex_core::context_manager::ContentBlock::Text { text } = block {
                            Some(text.clone())
                        } else {
                            None
                        }
                    })
                    .collect::<Vec<_>>()
                    .join("\n")
            })
            .unwrap_or_default();

        // Execute with streaming
        let mut rx = self
            .executor
            .execute(&conversation, &user_message)
            .await
            .map_err(|e| Self::map_cli_error(e))?;

        // Stream events to TUI and accumulate response
        let mut accumulated = String::new();
        let mut input_tokens = None;
        let mut output_tokens = None;

        tx.send_native_stream_start("Claude CLI", model.to_string(), "cli".to_string());

        while let Some(event) = rx.recv().await {
            match event {
                StreamEvent::Delta(text) => {
                    accumulated.push_str(&text);
                    tx.send_native_stream_delta(text);
                }
                StreamEvent::Metadata(metadata) => {
                    input_tokens = metadata.input_tokens;
                    output_tokens = metadata.output_tokens;
                }
                StreamEvent::Done => {
                    break;
                }
                StreamEvent::Error(e) => {
                    let error_msg = format!("{}", e);
                    tx.send_native_stream_error("Claude CLI", &error_msg);
                    return Err(Self::map_cli_error(e));
                }
            }
        }

        tx.send_native_stream_complete(
            "Claude CLI",
            input_tokens.map(|n| n as u32),
            output_tokens.map(|n| n as u32),
        );

        Ok(accumulated)
    }

    /// Convert context_manager messages to cli_executor format
    fn convert_messages(
        messages: &[codex_core::context_manager::Message],
        model: &str,
    ) -> Conversation {
        let mut conversation_messages = Vec::new();
        let mut system_prompt = None;

        for msg in messages {
            // Extract text content from ContentBlocks
            let content_text = msg
                .content
                .iter()
                .filter_map(|block| {
                    if let codex_core::context_manager::ContentBlock::Text { text } = block {
                        Some(text.clone())
                    } else {
                        None
                    }
                })
                .collect::<Vec<_>>()
                .join("\n");

            let role = match msg.role {
                codex_core::context_manager::MessageRole::System => {
                    // Extract first system message as system prompt
                    if system_prompt.is_none() {
                        system_prompt = Some(content_text);
                    }
                    continue; // Don't add to message history
                }
                codex_core::context_manager::MessageRole::User => Role::User,
                codex_core::context_manager::MessageRole::Assistant => Role::Assistant,
            };

            conversation_messages.push(Message {
                role,
                content: content_text,
                timestamp: None, // Will be added by context manager
            });
        }

        // Map preset model name to actual API model name
        let api_model = Self::map_model_name(model);

        Conversation {
            messages: conversation_messages,
            system_prompt,
            model: api_model.to_string(),
        }
    }

    /// Map preset model name to actual Claude API model name
    fn map_model_name(preset: &str) -> &str {
        let preset_lower = preset.to_ascii_lowercase();

        if preset_lower.contains("opus") {
            "claude-opus-4-1-20250805"
        } else if preset_lower.contains("sonnet") {
            "claude-sonnet-4-5-20250929"
        } else if preset_lower.contains("haiku") {
            "claude-haiku-4-5-20251001"
        } else {
            // Default to sonnet if unknown
            "claude-sonnet-4-5-20250929"
        }
    }

    /// Map CliError to ProviderError
    fn map_cli_error(e: CliError) -> ProviderError {
        match e {
            CliError::BinaryNotFound { binary, install_hint } => ProviderError::Provider {
                provider: "Claude".to_string(),
                message: format!("{} not found. {}", binary, install_hint),
            },
            CliError::NotAuthenticated { auth_command, .. } => ProviderError::Provider {
                provider: "Claude".to_string(),
                message: format!("Not authenticated. Run: {}", auth_command),
            },
            CliError::ProcessFailed { stderr, .. } => ProviderError::Provider {
                provider: "Claude".to_string(),
                message: stderr,
            },
            CliError::Timeout { elapsed } => ProviderError::Provider {
                provider: "Claude".to_string(),
                message: format!("Request timed out after {:?}", elapsed),
            },
            CliError::ParseError { details } => ProviderError::Provider {
                provider: "Claude".to_string(),
                message: format!("Parse error: {}", details),
            },
            CliError::Internal { message } => ProviderError::Provider {
                provider: "Claude".to_string(),
                message,
            },
        }
    }

    /// Check if Claude CLI is available
    pub fn is_available() -> bool {
        // Simple check: try to find claude in PATH
        which::which("claude").is_ok()
    }

    /// Get install instructions
    pub fn install_instructions() -> &'static str {
        "Install Claude CLI from:\n  \
         https://claude.ai/download\n\n\
         Then authenticate by running:\n  \
         claude\n\n\
         Follow the prompts to complete authentication."
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_is_available() {
        // This will only pass if claude CLI is installed
        let available = ClaudeStreamingProvider::is_available();
        if available {
            println!("Claude CLI is available");
        } else {
            println!("Claude CLI not found (expected in CI)");
        }
    }

    #[test]
    fn test_install_instructions() {
        let instructions = ClaudeStreamingProvider::install_instructions();
        assert!(instructions.contains("claude.ai/download"));
        assert!(instructions.contains("claude"));
    }
}
