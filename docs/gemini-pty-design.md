# Gemini PTY Provider Design (Option F)

**SPEC**: SPEC-952-F
**Status**: Implementation
**Date**: 2025-11-21

## Executive Summary

This document describes the Gemini PTY provider implementation using `expectrl` for interactive CLI session management. This replaces the failed headless approach (SPEC-952-B) and the incomplete `portable-pty` attempt.

### Why PTY Instead of Headless?

**Headless mode (`-p` flag)** limitations discovered:
- No true multi-turn conversation state
- Requires passing full conversation history as JSON (synthetic state)
- Timeouts after 2-3 messages (reliability issues)
- No access to CLI features: tools, memory, compression, checkpoints

**PTY interactive mode** advantages:
- Gemini CLI owns and manages conversation state natively
- True multi-turn conversations (no state reconstruction)
- Full CLI feature access: `/compress`, `/chat save/resume`, tools, memory
- Stable, reliable behavior (how CLI is designed to be used)

## Architecture Overview

```
┌─────────────────────────────────────────┐
│         TUI / ModelRouter               │
│                                         │
│  - Routes Gemini models to provider     │
│  - Manages conversation lifecycle       │
└────────────────┬────────────────────────┘
                 │
                 ▼
┌─────────────────────────────────────────┐
│      GeminiPtyProvider                  │
│                                         │
│  - Manages session pool                 │
│  - Maps ConversationId → Session        │
│  - Spawns blocking tasks                │
└────────────────┬────────────────────────┘
                 │
                 ▼
┌─────────────────────────────────────────┐
│      GeminiPtySession                   │
│                                         │
│  - Owns expectrl::Session               │
│  - Manages single long-lived CLI process│
│  - Handles prompt detection             │
│  - Streams output via channel           │
└────────────────┬────────────────────────┘
                 │
                 ▼
┌─────────────────────────────────────────┐
│         expectrl::Session               │
│                                         │
│  - PTY automation library               │
│  - Sync I/O (smol async runtime)       │
│  - Wrapped in spawn_blocking            │
└────────────────┬────────────────────────┘
                 │
                 ▼
┌─────────────────────────────────────────┐
│      Gemini CLI (interactive)           │
│                                         │
│  - Manages conversation history         │
│  - Tools: search, shell, git            │
│  - Memory: GEMINI.md integration        │
│  - Compression: /compress command       │
│  - Checkpoints: /chat save/resume       │
└─────────────────────────────────────────┘
```

## Why `expectrl`?

### Research Findings (crates.io 2025-11-21)

- **Version**: 0.7.1 (stable, circa 2022)
- **Async Runtime**: Uses `async-io` (smol ecosystem), NOT Tokio
- **API**: Session-based automation with `spawn()`, `send_line()`, `expect()`
- **Platform**: Unix-first (Linux, macOS), limited Windows PTY support

### Comparison with portable-pty

| Feature | portable-pty | expectrl |
|---------|--------------|----------|
| **I/O Model** | Blocking only | Async (smol) + blocking |
| **API Level** | Low-level PTY | High-level automation |
| **Pattern Matching** | Manual | Built-in `expect()` |
| **Tokio Integration** | Manual `spawn_blocking` | Manual `spawn_blocking` |
| **Complexity** | High (need own state machine) | Medium (expect patterns) |

**Decision**: Use `expectrl` for higher-level automation API, accept smol/Tokio runtime bridge.

## Component Design

### 1. GeminiPtySession

**Purpose**: Owns a single long-lived Gemini CLI process via PTY.

**Lifecycle**:
```
Create → Start (lazy) → Multiple Turns → Shutdown
         ↓                    ↓
    Wait for prompt    Send msg → Stream → Detect prompt
                                    ↓
                            Auto-checkpoint (every N turns)
```

**Key Methods**:

```rust
pub struct GeminiPtySession {
    session: expectrl::Session,
    prompt_detector: PromptDetector,
    turn_count: usize,
    last_checkpoint: Option<String>,
}

impl GeminiPtySession {
    // Spawn Gemini CLI in interactive mode via expectrl
    pub fn spawn(model: &str, cwd: Option<&Path>) -> Result<Self, GeminiError>;

    // Send user message (blocking, call from spawn_blocking)
    pub fn send_user_message(&mut self, text: &str) -> Result<(), GeminiError>;

    // Read and stream output until prompt detected (blocking)
    pub fn run_turn_loop(
        &mut self,
        tx: mpsc::Sender<StreamEvent>,
        cancel: CancellationToken,
    ) -> Result<(), GeminiError>;

    // Graceful shutdown (send /quit or Ctrl-D)
    pub fn shutdown(&mut self) -> Result<(), GeminiError>;
}
```

**Prompt Detection Integration**:
- Use existing `PromptDetector` with 9 passing tests
- Feed PTY output to detector
- Detect completion via: explicit prompt markers, idle timeout, response completeness
- Strip prompt markers from final output

**Error Recovery**:
- If CLI crashes: surface error to user
- Option to restart session (fresh CLI process, empty state)
- Checkpoint restoration if available (`/chat resume <id>`)

### 2. GeminiPtyProvider

**Purpose**: Provider interface that manages session pool and async/sync bridging.

**Session Management**:
```rust
pub struct GeminiPtyProvider {
    sessions: Arc<Mutex<HashMap<ConversationId, GeminiPtySession>>>,
}

impl GeminiPtyProvider {
    // Get or create session for conversation
    async fn get_session(&self, conv_id: ConversationId, model: &str)
        -> Result<GeminiPtySession, GeminiError>;

    // Send message (spawns blocking task for turn loop)
    async fn send_message(
        &self,
        conv_id: ConversationId,
        message: String,
    ) -> Result<mpsc::Receiver<StreamEvent>, GeminiError>;

    // Cleanup: shutdown all sessions
    async fn shutdown_all(&self) -> Result<(), GeminiError>;
}
```

**Async/Sync Bridging**:
```rust
// Public API is async
pub async fn send_message(...) -> Result<Receiver<StreamEvent>, Error> {
    let session = self.get_session(conv_id, model).await?;

    // Channel for streaming
    let (tx, rx) = mpsc::channel(32);

    // Run session in blocking task (expectrl is sync/smol-async)
    let handle = tokio::task::spawn_blocking(move || {
        session.send_user_message(&message)?;
        session.run_turn_loop(tx, cancel_token)?;
        Ok(())
    });

    // Return receiver immediately (streaming starts)
    Ok(rx)
}
```

### 3. Prompt Detection (Reuse Existing)

**Status**: Already implemented with 9 passing tests ✅

**Signals Used** (multi-signal heuristic):
1. **High confidence**: Explicit prompt markers (`\n> `, `\ngemini> `)
2. **Medium confidence**: Idle timeout + response looks complete
3. **Low confidence**: Fallback timeout

**Integration**:
```rust
let mut detector = PromptDetector::with_threshold(Duration::from_millis(500));

loop {
    let bytes = session.read_bytes()?; // expectrl read
    let text = strip_ansi_escapes::strip(&bytes);

    detector.update(&text);

    if detector.is_complete(&accumulated) {
        break; // Turn complete
    }
}
```

## Tokio Integration

**Challenge**: `expectrl` uses `async-io` (smol), not compatible with Tokio.

**Solution**: Spawn blocking tasks for all `expectrl` operations.

```rust
// Session lifecycle runs in blocking task
let session_handle = tokio::task::spawn_blocking(move || {
    let mut session = GeminiPtySession::spawn(model, cwd)?;

    // Session loop: receive commands from channel, execute, send responses
    while let Some(cmd) = cmd_rx.blocking_recv() {
        match cmd {
            SessionCmd::SendMessage { text, tx } => {
                session.send_user_message(&text)?;
                session.run_turn_loop(tx, cancel_token)?;
            }
            SessionCmd::Shutdown => {
                session.shutdown()?;
                break;
            }
        }
    }

    Ok(())
});
```

**Performance**: Blocking tasks run on dedicated thread pool, minimal overhead for I/O-bound PTY operations.

## Streaming to TUI

**Flow**:
```
PTY Output → UTF-8 decode → ANSI strip → Prompt detection
    ↓
StreamEvent::Delta → mpsc channel → TUI
    ↓
Prompt detected → StreamEvent::Done → End turn
```

**Safe UTF-8 Handling**:
```rust
let mut utf8_buffer = Vec::new();

loop {
    let bytes = session.read()?;
    utf8_buffer.extend_from_slice(bytes);

    // Try to decode, keep partial sequences
    match String::from_utf8(utf8_buffer.clone()) {
        Ok(text) => {
            utf8_buffer.clear();
            // Process text
        }
        Err(e) => {
            // Keep valid prefix, retain incomplete sequences
            let valid_up_to = e.utf8_error().valid_up_to();
            let text = String::from_utf8_lossy(&utf8_buffer[..valid_up_to]);
            utf8_buffer.drain(..valid_up_to);
            // Process partial text
        }
    }
}
```

## Cancellation

**User cancels response mid-generation**:

```rust
pub fn cancel(&mut self) -> Result<(), GeminiError> {
    // Send Ctrl-C to PTY
    self.session.send_control('c')?; // or write byte 0x03

    // Drain output until prompt returns
    let mut drain_buffer = Vec::new();
    loop {
        let bytes = self.session.read_timeout(Duration::from_millis(100))?;
        drain_buffer.extend(bytes);

        let text = String::from_utf8_lossy(&strip_ansi_escapes::strip(&drain_buffer));
        if text.contains("> ") || text.contains("gemini>") {
            break; // Prompt restored
        }
    }

    // Reset detector for next turn
    self.prompt_detector.reset();

    Ok(())
}
```

**Important**: Session stays alive after cancel, ready for next message.

## Configuration

```toml
[gemini_pty]
# Path to gemini binary
binary_path = "gemini"

# Model to use
model = "gemini-2.5-flash"

# Max response timeout
max_response_time_secs = 120

# Idle threshold for prompt detection
idle_threshold_ms = 500

# Auto-checkpoint interval (turns)
auto_checkpoint_interval = 5

# Session persistence
enable_checkpoints = true
checkpoint_dir = "~/.gemini/checkpoints"
```

## Limitations & Assumptions

### Platform Support
- **Primary**: Linux, macOS (full PTY support)
- **Windows**: Limited (expectrl has basic Windows support, may need testing)
- **Fallback**: If PTY fails on Windows, could fallback to headless (with caveats)

### CLI Availability
- Requires Gemini CLI installed: `npm install -g @google/gemini-cli`
- Requires authentication: `gemini` (follow login flow)
- Health check on startup: `gemini --version`

### Session State
- **Persistent**: CLI owns state, survives across turns
- **Loss scenarios**:
  - CLI crashes (can restart, offer checkpoint restore)
  - TUI crashes (state lost unless checkpoint available)
  - System reboot (state lost)
- **Mitigation**: Auto-checkpoints every N turns, manual `/chat save` support

### Performance
- **Startup latency**: ~500ms to spawn CLI and wait for prompt
- **Turn latency**: <100ms (PTY overhead minimal)
- **Memory**: One CLI process per conversation (~50-100MB per session)

### Comparison with Headless

| Aspect | Headless (-p) | PTY Interactive |
|--------|---------------|-----------------|
| **Multi-turn** | Synthetic (JSON) | Native (CLI state) |
| **Reliability** | Timeouts common | Stable |
| **Features** | Limited | Full (tools, memory, checkpoints) |
| **Complexity** | Low (one-shot) | Medium (session management) |
| **Performance** | Fast (~200ms) | Moderate (~500ms startup) |

**Decision**: Reliability and feature completeness > startup speed.

## Testing Strategy

### Unit Tests
- Session creation and config ✓ (already exists)
- Prompt detection ✓ (9 tests passing)
- Prompt marker removal ✓ (already exists)
- Error recovery scenarios (new)

### Integration Tests (behind `#[ignore]`)

**Test 1: Single-turn conversation**
```rust
#[tokio::test]
#[ignore] // Requires Gemini CLI installed
async fn test_single_turn_pty() {
    let mut session = GeminiPtySession::spawn("gemini-2.5-flash", None).unwrap();

    let (tx, mut rx) = mpsc::channel(32);

    tokio::task::spawn_blocking(move || {
        session.send_user_message("Say exactly: Hello World").unwrap();
        session.run_turn_loop(tx, CancellationToken::new()).unwrap();
    });

    let mut response = String::new();
    while let Some(event) = rx.recv().await {
        if let StreamEvent::Delta(text) = event {
            response.push_str(&text);
        }
    }

    assert!(response.contains("Hello World"));
}
```

**Test 2: Multi-turn conversation with state**
```rust
#[tokio::test]
#[ignore]
async fn test_multi_turn_state() {
    let mut session = GeminiPtySession::spawn("gemini-2.5-flash", None).unwrap();

    // Turn 1: Set name
    send_and_wait(&mut session, "My name is Alice.").await;

    // Turn 2: Recall name
    let response = send_and_wait(&mut session, "What's my name?").await;

    assert!(response.contains("Alice"));
}
```

**Test 3: Cancellation**
```rust
#[tokio::test]
#[ignore]
async fn test_cancellation() {
    let mut session = GeminiPtySession::spawn("gemini-2.5-flash", None).unwrap();

    // Start long response
    let cancel_token = CancellationToken::new();
    let token_clone = cancel_token.clone();

    let handle = tokio::task::spawn_blocking(move || {
        session.send_user_message("Write a long story...").unwrap();
        session.run_turn_loop(tx, token_clone).unwrap();
    });

    // Cancel after 100ms
    tokio::time::sleep(Duration::from_millis(100)).await;
    cancel_token.cancel();

    // Should complete without hanging
    handle.await.unwrap().unwrap();

    // Session should be ready for next message
    let response = send_and_wait(&mut session, "Say: Ready").await;
    assert!(response.contains("Ready"));
}
```

### Debug Binary

```rust
// cargo run --bin gemini_pty_debug
//
// Interactive debug tool for PTY session

fn main() {
    let mut session = GeminiPtySession::spawn("gemini-2.5-flash", None)?;

    println!("Gemini PTY Debug Console");
    println!("Commands: /quit to exit");

    loop {
        print!("> ");
        io::stdout().flush()?;

        let mut input = String::new();
        io::stdin().read_line(&mut input)?;

        if input.trim() == "/quit" {
            break;
        }

        session.send_user_message(input.trim())?;

        let (tx, mut rx) = mpsc::channel(32);

        tokio::task::spawn_blocking(move || {
            session.run_turn_loop(tx, CancellationToken::new())
        });

        while let Some(event) = rx.recv().await {
            match event {
                StreamEvent::Delta(text) => print!("{}", text),
                StreamEvent::Done => println!("\n[DONE]"),
                StreamEvent::Error(e) => eprintln!("[ERROR] {}", e),
                _ => {}
            }
        }
    }

    session.shutdown()?;
}
```

## Migration Path

### Phase 1: Core Implementation (Current)
- [x] Prompt detection (complete)
- [ ] GeminiPtySession with expectrl
- [ ] GeminiPtyProvider with session pool
- [ ] Basic integration tests

### Phase 2: Integration
- [ ] Wire into model_router
- [ ] Update TUI to use PTY provider
- [ ] Health checks and error messages
- [ ] Debug binary

### Phase 3: Optimization
- [ ] Auto-checkpoint implementation
- [ ] Session persistence (/chat save/resume)
- [ ] Memory management (session limits)
- [ ] Performance benchmarking

### Phase 4: Polish
- [ ] Windows testing and fallback
- [ ] Comprehensive error recovery
- [ ] Documentation and examples
- [ ] Remove old portable-pty code

## Open Questions

1. **Session pool limits**: How many concurrent sessions? (Suggest: 5 max, configurable)
2. **Session timeout**: Auto-close idle sessions after N minutes? (Suggest: 30 min)
3. **Checkpoint strategy**: Auto-save on every turn vs periodic? (Suggest: periodic)
4. **Error UX**: Auto-restart vs manual restart? (Suggest: offer restart, don't auto)
5. **Windows support**: Invest in PTY compatibility or fallback? (Suggest: test then decide)

## Appendix: expectrl API Reference

### Key Types

```rust
use expectrl::{Session, Eof, WaitStatus};

// Spawn command in PTY
let mut session = Session::spawn(command)?;

// Send text (with newline)
session.send_line("Hello")?;

// Read with pattern matching
session.expect(Regex::new(r"> "))?; // Wait for prompt

// Read without blocking
let bytes = session.read_available()?;

// Check if process alive
let status = session.status();

// Clean shutdown
session.send_line("exit")?;
session.expect(Eof)?;
```

### Error Handling

```rust
match session.expect(regex!("> ")) {
    Ok(found) => { /* prompt detected */ }
    Err(expectrl::Error::Timeout) => { /* timeout */ }
    Err(expectrl::Error::Eof) => { /* process died */ }
    Err(e) => { /* other error */ }
}
```

## Conclusion

This design provides a robust, feature-complete Gemini CLI integration via PTY that:

1. ✅ Leverages native CLI state management (no synthetic history)
2. ✅ Provides reliable multi-turn conversations
3. ✅ Enables full CLI features (tools, memory, checkpoints)
4. ✅ Reuses proven prompt detection (9 tests passing)
5. ✅ Bridges async runtimes (expectrl/smol → Tokio)
6. ✅ Handles errors gracefully with recovery options

**Next steps**: Implement GeminiPtySession with expectrl, validate with integration tests.
