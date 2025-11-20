//! Provider Login Module for CLI-based Authentication (SPEC-KIT-954)
//!
//! Implements the 5-ALT approach: CLI-based authentication for Claude and Gemini providers.
//! This aligns with SPEC-KIT-952's architectural decision to route Claude/Gemini through
//! native CLIs rather than implementing OAuth orchestration in the TUI.
//!
//! ## Design Rationale
//!
//! Claude's OAuth uses a web callback (`https://console.anthropic.com/oauth/code/callback`)
//! which is incompatible with localhost server patterns. Instead of implementing complex
//! device flows or manual code entry, we leverage the official CLIs which handle OAuth
//! correctly for their respective services.
//!
//! ## Flow
//!
//! 1. User selects provider in /login → Add Account
//! 2. TUI checks CLI installation status
//! 3. If not installed: show installation instructions
//! 4. If installed but not authenticated: show auth command
//! 5. User runs CLI command in their terminal to authenticate
//! 6. Once authenticated, tokens are available for native streaming (SPEC-KIT-953)

use crate::providers::claude;
use crate::providers::gemini;
use crate::providers::ProviderType;

/// Status of provider login/authentication
#[derive(Debug, Clone)]
pub enum ProviderLoginStatus {
    /// CLI is not installed - user needs to install it first
    CliNotInstalled {
        /// Installation instructions for the CLI
        install_instructions: String,
    },
    /// CLI is installed but not authenticated
    NotAuthenticated {
        /// Command to run for authentication
        auth_command: String,
        /// Detailed authentication instructions
        auth_instructions: String,
    },
    /// Provider is fully authenticated and ready to use
    Authenticated {
        /// Provider display name
        provider_name: String,
    },
    /// Error occurred during status check
    Error(String),
}

impl ProviderLoginStatus {
    /// Check if this status indicates successful authentication
    pub fn is_authenticated(&self) -> bool {
        matches!(self, Self::Authenticated { .. })
    }

    /// Check if CLI is installed (regardless of auth status)
    pub fn is_cli_installed(&self) -> bool {
        !matches!(self, Self::CliNotInstalled { .. })
    }

    /// Get a user-friendly message describing the status
    pub fn display_message(&self) -> String {
        match self {
            Self::CliNotInstalled { install_instructions } => {
                format!("CLI not installed.\n\n{}", install_instructions)
            }
            Self::NotAuthenticated {
                auth_command,
                auth_instructions,
            } => {
                format!(
                    "CLI installed but not authenticated.\n\n\
                     Run the following command in your terminal:\n\n  {}\n\n{}",
                    auth_command, auth_instructions
                )
            }
            Self::Authenticated { provider_name } => {
                format!("{} is authenticated and ready to use.", provider_name)
            }
            Self::Error(msg) => {
                format!("Error checking authentication status: {}", msg)
            }
        }
    }

    /// Get a short status label
    pub fn short_label(&self) -> &'static str {
        match self {
            Self::CliNotInstalled { .. } => "Not Installed",
            Self::NotAuthenticated { .. } => "Not Authenticated",
            Self::Authenticated { .. } => "Authenticated",
            Self::Error(_) => "Error",
        }
    }
}

/// Check the login status for a provider
///
/// This is the main entry point for checking provider authentication status.
/// It checks both CLI installation and authentication state.
///
/// # Arguments
/// * `provider` - The provider type to check
///
/// # Returns
/// The current login status for the provider
pub async fn check_provider_login_status(provider: ProviderType) -> ProviderLoginStatus {
    match provider {
        ProviderType::Claude => check_claude_status().await,
        ProviderType::Gemini => check_gemini_status().await,
        ProviderType::ChatGPT => {
            // ChatGPT uses native OAuth handled by codex_login
            ProviderLoginStatus::Authenticated {
                provider_name: "ChatGPT".to_string(),
            }
        }
    }
}

/// Check Claude CLI installation and authentication status
async fn check_claude_status() -> ProviderLoginStatus {
    // First check if CLI is installed
    if !claude::is_available() {
        return ProviderLoginStatus::CliNotInstalled {
            install_instructions: claude::install_instructions().to_string(),
        };
    }

    // CLI is installed, check if authenticated
    match claude::ClaudeProvider::new() {
        Ok(provider) => match provider.check_auth().await {
            Ok(true) => ProviderLoginStatus::Authenticated {
                provider_name: "Claude".to_string(),
            },
            Ok(false) => ProviderLoginStatus::NotAuthenticated {
                auth_command: "claude".to_string(),
                auth_instructions: claude::auth_instructions().to_string(),
            },
            Err(e) => {
                // Authentication check failed - might be network issue or CLI error
                // Treat as not authenticated since we can't verify
                ProviderLoginStatus::NotAuthenticated {
                    auth_command: "claude".to_string(),
                    auth_instructions: format!(
                        "{}\n\nNote: Authentication check failed: {}",
                        claude::auth_instructions(),
                        e
                    ),
                }
            }
        },
        Err(e) => ProviderLoginStatus::Error(e.to_string()),
    }
}

/// Check Gemini CLI installation and authentication status
async fn check_gemini_status() -> ProviderLoginStatus {
    // First check if CLI is installed
    if !gemini::is_available() {
        return ProviderLoginStatus::CliNotInstalled {
            install_instructions: gemini::install_instructions().to_string(),
        };
    }

    // CLI is installed, check if authenticated
    match gemini::GeminiProvider::new() {
        Ok(provider) => match provider.check_auth().await {
            Ok(true) => ProviderLoginStatus::Authenticated {
                provider_name: "Gemini".to_string(),
            },
            Ok(false) => ProviderLoginStatus::NotAuthenticated {
                auth_command: "gemini".to_string(),
                auth_instructions: gemini::auth_instructions().to_string(),
            },
            Err(e) => {
                // Authentication check failed
                ProviderLoginStatus::NotAuthenticated {
                    auth_command: "gemini".to_string(),
                    auth_instructions: format!(
                        "{}\n\nNote: Authentication check failed: {}",
                        gemini::auth_instructions(),
                        e
                    ),
                }
            }
        },
        Err(e) => ProviderLoginStatus::Error(e.to_string()),
    }
}

/// Get installation instructions for a provider's CLI
pub fn get_install_instructions(provider: ProviderType) -> Option<&'static str> {
    match provider {
        ProviderType::Claude => Some(claude::install_instructions()),
        ProviderType::Gemini => Some(gemini::install_instructions()),
        ProviderType::ChatGPT => None, // No CLI needed
    }
}

/// Get authentication instructions for a provider's CLI
pub fn get_auth_instructions(provider: ProviderType) -> Option<&'static str> {
    match provider {
        ProviderType::Claude => Some(claude::auth_instructions()),
        ProviderType::Gemini => Some(gemini::auth_instructions()),
        ProviderType::ChatGPT => None, // Uses OAuth in TUI
    }
}

/// Get the CLI command name for a provider
pub fn get_cli_command(provider: ProviderType) -> Option<&'static str> {
    provider.cli_name()
}

/// Check if a provider requires CLI-based authentication
pub fn requires_cli_auth(provider: ProviderType) -> bool {
    matches!(provider, ProviderType::Claude | ProviderType::Gemini)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_provider_login_status_display_message() {
        let not_installed = ProviderLoginStatus::CliNotInstalled {
            install_instructions: "Install from example.com".to_string(),
        };
        assert!(not_installed.display_message().contains("not installed"));

        let not_auth = ProviderLoginStatus::NotAuthenticated {
            auth_command: "test-cli".to_string(),
            auth_instructions: "Run test-cli to authenticate".to_string(),
        };
        assert!(not_auth.display_message().contains("not authenticated"));
        assert!(not_auth.display_message().contains("test-cli"));

        let authenticated = ProviderLoginStatus::Authenticated {
            provider_name: "TestProvider".to_string(),
        };
        assert!(authenticated.display_message().contains("ready to use"));
    }

    #[test]
    fn test_provider_login_status_checks() {
        let authenticated = ProviderLoginStatus::Authenticated {
            provider_name: "Test".to_string(),
        };
        assert!(authenticated.is_authenticated());
        assert!(authenticated.is_cli_installed());

        let not_installed = ProviderLoginStatus::CliNotInstalled {
            install_instructions: "test".to_string(),
        };
        assert!(!not_installed.is_authenticated());
        assert!(!not_installed.is_cli_installed());

        let not_auth = ProviderLoginStatus::NotAuthenticated {
            auth_command: "test".to_string(),
            auth_instructions: "test".to_string(),
        };
        assert!(!not_auth.is_authenticated());
        assert!(not_auth.is_cli_installed());
    }

    #[test]
    fn test_requires_cli_auth() {
        assert!(requires_cli_auth(ProviderType::Claude));
        assert!(requires_cli_auth(ProviderType::Gemini));
        assert!(!requires_cli_auth(ProviderType::ChatGPT));
    }

    #[test]
    fn test_get_cli_command() {
        assert_eq!(get_cli_command(ProviderType::Claude), Some("claude"));
        assert_eq!(get_cli_command(ProviderType::Gemini), Some("gemini"));
        assert_eq!(get_cli_command(ProviderType::ChatGPT), None);
    }

    #[test]
    fn test_get_install_instructions() {
        assert!(get_install_instructions(ProviderType::Claude).is_some());
        assert!(get_install_instructions(ProviderType::Gemini).is_some());
        assert!(get_install_instructions(ProviderType::ChatGPT).is_none());
    }

    #[test]
    fn test_short_labels() {
        assert_eq!(
            ProviderLoginStatus::Authenticated {
                provider_name: "Test".to_string()
            }
            .short_label(),
            "Authenticated"
        );
        assert_eq!(
            ProviderLoginStatus::CliNotInstalled {
                install_instructions: "test".to_string()
            }
            .short_label(),
            "Not Installed"
        );
    }

    #[tokio::test]
    async fn test_chatgpt_always_authenticated() {
        let status = check_provider_login_status(ProviderType::ChatGPT).await;
        assert!(status.is_authenticated());
    }
}
