//! Gemini CLI Provider with Streaming Support (SPEC-KIT-952 Phase 1)
//!
//! Uses codex_core::cli_executor for streaming CLI routing.
//! Provides fallback/alternative to native API approach.

use codex_core::cli_executor::{
    CliError, CliExecutor, Conversation, GeminiCliConfig, GeminiCliExecutor, Message, Role,
    StreamEvent,
};
use futures::StreamExt;
use std::time::Duration;
use tokio::sync::mpsc;

use crate::app_event_sender::AppEventSender;
use crate::providers::{ProviderError, ProviderResult};

/// Gemini CLI provider with streaming support
pub struct GeminiStreamingProvider {
    executor: GeminiCliExecutor,
}

impl GeminiStreamingProvider {
    /// Create a new Gemini streaming provider
    pub fn new() -> ProviderResult<Self> {
        // Check if gemini CLI is available
        if !Self::is_available() {
            return Err(ProviderError::Provider {
                provider: "Gemini".to_string(),
                message: "Gemini CLI not found. Install: npm install -g @google/gemini-cli"
                    .to_string(),
            });
        }

        let config = GeminiCliConfig::default();
        let executor = GeminiCliExecutor::new(config);

        Ok(Self { executor })
    }

    /// Create provider with custom timeout
    pub fn with_timeout(timeout_secs: u64) -> ProviderResult<Self> {
        if !Self::is_available() {
            return Err(ProviderError::Provider {
                provider: "Gemini".to_string(),
                message: "Gemini CLI not found".to_string(),
            });
        }

        let config = GeminiCliConfig {
            timeout_secs,
            ..Default::default()
        };
        let executor = GeminiCliExecutor::new(config);

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

        tx.send_native_stream_start("Gemini CLI", model.to_string(), "cli".to_string());

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
                    tx.send_native_stream_error("Gemini CLI", &error_msg);
                    return Err(Self::map_cli_error(e));
                }
            }
        }

        tx.send_native_stream_complete(
            "Gemini CLI",
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

    /// Map preset model name to actual Gemini API model name
    fn map_model_name(preset: &str) -> &str {
        let preset_lower = preset.to_ascii_lowercase();

        if preset_lower.contains("3-pro") {
            // gemini-3-pro → gemini-3-pro-preview (actual API model name)
            "gemini-3-pro-preview"
        } else if preset_lower.contains("2.5-flash-lite") {
            "gemini-2.5-flash-lite"
        } else if preset_lower.contains("2.5-flash") {
            "gemini-2.5-flash"
        } else if preset_lower.contains("2.5-pro") {
            "gemini-2.5-pro"
        } else {
            // Return preset as-is if unknown (CLI might handle it)
            preset
        }
    }

    /// Map CliError to ProviderError
    fn map_cli_error(e: CliError) -> ProviderError {
        match e {
            CliError::BinaryNotFound { binary, install_hint } => ProviderError::Provider {
                provider: "Gemini".to_string(),
                message: format!("{} not found. {}", binary, install_hint),
            },
            CliError::NotAuthenticated { auth_command, .. } => ProviderError::Provider {
                provider: "Gemini".to_string(),
                message: format!("Not authenticated. Run: {}", auth_command),
            },
            CliError::ProcessFailed { stderr, .. } => ProviderError::Provider {
                provider: "Gemini".to_string(),
                message: stderr,
            },
            CliError::Timeout { elapsed } => ProviderError::Provider {
                provider: "Gemini".to_string(),
                message: format!("Request timed out after {:?}", elapsed),
            },
            CliError::ParseError { details } => ProviderError::Provider {
                provider: "Gemini".to_string(),
                message: format!("Parse error: {}", details),
            },
            CliError::Internal { message } => ProviderError::Provider {
                provider: "Gemini".to_string(),
                message,
            },
        }
    }

    /// Check if Gemini CLI is available
    pub fn is_available() -> bool {
        // Simple check: try to find gemini in PATH
        which::which("gemini").is_ok()
    }

    /// Get install instructions
    pub fn install_instructions() -> &'static str {
        "Install Gemini CLI:\n  \
         npm install -g @google/gemini-cli\n\n\
         Then authenticate by running:\n  \
         gemini\n\n\
         Follow the OAuth prompts to complete authentication."
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_is_available() {
        // This will only pass if gemini CLI is installed
        let available = GeminiStreamingProvider::is_available();
        if available {
            println!("Gemini CLI is available");
        } else {
            println!("Gemini CLI not found (expected in CI)");
        }
    }

    #[test]
    fn test_install_instructions() {
        let instructions = GeminiStreamingProvider::install_instructions();
        assert!(instructions.contains("npm install"));
        assert!(instructions.contains("gemini"));
    }
}
