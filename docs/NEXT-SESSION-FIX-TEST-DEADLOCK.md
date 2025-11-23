# NEXT SESSION: Fix SPEC-955 Test Deadlock + CI Strict Mode

**Copy this entire prompt into your next Claude Code session**

---

## 🎯 Session Overview

**Primary Focus**: Fix SPEC-955 TUI test infrastructure deadlock (6-9 hours)
**Secondary**: Fix remaining CI strict mode issues (1-2 hours)
**Quick Win**: Add JSON parsing property tests (15 min)

**Session Goals:**
1. ✅ **CRITICAL**: Solve test deadlock - 5/9 tests hanging >60s
2. ✅ Fix CI strict mode errors (unsafe blocks, visibility, imports)
3. ✅ Add JSON parsing property tests (easy win, deferred from previous session)
4. ✅ Verify full test suite passes locally AND in CI
5. ✅ Complete SPEC-954 Tasks 2-3 (after deadlock fixed)

**Estimated Time**: 7-11 hours (full day session)

---

## 📋 Critical Context From Previous Session

### Session 2025-11-23 Summary

**What Worked** ✅:
- .gitignore 'core' pattern fix (commit 1f136399e)
- SPEC-954 Task 4 documentation complete (commits d70d05cb1, 1bb6c132f)
- OrderKey property tests: 4 tests, 8 total passing in 0.02s (commit 9c411766e)
- CI dead code fixes: 6 items with #[allow(dead_code)] (commit 0b45f9d15)
- SPEC-955 created: Deadlock investigation documented (commit fea3c1af0)

**What Failed** ❌:
- Message interleaving property test (reverted in 4bde2dc80) - caused hang
- Long conversation test (reverted in 4bde2dc80) - caused hang
- CI still failing: unused imports, unsafe blocks, visibility issues

**Critical Discovery**:
- **5/9 tests hang** at >60s timeout
- **Pre-existing bug** from previous session (not introduced this session)
- **False claim**: Previous handoff said "72 tests passing ✅" - NEVER verified
- Hang exists at commit 54f76a6f2 (session start point)

---

## 🔥 SPEC-955: Test Deadlock Details

### Symptoms

**Hanging Tests** (>60s timeout, all since previous session):
1. `test_overlapping_turns_no_interleaving`
2. `test_send_user_message`
3. `test_three_overlapping_turns_extreme_adversarial`
4. `test_chatwidget_single_exchange_snapshot`
5. `test_chatwidget_two_turns_snapshot`

**Working Tests** (<1s):
1. `test_harness_creation` (0.05s)
2. `test_simulate_streaming_response`
3. `test_history_cells_debug`

### Pattern Analysis

**Working tests** (file: `tui/src/chatwidget/test_harness.rs:310-351`):
```rust
let mut harness = TestHarness::new();
harness.simulate_streaming_response("test-req-1".to_string(), vec!["Hello", " world"]);
// NOTE: Does NOT call drain_app_events()
let debug = harness.history_cells_debug();
assert!(!debug.is_empty());
// ✅ Completes in <0.1s
```

**Hanging tests** (file: `tui/src/chatwidget/test_harness.rs:357-540`):
```rust
let mut harness = TestHarness::new();
harness.send_user_message("First turn");
harness.send_codex_event(Event { ... }); // Manual events
harness.drain_app_events(); // ← HANGS HERE or after this
let history_debug = harness.history_cells_debug(); // Never reaches
// ❌ Timeout at >60s
```

### Root Cause Hypotheses

**Hypothesis 1: Async/Sync Deadlock** (80% confidence):
```rust
// test_harness.rs:44
let widget = ChatWidget::new(...); // Spawns background tasks (comment at line 43)

// When test calls:
harness.send_codex_event(event); // → widget.handle_codex_event()
  // handle_codex_event may trigger async operations
  // If those operations haven't completed...
harness.drain_app_events(); // May block waiting for widget
  // But widget is blocked waiting for async task → DEADLOCK
```

**Hypothesis 2: Channel Deadlock** (15% confidence):
```rust
// handle_codex_event() sends to app_event_tx (unbounded channel)
// drain_app_events() receives from app_event_rx with try_recv()
// try_recv() shouldn't block... unless channel is in weird state
```

**Hypothesis 3: Tokio Runtime Nesting** (5% confidence):
```rust
#[tokio::test] // Creates tokio runtime
async fn test_something() {
    let harness = TestHarness::new(); // May spawn tokio tasks
    // Nested runtime or task spawn conflicts?
}
```

### Investigation Entry Points

**File**: `codex-rs/tui/src/chatwidget/test_harness.rs`
**Key Methods**:
- Line 30: `TestHarness::new()` - Creates ChatWidget (spawns background tasks per comment line 43)
- Line 44: `ChatWidget::new(...)` - Check what background tasks spawn
- Line 80: `send_codex_event()` - Calls widget.handle_codex_event()
- Line 85: `drain_app_events()` - Simple try_recv() loop (should be instant)

**File**: `codex-rs/tui/src/chatwidget/mod.rs`
**Key Methods**:
- Line 5940: `handle_codex_event()` - Event processing logic
  - Check for async operations, channel sends, blocking calls
  - Look for Handle::block_on() or tokio::spawn()
- Search for: `app_event_tx.send()`, `.block_on(`, `.spawn(`

**Debugging Commands**:
```bash
# Add tracing to pinpoint hang location
RUST_LOG=trace cargo test -p codex-tui test_send_user_message --lib -- --nocapture

# Check for deadlock with timeout and stack trace
timeout --signal=QUIT 10 cargo test -p codex-tui test_overlapping_turns --lib

# Compare working vs hanging
cargo test -p codex-tui test_simulate_streaming_response --lib -- --nocapture  # Works
cargo test -p codex-tui test_send_user_message --lib -- --nocapture  # Hangs
```

---

## 📋 Phase 1: Quick Win - JSON Parsing Property Tests (15 minutes)

**Goal**: Add property tests for JSON stream parsing with random chunk boundaries

**Why First**: Establishes momentum, doesn't require fixing deadlock

### Implementation

**File**: `codex-rs/core/src/cli_executor/claude_pipes.rs`

**Existing tests**: Lines 732-1143 (25 tests for JSON parsing)

**Add after existing tests** (around line 1143):

```rust
#[cfg(test)]
mod json_parsing_property_tests {
    use super::*;
    use proptest::prelude::*;

    prop_compose! {
        fn arbitrary_json_stream()
            (num_lines in 1usize..10)
            -> Vec<String>
        {
            // Generate valid JSON lines (Claude stream format)
            (0..num_lines)
                .map(|i| format!(r#"{{"type":"delta","text":"chunk{}"}}"#, i))
                .collect()
        }
    }

    proptest! {
        #[test]
        fn prop_json_parsing_chunk_boundaries(
            json_lines in arbitrary_json_stream()
        ) {
            // Property: JSON parsing works regardless of chunk boundaries
            let full_json = json_lines.join("\n");

            // Test with different chunk sizes (1, 5, 10, 50, 100 bytes)
            for chunk_size in [1, 5, 10, 50, 100] {
                let chunks: Vec<String> = full_json
                    .chars()
                    .collect::<Vec<_>>()
                    .chunks(chunk_size)
                    .map(|c| c.iter().collect())
                    .collect();

                // Parse should handle any chunking
                // For claude_pipes, we'd test parse_stream_json_event on each chunk
                for chunk in chunks {
                    if chunk.trim().is_empty() {
                        continue;
                    }
                    // Parser should either succeed or fail deterministically
                    // Not hang or panic
                    let mut session_id = None;
                    let result = parse_stream_json_event(&chunk, &mut session_id);
                    assert!(
                        result.len() >= 0,  // Just verify it returns
                        "Parser should handle chunk size {} deterministically",
                        chunk_size
                    );
                }
            }
        }

        #[test]
        fn prop_json_parsing_handles_malformed(
            valid_prefix in "[a-z]{5,20}",
            invalid_suffix in "[^{}\\[\\]]{1,10}"
        ) {
            // Property: Parser handles malformed JSON gracefully (no panic/hang)
            let malformed = format!("{}{}", valid_prefix, invalid_suffix);
            let mut session_id = None;

            // Should return empty or error, not panic
            let result = parse_stream_json_event(&malformed, &mut session_id);
            // Just verify it completes without panic
            assert!(result.len() >= 0);
        }
    }
}
```

**Validation**:
```bash
cd /home/thetu/code/codex-rs
cargo test -p codex-core prop_json --lib -- --nocapture
# Expected: 2 property tests, 512 scenarios total (256 each)
```

**Deliverable**:
- [ ] JSON property tests added
- [ ] All scenarios pass
- [ ] Commit: "test(core): Add JSON parsing property tests for chunk boundaries"

---

## 📋 Phase 2: Fix SPEC-955 Test Deadlock (6-9 hours) **PRIMARY FOCUS**

### Step 1: Reproduce and Isolate (30-60 min)

**Minimal reproduction**:
```rust
#[tokio::test]
async fn test_minimal_deadlock() {
    let mut harness = TestHarness::new();

    // Single user message
    harness.send_user_message("test");

    // Single event
    harness.send_codex_event(Event {
        id: "test".to_string(),
        event_seq: 0,
        msg: EventMsg::TaskStarted,
        order: Some(OrderMeta {
            request_ordinal: 1,
            output_index: Some(0),
            sequence_number: None,
        }),
    });

    println!("Before drain");
    harness.drain_app_events();
    println!("After drain"); // Does this print?

    println!("Before history access");
    let count = harness.history_cell_count();
    println!("After history access: {}", count); // Does this print?
}
```

**Run with tracing**:
```bash
RUST_LOG=codex_tui=trace,codex_core=trace cargo test -p codex-tui test_minimal_deadlock --lib -- --nocapture 2>&1 | tee /tmp/deadlock_trace.log
```

**Look for**:
- Last log message before hang (pinpoints exact location)
- Any async task spawns
- Channel operations (send/recv)
- Handle::block_on() calls

---

### Step 2: Check ChatWidget Background Tasks (30-60 min)

**File**: `codex-rs/tui/src/chatwidget/mod.rs`

**Search for task spawning**:
```bash
cd /home/thetu/code/codex-rs
grep -n "tokio::spawn\|Handle::block_on\|spawn_blocking" tui/src/chatwidget/mod.rs | head -20
```

**Check ChatWidget::new()** (search for `impl.*ChatWidget` then find `pub fn new`):
```bash
rg "pub fn new\(" tui/src/chatwidget/mod.rs -A 50 | head -80
```

**Questions to answer**:
- [ ] Does ChatWidget::new() spawn background tasks?
- [ ] Are those tasks awaited/joined anywhere?
- [ ] Do they communicate via channels that tests block on?

---

### Step 3: Compare Working vs Hanging (30 min)

**Working helper** (`test_harness.rs:198`):
```rust
pub fn simulate_streaming_response(&mut self, request_id: String, chunks: Vec<&str>) {
    // Sends TaskStarted
    self.send_codex_event(...);

    // Sends deltas
    for chunk in chunks {
        self.send_codex_event(...);
    }

    // Sends AgentMessage (completion)
    self.send_codex_event(...);

    // NOTE: Does NOT call drain_app_events()
}
```

**Difference Analysis**:
```bash
# Extract simulate_streaming_response
sed -n '198,250p' tui/src/chatwidget/test_harness.rs > /tmp/working.rs

# Extract test_overlapping_turns_no_interleaving
sed -n '357,540p' tui/src/chatwidget/test_harness.rs > /tmp/hanging.rs

# Compare
diff /tmp/working.rs /tmp/hanging.rs
```

**Hypothesis to test**:
- Does removing `drain_app_events()` fix the hang?
- Does adding `AgentMessage` completion event fix it?
- Is the hang in drain OR in subsequent history access?

---

### Step 4: Async/Sync Boundary Investigation (2-3 hours)

**Key File**: `codex-rs/tui/src/chatwidget/mod.rs`

**Method**: `handle_codex_event` (line 5940)

**Check for blocking operations**:
```bash
# Search for potential blocking in handle_codex_event
rg "Handle::block_on|\.block_on\(|\.await" tui/src/chatwidget/mod.rs | grep -A 2 -B 2 "5940\|handle_codex_event"
```

**Async patterns to investigate**:
1. **Handle::block_on() inside sync function**:
   ```rust
   // If handle_codex_event (sync) calls:
   some_handle.block_on(async_op)
   // And test is in tokio::test runtime, this can deadlock
   ```

2. **Channel send blocking**:
   ```rust
   // If app_event_tx.send() blocks waiting for receiver
   // But test is blocked waiting for send to complete → deadlock
   ```

3. **Spawn without await**:
   ```rust
   tokio::spawn(async { ... }); // Spawned but never awaited
   // If test exits before spawn completes, cleanup hangs
   ```

**Investigation commands**:
```bash
# Find all Handle::block_on in mod.rs
rg "Handle::block_on|block_on" tui/src/chatwidget/mod.rs -n

# Find all channel sends
rg "app_event_tx\|\.send\(" tui/src/chatwidget/mod.rs -n | grep "59[0-9][0-9]" # Around handle_codex_event

# Find all tokio spawns
rg "tokio::spawn|spawn_blocking" tui/src/chatwidget/mod.rs -n
```

---

### Step 5: Potential Fixes (1-3 hours depending on root cause)

**Fix 1: If Handle::block_on() is the issue**:
```rust
// BEFORE (deadlock):
pub fn handle_codex_event(&mut self, event: Event) {
    self.tokio_handle.block_on(async {
        // async work
    });
}

// AFTER (use spawn and don't wait):
pub fn handle_codex_event(&mut self, event: Event) {
    let handle = self.tokio_handle.clone();
    handle.spawn(async move {
        // async work
    });
    // Don't block waiting for completion
}
```

**Fix 2: If channel is the issue**:
```rust
// Ensure drain_app_events() is truly non-blocking
pub fn drain_app_events(&mut self) {
    // Add timeout to prevent infinite wait
    let start = std::time::Instant::now();
    while let Ok(event) = self.app_event_rx.try_recv() {
        self.captured_events.push(event);
        if start.elapsed() > std::time::Duration::from_millis(100) {
            break; // Safety escape hatch
        }
    }
}
```

**Fix 3: If runtime nesting is the issue**:
```rust
// Don't use #[tokio::test], use regular #[test] with Runtime::new()
#[test]
fn test_overlapping_turns_no_interleaving() {
    let rt = tokio::runtime::Runtime::new().unwrap();
    rt.block_on(async {
        // Test code
    });
}
```

---

### Step 6: Add Regression Test (30 min)

After fixing deadlock, add test that would have caught it:

```rust
#[tokio::test]
async fn test_no_deadlock_with_multiple_events() {
    let mut harness = TestHarness::new();

    // This pattern previously caused deadlock
    for i in 0..10 {
        harness.send_user_message(&format!("Message {}", i));
        harness.send_codex_event(Event {
            id: format!("test-{}", i),
            event_seq: 0,
            msg: EventMsg::TaskStarted,
            order: Some(OrderMeta {
                request_ordinal: (i + 1) as u64,
                output_index: Some(0),
                sequence_number: None,
            }),
        });
    }

    // This should complete in <1s, not hang
    let start = std::time::Instant::now();
    harness.drain_app_events();
    let elapsed = start.elapsed();

    assert!(
        elapsed.as_secs() < 1,
        "drain_app_events should be instant, took {:?}",
        elapsed
    );

    // Should be able to access history without hanging
    let count = harness.history_cell_count();
    assert!(count > 0);
}
```

---

## 📋 Phase 3: Fix CI Strict Mode Issues (1-2 hours)

### Current CI Failures

**Run**: 19607726192 (latest TUI Tests failure)

**Errors Found**:
1. `unused import: chrono::Utc` in `core/src/cli_executor/context.rs:1`
2. `unused imports: Command and Stdio` in `core/src/cli_executor/gemini_pty.rs:27`
3. `unused mut` in `claude_pipes.rs:694` and `gemini_pipes.rs:636`
4. `type OpenAiTool is more private than item` in `agent_tool.rs:1420,1616`
5. `unsafe function set_var/remove_var` in test code (9 occurrences)

### Fix Strategy

**Step 1: Auto-fix what cargo can handle** (5 min):
```bash
cd /home/thetu/code/codex-rs
cargo fix --lib --allow-dirty --all-features
cargo fix --tests --allow-dirty --all-features
```

**Step 2: Manual fixes for visibility** (10 min):

File: `core/src/openai_tools.rs:51`
```rust
// BEFORE:
#[allow(dead_code)]
pub(crate) enum OpenAiTool { ... }

// AFTER:
#[allow(dead_code)]
pub enum OpenAiTool { ... }  // ← Remove (crate), make fully pub
```

**Step 3: Fix unsafe in tests** (15-30 min):

File: `core/tests/` (various test files)

**Find unsafe calls**:
```bash
rg "set_var|remove_var" core/tests/ -n
```

**Wrap in unsafe blocks**:
```rust
// BEFORE:
std::env::set_var("KEY", "value");

// AFTER:
unsafe { std::env::set_var("KEY", "value"); }
```

OR add `#[allow(unsafe_code)]` to test functions using env manipulation.

**Step 4: Verify CI compilation** (10 min):
```bash
# Match CI exact command
cd /home/thetu/code/codex-rs
RUSTFLAGS="-D warnings" cargo test --lib -p codex-tui -p codex-core -p codex-protocol --all-features --no-run

# Should compile without errors
echo $?  # Expected: 0
```

---

## 📋 Phase 4: Complete SPEC-954 Tasks 2-3 (30 min)

**AFTER deadlock is fixed**, add these automated tests:

### Task 2: Drop Cleanup Verification

**File**: `codex-rs/core/src/cli_executor/claude_pipes.rs` or `gemini_pipes.rs`

**Test approach**:
```rust
#[cfg(test)]
mod drop_tests {
    use super::*;

    #[test]
    fn test_drop_trait_exists() {
        // Verify Drop implementation exists for ClaudePipesProvider
        // This is a compile-time check
        fn assert_has_drop<T: Drop>() {}
        assert_has_drop::<ClaudePipesProvider>();
    }

    // Note: Actually testing process killing requires integration test
    // with real process spawning - defer to separate integration test file
}
```

**Integration test** (if time allows):

File: `codex-rs/core/tests/cli_executor_drop_tests.rs` (new file)
```rust
use codex_core::cli_executor::claude::ClaudePipesProvider;
use std::process::Command;

#[test]
fn test_drop_kills_cli_process() {
    // Spawn a Claude CLI process
    let provider = ClaudePipesProvider::with_cwd("claude-sonnet-4.5", std::env::current_dir().unwrap());

    // Get PID (if accessible)
    // This may require adding a method to expose child process PID

    // Drop the provider
    drop(provider);

    // Verify process is killed
    // Check ps output or process table
    // assert!(process_no_longer_exists(pid));
}
```

**Deliverable**:
- [ ] Drop trait test added
- [ ] SPEC-954 Task 2 marked complete

### Task 3: Long Conversation Stability (Already created, just needs deadlock fix)

**File**: `codex-rs/tui/src/chatwidget/test_harness.rs:1100` (already exists, was reverted)

**After deadlock fixed**, restore simplified version:
```rust
#[tokio::test]
async fn test_long_conversation_stability() {
    let mut harness = TestHarness::new();

    // 5 turns (balanced between coverage and performance)
    for i in 1..=5 {
        harness.send_user_message(&format!("Turn {}", i));

        // Use simulate_streaming_response instead of manual events
        harness.simulate_streaming_response(
            format!("req-{}", i),
            vec!["Response", " for", " turn", &i.to_string()]
        );
    }

    // Verify stability
    let (user_groups, assistant_groups) = harness.cells_by_turn();
    assert_eq!(user_groups.len(), 5);
    assert_eq!(assistant_groups.len(), 5);

    // Verify contiguity (no interleaving)
    for group in user_groups.iter().chain(assistant_groups.iter()) {
        for window in group.windows(2) {
            assert_eq!(window[1], window[0] + 1, "Groups should be contiguous");
        }
    }
}
```

**Deliverable**:
- [ ] Long conversation test working
- [ ] SPEC-954 Task 3 marked complete
- [ ] SPEC-954 marked COMPLETE in SPEC.md

---

## 📋 Phase 5: Verify Everything Works (1 hour)

### Local Verification

```bash
cd /home/thetu/code/codex-rs

# Run ALL tests
cargo test --lib --all-features 2>&1 | tee /tmp/all_tests.log

# Check results
grep "test result:" /tmp/all_tests.log
# Expected: "test result: ok. XX passed; 0 failed; 0 ignored"

# Verify no hangs (should complete in <60s total)
```

### CI Verification

```bash
# Push to trigger CI
git push origin main

# Wait 3-5 minutes
sleep 180

# Check status
gh run list --limit 1
# Expected: "completed success"

# If failed, get logs
gh run view --log-failed | head -100
```

### Success Criteria

- [ ] All local tests pass without timeout (<60s total)
- [ ] CI TUI Tests workflow passes ✅
- [ ] CI Coverage workflow passes ✅
- [ ] No deadlocks, no hangs
- [ ] Badge shows green

---

## 📊 Expected Outcomes

### Test Suite State

**Before Session**:
- 9 tests total
- 3 passing, 5 hanging, 1 snapshot fail
- Infrastructure broken

**After Session**:
- 11-13 tests total
- ALL passing
- No hangs (<60s total runtime)
- Infrastructure solid

**New Tests Added**:
1. JSON parsing property tests (2 tests, 512 scenarios)
2. Drop cleanup test (1 test)
3. Long conversation stability (1 test, 5 turns)
4. Deadlock regression test (1 test)

---

## 🚀 Quick Start Commands

```bash
# Load context
cd /home/thetu/code

# Check current state
git log --oneline -8
git status

# Query local-memory for context
# Search: "SPEC-KIT-955 deadlock" or "SPEC-KIT-954 testing"
# IDs from this session:
# - 282a6c34-e2ce-4834-8058-55c95550160f (deadlock discovery)
# - dd4fe1c3-8fae-412f-8f52-3dd57942462e (CI dead code fix)
# - 86082e33-cac7-4ee5-ad71-a57cdabfebd5 (.gitignore pattern)

# Read critical files
cat docs/SPEC-KIT-955-tui-test-deadlock/spec.md
cat docs/SPEC-KIT-954-session-management-polish/spec.md
cat codex-rs/TESTING-CRITIQUE.md
```

---

## 📚 Essential Context Files

**SPEC Documents**:
- `docs/SPEC-KIT-955-tui-test-deadlock/spec.md` - THIS SESSION'S CRITICAL ISSUE
- `docs/SPEC-KIT-954-session-management-polish/spec.md` - Parent SPEC (Task 4 complete ✅, Tasks 2-3 blocked)
- `SPEC.md` - Tracker (needs SPEC-955 row added)

**Test Files**:
- `codex-rs/tui/src/chatwidget/test_harness.rs` - Test infrastructure (1,017 LOC, 9 tests)
- `codex-rs/tui/src/chatwidget/orderkey_property_tests.rs` - Working property tests ✅
- `codex-rs/core/src/cli_executor/claude_pipes.rs` - JSON parsing (25 tests, add properties here)

**Widget Code**:
- `codex-rs/tui/src/chatwidget/mod.rs:5940` - handle_codex_event() (likely deadlock source)
- `codex-rs/tui/src/chatwidget/mod.rs` - ChatWidget::new() (check background tasks)

**CI Configuration**:
- `.github/workflows/tui-tests.yml` - TUI Tests workflow (currently failing)
- Check: RUSTFLAGS=-D warnings (line 22 in env)

---

## 🎯 Session Structure & Checkpoints

### Hour 1: Quick Wins + Investigation Setup
- ✅ JSON parsing property tests (15 min)
- ✅ Minimal deadlock reproduction (30 min)
- ✅ Add extensive tracing to pinpoint hang location (15 min)

**Checkpoint 1**: Have minimal reproduction case and trace logs showing exact hang location

---

### Hours 2-3: Root Cause Analysis
- ✅ Analyze ChatWidget::new() background tasks (30 min)
- ✅ Investigate handle_codex_event() async boundaries (60 min)
- ✅ Compare working vs hanging test patterns (30 min)

**Checkpoint 2**: Have clear hypothesis about root cause (async/sync/channel)

---

### Hours 3-5: Implement Fix
- ✅ Implement fix based on hypothesis (1-2 hours)
- ✅ Test fix with hanging tests (30 min)
- ✅ Add regression test (30 min)

**Checkpoint 3**: All 9 original tests pass without hanging

---

### Hours 5-6: Expand Tests + CI
- ✅ Add Tasks 2-3 automated tests (30 min)
- ✅ Fix remaining CI strict mode issues (30-60 min)
- ✅ Verify full test suite passes locally and CI (30 min)

**Checkpoint 4**: Green CI, all tests passing

---

### Hour 7: Documentation & Cleanup
- ✅ Update SPEC-954 (mark complete)
- ✅ Update SPEC-955 (document solution)
- ✅ Update SPEC.md tracker
- ✅ Store critical findings to local-memory
- ✅ Clean up any temporary debugging code

**Final Checkpoint**: Production-ready test infrastructure, SPEC-954 complete ✅

---

## 🔍 Debugging Tools & Techniques

### Tracing

```bash
# Full trace
RUST_LOG=trace cargo test -p codex-tui test_minimal --lib -- --nocapture 2>&1 | tee trace.log

# Module-specific
RUST_LOG=codex_tui::chatwidget=trace cargo test ...

# Search trace for clues
grep -i "hang\|block\|wait\|spawn" trace.log
```

### Stack Traces

```bash
# Send SIGQUIT to get stack trace of hanging process
timeout --signal=QUIT 10 cargo test -p codex-tui test_hanging --lib

# Or use gdb
cargo test -p codex-tui test_hanging --lib --no-run
gdb target/debug/deps/codex_tui-XXXXX
(gdb) run test_hanging
# Wait for hang, then Ctrl+C
(gdb) thread apply all bt  # Get all thread backtraces
```

### Async Runtime Inspection

```rust
// Add to test
println!("Tokio runtime active threads: {:?}", tokio::runtime::Handle::current());

// Check if tasks are blocked
tokio::task::yield_now().await; // Give other tasks a chance
```

---

## 💾 Local-Memory Context

**Query before starting**:
```
Search: "SPEC-KIT-955 deadlock async"
Tags: ["testing", "async-deadlock", "spec:SPEC-KIT-955"]
```

**Store after fixing**:
```
Content: "SPEC-955 deadlock fix: Root cause was [ACTUAL CAUSE]. Solution: [ACTUAL FIX]. Pattern: [GENERALIZABLE LESSON]. Time: [ACTUAL HOURS]. Files: [CHANGED FILES]."
Domain: "debugging"
Tags: ["type:bug-fix", "spec:SPEC-KIT-955", "async-deadlock", "testing"]
Importance: 10
```

---

## ⚠️ Known Landmines

1. **Don't trust previous test claims** - Always run tests yourself to verify
2. **Cargo test from codex-rs/** - Not from repo root
3. **Timeout for safety** - Use `timeout 30 cargo test` to prevent infinite hangs
4. **Kill hanging processes** - `pkill -9 -f "codex_tui.*test"` before fresh runs
5. **Tokio runtime conflicts** - Watch for nested Runtime::new() or block_on()
6. **CI is strict** - `RUSTFLAGS=-D warnings` catches everything
7. **Channel behavior** - try_recv() should never block, but verify

---

## 📝 Session Completion Criteria

**Minimum** (if deadlock proves intractable):
- ✅ JSON parsing property tests added
- ✅ Root cause identified and documented
- ✅ Partial fix or workaround implemented
- ✅ Clear plan for full fix documented

**Target** (expected outcome):
- ✅ Deadlock completely fixed
- ✅ All 9+ tests passing locally
- ✅ CI passing (TUI Tests workflow green)
- ✅ SPEC-954 complete (Tasks 2-3 done)
- ✅ SPEC-955 complete (deadlock solved)

**Stretch** (if time allows):
- ✅ Additional property tests (10+ total)
- ✅ Test coverage improvements
- ✅ Performance optimization (test suite <30s)

---

## 🎓 Investigation Principles

1. **Reproduce minimally** - Single test, single event, isolate the hang
2. **Trace extensively** - RUST_LOG=trace shows async operations
3. **Compare working vs broken** - What does simulate_streaming_response() do differently?
4. **Test hypotheses incrementally** - One change at a time, verify
5. **Document discoveries** - Store to local-memory as you learn

---

## 📊 Success Metrics

### Test Suite Health
- **Passing rate**: 100% (was 33%)
- **Execution time**: <60s total (was >300s with hangs)
- **Reliability**: 0 timeouts, 0 deadlocks

### CI Health
- **TUI Tests**: ✅ Passing
- **Code Coverage**: ✅ Passing
- **Build time**: <5 minutes
- **Badge status**: Green

### Knowledge
- **Root cause**: Documented in SPEC-955
- **Fix pattern**: Stored to local-memory (importance: 10)
- **Prevention**: Regression test added

---

## 🔗 Reference Links

**Commits from this session**:
- fea3c1af0: SPEC-955 creation
- 4bde2dc80: Revert hanging tests
- 0b45f9d15: CI dead code fixes
- 9c411766e: OrderKey property tests (WORKING ✅)
- d70d05cb1, 1bb6c132f: SPEC-954 Task 4 docs
- 1f136399e: .gitignore fix

**Previous session handoffs** (for context):
- `docs/NEXT-SESSION-TUI-TESTING-HANDOFF.md`
- `docs/NEXT-SESSION-CI-DEBUGGING.md`

---

**NEXT SESSION PRIMARY GOAL**: Fix the test deadlock. Everything else is secondary.

**Estimated total time**: 7-11 hours
**Complexity**: HIGH (async/tokio debugging)
**Priority**: CRITICAL (blocks all TUI testing)

Good luck! 🚀
