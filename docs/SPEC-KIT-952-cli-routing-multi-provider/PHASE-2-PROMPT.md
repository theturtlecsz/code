# CLI Routing Implementation - Phase 2 Session Prompt

**Copy this entire prompt to start Phase 2 session**

---

# Context: Multi-Provider CLI Routing - Phase 2 (SPEC-KIT-952)

You are continuing work on implementing **production-grade CLI routing** for Claude and Gemini models in the **codex-rs TUI project** (fork; see `UPSTREAM-SYNC.md`). CLI routing is the **primary and intended method** for these providers.

**Phase 1 Status**: ✅ **COMPLETE** - Streaming CLI executors implemented
**Phase 2 Goal**: Wire into router, test end-to-end, verify all 6 models work in production

---

## Project Context

**Repository**: theturtlecsz/code (fork; see `UPSTREAM-SYNC.md`)
**Location**: /home/thetu/code
**Current Branch**: main
**Spec ID**: SPEC-KIT-952 (cli-routing-multi-provider)

**Important**: This is a **codex-rs derivative TUI**, NOT Anthropic's Claude Code. Calling the `claude` CLI from our TUI is a normal CLI integration.

**Memory System**: **local-memory MCP ONLY** (see project's `MEMORY-POLICY.md`)

---

## Architecture: Multi-Provider Routing

### Production Routing Strategy

| Provider | Models | Auth Method | Routing Implementation |
|----------|--------|-------------|----------------------|
| **ChatGPT** | gpt-5, gpt-5.1-*, gpt-5-codex | Native OAuth | Existing codex-core flow |
| **Claude** | claude-opus-4.1, claude-sonnet-4.5, claude-haiku-4.5 | **CLI routing** | **SPEC-KIT-952 (this implementation)** |
| **Gemini** | gemini-3-pro, gemini-2.5-pro, gemini-2.5-flash | **CLI routing** | **SPEC-KIT-952 (this implementation)** |

### Why CLI Routing for Claude/Gemini?

**Advantages**:
1. **No API key management** - Users just authenticate CLIs once
2. **Simpler setup** - `claude` + `gemini` commands handle auth
3. **Consistent UX** - Same authentication flow as standalone CLI usage
4. **Session management** - CLIs handle token refresh, rate limits
5. **Official support** - Using vendor-provided tools

**Trade-offs accepted**:
- ~4-6s startup latency (Claude) - mitigated with "thinking" indicator
- Re-sends history each request - acceptable with context compression
- Depends on external binaries - validated in Phase 0

---

## What We've Completed

### ✅ Phase 0: CLI Validation & Discovery - COMPLETE
- Both `claude` (v2.0.47) and `gemini` (v0.16.0) CLIs validated
- `--output-format stream-json` tested and working
- Architecture decision: Option A (Stateless Per-Request)
- Documentation: `docs/SPEC-KIT-952-cli-routing-multi-provider/discovery.md`

### ✅ Phase 1: Minimal Viable Implementation - COMPLETE
- Core streaming CLI executor implemented (`codex-rs/core/src/cli_executor/`)
- Both Claude and Gemini executors with streaming support
- TUI streaming providers created (`codex-rs/tui/src/providers/*_streaming.rs`)
- All 6 models supported (3 Claude + 3 Gemini)
- Tests: 12/12 passing, clean compilation
- Documentation: `docs/SPEC-KIT-952-cli-routing-multi-provider/PHASE-1-COMPLETE.md`

---

## Current Implementation Status

### What Exists ✅

**Core layer** (`codex-rs/core/src/cli_executor/`):
- ✅ `CliExecutor` trait with async streaming
- ✅ `ClaudeCliExecutor` - spawns `claude` CLI, parses stream-json
- ✅ `GeminiCliExecutor` - spawns `gemini` CLI, parses stream-json
- ✅ `CliContextManager` - history formatting, token estimation, compression
- ✅ Stream parsers for both providers
- ✅ Comprehensive error handling (BinaryNotFound, NotAuthenticated, etc.)

**TUI layer** (`codex-rs/tui/src/providers/`):
- ✅ `ClaudeStreamingProvider` - bridges core executor to AppEventSender
- ✅ `GeminiStreamingProvider` - bridges core executor to AppEventSender
- ✅ Type conversion: `context_manager::Message` → `cli_executor::Message`
- ✅ Real-time streaming to UI via delta events

**Tests**:
- ✅ 12/12 core tests passing
- ✅ Health checks confirm both CLIs available
- ✅ Clean compilation (codex-core + codex-tui)

### What's Missing ❌ (Phase 2 Tasks)

**Integration**:
- ❌ Streaming providers not wired into `model_router.rs`
- ❌ Router still uses old/placeholder routing logic
- ❌ Chat widget not calling CLI streaming providers

**Testing**:
- ❌ No end-to-end testing (haven't sent actual messages)
- ❌ Multi-turn conversations not tested
- ❌ Error scenarios not verified
- ❌ Performance not measured

**Documentation**:
- ❌ User-facing docs for CLI setup
- ❌ Troubleshooting guide
- ❌ SPEC.md not marked complete

---

## Data Flow: CLI Routing (Production Path)

```
User types message in TUI
    ↓
ChatWidget captures input
    ↓
ModelRouter.route_message(model, messages)
    ↓
Check model type:
  - gpt-* → Use existing OAuth flow (codex-core)
  - claude-* → ClaudeStreamingProvider (SPEC-KIT-952)
  - gemini-* → GeminiStreamingProvider (SPEC-KIT-952)
    ↓
ClaudeStreamingProvider.execute_streaming(messages, model, tx)
    ↓
Convert context_manager::Message → cli_executor::Message
    ↓
ClaudeCliExecutor.execute(&conversation, user_message)
    ↓
Spawn: `claude --print --output-format stream-json --model claude-sonnet-4.5`
    ↓
Write formatted history + current message to stdin
    ↓
Parse stdout (newline-delimited JSON):
  - {"type":"system",...} → Log metadata
  - {"type":"assistant","message":{...}} → Extract text
    ↓
Stream events to UI:
  - StreamEvent::Delta(text) → tx.send_native_stream_delta(text)
  - StreamEvent::Metadata(usage) → tx.send_native_stream_complete(...)
    ↓
User sees response appear in real-time ✨
```

---

## Phase 2 Implementation Checklist

### Step 1: Wire Streaming Providers into Router ⏸️ START HERE

**Goal**: Make CLI routing the active production path for Claude/Gemini

**File to modify**: `codex-rs/tui/src/model_router.rs`

**Current state** (lines 221-255):
```rust
pub async fn execute_with_native_streaming(...) -> Result<String, String> {
    // Uses AnthropicClient, GeminiClient (direct API)
    // This is NOT what we want for production
}
```

**What we need**: Replace with CLI routing implementation

**Implementation**:

```rust
/// Execute prompt via CLI routing with streaming (SPEC-KIT-952)
///
/// PRIMARY routing method for Claude and Gemini models.
/// Uses external CLI processes with streaming support.
pub async fn execute_with_cli_streaming(
    model: &str,
    messages: &[Message],
    tx: AppEventSender,
) -> Result<String, String> {
    let provider_type = ProviderType::from_model_name(model);

    match provider_type {
        ProviderType::ChatGPT => {
            // ChatGPT uses native OAuth flow (existing)
            Err("ChatGPT should use native codex-core flow".to_string())
        }
        ProviderType::Claude => {
            // CLI routing for Claude (SPEC-KIT-952)
            use crate::providers::claude_streaming::ClaudeStreamingProvider;

            let provider = ClaudeStreamingProvider::new()
                .map_err(|e| format!("Failed to create Claude provider: {}", e))?;

            provider.execute_streaming(messages, model, tx).await
                .map_err(|e| format!("{}", e))
        }
        ProviderType::Gemini => {
            // CLI routing for Gemini (SPEC-KIT-952)
            use crate::providers::gemini_streaming::GeminiStreamingProvider;

            let provider = GeminiStreamingProvider::new()
                .map_err(|e| format!("Failed to create Gemini provider: {}", e))?;

            provider.execute_streaming(messages, model, tx).await
                .map_err(|e| format!("{}", e))
        }
    }
}
```

**Where to call it** (likely in `chatwidget/mod.rs`):

Find the code that currently calls `execute_with_native_streaming()` for Claude/Gemini and replace with:
```rust
// OLD (native API - not used):
// let response = execute_with_native_streaming(model, messages, codex_home, tx).await?;

// NEW (CLI routing - production):
let response = execute_with_cli_streaming(model, messages, tx).await?;
```

**Checklist**:
- [ ] Add `execute_with_cli_streaming()` to `model_router.rs`
- [ ] Import `ClaudeStreamingProvider` and `GeminiStreamingProvider`
- [ ] Find where ChatWidget calls model execution for Claude/Gemini
- [ ] Replace with `execute_with_cli_streaming()`
- [ ] Remove or deprecate `execute_with_native_streaming()` for Claude/Gemini
- [ ] Verify compilation: `cargo check -p codex-tui`
- [ ] Search codebase for any other calls to native API for Claude/Gemini

**Expected outcome**:
- Code compiles cleanly
- Claude/Gemini models route through CLI streaming providers
- ChatGPT still uses native OAuth (unchanged)

---

### Step 2: Manual End-to-End Testing ⏸️ NEXT

**Goal**: Verify CLI routing works for all 6 models in production

**Build and run TUI**:
```bash
cd /home/thetu/code
~/code/build-fast.sh run
```

**Test Matrix** (all must pass):

| Test | Model | Input | Expected Output | Status |
|------|-------|-------|-----------------|--------|
| Single message | claude-sonnet-4.5 | "What's 2+2?" | "4" with streaming | ⏸️ |
| Multi-turn | claude-opus-4.1 | "My name is Alice" → "What's my name?" | "Alice" | ⏸️ |
| Long prompt | gemini-3-pro | 1000+ word code snippet | Full response, no truncation | ⏸️ |
| Streaming | gemini-2.5-flash | "Count to 10" | See numbers appear one by one | ⏸️ |
| Token usage | claude-haiku-4.5 | Any message | Token counts displayed | ⏸️ |
| History | gemini-2.5-pro | 3+ turn conversation | Context preserved | ⏸️ |

**Detailed test scenarios**:

1. **Single message - Claude Sonnet**:
   ```
   /model claude-sonnet-4.5
   > What's 2+2?

   Expected:
   - "Thinking..." indicator appears immediately
   - Response starts streaming within 4-6s
   - Text appears incrementally (Delta events)
   - Final response: "4" or "2+2 equals 4"
   - Token usage shown: ~10 input, ~5-10 output
   ```

2. **Multi-turn - Claude Opus**:
   ```
   /model claude-opus-4.1
   > My name is Alice
   [Response acknowledges]
   > What's my name?

   Expected:
   - Second response mentions "Alice"
   - History from first message preserved
   - Context manager formatting visible in logs
   ```

3. **Long prompt - Gemini 3 Pro**:
   ```
   /model gemini-3-pro
   > [Paste 1000+ word code file]

   Expected:
   - Full code accepted (no truncation)
   - Response addresses entire input
   - Token estimation accurate (~250-300 tokens)
   - Context compression doesn't trigger (under limit)
   ```

4. **All 6 models smoke test**:
   ```
   For each model: claude-opus-4.1, claude-sonnet-4.5, claude-haiku-4.5,
                   gemini-3-pro, gemini-2.5-pro, gemini-2.5-flash

   /model [model-name]
   > Say hello and tell me your model name

   Expected:
   - Each responds successfully
   - Model name mentioned in response
   - No errors or crashes
   ```

**Checklist**:
- [ ] Single message test passes (Claude Sonnet)
- [ ] Multi-turn conversation works (Claude Opus)
- [ ] Long prompt handled (Gemini 3 Pro)
- [ ] Streaming is visible (Gemini 2.5 Flash)
- [ ] Token usage displayed (Claude Haiku)
- [ ] History preserved (Gemini 2.5 Pro)
- [ ] All 6 models smoke test passes
- [ ] Document any issues or unexpected behaviors

---

### Step 3: Error Scenario Testing ⏸️ PENDING

**Goal**: Verify error handling is production-ready

**Test 1: CLI not found**:
```bash
# Temporarily hide claude CLI
sudo mv $(which claude) $(which claude).bak

# In TUI:
/model claude-sonnet-4.5
> test message

# Expected error message:
"Claude CLI not found. Install from https://claude.ai/download"

# Restore:
sudo mv $(which claude).bak $(which claude)
```

**Test 2: Authentication failure**:
```bash
# If possible, clear credentials:
rm -rf ~/.claude/session  # or wherever credentials are stored

# In TUI:
/model claude-sonnet-4.5
> test message

# Expected error message:
"Claude CLI not authenticated. Run: claude"

# Re-authenticate:
claude
```

**Test 3: Timeout**:
```rust
// Temporarily modify ClaudeCliConfig in code:
ClaudeCliConfig {
    timeout_secs: 2,  // Very low timeout
    ..Default::default()
}

// Rebuild and test
/model claude-sonnet-4.5
> [Complex prompt that takes >2s]

// Expected error:
"Request timed out after 2s"

// Restore: Set timeout back to 120
```

**Test 4: Rate limit (Gemini)**:
```bash
# Send many requests rapidly
for i in {1..10}; do
    echo "Message $i"
done

# Expected:
- CLI auto-retries transparently
- Eventually succeeds
- OR shows clear rate limit message with retry time
```

**Test 5: Malformed response**:
```bash
# Simulate by modifying stream parser to fail
# Expected: Clear parse error message, not crash
```

**Checklist**:
- [ ] CLI not found → user-friendly message (Claude)
- [ ] CLI not found → user-friendly message (Gemini)
- [ ] Auth failure → clear instructions
- [ ] Timeout → informative error with duration
- [ ] Rate limit → handled gracefully (Gemini)
- [ ] Parse error → doesn't crash TUI
- [ ] All error messages are actionable (tell user what to do)

---

### Step 4: Performance Measurement ⏸️ PENDING

**Goal**: Validate latency meets expectations from discovery phase

**Metrics to measure**:

| Metric | Target | Claude | Gemini | Status |
|--------|--------|--------|--------|--------|
| Cold start | <6s | ⏸️ | ⏸️ | ⏸️ |
| First token | <1s after start | ⏸️ | ⏸️ | ⏸️ |
| Streaming rate | >50 tok/s | ⏸️ | ⏸️ | ⏸️ |
| Multi-turn overhead | <1s vs cold | ⏸️ | ⏸️ | ⏸️ |

**How to measure**:

1. **Manual timing**:
   ```
   - Start stopwatch when pressing Enter
   - Stop when first word appears
   - Record in table above
   ```

2. **Add logging** (optional):
   ```rust
   let start = std::time::Instant::now();
   // ... execute ...
   tracing::info!("Request completed in {:?}", start.elapsed());
   ```

3. **Use TUI timing** (if available):
   - Check if TUI displays request duration
   - Compare to discovery phase measurements

**Expected results** (from Phase 0 discovery):
- Claude: ~4-6s cold start
- Gemini: ~2-3s cold start
- Both: <1s first token after process starts streaming

**Checklist**:
- [ ] Measure Claude cold start (10 samples, average)
- [ ] Measure Gemini cold start (10 samples, average)
- [ ] Measure first token time
- [ ] Subjective: Does streaming feel responsive?
- [ ] Compare to discovery phase baselines
- [ ] Document actual measured latencies
- [ ] Identify any performance regressions

---

### Step 5: Documentation & Completion ⏸️ PENDING

**Goal**: Document usage and mark SPEC complete

**File 1: Update `CLAUDE.md`** (project instructions):

Add section after multi-provider CLI setup:

```markdown
## Using Claude and Gemini Models (CLI Routing)

The TUI routes Claude and Gemini models through their native CLIs (SPEC-KIT-952).

### Setup

**Claude models** (claude-opus-4.1, claude-sonnet-4.5, claude-haiku-4.5):
1. Install: https://claude.ai/download
2. Authenticate: `claude` (follow prompts)
3. Verify: `claude --version` (should show v2.0.47+)

**Gemini models** (gemini-3-pro, gemini-2.5-pro, gemini-2.5-flash):
1. Install: `npm install -g @google/gemini-cli`
2. Authenticate: `gemini` (follow OAuth)
3. Verify: `gemini --version` (should show v0.16.0+)

### Usage

Select model via `/model` command:
```bash
/model claude-sonnet-4.5
/model gemini-3-pro
```

Models automatically route through CLI with streaming support.

### Troubleshooting

**Error: "Claude CLI not found"**
- Install from https://claude.ai/download
- Ensure in PATH: `which claude`

**Error: "CLI not authenticated"**
- Run authentication: `claude` or `gemini`
- Follow prompts to complete

**Performance: Slow startup (4-6s)**
- Expected behavior (CLI initialization)
- Subsequent messages similar (stateless per-request)
- "Thinking" indicator shows progress

### Technical Details

- **Latency**: 4-6s (Claude), 2-3s (Gemini) - includes CLI startup
- **Context**: History re-sent each request (auto-compressed >180K tokens)
- **Streaming**: Real-time delta updates via `--output-format stream-json`
- **Error handling**: User-friendly messages with action items

See: `docs/SPEC-KIT-952-cli-routing-multi-provider/` for full details
```

**File 2: Create `docs/SPEC-KIT-952-cli-routing-multi-provider/README.md`**:

```markdown
# CLI Routing for Multi-Provider Support (SPEC-KIT-952)

Complete implementation guide for Claude and Gemini CLI routing.

## Summary

Implements production-grade streaming via external CLI processes for:
- 3 Claude models (opus-4.1, sonnet-4.5, haiku-4.5)
- 3 Gemini models (3-pro, 2.5-pro, 2.5-flash)

## Architecture

[Paste architecture diagram from PHASE-1-COMPLETE.md]

## Performance

[Paste measured latencies from Step 4]

## Comparison: CLI Routing vs Native API

| Aspect | CLI Routing | Native API |
|--------|-------------|------------|
| Setup | Install CLI, authenticate once | API keys, OAuth flow |
| Latency | 4-6s (CLI startup) | 1-2s (direct API) |
| Maintenance | Vendor updates CLI | Update API client code |
| Session mgmt | CLI handles | Manual token refresh |
| Used for | Claude, Gemini | ChatGPT |

## Files

[List all files created in Phase 1]

## Testing

[Link to test results from Steps 2-4]
```

**File 3: Update `SPEC.md`**:

```markdown
| SPEC-KIT-952 | cli-routing-multi-provider | Done | 2025-11-20 | CLI routing with streaming for Claude/Gemini (6 models) |
```

**Checklist**:
- [ ] Update CLAUDE.md with CLI routing section
- [ ] Create comprehensive README.md
- [ ] Update SPEC.md status to "Done"
- [ ] Add troubleshooting section
- [ ] Document all 6 models with examples
- [ ] Link to Phase 1/2 completion docs
- [ ] Add performance characteristics table

---

## Success Criteria for Phase 2

Phase 2 is **COMPLETE** when:

- ✅ All 6 models route through CLI streaming (not native API)
- ✅ End-to-end test passes for each model
- ✅ Multi-turn conversations preserve history
- ✅ Error scenarios show user-friendly messages
- ✅ Performance matches discovery phase (<6s cold start)
- ✅ Documentation updated (CLAUDE.md, README.md)
- ✅ SPEC-KIT-952 marked "Done" in SPEC.md

**When all checked**: SPEC-KIT-952 is production-ready ✨

---

## Common Commands

**Build & Run**:
```bash
# Build and run TUI
cd /home/thetu/code
~/code/build-fast.sh run

# Build only
~/code/build-fast.sh
```

**Verify CLIs**:
```bash
# Check installation
which claude    # Should show path
which gemini    # Should show path

# Check versions
claude --version    # Should show v2.0.47
gemini --version    # Should show v0.16.0

# Test manually
echo "test" | claude --print --output-format stream-json --model claude-sonnet-4.5
gemini --model gemini-2.5-flash "test"
```

**Test code**:
```bash
cd codex-rs

# Test core
cargo test -p codex-core --lib cli_executor

# Test providers
cargo test -p codex-tui --lib providers::claude_streaming -- --nocapture
cargo test -p codex-tui --lib providers::gemini_streaming -- --nocapture

# Check compilation
cargo check -p codex-core
cargo check -p codex-tui
```

---

## Files to Reference

**Implementation**:
- `codex-rs/core/src/cli_executor/` - Core executors
- `codex-rs/tui/src/providers/*_streaming.rs` - TUI providers
- `codex-rs/tui/src/model_router.rs` - **Routing logic to modify**
- `codex-rs/tui/src/chatwidget/mod.rs` - **Chat widget to update**

**Documentation**:
- `docs/SPEC-KIT-952-cli-routing-multi-provider/PHASE-1-COMPLETE.md` - What we built
- `docs/SPEC-KIT-952-cli-routing-multi-provider/discovery.md` - CLI validation
- `CLAUDE.md` - Project instructions (to update)
- `SPEC.md` - Task tracker (to mark complete)

**Configuration**:
- `codex-rs/common/src/model_presets.rs` - Model definitions (6 models)
- `.env` - Environment (if needed for config)

---

## 🎯 NEXT ACTION (Start Here)

**Current Phase**: Phase 2, Step 1
**Current Task**: Wire CLI streaming into router

**First commands**:
```bash
# 1. Read current router implementation
cat /home/thetu/code/codex-rs/tui/src/model_router.rs

# 2. Find where ChatWidget calls model execution
grep -r "execute_with" codex-rs/tui/src/chatwidget/

# 3. Start implementing execute_with_cli_streaming()
```

**First implementation steps**:
1. Add `execute_with_cli_streaming()` function to `model_router.rs`
2. Import streaming providers
3. Find ChatWidget call site for Claude/Gemini
4. Replace with CLI streaming call
5. Build: `cargo check -p codex-tui`
6. Fix any compilation errors

**After Step 1 complete**: Move to Step 2 (Manual testing with real TUI)

---

**Remember**:
- CLI routing is **PRIMARY** for Claude/Gemini (not fallback)
- ChatGPT keeps using native OAuth (unchanged)
- All 6 models must work before Phase 2 is complete

**Let's ship production CLI routing! 🚀**
