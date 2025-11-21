# Gemini PTY Provider Design (Option F)

**Date**: 2025-11-21
**Status**: Design Phase
**Goal**: Persistent interactive Gemini CLI wrapper via PTY

---

## Executive Summary

**Problem**: Gemini CLI headless mode is stateless and unreliable for multi-turn conversations.

**Solution**: Wrap Gemini CLI **interactive mode** using a persistent PTY, letting Gemini CLI manage its own state.

**Benefits**:
- ✅ Gemini CLI owns conversation history (not us)
- ✅ Tool usage (search, shell, git) handled natively
- ✅ Context compression (/compress) built-in
- ✅ Session checkpoints (/chat save/resume) available
- ✅ Persistent memory (GEMINI.md) automatic
- ✅ True multi-turn (not synthetic JSON state)

---

## Architecture Overview

### Component Hierarchy

```
┌─────────────────────────────────────────────┐
│ TUI (ChatWidget)                             │
│  - User types message                        │
│  - Receives StreamEvent deltas               │
└────────────────┬────────────────────────────┘
                 │
                 v
┌─────────────────────────────────────────────┐
│ ModelRouter                                  │
│  - Detects Gemini model                      │
│  - Routes to GeminiPtyProvider               │
└────────────────┬────────────────────────────┘
                 │
                 v
┌─────────────────────────────────────────────┐
│ GeminiPtyProvider                            │
│  - Manages GeminiPtySession lifecycle        │
│  - Converts StreamEvent to TUI format        │
└────────────────┬────────────────────────────┘
                 │
                 v
┌─────────────────────────────────────────────┐
│ GeminiPtySession                             │
│  - Owns PTY + gemini child process           │
│  - Sends user messages to PTY stdin          │
│  - Reads PTY stdout/stderr                   │
│  - Detects response completion (prompt)      │
│  - Handles cancellation (Ctrl+C)             │
└────────────────┬────────────────────────────┘
                 │
                 v
┌─────────────────────────────────────────────┐
│ PTY (pty-process crate)                      │
│  - Cross-platform PTY abstraction            │
│  - Async I/O with tokio                      │
└────────────────┬────────────────────────────┘
                 │
                 v
┌─────────────────────────────────────────────┐
│ gemini (interactive CLI process)             │
│  - Owns conversation state                   │
│  - Handles tools, compression, memory        │
└─────────────────────────────────────────────┘
```

---

## Key Design Decisions

### 1. PTY Library: `pty-process` with `async` feature

**Choice**: `pty-process` (not `portable-pty` or `tokio-pty-process`)

**Rationale**:
| Library | Pros | Cons | Verdict |
|---------|------|------|---------|
| portable-pty | Cross-platform, wezterm-backed | Blocking I/O (need spawn_blocking) | ❌ Not async-native |
| pty-process | Async tokio, modern, both APIs | Less battle-tested than portable-pty | ✅ **CHOSEN** |
| tokio-pty-process | Tokio integration | Unix-only, outdated (2019) | ❌ Windows unsupported |

**Dependencies**:
```toml
[dependencies]
pty-process = { version = "0.4", features = ["async"] }
tokio = { version = "1", features = ["process", "io-util"] }
```

### 2. Session Lifecycle

**One PTY per conversation** (not per message):

```rust
// On first message for a conversation:
session = GeminiPtySession::new()?;
session.start().await?;

// On subsequent messages (same conversation):
session.send_message(text).await?;
let response = session.read_until_prompt().await?;

// On conversation end or timeout:
session.shutdown().await?;
```

**Benefits**:
- Gemini CLI maintains full state
- No re-parsing of history
- Tools remain available across turns
- Compression handled automatically

### 3. Prompt Detection (Critical Component)

**Challenge**: Know when Gemini's response is complete.

**Approach**: Multi-signal heuristic

```rust
enum PromptState {
    Responding,      // CLI is generating text
    Idle,            // CLI returned to prompt
    Waiting,         // Ambiguous (collecting data)
}

fn detect_prompt_state(buffer: &str) -> PromptState {
    // Signal 1: Gemini prompt marker
    if buffer.ends_with("\n> ") || buffer.ends_with("\ngemini> ") {
        return PromptState::Idle;
    }

    // Signal 2: No output for N milliseconds + looks complete
    if last_output_elapsed() > 500ms && looks_like_complete_response(buffer) {
        return PromptState::Idle;
    }

    // Signal 3: ANSI status line updates stopped
    if status_line_stable_for(300ms) {
        return PromptState::Idle;
    }

    PromptState::Responding
}
```

**Rationale**:
- Prompt markers most reliable (`> ` or `gemini> `)
- Idle timeout as fallback (500ms conservative)
- ANSI parsing for status updates (optional enhancement)

**Test cases**:
1. Simple response: "What's 2+2?" → "4\n> " (instant detection)
2. Long response: Code generation → pause after completion → detect prompt
3. Streaming: Detect text streaming vs idle state
4. Tool usage: CLI runs tools → returns to prompt

### 4. Streaming Integration

**Flow**:
```rust
async fn stream_response(&mut self, tx: mpsc::Sender<StreamEvent>) -> Result<String> {
    let mut accumulated = String::new();
    let mut buffer = Vec::new();

    loop {
        // Read from PTY (non-blocking)
        match self.pty_reader.read(&mut buffer).await {
            Ok(n) if n > 0 => {
                let text = String::from_utf8_lossy(&buffer[..n]);

                // Strip ANSI codes
                let clean_text = strip_ansi_codes(&text);

                // Emit delta
                tx.send(StreamEvent::Delta(clean_text.clone())).await?;
                accumulated.push_str(&clean_text);

                // Check if response complete
                if self.prompt_detector.is_complete(&accumulated) {
                    break;
                }
            }
            Ok(_) => break, // EOF
            Err(e) if e.kind() == ErrorKind::WouldBlock => {
                // No data yet, check timeout
                tokio::time::sleep(Duration::from_millis(10)).await;
                if self.response_timeout_exceeded() {
                    return Err(TimeoutError);
                }
            }
            Err(e) => return Err(e.into()),
        }
    }

    tx.send(StreamEvent::Done).await?;
    Ok(accumulated)
}
```

### 5. Cancellation Handling

**User presses cancel during generation**:

```rust
async fn cancel_current_response(&mut self) -> Result<()> {
    // Send Ctrl+C to PTY (SIGINT)
    self.pty.send_signal(Signal::SIGINT).await?;

    // Drain output until prompt returns
    let mut drain_buffer = Vec::new();
    loop {
        match timeout(Duration::from_secs(5), self.pty_reader.read(&mut drain_buffer)).await {
            Ok(Ok(n)) if n > 0 => {
                let text = String::from_utf8_lossy(&drain_buffer[..n]);
                if text.contains("> ") {
                    // Prompt detected, CLI ready for next message
                    break;
                }
            }
            _ => break, // Timeout or EOF
        }
    }

    Ok(())
}
```

**Benefits**:
- Session stays alive
- Next message works normally
- No need to restart CLI

### 6. Error Recovery

**Gemini CLI crashes or exits**:

```rust
async fn ensure_alive(&mut self) -> Result<()> {
    if !self.is_process_running() {
        tracing::warn!("Gemini CLI process died, restarting...");
        self.start().await?;

        // Optionally: restore state with /chat resume
        if let Some(checkpoint_id) = &self.last_checkpoint {
            self.send_command(&format!("/chat resume {}", checkpoint_id)).await?;
        }
    }
    Ok(())
}
```

**Checkpoint strategy**:
- Auto-save after every N messages (e.g., N=5)
- `send_command("/chat save auto_checkpoint_{id}").await?`
- On restart, resume from last checkpoint

---

## Implementation Phases

### Phase 1: Core PTY Wrapper (2-3h)

**Deliverables**:
- `gemini_pty_session.rs`: PTY spawn, read/write, basic I/O
- Spawn `gemini` in PTY
- Send single message
- Read raw output

**Tests**:
- Can spawn gemini
- Can send message and receive response
- Process stays alive

### Phase 2: Prompt Detection (2-3h)

**Deliverables**:
- `prompt_detector.rs`: Heuristic for completion detection
- Implement prompt marker detection (`> `)
- Implement idle timeout fallback (500ms)
- ANSI code stripping

**Tests**:
- Detects prompt on simple response
- Handles multi-line responses
- Timeout works for ambiguous cases

### Phase 3: Streaming Integration (1-2h)

**Deliverables**:
- Convert PTY output → StreamEvent
- Real-time delta emission
- Accumulated response tracking

**Tests**:
- Deltas emitted in real-time
- Final response accumulated correctly
- Done event sent when complete

### Phase 4: Provider Integration (1-2h)

**Deliverables**:
- `GeminiPtyProvider`: Implements provider interface
- Session management (start/reuse)
- Model routing integration

**Tests**:
- Multi-turn conversations work
- Context preserved across turns
- Model selection works

### Phase 5: Cancellation & Error Handling (2-3h)

**Deliverables**:
- Ctrl+C cancellation
- Process crash recovery
- Timeout handling
- Checkpoint auto-save

**Tests**:
- Cancel works without killing session
- Process restart restores state
- Timeouts handled gracefully

### Phase 6: Debug Harness (1h)

**Deliverables**:
- `gemini_pty_debug` binary
- Interactive testing tool
- Output mirroring

**Tests**:
- Manual testing via harness
- Debug output validation

**Total Estimate**: 9-14 hours

---

## API Design

### GeminiPtySession

```rust
pub struct GeminiPtySession {
    pty: Child,
    pty_reader: BufReader<Box<dyn AsyncRead + Unpin>>,
    pty_writer: Box<dyn AsyncWrite + Unpin>,
    prompt_detector: PromptDetector,
    model: String,
    conversation_id: Option<String>,
    last_checkpoint: Option<String>,
}

impl GeminiPtySession {
    /// Create new session (doesn't start process yet)
    pub fn new(model: &str) -> Self;

    /// Start Gemini CLI in PTY
    pub async fn start(&mut self) -> Result<()>;

    /// Send user message and stream response
    pub async fn send_message(
        &mut self,
        message: &str,
        tx: mpsc::Sender<StreamEvent>,
    ) -> Result<String>;

    /// Send CLI command (e.g., /compress, /chat save)
    pub async fn send_command(&mut self, command: &str) -> Result<()>;

    /// Cancel current generation
    pub async fn cancel(&mut self) -> Result<()>;

    /// Gracefully shutdown CLI
    pub async fn shutdown(mut self) -> Result<()>;

    /// Check if process is still running
    fn is_alive(&self) -> bool;
}
```

### PromptDetector

```rust
pub struct PromptDetector {
    last_output_time: Instant,
    idle_threshold: Duration,
    prompt_markers: Vec<&'static str>,
}

impl PromptDetector {
    pub fn new() -> Self;

    /// Check if response appears complete
    pub fn is_complete(&mut self, output: &str) -> bool;

    /// Update with new output
    pub fn update(&mut self, text: &str);

    /// Reset for new turn
    pub fn reset(&mut self);
}
```

### GeminiPtyProvider

```rust
pub struct GeminiPtyProvider {
    sessions: HashMap<String, GeminiPtySession>,
}

impl GeminiPtyProvider {
    pub fn new() -> Result<Self>;

    /// Execute with PTY (provider interface)
    pub async fn execute_streaming(
        &mut self,
        conversation_id: &str,
        message: &str,
        model: &str,
        tx: AppEventSender,
    ) -> ProviderResult<String>;

    /// Get or create session for conversation
    async fn get_session(&mut self, id: &str, model: &str) -> Result<&mut GeminiPtySession>;

    /// Cleanup expired sessions
    async fn cleanup_idle_sessions(&mut self);
}
```

---

## Prompt Detection Strategy

### Signals for Completion

**Primary Signal**: Prompt marker
```
> █
```
or
```
gemini> █
```

**Secondary Signal**: Idle timeout
- No output for 500ms
- Last line doesn't end with partial sentence
- No active streaming indicators

**Tertiary Signal**: ANSI status line
- Parse bottom status bar updates
- When status = "Idle" or "Ready"

### Detection Algorithm

```rust
pub fn is_complete(&mut self, buffer: &str) -> bool {
    let now = Instant::now();

    // Signal 1: Explicit prompt marker (highest confidence)
    if buffer.ends_with("\n> ") || buffer.ends_with("\ngemini> ") {
        self.confidence = High;
        return true;
    }

    // Signal 2: Idle timeout (medium confidence)
    if now.duration_since(self.last_output_time) > self.idle_threshold {
        // Check if looks complete
        if self.looks_complete(buffer) {
            self.confidence = Medium;
            return true;
        }
    }

    // Signal 3: Still active
    self.confidence = Low;
    false
}

fn looks_complete(&self, buffer: &str) -> bool {
    let last_line = buffer.lines().last().unwrap_or("");

    // Not complete if ends with incomplete markers
    if last_line.ends_with("...") || last_line.ends_with(",") {
        return false;
    }

    // Not complete if ends with code fence opener
    if buffer.trim_end().ends_with("```") && !buffer.contains("```\n```") {
        return false;
    }

    // Likely complete if ends with period, closing fence, or newline
    buffer.trim_end().ends_with(".")
        || buffer.trim_end().ends_with("```")
        || buffer.trim_end().ends_with("\n")
}
```

### False Positive Mitigation

**Problem**: Detect prompt too early (mid-response)

**Solutions**:
1. **Minimum response time**: Don't detect prompt <200ms after send
2. **Token counter**: Expect minimum output length for non-trivial questions
3. **Pattern matching**: Common response patterns (code blocks, lists, etc.)

---

## ANSI Code Handling

Gemini CLI outputs ANSI escape sequences for:
- Colors
- Cursor movement
- Status line updates
- Progress indicators

**Strategy**: Strip all ANSI before streaming to TUI

```rust
use strip_ansi_escapes;

fn clean_output(raw: &[u8]) -> String {
    let stripped = strip_ansi_escapes::strip(raw);
    String::from_utf8_lossy(&stripped).to_string()
}
```

---

## Session Management

### Lifecycle

```
[User Action]           [Session State]        [CLI Process]
───────────────────────────────────────────────────────────
First message      →    Create session    →    Spawn gemini
                        State: Starting        (PTY created)
                             ↓
Wait for prompt    ←    Ready             ←    Initialized
                             ↓
Send message       →    Sending           →    Write to PTY
                             ↓
Stream response    ←    Receiving         ←    Read from PTY
                             ↓
Detect prompt      ←    Ready             ←    Prompt detected
                             ↓
Next message       →    (reuse session)   →    (same process)
───────────────────────────────────────────────────────────
Close conv         →    Shutdown          →    Send /quit
                        State: Terminated      Process exits
```

### Session Reuse

**Key**: Keep same PTY/process alive for entire conversation.

```rust
pub struct SessionManager {
    sessions: HashMap<ConversationId, GeminiPtySession>,
}

impl SessionManager {
    async fn get_or_create(&mut self, id: &str, model: &str) -> &mut GeminiPtySession {
        if !self.sessions.contains_key(id) {
            let mut session = GeminiPtySession::new(model);
            session.start().await?;
            self.sessions.insert(id.to_string(), session);
        }
        self.sessions.get_mut(id).unwrap()
    }

    async fn cleanup_idle(&mut self, max_idle: Duration) {
        // Remove sessions idle > max_idle
        let now = Instant::now();
        self.sessions.retain(|_, session| {
            now.duration_since(session.last_activity) < max_idle
        });
    }
}
```

---

## Streaming Implementation

### Reading PTY Output

```rust
async fn read_and_stream(
    &mut self,
    tx: mpsc::Sender<StreamEvent>,
) -> Result<String> {
    let mut accumulated = String::new();
    let mut buffer = vec![0u8; 4096];

    loop {
        // Non-blocking read with timeout
        match timeout(Duration::from_millis(100), self.pty_reader.read(&mut buffer)).await {
            Ok(Ok(n)) if n > 0 => {
                // Process new data
                let raw_text = &buffer[..n];
                let clean_text = clean_output(raw_text);

                // Update prompt detector
                self.prompt_detector.update(&clean_text);

                // Emit delta (skip prompt markers)
                if !clean_text.contains(">") || !self.prompt_detector.is_complete(&accumulated) {
                    tx.send(StreamEvent::Delta(clean_text.clone())).await?;
                    accumulated.push_str(&clean_text);
                }

                // Check completion
                if self.prompt_detector.is_complete(&accumulated) {
                    break;
                }
            }
            Ok(Ok(_)) => {
                // EOF - process might have died
                return Err(ProcessDiedError);
            }
            Ok(Err(e)) => return Err(e.into()),
            Err(_) => {
                // Timeout - check if we're idle
                if self.prompt_detector.is_complete(&accumulated) {
                    break;
                }
            }
        }
    }

    Ok(accumulated)
}
```

---

## Cancellation Design

### User Cancels Generation

```rust
pub async fn cancel_current_generation(&mut self) -> Result<()> {
    tracing::info!("Cancelling current Gemini generation");

    // Send Ctrl+C to PTY
    self.pty_writer.write_all(b"\x03").await?;  // ASCII 0x03 = Ctrl+C
    self.pty_writer.flush().await?;

    // Drain output until prompt returns
    let deadline = Instant::now() + Duration::from_secs(5);
    let mut drain_buffer = vec![0u8; 4096];

    while Instant::now() < deadline {
        match timeout(Duration::from_millis(100), self.pty_reader.read(&mut drain_buffer)).await {
            Ok(Ok(n)) if n > 0 => {
                let text = String::from_utf8_lossy(&drain_buffer[..n]);

                // Check for prompt return
                if text.contains("> ") {
                    tracing::info!("Cancellation complete, prompt returned");
                    self.prompt_detector.reset();
                    return Ok(());
                }
            }
            _ => {
                tokio::time::sleep(Duration::from_millis(50)).await;
            }
        }
    }

    Err(CancellationTimeoutError)
}
```

---

## Error Handling

### Process Death Recovery

```rust
pub async fn ensure_alive(&mut self) -> Result<()> {
    // Check if process is still running
    match self.pty.try_wait() {
        Ok(Some(status)) => {
            // Process exited
            tracing::error!("Gemini CLI exited with status: {:?}", status);

            // Restart
            self.start().await?;

            // Try to restore from checkpoint
            if let Some(checkpoint) = &self.last_checkpoint {
                tracing::info!("Restoring from checkpoint: {}", checkpoint);
                self.send_command(&format!("/chat resume {}", checkpoint)).await?;
            } else {
                tracing::warn!("No checkpoint available, conversation state lost");
            }

            Ok(())
        }
        Ok(None) => {
            // Process still running
            Ok(())
        }
        Err(e) => Err(e.into()),
    }
}
```

### Auto-Checkpoint Strategy

```rust
pub async fn auto_checkpoint(&mut self, turn_count: usize) -> Result<()> {
    // Save every 5 turns
    if turn_count % 5 == 0 {
        let checkpoint_id = format!("auto_{}", turn_count);
        self.send_command(&format!("/chat save {}", checkpoint_id)).await?;
        self.last_checkpoint = Some(checkpoint_id);
        tracing::info!("Auto-checkpoint created at turn {}", turn_count);
    }
    Ok(())
}
```

---

## Testing Strategy

### Unit Tests

**Prompt Detection**:
```rust
#[test]
fn test_detect_simple_prompt() {
    let mut detector = PromptDetector::new();
    assert!(!detector.is_complete("4"));
    assert!(detector.is_complete("4\n> "));
}

#[test]
fn test_detect_multi_line() {
    let mut detector = PromptDetector::new();
    let response = "Here's the answer:\n- Point 1\n- Point 2\n\n> ";
    assert!(detector.is_complete(response));
}

#[test]
fn test_incomplete_code_fence() {
    let mut detector = PromptDetector::new();
    let response = "Here's code:\n```rust\nfn main() {}\n```";  // No prompt yet
    assert!(!detector.is_complete(response));
}
```

**ANSI Stripping**:
```rust
#[test]
fn test_strip_ansi() {
    let with_ansi = "\x1b[32mGreen text\x1b[0m";
    let clean = clean_output(with_ansi.as_bytes());
    assert_eq!(clean, "Green text");
}
```

### Integration Tests

**Multi-Turn**:
```rust
#[tokio::test]
async fn test_multi_turn_conversation() {
    let mut session = GeminiPtySession::new("gemini-2.5-flash");
    session.start().await.unwrap();

    // Turn 1
    let response1 = session.send_message("My name is Alice").await.unwrap();
    assert!(response1.len() > 0);

    // Turn 2
    let response2 = session.send_message("What's my name?").await.unwrap();
    assert!(response2.to_lowercase().contains("alice"));

    session.shutdown().await.unwrap();
}
```

**Cancellation**:
```rust
#[tokio::test]
async fn test_cancellation() {
    let mut session = GeminiPtySession::new("gemini-2.5-flash");
    session.start().await.unwrap();

    // Start long generation
    let (tx, mut rx) = mpsc::channel(100);
    let send_task = tokio::spawn(async move {
        session.send_message("Write a 1000-line poem", tx).await
    });

    // Wait for some output
    tokio::time::sleep(Duration::from_secs(1)).await;

    // Cancel
    session.cancel().await.unwrap();

    // Session should still be alive
    assert!(session.is_alive());

    // Next message should work
    let response = session.send_message("What's 2+2?").await.unwrap();
    assert!(response.contains("4"));
}
```

### Debug Harness

```rust
// bin/gemini_pty_debug.rs

#[tokio::main]
async fn main() -> Result<()> {
    // Setup logging
    tracing_subscriber::fmt::init();

    // Create session
    let mut session = GeminiPtySession::new("gemini-2.5-flash");
    session.start().await?;

    println!("Gemini PTY Debug Harness");
    println!("Type messages (Ctrl+D to quit)");
    println!("Commands: /compress, /chat save <id>, /chat resume <id>");
    println!("────────────────────────────────────");

    // Read from stdin, send to PTY
    let stdin = tokio::io::BufReader::new(tokio::io::stdin());
    let mut lines = stdin.lines();

    while let Some(line) = lines.next_line().await? {
        if line.is_empty() {
            continue;
        }

        println!("\n[Sending: {}]", line);

        // Send to Gemini
        let (tx, mut rx) = mpsc::channel(100);
        let send_task = tokio::spawn(async move {
            session.send_message(&line, tx).await
        });

        // Stream output
        print!("\n[Response] ");
        while let Some(event) = rx.recv().await {
            match event {
                StreamEvent::Delta(text) => print!("{}", text),
                StreamEvent::Done => {
                    println!("\n[Done]");
                    break;
                }
                StreamEvent::Error(e) => {
                    println!("\n[Error: {}]", e);
                    break;
                }
                _ => {}
            }
        }

        send_task.await??;
        println!("\n────────────────────────────────────");
    }

    session.shutdown().await?;
    Ok(())
}
```

---

## Configuration

### Model Support

All Gemini models use same PTY wrapper (model passed to CLI):

```rust
pub enum GeminiModel {
    Flash25,      // gemini-2.5-flash
    Pro25,        // gemini-2.5-pro
    Pro3Preview,  // gemini-3-pro-preview
}

impl GeminiModel {
    fn to_cli_arg(&self) -> &str {
        match self {
            GeminiModel::Flash25 => "--model=gemini-2.5-flash",
            GeminiModel::Pro25 => "--model=gemini-2.5-pro",
            GeminiModel::Pro3Preview => "--model=gemini-3-pro-preview",
        }
    }
}
```

### Environment Variables

```bash
# PTY configuration
GEMINI_PTY_ENABLED=1              # Enable PTY wrapper (default: 1)
GEMINI_PTY_IDLE_THRESHOLD_MS=500  # Prompt detection timeout (default: 500)
GEMINI_PTY_AUTO_CHECKPOINT=5      # Checkpoint every N turns (default: 5)
GEMINI_PTY_MAX_RESPONSE_TIME=120  # Max response time (default: 120s)

# Fallback configuration
GEMINI_HEADLESS_FALLBACK=0        # Fall back to headless if PTY fails (default: 0)
```

---

## Migration Path

### Phase 1: Parallel Implementation
- Keep existing headless code
- Add new PTY provider
- Both available, PTY is default

### Phase 2: Testing & Validation
- Test PTY with all 3 Gemini models
- Validate multi-turn (10+ messages)
- Stress test (long conversations, cancellations)

### Phase 3: Deprecation
- Mark headless as deprecated
- Update documentation
- Eventually remove headless code

---

## Risk Analysis

### Risk 1: Prompt Detection Reliability

**Issue**: False positives (detect too early) or false negatives (wait too long)

**Probability**: Medium (hardest part of design)

**Mitigation**:
- Multi-signal approach (prompt + idle + ANSI)
- Tunable thresholds (config)
- Extensive testing with various response types
- Fallback to timeout (120s max)

### Risk 2: Platform Compatibility

**Issue**: PTY behavior differs on Windows vs Unix

**Probability**: Medium (Windows PTY less mature)

**Mitigation**:
- Use `pty-process` (tested cross-platform)
- Extensive Windows testing
- Fallback to headless on Windows if needed

### Risk 3: Process Management Complexity

**Issue**: Cleanup, zombies, resource leaks

**Probability**: Low (tokio handles this well)

**Mitigation**:
- Proper Drop implementation
- Timeout-based cleanup
- Resource monitoring in tests

### Risk 4: ANSI Parsing Edge Cases

**Issue**: Malformed ANSI sequences, partial reads

**Probability**: Low (strip_ansi_escapes is mature)

**Mitigation**:
- Use battle-tested library
- Fallback to raw output on parse errors

---

## Success Criteria

### Functional
- ✅ All 3 Gemini models work via PTY
- ✅ Multi-turn conversations (20+ messages) work
- ✅ Context preserved correctly
- ✅ Cancellation works without killing session
- ✅ Process crashes recover gracefully

### Performance
- ✅ Response time <10s for typical queries
- ✅ No timeouts for conversations <50 messages
- ✅ Prompt detection <1s idle time
- ✅ Session startup <2s

### Quality
- ✅ 15+ tests passing (unit + integration)
- ✅ No clippy warnings
- ✅ Debug harness works
- ✅ Documentation complete

---

## Next Steps

1. ✅ Research complete (Gemini CLI + PTY libraries)
2. ✅ Design document complete
3. **→ Begin implementation**: Start with Phase 1 (Core PTY Wrapper)

**Ready to implement!** 🚀

Would you like me to proceed with Phase 1 implementation?