use async_trait::async_trait;
use tokio::io::AsyncWriteExt;
use tokio::process::Command;
use tokio::sync::mpsc;
use tokio::time::{Duration, timeout};

use super::CliExecutor;
use super::context::CliContextManager;
use super::stream::parse_claude_stream;
use super::types::{CliError, Conversation, StreamEvent};

/// Configuration for Claude CLI executor
#[derive(Debug, Clone)]
pub struct ClaudeCliConfig {
    /// Path to claude binary (default: "claude")
    pub binary_path: String,
    /// Model to use (e.g., "claude-opus-4.1", "claude-sonnet-4.5")
    pub model: Option<String>,
    /// Timeout for requests (default: 120 seconds)
    pub timeout_secs: u64,
}

impl Default for ClaudeCliConfig {
    fn default() -> Self {
        Self {
            binary_path: "claude".to_string(),
            model: None,
            timeout_secs: 120,
        }
    }
}

/// Claude CLI executor
///
/// Spawns external `claude` CLI process and manages request/response lifecycle.
/// Uses `--output-format stream-json` for structured streaming output.
pub struct ClaudeCliExecutor {
    config: ClaudeCliConfig,
}

impl ClaudeCliExecutor {
    pub fn new(config: ClaudeCliConfig) -> Self {
        Self { config }
    }

    /// Build command args for Claude CLI
    fn build_command_args(&self, conversation: &Conversation) -> Vec<String> {
        let mut args = vec![
            "--print".to_string(),
            "--output-format".to_string(),
            "stream-json".to_string(),
        ];

        // Add model if specified in config or conversation
        let model = self
            .config
            .model
            .as_ref()
            .or(Some(&conversation.model))
            .cloned();

        if let Some(m) = model {
            args.push("--model".to_string());
            args.push(m);
        }

        args
    }
}

#[async_trait]
impl CliExecutor for ClaudeCliExecutor {
    async fn execute(
        &self,
        conversation: &Conversation,
        user_message: &str,
    ) -> Result<mpsc::Receiver<StreamEvent>, CliError> {
        // Compress conversation if needed
        let conversation = CliContextManager::compress_if_needed(conversation, user_message);

        // Format history into prompt
        let prompt = CliContextManager::format_history(&conversation, user_message);

        tracing::debug!(
            "Executing Claude CLI: {} chars, ~{} tokens",
            prompt.len(),
            CliContextManager::estimate_tokens(&prompt)
        );

        // Build command
        let args = self.build_command_args(&conversation);
        let mut child = Command::new(&self.config.binary_path)
            .args(&args)
            .stdin(std::process::Stdio::piped())
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::piped())
            .spawn()
            .map_err(|e| {
                if e.kind() == std::io::ErrorKind::NotFound {
                    CliError::BinaryNotFound {
                        binary: self.config.binary_path.clone(),
                        install_hint: "Visit https://claude.ai/download".to_string(),
                    }
                } else {
                    CliError::Internal {
                        message: format!("Failed to spawn Claude CLI: {}", e),
                    }
                }
            })?;

        // Write prompt to stdin
        if let Some(mut stdin) = child.stdin.take() {
            stdin
                .write_all(prompt.as_bytes())
                .await
                .map_err(|e| CliError::Internal {
                    message: format!("Failed to write to stdin: {}", e),
                })?;
            stdin.shutdown().await.map_err(|e| CliError::Internal {
                message: format!("Failed to close stdin: {}", e),
            })?;
        }

        // Create channel for streaming events
        let (tx, rx) = mpsc::channel(100);

        // Spawn task to parse stdout
        let stdout = child.stdout.take().ok_or_else(|| CliError::Internal {
            message: "Failed to capture stdout".to_string(),
        })?;

        let tx_clone = tx.clone();
        let timeout_duration = Duration::from_secs(self.config.timeout_secs);

        tokio::spawn(async move {
            let parse_result = timeout(timeout_duration, parse_claude_stream(stdout, tx_clone))
                .await
                .map_err(|_| CliError::Timeout {
                    elapsed: timeout_duration,
                });

            match parse_result {
                Ok(Ok(())) => {
                    tracing::debug!("Claude stream parsing completed successfully");
                }
                Ok(Err(e)) => {
                    tracing::error!("Claude stream parsing failed: {:?}", e);
                    let _ = tx.send(StreamEvent::Error(e)).await;
                }
                Err(timeout_err) => {
                    tracing::error!("Claude request timed out: {:?}", timeout_err);
                    let _ = tx.send(StreamEvent::Error(timeout_err)).await;
                }
            }

            // Wait for process to exit and check status
            if let Ok(status) = child.wait().await {
                if !status.success() {
                    let code = status.code().unwrap_or(-1);
                    tracing::error!("Claude CLI exited with code: {}", code);

                    // Try to read stderr for error details
                    if let Some(mut stderr) = child.stderr {
                        let mut stderr_content = String::new();
                        use tokio::io::AsyncReadExt;
                        if let Ok(_) = stderr.read_to_string(&mut stderr_content).await {
                            let _ = tx
                                .send(StreamEvent::Error(CliError::ProcessFailed {
                                    code,
                                    stderr: stderr_content,
                                }))
                                .await;
                        }
                    }
                }
            }
        });

        Ok(rx)
    }

    async fn health_check(&self) -> Result<(), CliError> {
        let output = Command::new(&self.config.binary_path)
            .arg("--version")
            .output()
            .await
            .map_err(|e| {
                if e.kind() == std::io::ErrorKind::NotFound {
                    CliError::BinaryNotFound {
                        binary: self.config.binary_path.clone(),
                        install_hint: "Visit https://claude.ai/download".to_string(),
                    }
                } else {
                    CliError::Internal {
                        message: format!("Failed to run health check: {}", e),
                    }
                }
            })?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);

            // Check for auth errors
            if stderr.contains("not authenticated") || stderr.contains("login") {
                return Err(CliError::NotAuthenticated {
                    auth_command: "claude login".to_string(),
                });
            }

            return Err(CliError::ProcessFailed {
                code: output.status.code().unwrap_or(-1),
                stderr: stderr.to_string(),
            });
        }

        let version = String::from_utf8_lossy(&output.stdout);
        tracing::info!("Claude CLI health check passed: {}", version.trim());

        Ok(())
    }

    fn estimate_tokens(&self, conversation: &Conversation) -> usize {
        let mut total = 0;

        if let Some(system) = &conversation.system_prompt {
            total += CliContextManager::estimate_tokens(system);
        }

        for msg in &conversation.messages {
            total += CliContextManager::estimate_tokens(&msg.content);
        }

        total
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_health_check() {
        let executor = ClaudeCliExecutor::new(ClaudeCliConfig::default());

        // This will only pass if claude CLI is installed
        match executor.health_check().await {
            Ok(_) => println!("Claude CLI is available"),
            Err(CliError::BinaryNotFound { .. }) => {
                println!("Claude CLI not found (expected in CI)")
            }
            Err(e) => panic!("Unexpected error: {:?}", e),
        }
    }

    #[test]
    fn test_estimate_tokens() {
        let executor = ClaudeCliExecutor::new(ClaudeCliConfig::default());
        let conversation = Conversation {
            messages: vec![],
            system_prompt: Some("You are helpful.".to_string()),
            model: "claude-sonnet-4.5".to_string(),
        };

        let tokens = executor.estimate_tokens(&conversation);
        assert!(tokens > 0);
    }
}
