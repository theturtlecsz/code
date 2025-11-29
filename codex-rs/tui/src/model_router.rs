//! Model Router for Multi-Provider Support (SPEC-KIT-952)
//!
//! Routes prompts to appropriate providers based on model selection.
//! - ChatGPT models → Native OAuth (existing codex-core flow)
//! - Claude models → CLI routing with streaming (SPEC-KIT-952)
//! - Gemini models → CLI routing with streaming (SPEC-KIT-952)

use crate::app_event_sender::AppEventSender;
use crate::providers::ProviderType;

/// Execute prompt via CLI routing with streaming (SPEC-KIT-952)
///
/// PRIMARY routing method for Claude and Gemini models.
/// Uses external CLI processes with streaming support.
///
/// # Arguments
/// * `model` - Model name/ID
/// * `prompt` - User prompt text
/// * `tx` - Event sender for streaming updates
///
/// # Returns
/// Accumulated response text
pub async fn execute_with_cli_streaming(
    model: &str,
    prompt: &str,
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

            provider
                .execute_streaming(prompt, model, tx)
                .await
                .map_err(|e| format!("{}", e))
        }
        ProviderType::Gemini => {
            // Gemini PTY routing (SPEC-KIT-952-F)
            use crate::providers::gemini_streaming::GeminiStreamingProvider;

            let provider = GeminiStreamingProvider::new()
                .map_err(|e| format!("Failed to create Gemini provider: {}", e))?;

            provider
                .execute_streaming(prompt, model, tx)
                .await
                .map_err(|e| format!("{}", e))
        }
    }
}

/// Check if CLI streaming is available for a model (SPEC-KIT-952)
///
/// Returns true for Claude and Gemini models (both use CLI routing).
/// ChatGPT uses native OAuth flow.
pub fn supports_cli_streaming(model: &str) -> bool {
    let provider_type = ProviderType::from_model_name(model);
    matches!(provider_type, ProviderType::Claude | ProviderType::Gemini)
}

/// Get provider display name for a model
pub fn provider_display_name(model: &str) -> &'static str {
    ProviderType::from_model_name(model).display_name()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_supports_cli_streaming() {
        // Claude models should use CLI streaming
        assert!(supports_cli_streaming("claude-sonnet-4.5"));
        assert!(supports_cli_streaming("claude-opus-4.5"));
        assert!(supports_cli_streaming("claude-haiku-4.5"));

        // Gemini models should use CLI streaming
        assert!(supports_cli_streaming("gemini-3-pro"));
        assert!(supports_cli_streaming("gemini-2.5-flash"));

        // GPT models should NOT use CLI streaming
        assert!(!supports_cli_streaming("gpt-5"));
        assert!(!supports_cli_streaming("gpt-5.1-high"));
        assert!(!supports_cli_streaming("gpt-5-codex"));
    }

    #[test]
    fn test_provider_display_name() {
        assert_eq!(provider_display_name("claude-sonnet-4.5"), "Claude");
        assert_eq!(provider_display_name("gemini-3-pro"), "Gemini");
        assert_eq!(provider_display_name("gpt-5"), "ChatGPT");
    }
}
