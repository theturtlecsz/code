//! CLI Executor for Multi-Provider Model Support (SPEC-KIT-952)
//!
//! This module provides infrastructure for executing CLI commands for non-OpenAI
//! providers (Claude, Gemini) that don't support third-party OAuth.

#![allow(dead_code)] // CLI execution infrastructure, some helpers pending integration

use std::path::PathBuf;
use std::time::Duration;
use tokio::process::Command;

/// Default timeout for CLI command execution (5 minutes)
pub const DEFAULT_TIMEOUT_SECS: u64 = 300;

/// Result type for CLI execution operations
pub type CliResult<T> = Result<T, CliError>;

/// Errors that can occur during CLI execution
#[derive(Debug, Clone)]
pub enum CliError {
    /// CLI executable not found in PATH
    CliNotFound {
        cli_name: String,
        install_instructions: String,
    },
    /// CLI is not authenticated
    NotAuthenticated {
        cli_name: String,
        auth_instructions: String,
    },
    /// Command execution timed out
    Timeout { cli_name: String, timeout_secs: u64 },
    /// Command execution failed
    ExecutionFailed { cli_name: String, message: String },
    /// Failed to parse CLI output
    ParseError { cli_name: String, message: String },
    /// Invalid UTF-8 in output
    InvalidUtf8 { cli_name: String },
}

impl std::fmt::Display for CliError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            CliError::CliNotFound {
                cli_name,
                install_instructions,
            } => {
                write!(
                    f,
                    "{} CLI not found in PATH.\n\n{}",
                    cli_name, install_instructions
                )
            }
            CliError::NotAuthenticated {
                cli_name,
                auth_instructions,
            } => {
                write!(
                    f,
                    "{} CLI is not authenticated.\n\n{}",
                    cli_name, auth_instructions
                )
            }
            CliError::Timeout {
                cli_name,
                timeout_secs,
            } => {
                write!(
                    f,
                    "{} CLI did not respond within {} seconds.\n\n\
                     The command may still be running. Check with:\n  \
                     ps aux | grep {}",
                    cli_name, timeout_secs, cli_name
                )
            }
            CliError::ExecutionFailed { cli_name, message } => {
                write!(f, "{} CLI error: {}", cli_name, message)
            }
            CliError::ParseError { cli_name, message } => {
                write!(f, "Failed to parse {} CLI output: {}", cli_name, message)
            }
            CliError::InvalidUtf8 { cli_name } => {
                write!(f, "{} CLI returned invalid UTF-8 output", cli_name)
            }
        }
    }
}

impl std::error::Error for CliError {}

/// Output from a CLI command execution
#[derive(Debug, Clone)]
pub struct CliOutput {
    /// Standard output from the command
    pub stdout: String,
    /// Standard error from the command (if any)
    pub stderr: String,
    /// Whether the command exited successfully
    pub success: bool,
    /// Exit code if available
    pub exit_code: Option<i32>,
}

/// Executor for CLI commands with timeout and output capture
#[derive(Debug, Clone)]
pub struct CliExecutor {
    /// Path to the CLI executable
    cli_path: PathBuf,
    /// Name of the CLI for error messages
    cli_name: String,
    /// Timeout duration for command execution
    timeout: Duration,
}

impl CliExecutor {
    /// Detect if a CLI is available in PATH
    ///
    /// # Arguments
    /// * `cli_name` - Name of the CLI executable (e.g., "claude", "gemini")
    ///
    /// # Returns
    /// Path to the CLI executable if found
    pub fn detect(cli_name: &str) -> CliResult<PathBuf> {
        which::which(cli_name).map_err(|_| {
            let install_instructions = match cli_name {
                "claude" => {
                    "Install Claude Code CLI from:\n  \
                     https://claude.ai/download\n\n\
                     Then authenticate by running:\n  \
                     claude\n\n\
                     After authentication, retry your command."
                }
                "gemini" => {
                    "Install Gemini CLI:\n  \
                     npm install -g @anthropic-ai/gemini-cli\n\n\
                     Then authenticate by running:\n  \
                     gemini\n\n\
                     After authentication, retry your command."
                }
                _ => "Please install the CLI and ensure it's in your PATH.",
            };
            CliError::CliNotFound {
                cli_name: cli_name.to_string(),
                install_instructions: install_instructions.to_string(),
            }
        })
    }

    /// Create a new CLI executor
    ///
    /// # Arguments
    /// * `cli_path` - Path to the CLI executable
    /// * `cli_name` - Name of the CLI for error messages
    /// * `timeout` - Maximum duration to wait for command completion
    pub fn new(cli_path: PathBuf, cli_name: String, timeout: Duration) -> Self {
        Self {
            cli_path,
            cli_name,
            timeout,
        }
    }

    /// Create a new CLI executor with default timeout
    pub fn with_default_timeout(cli_path: PathBuf, cli_name: String) -> Self {
        Self::new(
            cli_path,
            cli_name,
            Duration::from_secs(DEFAULT_TIMEOUT_SECS),
        )
    }

    /// Get the CLI name
    pub fn cli_name(&self) -> &str {
        &self.cli_name
    }

    /// Get the CLI path
    pub fn cli_path(&self) -> &PathBuf {
        &self.cli_path
    }

    /// Execute a CLI command with the given arguments
    ///
    /// # Arguments
    /// * `args` - Command line arguments to pass to the CLI
    ///
    /// # Returns
    /// The command output including stdout, stderr, and exit status
    pub async fn execute(&self, args: &[&str]) -> CliResult<CliOutput> {
        let output_result = tokio::time::timeout(
            self.timeout,
            Command::new(&self.cli_path).args(args).output(),
        )
        .await;

        // Handle timeout
        let output = match output_result {
            Ok(result) => result.map_err(|e| CliError::ExecutionFailed {
                cli_name: self.cli_name.clone(),
                message: e.to_string(),
            })?,
            Err(_) => {
                return Err(CliError::Timeout {
                    cli_name: self.cli_name.clone(),
                    timeout_secs: self.timeout.as_secs(),
                });
            }
        };

        // Convert output to strings
        let stdout = String::from_utf8(output.stdout).map_err(|_| CliError::InvalidUtf8 {
            cli_name: self.cli_name.clone(),
        })?;

        let stderr = String::from_utf8(output.stderr).map_err(|_| CliError::InvalidUtf8 {
            cli_name: self.cli_name.clone(),
        })?;

        Ok(CliOutput {
            stdout,
            stderr,
            success: output.status.success(),
            exit_code: output.status.code(),
        })
    }

    /// Execute a CLI command and return stdout if successful
    ///
    /// # Arguments
    /// * `args` - Command line arguments to pass to the CLI
    ///
    /// # Returns
    /// The stdout content if the command succeeded
    pub async fn execute_for_stdout(&self, args: &[&str]) -> CliResult<String> {
        let output = self.execute(args).await?;

        if output.success {
            Ok(output.stdout)
        } else {
            // Check for common authentication errors
            let stderr_lower = output.stderr.to_ascii_lowercase();
            if stderr_lower.contains("auth")
                || stderr_lower.contains("login")
                || stderr_lower.contains("token")
                || stderr_lower.contains("credential")
            {
                return Err(CliError::NotAuthenticated {
                    cli_name: self.cli_name.clone(),
                    auth_instructions: format!(
                        "Please authenticate the {} CLI first:\n  {}\n\n\
                         Then retry your command.",
                        self.cli_name, self.cli_name
                    ),
                });
            }

            Err(CliError::ExecutionFailed {
                cli_name: self.cli_name.clone(),
                message: if output.stderr.is_empty() {
                    format!("Command failed with exit code {:?}", output.exit_code)
                } else {
                    output.stderr
                },
            })
        }
    }
}

/// Check if a CLI is available and optionally verify authentication
pub async fn check_cli_available(cli_name: &str, verify_auth: bool) -> CliResult<CliExecutor> {
    let cli_path = CliExecutor::detect(cli_name)?;
    let executor = CliExecutor::with_default_timeout(cli_path, cli_name.to_string());

    if verify_auth {
        // Run a simple test command to verify CLI is working
        // This is a lightweight check - actual auth verification happens on first real use
        let test_args = match cli_name {
            "claude" => vec!["--version"],
            "gemini" => vec!["--version"],
            _ => vec!["--help"],
        };

        executor.execute(&test_args).await?;
    }

    Ok(executor)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cli_error_display() {
        let error = CliError::CliNotFound {
            cli_name: "claude".to_string(),
            install_instructions: "Install from https://claude.ai".to_string(),
        };
        let display = format!("{}", error);
        assert!(display.contains("claude CLI not found"));
        assert!(display.contains("Install from"));
    }

    #[test]
    fn test_cli_error_timeout_display() {
        let error = CliError::Timeout {
            cli_name: "gemini".to_string(),
            timeout_secs: 300,
        };
        let display = format!("{}", error);
        assert!(display.contains("gemini CLI did not respond"));
        assert!(display.contains("300 seconds"));
    }

    #[test]
    fn test_cli_error_not_authenticated_display() {
        let error = CliError::NotAuthenticated {
            cli_name: "claude".to_string(),
            auth_instructions: "Run: claude".to_string(),
        };
        let display = format!("{}", error);
        assert!(display.contains("not authenticated"));
        assert!(display.contains("Run: claude"));
    }

    #[tokio::test]
    async fn test_detect_nonexistent_cli() {
        let result = CliExecutor::detect("nonexistent-cli-12345");
        assert!(result.is_err());
        if let Err(CliError::CliNotFound { cli_name, .. }) = result {
            assert_eq!(cli_name, "nonexistent-cli-12345");
        } else {
            panic!("Expected CliNotFound error");
        }
    }

    #[tokio::test]
    async fn test_executor_with_echo() {
        // Test with 'echo' which should be available on all systems
        if let Ok(echo_path) = CliExecutor::detect("echo") {
            let executor = CliExecutor::with_default_timeout(echo_path, "echo".to_string());
            let result = executor.execute(&["hello", "world"]).await;
            assert!(result.is_ok());
            let output = result.unwrap();
            assert!(output.success);
            assert!(output.stdout.contains("hello"));
            assert!(output.stdout.contains("world"));
        }
    }

    #[tokio::test]
    async fn test_executor_timeout() {
        // Test with sleep command to verify timeout works
        if let Ok(sleep_path) = CliExecutor::detect("sleep") {
            let executor = CliExecutor::new(
                sleep_path,
                "sleep".to_string(),
                Duration::from_millis(100), // Very short timeout
            );
            let result = executor.execute(&["10"]).await; // Sleep for 10 seconds
            assert!(result.is_err());
            if let Err(CliError::Timeout { cli_name, .. }) = result {
                assert_eq!(cli_name, "sleep");
            } else {
                panic!("Expected Timeout error");
            }
        }
    }

    #[tokio::test]
    async fn test_executor_failed_command() {
        // Test with a command that will fail
        if let Ok(ls_path) = CliExecutor::detect("ls") {
            let executor = CliExecutor::with_default_timeout(ls_path, "ls".to_string());
            let result = executor
                .execute_for_stdout(&["/nonexistent-directory-12345"])
                .await;
            assert!(result.is_err());
        }
    }
}
