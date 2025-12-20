//! Claude CLI Provider with Streaming Support (SPEC-KIT-952 Phase 1)
//!
//! Uses codex_core::cli_executor for streaming CLI routing.
//! Provides fallback/alternative to native API approach.
//!
//! UPDATED: Now uses ClaudePipesProvider for session-based multi-turn conversations.
//! Maintains per-model provider instances to honor model selection from UI.

#![allow(dead_code)] // Streaming provider helpers

use codex_core::cli_executor::{ClaudePipesProvider, CliError, ConversationId, StreamEvent};
use std::collections::HashMap;
use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};
use std::sync::{Arc, Mutex, OnceLock};

use crate::app_event_sender::AppEventSender;
use crate::providers::{ProviderError, ProviderResult};

/// Per-model Claude provider cache (model_name -> provider)
/// Empty string key represents default model (CLI's default behavior)
static CLAUDE_PROVIDERS: OnceLock<Mutex<HashMap<String, Arc<ClaudePipesProvider>>>> =
    OnceLock::new();

/// Get or create a Claude provider for the specified model
fn get_claude_provider_for_model(model: &str) -> Arc<ClaudePipesProvider> {
    let providers = CLAUDE_PROVIDERS.get_or_init(|| Mutex::new(HashMap::new()));
    let mut cache = providers.lock().unwrap();

    // Normalize model name (empty string = default)
    let model_key = model.to_string();

    if let Some(provider) = cache.get(&model_key) {
        return Arc::clone(provider);
    }

    // Create new provider for this model
    let cwd = std::env::current_dir()
        .ok()
        .and_then(|p| p.to_str().map(String::from))
        .unwrap_or_else(|| {
            tracing::warn!("Failed to get current directory, using '.'");
            String::from(".")
        });

    tracing::info!(
        "Creating Claude pipes provider for model='{}' with cwd={}",
        if model.is_empty() { "(default)" } else { model },
        cwd
    );

    let provider = Arc::new(ClaudePipesProvider::with_cwd(model, &cwd));
    cache.insert(model_key, Arc::clone(&provider));
    provider
}

/// Get the default Claude provider (for backwards compatibility)
fn get_claude_provider() -> Arc<ClaudePipesProvider> {
    get_claude_provider_for_model("")
}

/// Claude CLI provider with streaming support (session-based)
pub struct ClaudeStreamingProvider {
    /// Model to use (empty string = CLI default)
    model: String,
}

impl ClaudeStreamingProvider {
    /// Create a new Claude streaming provider with default model
    pub fn new() -> ProviderResult<Self> {
        Self::with_model("")
    }

    /// Create provider with specific model
    pub fn with_model(model: &str) -> ProviderResult<Self> {
        // Check if claude CLI is available
        if !Self::is_available() {
            return Err(ProviderError::Provider {
                provider: "Claude".to_string(),
                message: "Claude CLI not found. Install from https://claude.ai/download"
                    .to_string(),
            });
        }

        // Initialize provider for this model (cached per-model)
        let _ = get_claude_provider_for_model(model);

        Ok(Self {
            model: model.to_string(),
        })
    }

    /// Execute prompt with streaming to AppEventSender
    ///
    /// Streams response deltas in real-time to the TUI via event sender.
    /// Accumulates full response text for conversation history.
    /// Uses session-based API for O(1) data transfer per turn.
    ///
    /// The `model` parameter allows overriding the model for this specific call.
    /// If empty, uses the model configured when the provider was created.
    pub async fn execute_streaming(
        &self,
        prompt: &str,
        model: &str,
        tx: AppEventSender,
    ) -> ProviderResult<String> {
        // Use provided model or fall back to instance model
        let effective_model = if model.is_empty() { &self.model } else { model };

        // Derive conversation ID from prompt hash
        let conv_id = Self::derive_conversation_id(prompt);

        // Generate unique message ID per turn
        use std::time::{SystemTime, UNIX_EPOCH};
        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_millis();
        let message_id = format!("{}-msg{}", conv_id, timestamp);

        // Send message via model-specific provider (creates/reuses per-model session)
        let provider = get_claude_provider_for_model(effective_model);
        let mut rx = provider
            .send_message(conv_id, prompt.to_string())
            .await
            .map_err(|e| Self::map_cli_error(e))?;

        // Stream events to TUI and accumulate response
        let mut accumulated = String::new();
        let mut input_tokens = None;
        let mut output_tokens = None;
        let mut received_done = false;

        tx.send_native_stream_start("Claude Pipes", model.to_string(), message_id);

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
                    received_done = true;
                    break;
                }
                StreamEvent::Error(e) => {
                    let error_msg = format!("{}", e);
                    tx.send_native_stream_error("Claude Pipes", &error_msg);
                    return Err(Self::map_cli_error(e));
                }
            }
        }

        // If channel closed without Done event, something went wrong
        if !received_done && accumulated.is_empty() {
            let error_msg = "Claude CLI process died without sending response";
            tx.send_native_stream_error("Claude Pipes", error_msg);
            return Err(ProviderError::Provider {
                provider: "Claude".to_string(),
                message: error_msg.to_string(),
            });
        }

        tx.send_native_stream_complete(
            "Claude Pipes",
            input_tokens.map(|n| n as u32),
            output_tokens.map(|n| n as u32),
        );

        Ok(accumulated)
    }

    /// Derive a conversation ID from prompt
    ///
    /// Uses hash of prompt to create conversation ID.
    fn derive_conversation_id(prompt: &str) -> ConversationId {
        let mut hasher = DefaultHasher::new();
        prompt.hash(&mut hasher);
        format!("claude-conv-{:x}", hasher.finish())
    }

    /// Map CliError to ProviderError
    fn map_cli_error(e: CliError) -> ProviderError {
        match e {
            CliError::BinaryNotFound {
                binary,
                install_hint,
            } => ProviderError::Provider {
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

    /// Get access to the default Claude provider (for session management)
    pub fn global_provider() -> Arc<ClaudePipesProvider> {
        get_claude_provider()
    }

    /// Get access to a model-specific Claude provider
    pub fn provider_for_model(model: &str) -> Arc<ClaudePipesProvider> {
        get_claude_provider_for_model(model)
    }
}

#[cfg(test)]
#[allow(clippy::print_stdout)]
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
