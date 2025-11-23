//! Gemini CLI Session-Based Integration
//!
//! This module implements Gemini CLI integration using its session management
//! feature. Unlike PTY approaches (which trigger Gemini's full TUI) or long-lived
//! pipes (which hang in interactive mode), this uses one-shot processes with
//! session resumption.
//!
//! ## Architecture
//!
//! ```text
//! TUI ModelProvider
//!   └─> GeminiPipesProvider (async)
//!       └─> GeminiPipesSession (per conversation)
//!           ├─ session_id (Gemini CLI managed)
//!           └─ Per-message process:
//!               gemini --model X --output-format stream-json [--resume ID] -p "msg"
//! ```
//!
//! ## Key Design Points
//!
//! 1. **No long-lived process**: Each message spawns a new `gemini` process.
//!    Gemini CLI maintains conversation state via session files.
//!
//! 2. **Session-based continuity**: First message captures session_id from output.
//!    Subsequent messages use `--resume <session_id>` to continue conversation.
//!
//! 3. **Structured JSON output**: Using `--output-format stream-json` gives us
//!    clean, parseable events without ANSI codes or prompt detection.
//!
//! 4. **Reliable completion**: Process exits after each message, giving us
//!    deterministic completion detection.

use std::path::Path;
use std::process::Stdio;
use std::time::Duration;

use tokio::io::{AsyncBufReadExt, BufReader};
use tokio::process::Command;
use tokio::sync::mpsc;
use tokio_util::sync::CancellationToken;

use super::types::{CliError, StreamEvent};

/// Configuration for Gemini pipes session
#[derive(Debug, Clone)]
pub struct GeminiPipesConfig {
    /// Path to gemini binary
    pub binary_path: String,

    /// Model to use
    pub model: String,

    /// Working directory (for GEMINI.md project memory)
    pub cwd: Option<String>,

    /// Maximum response time before timeout
    pub max_response_time: Duration,

    /// Prompt detection idle threshold
    pub idle_threshold: Duration,
}

impl Default for GeminiPipesConfig {
    fn default() -> Self {
        Self {
            binary_path: "gemini".to_string(),
            model: "gemini-2.5-flash".to_string(),
            cwd: None,
            max_response_time: Duration::from_secs(120),
            idle_threshold: Duration::from_millis(500),
        }
    }
}

/// Gemini CLI session using one-shot + resume pattern
///
/// Spawns a new process for each message, using Gemini CLI's session
/// management to maintain conversation state across invocations.
pub struct GeminiPipesSession {
    config: GeminiPipesConfig,
    session_id: Option<String>,
    turn_count: usize,
    /// Current running process ID (if any)
    current_pid: Option<u32>,
    /// Session creation time
    created_at: std::time::SystemTime,
}

impl GeminiPipesSession {
    /// Create a new Gemini CLI session
    ///
    /// No process is spawned here. Each message will spawn its own
    /// one-shot process using Gemini CLI's session management.
    pub async fn spawn(model: &str, cwd: Option<&Path>) -> Result<Self, CliError> {
        let mut config = GeminiPipesConfig::default();
        config.model = model.to_string();
        if let Some(dir) = cwd {
            config.cwd = Some(dir.to_string_lossy().to_string());
        }

        Self::spawn_with_config(config).await
    }

    /// Create session with custom configuration
    pub async fn spawn_with_config(config: GeminiPipesConfig) -> Result<Self, CliError> {
        tracing::info!(
            "Creating Gemini CLI session (model: {}, cwd: {:?})",
            config.model,
            config.cwd
        );

        // Verify binary is available
        if !which::which(&config.binary_path).is_ok() {
            return Err(CliError::BinaryNotFound {
                binary: config.binary_path.clone(),
                install_hint: "Install: npm install -g @google/gemini-cli && gemini (to auth)"
                    .to_string(),
            });
        }

        Ok(Self {
            config,
            session_id: None,
            turn_count: 0,
            current_pid: None,
            created_at: std::time::SystemTime::now(),
        })
    }

    /// Send user message and stream response
    ///
    /// This is now a convenience method that combines message sending with streaming.
    /// Calls stream_turn internally.
    pub async fn send_user_message(&mut self, text: &str) -> Result<(), CliError> {
        self.turn_count += 1;
        tracing::debug!(
            "Queuing message for turn {}: {}",
            self.turn_count,
            if text.len() > 50 {
                format!("{}...", &text[..50])
            } else {
                text.to_string()
            }
        );
        Ok(())
    }

    /// Stream one turn of Gemini's response
    ///
    /// Spawns a one-shot gemini process for this message, using --resume
    /// if we have a session_id from previous turns.
    ///
    /// Parses stream-json format events and emits StreamEvent::Delta for content.
    /// Captures session_id from init event on first turn.
    pub async fn stream_turn(
        &mut self,
        message: String,
        tx: mpsc::Sender<StreamEvent>,
        cancel: CancellationToken,
    ) -> Result<(), CliError> {
        let dump_json = std::env::var("GEMINI_CLI_DUMP_JSON").is_ok();

        tracing::debug!(
            "Starting turn {} (session: {:?}, dump_json: {})",
            self.turn_count,
            self.session_id,
            dump_json
        );

        // Build command
        let mut cmd = Command::new(&self.config.binary_path);
        cmd.arg("--model")
            .arg(&self.config.model)
            .arg("--output-format")
            .arg("stream-json")
            .arg("--approval-mode")
            .arg("yolo"); // Auto-approve tools to prevent blocking

        // If we have session_id, resume; otherwise start fresh
        if let Some(ref session_id) = self.session_id {
            tracing::info!(
                "Turn {} using --resume {} (message: {}...)",
                self.turn_count,
                session_id,
                message.chars().take(50).collect::<String>()
            );
            cmd.arg("--resume").arg(session_id);
            cmd.arg("-p").arg(&message); // Must use -p with --resume
        } else {
            tracing::info!(
                "Turn {} starting fresh (no session_id, message: {}...)",
                self.turn_count,
                message.chars().take(50).collect::<String>()
            );
            cmd.arg(&message); // Positional for first message
        }

        if let Some(ref dir) = self.config.cwd {
            cmd.current_dir(dir);
        }

        cmd.stdout(Stdio::piped()).stderr(Stdio::piped());

        // Spawn process
        let mut child = cmd.spawn().map_err(|e| CliError::Internal {
            message: format!("Failed to spawn gemini: {}", e),
        })?;

        // Track process ID
        self.current_pid = child.id();
        tracing::debug!(
            "Gemini process spawned: PID={:?}, session={:?}, turn={}",
            self.current_pid,
            self.session_id,
            self.turn_count
        );

        let stdout = child.stdout.take().ok_or_else(|| CliError::Internal {
            message: "Stdout not available".to_string(),
        })?;

        // Spawn stderr logger
        if let Some(stderr) = child.stderr.take() {
            tokio::spawn(async move {
                let reader = BufReader::new(stderr);
                let mut lines = reader.lines();
                while let Ok(Some(line)) = lines.next_line().await {
                    if !line.is_empty() && !line.contains("Loaded cached credentials") {
                        tracing::debug!("Gemini stderr: {}", line);
                    }
                }
            });
        }

        // Read and parse JSON events
        let mut reader = BufReader::new(stdout);
        let mut line = String::new();
        let deadline = tokio::time::Instant::now() + self.config.max_response_time;

        // Optional JSON dump for debugging
        let dump_file = if dump_json {
            let path = std::env::temp_dir().join(format!("gemini_turn_{}.jsonl", self.turn_count));
            match std::fs::File::create(&path) {
                Ok(f) => {
                    tracing::info!("Dumping JSON events to: {:?}", path);
                    Some(f)
                }
                Err(e) => {
                    tracing::warn!("Failed to create dump file: {}", e);
                    None
                }
            }
        } else {
            None
        };
        let mut dump_file = dump_file;

        loop {
            // Check cancellation
            if cancel.is_cancelled() {
                tracing::info!("Turn cancelled by user");
                let _ = child.kill().await;
                let _ = tx.send(StreamEvent::Done).await;
                return Ok(());
            }

            // Check timeout
            if tokio::time::Instant::now() > deadline {
                tracing::error!("Turn timeout after {:?}", self.config.max_response_time);
                let _ = child.kill().await;
                return Err(CliError::Timeout {
                    elapsed: self.config.max_response_time,
                });
            }

            // Read line
            line.clear();
            match tokio::time::timeout(Duration::from_millis(100), reader.read_line(&mut line))
                .await
            {
                Ok(Ok(0)) => {
                    // EOF - process exited
                    let _ = child.wait().await;
                    let _ = tx.send(StreamEvent::Done).await;
                    return Ok(());
                }
                Ok(Ok(_)) => {
                    // Dump raw JSON if enabled
                    if let Some(ref mut file) = dump_file {
                        use std::io::Write;
                        let _ = writeln!(file, "{}", line.trim());
                    }

                    // Parse JSON event
                    if let Ok(event) = serde_json::from_str::<serde_json::Value>(&line) {
                        let event_type = event.get("type").and_then(|t| t.as_str());
                        tracing::trace!("Gemini event: {:?}", event_type);

                        match event_type {
                            Some("init") => {
                                // Capture session_id on first turn
                                if self.session_id.is_none() {
                                    if let Some(session_id) =
                                        event.get("session_id").and_then(|s| s.as_str())
                                    {
                                        self.session_id = Some(session_id.to_string());
                                        tracing::info!("Captured session_id: {}", session_id);
                                    }
                                }
                            }
                            Some("message") => {
                                // Assistant message with content
                                if let Some(content) = event.get("content").and_then(|c| c.as_str())
                                {
                                    if event.get("role").and_then(|r| r.as_str())
                                        == Some("assistant")
                                    {
                                        let _ =
                                            tx.send(StreamEvent::Delta(content.to_string())).await;
                                    }
                                }
                            }
                            Some("result") => {
                                // Turn complete - result event indicates end
                                tracing::info!("Turn complete (result event received)");

                                // Send Done and exit immediately
                                let _ = tx.send(StreamEvent::Done).await;
                                let _ = child.kill().await; // Clean up process
                                return Ok(());
                            }
                            Some("tool_use") => {
                                tracing::debug!("Tool use event (auto-approved with yolo mode)");
                                // Tool execution events - handled automatically with --approval-mode yolo
                            }
                            Some("tool_result") => {
                                tracing::debug!("Tool result event");
                                // Tool result - handled automatically
                            }
                            _ => {
                                tracing::trace!("Unhandled event type: {:?}", event.get("type"));
                            }
                        }
                    }
                }
                Ok(Err(e)) => {
                    tracing::error!("Read error: {}", e);
                    return Err(CliError::Internal {
                        message: format!("Read error: {}", e),
                    });
                }
                Err(_) => {
                    // Timeout - continue waiting
                }
            }
        }
    }

    /// Check if session is still valid
    ///
    /// Since we don't have a long-lived process, we just check if we have
    /// a valid config. Session state is maintained by Gemini CLI.
    pub fn is_alive(&mut self) -> bool {
        true // Session is always "alive" until explicitly closed
    }

    /// Kill the current running process (if any)
    pub fn kill_process(&mut self) -> Result<(), CliError> {
        if let Some(pid) = self.current_pid {
            tracing::info!("Killing Gemini process: PID={}", pid);

            // Use kill command on Unix, taskkill on Windows
            #[cfg(unix)]
            {
                use std::process::Command;
                let _ = Command::new("kill")
                    .arg("-TERM")
                    .arg(pid.to_string())
                    .output();
            }

            #[cfg(windows)]
            {
                use std::process::Command;
                let _ = Command::new("taskkill")
                    .args(&["/PID", &pid.to_string(), "/F"])
                    .output();
            }

            self.current_pid = None;
        }
        Ok(())
    }

    /// Gracefully shutdown the session
    ///
    /// With one-shot processes, there's nothing to clean up per se.
    /// Could optionally delete the session using Gemini CLI's --delete-session.
    pub async fn shutdown(mut self) -> Result<(), CliError> {
        tracing::info!("Closing Gemini session (session_id: {:?})", self.session_id);

        // Kill any running process
        self.kill_process()?;

        // Optional: Could delete session here with --delete-session
        Ok(())
    }

    /// Get session statistics
    pub fn stats(&self) -> SessionStats {
        SessionStats {
            turn_count: self.turn_count,
            is_alive: true,
            session_id: self.session_id.clone(),
            model: self.config.model.clone(),
        }
    }
}

/// Session statistics
#[derive(Debug, Clone)]
pub struct SessionStats {
    pub turn_count: usize,
    pub is_alive: bool,
    pub session_id: Option<String>,
    pub model: String,
}

//
// ═══════════════════════════════════════════════════════════════════════════
// PROVIDER API
// ═══════════════════════════════════════════════════════════════════════════
//

use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::Mutex;

/// Conversation ID type for session management
pub type ConversationId = String;

/// Gemini Pipes Provider - manages long-lived sessions per conversation
///
/// Provides async API for multi-turn Gemini conversations using pipes.
/// Each conversation gets its own Gemini CLI child process that maintains
/// native conversation state.
///
/// ## Usage
///
/// ```no_run
/// # use codex_core::cli_executor::gemini_pipes::{GeminiPipesProvider, ConversationId};
/// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
/// let provider = GeminiPipesProvider::new("gemini-2.5-flash");
///
/// let conv_id = "conv_123".to_string();
/// let mut rx = provider.send_message(conv_id.clone(), "Hello".to_string()).await?;
///
/// while let Some(event) = rx.recv().await {
///     // Handle stream events
/// }
/// # Ok(())
/// # }
/// ```
pub struct GeminiPipesProvider {
    /// Map of conversation ID to session
    sessions: Arc<Mutex<HashMap<ConversationId, GeminiPipesSession>>>,
    /// Default model for new sessions
    model: String,
    /// Optional working directory for sessions
    cwd: Option<String>,
}

impl GeminiPipesProvider {
    /// Create new provider with default model
    pub fn new(model: &str) -> Self {
        Self {
            sessions: Arc::new(Mutex::new(HashMap::new())),
            model: model.to_string(),
            cwd: None,
        }
    }

    /// Create provider with working directory for GEMINI.md
    pub fn with_cwd(model: &str, cwd: &str) -> Self {
        Self {
            sessions: Arc::new(Mutex::new(HashMap::new())),
            model: model.to_string(),
            cwd: Some(cwd.to_string()),
        }
    }

    /// Send message to a conversation (async API)
    ///
    /// Gets or creates a session for the conversation, then sends the message.
    /// Returns a channel that streams response events.
    ///
    /// # Arguments
    /// * `conv_id` - Conversation identifier
    /// * `message` - User message text
    ///
    /// # Returns
    /// Receiver channel for `StreamEvent` stream
    pub async fn send_message(
        &self,
        conv_id: ConversationId,
        message: String,
    ) -> Result<mpsc::Receiver<StreamEvent>, CliError> {
        // Channel for streaming
        let (tx, rx) = mpsc::channel(128);
        let cancel_token = CancellationToken::new();

        // Clone handles for spawn
        let sessions_clone = Arc::clone(&self.sessions);
        let model = self.model.clone();
        let cwd = self.cwd.clone();

        tokio::spawn(async move {
            // Get or create session
            let mut sessions = sessions_clone.lock().await;

            let session_exists = sessions.contains_key(&conv_id);

            if !session_exists {
                tracing::info!("Creating new Gemini pipes session for conv: {}", conv_id);

                // Create session
                let cwd_path = cwd.as_ref().map(|s| Path::new(s));
                match GeminiPipesSession::spawn(&model, cwd_path).await {
                    Ok(session) => {
                        sessions.insert(conv_id.clone(), session);
                    }
                    Err(e) => {
                        tracing::error!("Failed to spawn session: {:?}", e);
                        let _ = tx.send(StreamEvent::Error(e)).await;
                        return;
                    }
                }
            }

            // Get session (must exist now)
            let session = match sessions.get_mut(&conv_id) {
                Some(s) => s,
                None => {
                    let _ = tx
                        .send(StreamEvent::Error(CliError::Internal {
                            message: "Session disappeared".to_string(),
                        }))
                        .await;
                    return;
                }
            };

            // Check if alive (always true for one-shot approach)
            if !session.is_alive() {
                tracing::warn!("Session {} invalid, will be recreated next turn", conv_id);
                sessions.remove(&conv_id);
                let _ = tx
                    .send(StreamEvent::Error(CliError::Internal {
                        message: "Session invalid".to_string(),
                    }))
                    .await;
                return;
            }

            // Update turn count
            let _ = session.send_user_message(&message).await;

            // Stream response (spawns one-shot process)
            if let Err(e) = session.stream_turn(message, tx.clone(), cancel_token).await {
                tracing::error!("Stream error: {:?}", e);
                let _ = tx.send(StreamEvent::Error(e)).await;
            }
        });

        Ok(rx)
    }

    /// Close a conversation session
    ///
    /// Gracefully shuts down the CLI process and removes it from the pool.
    pub async fn close_conversation(&self, conv_id: &ConversationId) -> Result<(), CliError> {
        let mut sessions = self.sessions.lock().await;

        if let Some(session) = sessions.remove(conv_id) {
            tracing::info!("Closing conversation: {}", conv_id);
            session.shutdown().await?;
        }

        Ok(())
    }

    /// Shutdown all sessions
    ///
    /// Gracefully closes all active sessions.
    pub async fn shutdown_all(&self) -> Result<(), CliError> {
        let mut sessions = self.sessions.lock().await;
        let all_sessions: Vec<_> = sessions.drain().collect();

        drop(sessions); // Release lock

        for (id, session) in all_sessions {
            tracing::info!("Shutting down session: {}", id);
            let _ = session.shutdown().await;
        }

        Ok(())
    }

    /// Get active session count
    pub async fn active_session_count(&self) -> usize {
        self.sessions.lock().await.len()
    }

    /// Get information about all active sessions
    pub async fn list_sessions(&self) -> Vec<super::claude_pipes::SessionInfo> {
        let sessions = self.sessions.lock().await;

        sessions
            .iter()
            .map(|(conv_id, session)| super::claude_pipes::SessionInfo {
                conv_id: conv_id.clone(),
                session_id: session.session_id.clone(),
                model: session.config.model.clone(),
                turn_count: session.turn_count,
                current_pid: session.current_pid,
                created_at: session.created_at,
                provider: "Gemini".to_string(),
            })
            .collect()
    }

    /// Kill a specific session
    pub async fn kill_session(&self, conv_id: &ConversationId) -> Result<(), CliError> {
        let mut sessions = self.sessions.lock().await;

        if let Some(session) = sessions.get_mut(conv_id) {
            session.kill_process()?;
            tracing::info!("Killed session: {}", conv_id);
        }

        Ok(())
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
         Follow the OAuth prompts to complete authentication."
    }
}

impl Default for GeminiPipesProvider {
    fn default() -> Self {
        Self::new("gemini-2.5-flash")
    }
}

impl Drop for GeminiPipesProvider {
    fn drop(&mut self) {
        tracing::info!("GeminiPipesProvider dropping - cleaning up sessions");

        // Try to cleanup sessions synchronously
        // Note: This is best-effort since Drop can't be async
        if let Ok(mut sessions) = self.sessions.try_lock() {
            let session_ids: Vec<_> = sessions.keys().cloned().collect();

            for conv_id in session_ids {
                if let Some(mut session) = sessions.remove(&conv_id) {
                    tracing::info!("Cleaning up session: {}", conv_id);
                    // Kill any running process
                    let _ = session.kill_process();
                }
            }
        } else {
            tracing::warn!("Could not acquire sessions lock during drop - some processes may leak");
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_session_creation() {
        let config = GeminiPipesConfig::default();
        assert_eq!(config.binary_path, "gemini");
        assert_eq!(config.model, "gemini-2.5-flash");
    }

    #[tokio::test]
    #[ignore] // Requires Gemini CLI installed
    async fn test_single_turn_pipes() {
        let mut session = GeminiPipesSession::spawn("gemini-2.5-flash", None)
            .await
            .expect("Failed to spawn session");

        assert!(session.is_alive());

        let (tx, mut rx) = mpsc::channel(32);
        let cancel = CancellationToken::new();

        let message = "Say exactly: Hello World".to_string();
        let stream_result = session.stream_turn(message, tx, cancel).await;

        assert!(stream_result.is_ok(), "Stream should succeed");

        let mut response = String::new();
        while let Some(event) = rx.recv().await {
            if let StreamEvent::Delta(text) = event {
                response.push_str(&text);
            }
        }

        println!("Response: {}", response);
        assert!(
            response.contains("Hello") || response.contains("World"),
            "Response should contain greeting"
        );

        session.shutdown().await.ok();
    }

    #[tokio::test]
    #[ignore] // Requires Gemini CLI installed
    async fn test_multi_turn_state() {
        let mut session = GeminiPipesSession::spawn("gemini-2.5-flash", None)
            .await
            .expect("Failed to spawn session");

        // Turn 1: Set name
        let (tx1, mut rx1) = mpsc::channel(32);
        let msg1 = "My name is Alice.".to_string();
        session
            .stream_turn(msg1, tx1, CancellationToken::new())
            .await
            .unwrap();
        while rx1.recv().await.is_some() {}

        // Turn 2: Recall name (should use --resume with captured session_id)
        let (tx2, mut rx2) = mpsc::channel(32);
        let msg2 = "What's my name?".to_string();
        session
            .stream_turn(msg2, tx2, CancellationToken::new())
            .await
            .unwrap();

        let mut response = String::new();
        while let Some(event) = rx2.recv().await {
            if let StreamEvent::Delta(text) = event {
                response.push_str(&text);
            }
        }

        println!("Response: {}", response);
        assert!(
            response.contains("Alice"),
            "Should remember name from turn 1"
        );

        session.shutdown().await.ok();
    }
}
