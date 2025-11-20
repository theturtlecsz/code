# CLI Wrapper Implementation - Session Continuation Prompt

**Copy this entire prompt to start the next session**

---

# Context: Multi-Provider CLI Wrapper Implementation (SPEC-KIT-952)

You are continuing work on implementing production-grade CLI wrapper support for Claude and Gemini models in the **codex-rs TUI project** (fork of just-every/code). This allows users to route model requests through native `claude` and `gemini` CLIs instead of using direct API/OAuth integration.

---

## Project Context

**Repository**: theturtlecsz/code (fork of just-every/code)
**Location**: /home/thetu/code
**Current Branch**: main
**Spec ID**: SPEC-KIT-952 (cli-routing-multi-provider)

**Important**: This is a **codex-rs derivative TUI**, NOT Anthropic's Claude Code. Calling the `claude` CLI from our TUI is a normal CLI integration (like calling `git` or `cargo`). There is no circular dependency.

---

## Implementation Status

### ✅ Phase 0: CLI Validation & Discovery - COMPLETE

**Completion Date**: 2025-11-20
**Decision**: ✅ **GO** - Proceed with Option A (Stateless Per-Request)

**Key Findings**:
- Both `claude` (v2.0.47) and `gemini` (v0.16.0) CLIs installed and functional
- Both support `--output-format stream-json` (newline-delimited JSON)
- Both support model selection via `--model` flag
- Claude: ~4-6s latency, returns structured JSON with token usage
- Gemini: ~2-3s latency, has built-in retry for rate limits
- Option A (stateless per-request) chosen for simplicity and reliability

**Documentation**: `docs/SPEC-KIT-952-cli-routing-multi-provider/discovery.md` (complete)

---

### 🔵 Phase 1: Minimal Viable Implementation - IN PROGRESS

**Goal**: Single request/response with Claude CLI (no history, no compression yet)

**Status**: 0% complete

**Current Task**: Create core abstractions

---

## Phase 1 Implementation Checklist

### Step 1: Create Core Module Structure ⏸️ CURRENT STEP

**Location**: `codex-rs/core/src/cli_executor/`

**Files to create**:
```
codex-rs/core/src/cli_executor/
├── mod.rs           # Public API, exports, CliExecutor trait
├── types.rs         # Message, Conversation, StreamEvent, CliError enums
├── claude.rs        # ClaudeCliExecutor implementation
├── context.rs       # CliContextManager (history formatting, compression)
└── stream.rs        # CliStreamHandler (parse stream-json output)
```

**Checklist**:
- [ ] Create directory: `mkdir -p codex-rs/core/src/cli_executor`
- [ ] Create `types.rs` with core data structures (see Technical Reference below)
- [ ] Create `mod.rs` with `CliExecutor` trait definition
- [ ] Update `codex-rs/core/src/lib.rs` to export `cli_executor` module
- [ ] Verify compilation: `cd codex-rs && cargo check -p codex-core`

**Expected Outcome**: Clean compilation, core types available for use

---

### Step 2: Implement ClaudeCliExecutor (Basic) ⏸️ NEXT

**File**: `codex-rs/core/src/cli_executor/claude.rs`

**Requirements**:
1. Spawn `claude --print --output-format stream-json "<prompt>"`
2. Read stdout line-by-line (newline-delimited JSON)
3. Parse JSON and extract assistant response
4. Return single-shot response (no streaming yet)
5. Basic error handling (binary not found, process failure)

**Checklist**:
- [ ] Implement `ClaudeCliExecutor` struct with config
- [ ] Implement `execute()` method (spawn, write stdin, read stdout)
- [ ] Parse `stream-json` format (system init + assistant message)
- [ ] Extract response text from `message.content[0].text`
- [ ] Implement `health_check()` (run `claude --version`)
- [ ] Add error handling for common cases
- [ ] Unit test: mock process output and verify parsing

**Expected Outcome**: Can send single message and receive response

---

### Step 3: Context Manager (History Formatting) ⏸️ PENDING

**File**: `codex-rs/core/src/cli_executor/context.rs`

**Requirements**:
1. Format conversation history into CLI-friendly prompt
2. Embed prior messages with clear delimiters
3. Token estimation (char_count / 4 heuristic)
4. Basic compression (keep first + last N messages)

**Checklist**:
- [ ] Implement `CliContextManager::format_history()`
- [ ] Add timestamp formatting for messages
- [ ] Implement `estimate_tokens()` heuristic
- [ ] Implement `compress_if_needed()` (Level 1: lossless)
- [ ] Unit tests for history formatting edge cases

**Expected Outcome**: Multi-turn conversations work (history preserved)

---

### Step 4: TUI Integration ⏸️ PENDING

**Files**:
- `codex-rs/tui/src/model_router.rs` (modify)
- `codex-rs/tui/src/providers/` (new, optional)

**Requirements**:
1. Register Claude CLI executor in ModelRouter
2. Map model names (claude-opus-4.1, claude-sonnet-4.5) to CLI executor
3. Wire up to existing chat widget
4. Add error display in TUI

**Checklist**:
- [ ] Update `ModelRouter` to support CLI executors
- [ ] Register `claude-opus-4.1` → ClaudeCliExecutor
- [ ] Register `claude-sonnet-4.5` → ClaudeCliExecutor
- [ ] Test: Send message via `/model claude-sonnet-4.5`
- [ ] Verify response appears in chat widget
- [ ] Test error paths (CLI not found, auth failure)

**Expected Outcome**: Can select Claude model in TUI and send messages

---

### Step 5: Manual Testing & Validation ⏸️ PENDING

**Test Scenarios**:
1. **Single message**: "What's 2+2?"
2. **Multi-turn**: "My name is Alice" → "What's my name?"
3. **Long prompt**: 1000+ word message
4. **Error: CLI not found**: Rename claude binary temporarily
5. **Error: Auth failure**: (simulate if possible)

**Checklist**:
- [ ] Test single message end-to-end
- [ ] Test 3-turn conversation (verify history preserved)
- [ ] Test long prompt (no truncation)
- [ ] Test "CLI not found" error message
- [ ] Test "Auth required" error message
- [ ] Measure actual latency (should be 4-6s as per discovery)
- [ ] Document any unexpected behaviors

**Expected Outcome**: All scenarios pass, errors have helpful messages

---

## Technical Reference

### Core Type Definitions (types.rs)

```rust
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use chrono::{DateTime, Utc};
use thiserror::Error;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Role {
    System,
    User,
    Assistant,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Message {
    pub role: Role,
    pub content: String,
    pub timestamp: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone)]
pub struct Conversation {
    pub messages: Vec<Message>,
    pub system_prompt: Option<String>,
    pub model: String,
}

#[derive(Debug, Clone)]
pub enum StreamEvent {
    Delta(String),              // Incremental text
    Metadata(ResponseMetadata), // Token usage, model info
    Done,                       // Response complete
    Error(CliError),            // Error occurred
}

#[derive(Debug, Clone)]
pub struct ResponseMetadata {
    pub model: String,
    pub input_tokens: Option<usize>,
    pub output_tokens: Option<usize>,
}

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

    #[error("CLI process failed with code {code}: {stderr}")]
    ProcessFailed {
        code: i32,
        stderr: String,
    },

    #[error("Timeout after {elapsed:?}")]
    Timeout {
        elapsed: std::time::Duration,
    },

    #[error("Parse error: {details}")]
    ParseError {
        details: String,
    },

    #[error("Internal error: {message}")]
    Internal {
        message: String,
    },
}
```

### CliExecutor Trait (mod.rs)

```rust
use async_trait::async_trait;
use tokio::sync::mpsc;
use crate::cli_executor::types::*;

#[async_trait]
pub trait CliExecutor: Send + Sync {
    /// Execute a request with conversation history
    async fn execute(
        &self,
        conversation: &Conversation,
        user_message: &str,
    ) -> Result<mpsc::Receiver<StreamEvent>, CliError>;

    /// Check if CLI is available and authenticated
    async fn health_check(&self) -> Result<(), CliError>;

    /// Estimate token count for validation
    fn estimate_tokens(&self, conversation: &Conversation) -> usize;
}
```

### Claude Stream-JSON Format

**Example output from `claude --print --output-format stream-json "test"`**:

```json
{"type":"system","subtype":"init","session_id":"uuid","model":"claude-sonnet-4-5-20250929","tools":[...]}
{"type":"assistant","message":{"model":"claude-sonnet-4-5-20250929","id":"msg_...","content":[{"type":"text","text":"Response text here"}],"usage":{"input_tokens":3,"output_tokens":11}}}
```

**Parsing Strategy**:
1. Read stdout line-by-line (each line is complete JSON)
2. Parse each line as `serde_json::Value`
3. Check `json["type"]`:
   - `"system"`: Log metadata, ignore for response
   - `"assistant"`: Extract `message.content[0].text` as response
4. Extract token usage from `message.usage`

### History Formatting Example

```
SYSTEM: You are a helpful coding assistant.

--- Previous Conversation ---
USER (2025-11-20 19:30):
What's the best error handling in Rust?

ASSISTANT (2025-11-20 19:30):
Rust uses Result<T, E> for recoverable errors...

USER (2025-11-20 19:31):
Show me an example.
--- End Previous Conversation ---

USER (current):
Make it work with custom error types.
```

---

## Common Commands

**Build & Test**:
```bash
# Build TUI
cd /home/thetu/code
~/code/build-fast.sh

# Check compilation
cd codex-rs
cargo check -p codex-core

# Run tests
cargo test -p codex-core cli_executor

# Format & lint
cargo fmt --all
cargo clippy -- -D warnings
```

**Test CLIs manually**:
```bash
# Claude
echo "test" | claude --print --output-format stream-json

# Gemini
gemini "test"

# Check versions
claude --version
gemini --version
```

---

## Decision Log

| Decision | Rationale | Date |
|----------|-----------|------|
| **Option A (Stateless Per-Request)** | Simple, reliable, no persistent state. Can optimize later. | 2025-11-20 |
| **Use stream-json format** | Structured output with token usage, easier to parse | 2025-11-20 |
| **History via prompt embedding** | No native session support in CLIs, embed manually | 2025-11-20 |
| **char/4 token estimation** | Good enough for MVP, can add tiktoken later | 2025-11-20 |
| **Single-shot first, streaming later** | Get basic flow working before optimizing latency | 2025-11-20 |

---

## Known Issues & Gotchas

1. **Claude CLI latency**: 4-6s startup time
   - **Mitigation**: Show "thinking" indicator immediately, optimistic UI updates

2. **Gemini rate limits**: Hit 429 during testing
   - **Mitigation**: CLI has built-in retry, works transparently

3. **No true token-level streaming**: `stream-json` returns full response in one block
   - **Mitigation**: Still better than blocking, can improve in Phase 2

4. **History re-sent each request**: No native session IDs
   - **Mitigation**: Context compression, efficient formatting

---

## Files Created This Session

```
docs/SPEC-KIT-952-cli-routing-multi-provider/
├── discovery.md           # Complete CLI capabilities documentation
└── CONTINUATION-PROMPT.md # This file (session bootstrap)
```

**Test artifacts** (temporary, can delete):
- `/tmp/claude-version.txt`
- `/tmp/claude-help.txt`
- `/tmp/claude-test-basic.txt`
- `/tmp/claude-stream-test.txt`
- `/tmp/gemini-help.txt`
- `/tmp/gemini-version.txt`
- `/tmp/gemini-test-basic.txt`

---

## Architecture Diagram

```
┌─────────────────────────────────────────┐
│           codex-rs TUI                   │
│  (User selects /model claude-sonnet-4.5) │
└──────────────────┬──────────────────────┘
                   │
                   ▼
┌─────────────────────────────────────────┐
│          ModelRouter                     │
│  Routes to appropriate executor          │
└──────────────────┬──────────────────────┘
                   │
                   ▼
┌─────────────────────────────────────────┐
│       ClaudeCliExecutor                  │
│  1. Format history (CliContextManager)   │
│  2. Spawn: claude --print --output-format stream-json │
│  3. Write prompt to stdin                │
│  4. Parse stdout (CliStreamHandler)      │
│  5. Extract response + token usage       │
└──────────────────┬──────────────────────┘
                   │
                   ▼
┌─────────────────────────────────────────┐
│     External claude CLI Process          │
│  (Separate binary, no circular dep)      │
└──────────────────┬──────────────────────┘
                   │
                   ▼
┌─────────────────────────────────────────┐
│         Claude API                       │
│  (via claude CLI's session)              │
└─────────────────────────────────────────┘
```

---

## Session Workflow

**At start of each session**:

1. **Review status**: Read this prompt, check current phase/step
2. **Load context**: Read `discovery.md` if needed
3. **Check last completed**: Review checklist, identify next task
4. **Execute work**: Complete next checklist item(s)
5. **Update status**: Mark items complete, update "Current Task" section
6. **Document findings**: Add to discovery.md or create new docs as needed
7. **End of session**: Update this prompt with new status, provide next action

---

## 🎯 NEXT ACTION (Start Here)

**Current Phase**: Phase 1, Step 1
**Current Task**: Create core module structure

**Commands to run**:
```bash
# 1. Create directory
mkdir -p /home/thetu/code/codex-rs/core/src/cli_executor

# 2. Start with types.rs (core data structures)
# Create file with Message, Conversation, StreamEvent, CliError types

# 3. Then create mod.rs (trait definition)
# Define CliExecutor trait with execute(), health_check(), estimate_tokens()

# 4. Update core/src/lib.rs
# Add: pub mod cli_executor;

# 5. Verify compilation
cd /home/thetu/code/codex-rs
cargo check -p codex-core
```

**Expected output**: Clean compilation, no errors

**After completing Step 1**: Update checklist, move to Step 2 (Implement ClaudeCliExecutor)

---

## Questions to Ask Before Starting

1. **Do you want to implement just Claude first, or both Claude + Gemini in parallel?**
   - Recommendation: Claude first (proven in tests), Gemini second

2. **Should we implement streaming in Phase 1, or defer to Phase 2?**
   - Recommendation: Single-shot first (simpler), streaming Phase 2

3. **Any specific error cases you want handled beyond the standard set?**
   - Current plan: Binary not found, auth failure, process failure, timeout

---

## Success Criteria for Phase 1

- ✅ Can send single message to Claude CLI and receive response
- ✅ Error messages are clear and actionable (not raw CLI output)
- ✅ Multi-turn conversations work (history preserved)
- ✅ Latency is acceptable (<6s with UX masking)
- ✅ Clean compilation, no warnings
- ✅ Basic unit tests pass

**When Phase 1 is complete**: Move to Phase 2 (Streaming Support) or Phase 3 (Gemini implementation) based on priority.

---

**Remember**: This is a **normal CLI integration** (codex-rs TUI → claude CLI). No circular dependency. The architecture is sound. Focus on clean implementation following the checklist.

**Let's build! 🚀**
