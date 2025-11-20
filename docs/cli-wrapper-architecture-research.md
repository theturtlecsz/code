# Production-Grade CLI Wrapper Architecture for Claude & Gemini

## 3.1 CLI Capabilities Audit (Theoretical)

### Likely CLI Capabilities

**Claude CLI (`claude`):**
Based on typical patterns from Anthropic's tooling:

```bash
# Likely invocation patterns
claude                          # Interactive REPL mode
claude chat                     # Chat session (possibly stateful?)
claude code                     # Claude Code mode
claude --model opus-4.1 --prompt "..."
claude --system "..." --message "..." [--message "..."]
claude --session-id <uuid>      # Session restoration (maybe?)
claude --stream / --no-stream   # Output control
claude --format json            # Structured output
```

**Expected capabilities:**
- **Input modes**: stdin, `--message` flags, file via `@filename`, possibly `--messages-json`
- **Output formats**: plain text (default), JSON (for structured), potentially SSE-like streaming markers
- **Session handling**: Unknown if native session IDs exist; likely requires history replay
- **Context window**: ~200K tokens (Opus), but CLI may have smaller practical limits
- **Streaming**: Likely line-buffered or sentence-buffered, not true token-level

**Gemini CLI (likely `gcloud ai` or standalone):**

```bash
# Google Cloud AI pattern
gcloud ai models generate-text --model gemini-2.5-pro --prompt "..."
gcloud ai conversations create --model gemini-2.5-pro
gcloud ai conversations send --conversation-id <id> --message "..."

# Or standalone CLI
gemini chat
gemini --model gemini-2.5-pro --prompt "..."
gemini --history-file session.json
```

**Expected capabilities:**
- **Input modes**: Similar to Claude, plus possible `--history` file
- **Output formats**: Text, JSON, potentially proto-based
- **Session handling**: Google Cloud might have native conversation IDs
- **Context window**: ~1M tokens (Gemini 2.0), but practical limits unknown
- **Streaming**: Google services typically support SSE; CLI might buffer more aggressively

### Critical Unknowns Requiring Empirical Testing

**Checklist for `claude` CLI:**
```bash
# 1. Help & capabilities discovery
claude --help
claude chat --help
claude code --help
claude --version

# 2. Multi-turn conversation mechanisms
# Test A: Multiple --message flags
claude --message "user:Hello" --message "assistant:Hi there" --message "user:What's 2+2?"

# Test B: JSON messages input
echo '{"messages":[{"role":"user","content":"Hello"}]}' | claude --messages-json

# Test C: Session restoration
claude --session-id test-123 --message "Remember this"
claude --session-id test-123 --message "What did I just say?"

# 3. Streaming behavior
claude --stream --message "Write a long story" | xxd  # Hex dump to see buffering
time claude --stream --message "Count to 100"  # Latency test

# 4. Max input size before errors
python -c "print('x'*1000000)" | claude  # Test large input handling

# 5. Error surfacing
claude --model invalid-model --message "test"  # Check exit code, stderr format
claude --message "test" 2>&1 | tee /tmp/claude-errors.log

# 6. Cancellation behavior
claude --message "Write forever" &
PID=$!
sleep 2
kill -INT $PID  # Test SIGINT handling
```

**Checklist for Gemini CLI:**
```bash
# Similar pattern, adjust for actual CLI name
gemini --help
gcloud ai models list  # Discover available models

# Test conversation APIs if they exist
gcloud ai conversations create --model gemini-2.5-pro
# (record conversation ID)
gcloud ai conversations send --conversation-id <id> --message "Hello"

# Test streaming
gemini --stream --prompt "Long output test"

# Test context limits
python gen_large_prompt.py | gemini  # Generate >100K token input
```

**Performance measurements needed:**
```bash
# Startup overhead
hyperfine 'claude --message "hi"' 'echo "hi" | claude'

# Streaming latency (time to first byte)
time -p bash -c 'claude --stream --message "Say hi" | head -n1'

# Memory footprint during long conversation
/usr/bin/time -v claude --message "$(cat long-history.txt)"
```

---

## 3.2 Architecture Options & Tradeoffs

### Option A: Stateless Per-Request with Synthetic History Embedding

**Core Concept:**
Every request spawns a fresh CLI process. Conversation history is reconstructed by formatting prior messages into the prompt.

```
┌─────────────┐
│   TUI       │
│  (model_    │
│   router)   │
└──────┬──────┘
       │ send_message(history: Vec<Message>, new_prompt: String)
       ▼
┌──────────────────────────────┐
│  CliContextManager           │
│  - format_history()          │
│  - estimate_tokens()         │
│  - compress_if_needed()      │
└──────┬───────────────────────┘
       │ formatted_prompt: String
       ▼
┌──────────────────────────────┐
│  CliExecutor::execute()      │
│  - spawn process             │
│  - write stdin               │
│  - stream stdout             │
└──────┬───────────────────────┘
       │ stdout chunks
       ▼
┌──────────────────────────────┐
│  CliStreamHandler            │
│  - parse chunks              │
│  - emit deltas               │
│  - detect completion         │
└──────┬───────────────────────┘
       │ streamed tokens
       ▼
┌─────────────┐
│   TUI       │
│  (render)   │
└─────────────┘
```

**History Embedding Format:**
```
SYSTEM: You are a helpful coding assistant.

--- Previous Conversation ---
USER (2024-11-20 14:23):
What's the best way to handle errors in Rust?

ASSISTANT (2024-11-20 14:23):
Rust uses the Result<T, E> type for recoverable errors...

USER (2024-11-20 14:25):
Can you show an example?
--- End Previous Conversation ---

USER (current):
Make it work with custom error types.
```

**Pros:**
- ✅ **Simplicity**: No persistent state, no process lifecycle complexity
- ✅ **Reliability**: Process crashes can't corrupt state; easy retries
- ✅ **Debuggability**: Each invocation is independent; logs are self-contained
- ✅ **Cross-platform**: No OS-specific process management quirks
- ✅ **Testability**: Easy to mock; deterministic given same history

**Cons:**
- ❌ **Startup latency**: 200-500ms overhead per request (typical CLI startup)
- ❌ **Context limit pressure**: Must fit entire history in one prompt
- ❌ **Token waste**: Re-sending full history every time (though user pays via CLI subscription)
- ❌ **Compression complexity**: Need smart summarization for long sessions
- ❌ **Rate limit risk**: Each request is a fresh API call from CLI's perspective

**Mitigations:**
- Use aggressive but lossless compression (e.g., remove old code blocks, keep summaries)
- Implement request coalescing (batch rapid user edits before sending)
- Cache recent responses to avoid re-requesting identical prompts

---

### Option B: Stateful Long-Lived Process per Session

**Core Concept:**
Spawn one CLI process at session start, keep it alive via interactive mode (if available), send incremental messages.

```
┌─────────────┐
│   TUI       │
└──────┬──────┘
       │ session_start()
       ▼
┌──────────────────────────────┐
│  CliSessionManager           │
│  - spawn_persistent_cli()    │
│  - maintain_heartbeat()      │
│  - handle_reconnection()     │
└──────┬───────────────────────┘
       │ process handle
       ▼
┌──────────────────────────────┐
│  Long-lived CLI Process      │
│  (stdin/stdout pipes open)   │
└──────┬───────────────────────┘
       │
       │ send_incremental_message()
       ▼
┌──────────────────────────────┐
│  CliProtocolHandler          │
│  - encode_message()          │
│  - await_response_delimiter  │
│  - parse_streaming_output    │
└──────┬───────────────────────┘
       │ streamed response
       ▼
┌─────────────┐
│   TUI       │
└─────────────┘
```

**Protocol Example (hypothetical):**
```bash
# Initial spawn
claude chat --session-mode --format json

# Session protocol (JSON-lines over stdin/stdout)
→ {"type":"message","role":"user","content":"Hello"}
← {"type":"delta","content":"Hi"}
← {"type":"delta","content":" there"}
← {"type":"done","message_id":"msg_abc123"}

→ {"type":"message","role":"user","content":"What's 2+2?"}
← {"type":"delta","content":"2"}
← {"type":"delta","content":"+2"}
← {"type":"delta","content":" equals 4"}
← {"type":"done","message_id":"msg_def456"}
```

**Pros:**
- ✅ **Low latency**: No startup overhead after initial spawn (~50ms response start)
- ✅ **Efficient context**: CLI maintains state; no re-sending history
- ✅ **True streaming**: Direct pipeline to model's token generation
- ✅ **Natural sessions**: Aligns with how humans use interactive CLIs

**Cons:**
- ❌ **Complexity**: Process lifecycle, heartbeats, crash recovery, reconnection logic
- ❌ **State synchronization**: TUI and CLI process can desync (crashes, network blips)
- ❌ **Platform quirks**: stdin/stdout buffering differs on Windows vs Unix
- ❌ **Debugging**: Harder to reproduce; stateful bugs are non-deterministic
- ❌ **Unknown feasibility**: Requires CLI to support session mode (may not exist!)

**Critical Unknowns:**
- Does `claude chat` or `gemini` have an interactive session mode?
- If yes, what's the protocol? JSON-lines? Custom framing?
- How does the CLI signal "response complete"?
- What happens on network errors mid-session?

**Fallback Strategy:**
If no native session mode exists, **Option B is not viable**. Fall back to Option A.

---

### Option C: Hybrid – Process Pool with Request Routing

**Core Concept:**
Maintain a pool of 2-4 warm CLI processes. Route requests to available process, embed minimal context.

```
┌─────────────────────────────────┐
│  CliProcessPool                 │
│  - workers: Vec<CliWorker>      │
│  - schedule_request()           │
│  - reap_idle_workers()          │
└──────┬──────────────────────────┘
       │
       ├─► CliWorker #1 (idle)
       ├─► CliWorker #2 (busy)
       ├─► CliWorker #3 (idle)
       └─► CliWorker #4 (crashed, restarting)
```

**Request Routing:**
- **Stateless requests** (new conversation): any idle worker
- **Continuation requests**: prefer worker that handled prior message (soft affinity)
- **Concurrent users**: different workers for parallel sessions

**Pros:**
- ✅ **Reduced latency**: Warm processes avoid startup overhead
- ✅ **Concurrency**: Support multiple simultaneous conversations
- ✅ **Fault tolerance**: Crashed workers auto-restart; others continue serving

**Cons:**
- ❌ **Resource overhead**: 4 idle processes consume memory (~200MB each?)
- ❌ **Complexity**: Scheduling, affinity tracking, health checks
- ❌ **Questionable benefit**: Most TUI users have 1-2 active conversations max
- ❌ **Context confusion**: Soft affinity isn't guaranteed; still need history embedding

**Verdict:**
Premature optimization for a TUI use case. Consider only if profiling shows startup overhead is a UX killer.

---

### Recommendation: **Option A** (with Option B as aspirational future)

**Rationale:**

1. **Practicality**: We don't know if CLIs support persistent sessions. Option A is guaranteed to work.
2. **Simplicity**: Debugging and testing are 10x easier. Ship fast, optimize later.
3. **Robustness**: No persistent state means no state corruption bugs.
4. **Performance**: 200-500ms startup overhead is annoying but tolerable; UX can mask it with optimistic UI updates.

**Migration Path:**
1. **Phase 1 (MVP)**: Implement Option A fully.
2. **Phase 2 (Optimization)**: Experiment with `claude chat` interactive mode (if it exists). If successful, add Option B as an opt-in feature flag.
3. **Phase 3 (Scale)**: If user base grows and concurrent sessions become common, revisit Option C.

---

## 3.3 Concrete Rust API/Traits Design

### Core Abstractions

```rust
use std::process::Stdio;
use tokio::process::{Child, Command};
use tokio::io::{AsyncBufReadExt, BufReader};
use tokio::sync::mpsc;
use thiserror::Error;

// ============================================================================
// Message representation (provider-agnostic)
// ============================================================================

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Role {
    System,
    User,
    Assistant,
    Tool,  // For function call results
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Message {
    pub role: Role,
    pub content: String,
    pub timestamp: Option<DateTime<Utc>>,
    pub metadata: HashMap<String, String>,  // tool_call_id, etc.
}

#[derive(Debug, Clone)]
pub struct Conversation {
    pub messages: Vec<Message>,
    pub system_prompt: Option<String>,
    pub model: String,
    pub created_at: DateTime<Utc>,
}

// ============================================================================
// Streaming response types
// ============================================================================

#[derive(Debug, Clone)]
pub enum StreamEvent {
    Delta(String),           // Incremental text
    ToolCall(ToolCallData),  // Structured tool invocation
    Metadata(ResponseMetadata),  // Model, tokens, etc.
    Done,                    // Response complete
    Error(CliError),         // Recoverable error mid-stream
}

#[derive(Debug, Clone)]
pub struct ResponseMetadata {
    pub model: String,
    pub input_tokens: Option<usize>,
    pub output_tokens: Option<usize>,
    pub stop_reason: Option<String>,
}

// ============================================================================
// Error taxonomy
// ============================================================================

#[derive(Error, Debug, Clone)]
pub enum CliError {
    #[error("CLI binary not found: {binary}. Install via: {install_hint}")]
    BinaryNotFound {
        binary: String,
        install_hint: String,
    },

    #[error("CLI not authenticated. Run: {auth_command}")]
    NotAuthenticated {
        auth_command: String,
    },

    #[error("Rate limit exceeded. Retry after: {retry_after:?}")]
    RateLimited {
        retry_after: Option<Duration>,
    },

    #[error("CLI process exited with code {code}: {stderr}")]
    ProcessFailed {
        code: i32,
        stdout: String,
        stderr: String,
    },

    #[error("Network error: {message}")]
    Network {
        message: String,
    },

    #[error("Timeout after {elapsed:?}")]
    Timeout {
        elapsed: Duration,
    },

    #[error("Context too large: {size} tokens exceeds {limit}")]
    ContextTooLarge {
        size: usize,
        limit: usize,
    },

    #[error("Invalid response format: {details}")]
    ParseError {
        details: String,
    },

    #[error("Internal error: {message}")]
    Internal {
        message: String,
    },
}

impl CliError {
    /// Is this error retryable?
    pub fn is_retryable(&self) -> bool {
        matches!(self,
            CliError::Network { .. } |
            CliError::RateLimited { .. } |
            CliError::Timeout { .. }
        )
    }

    /// Suggested user action
    pub fn user_action(&self) -> String {
        match self {
            CliError::BinaryNotFound { install_hint, .. } => install_hint.clone(),
            CliError::NotAuthenticated { auth_command } =>
                format!("Run: {}", auth_command),
            CliError::RateLimited { retry_after: Some(d) } =>
                format!("Wait {}s and retry", d.as_secs()),
            CliError::RateLimited { retry_after: None } =>
                "Wait a moment and retry".to_string(),
            CliError::ContextTooLarge { .. } =>
                "Conversation too long. Start a new session or summarize.".to_string(),
            _ => "Check logs for details".to_string(),
        }
    }
}

// ============================================================================
// Core trait: CLI execution abstraction
// ============================================================================

#[async_trait]
pub trait CliExecutor: Send + Sync {
    /// Execute a one-shot request with conversation history
    async fn execute(
        &self,
        conversation: &Conversation,
        user_message: &str,
    ) -> Result<mpsc::Receiver<StreamEvent>, CliError>;

    /// Check if CLI is available and authenticated
    async fn health_check(&self) -> Result<(), CliError>;

    /// Estimate token count for context size validation
    fn estimate_tokens(&self, conversation: &Conversation) -> usize;

    /// Provider-specific config (model, temperature, etc.)
    fn config(&self) -> &CliExecutorConfig;
}

#[derive(Debug, Clone)]
pub struct CliExecutorConfig {
    pub binary_path: String,
    pub model: String,
    pub timeout: Duration,
    pub max_context_tokens: usize,
    pub retry_config: RetryConfig,
}

#[derive(Debug, Clone)]
pub struct RetryConfig {
    pub max_attempts: u32,
    pub initial_backoff: Duration,
    pub max_backoff: Duration,
    pub backoff_multiplier: f64,
}

impl Default for RetryConfig {
    fn default() -> Self {
        Self {
            max_attempts: 3,
            initial_backoff: Duration::from_secs(1),
            max_backoff: Duration::from_secs(30),
            backoff_multiplier: 2.0,
        }
    }
}

// ============================================================================
// Context manager: history formatting and compression
// ============================================================================

pub struct CliContextManager {
    max_tokens: usize,
    compression_threshold: f64,  // 0.8 = compress when 80% full
}

impl CliContextManager {
    pub fn new(max_tokens: usize) -> Self {
        Self {
            max_tokens,
            compression_threshold: 0.8,
        }
    }

    /// Format conversation history into CLI-friendly prompt
    pub fn format_history(&self, conversation: &Conversation) -> String {
        let mut output = String::new();

        // System prompt first
        if let Some(system) = &conversation.system_prompt {
            output.push_str("SYSTEM: ");
            output.push_str(system);
            output.push_str("\n\n");
        }

        // Previous messages
        if conversation.messages.len() > 1 {
            output.push_str("--- Previous Conversation ---\n");
            for msg in &conversation.messages[..conversation.messages.len()-1] {
                self.format_message(&mut output, msg);
            }
            output.push_str("--- End Previous Conversation ---\n\n");
        }

        // Current message
        if let Some(last) = conversation.messages.last() {
            output.push_str("USER (current):\n");
            output.push_str(&last.content);
        }

        output
    }

    fn format_message(&self, output: &mut String, msg: &Message) {
        let role = match msg.role {
            Role::System => "SYSTEM",
            Role::User => "USER",
            Role::Assistant => "ASSISTANT",
            Role::Tool => "TOOL RESULT",
        };

        if let Some(ts) = msg.timestamp {
            output.push_str(&format!("{} ({}):\n", role, ts.format("%Y-%m-%d %H:%M")));
        } else {
            output.push_str(&format!("{}:\n", role));
        }

        output.push_str(&msg.content);
        output.push_str("\n\n");
    }

    /// Compress conversation if nearing token limit
    pub fn compress_if_needed(&self, conversation: &mut Conversation) -> Result<(), CliError> {
        let current_tokens = self.estimate_tokens(conversation);
        let threshold = (self.max_tokens as f64 * self.compression_threshold) as usize;

        if current_tokens > threshold {
            self.compress_conversation(conversation)?;
        }

        Ok(())
    }

    fn compress_conversation(&self, conversation: &mut Conversation) -> Result<(), CliError> {
        // Strategy: Keep first message, last 3 messages, summarize middle
        if conversation.messages.len() <= 4 {
            return Ok(()); // Too short to compress
        }

        let first = conversation.messages[0].clone();
        let middle = &conversation.messages[1..conversation.messages.len()-3];
        let last_three = conversation.messages[conversation.messages.len()-3..].to_vec();

        // Create summary of middle messages
        let summary_content = format!(
            "[{} messages summarized: conversation about {}]",
            middle.len(),
            self.extract_topic_keywords(middle)
        );

        let summary = Message {
            role: Role::System,
            content: summary_content,
            timestamp: middle.first().and_then(|m| m.timestamp),
            metadata: HashMap::from([("compressed".to_string(), "true".to_string())]),
        };

        conversation.messages = vec![first, summary];
        conversation.messages.extend(last_three);

        Ok(())
    }

    fn extract_topic_keywords(&self, messages: &[Message]) -> String {
        // Simple heuristic: extract nouns/verbs from first user message
        // In production: use TF-IDF, keyword extraction, or even a cheap LLM call
        messages.iter()
            .filter(|m| matches!(m.role, Role::User))
            .take(1)
            .map(|m| {
                m.content.split_whitespace()
                    .filter(|w| w.len() > 4)  // Likely content words
                    .take(3)
                    .collect::<Vec<_>>()
                    .join(", ")
            })
            .next()
            .unwrap_or_else(|| "various topics".to_string())
    }

    /// Estimate tokens (heuristic: ~4 chars per token for English)
    pub fn estimate_tokens(&self, conversation: &Conversation) -> usize {
        let char_count: usize = conversation.messages.iter()
            .map(|m| m.content.len())
            .sum();

        let system_chars = conversation.system_prompt.as_ref()
            .map(|s| s.len())
            .unwrap_or(0);

        (char_count + system_chars) / 4  // Conservative estimate
    }
}

// ============================================================================
// Stream handler: parse CLI stdout into events
// ============================================================================

pub struct CliStreamHandler {
    buffer: String,
    utf8_buffer: Vec<u8>,
}

impl CliStreamHandler {
    pub fn new() -> Self {
        Self {
            buffer: String::new(),
            utf8_buffer: Vec::new(),
        }
    }

    /// Process a chunk of stdout bytes
    pub fn process_chunk(&mut self, chunk: &[u8]) -> Vec<StreamEvent> {
        self.utf8_buffer.extend_from_slice(chunk);

        // Try to decode as UTF-8
        match String::from_utf8(self.utf8_buffer.clone()) {
            Ok(valid_str) => {
                self.utf8_buffer.clear();
                self.process_valid_string(&valid_str)
            }
            Err(e) => {
                // Partial UTF-8 sequence at end
                let valid_up_to = e.utf8_error().valid_up_to();
                if valid_up_to > 0 {
                    let valid_bytes = self.utf8_buffer.drain(..valid_up_to).collect::<Vec<_>>();
                    let valid_str = String::from_utf8(valid_bytes).unwrap();
                    self.process_valid_string(&valid_str)
                } else {
                    vec![]  // Wait for more bytes
                }
            }
        }
    }

    fn process_valid_string(&mut self, s: &str) -> Vec<StreamEvent> {
        // Strip ANSI escape codes
        let clean = self.strip_ansi(s);

        // Detect special markers (provider-specific)
        if clean.contains("```") {
            // Handle code blocks specially for syntax highlighting
            vec![StreamEvent::Delta(clean)]
        } else if clean.trim().is_empty() {
            vec![]  // Ignore whitespace-only chunks
        } else {
            vec![StreamEvent::Delta(clean)]
        }
    }

    fn strip_ansi(&self, s: &str) -> String {
        // Regex: \x1b\[[0-9;]*[a-zA-Z]
        // Simple approach: remove all \x1b[...m sequences
        let re = regex::Regex::new(r"\x1b\[[0-9;]*m").unwrap();
        re.replace_all(s, "").to_string()
    }

    /// Signal that stream is complete
    pub fn finalize(&mut self) -> Vec<StreamEvent> {
        let mut events = Vec::new();

        if !self.buffer.is_empty() {
            events.push(StreamEvent::Delta(std::mem::take(&mut self.buffer)));
        }

        events.push(StreamEvent::Done);
        events
    }
}

// ============================================================================
// Concrete implementation: Claude CLI
// ============================================================================

pub struct ClaudeCliExecutor {
    config: CliExecutorConfig,
    context_manager: CliContextManager,
}

impl ClaudeCliExecutor {
    pub fn new(model: String, binary_path: Option<String>) -> Self {
        let binary = binary_path.unwrap_or_else(|| "claude".to_string());

        Self {
            config: CliExecutorConfig {
                binary_path: binary,
                model,
                timeout: Duration::from_secs(120),
                max_context_tokens: 180_000,  // Conservative for Opus
                retry_config: RetryConfig::default(),
            },
            context_manager: CliContextManager::new(180_000),
        }
    }

    async fn execute_with_retry(
        &self,
        formatted_prompt: String,
    ) -> Result<mpsc::Receiver<StreamEvent>, CliError> {
        let mut attempt = 0;
        let mut backoff = self.config.retry_config.initial_backoff;

        loop {
            attempt += 1;

            match self.spawn_and_stream(&formatted_prompt).await {
                Ok(rx) => return Ok(rx),
                Err(e) if e.is_retryable() && attempt < self.config.retry_config.max_attempts => {
                    tracing::warn!("Attempt {}/{} failed: {}. Retrying in {:?}...",
                        attempt, self.config.retry_config.max_attempts, e, backoff);

                    tokio::time::sleep(backoff).await;
                    backoff = std::cmp::min(
                        Duration::from_secs_f64(backoff.as_secs_f64() * self.config.retry_config.backoff_multiplier),
                        self.config.retry_config.max_backoff,
                    );
                }
                Err(e) => return Err(e),
            }
        }
    }

    async fn spawn_and_stream(
        &self,
        prompt: &str,
    ) -> Result<mpsc::Receiver<StreamEvent>, CliError> {
        let mut child = Command::new(&self.config.binary_path)
            .arg("--model")
            .arg(&self.config.model)
            .arg("--stream")
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .map_err(|e| {
                if e.kind() == std::io::ErrorKind::NotFound {
                    CliError::BinaryNotFound {
                        binary: self.config.binary_path.clone(),
                        install_hint: "Visit https://claude.ai/download".to_string(),
                    }
                } else {
                    CliError::Internal { message: e.to_string() }
                }
            })?;

        // Write prompt to stdin
        if let Some(mut stdin) = child.stdin.take() {
            use tokio::io::AsyncWriteExt;
            stdin.write_all(prompt.as_bytes()).await
                .map_err(|e| CliError::Internal { message: e.to_string() })?;
            stdin.flush().await
                .map_err(|e| CliError::Internal { message: e.to_string() })?;
            drop(stdin);  // Close stdin
        }

        let stdout = child.stdout.take()
            .ok_or_else(|| CliError::Internal { message: "Failed to capture stdout".to_string() })?;

        let stderr = child.stderr.take()
            .ok_or_else(|| CliError::Internal { message: "Failed to capture stderr".to_string() })?;

        // Spawn async task to stream stdout
        let (tx, rx) = mpsc::channel(100);

        tokio::spawn(async move {
            Self::stream_output(child, stdout, stderr, tx).await;
        });

        Ok(rx)
    }

    async fn stream_output(
        mut child: Child,
        stdout: tokio::process::ChildStdout,
        stderr: tokio::process::ChildStderr,
        tx: mpsc::Sender<StreamEvent>,
    ) {
        let mut handler = CliStreamHandler::new();
        let mut reader = BufReader::new(stdout).lines();
        let mut stderr_reader = BufReader::new(stderr);

        // Read stdout line-by-line
        while let Ok(Some(line)) = reader.next_line().await {
            for event in handler.process_chunk(line.as_bytes()) {
                if tx.send(event).await.is_err() {
                    break;  // Receiver dropped
                }
            }
        }

        // Finalize stream
        for event in handler.finalize() {
            let _ = tx.send(event).await;
        }

        // Wait for process to exit
        match child.wait().await {
            Ok(status) if !status.success() => {
                // Read stderr
                let mut stderr_content = String::new();
                use tokio::io::AsyncReadExt;
                let _ = stderr_reader.read_to_string(&mut stderr_content).await;

                let error = CliError::ProcessFailed {
                    code: status.code().unwrap_or(-1),
                    stdout: String::new(),
                    stderr: stderr_content,
                };

                let _ = tx.send(StreamEvent::Error(error)).await;
            }
            Err(e) => {
                let error = CliError::Internal { message: e.to_string() };
                let _ = tx.send(StreamEvent::Error(error)).await;
            }
            _ => {}
        }
    }
}

#[async_trait]
impl CliExecutor for ClaudeCliExecutor {
    async fn execute(
        &self,
        conversation: &Conversation,
        user_message: &str,
    ) -> Result<mpsc::Receiver<StreamEvent>, CliError> {
        // Build conversation with new message
        let mut conv = conversation.clone();
        conv.messages.push(Message {
            role: Role::User,
            content: user_message.to_string(),
            timestamp: Some(Utc::now()),
            metadata: HashMap::new(),
        });

        // Check and compress context
        self.context_manager.compress_if_needed(&mut conv)?;

        // Validate token count
        let tokens = self.estimate_tokens(&conv);
        if tokens > self.config.max_context_tokens {
            return Err(CliError::ContextTooLarge {
                size: tokens,
                limit: self.config.max_context_tokens,
            });
        }

        // Format history
        let formatted_prompt = self.context_manager.format_history(&conv);

        // Execute with retry
        self.execute_with_retry(formatted_prompt).await
    }

    async fn health_check(&self) -> Result<(), CliError> {
        // Try to run --version
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
                    CliError::Internal { message: e.to_string() }
                }
            })?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);

            // Check for auth error patterns
            if stderr.contains("not authenticated") || stderr.contains("login required") {
                return Err(CliError::NotAuthenticated {
                    auth_command: "claude login".to_string(),
                });
            }

            return Err(CliError::ProcessFailed {
                code: output.status.code().unwrap_or(-1),
                stdout: String::from_utf8_lossy(&output.stdout).to_string(),
                stderr: stderr.to_string(),
            });
        }

        Ok(())
    }

    fn estimate_tokens(&self, conversation: &Conversation) -> usize {
        self.context_manager.estimate_tokens(conversation)
    }

    fn config(&self) -> &CliExecutorConfig {
        &self.config
    }
}

// ============================================================================
// Integration with TUI layer
// ============================================================================

pub struct ModelRouter {
    executors: HashMap<String, Box<dyn CliExecutor>>,
}

impl ModelRouter {
    pub fn new() -> Self {
        let mut executors: HashMap<String, Box<dyn CliExecutor>> = HashMap::new();

        // Register Claude models
        executors.insert(
            "claude-opus-4.1".to_string(),
            Box::new(ClaudeCliExecutor::new("claude-opus-4.1".to_string(), None)),
        );
        executors.insert(
            "claude-sonnet-4.5".to_string(),
            Box::new(ClaudeCliExecutor::new("claude-sonnet-4.5".to_string(), None)),
        );

        // Register Gemini models (similar pattern)
        // executors.insert("gemini-2.5-pro".to_string(), Box::new(GeminiCliExecutor::new(...)));

        Self { executors }
    }

    pub async fn send_message(
        &self,
        model: &str,
        conversation: &Conversation,
        message: &str,
    ) -> Result<mpsc::Receiver<StreamEvent>, CliError> {
        let executor = self.executors.get(model)
            .ok_or_else(|| CliError::Internal {
                message: format!("Unknown model: {}", model)
            })?;

        executor.execute(conversation, message).await
    }

    pub async fn health_check_all(&self) -> HashMap<String, Result<(), CliError>> {
        let mut results = HashMap::new();

        for (model, executor) in &self.executors {
            results.insert(model.clone(), executor.health_check().await);
        }

        results
    }
}
```

---

## 3.4 Context Management Strategy

### Provider-Agnostic Conversation Representation

**Core struct:**
```rust
pub struct Conversation {
    pub id: Uuid,
    pub messages: Vec<Message>,
    pub system_prompt: Option<String>,
    pub model: String,
    pub created_at: DateTime<Utc>,
    pub metadata: ConversationMetadata,
}

pub struct ConversationMetadata {
    pub total_tokens: usize,         // Running estimate
    pub compressed_count: usize,     // How many times compressed
    pub last_compressed_at: Option<DateTime<Utc>>,
    pub session_file: Option<PathBuf>,  // For persistence
}
```

### History → CLI Prompt Conversion

**Format Strategy:**

```
┌─────────────────────────────────────────────────────────────┐
│ SYSTEM: <system_prompt>                                     │
│                                                              │
│ --- Previous Conversation ---                               │
│ USER (2024-11-20 14:23):                                    │
│ <message content>                                            │
│                                                              │
│ ASSISTANT (2024-11-20 14:23):                               │
│ <message content>                                            │
│                                                              │
│ TOOL RESULT (2024-11-20 14:24):                             │
│ Function: run_bash                                           │
│ Output:                                                      │
│ <tool output>                                                │
│                                                              │
│ ASSISTANT (2024-11-20 14:24):                               │
│ <message content>                                            │
│ --- End Previous Conversation ---                           │
│                                                              │
│ USER (current):                                              │
│ <new message>                                                │
└─────────────────────────────────────────────────────────────┘
```

**Rationale:**
- **Clear delimiters**: "---" makes history boundaries unambiguous
- **Timestamps**: Help model understand temporal context ("earlier you said...")
- **Role prefixes**: Explicit `USER:`/`ASSISTANT:` prevents role confusion
- **Tool results**: Embedded inline with clear labels for traceability
- **Current marker**: Signals to model what it should respond to

**Alternative for CLIs with JSON support:**
```json
{
  "system": "You are a helpful coding assistant.",
  "messages": [
    {"role": "user", "content": "Hello"},
    {"role": "assistant", "content": "Hi there!"},
    {"role": "user", "content": "What's 2+2?"}
  ]
}
```

### Context Compression Strategies

**Trigger Conditions:**
```rust
fn should_compress(conversation: &Conversation, max_tokens: usize) -> bool {
    let current = estimate_tokens(conversation);
    current > (max_tokens as f64 * 0.8) as usize  // 80% threshold
}
```

**Compression Levels:**

**Level 1: Lossless Reduction**
- Remove redundant whitespace
- Strip code blocks already mentioned ("see previous code")
- Collapse repeated acknowledgments ("ok", "sure", "yes")

```rust
fn compress_lossless(messages: &mut Vec<Message>) {
    for msg in messages {
        msg.content = msg.content.trim().to_string();
        msg.content = remove_redundant_code_blocks(&msg.content);
    }
}
```

**Level 2: Middle Message Summarization**
- Keep first message (sets context)
- Keep last 3 messages (immediate context)
- Summarize middle with bullet points

```rust
fn compress_middle(messages: &[Message]) -> Vec<Message> {
    let first = messages[0].clone();
    let middle = &messages[1..messages.len()-3];
    let last_three = messages[messages.len()-3..].to_vec();

    let summary = Message {
        role: Role::System,
        content: format!(
            "[Summarized {} messages: User asked about {}. Assistant explained {} and provided code examples.]",
            middle.len(),
            extract_user_topics(middle),
            extract_assistant_topics(middle)
        ),
        timestamp: middle.first().unwrap().timestamp,
        metadata: HashMap::from([("compressed".into(), "true".into())]),
    };

    vec![vec![first, summary], last_three].concat()
}
```

**Level 3: Aggressive Pruning**
- Keep only essential decision points
- Remove all code blocks (reference "code was provided earlier")
- Distill to 5-10 bullet points maximum

```rust
fn compress_aggressive(messages: &[Message]) -> Vec<Message> {
    let key_points = extract_key_decisions(messages);

    vec![Message {
        role: Role::System,
        content: format!(
            "Previous conversation summary:\n{}",
            key_points.iter()
                .map(|p| format!("- {}", p))
                .collect::<Vec<_>>()
                .join("\n")
        ),
        timestamp: messages.first().unwrap().timestamp,
        metadata: HashMap::from([("compressed".into(), "aggressive".into())]),
    }]
}
```

**Level 4: Emergency Truncation**
- If all else fails, hard truncate to last N messages
- Warn user that early context is lost
- Suggest starting new session

### Token Estimation Heuristics

**Without API Access:**

```rust
fn estimate_tokens(conversation: &Conversation) -> usize {
    let mut total = 0;

    // System prompt
    if let Some(system) = &conversation.system_prompt {
        total += estimate_text_tokens(system);
    }

    // Messages
    for msg in &conversation.messages {
        total += estimate_text_tokens(&msg.content);
        total += 4;  // Role/metadata overhead
    }

    // Add 10% safety margin
    (total as f64 * 1.1) as usize
}

fn estimate_text_tokens(text: &str) -> usize {
    // Heuristic: ~4 characters per token for English
    // More conservative for code (more symbols/keywords)
    let char_count = text.chars().count();

    if looks_like_code(text) {
        char_count / 3  // Code is denser
    } else {
        char_count / 4  // Prose
    }
}

fn looks_like_code(text: &str) -> bool {
    let code_indicators = ["{", "}", "fn ", "def ", "class ", "import ", "const "];
    code_indicators.iter().any(|&indicator| text.contains(indicator))
}
```

**Safe Margins:**
- **Claude Opus**: 200K limit → use 180K max (90%)
- **Gemini 2.5**: 1M limit → use 900K max (90%)
- **Reason**: Account for estimation errors, metadata overhead, future message

### Session Persistence

**Auto-save conversation:**
```rust
pub struct SessionPersistence {
    session_dir: PathBuf,  // ~/.config/codex/sessions/
}

impl SessionPersistence {
    pub async fn save(&self, conversation: &Conversation) -> Result<()> {
        let file_path = self.session_dir.join(format!("{}.json", conversation.id));

        let json = serde_json::to_string_pretty(conversation)?;
        tokio::fs::write(&file_path, json).await?;

        Ok(())
    }

    pub async fn load(&self, session_id: Uuid) -> Result<Conversation> {
        let file_path = self.session_dir.join(format!("{}.json", session_id));
        let json = tokio::fs::read_to_string(&file_path).await?;
        let conversation = serde_json::from_str(&json)?;
        Ok(conversation)
    }

    pub async fn list_recent(&self, limit: usize) -> Result<Vec<ConversationSummary>> {
        // Return list of recent sessions with metadata
        // User can resume from TUI
    }
}
```

**Benefits:**
- Survive app crashes/restarts
- Resume long-running sessions
- Archive old conversations for reference

---

## 3.5 Streaming & UX Plan

### Stdout Chunks → Streamed Tokens

**Challenges:**
1. **Buffering**: CLIs may buffer output (stdio, line buffering)
2. **Partial UTF-8**: Bytes may split multi-byte characters
3. **ANSI codes**: Color/formatting escapes need stripping
4. **Markdown**: Code blocks need special handling for syntax highlighting

**Solution Architecture:**

```rust
pub struct StreamProcessor {
    utf8_buffer: Vec<u8>,
    ansi_stripper: AnsiStripper,
    markdown_parser: MarkdownStreamParser,
}

impl StreamProcessor {
    pub fn process_chunk(&mut self, bytes: &[u8]) -> Vec<UiEvent> {
        // 1. Handle partial UTF-8
        self.utf8_buffer.extend_from_slice(bytes);

        let valid_str = match std::str::from_utf8(&self.utf8_buffer) {
            Ok(s) => {
                let result = s.to_string();
                self.utf8_buffer.clear();
                result
            }
            Err(e) => {
                let valid_up_to = e.valid_up_to();
                if valid_up_to == 0 {
                    return vec![];  // Need more bytes
                }

                let valid_bytes = self.utf8_buffer.drain(..valid_up_to).collect::<Vec<_>>();
                String::from_utf8(valid_bytes).unwrap()
            }
        };

        // 2. Strip ANSI
        let clean = self.ansi_stripper.strip(&valid_str);

        // 3. Parse markdown structure
        let events = self.markdown_parser.process(&clean);

        events
    }
}

pub struct AnsiStripper {
    regex: Regex,
}

impl AnsiStripper {
    pub fn new() -> Self {
        Self {
            regex: Regex::new(r"\x1b\[[0-9;]*[a-zA-Z]").unwrap(),
        }
    }

    pub fn strip(&self, text: &str) -> String {
        self.regex.replace_all(text, "").to_string()
    }
}

pub struct MarkdownStreamParser {
    state: ParserState,
    current_code_block: Option<CodeBlock>,
}

#[derive(Debug)]
enum ParserState {
    PlainText,
    InCodeBlock { language: String, start_line: usize },
}

#[derive(Debug, Clone)]
pub struct CodeBlock {
    pub language: String,
    pub content: String,
    pub complete: bool,
}

pub enum UiEvent {
    TextDelta(String),
    CodeBlockStart { language: String },
    CodeBlockContent(String),
    CodeBlockEnd,
}

impl MarkdownStreamParser {
    pub fn process(&mut self, text: &str) -> Vec<UiEvent> {
        let mut events = Vec::new();

        for line in text.lines() {
            if line.starts_with("```") {
                match &self.state {
                    ParserState::PlainText => {
                        // Start code block
                        let language = line.trim_start_matches("```").trim().to_string();
                        self.state = ParserState::InCodeBlock {
                            language: language.clone(),
                            start_line: 0
                        };
                        events.push(UiEvent::CodeBlockStart { language });
                    }
                    ParserState::InCodeBlock { .. } => {
                        // End code block
                        self.state = ParserState::PlainText;
                        events.push(UiEvent::CodeBlockEnd);
                    }
                }
            } else {
                match &self.state {
                    ParserState::PlainText => {
                        events.push(UiEvent::TextDelta(line.to_string() + "\n"));
                    }
                    ParserState::InCodeBlock { .. } => {
                        events.push(UiEvent::CodeBlockContent(line.to_string() + "\n"));
                    }
                }
            }
        }

        events
    }
}
```

### UX State Machine

**States:**
```rust
pub enum ResponseState {
    Idle,
    Thinking,           // CLI process spawned, waiting for first byte
    Streaming,          // Receiving tokens
    Complete,           // Response finished successfully
    Error(CliError),    // Failed
    Cancelled,          // User cancelled
}
```

**TUI Rendering:**
```rust
impl ChatWidget {
    fn render_response(&self, state: &ResponseState, partial_text: &str) {
        match state {
            ResponseState::Idle => {
                // Show prompt, waiting for user input
            }
            ResponseState::Thinking => {
                self.render_spinner("Claude is thinking...");
            }
            ResponseState::Streaming => {
                // Render partial response with syntax highlighting
                self.render_markdown(partial_text);
                self.render_cursor();  // Blinking cursor at end
            }
            ResponseState::Complete => {
                self.render_markdown(partial_text);
                self.render_complete_indicator();  // ✓ or timestamp
            }
            ResponseState::Error(err) => {
                self.render_error(err);
                self.render_retry_button();
            }
            ResponseState::Cancelled => {
                self.render_cancelled_indicator();
            }
        }
    }
}
```

### Latency Masking Techniques

**1. Optimistic UI Updates**
```rust
// User presses Enter
async fn on_user_submit(message: String) {
    // Immediately show user message (no wait)
    ui.append_message(Message {
        role: Role::User,
        content: message.clone(),
        timestamp: Some(Utc::now()),
    });

    // Show "thinking" indicator immediately
    ui.set_state(ResponseState::Thinking);

    // Start async request (don't block UI)
    let rx = executor.execute(conversation, &message).await;

    // Stream results as they arrive
    while let Some(event) = rx.recv().await {
        match event {
            StreamEvent::Delta(text) => {
                ui.append_delta(&text);
                ui.set_state(ResponseState::Streaming);
            }
            StreamEvent::Done => {
                ui.set_state(ResponseState::Complete);
            }
            // ...
        }
    }
}
```

**2. Progressive Rendering**
- Don't wait for newlines; render character-by-character
- Syntax highlight code blocks as they stream in
- Show partial markdown formatting (bold, italics) immediately

**3. Buffered Writes (avoid flicker)**
```rust
pub struct StreamBuffer {
    pending: String,
    flush_interval: Duration,
    last_flush: Instant,
}

impl StreamBuffer {
    pub fn add_delta(&mut self, delta: String) -> Option<String> {
        self.pending.push_str(&delta);

        // Flush if:
        // - Buffer > 100 chars (smooth flow)
        // - Time since last flush > 50ms (avoid stutter)
        // - Contains newline (semantic boundary)
        if self.pending.len() > 100
            || self.last_flush.elapsed() > self.flush_interval
            || self.pending.contains('\n')
        {
            let result = std::mem::take(&mut self.pending);
            self.last_flush = Instant::now();
            Some(result)
        } else {
            None
        }
    }
}
```

### Cancellation Handling

**User-initiated cancellation (Ctrl+C):**
```rust
impl CliExecutor {
    pub async fn execute_cancellable(
        &self,
        conversation: &Conversation,
        message: &str,
        cancel_token: CancellationToken,
    ) -> Result<mpsc::Receiver<StreamEvent>, CliError> {
        let (tx, rx) = mpsc::channel(100);

        let child_handle = tokio::spawn(async move {
            // Spawn CLI process
            let mut child = Command::new("claude").spawn().unwrap();

            tokio::select! {
                // Wait for natural completion
                status = child.wait() => {
                    // Normal flow
                }

                // Or wait for cancellation
                _ = cancel_token.cancelled() => {
                    // Kill child process
                    let _ = child.kill().await;

                    // Send cancellation event
                    let _ = tx.send(StreamEvent::Error(CliError::Internal {
                        message: "Cancelled by user".to_string(),
                    })).await;
                }
            }
        });

        Ok(rx)
    }
}
```

**TUI integration:**
```rust
// User presses Ctrl+C
fn on_cancel_key(&mut self) {
    if let Some(cancel_token) = &self.active_request_cancel {
        cancel_token.cancel();
        self.ui_state = ResponseState::Cancelled;
    }
}
```

**Responsiveness:**
- Cancellation should be near-instant (<100ms)
- Clean up child process to avoid zombies
- Show clear UI feedback ("Request cancelled")

---

## 3.6 Testing & Benchmarking Plan

### Unit Tests

**Context formatting:**
```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_format_simple_conversation() {
        let mut conv = Conversation::new("test-model");
        conv.messages.push(Message::user("Hello"));
        conv.messages.push(Message::assistant("Hi there!"));
        conv.messages.push(Message::user("How are you?"));

        let mgr = CliContextManager::new(100_000);
        let formatted = mgr.format_history(&conv);

        assert!(formatted.contains("USER (2024-"));
        assert!(formatted.contains("ASSISTANT (2024-"));
        assert!(formatted.contains("--- Previous Conversation ---"));
        assert!(formatted.contains("USER (current):\nHow are you?"));
    }

    #[test]
    fn test_compression_threshold() {
        let mut conv = Conversation::new("test-model");

        // Add 100 long messages
        for i in 0..100 {
            conv.messages.push(Message::user(&"x".repeat(1000)));
        }

        let mgr = CliContextManager::new(10_000);  // Small limit
        mgr.compress_if_needed(&mut conv).unwrap();

        assert!(conv.messages.len() < 100);  // Should be compressed
        assert!(mgr.estimate_tokens(&conv) < 10_000);
    }

    #[test]
    fn test_token_estimation() {
        let mgr = CliContextManager::new(100_000);

        let text = "This is a test message with about 10 words in it.";
        let tokens = mgr.estimate_text_tokens(text);

        // ~10 words * ~1.3 tokens/word ≈ 13 tokens
        assert!(tokens >= 10 && tokens <= 20);
    }
}
```

**Error classification:**
```rust
#[test]
fn test_error_retryability() {
    assert!(CliError::RateLimited { retry_after: None }.is_retryable());
    assert!(CliError::Network { message: "timeout".into() }.is_retryable());
    assert!(!CliError::BinaryNotFound {
        binary: "claude".into(),
        install_hint: "...".into()
    }.is_retryable());
}

#[test]
fn test_user_action_hints() {
    let err = CliError::NotAuthenticated {
        auth_command: "claude login".to_string(),
    };

    assert_eq!(err.user_action(), "Run: claude login");
}
```

**Stream processing:**
```rust
#[test]
fn test_partial_utf8_handling() {
    let mut handler = CliStreamHandler::new();

    // Split multi-byte character (€ = 0xE2 0x82 0xAC)
    let chunk1 = &[0xE2, 0x82];  // Incomplete
    let chunk2 = &[0xAC];        // Completes it

    let events1 = handler.process_chunk(chunk1);
    assert!(events1.is_empty());  // Should buffer

    let events2 = handler.process_chunk(chunk2);
    assert_eq!(events2.len(), 1);
    match &events2[0] {
        StreamEvent::Delta(s) => assert_eq!(s, "€"),
        _ => panic!("Wrong event type"),
    }
}

#[test]
fn test_ansi_stripping() {
    let stripper = AnsiStripper::new();
    let input = "\x1b[1;31mError:\x1b[0m Something failed";
    let output = stripper.strip(input);
    assert_eq!(output, "Error: Something failed");
}
```

### Integration Tests (Require CLI Installed)

```rust
#[tokio::test]
#[ignore]  // Only run when CLIs are available
async fn test_claude_health_check() {
    let executor = ClaudeCliExecutor::new("claude-opus-4.1".into(), None);
    let result = executor.health_check().await;

    match result {
        Ok(_) => println!("Claude CLI is authenticated"),
        Err(CliError::BinaryNotFound { .. }) => {
            eprintln!("Skipping: Claude CLI not installed");
        }
        Err(CliError::NotAuthenticated { auth_command }) => {
            eprintln!("Skipping: Not authenticated. Run: {}", auth_command);
        }
        Err(e) => panic!("Unexpected error: {}", e),
    }
}

#[tokio::test]
#[ignore]
async fn test_streaming_response() {
    let executor = ClaudeCliExecutor::new("claude-opus-4.1".into(), None);
    let conversation = Conversation::new("claude-opus-4.1");

    let mut rx = executor.execute(&conversation, "Say 'hello' three times").await.unwrap();

    let mut full_response = String::new();
    let start = Instant::now();
    let mut first_byte_latency = None;

    while let Some(event) = rx.recv().await {
        match event {
            StreamEvent::Delta(text) => {
                if first_byte_latency.is_none() {
                    first_byte_latency = Some(start.elapsed());
                }
                full_response.push_str(&text);
            }
            StreamEvent::Done => break,
            StreamEvent::Error(e) => panic!("Stream error: {}", e),
            _ => {}
        }
    }

    println!("First byte latency: {:?}", first_byte_latency.unwrap());
    println!("Total time: {:?}", start.elapsed());
    println!("Response: {}", full_response);

    assert!(full_response.to_lowercase().contains("hello"));
}

#[tokio::test]
#[ignore]
async fn test_multi_turn_conversation() {
    let executor = ClaudeCliExecutor::new("claude-opus-4.1".into(), None);
    let mut conversation = Conversation::new("claude-opus-4.1");

    // Turn 1
    let mut rx1 = executor.execute(&conversation, "My name is Alice").await.unwrap();
    let response1 = collect_response(rx1).await;
    conversation.messages.push(Message::user("My name is Alice"));
    conversation.messages.push(Message::assistant(&response1));

    // Turn 2
    let mut rx2 = executor.execute(&conversation, "What's my name?").await.unwrap();
    let response2 = collect_response(rx2).await;

    println!("Response 2: {}", response2);
    assert!(response2.to_lowercase().contains("alice"));
}

async fn collect_response(mut rx: mpsc::Receiver<StreamEvent>) -> String {
    let mut result = String::new();
    while let Some(event) = rx.recv().await {
        if let StreamEvent::Delta(text) = event {
            result.push_str(&text);
        }
    }
    result
}
```

### Performance Benchmarks

**Startup time:**
```rust
use criterion::{black_box, criterion_group, criterion_main, Criterion};

fn bench_cli_startup(c: &mut Criterion) {
    c.bench_function("claude cli startup", |b| {
        b.iter(|| {
            let output = std::process::Command::new("claude")
                .arg("--version")
                .output()
                .unwrap();
            black_box(output)
        });
    });
}

criterion_group!(benches, bench_cli_startup);
criterion_main!(benches);
```

**Streaming throughput:**
```bash
# Generate large response and measure tokens/second
time claude --message "Write a 1000-line Python program" | wc -l

# Measure latency distribution
for i in {1..100}; do
    (time -p claude --message "Say hi") 2>&1 | grep real
done | awk '{sum+=$2; sumsq+=$2*$2} END {print "Mean:", sum/NR, "StdDev:", sqrt(sumsq/NR - (sum/NR)^2)}'
```

**Memory footprint:**
```bash
# Monitor memory during long conversation
/usr/bin/time -v ./my-tui-app <<EOF
# Paste 100-message conversation
EOF

# Look for "Maximum resident set size"
```

**Target Metrics:**
- CLI startup: <500ms (P50), <1s (P95)
- First byte latency: <200ms (P50), <500ms (P95)
- Streaming throughput: >50 tokens/sec
- Memory footprint: <100MB for 100-message conversation

---

## Summary & Action Plan

### Key Assumptions Made

1. **Claude/Gemini CLIs** likely do NOT have native session modes (persistent processes)
2. Both CLIs probably support `--stream` for incremental output
3. Input via stdin or `--message` flags is supported
4. JSON structured input/output may be available but not guaranteed
5. ANSI escape codes will be present in output and need stripping
6. Startup overhead is ~200-500ms (typical for Python/Node CLIs)
7. No official documentation exists yet (beta CLIs)

### Critical Unknowns (Resolve via experiments)

| Unknown | Experiment | Risk if Wrong |
|---------|-----------|---------------|
| Does `claude` support multi-turn sessions natively? | `claude chat` exploration | May need full history replay |
| What's the exact streaming format? | Capture stdout with `script` | Parser implementation changes |
| Token limits in practice? | Send 100K+ token prompts | Context compression urgency |
| Startup latency distribution? | Benchmark 100 runs | UX expectations |
| How are errors surfaced? | Trigger auth/network/rate errors | Error handling completeness |
| Does Gemini have a standalone CLI? | Check Google Cloud docs | May need `gcloud ai` wrapper |

### Recommended Architecture: **Option A (Stateless Per-Request)**

**Why:**
- Guaranteed to work regardless of CLI capabilities
- Simple, testable, debuggable
- No persistent state to corrupt
- Easy retry/recovery logic

**Trade-off Accepted:**
- Higher latency (startup overhead)
- Token waste (re-sending history)

**Mitigations:**
- Smart compression to keep history under limits
- Optimistic UI to mask latency
- Request coalescing for rapid edits

---

## Step-by-Step Action Plan

### Week 1: Discovery & Validation

**Day 1-2: CLI Capabilities Audit**
```bash
# Run all checklist experiments from section 3.1
claude --help > /tmp/claude-help.txt
# Test multi-turn, streaming, errors, limits
# Document findings in discovery.md
```

**Day 3-4: Spike Implementation**
- Implement bare-bones `ClaudeCliExecutor`
- Test: spawn process, write stdin, read stdout
- Validate UTF-8 handling, ANSI stripping
- Measure actual startup latency

**Day 5: Architecture Decision**
- Review discovery findings
- Decide: Option A, or Option B if session mode exists
- Write decision doc with rationale

---

### Week 2: Core Implementation (Option A)

**Day 1-2: Context Management**
- Implement `CliContextManager`
- History formatting logic
- Token estimation heuristics
- Compression strategies (lossless → aggressive)
- Unit tests

**Day 3-4: Streaming & Error Handling**
- Implement `CliStreamHandler`
- UTF-8 buffer logic
- ANSI stripper
- Markdown parser for code blocks
- Error classification and retry logic

**Day 5: Integration**
- Wire up `CliExecutor` trait
- Connect to existing `ModelRouter`
- Basic TUI integration
- Manual testing with real CLIs

---

### Week 3: Robustness & UX

**Day 1-2: Error Recovery**
- Implement exponential backoff retries
- Health check system
- User-friendly error messages
- Fallback to OpenAI if CLI fails

**Day 3-4: UX Polish**
- Optimistic UI updates
- Streaming state machine
- Cancellation handling (Ctrl+C)
- Progressive markdown rendering
- Latency masking techniques

**Day 5: Session Persistence**
- Implement `SessionPersistence`
- Save/load conversation history
- Recent sessions list
- Resume support

---

### Week 4: Testing & Optimization

**Day 1-2: Test Suite**
- Unit tests for all components
- Integration tests (require CLIs)
- Error path coverage
- Compression edge cases

**Day 3: Performance Benchmarking**
- Startup latency measurements
- Streaming throughput tests
- Memory profiling
- Identify bottlenecks

**Day 4: Optimization**
- Address performance issues
- Tune buffer sizes
- Optimize token estimation
- Reduce allocations in hot path

**Day 5: Documentation & Handoff**
- API documentation
- Architecture decision record
- User guide (setup CLIs, auth, troubleshooting)
- Known limitations doc

---

### Week 5 (Optional): Advanced Features

**If Option B is viable:**
- Implement persistent session mode
- Protocol handler for JSON-lines
- Heartbeat and reconnection logic
- A/B test latency vs Option A

**If staying with Option A:**
- Implement request coalescing
- Smart caching of recent responses
- Context compression tuning based on real usage
- Multi-provider support (Gemini)

---

## Success Criteria Checklist

**Functional:**
- ✅ Full conversation history preserved (or intentionally compressed)
- ✅ Streaming responses with <100ms chunk latency
- ✅ Retry logic with exponential backoff
- ✅ Graceful fallback to OpenAI on CLI failure
- ✅ Session persistence across restarts

**Performance:**
- ✅ First response within 1s (P50), 2s (P95)
- ✅ Streaming feels real-time (50+ tokens/sec)
- ✅ Memory footprint <100MB for 100-message conversation
- ✅ Startup overhead masked by optimistic UI

**UX:**
- ✅ Smooth streaming (no lumpy flushes)
- ✅ Clear error messages with actionable hints
- ✅ Fast cancellation (<100ms)
- ✅ Progressive markdown rendering
- ✅ No perceived difference from native API

**Robustness:**
- ✅ Stable over 8-hour coding sessions
- ✅ Handles network blips gracefully
- ✅ No zombie processes
- ✅ Cross-platform (Linux/macOS/Windows)

---

## Final Recommendations

1. **Start with Option A** - It's guaranteed to work and ships fast.

2. **Invest heavily in UX polish** - Latency is acceptable if masked well.

3. **Make compression smart** - This is your only defense against context limits.

4. **Test error paths thoroughly** - CLIs fail in creative ways.

5. **Keep Option B as a future optimization** - If CLIs add session modes, you can layer it on.

6. **Document limitations** - Be honest with users about latency vs API.

7. **Build instrumentation from day 1** - Metrics on latency, token usage, compression rates will guide optimization.

This architecture prioritizes **reliability and simplicity** over raw performance, which is correct for an MVP. You can always optimize later once you have real usage data.
