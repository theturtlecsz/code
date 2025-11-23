use async_trait::async_trait;
use tokio::io::AsyncWriteExt;
use tokio::process::Command;
use tokio::sync::mpsc;
use tokio::time::{Duration, timeout};

use super::CliExecutor;
use super::context::CliContextManager;
use super::stream::parse_gemini_stream;
use super::types::{CliError, Conversation, StreamEvent};

/// Configuration for Gemini CLI executor
#[derive(Debug, Clone)]
pub struct GeminiCliConfig {
    /// Path to gemini binary (default: "gemini")
    pub binary_path: String,
    /// Model to use (e.g., "gemini-2.5-pro", "gemini-2.0-flash")
    pub model: Option<String>,
    /// Timeout for requests (default: 120 seconds)
    pub timeout_secs: u64,
}

impl Default for GeminiCliConfig {
    fn default() -> Self {
        Self {
            binary_path: "gemini".to_string(),
            model: None,
            timeout_secs: 120,
        }
    }
}

/// Gemini CLI executor
///
/// Spawns external `gemini` CLI process and manages request/response lifecycle.
/// Handles rate limits automatically via CLI's built-in retry mechanism.
pub struct GeminiCliExecutor {
    config: GeminiCliConfig,
}

impl GeminiCliExecutor {
    pub fn new(config: GeminiCliConfig) -> Self {
        Self { config }
    }

    /// Build command args for Gemini CLI
    fn build_command_args(&self, conversation: &Conversation) -> Vec<String> {
        let mut args = vec![];

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

        // Add output format for structured responses
        args.push("--output-format".to_string());
        args.push("stream-json".to_string());

        args
    }
}

#[async_trait]
impl CliExecutor for GeminiCliExecutor {
    async fn execute(
        &self,
        conversation: &Conversation,
        user_message: &str,
    ) -> Result<mpsc::Receiver<StreamEvent>, CliError> {
        // NOTE: Gemini CLI routing currently DISABLED in production (reliability issues)
        // This code kept for reference only

        // Compress conversation if needed
        let conversation = CliContextManager::compress_if_needed(conversation, user_message);

        // Format history into prompt
        let prompt = CliContextManager::format_history(&conversation, user_message);

        tracing::debug!(
            "Executing Gemini CLI: {} chars, ~{} tokens",
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
                        install_hint:
                            "Run: npm install -g @google/gemini-cli && gemini (to authenticate)"
                                .to_string(),
                    }
                } else {
                    CliError::Internal {
                        message: format!("Failed to spawn Gemini CLI: {}", e),
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
            let parse_result = timeout(timeout_duration, parse_gemini_stream(stdout, tx_clone))
                .await
                .map_err(|_| CliError::Timeout {
                    elapsed: timeout_duration,
                });

            match parse_result {
                Ok(Ok(())) => {
                    tracing::debug!("Gemini stream parsing completed successfully");
                }
                Ok(Err(e)) => {
                    tracing::error!("Gemini stream parsing failed: {:?}", e);
                    let _ = tx.send(StreamEvent::Error(e)).await;
                }
                Err(timeout_err) => {
                    tracing::error!("Gemini request timed out: {:?}", timeout_err);
                    let _ = tx.send(StreamEvent::Error(timeout_err)).await;
                }
            }

            // Wait for process to exit and check status
            if let Ok(status) = child.wait().await {
                if !status.success() {
                    let code = status.code().unwrap_or(-1);
                    tracing::error!("Gemini CLI exited with code: {}", code);

                    // Try to read stderr for error details
                    if let Some(mut stderr) = child.stderr {
                        let mut stderr_content = String::new();
                        use tokio::io::AsyncReadExt;
                        if let Ok(_) = stderr.read_to_string(&mut stderr_content).await {
                            // Check for rate limit errors (CLI may have already retried)
                            if stderr_content.contains("exhausted")
                                || stderr_content.contains("429")
                            {
                                tracing::warn!(
                                    "Gemini rate limit encountered (CLI should auto-retry)"
                                );
                            }

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
                        install_hint:
                            "Run: npm install -g @google/gemini-cli && gemini (to authenticate)"
                                .to_string(),
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
                    auth_command: "gemini (then follow OAuth prompts)".to_string(),
                });
            }

            return Err(CliError::ProcessFailed {
                code: output.status.code().unwrap_or(-1),
                stderr: stderr.to_string(),
            });
        }

        let version = String::from_utf8_lossy(&output.stdout);
        tracing::info!("Gemini CLI health check passed: {}", version.trim());

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
        let executor = GeminiCliExecutor::new(GeminiCliConfig::default());

        // This will only pass if gemini CLI is installed
        match executor.health_check().await {
            Ok(_) => println!("Gemini CLI is available"),
            Err(CliError::BinaryNotFound { .. }) => {
                println!("Gemini CLI not found (expected in CI)")
            }
            Err(e) => panic!("Unexpected error: {:?}", e),
        }
    }

    #[test]
    fn test_estimate_tokens() {
        let executor = GeminiCliExecutor::new(GeminiCliConfig::default());
        let conversation = Conversation {
            messages: vec![],
            system_prompt: Some("You are helpful.".to_string()),
            model: "gemini-2.5-pro".to_string(),
        };

        let tokens = executor.estimate_tokens(&conversation);
        assert!(tokens > 0);
    }
}
