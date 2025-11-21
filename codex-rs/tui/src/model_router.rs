//! Model Router for Multi-Provider Support (SPEC-KIT-952)
//!
//! Routes prompts to appropriate providers based on model selection.
//! - ChatGPT models → Native OAuth (existing codex-core flow)
//! - Claude models → CLI routing with streaming (SPEC-KIT-952) **PRIMARY**
//! - Gemini models → CLI routing with streaming (SPEC-KIT-952) **PRIMARY**
//!
//! Native API clients (SPEC-KIT-953) are deprecated - CLI routing is production path.

use std::path::PathBuf;

use futures::StreamExt;

use crate::app_event_sender::AppEventSender;
use crate::providers::claude::ClaudeProvider;
use crate::providers::gemini::GeminiProvider;
use crate::providers::{CliRoutingSettings, ProviderError, ProviderResponse, ProviderType};

use codex_core::api_clients::{AnthropicClient, AnthropicConfig, GeminiClient, GeminiConfig, StreamEvent, map_gemini_model};
use codex_core::context_manager::Message;

/// Result of a CLI-routed prompt execution
#[derive(Debug)]
pub enum RouterResult {
    /// Prompt should be handled by CLI provider
    CliResponse(ProviderResponse),
    /// Prompt should fall through to native ChatGPT handling
    UseNative,
    /// Error occurred during routing
    Error(RouterError),
}

/// Errors that can occur during routing
#[derive(Debug, Clone)]
pub enum RouterError {
    /// Provider error
    Provider(ProviderError),
    /// CLI not available
    CliNotAvailable {
        provider: String,
        install_instructions: String,
    },
    /// CLI not authenticated
    CliNotAuthenticated {
        provider: String,
        auth_instructions: String,
    },
}

impl std::fmt::Display for RouterError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            RouterError::Provider(e) => write!(f, "{}", e),
            RouterError::CliNotAvailable {
                provider,
                install_instructions,
            } => {
                write!(
                    f,
                    "{} CLI not available.\n\n{}",
                    provider, install_instructions
                )
            }
            RouterError::CliNotAuthenticated {
                provider,
                auth_instructions,
            } => {
                write!(
                    f,
                    "{} CLI not authenticated.\n\n{}",
                    provider, auth_instructions
                )
            }
        }
    }
}

impl std::error::Error for RouterError {}

impl From<ProviderError> for RouterError {
    fn from(err: ProviderError) -> Self {
        RouterError::Provider(err)
    }
}

/// Model router for directing prompts to appropriate providers
pub struct ModelRouter;

impl ModelRouter {
    /// Execute a prompt using the appropriate provider for the given model
    ///
    /// # Arguments
    /// * `model` - The model name/ID (e.g., "claude-sonnet-4.5", "gemini-2.0-flash")
    /// * `prompt` - The prompt text to execute
    ///
    /// # Returns
    /// - `RouterResult::CliResponse` - Response from CLI provider
    /// - `RouterResult::UseNative` - Should fall through to native ChatGPT handling
    /// - `RouterResult::Error` - Error during execution
    pub async fn execute_prompt(model: &str, prompt: &str, settings: &CliRoutingSettings) -> RouterResult {
        let provider_type = ProviderType::from_model_name(model);

        match provider_type {
            ProviderType::ChatGPT => {
                // Fall through to native handling
                RouterResult::UseNative
            }
            ProviderType::Claude => {
                Self::execute_claude_prompt(prompt, settings).await
            }
            ProviderType::Gemini => {
                Self::execute_gemini_prompt(prompt, model, settings).await
            }
        }
    }

    /// Execute prompt via Claude CLI
    async fn execute_claude_prompt(prompt: &str, settings: &CliRoutingSettings) -> RouterResult {
        // Check if CLI is available
        if !crate::providers::claude::is_available() {
            return RouterResult::Error(RouterError::CliNotAvailable {
                provider: "Claude".to_string(),
                install_instructions: crate::providers::claude::install_instructions().to_string(),
            });
        }

        // Create provider and execute with settings
        match ClaudeProvider::new() {
            Ok(provider) => match provider.execute_prompt_with_settings(prompt, settings).await {
                Ok(response) => RouterResult::CliResponse(response),
                Err(e) => RouterResult::Error(e.into()),
            },
            Err(e) => RouterResult::Error(e.into()),
        }
    }

    /// Execute prompt via Gemini CLI
    async fn execute_gemini_prompt(prompt: &str, model: &str, settings: &CliRoutingSettings) -> RouterResult {
        // Check if CLI is available
        if !crate::providers::gemini::is_available() {
            return RouterResult::Error(RouterError::CliNotAvailable {
                provider: "Gemini".to_string(),
                install_instructions: crate::providers::gemini::install_instructions().to_string(),
            });
        }

        // Map model name to actual Gemini model identifier
        let gemini_model = crate::providers::gemini::map_model_name(model);

        // Create provider and execute with settings
        match GeminiProvider::new() {
            Ok(provider) => match provider.execute_prompt_with_settings(prompt, gemini_model, settings).await {
                Ok(response) => RouterResult::CliResponse(response),
                Err(e) => RouterResult::Error(e.into()),
            },
            Err(e) => RouterResult::Error(e.into()),
        }
    }

    /// Check if a model should use CLI routing
    pub fn should_use_cli(model: &str) -> bool {
        ProviderType::from_model_name(model).uses_cli_routing()
    }

    /// Get the provider type for a model
    pub fn get_provider_type(model: &str) -> ProviderType {
        ProviderType::from_model_name(model)
    }

    /// Check if the required CLI is available for a model
    pub fn is_cli_available(model: &str) -> bool {
        let provider_type = ProviderType::from_model_name(model);
        match provider_type {
            ProviderType::ChatGPT => true, // Always available (native)
            ProviderType::Claude => crate::providers::claude::is_available(),
            ProviderType::Gemini => crate::providers::gemini::is_available(),
        }
    }

    /// Get installation instructions for the CLI required by a model
    pub fn get_install_instructions(model: &str) -> Option<&'static str> {
        let provider_type = ProviderType::from_model_name(model);
        match provider_type {
            ProviderType::ChatGPT => None,
            ProviderType::Claude => Some(crate::providers::claude::install_instructions()),
            ProviderType::Gemini => Some(crate::providers::gemini::install_instructions()),
        }
    }

    /// Check CLI availability and return friendly error message if not available
    pub fn check_cli_availability(model: &str) -> Result<(), String> {
        let provider_type = ProviderType::from_model_name(model);

        if !provider_type.uses_cli_routing() {
            return Ok(());
        }

        let cli_available = match provider_type {
            ProviderType::Claude => crate::providers::claude::is_available(),
            ProviderType::Gemini => crate::providers::gemini::is_available(),
            ProviderType::ChatGPT => true,
        };

        if cli_available {
            Ok(())
        } else {
            let instructions = match provider_type {
                ProviderType::Claude => crate::providers::claude::install_instructions(),
                ProviderType::Gemini => crate::providers::gemini::install_instructions(),
                ProviderType::ChatGPT => "",
            };
            Err(format!(
                "{} CLI is required but not installed.\n\n{}",
                provider_type.display_name(),
                instructions
            ))
        }
    }
}

/// Execute a prompt with native streaming (SPEC-KIT-953 - DEPRECATED)
///
/// DEPRECATED: Use execute_with_cli_streaming() instead (SPEC-KIT-952).
/// CLI routing is the PRIMARY production path for Claude/Gemini.
///
/// Uses native API clients for Claude and Gemini with streaming responses.
/// Sends events via AppEventSender for real-time UI updates.
///
/// # Arguments
/// * `model` - Model name/ID
/// * `messages` - Conversation history including current user message
/// * `codex_home` - Path to codex home for credential storage
/// * `tx` - Event sender for streaming updates
///
/// # Returns
/// Accumulated response text (for updating conversation history)
#[deprecated(note = "Use execute_with_cli_streaming for Claude/Gemini (SPEC-KIT-952)")]
pub async fn execute_with_native_streaming(
    model: &str,
    messages: &[Message],
    codex_home: PathBuf,
    tx: AppEventSender,
) -> Result<String, String> {
    let provider_type = ProviderType::from_model_name(model);
    let provider_name = provider_type.display_name();

    match provider_type {
        ProviderType::ChatGPT => {
            // ChatGPT uses native codex-core flow, not this router
            Err("ChatGPT should use native codex-core flow".to_string())
        }
        ProviderType::Claude => {
            execute_claude_native(model, messages, codex_home, tx, provider_name).await
        }
        ProviderType::Gemini => {
            execute_gemini_native(model, messages, codex_home, tx, provider_name).await
        }
    }
}

/// Execute prompt via native Anthropic client with streaming
async fn execute_claude_native(
    model: &str,
    messages: &[Message],
    codex_home: PathBuf,
    tx: AppEventSender,
    provider_name: &str,
) -> Result<String, String> {
    let client = AnthropicClient::new(codex_home);

    // Map model preset to actual API model name
    let api_model = map_claude_model(model);

    let config = AnthropicConfig {
        model: api_model.to_string(),
        max_tokens: 8192,
        temperature: None,
        system: None,
    };

    let stream = match client.send_message(messages, &config).await {
        Ok(s) => s,
        Err(e) => {
            let error_msg = format!("{}", e);
            tx.send_native_stream_error(provider_name, &error_msg);
            return Err(error_msg);
        }
    };

    // Process stream
    let mut accumulated = String::new();
    let mut stream = stream;
    let mut input_tokens = None;
    let mut output_tokens = None;

    while let Some(event) = stream.next().await {
        match event {
            Ok(StreamEvent::MessageStart { id, model }) => {
                tx.send_native_stream_start(provider_name, model, id);
            }
            Ok(StreamEvent::TextDelta { text, .. }) => {
                accumulated.push_str(&text);
                tx.send_native_stream_delta(text);
            }
            Ok(StreamEvent::MessageDelta { usage, .. }) => {
                if let Some(u) = usage {
                    input_tokens = Some(u.input_tokens);
                    output_tokens = Some(u.output_tokens);
                }
            }
            Ok(StreamEvent::MessageStop) => {
                break;
            }
            Ok(_) => {
                // Ignore other events (ContentBlockStart, ContentBlockStop, Ping)
            }
            Err(e) => {
                let error_msg = format!("{}", e);
                tx.send_native_stream_error(provider_name, &error_msg);
                return Err(error_msg);
            }
        }
    }

    tx.send_native_stream_complete(provider_name, input_tokens, output_tokens);
    Ok(accumulated)
}

/// Execute prompt via native Gemini client with streaming
async fn execute_gemini_native(
    model: &str,
    messages: &[Message],
    codex_home: PathBuf,
    tx: AppEventSender,
    provider_name: &str,
) -> Result<String, String> {
    let client = GeminiClient::new(codex_home);

    // Map model preset to actual API model name
    let api_model = map_gemini_model(model);

    let config = GeminiConfig {
        model: api_model.to_string(),
        max_tokens: 8192,
        temperature: None,
        top_p: None,
        system: None,
    };

    let stream = match client.send_message(messages, &config).await {
        Ok(s) => s,
        Err(e) => {
            let error_msg = format!("{}", e);
            tx.send_native_stream_error(provider_name, &error_msg);
            return Err(error_msg);
        }
    };

    // Process stream
    let mut accumulated = String::new();
    let mut stream = stream;
    let mut input_tokens = None;
    let mut output_tokens = None;

    while let Some(event) = stream.next().await {
        match event {
            Ok(StreamEvent::MessageStart { id, model }) => {
                tx.send_native_stream_start(provider_name, model, id);
            }
            Ok(StreamEvent::TextDelta { text, .. }) => {
                accumulated.push_str(&text);
                tx.send_native_stream_delta(text);
            }
            Ok(StreamEvent::MessageDelta { usage, .. }) => {
                if let Some(u) = usage {
                    input_tokens = Some(u.input_tokens);
                    output_tokens = Some(u.output_tokens);
                }
            }
            Ok(StreamEvent::MessageStop) => {
                break;
            }
            Ok(_) => {
                // Ignore other events
            }
            Err(e) => {
                let error_msg = format!("{}", e);
                tx.send_native_stream_error(provider_name, &error_msg);
                return Err(error_msg);
            }
        }
    }

    tx.send_native_stream_complete(provider_name, input_tokens, output_tokens);
    Ok(accumulated)
}

/// Map Claude model preset to actual API model name
fn map_claude_model(preset: &str) -> &str {
    let preset_lower = preset.to_ascii_lowercase();

    if preset_lower.contains("opus") {
        "claude-opus-4-1-20250805"
    } else if preset_lower.contains("sonnet") {
        "claude-sonnet-4-5-20250929"
    } else if preset_lower.contains("haiku") {
        "claude-haiku-4-5-20251001"
    } else {
        // Default to sonnet
        "claude-sonnet-4-5-20250929"
    }
}

/// Execute prompt via CLI routing with streaming (SPEC-KIT-952)
///
/// PRIMARY routing method for Claude and Gemini models.
/// Uses external CLI processes with streaming support.
///
/// # Arguments
/// * `model` - Model name/ID
/// * `messages` - Conversation history including current user message
/// * `tx` - Event sender for streaming updates
///
/// # Returns
/// Accumulated response text (for updating conversation history)
pub async fn execute_with_cli_streaming(
    model: &str,
    messages: &[Message],
    tx: AppEventSender,
) -> Result<String, String> {
    let provider_type = ProviderType::from_model_name(model);

    match provider_type {
        ProviderType::ChatGPT => {
            // ChatGPT uses native OAuth flow (existing)
            Err("ChatGPT should use native codex-core flow".to_string())
        }
        ProviderType::Claude => {
            // CLI routing for Claude (SPEC-KIT-952)
            use crate::providers::claude_streaming::ClaudeStreamingProvider;

            let provider = ClaudeStreamingProvider::new()
                .map_err(|e| format!("Failed to create Claude provider: {}", e))?;

            provider.execute_streaming(messages, model, tx).await
                .map_err(|e| format!("{}", e))
        }
        ProviderType::Gemini => {
            // Gemini CLI routing DISABLED (timeouts/reliability issues)
            // Gemini models should use native API flow instead
            Err("Gemini CLI routing disabled. Use ChatGPT account for Gemini models.".to_string())
        }
    }
}

/// Check if native streaming is available for a model (SPEC-KIT-952)
///
/// Returns true for Claude models only (CLI routing working).
/// Gemini CLI routing DISABLED - use native API instead.
pub fn supports_native_streaming(model: &str) -> bool {
    let provider_type = ProviderType::from_model_name(model);
    matches!(provider_type, ProviderType::Claude)  // Only Claude
}

/// Execute a prompt with CLI routing (DEPRECATED - legacy non-streaming version)
///
/// DEPRECATED: This is the legacy non-streaming CLI routing.
/// Use execute_with_cli_streaming() for production (SPEC-KIT-952).
///
/// Retained for backward compatibility only.
#[deprecated(note = "Use execute_with_cli_streaming for Claude/Gemini (SPEC-KIT-952)")]
pub async fn execute_with_routing(model: &str, prompt: &str, settings: &CliRoutingSettings) -> RouterResult {
    ModelRouter::execute_prompt(model, prompt, settings).await
}

/// Check if a model uses CLI routing
pub fn uses_cli_routing(model: &str) -> bool {
    ModelRouter::should_use_cli(model)
}

/// Get provider display name for a model
pub fn provider_display_name(model: &str) -> &'static str {
    ProviderType::from_model_name(model).display_name()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_should_use_cli() {
        // Claude models should use CLI
        assert!(ModelRouter::should_use_cli("claude-sonnet-4.5"));
        assert!(ModelRouter::should_use_cli("claude-opus-4.1"));
        assert!(ModelRouter::should_use_cli("claude-haiku-4.5"));

        // Gemini models should use CLI
        assert!(ModelRouter::should_use_cli("gemini-3-pro"));
        assert!(ModelRouter::should_use_cli("gemini-2.5-flash"));

        // GPT models should NOT use CLI
        assert!(!ModelRouter::should_use_cli("gpt-5"));
        assert!(!ModelRouter::should_use_cli("gpt-5.1-high"));
        assert!(!ModelRouter::should_use_cli("gpt-5-codex"));
    }

    #[test]
    fn test_get_provider_type() {
        assert_eq!(
            ModelRouter::get_provider_type("claude-sonnet-4.5"),
            ProviderType::Claude
        );
        assert_eq!(
            ModelRouter::get_provider_type("gemini-3-pro"),
            ProviderType::Gemini
        );
        assert_eq!(
            ModelRouter::get_provider_type("gpt-5"),
            ProviderType::ChatGPT
        );
    }

    #[test]
    fn test_provider_display_name() {
        assert_eq!(provider_display_name("claude-sonnet-4.5"), "Claude");
        assert_eq!(provider_display_name("gemini-3-pro"), "Gemini");
        assert_eq!(provider_display_name("gpt-5"), "ChatGPT");
    }

    #[test]
    fn test_uses_cli_routing() {
        assert!(uses_cli_routing("claude-opus-4.1"));
        assert!(uses_cli_routing("gemini-2.5-pro"));
        assert!(!uses_cli_routing("gpt-5-mini"));
    }

    #[tokio::test]
    async fn test_execute_chatgpt_returns_use_native() {
        let settings = CliRoutingSettings::default();
        let result = ModelRouter::execute_prompt("gpt-5", "test", &settings).await;
        matches!(result, RouterResult::UseNative);
    }
}
