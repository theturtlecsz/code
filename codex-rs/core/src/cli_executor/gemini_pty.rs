//! Gemini PTY-based provider (Option F - Interactive CLI wrapper)
//!
//! Wraps Gemini CLI in interactive mode using a persistent PTY, letting the CLI
//! manage conversation state, tools, and memory instead of synthetic JSON state.
//!
//! Key benefits over headless mode:
//! - CLI owns conversation history (true multi-turn)
//! - Tool usage (search, shell, git) works natively
//! - Context compression (/compress) handled automatically
//! - Session checkpoints (/chat save/resume) available
//! - Persistent memory (GEMINI.md) automatic

use std::io::{Read, Write};
use std::time::{Duration, Instant};
use portable_pty::{CommandBuilder, PtySize, PtySystem};
use tokio::sync::mpsc;

use super::prompt_detector::PromptDetector;
use super::types::{CliError, StreamEvent};

/// Remove prompt marker from text
fn remove_prompt_marker(text: &str) -> String {
    let mut result = text.to_string();

    for marker in &["\n> ", "\ngemini> ", "> ", "gemini> "] {
        if result.ends_with(marker) {
            result.truncate(result.len() - marker.len());
            break;
        }
    }

    result
}

/// Configuration for Gemini PTY session
#[derive(Debug, Clone)]
pub struct GeminiPtyConfig {
    /// Path to gemini binary
    pub binary_path: String,

    /// Model to use (passed to --model flag)
    pub model: String,

    /// Maximum response timeout
    pub max_response_time: Duration,

    /// Prompt detection idle threshold
    pub idle_threshold: Duration,

    /// Auto-checkpoint interval (turns)
    pub auto_checkpoint_interval: Option<usize>,
}

impl Default for GeminiPtyConfig {
    fn default() -> Self {
        Self {
            binary_path: "gemini".to_string(),
            model: "gemini-2.5-flash".to_string(),
            max_response_time: Duration::from_secs(120),
            idle_threshold: Duration::from_millis(500),
            auto_checkpoint_interval: Some(5),
        }
    }
}

/// PTY-based Gemini CLI session
///
/// Manages a long-lived Gemini CLI process in interactive mode via PTY.
/// The CLI owns conversation state; we just drive it like a terminal.
pub struct GeminiPtySession {
    config: GeminiPtyConfig,
    pty_pair: Option<portable_pty::PtyPair>,
    child: Option<Box<dyn portable_pty::Child + Send>>,
    prompt_detector: PromptDetector,
    turn_count: usize,
    last_checkpoint: Option<String>,
    conversation_id: Option<String>,
}

impl GeminiPtySession {
    /// Create new session (doesn't start process yet)
    pub fn new(model: &str) -> Self {
        let mut config = GeminiPtyConfig::default();
        config.model = model.to_string();
        let idle_threshold = config.idle_threshold;

        Self {
            config,
            pty_pair: None,
            child: None,
            prompt_detector: PromptDetector::with_threshold(idle_threshold),
            turn_count: 0,
            last_checkpoint: None,
            conversation_id: None,
        }
    }

    /// Create with custom config
    pub fn with_config(config: GeminiPtyConfig) -> Self {
        let idle_threshold = config.idle_threshold;
        Self {
            prompt_detector: PromptDetector::with_threshold(idle_threshold),
            config,
            pty_pair: None,
            child: None,
            turn_count: 0,
            last_checkpoint: None,
            conversation_id: None,
        }
    }

    /// Start Gemini CLI process in interactive mode with REAL PTY
    pub async fn start(&mut self) -> Result<(), CliError> {
        tracing::info!("Starting Gemini CLI in PTY (model: {})", self.config.model);

        // Spawn in blocking task (portable-pty is blocking I/O)
        let binary_path = self.config.binary_path.clone();
        let model = self.config.model.clone();

        let (pty_pair, child) = tokio::task::spawn_blocking(move || {
            // Get native PTY system
            let pty_system = portable_pty::native_pty_system();

            // Open PTY with size
            let pty_pair = pty_system.openpty(PtySize {
                rows: 24,
                cols: 120,
                pixel_width: 0,
                pixel_height: 0,
            }).map_err(|e| {
                CliError::Internal {
                    message: format!("Failed to open PTY: {}", e),
                }
            })?;

            // Build command
            let mut cmd = CommandBuilder::new(&binary_path);
            cmd.arg("--model");
            cmd.arg(&model);

            // Spawn process in PTY
            let child = pty_pair.slave.spawn_command(cmd).map_err(|e| {
                let error_msg = format!("{}", e);
                if error_msg.contains("No such file") || error_msg.contains("not found") {
                    CliError::BinaryNotFound {
                        binary: binary_path.clone(),
                        install_hint: "Run: npm install -g @google/gemini-cli && gemini (to authenticate)"
                            .to_string(),
                    }
                } else {
                    CliError::Internal {
                        message: format!("Failed to spawn in PTY: {}", e),
                    }
                }
            })?;

            Ok::<_, CliError>((pty_pair, child))
        }).await.map_err(|e| {
            CliError::Internal {
                message: format!("PTY spawn task failed: {}", e),
            }
        })??;

        tracing::debug!("Gemini CLI spawned in PTY");

        self.pty_pair = Some(pty_pair);
        self.child = Some(child);

        // Wait for initial prompt (CLI initialization)
        self.wait_for_initial_prompt().await?;

        tracing::info!("Gemini CLI PTY session ready");
        Ok(())
    }

    /// Wait for Gemini CLI to finish initializing and show prompt
    async fn wait_for_initial_prompt(&mut self) -> Result<(), CliError> {
        tracing::debug!("Waiting for initial Gemini CLI prompt...");

        let deadline = Instant::now() + Duration::from_secs(10);
        let mut init_buffer = Vec::new();
        let mut buffer = vec![0u8; 4096];

        while Instant::now() < deadline {
            if let Some(pty) = &mut self.pty {
                match timeout(Duration::from_millis(100), pty.read(&mut buffer)).await {
                    Ok(Ok(n)) if n > 0 => {
                        init_buffer.extend_from_slice(&buffer[..n]);

                        // Strip ANSI and check for prompt
                        let stripped = strip_ansi_escapes::strip(&init_buffer);
                        let text = String::from_utf8_lossy(&stripped);

                        tracing::trace!("Init output: {}", text.trim());

                        // Check for prompt markers
                        if text.contains("> ") || text.contains("gemini>") {
                            tracing::info!("Initial prompt detected");
                            return Ok(());
                        }
                    }
                    Ok(Ok(_)) => {
                        // EOF - process exited
                        return Err(CliError::Internal {
                            message: "CLI exited during initialization".to_string(),
                        });
                    }
                    Ok(Err(e)) => {
                        return Err(CliError::Internal {
                            message: format!("Failed to read init output: {}", e),
                        });
                    }
                    Err(_) => {
                        // Timeout on this read, continue waiting
                        tokio::time::sleep(Duration::from_millis(50)).await;
                    }
                }
            }
        }

        Err(CliError::Timeout {
            elapsed: Duration::from_secs(10),
        })
    }

    /// Send user message and stream response
    pub async fn send_message(
        &mut self,
        message: &str,
        tx: mpsc::Sender<StreamEvent>,
    ) -> Result<String, CliError> {
        // Ensure process is alive
        self.ensure_alive().await?;

        // Reset prompt detector for new turn
        self.prompt_detector.reset();
        self.turn_count += 1;

        tracing::debug!("Sending message (turn {}): {}", self.turn_count,
            if message.len() > 50 { &format!("{}...", &message[..50]) } else { message });

        // Write message to PTY
        if let Some(pty) = &mut self.pty {
            pty.write_all(message.as_bytes()).await.map_err(|e| {
                CliError::Internal {
                    message: format!("Failed to write to PTY: {}", e),
                }
            })?;

            pty.write_all(b"\n").await.map_err(|e| {
                CliError::Internal {
                    message: format!("Failed to write newline: {}", e),
                }
            })?;

            pty.flush().await.map_err(|e| {
                CliError::Internal {
                    message: format!("Failed to flush PTY: {}", e),
                }
            })?;
        } else {
            return Err(CliError::Internal {
                message: "PTY not initialized".to_string(),
            });
        }

        // Read and stream response
        let response = self.read_and_stream_response(tx).await?;

        // Auto-checkpoint if configured
        if let Some(interval) = self.config.auto_checkpoint_interval {
            if self.turn_count % interval == 0 {
                self.auto_checkpoint().await?;
            }
        }

        Ok(response)
    }

    /// Read PTY output and stream to channel until prompt detected
    async fn read_and_stream_response(
        &mut self,
        tx: mpsc::Sender<StreamEvent>,
    ) -> Result<String, CliError> {
        let mut accumulated = String::new();
        let mut buffer = vec![0u8; 4096];
        let deadline = Instant::now() + self.config.max_response_time;

        if let Some(pty) = &mut self.pty {
            loop {
                // Check timeout
                if Instant::now() > deadline {
                    tracing::error!("Response timeout after {:?}", self.config.max_response_time);
                    return Err(CliError::Timeout {
                        elapsed: self.config.max_response_time,
                    });
                }

                // Non-blocking read with short timeout
                match timeout(Duration::from_millis(100), pty.read(&mut buffer)).await {
                    Ok(Ok(n)) if n > 0 => {
                        // Got data
                        let raw_bytes = &buffer[..n];

                        // Strip ANSI codes
                        let stripped = strip_ansi_escapes::strip(raw_bytes);

                        let clean_text = String::from_utf8_lossy(&stripped).to_string();

                        // Update prompt detector
                        self.prompt_detector.update(&clean_text);

                        // Skip prompt markers in output
                        let delta = if self.prompt_detector.is_complete(&accumulated) {
                            // Remove prompt marker from end
                            remove_prompt_marker(&clean_text)
                        } else {
                            clean_text.clone()
                        };

                        // Emit delta if not empty
                        if !delta.is_empty() {
                            let _ = tx.send(StreamEvent::Delta(delta.clone())).await;
                            accumulated.push_str(&delta);
                        }

                        // Check if response complete
                        if self.prompt_detector.is_complete(&accumulated) {
                            tracing::debug!("Response complete (confidence: {:?}, {} chars)",
                                self.prompt_detector.confidence(), accumulated.len());
                            break;
                        }
                    }
                    Ok(Ok(_)) => {
                        // EOF - process died
                        tracing::error!("Gemini CLI process exited unexpectedly");
                        return Err(CliError::Internal {
                            message: "CLI process died during response".to_string(),
                        });
                    }
                    Ok(Err(e)) => {
                        return Err(CliError::Internal {
                            message: format!("Read error: {}", e),
                        });
                    }
                    Err(_) => {
                        // Timeout on this read - check if we're done
                        if self.prompt_detector.is_complete(&accumulated) {
                            break;
                        }
                        // Otherwise continue waiting
                    }
                }
            }
        } else {
            return Err(CliError::Internal {
                message: "PTY not initialized".to_string(),
            });
        }

        // Send done event
        let _ = tx.send(StreamEvent::Done).await;

        Ok(accumulated.trim_end().to_string())
    }


    /// Send CLI command (e.g., /compress, /chat save)
    pub async fn send_command(&mut self, command: &str) -> Result<(), CliError> {
        tracing::debug!("Sending CLI command: {}", command);

        if let Some(pty) = &mut self.pty {
            pty.write_all(command.as_bytes()).await.map_err(|e| {
                CliError::Internal {
                    message: format!("Failed to write command: {}", e),
                }
            })?;

            pty.write_all(b"\n").await.map_err(|e| {
                CliError::Internal {
                    message: format!("Failed to write newline: {}", e),
                }
            })?;

            pty.flush().await.map_err(|e| {
                CliError::Internal {
                    message: format!("Failed to flush: {}", e),
                }
            })?;
        }

        // Wait briefly for command to execute
        tokio::time::sleep(Duration::from_millis(100)).await;

        Ok(())
    }

    /// Auto-checkpoint conversation
    async fn auto_checkpoint(&mut self) -> Result<(), CliError> {
        let checkpoint_id = format!("auto_{}", self.turn_count);

        tracing::info!("Creating auto-checkpoint: {}", checkpoint_id);

        self.send_command(&format!("/chat save {}", checkpoint_id)).await?;
        self.last_checkpoint = Some(checkpoint_id);

        Ok(())
    }

    /// Cancel current generation (send Ctrl+C)
    pub async fn cancel(&mut self) -> Result<(), CliError> {
        tracing::info!("Cancelling current generation");

        if let Some(pty) = &mut self.pty {
            // Send Ctrl+C (ASCII 0x03)
            pty.write_all(&[0x03]).await.map_err(|e| {
                CliError::Internal {
                    message: format!("Failed to send Ctrl+C: {}", e),
                }
            })?;

            pty.flush().await.map_err(|e| {
                CliError::Internal {
                    message: format!("Failed to flush: {}", e),
                }
            })?;
        }

        // Drain output until prompt returns
        self.drain_until_prompt().await?;

        // Reset for next turn
        self.prompt_detector.reset();

        Ok(())
    }

    /// Drain PTY output until prompt detected
    async fn drain_until_prompt(&mut self) -> Result<(), CliError> {
        let deadline = Instant::now() + Duration::from_secs(5);
        let mut drain_buffer = vec![0u8; 4096];

        if let Some(pty) = &mut self.pty {
            while Instant::now() < deadline {
                match timeout(Duration::from_millis(100), pty.read(&mut drain_buffer)).await {
                    Ok(Ok(n)) if n > 0 => {
                        let stripped = strip_ansi_escapes::strip(&drain_buffer[..n]);
                        let text = String::from_utf8_lossy(&stripped);

                        tracing::trace!("Draining: {}", text.trim());

                        // Check for prompt
                        if text.contains("> ") || text.contains("gemini>") {
                            tracing::debug!("Prompt detected during drain");
                            return Ok(());
                        }
                    }
                    Ok(Ok(_)) => {
                        // EOF
                        return Err(CliError::Internal {
                            message: "Process died during drain".to_string(),
                        });
                    }
                    Ok(Err(e)) => {
                        return Err(CliError::Internal {
                            message: format!("Drain error: {}", e),
                        });
                    }
                    Err(_) => {
                        // Timeout - continue waiting
                        tokio::time::sleep(Duration::from_millis(50)).await;
                    }
                }
            }
        }

        Err(CliError::Timeout {
            elapsed: Duration::from_secs(5),
        })
    }

    /// Ensure process is alive, restart if needed
    pub async fn ensure_alive(&mut self) -> Result<(), CliError> {
        if !self.is_alive() {
            tracing::warn!("Gemini CLI process died, restarting...");

            // Restart
            self.start().await?;

            // Try to restore from last checkpoint
            if let Some(checkpoint) = &self.last_checkpoint {
                tracing::info!("Restoring from checkpoint: {}", checkpoint);
                self.send_command(&format!("/chat resume {}", checkpoint)).await?;
            } else {
                tracing::warn!("No checkpoint available, conversation state lost");
            }
        }

        Ok(())
    }

    /// Check if process is still running
    pub fn is_alive(&mut self) -> bool {
        if let Some(child) = &mut self.child {
            match child.try_wait() {
                Ok(Some(_)) => false, // Process exited
                Ok(None) => true,     // Still running
                Err(_) => false,      // Error checking status
            }
        } else {
            false
        }
    }

    /// Gracefully shutdown CLI
    pub async fn shutdown(mut self) -> Result<(), CliError> {
        tracing::info!("Shutting down Gemini CLI session");

        // Send /quit command
        if self.is_alive() {
            if let Err(e) = self.send_command("/quit").await {
                tracing::warn!("Failed to send /quit command: {}", e);
            }

            // Wait briefly for graceful exit
            tokio::time::sleep(Duration::from_millis(500)).await;
        }

        // Force kill if still alive
        if let Some(mut child) = self.child {
            if child.try_wait().ok().flatten().is_none() {
                tracing::debug!("Force killing Gemini CLI process");
                let _ = child.kill();
            }
        }

        Ok(())
    }

    /// Get session statistics
    pub fn stats(&self) -> SessionStats {
        SessionStats {
            turn_count: self.turn_count,
            last_checkpoint: self.last_checkpoint.clone(),
            is_alive: false, // Can't check without &mut
            conversation_id: self.conversation_id.clone(),
        }
    }
}

/// Session statistics
#[derive(Debug, Clone)]
pub struct SessionStats {
    pub turn_count: usize,
    pub last_checkpoint: Option<String>,
    pub is_alive: bool,
    pub conversation_id: Option<String>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_session_creation() {
        let session = GeminiPtySession::new("gemini-2.5-flash");
        assert_eq!(session.config.model, "gemini-2.5-flash");
        assert_eq!(session.turn_count, 0);
        assert!(session.last_checkpoint.is_none());
    }

    #[test]
    fn test_config_defaults() {
        let config = GeminiPtyConfig::default();
        assert_eq!(config.binary_path, "gemini");
        assert_eq!(config.max_response_time, Duration::from_secs(120));
        assert_eq!(config.idle_threshold, Duration::from_millis(500));
        assert_eq!(config.auto_checkpoint_interval, Some(5));
    }

    #[test]
    fn test_remove_prompt_marker() {
        assert_eq!(remove_prompt_marker("4\n> "), "4");
        assert_eq!(remove_prompt_marker("Response\ngemini> "), "Response");
        assert_eq!(remove_prompt_marker("No prompt"), "No prompt");
    }
}
