# Gemini CLI Pipes Integration - Status & Next Steps

**Date**: 2025-11-21 (Updated after session)
**Status**: Core implementation complete ✅ | Multi-turn debugging needed ⚠️
**Priority**: High - Multi-turn test refinement
**Progress**: 90% - Single-turn proven working

---

## Executive Summary

Implemented a long-lived pipes-based Gemini CLI integration to enable true multi-turn conversations without PTY. **Core implementation complete** but integration test reveals a **timeout issue** (120s) where stream never completes.

### Why Pipes Instead of PTY?

**PTY Approach Failed** (attempted earlier):
- Gemini CLI detects TTY and launches **full-screen TUI** with spinners, logos, heavy ANSI
- No stable prompt string to pattern-match on
- Fundamentally incompatible with simple `expect()` automation

**Pipes Approach** (current):
- With stdin/stdout pipes (no TTY), Gemini CLI uses **simple line-oriented mode**
- Multi-turn chat works (verified manually)
- CLI maintains conversation state natively
- **Issue**: Stream completion detection not working yet

---

## Implementation Status

### ✅ Completed Components

1. **`GeminiPipesSession`** (`core/src/cli_executor/gemini_pipes.rs`)
   - Async child process management via `tokio::process::Command`
   - Piped stdin/stdout/stderr (no PTY)
   - Methods: `spawn()`, `send_user_message()`, `stream_turn()`, `shutdown()`
   - Lines: 1-372

2. **`GeminiPipesProvider`** (same file)
   - Session pool management (HashMap<ConversationId, Session>)
   - Async API for multi-turn conversations
   - Auto-creates sessions on first message
   - Handles process death and recreation
   - Lines: 383-591

3. **Debug Binary** (`core/src/bin/gemini_pipes_debug.rs`)
   - Interactive REPL for manual testing
   - Commands: message, /quit, /stats, /model
   - Build: `cargo build --bin gemini_pipes_debug`
   - Run: `cargo run --bin gemini_pipes_debug`

4. **Module Integration**
   - Exported from `core/src/cli_executor/mod.rs`
   - Available as: `codex_core::cli_executor::{GeminiPipesSession, GeminiPipesProvider}`

### ❌ Current Issue: Test Timeout

**Symptom**:
```bash
cargo test -p codex-core --lib cli_executor::gemini_pipes::tests::test_single_turn_pipes -- --ignored --nocapture
# Result: FAILED after 120.09s (timeout)
# Panic: "Stream should succeed"
```

**What's happening**:
- Process spawns successfully
- Message sent to stdin
- But `stream_turn()` never completes
- Times out after 120s (max_response_time)

**Likely causes**:
1. Gemini CLI might not output anything when stdin is piped (authentication issue?)
2. Output format incompatible with line-by-line reading
3. Prompt detector never triggers completion
4. Process might be waiting for something we're not providing

---

## Files Created/Modified

### New Files
```
core/src/cli_executor/gemini_pipes.rs         (591 lines)
core/src/bin/gemini_pipes_debug.rs            (145 lines)
docs/gemini-cli-pipes-status.md               (this file)
```

### Modified Files
```
core/src/cli_executor/mod.rs                  (+2 lines: module export)
```

### Old Files (Not Deleted Yet)
```
core/src/cli_executor/gemini_pty.rs           (failed PTY attempt, needs cleanup)
docs/gemini-pty-design.md                     (PTY design doc, now obsolete)
```

---

## Next Steps for Debugging

### 1. Manual Test with Debug Binary

**First priority**: Use the debug binary to see actual behavior:

```bash
# Build and run
cargo run --bin gemini_pipes_debug

# Try a simple message
> Say: test

# Observe:
# - Does it hang?
# - Any output on stdout?
# - Check stderr logs (set RUST_LOG=debug)
```

**Expected outcomes**:
- If it hangs: Same issue as test, need to investigate why
- If it works: Test harness issue, not core logic
- If authentication error: Need to handle differently

### 2. Check Gemini CLI Behavior

**Verify CLI behavior with pipes**:

```bash
# Test 1: Does CLI stay alive with piped stdin?
echo "Say: Hello" | gemini --model gemini-2.5-flash

# Test 2: Can we send multiple messages?
(echo "My name is Alice"; sleep 1; echo "What's my name?") | gemini --model gemini-2.5-flash

# Test 3: What does stderr show?
echo "Say: Hello" | gemini --model gemini-2.5-flash 2>&1
```

### 3. Add Verbose Logging

**Enhance debugging in `stream_turn()`**:

```rust
// In gemini_pipes.rs, stream_turn() method
// Add detailed tracing:

tracing::debug!("Waiting for stdout line...");
match tokio::time::timeout(Duration::from_millis(100), reader.read_line(&mut line)).await {
    Ok(Ok(0)) => {
        tracing::error!("EOF on stdout");
        // ...
    }
    Ok(Ok(n)) => {
        tracing::debug!("Read {} bytes: {:?}", n, line);
        // ...
    }
    Err(_) => {
        tracing::trace!("Read timeout (no data)");
        // ...
    }
}
```

### 4. Alternative: Use Buffered Reading

**Current approach** reads line-by-line with `BufReader::read_line()`.
**Alternative**: Read raw bytes in chunks, then parse:

```rust
let mut buffer = [0u8; 4096];
match stdout.read(&mut buffer).await {
    Ok(0) => { /* EOF */ }
    Ok(n) => {
        let chunk = String::from_utf8_lossy(&buffer[..n]);
        // Process chunk
    }
    Err(e) => { /* Error */ }
}
```

This might work better if Gemini doesn't output newlines regularly.

---

## Integration Roadmap (After Debug Fix)

Once stream completion works:

### Phase 1: TUI Integration (High Priority)

1. **Update `tui/src/providers/gemini_streaming.rs`**:
   - Replace current implementation with `GeminiPipesProvider`
   - Handle conversation ID derivation
   - Map StreamEvents to TUI events

2. **Update `tui/src/model_router.rs`**:
   - Register Gemini pipes provider for Gemini models
   - Add health check (CLI availability)
   - Set as default for `gemini-*` models

3. **Configuration**:
   - Add config options for pipes vs. headless mode
   - Working directory for GEMINI.md

### Phase 2: Testing (Medium Priority)

1. **Fix integration tests**:
   - `test_single_turn_pipes` ✅ (after debug)
   - `test_multi_turn_state` (verify conversation memory)

2. **Add provider tests**:
   - Session pool management
   - Concurrent conversations
   - Process death recovery

3. **Manual testing**:
   - End-to-end TUI flows
   - Different models
   - Long conversations

### Phase 3: Documentation (Low Priority)

1. **Design document**: `docs/gemini-cli-pipes-design.md`
   - Architecture overview
   - Why not PTY
   - Prompt detection approach
   - Known limitations

2. **Update CLAUDE.md**:
   - Note pipes as primary Gemini integration
   - Mark PTY approach as obsolete

3. **Cleanup**:
   - Remove or archive `gemini_pty.rs`
   - Archive `gemini-pty-design.md`

---

## Known Limitations

### Current Design Constraints

1. **Process per conversation**: Resource overhead (~50-100MB per session)
   - Mitigation: Session timeout/cleanup (not implemented yet)

2. **Cancellation semantics**: Kill entire process
   - No mid-turn soft cancel
   - Conversation state lost on cancel

3. **No checkpoint/resume**: If process dies, state lost
   - Gemini CLI supports `/chat save`, not implemented yet

4. **Authentication required**: CLI must be pre-authenticated
   - Error if `gemini` not installed or not logged in

### Platform Support

- **Linux**: ✅ Primary target
- **macOS**: ✅ Should work (untested)
- **Windows**: ⚠️ Pipes may behave differently, needs testing

---

## Debug Commands Reference

```bash
# Compile check
cargo check -p codex-core

# Run integration test (requires Gemini CLI)
cargo test -p codex-core --lib gemini_pipes::tests::test_single_turn_pipes -- --ignored --nocapture

# Run debug binary
cargo run --bin gemini_pipes_debug

# With verbose logging
RUST_LOG=debug cargo run --bin gemini_pipes_debug

# Check if Gemini CLI available
which gemini
gemini --version

# Manual pipe test
echo "Say: test" | gemini --model gemini-2.5-flash
```

---

## Immediate Action Items

**Priority 1: Debug timeout issue**
- [ ] Run debug binary and observe behavior
- [ ] Check Gemini CLI pipe behavior manually
- [ ] Add verbose logging to `stream_turn()`
- [ ] Identify why stream never completes

**Priority 2: Once working**
- [ ] Fix integration tests
- [ ] Wire into TUI model router
- [ ] Test end-to-end in TUI

**Priority 3: Polish**
- [ ] Write design documentation
- [ ] Cleanup old PTY code
- [ ] Add session timeout/cleanup

---

## Key Code Locations

```
Implementation:
- core/src/cli_executor/gemini_pipes.rs       (session + provider)
- core/src/cli_executor/mod.rs                (exports)
- core/src/cli_executor/prompt_detector.rs    (reused for completion)
- core/src/cli_executor/types.rs              (StreamEvent, CliError)

Debug/Test:
- core/src/bin/gemini_pipes_debug.rs          (manual testing)
- core/src/cli_executor/gemini_pipes.rs:593+  (unit tests)

TUI Integration (TODO):
- tui/src/providers/gemini_streaming.rs       (needs update)
- tui/src/model_router.rs                     (needs registration)

Documentation:
- docs/gemini-cli-pipes-status.md             (this file)
- docs/gemini-pty-design.md                   (obsolete)
```

---

## Technical Notes

### Stream Completion Detection

**Current approach**:
- Uses existing `PromptDetector` (9 tests passing)
- Looks for idle timeout OR explicit prompt markers
- Problem: May not work with pipes output format

**Alternative approaches**:
1. Look for specific Gemini output patterns (JSON mode?)
2. Use timeout-based completion (less reliable)
3. Gemini CLI flag for explicit end-of-response marker

### Async/Sync Bridging

- Provider API is fully async (tokio)
- Child process I/O is async (tokio::process)
- No blocking needed (unlike PTY with expectrl/smol)
- Streams use mpsc channels for backpressure

### Error Recovery

- Process death detected via EOF on stdout
- Session automatically removed from pool
- Next message to same conversation creates new session
- **Limitation**: Conversation history lost (no state transfer)

---

## Contact/Handoff

**Status**: Ready for debugging session
**Next assignee**: Should start with debug binary testing
**Estimated time**: 1-2 hours to identify and fix timeout issue
**Blocking**: Cannot integrate into TUI until stream completion works
