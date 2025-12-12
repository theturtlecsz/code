# CLI Routing Architecture for Claude & Gemini

**Source**: theturtlecsz/code (fork; see `UPSTREAM-SYNC.md`)
**Purpose**: Multi-provider LLM support via CLI routing with session persistence

---

## Architecture Overview

```
┌─────────────────────────────────────────────────────────────────────┐
│                              TUI Layer                               │
│  codex-rs/tui/src/providers/                                        │
│  ├─ claude_streaming.rs    (streaming interface)                    │
│  ├─ gemini_streaming.rs    (streaming interface)                    │
│  └─ mod.rs                 (ProviderType routing logic)             │
└─────────────────────────────────────────────────────────────────────┘
                                    │
                                    ▼
┌─────────────────────────────────────────────────────────────────────┐
│                           Core CLI Executors                         │
│  codex-rs/core/src/cli_executor/                                    │
│  ├─ mod.rs                 (CliExecutor trait)                      │
│  ├─ claude_pipes.rs        (Claude session management)              │
│  ├─ gemini_pipes.rs        (Gemini session management)              │
│  ├─ context.rs             (Context/history formatting)             │
│  └─ types.rs               (StreamEvent, CliError)                  │
└─────────────────────────────────────────────────────────────────────┘
                                    │
                                    ▼
┌─────────────────────────────────────────────────────────────────────┐
│                        External CLI Tools                            │
│  ├─ claude                 (Anthropic CLI)                          │
│  └─ gemini                 (Google CLI)                             │
│      Both maintain session state via internal files                  │
└─────────────────────────────────────────────────────────────────────┘
```

---

## Key Files

### 1. Provider Routing (`tui/src/providers/mod.rs`)

**Purpose**: Determines which provider handles a model, routes accordingly.

```rust
pub enum ProviderType {
    ChatGPT,  // Native OAuth API
    Claude,   // CLI routing
    Gemini,   // CLI routing
}

impl ProviderType {
    pub fn from_model_name(model: &str) -> Self {
        // Routes "claude-*", "opus", "sonnet", "haiku" → Claude
        // Routes "gemini-*", "flash", "bison-*" → Gemini
        // Everything else → ChatGPT (default)
    }

    pub fn uses_cli_routing(&self) -> bool {
        matches!(self, Self::Claude | Self::Gemini)
    }
}
```

### 2. Claude Session Management (`core/src/cli_executor/claude_pipes.rs`)

**Key Design**: One-shot process per message + session resumption

```rust
// Architecture:
// TUI → ClaudePipesProvider → ClaudePipesSession
//                             ├─ session_id (CLI managed)
//                             └─ Per-message process:
//                                claude --print --output-format stream-json \
//                                       [--resume ID] "message"

pub struct ClaudePipesSession {
    config: ClaudePipesConfig,
    session_id: Option<String>,  // Captured from first response
    cancel_token: CancellationToken,
}
```

**Why this works**:
1. **No long-lived process** - Each message spawns new `claude` process
2. **Session-based continuity** - First message captures `session_id`, subsequent use `--resume <id>`
3. **Structured JSON output** - `--output-format stream-json` gives clean events
4. **Efficient context** - Only sends new message per turn (CLI caches internally)
5. **Deterministic completion** - Process exits after each message

### 3. Gemini Session Management (`core/src/cli_executor/gemini_pipes.rs`)

**Same pattern as Claude**:

```rust
// Architecture:
// TUI → GeminiPipesProvider → GeminiPipesSession
//                             ├─ session_id (CLI managed)
//                             └─ Per-message process:
//                                gemini --model X --output-format stream-json \
//                                       [--resume ID] -p "message"
```

### 4. Streaming Provider (`tui/src/providers/claude_streaming.rs`)

**Key Pattern**: Global provider instance for session persistence

```rust
static CLAUDE_PROVIDER: OnceLock<ClaudePipesProvider> = OnceLock::new();

fn get_claude_provider() -> &'static ClaudePipesProvider {
    CLAUDE_PROVIDER.get_or_init(|| {
        let cwd = std::env::current_dir()...;
        ClaudePipesProvider::with_cwd("", &cwd)
    })
}

impl ClaudeStreamingProvider {
    pub async fn execute_streaming(
        &self,
        messages: &[Message],
        model: &str,
        tx: AppEventSender,
    ) -> ProviderResult<String> {
        // Derive conversation ID from message history
        let conv_id = Self::derive_conversation_id(messages);

        // Send only last message (session handles history)
        let user_message = messages.last()...;

        // Stream response back to TUI
        let rx = provider.send_message(conv_id, &user_message).await?;
        // ... stream events to tx
    }
}
```

### 5. Context Formatting (`core/src/cli_executor/context.rs`)

**For stateless fallback** (when session not available):

```rust
impl CliContextManager {
    pub fn format_history(conversation: &Conversation, current_message: &str) -> String {
        // Formats as:
        // SYSTEM: <system prompt>
        // --- Previous Conversation ---
        // USER (timestamp): <message>
        // ASSISTANT (timestamp): <response>
        // --- End Previous Conversation ---
        // USER (current): <new message>
    }
}
```

---

## Session Persistence Flow

```
First Message:
┌─────┐                 ┌──────────────┐              ┌───────────┐
│ TUI │ ──send_msg()──> │ PipesSession │ ──spawn()──> │ claude/   │
│     │                 │  (no sess_id)│              │ gemini    │
└─────┘                 └──────────────┘              └───────────┘
                               │                            │
                               │ <──────── response ────────┤
                               │     (includes session_id)  │
                               ▼                            ▼
                        Store session_id           CLI stores state

Subsequent Messages:
┌─────┐                 ┌──────────────┐              ┌───────────┐
│ TUI │ ──send_msg()──> │ PipesSession │ ──spawn()──> │ claude    │
│     │                 │ (has sess_id)│   --resume   │ --resume X│
└─────┘                 └──────────────┘              └───────────┘
                                                            │
                        Only new message sent ──────────────┘
                        CLI loads cached context internally
```

---

## Stream Event Types

```rust
pub enum StreamEvent {
    Delta(String),      // Incremental text output
    Metadata(serde_json::Value),  // Token usage, etc.
    Error(String),      // Error message
    Done,               // Stream complete
}
```

---

## JSON Output Parsing

Both CLIs support `--output-format stream-json`:

```rust
fn parse_stream_json_event(line: &str, session_id: &mut Option<String>) -> Vec<StreamEvent> {
    match event_type {
        "system"    => capture session_id
        "assistant" => extract text → StreamEvent::Delta
        "result"    → StreamEvent::Done
    }
}
```

---

## Usage Example

```rust
// Check provider type
let provider_type = ProviderType::from_model_name("claude-sonnet-4.5");
assert!(provider_type.uses_cli_routing());

// Create streaming provider
let provider = ClaudeStreamingProvider::new()?;

// Execute with streaming
let response = provider.execute_streaming(
    &conversation_messages,
    "claude-sonnet-4.5",
    event_sender,
).await?;
```

---

## Key Advantages

1. **No API keys needed** - Uses CLI's existing authentication
2. **Session persistence** - CLI handles context caching internally
3. **O(1) data per turn** - Only send new message, not full history
4. **Streaming support** - Real-time token output to UI
5. **Multi-provider** - Same pattern works for Claude, Gemini, etc.
6. **Fallback handling** - Graceful degradation when CLI unavailable

---

## Files to Copy/Reference

```
codex-rs/
├─ core/src/cli_executor/
│  ├─ mod.rs            # CliExecutor trait definition
│  ├─ types.rs          # StreamEvent, CliError, Conversation
│  ├─ claude_pipes.rs   # Claude session-based integration
│  ├─ gemini_pipes.rs   # Gemini session-based integration
│  └─ context.rs        # History formatting (stateless fallback)
│
└─ tui/src/providers/
   ├─ mod.rs              # ProviderType routing logic
   ├─ claude_streaming.rs # TUI integration for Claude
   └─ gemini_streaming.rs # TUI integration for Gemini
```

---

## Dependencies

```toml
[dependencies]
tokio = { version = "1", features = ["process", "io-util", "sync"] }
tokio-util = "0.7"
serde_json = "1"
tracing = "0.1"
```

---

**Last Updated**: 2025-11-28
**Source Repo**: https://github.com/theturtlecsz/code
