//! Gemini CLI Provider with Pipes Streaming (SPEC-KIT-952-F)
//!
//! Uses GeminiPipesProvider for session-based multi-turn conversations.
//! Replaces PTY mode with one-shot + resume pattern.
//! Uses global provider instance to maintain sessions across messages.

use codex_core::cli_executor::{
    CliError, ConversationId, GeminiPipesProvider, StreamEvent,
};
use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};
use std::sync::OnceLock;

use crate::app_event_sender::AppEventSender;
use crate::providers::{ProviderError, ProviderResult};

/// Global Gemini provider instance (shared across all messages)
static GEMINI_PROVIDER: OnceLock<GeminiPipesProvider> = OnceLock::new();

/// Get or create the global Gemini provider
fn get_gemini_provider() -> &'static GeminiPipesProvider {
    GEMINI_PROVIDER.get_or_init(|| {
        // Get actual working directory for project context
        let cwd = std::env::current_dir()
            .ok()
            .and_then(|p| p.to_str().map(String::from))
            .unwrap_or_else(|| {
                tracing::warn!("Failed to get current directory, using '.'");
                String::from(".")
            });

        tracing::info!("Initializing global Gemini pipes provider with cwd={}", cwd);
        GeminiPipesProvider::with_cwd("gemini-2.5-flash", &cwd)
    })
}

/// Gemini CLI provider with pipes streaming support (session-based)
pub struct GeminiStreamingProvider {
    // No longer stores provider - uses global instance
}

impl GeminiStreamingProvider {
    /// Create a new Gemini pipes streaming provider (uses global instance)
    pub fn new() -> ProviderResult<Self> {
        // Check if gemini CLI is available
        if !Self::is_available() {
            return Err(ProviderError::Provider {
                provider: "Gemini".to_string(),
                message: "Gemini CLI not found. Install: npm install -g @google/gemini-cli"
                    .to_string(),
            });
        }

        // Initialize global provider (happens once)
        let _ = get_gemini_provider();

        Ok(Self {})
    }

    /// Create provider with specific model
    pub fn with_model(_model: &str) -> ProviderResult<Self> {
        // Note: Global provider uses default model (gemini-2.5-flash)
        // Model-specific configuration not currently supported with global instance
        Self::new()
    }

    /// Execute prompt with streaming to AppEventSender
    ///
    /// Streams response deltas in real-time to the TUI via event sender.
    /// Accumulates full response text for conversation history.
    /// Uses session-based API for O(1) data transfer per turn.
    pub async fn execute_streaming(
        &self,
        messages: &[codex_core::context_manager::Message],
        model: &str,
        tx: AppEventSender,
    ) -> ProviderResult<String> {
        // Derive conversation ID from message history (hash of conversation)
        let conv_id = Self::derive_conversation_id(messages);

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

        // Send message via global session-based provider (creates/reuses session)
        let provider = get_gemini_provider();
        let mut rx = provider
            .send_message(conv_id, user_message)
            .await
            .map_err(|e| Self::map_cli_error(e))?;

        // Stream events to TUI and accumulate response
        let mut accumulated = String::new();
        let mut received_done = false;

        tx.send_native_stream_start("Gemini Pipes", model.to_string(), "pipes".to_string());

        while let Some(event) = rx.recv().await {
            match event {
                StreamEvent::Delta(text) => {
                    accumulated.push_str(&text);
                    tx.send_native_stream_delta(text);
                }
                StreamEvent::Done => {
                    received_done = true;
                    break;
                }
                StreamEvent::Error(e) => {
                    let error_msg = format!("{}", e);
                    tx.send_native_stream_error("Gemini Pipes", &error_msg);
                    return Err(Self::map_cli_error(e));
                }
                _ => {
                    // Ignore other events (metadata if added in future)
                }
            }
        }

        // If channel closed without Done event, something went wrong
        if !received_done && accumulated.is_empty() {
            let error_msg = "Gemini CLI process died without sending response";
            tx.send_native_stream_error("Gemini Pipes", error_msg);
            return Err(ProviderError::Provider {
                provider: "Gemini".to_string(),
                message: error_msg.to_string(),
            });
        }

        tx.send_native_stream_complete("Gemini Pipes", None, None);

        Ok(accumulated)
    }

    /// Derive a conversation ID from message history
    ///
    /// FIXED: Uses stable ID based on FIRST user message only.
    /// This ensures the same conversation reuses the same session.
    fn derive_conversation_id(messages: &[codex_core::context_manager::Message]) -> ConversationId {
        // Find first user message to use as stable anchor
        let first_user_msg = messages
            .iter()
            .find(|msg| matches!(msg.role, codex_core::context_manager::MessageRole::User))
            .and_then(|msg| {
                msg.content.iter().find_map(|block| {
                    if let codex_core::context_manager::ContentBlock::Text { text } = block {
                        Some(text.clone())
                    } else {
                        None
                    }
                })
            });

        // Hash only the first user message to create stable ID
        let mut hasher = DefaultHasher::new();
        if let Some(first_msg) = first_user_msg {
            first_msg.hash(&mut hasher);
        } else {
            // Fallback: use current timestamp (new conversation each time)
            use std::time::{SystemTime, UNIX_EPOCH};
            let timestamp = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_secs();
            timestamp.hash(&mut hasher);
        }

        format!("gemini-conv-{:x}", hasher.finish())
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

    /// Get access to the global Gemini provider (for session management)
    pub fn global_provider() -> &'static GeminiPipesProvider {
        get_gemini_provider()
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
