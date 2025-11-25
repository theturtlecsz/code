//! Provider implementations for CLI-routed models (SPEC-KIT-952)
//!
//! This module contains provider implementations for models that route through
//! native CLIs instead of direct API calls (Claude, Gemini).

pub mod claude;
pub mod claude_streaming; // SPEC-KIT-952 Phase 1: Streaming CLI support
pub mod gemini;
pub mod gemini_streaming; // SPEC-KIT-952 Phase 1: Streaming CLI support

use crate::cli_executor::CliError;
use codex_core::protocol::{AskForApproval, SandboxPolicy};

/// CLI routing settings derived from TUI config
#[derive(Debug, Clone, Default)]
pub struct CliRoutingSettings {
    /// Sandbox policy for execution restrictions
    pub sandbox_policy: Option<SandboxPolicy>,
    /// Approval policy for command execution
    pub approval_policy: Option<AskForApproval>,
}

/// Common response type for all CLI providers
#[derive(Debug, Clone)]
#[allow(dead_code)] // Fields used when constructing, consumed via streaming
pub struct ProviderResponse {
    /// The generated text content
    pub content: String,
    /// Model identifier used for generation
    pub model: String,
    /// Token usage information (if available)
    pub usage: Option<TokenUsage>,
}

/// Token usage information from providers
#[derive(Debug, Clone, Default)]
#[allow(dead_code)] // Fields populated by CLI parsers, consumed downstream
pub struct TokenUsage {
    /// Number of tokens in the input/prompt
    pub input_tokens: Option<u32>,
    /// Number of tokens in the output/response
    pub output_tokens: Option<u32>,
}

/// Errors specific to provider operations
#[derive(Debug, Clone)]
pub enum ProviderError {
    /// Underlying CLI error
    Cli(CliError),
    /// Provider-specific error
    Provider { provider: String, message: String },
}

impl std::fmt::Display for ProviderError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ProviderError::Cli(err) => write!(f, "{}", err),
            ProviderError::Provider { provider, message } => {
                write!(f, "{} provider error: {}", provider, message)
            }
        }
    }
}

impl std::error::Error for ProviderError {}

impl From<CliError> for ProviderError {
    fn from(err: CliError) -> Self {
        ProviderError::Cli(err)
    }
}

/// Result type for provider operations
pub type ProviderResult<T> = Result<T, ProviderError>;

/// Provider type for model routing
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProviderType {
    /// OpenAI ChatGPT - uses native OAuth
    ChatGPT,
    /// Anthropic Claude - routes through CLI
    Claude,
    /// Google Gemini - routes through CLI
    Gemini,
}

impl ProviderType {
    /// Determine provider type from model name
    pub fn from_model_name(model: &str) -> Self {
        let model_lower = model.to_ascii_lowercase();

        // Claude models
        if model_lower.contains("claude")
            || model_lower.contains("opus")
            || model_lower.contains("sonnet")
            || model_lower.contains("haiku")
        {
            return Self::Claude;
        }

        // Gemini models
        if model_lower.contains("gemini")
            || model_lower.contains("flash")
            || model_lower.starts_with("bison")
        {
            return Self::Gemini;
        }

        // Default to ChatGPT for GPT models and unknown
        Self::ChatGPT
    }

    /// Check if this provider uses CLI routing
    pub fn uses_cli_routing(&self) -> bool {
        matches!(self, Self::Claude | Self::Gemini)
    }

    /// Get the CLI name for this provider (if applicable)
    pub fn cli_name(&self) -> Option<&'static str> {
        match self {
            Self::Claude => Some("claude"),
            Self::Gemini => Some("gemini"),
            Self::ChatGPT => None,
        }
    }

    /// Get display name for the provider
    pub fn display_name(&self) -> &'static str {
        match self {
            Self::ChatGPT => "ChatGPT",
            Self::Claude => "Claude",
            Self::Gemini => "Gemini",
        }
    }
}

impl std::fmt::Display for ProviderType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.display_name())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_provider_from_model_name() {
        // Claude models
        assert_eq!(
            ProviderType::from_model_name("claude-sonnet-4.5"),
            ProviderType::Claude
        );
        assert_eq!(
            ProviderType::from_model_name("claude-opus-4.5"),
            ProviderType::Claude
        );
        assert_eq!(
            ProviderType::from_model_name("claude-haiku-4.5"),
            ProviderType::Claude
        );

        // Gemini models
        assert_eq!(
            ProviderType::from_model_name("gemini-3-pro"),
            ProviderType::Gemini
        );
        assert_eq!(
            ProviderType::from_model_name("gemini-2.5-flash"),
            ProviderType::Gemini
        );

        // GPT models
        assert_eq!(
            ProviderType::from_model_name("gpt-5"),
            ProviderType::ChatGPT
        );
        assert_eq!(
            ProviderType::from_model_name("gpt-5.1-high"),
            ProviderType::ChatGPT
        );

        // Unknown defaults to ChatGPT
        assert_eq!(
            ProviderType::from_model_name("unknown-model"),
            ProviderType::ChatGPT
        );
    }

    #[test]
    fn test_provider_uses_cli_routing() {
        assert!(!ProviderType::ChatGPT.uses_cli_routing());
        assert!(ProviderType::Claude.uses_cli_routing());
        assert!(ProviderType::Gemini.uses_cli_routing());
    }

    #[test]
    fn test_provider_cli_name() {
        assert_eq!(ProviderType::ChatGPT.cli_name(), None);
        assert_eq!(ProviderType::Claude.cli_name(), Some("claude"));
        assert_eq!(ProviderType::Gemini.cli_name(), Some("gemini"));
    }
}
