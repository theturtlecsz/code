//! Gemini PTY-based provider (Option F - Interactive CLI wrapper)
//!
//! Wraps Gemini CLI in interactive mode using a persistent PTY via expectrl,
//! letting the CLI manage conversation state, tools, and memory.
//!
//! Key benefits over headless mode:
//! - CLI owns conversation history (true multi-turn)
//! - Tool usage (search, shell, git) works natively
//! - Context compression (/compress) handled automatically
//! - Session checkpoints (/chat save/resume) available
//! - Persistent memory (GEMINI.md) automatic
//!
//! ## Architecture
//!
//! ```text
//! GeminiPtyProvider (async)
//!   └─> spawn_blocking
//!       └─> GeminiPtySession (sync)
//!           └─> expectrl::Session (smol async, used synchronously)
//!               └─> gemini CLI (interactive mode)
//! ```
//!
//! Pipe-based approach: NOT using PTY due to Gemini CLI UI requirements.
//! Uses plain stdin/stdout pipes with careful output filtering.

use std::io::{BufRead, BufReader, Write};
use std::process::{Child, ChildStdin, ChildStdout};
use std::time::{Duration, Instant};

use tokio::sync::mpsc;
use tokio_util::sync::CancellationToken;

use super::prompt_detector::PromptDetector;
use super::types::{CliError, StreamEvent};

/// Remove prompt marker from end of text
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

    /// Auto-checkpoint interval (turns), None to disable
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

/// Pipe-based Gemini CLI session
///
/// Manages a long-lived Gemini CLI process via stdin/stdout pipes.
/// The CLI owns conversation state; we write messages to stdin and read from stdout.
///
/// ## Important: Blocking I/O
///
/// This type uses **synchronous** I/O.
/// All methods must be called from `tokio::task::spawn_blocking`.
pub struct GeminiPtySession {
    config: GeminiPtyConfig,
    child: Option<Child>,
    stdin: Option<ChildStdin>,
    stdout: Option<BufReader<ChildStdout>>,
    prompt_detector: PromptDetector,
    turn_count: usize,
    last_checkpoint: Option<String>,
    conversation_id: Option<String>,
}

impl GeminiPtySession {
    /// Create new session (doesn't start process yet)
    pub fn new(model: &str) -> Self {
        let config = GeminiPtyConfig {
            model: model.to_string(),
            ..Default::default()
        };
        let idle_threshold = config.idle_threshold;

        Self {
            config,
            child: None,
            stdin: None,
            stdout: None,
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
            child: None,
            stdin: None,
            stdout: None,
            turn_count: 0,
            last_checkpoint: None,
            conversation_id: None,
        }
    }

    /// Start Gemini CLI process with pipes (stdin/stdout)
    ///
    /// **BLOCKING**: Must be called from `spawn_blocking`
    pub fn start(&mut self) -> Result<(), CliError> {
        tracing::info!(
            "Starting Gemini CLI with pipes (model: {})",
            self.config.model
        );

        use std::io::BufReader;
        use std::process::{Command, Stdio};

        // Spawn with piped stdin/stdout, CAPTURE stderr to see errors
        let mut child = Command::new(&self.config.binary_path)
            .arg("--model")
            .arg(&self.config.model)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped()) // ✅ CAPTURE stderr instead of null
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
                        message: format!("Failed to spawn gemini: {e}"),
                    }
                }
            })?;

        // Capture stderr in a thread so we can log it
        let Some(stderr) = child.stderr.take() else {
            return Err(CliError::Internal {
                message: "Failed to capture stderr".to_string(),
            });
        };
        std::thread::spawn(move || {
            let reader = BufReader::new(stderr);
            use std::io::BufRead;
            for line in reader.lines() {
                if let Ok(line) = line
                    && !line.is_empty()
                {
                    tracing::warn!("Gemini CLI stderr: {}", line);
                }
            }
        });

        let stdin = child.stdin.take().ok_or_else(|| CliError::Internal {
            message: "Failed to capture stdin".to_string(),
        })?;

        let stdout = child.stdout.take().ok_or_else(|| CliError::Internal {
            message: "Failed to capture stdout".to_string(),
        })?;

        // Store process handle and I/O streams
        self.child = Some(child);
        self.stdin = Some(stdin);
        self.stdout = Some(BufReader::new(stdout));

        // ✅ Consume initial startup output and wait for first prompt
        self.consume_until_prompt("startup")?;

        tracing::info!(
            "Gemini CLI process ready (PID: {:?})",
            self.child.as_ref().map(std::process::Child::id)
        );
        Ok(())
    }

    /// Consume output until we see a prompt marker
    ///
    /// **BLOCKING**: Must be called from `spawn_blocking`
    ///
    /// Reads and discards output until we detect a prompt ("> " or "gemini> ").
    /// This is critical for:
    /// - Startup: consume welcome messages before first user message
    /// - After response: ensure prompt is consumed before next message
    fn consume_until_prompt(&mut self, context: &str) -> Result<(), CliError> {
        let stdout = self.stdout.as_mut().ok_or_else(|| CliError::Internal {
            message: "Stdout not available".to_string(),
        })?;

        let deadline = Instant::now() + Duration::from_secs(10); // 10s timeout for prompt
        let mut accumulated = String::new();

        tracing::debug!("Consuming output until prompt ({})...", context);

        loop {
            if Instant::now() > deadline {
                return Err(CliError::Timeout {
                    elapsed: Duration::from_secs(10),
                });
            }

            let mut line = String::new();
            match stdout.read_line(&mut line) {
                Ok(0) => {
                    // EOF - process died
                    return Err(CliError::Internal {
                        message: "CLI process died while waiting for prompt".to_string(),
                    });
                }
                Ok(_) => {
                    // Clean ANSI codes
                    let bytes = line.as_bytes();
                    let stripped = strip_ansi_escapes::strip(bytes);
                    let clean = String::from_utf8_lossy(&stripped).to_string();

                    accumulated.push_str(&clean);

                    // Check for prompt markers
                    if clean.trim().ends_with('>')
                        || accumulated.contains("\n> ")
                        || accumulated.contains("\ngemini> ")
                        || clean.trim() == ">"
                        || clean.trim() == "gemini>"
                    {
                        tracing::debug!(
                            "Prompt detected after {} bytes in {}",
                            accumulated.len(),
                            context
                        );
                        return Ok(());
                    }
                }
                Err(e) => {
                    return Err(CliError::Internal {
                        message: format!("Read error while waiting for prompt: {e}"),
                    });
                }
            }

            // Brief sleep to avoid busy loop
            std::thread::sleep(Duration::from_millis(10));
        }
    }

    /// Send user message and stream response
    ///
    /// **BLOCKING**: Must be called from `spawn_blocking`
    ///
    /// The `tx` channel runs asynchronously via Tokio, but we send to it
    /// synchronously using `blocking_send()`.
    pub fn send_message(
        &mut self,
        message: &str,
        tx: mpsc::Sender<StreamEvent>,
        cancel: CancellationToken,
    ) -> Result<String, CliError> {
        // Ensure process is alive
        self.ensure_alive()?;

        // Reset prompt detector for new turn
        self.prompt_detector.reset();
        self.turn_count += 1;

        tracing::debug!(
            "Sending message (turn {}): {}",
            self.turn_count,
            if message.len() > 50 {
                format!("{}...", &message[..50])
            } else {
                message.to_string()
            }
        );

        // Write message to stdin
        let stdin = self.stdin.as_mut().ok_or_else(|| CliError::Internal {
            message: "Stdin not available".to_string(),
        })?;

        writeln!(stdin, "{message}").map_err(|e| CliError::Internal {
            message: format!("Failed to write to stdin: {e}"),
        })?;

        stdin.flush().map_err(|e| CliError::Internal {
            message: format!("Failed to flush stdin: {e}"),
        })?;

        // Read and stream response from stdout
        let response = self.read_and_stream_response(tx, cancel)?;

        // Auto-checkpoint if configured
        if let Some(interval) = self.config.auto_checkpoint_interval
            && self.turn_count.is_multiple_of(interval)
        {
            self.auto_checkpoint()?;
        }

        Ok(response)
    }

    /// Read stdout and stream to channel until response complete
    ///
    /// **BLOCKING**: Must be called from `spawn_blocking`
    ///
    /// Gemini CLI via stdin outputs response lines then returns to waiting.
    /// We detect completion via empty line or idle timeout.
    fn read_and_stream_response(
        &mut self,
        tx: mpsc::Sender<StreamEvent>,
        cancel: CancellationToken,
    ) -> Result<String, CliError> {
        let mut accumulated = String::new();
        let deadline = Instant::now() + self.config.max_response_time;

        let stdout = self.stdout.as_mut().ok_or_else(|| CliError::Internal {
            message: "Stdout not available".to_string(),
        })?;

        let mut last_activity = Instant::now();
        let idle_threshold = self.config.idle_threshold;

        loop {
            // Check cancellation
            if cancel.is_cancelled() {
                tracing::info!("Response cancelled by user");
                // ✅ Strip prompt markers before returning
                let clean = remove_prompt_marker(accumulated.trim_end());
                return Ok(clean);
            }

            // Check timeout
            if Instant::now() > deadline {
                tracing::error!("Response timeout after {:?}", self.config.max_response_time);
                return Err(CliError::Timeout {
                    elapsed: self.config.max_response_time,
                });
            }

            // Check idle timeout (no new data for idle_threshold duration)
            if Instant::now().duration_since(last_activity) > idle_threshold
                && !accumulated.is_empty()
            {
                tracing::debug!("Idle timeout reached, treating as complete");
                break;
            }

            // Try to read a line (non-blocking via peek check)
            let mut line = String::new();
            match stdout.read_line(&mut line) {
                Ok(0) => {
                    // EOF - process exited
                    if accumulated.is_empty() {
                        return Err(CliError::Internal {
                            message: "CLI process exited without response".to_string(),
                        });
                    }
                    break;
                }
                Ok(_) => {
                    // Got a line
                    last_activity = Instant::now();

                    // Clean ANSI codes
                    let bytes = line.as_bytes();
                    let stripped = strip_ansi_escapes::strip(bytes);
                    let clean_text = String::from_utf8_lossy(&stripped).to_string();

                    // Update detector BEFORE adding to accumulated
                    self.prompt_detector.update(&clean_text);

                    // ✅ Check if this line contains a prompt marker
                    let is_prompt_line = clean_text.trim().ends_with('>')
                        || clean_text.trim() == ">"
                        || clean_text.trim() == "gemini>";

                    // Only add to accumulated if NOT a prompt-only line
                    if !is_prompt_line {
                        accumulated.push_str(&clean_text);

                        // Emit delta for non-prompt lines
                        if !clean_text.trim().is_empty() {
                            let _ = tx.blocking_send(StreamEvent::Delta(clean_text));
                        }
                    } else {
                        // This is the prompt line - response is complete
                        tracing::debug!("Prompt line detected, response complete");
                        break;
                    }

                    // Check completion via detector (backup mechanism)
                    if self.prompt_detector.is_complete(&accumulated) {
                        tracing::debug!("Response complete via prompt detector");
                        break;
                    }
                }
                Err(e) => {
                    return Err(CliError::Internal {
                        message: format!("Read error: {e}"),
                    });
                }
            }

            // Brief sleep to avoid busy loop
            std::thread::sleep(Duration::from_millis(10));
        }

        // ✅ Strip any trailing prompt markers that might have leaked through
        let clean_response = remove_prompt_marker(accumulated.trim_end());

        // Send done event
        let _ = tx.blocking_send(StreamEvent::Done);

        Ok(clean_response)
    }

    /// Send CLI command (e.g., /compress, /chat save)
    ///
    /// **BLOCKING**: Must be called from `spawn_blocking`
    pub fn send_command(&mut self, command: &str) -> Result<(), CliError> {
        tracing::debug!("Sending CLI command: {}", command);

        let stdin = self.stdin.as_mut().ok_or_else(|| CliError::Internal {
            message: "Stdin not available".to_string(),
        })?;

        writeln!(stdin, "{command}").map_err(|e| CliError::Internal {
            message: format!("Failed to write command: {e}"),
        })?;

        stdin.flush().map_err(|e| CliError::Internal {
            message: format!("Failed to flush stdin: {e}"),
        })?;

        // ✅ Wait for command to complete and prompt to return
        // Commands like /chat save, /compress produce output then return to prompt
        self.consume_until_prompt(&format!("after command: {command}"))?;

        Ok(())
    }

    /// Auto-checkpoint conversation
    ///
    /// **BLOCKING**: Must be called from `spawn_blocking`
    fn auto_checkpoint(&mut self) -> Result<(), CliError> {
        let checkpoint_id = format!("auto_{}", self.turn_count);

        tracing::info!("Creating auto-checkpoint: {}", checkpoint_id);

        self.send_command(&format!("/chat save {checkpoint_id}"))?;
        self.last_checkpoint = Some(checkpoint_id);

        Ok(())
    }

    /// Cancel current generation (send Ctrl+C)
    ///
    /// **BLOCKING**: Must be called from `spawn_blocking`
    #[allow(dead_code)]
    fn cancel_internal(&mut self) -> Result<(), CliError> {
        tracing::info!("Cancelling current generation");

        // For piped stdin/stdout, cancellation is handled by the read loop
        // checking the cancel token and returning early. No need to signal process.

        self.prompt_detector.reset();
        Ok(())
    }

    /// Ensure process is alive, restart if needed
    ///
    /// **BLOCKING**: Must be called from `spawn_blocking`
    pub fn ensure_alive(&mut self) -> Result<(), CliError> {
        if !self.is_alive() {
            tracing::warn!("Gemini CLI process died, restarting...");

            // Restart
            self.start()?;

            // Try to restore from last checkpoint
            if let Some(checkpoint) = &self.last_checkpoint.clone() {
                tracing::info!("Restoring from checkpoint: {}", checkpoint);
                self.send_command(&format!("/chat resume {checkpoint}"))?;
            } else {
                tracing::warn!("No checkpoint available, conversation state lost");
            }
        }

        Ok(())
    }

    /// Check if process is still running
    ///
    /// **BLOCKING**: Can be called from anywhere
    pub fn is_alive(&mut self) -> bool {
        if let Some(child) = &mut self.child {
            match child.try_wait() {
                Ok(None) => true,              // Still running
                Ok(Some(_)) | Err(_) => false, // Exited or error
            }
        } else {
            false
        }
    }

    /// Gracefully shutdown CLI
    ///
    /// **BLOCKING**: Must be called from `spawn_blocking`
    pub fn shutdown(mut self) -> Result<(), CliError> {
        tracing::info!("Shutting down Gemini CLI session");

        // Close stdin to signal end
        drop(self.stdin.take());

        // Send /quit command if process still alive
        if self.is_alive() {
            // Wait briefly for graceful exit
            std::thread::sleep(Duration::from_millis(500));
        }

        // Force kill if still alive
        if let Some(mut child) = self.child
            && let Ok(None) = child.try_wait()
        {
            tracing::debug!("Force killing Gemini CLI process");
            let _ = child.kill();
            let _ = child.wait();
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
    // SPEC-957: Allow test code flexibility
    #![allow(clippy::print_stdout, clippy::print_stderr)]

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

    // Integration tests (require Gemini CLI installed)
    #[tokio::test]
    #[ignore] // Only run with: cargo test --ignored
    async fn test_single_turn_pty() {
        use tokio::sync::mpsc;

        let result = tokio::task::spawn_blocking(|| {
            let mut session = GeminiPtySession::new("gemini-2.5-flash");
            session.start()?;

            let (tx, rx) = mpsc::channel(32);
            let cancel = CancellationToken::new();

            let response = session.send_message("Say exactly: Hello World", tx, cancel)?;

            Ok::<String, CliError>(response)
        })
        .await
        .unwrap();

        match result {
            Ok(response) => {
                println!("Response: {response}");
                assert!(
                    response.contains("Hello World") || response.contains("Hello"),
                    "Expected response to contain 'Hello World' or 'Hello', got: {response}"
                );
            }
            Err(e) => {
                panic!("Test failed: {e:?}");
            }
        }
    }

    #[tokio::test]
    #[ignore]
    async fn test_multi_turn_state() {
        let result = tokio::task::spawn_blocking(|| {
            let mut session = GeminiPtySession::new("gemini-2.5-flash");
            session.start()?;

            // Turn 1: Set name
            let (tx1, _rx1) = mpsc::channel(32);
            let cancel1 = CancellationToken::new();
            session.send_message("My name is Alice.", tx1, cancel1)?;

            // Turn 2: Recall name
            let (tx2, _rx2) = mpsc::channel(32);
            let cancel2 = CancellationToken::new();
            let response = session.send_message("What's my name?", tx2, cancel2)?;

            Ok::<String, CliError>(response)
        })
        .await
        .unwrap();

        match result {
            Ok(response) => {
                println!("Response: {response}");
                assert!(
                    response.contains("Alice"),
                    "Expected response to contain 'Alice', got: {response}"
                );
            }
            Err(e) => {
                panic!("Test failed: {e:?}");
            }
        }
    }
}

//
// ═══════════════════════════════════════════════════════════════════════════
// ASYNC PROVIDER API
// ═══════════════════════════════════════════════════════════════════════════
//

use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::Mutex;

/// Conversation ID type for session management
pub type ConversationId = String;

/// Gemini PTY Provider - async interface over blocking sessions
///
/// Manages a pool of `GeminiPtySession` instances, one per conversation.
/// Provides async API that bridges to blocking PTY operations via `spawn_blocking`.
///
/// ## Usage
///
/// ```no_run
/// # use codex_core::cli_executor::gemini_pty::{GeminiPtyProvider, ConversationId};
/// # use tokio::sync::mpsc;
/// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
/// let provider = GeminiPtyProvider::new();
///
/// let conv_id = "conv_123".to_string();
/// let model = "gemini-2.5-flash";
///
/// let mut rx = provider.send_message(conv_id.clone(), "Hello".to_string(), model).await?;
///
/// while let Some(event) = rx.recv().await {
///     // Handle stream events
/// }
/// # Ok(())
/// # }
/// ```
pub struct GeminiPtyProvider {
    /// Map of conversation ID to session
    /// Wrapped in Arc<Mutex> for async access
    sessions: Arc<Mutex<HashMap<ConversationId, GeminiPtySession>>>,
}

impl GeminiPtyProvider {
    /// Create new provider with empty session pool
    pub fn new() -> Self {
        Self {
            sessions: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    /// Send message to a conversation (async API)
    ///
    /// Gets or creates a session for the conversation, then sends the message
    /// in a blocking task. Returns a channel that streams response events.
    ///
    /// # Arguments
    /// * `conv_id` - Conversation identifier
    /// * `message` - User message text
    /// * `model` - Model to use (if creating new session)
    ///
    /// # Returns
    /// Receiver channel for `StreamEvent` stream
    pub async fn send_message(
        &self,
        conv_id: ConversationId,
        message: String,
        model: &str,
    ) -> Result<mpsc::Receiver<StreamEvent>, CliError> {
        // Get or create session
        let mut sessions = self.sessions.lock().await;

        let session_exists = sessions.contains_key(&conv_id);

        if !session_exists {
            // Create new session (blocking operation)
            let model_owned = model.to_string();
            let session = tokio::task::spawn_blocking(move || {
                let mut session = GeminiPtySession::new(&model_owned);
                session.start()?;
                Ok::<_, CliError>(session)
            })
            .await
            .map_err(|e| CliError::Internal {
                message: format!("Session spawn task failed: {e}"),
            })??;

            sessions.insert(conv_id.clone(), session);
        }

        // Drop lock before blocking task (we can't hold lock across await)
        drop(sessions);

        // Create channel for streaming
        let (tx, rx) = mpsc::channel(32);
        let cancel = CancellationToken::new();

        // Spawn blocking task for message send
        let sessions_clone = Arc::clone(&self.sessions);
        let conv_id_clone = conv_id.clone();

        tokio::task::spawn(async move {
            // Get session with mut access
            let mut sessions = sessions_clone.lock().await;
            let mut session =
                sessions
                    .remove(&conv_id_clone)
                    .ok_or_else(|| CliError::Internal {
                        message: "Session disappeared".to_string(),
                    })?;
            drop(sessions); // Release lock before blocking operation

            // Run send_message in blocking task
            let result = tokio::task::spawn_blocking(move || {
                let response_result = session.send_message(&message, tx, cancel);
                (session, response_result)
            })
            .await;

            // Put session back regardless of result
            match result {
                Ok((session, _response_result)) => {
                    let mut sessions = sessions_clone.lock().await;
                    sessions.insert(conv_id_clone, session);
                }
                Err(e) => {
                    tracing::error!("Session task panicked: {}", e);
                }
            }

            Ok::<(), CliError>(())
        });

        Ok(rx)
    }

    /// Cancel ongoing generation for a conversation
    ///
    /// Sends Ctrl-C to the PTY session and drains until prompt returns.
    pub async fn cancel_generation(&self, _conv_id: &ConversationId) -> Result<(), CliError> {
        // TODO: Implement cancellation via CancellationToken
        // For now, cancellation is handled within send_message via the token
        tracing::warn!("Cancel generation called but not yet fully implemented");
        Ok(())
    }

    /// Close a conversation session
    ///
    /// Gracefully shuts down the PTY session and removes it from the pool.
    pub async fn close_conversation(&self, conv_id: &ConversationId) -> Result<(), CliError> {
        let mut sessions = self.sessions.lock().await;

        if let Some(session) = sessions.remove(conv_id) {
            tokio::task::spawn_blocking(move || session.shutdown())
                .await
                .map_err(|e| CliError::Internal {
                    message: format!("Shutdown task failed: {e}"),
                })??;
        }

        Ok(())
    }

    /// Shutdown all sessions
    ///
    /// Gracefully closes all active PTY sessions.
    pub async fn shutdown_all(&self) -> Result<(), CliError> {
        let mut sessions = self.sessions.lock().await;
        let all_sessions: Vec<_> = sessions.drain().collect();

        drop(sessions); // Release lock before blocking

        for (_id, session) in all_sessions {
            tokio::task::spawn_blocking(move || session.shutdown())
                .await
                .map_err(|e| CliError::Internal {
                    message: format!("Shutdown task failed: {e}"),
                })??;
        }

        Ok(())
    }

    /// Get active session count
    pub async fn active_session_count(&self) -> usize {
        self.sessions.lock().await.len()
    }

    /// Health check: verify Gemini CLI is available
    pub fn is_available() -> bool {
        which::which("gemini").is_ok()
    }

    /// Get installation instructions
    pub fn install_instructions() -> &'static str {
        "Install Gemini CLI:\n  \
         npm install -g @google/gemini-cli\n\n\
         Then authenticate by running:\n  \
         gemini\n\n\
         Follow the prompts to complete authentication."
    }
}

impl Default for GeminiPtyProvider {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod provider_tests {
    // SPEC-957: Allow test code flexibility
    #![allow(clippy::print_stdout, clippy::print_stderr)]
    #![allow(clippy::uninlined_format_args)]

    use super::*;

    #[tokio::test]
    async fn test_provider_creation() {
        let provider = GeminiPtyProvider::new();
        assert_eq!(provider.active_session_count().await, 0);
    }

    #[tokio::test]
    async fn test_health_check() {
        let available = GeminiPtyProvider::is_available();
        if available {
            println!("Gemini CLI is available");
        } else {
            println!("Gemini CLI not found (expected in CI)");
        }
    }

    #[tokio::test]
    #[ignore] // Requires Gemini CLI
    async fn test_provider_single_message() {
        let provider = GeminiPtyProvider::new();
        let conv_id = "test_conv_1".to_string();

        let mut rx = provider
            .send_message(conv_id.clone(), "Say: Test".to_string(), "gemini-2.5-flash")
            .await
            .expect("Failed to send message");

        let mut response = String::new();
        while let Some(event) = rx.recv().await {
            if let StreamEvent::Delta(text) = event {
                response.push_str(&text);
            }
        }

        println!("Response: {response}");
        assert!(!response.is_empty());

        // Cleanup
        provider.close_conversation(&conv_id).await.unwrap();
    }

    #[tokio::test]
    #[ignore]
    async fn test_provider_multi_turn() {
        let provider = GeminiPtyProvider::new();
        let conv_id = "test_conv_2".to_string();

        // Turn 1
        let mut rx1 = provider
            .send_message(
                conv_id.clone(),
                "My name is Bob.".to_string(),
                "gemini-2.5-flash",
            )
            .await
            .unwrap();

        while let Some(_) = rx1.recv().await {}

        // Turn 2
        let mut rx2 = provider
            .send_message(
                conv_id.clone(),
                "What's my name?".to_string(),
                "gemini-2.5-flash",
            )
            .await
            .unwrap();

        let mut response = String::new();
        while let Some(event) = rx2.recv().await {
            if let StreamEvent::Delta(text) = event {
                response.push_str(&text);
            }
        }

        println!("Response: {response}");
        assert!(response.contains("Bob"));

        // Cleanup
        provider.close_conversation(&conv_id).await.unwrap();
    }
}
