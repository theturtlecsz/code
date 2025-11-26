//! Gemini CLI Provider Implementation (SPEC-KIT-952)
//!
//! Routes prompts through the `gemini` CLI for Google models.
//! Uses plain text output format.

#![allow(dead_code)] // CLI provider helpers pending integration

use crate::cli_executor::{CliError, CliExecutor};
use crate::providers::{ProviderResponse, ProviderResult};
use std::time::Duration;

/// Gemini CLI provider for executing prompts
#[derive(Debug)]
pub struct GeminiProvider {
    executor: CliExecutor,
}

impl GeminiProvider {
    /// Create a new Gemini provider
    ///
    /// Detects the Gemini CLI in PATH and verifies it's available.
    pub fn new() -> ProviderResult<Self> {
        let cli_path = CliExecutor::detect("gemini")?;
        let executor = CliExecutor::with_default_timeout(cli_path, "gemini".to_string());
        Ok(Self { executor })
    }

    /// Create a new Gemini provider with custom timeout
    pub fn with_timeout(timeout_secs: u64) -> ProviderResult<Self> {
        let cli_path = CliExecutor::detect("gemini")?;
        let executor = CliExecutor::new(
            cli_path,
            "gemini".to_string(),
            Duration::from_secs(timeout_secs),
        );
        Ok(Self { executor })
    }

    /// Execute a prompt using the Gemini CLI
    ///
    /// # Arguments
    /// * `prompt` - The prompt text to send to Gemini
    /// * `model` - The model to use (e.g., "gemini-2.0-flash")
    ///
    /// # Returns
    /// The provider response with generated content
    pub async fn execute_prompt(
        &self,
        prompt: &str,
        model: &str,
    ) -> ProviderResult<ProviderResponse> {
        // Build command arguments
        let args = vec!["-p", prompt, "-m", model];

        // Execute CLI command
        let stdout = self.executor.execute_for_stdout(&args).await?;

        // Gemini returns plain text - trim whitespace
        let content = stdout.trim().to_string();

        Ok(ProviderResponse {
            content,
            model: model.to_string(),
            usage: None, // Gemini CLI doesn't provide token usage
        })
    }

    /// Execute a prompt with CLI routing settings
    ///
    /// # Arguments
    /// * `prompt` - The prompt text to send to Gemini
    /// * `model` - The model to use
    /// * `settings` - CLI routing settings from TUI config
    ///
    /// # Returns
    /// The provider response with generated content
    pub async fn execute_prompt_with_settings(
        &self,
        prompt: &str,
        model: &str,
        settings: &crate::providers::CliRoutingSettings,
    ) -> ProviderResult<ProviderResponse> {
        // Build command arguments
        let mut args = vec!["-p", prompt, "-m", model];

        // Map sandbox policy to CLI flags
        if let Some(ref sandbox) = settings.sandbox_policy {
            match sandbox {
                codex_core::protocol::SandboxPolicy::DangerFullAccess => {
                    // Full access maps to YOLO mode
                    args.push("--approval-mode");
                    args.push("yolo");
                }
                codex_core::protocol::SandboxPolicy::ReadOnly => {
                    // Enable sandbox mode for read-only
                    args.push("-s");
                }
                codex_core::protocol::SandboxPolicy::WorkspaceWrite { .. } => {
                    // Default behavior for workspace write
                }
            }
        }

        // Map approval policy to approval mode (if not already set by sandbox)
        if (settings.sandbox_policy.is_none()
            || !matches!(
                settings.sandbox_policy,
                Some(codex_core::protocol::SandboxPolicy::DangerFullAccess)
            ))
            && let Some(ref approval) = settings.approval_policy
        {
            match approval {
                codex_core::protocol::AskForApproval::Never => {
                    args.push("--approval-mode");
                    args.push("yolo");
                }
                codex_core::protocol::AskForApproval::OnFailure => {
                    args.push("--approval-mode");
                    args.push("auto_edit");
                }
                codex_core::protocol::AskForApproval::OnRequest
                | codex_core::protocol::AskForApproval::UnlessTrusted => {
                    // Default behavior, no flag needed
                }
            }
        }

        // Execute CLI command
        let stdout = self.executor.execute_for_stdout(&args).await?;

        // Gemini returns plain text - trim whitespace
        let content = stdout.trim().to_string();

        Ok(ProviderResponse {
            content,
            model: model.to_string(),
            usage: None,
        })
    }

    /// Execute a prompt with default model
    ///
    /// Uses gemini-2.0-flash as the default model.
    pub async fn execute_prompt_default(&self, prompt: &str) -> ProviderResult<ProviderResponse> {
        self.execute_prompt(prompt, "gemini-2.0-flash").await
    }

    /// Execute a prompt with additional options
    ///
    /// # Arguments
    /// * `prompt` - The prompt text to send to Gemini
    /// * `model` - The model to use
    /// * `auto_approve` - Whether to auto-approve actions (yolo mode)
    pub async fn execute_with_options(
        &self,
        prompt: &str,
        model: &str,
        auto_approve: bool,
    ) -> ProviderResult<ProviderResponse> {
        // Build command arguments
        let mut args = vec!["-p", prompt, "-m", model];

        // Add yolo flag if auto-approve is enabled
        if auto_approve {
            args.push("-y");
        }

        // Execute CLI command
        let stdout = self.executor.execute_for_stdout(&args).await?;

        // Gemini returns plain text
        let content = stdout.trim().to_string();

        Ok(ProviderResponse {
            content,
            model: model.to_string(),
            usage: None,
        })
    }

    /// Check if Gemini CLI is authenticated
    ///
    /// Attempts a simple test command to verify authentication status.
    pub async fn check_auth(&self) -> ProviderResult<bool> {
        // Try --version to check if CLI is responsive
        let result = self.executor.execute(&["--version"]).await;

        match result {
            Ok(output) => Ok(output.success),
            Err(CliError::NotAuthenticated { .. }) => Ok(false),
            Err(e) => Err(e.into()),
        }
    }
}

/// Check if Gemini CLI is available in PATH
pub fn is_available() -> bool {
    CliExecutor::detect("gemini").is_ok()
}

/// Get installation instructions for Gemini CLI
pub fn install_instructions() -> &'static str {
    "Install Gemini CLI:\n  \
     npm install -g @google/gemini-cli\n\n\
     Then authenticate by running:\n  \
     gemini\n\n\
     Follow the prompts to complete OAuth authentication."
}

/// Get authentication instructions for Gemini CLI
pub fn auth_instructions() -> &'static str {
    "Please authenticate the Gemini CLI:\n  \
     gemini\n\n\
     Follow the OAuth prompts to complete authentication,\n\
     then retry your command."
}

/// Get the default model for Gemini
pub fn default_model() -> &'static str {
    "gemini-2.0-flash"
}

/// Map model presets to Gemini model names
pub fn map_model_name(preset: &str) -> &str {
    let preset_lower = preset.to_ascii_lowercase();

    // Map preset IDs to actual model names
    if preset_lower.contains("3-pro") || preset_lower.contains("3.0-pro") {
        "gemini-3.0-pro"
    } else if preset_lower.contains("2.5-pro") {
        "gemini-2.5-pro"
    } else if preset_lower.contains("2.5-flash") {
        "gemini-2.5-flash"
    } else if preset_lower.contains("2.0-flash") || preset_lower.contains("flash") {
        "gemini-2.0-flash"
    } else {
        // Default to the preset name itself
        preset
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_map_model_name() {
        assert_eq!(map_model_name("gemini-3-pro"), "gemini-3.0-pro");
        assert_eq!(map_model_name("gemini-2.5-pro"), "gemini-2.5-pro");
        assert_eq!(map_model_name("gemini-2.5-flash"), "gemini-2.5-flash");
        assert_eq!(map_model_name("gemini-2.0-flash"), "gemini-2.0-flash");
        assert_eq!(map_model_name("flash"), "gemini-2.0-flash");
    }

    #[test]
    fn test_default_model() {
        assert_eq!(default_model(), "gemini-2.0-flash");
    }

    #[test]
    fn test_install_instructions() {
        let instructions = install_instructions();
        assert!(instructions.contains("npm install"));
        assert!(instructions.contains("gemini-cli"));
    }

    #[test]
    fn test_auth_instructions() {
        let instructions = auth_instructions();
        assert!(instructions.contains("gemini"));
        assert!(instructions.contains("OAuth"));
    }
}
