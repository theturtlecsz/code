//! Gemini CLI Provider with Pipes Streaming (SPEC-KIT-952-F)
//!
//! Uses GeminiPipesProvider for session-based multi-turn conversations.
//! Replaces PTY mode with one-shot + resume pattern.
//! Maintains per-model provider instances to honor model selection from UI.

#![allow(dead_code)] // Streaming provider helpers

use codex_core::cli_executor::{CliError, ConversationId, GeminiPipesProvider, StreamEvent};
use std::collections::HashMap;
use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};
use std::sync::{Arc, Mutex, OnceLock};

use crate::app_event_sender::AppEventSender;
use crate::providers::{ProviderError, ProviderResult};

/// Default model when none specified
const DEFAULT_GEMINI_MODEL: &str = "gemini-2.5-flash";

/// Per-model Gemini provider cache (model_name -> provider)
static GEMINI_PROVIDERS: OnceLock<Mutex<HashMap<String, Arc<GeminiPipesProvider>>>> =
    OnceLock::new();

/// Get or create a Gemini provider for the specified model
fn get_gemini_provider_for_model(model: &str) -> Arc<GeminiPipesProvider> {
    let providers = GEMINI_PROVIDERS.get_or_init(|| Mutex::new(HashMap::new()));
    let mut cache = providers.lock().unwrap();

    // Use default model if empty
    let effective_model = if model.is_empty() {
        DEFAULT_GEMINI_MODEL
    } else {
        model
    };

    // Map preset names to actual API model names
    let mapped_model = GeminiStreamingProvider::map_model_name(effective_model);
    let model_key = mapped_model.to_string();

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
        "Creating Gemini pipes provider for model='{}' (mapped from '{}') with cwd={}",
        mapped_model,
        effective_model,
        cwd
    );

    let provider = Arc::new(GeminiPipesProvider::with_cwd(mapped_model, &cwd));
    cache.insert(model_key, Arc::clone(&provider));
    provider
}

/// Get the default Gemini provider (for backwards compatibility)
fn get_gemini_provider() -> Arc<GeminiPipesProvider> {
    get_gemini_provider_for_model(DEFAULT_GEMINI_MODEL)
}

/// Gemini CLI provider with pipes streaming support (session-based)
pub struct GeminiStreamingProvider {
    /// Model to use (empty string = default)
    model: String,
}

impl GeminiStreamingProvider {
    /// Create a new Gemini pipes streaming provider with default model
    pub fn new() -> ProviderResult<Self> {
        Self::with_model(DEFAULT_GEMINI_MODEL)
    }

    /// Create provider with specific model
    pub fn with_model(model: &str) -> ProviderResult<Self> {
        // Check if gemini CLI is available
        if !Self::is_available() {
            return Err(ProviderError::Provider {
                provider: "Gemini".to_string(),
                message: "Gemini CLI not found. Install: npm install -g @google/gemini-cli"
                    .to_string(),
            });
        }

        // Initialize provider for this model (cached per-model)
        let _ = get_gemini_provider_for_model(model);

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

        // Send message via model-specific provider (creates/reuses per-model session)
        let provider = get_gemini_provider_for_model(effective_model);
        let mut rx = provider
            .send_message(conv_id, prompt.to_string())
            .await
            .map_err(|e| Self::map_cli_error(e))?;

        // Stream events to TUI and accumulate response
        let mut accumulated = String::new();
        let mut received_done = false;

        // Generate unique message ID per turn
        use std::time::{SystemTime, UNIX_EPOCH};
        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_millis();
        let message_id = format!("gemini-msg{}", timestamp);

        tx.send_native_stream_start("Gemini Pipes", effective_model.to_string(), message_id);

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

    /// Derive a conversation ID from prompt
    ///
    /// Uses hash of prompt to create conversation ID.
    fn derive_conversation_id(prompt: &str) -> ConversationId {
        let mut hasher = DefaultHasher::new();
        prompt.hash(&mut hasher);
        format!("gemini-conv-{:x}", hasher.finish())
    }

    /// Map preset model name to actual Gemini API model name
    pub fn map_model_name(preset: &str) -> &str {
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
            CliError::BinaryNotFound {
                binary,
                install_hint,
            } => ProviderError::Provider {
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

    /// Get access to the default Gemini provider (for session management)
    pub fn global_provider() -> Arc<GeminiPipesProvider> {
        get_gemini_provider()
    }

    /// Get access to a model-specific Gemini provider
    pub fn provider_for_model(model: &str) -> Arc<GeminiPipesProvider> {
        get_gemini_provider_for_model(model)
    }
}

#[cfg(test)]
#[allow(clippy::print_stdout)]
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

    #[test]
    fn test_model_mapping() {
        assert_eq!(
            GeminiStreamingProvider::map_model_name("gemini-3-pro"),
            "gemini-3-pro-preview"
        );
        assert_eq!(
            GeminiStreamingProvider::map_model_name("gemini-2.5-flash"),
            "gemini-2.5-flash"
        );
        assert_eq!(
            GeminiStreamingProvider::map_model_name("gemini-2.5-pro"),
            "gemini-2.5-pro"
        );
        // Unknown model returns as-is
        assert_eq!(
            GeminiStreamingProvider::map_model_name("custom-model"),
            "custom-model"
        );
    }
}
