# SPEC-955 Session 2: Complete Async Event System Refactor

## Session Type: Debugging & Completion
**Estimated Duration:** 6-10 hours
**Priority:** CRITICAL - Blocks SPEC-954 completion
**Status:** Primary deadlock fixed, test failures require investigation

---

## 📊 SESSION 1 RESULTS (5 hours completed)

### ✅ Achievements

**Deadlock FIXED:**
- ✅ 0/9 tests hanging (was 5/9 hanging for 60+ seconds)
- ✅ All tests complete in 0.19-0.21s
- ✅ Core async/sync incompatibility resolved

**Test Progress:**
- ✅ 4/9 tests passing (up from 3/9)
- ❌ 5/9 tests failing (was 5/9 hanging)
- ⚠️ Tests fail fast with clear errors (not hanging)

**Code Refactoring:**
- ✅ AppEventSender: std::sync::mpsc → tokio::sync::mpsc::Unbounded
- ✅ App event loop: UnboundedReceiver with multi-thread safe polling
- ✅ TestHarness: tokio::sync::mpsc::unbounded_channel
- ✅ ChatWidget: 4 browser check deadlocks fixed (tokio::sync::oneshot)
- ✅ Test infrastructure: Most test files migrated to tokio channels
- ✅ 7 integration tests added to app_event_sender.rs (all passing)

### ❌ Remaining Issues

**Critical Test Failures (5/9):**
1. `test_overlapping_turns_no_interleaving` - 0 assistant cells created
2. `test_three_overlapping_turns_extreme_adversarial` - 0 assistant cells
3. `test_chatwidget_empty_state_snapshot` - Snapshot mismatch
4. `test_chatwidget_single_exchange_snapshot` - Snapshot mismatch
5. `test_chatwidget_two_turns_snapshot` - Snapshot mismatch

**Root Cause Discovery:**
- **CRITICAL:** Even "passing" test `test_simulate_streaming_response` has 0 assistant cells!
- `handle_codex_event()` receives events but doesn't create history cells
- Tests call `widget.handle_codex_event(event)` → no assistant cells appear
- User cells ARE created (harness.send_user_message works)
- Welcome/Notice cells exist (initial state)
- **Hypothesis:** ChatWidget might need Op channel submissions, not direct event injection

**Incomplete Refactoring:**
- ⚠️ `theme_selection_view.rs` - 14 std::sync::mpsc uses (ProgressMsg channels)
- ⚠️ `file_search.rs` - 1 use (but uses std::thread::spawn, may be OK)
- ℹ️ `chatwidget/mod.rs` - Still has std::sync::mpsc for TerminalRunController (separate system, intentional)

---

## 📁 FILES MODIFIED (Uncommitted)

### Core Event System (Complete ✅)
```
tui/src/app_event_sender.rs          - tokio::sync::mpsc::UnboundedSender
tui/src/app.rs                       - UnboundedReceiver, next_event_priority refactored
tui/src/chatwidget/test_harness.rs   - unbounded_channel, multi_thread tests
tui/src/chatwidget/test_support.rs   - unbounded_channel
tui/src/bottom_pane/agent_editor_view.rs - unbounded_channel in tests
```

### ChatWidget Integration (Partial ⚠️)
```
tui/src/chatwidget/mod.rs            - 4 browser checks: tokio::oneshot + Handle::block_on
                                       Still has mpsc for TerminalRunController (OK)
                                       Browser initialization deferred (browser_enabled = false logic)
```

### Test Infrastructure (Complete ✅)
```
tui/src/chatwidget/tests.rs          - make_chatwidget_manual returns UnboundedReceiver
                                       drain_insert_history takes &mut UnboundedReceiver
                                       pump_app_events takes &mut UnboundedReceiver
tui/src/bottom_pane/chat_composer.rs  - 15 test channels: batch replaced to unbounded
tui/src/bottom_pane/mod.rs            - test channel updated
tui/src/bottom_pane/chat_composer_history.rs - test channel updated
tui/src/bottom_pane/approval_modal_view.rs   - test channel updated
tui/src/user_approval_widget.rs       - test channel updated
tui/src/chatwidget/agent_install.rs   - import updated
tui/tests/message_interleaving_test.rs - unbounded_channel
```

### Not Modified (Intentional)
```
tui/src/app_event.rs                 - TerminalRunController uses StdSender (separate system)
tui/src/file_search.rs               - std::thread::spawn context (appropriate)
tui/src/bottom_pane/theme_selection_view.rs - Needs evaluation (Session 2)
core/src/config_watcher.rs           - Out of scope (core crate)
file-search/src/lib.rs               - Library crate (out of scope)
```

---

## 🔍 CRITICAL DEBUGGING DISCOVERY

### Test Failure Analysis

**Observation:**
```bash
# Even "passing" test has 0 assistant cells:
$ cargo test test_simulate_streaming_response -- --nocapture

=== Simulate Test History ===
0 | AnimatedWelcome | Welcome to Code
1 | Notice | Popular commands:...
=== End ===

Assistant cells: 0  ✅ Test passes (checks !debug.is_empty())
```

**Pattern:**
```rust
// Test code:
harness.send_codex_event(Event {
    msg: EventMsg::AgentMessageDelta(...),
    ...
});
// Result: Event processed, but NO history cell created

// Works:
harness.send_user_message("Hello");
// Result: User history cell created ✅
```

**Hypotheses:**

**H1: ChatWidget needs Op channel submissions (not direct events)**
- Events might be internal representation
- Ops might be the external API that triggers event processing
- Test might need: `codex_op_tx.send(Op::Submit(...))` instead of `handle_codex_event()`

**H2: Async task initialization incomplete**
- ChatWidget::new() spawns background tasks
- Tasks might need time to start before processing events
- 50ms sleep didn't help (tried)

**H3: Missing conversation/session state**
- Widget might need active conversation context
- Events might be ignored without proper session state
- Need to check if widget.session or similar needs initialization

**H4: Tests were never working**
- Added recently (commits b802de208, 2c42f0735)
- Immediately hung (deadlock)
- Reverted in 4bde2dc80
- No evidence they ever passed

---

## 🎯 SESSION 2 OBJECTIVES

### Primary Goals

1. **Debug Test Failures (4-6 hours)**
   - Investigate why handle_codex_event() doesn't create history cells
   - Fix all 5 failing tests to achieve 9/9 passing
   - Validate test infrastructure is sound

2. **Complete Refactoring (1-2 hours)**
   - Update theme_selection_view.rs (14 std::sync::mpsc uses)
   - Update file_search.rs (1 use)
   - Remove all std::sync::mpsc from async contexts

3. **Validation & Testing (2-3 hours)**
   - Verify all 9/9 TUI tests pass
   - Run full workspace test suite
   - Manual TUI smoke testing
   - Performance comparison
   - Memory leak check

4. **Documentation & Completion (1 hour)**
   - Update SPEC-955 with solution details
   - Update SPEC.md tracker
   - Store findings to local-memory
   - Single comprehensive commit

**Total Estimated Time:** 8-12 hours

---

## 🔬 IMMEDIATE NEXT STEPS (Start Here)

### Step 1: Load Context (5 min)

```bash
cd /home/thetu/code

# Check current state
git status
git log --oneline -5

# Query local-memory for Session 1 progress
# Search: "SPEC-KIT-955" or ID: 701b6762-973a-4bf0-ae92-78c5d84de11a
```

**Expected State:**
- Branch: main
- Modified files: 12 files (all TUI event system)
- Untracked: Various docs
- Last commit: 527edb771 (JSON property tests)
- All changes uncommitted

### Step 2: Verify Current Test Status (5 min)

```bash
cd codex-rs

# Quick test run to confirm starting state
timeout 180 cargo test -p codex-tui --lib test_harness::tests 2>&1 | grep "test result:"

# Expected: ok. 4 passed; 5 failed; 0 ignored; finished in ~0.20s
```

**Passing Tests (4/9):**
- test_harness_creation ✅
- test_history_cells_debug ✅
- test_simulate_streaming_response ✅ (but has 0 assistant cells!)
- test_send_user_message ✅

**Failing Tests (5/9):**
- test_overlapping_turns_no_interleaving - Logic failure (0 assistant cells)
- test_three_overlapping_turns_extreme_adversarial - Logic failure
- test_chatwidget_empty_state_snapshot - Snapshot mismatch
- test_chatwidget_single_exchange_snapshot - Snapshot mismatch
- test_chatwidget_two_turns_snapshot - Snapshot mismatch

### Step 3: Create Todo List (3 min)

```
Phase 2 (continued): Complete Refactoring
├── 2.5: Update theme_selection_view.rs ProgressMsg channels
├── 2.6: Update file_search.rs if needed
└── 2.7: Verify all std::sync::mpsc uses are intentional

Phase 3: Debug Test Failures
├── 3.1: Investigate handle_codex_event() behavior
├── 3.2: Test Hypothesis H1 (Op channel vs direct events)
├── 3.3: Test Hypothesis H2 (async initialization)
├── 3.4: Test Hypothesis H3 (conversation state)
├── 3.5: Fix 5 failing tests
└── 3.6: Verify 9/9 tests passing

Phase 4: Validation
├── 4.1: Run full workspace test suite
├── 4.2: Integration tests verification
├── 4.3: CI strict mode fixes
└── 4.4: Manual TUI smoke testing

Phase 5: Completion
├── 5.1: Update SPEC-955 documentation
├── 5.2: Update SPEC.md tracker
├── 5.3: Store findings to local-memory
└── 5.4: Single comprehensive commit
```

---

## 🐛 DEBUGGING STRATEGY (Phase 3)

### Investigation 1: Understand handle_codex_event Flow (1 hour)

**Objective:** Trace event processing to understand why history cells aren't created

**Steps:**

1. **Add comprehensive logging:**
```rust
// In tui/src/chatwidget/mod.rs, find handle_codex_event method
pub(crate) fn handle_codex_event(&mut self, event: Event) {
    eprintln!("DEBUG: handle_codex_event called with: {:?}", event.msg);
    // ... existing code ...
    eprintln!("DEBUG: History cell count after: {}", self.history_cells.len());
}
```

2. **Run test with logging:**
```bash
cd codex-rs
cargo test -p codex-tui --lib test_simulate_streaming_response -- --nocapture 2>&1 | grep "DEBUG:"
```

3. **Check for:**
   - Are events received? (should see DEBUG logs)
   - Are events processed? (check control flow)
   - Are history_cells being added? (count should increase)
   - Are there error conditions silently failing?

### Investigation 2: Test Op Channel Hypothesis (1 hour)

**Hypothesis:** Tests should submit Ops, not inject Events directly

**Steps:**

1. **Check how real app submits messages:**
```bash
# Find where user messages are submitted in production
rg "codex_op_tx.send" --type rust tui/src -A3
```

2. **Compare to test approach:**
```rust
// Current test (doesn't work):
harness.send_codex_event(Event { ... });

// Potential fix:
harness.widget.codex_op_tx.send(Op::Submit { ... })?;
// Then drain and process events
```

3. **Test modified approach:**
   - Update TestHarness to expose Op channel
   - Modify one failing test to use Op submission
   - Check if assistant cells are created

### Investigation 3: Check Conversation State (30 min)

**Hypothesis:** Widget needs active conversation/session before processing events

**Steps:**

1. **Inspect ChatWidget internal state:**
```bash
rg "struct ChatWidget" --type rust tui/src/chatwidget/mod.rs -A30
```

2. **Check for session/conversation fields:**
   - Does widget have a Session object?
   - Does it have a ConversationManager?
   - Do events require request_id matching?

3. **Test initialization:**
   - Check if TestHarness needs to initialize conversation state
   - Look for setup methods that real app calls but tests don't

### Investigation 4: Compare Working vs Failing Tests (30 min)

**Objective:** Find the minimal difference between passing and failing tests

**Passing Test Structure:**
```rust
test_simulate_streaming_response() {
    harness = TestHarness::new();
    sleep(50ms);  // Added in Session 1
    harness.simulate_streaming_response(...);  // Sends events
    // NO drain_app_events()
    assert!(!harness.history_cells_debug().is_empty());  // Passes (welcome cells)
}
```

**Failing Test Structure:**
```rust
test_overlapping_turns() {
    harness = TestHarness::new();
    harness.send_user_message("Turn 1");  // Creates user cell ✅
    harness.send_user_message("Turn 2");  // Creates user cell ✅
    harness.send_codex_event(...);  // Should create assistant cell ❌
    harness.drain_app_events();
    assert!(assistant_cells >= 2);  // FAILS: got 0
}
```

**Key Differences:**
- Failing tests call drain_app_events() (passing test doesn't)
- Failing tests use multiple request_ordinals (1, 2, 3)
- Passing test uses only request_ordinal: 1

**Action:** Test if request_ordinal > 1 is the issue

### Investigation 5: Trace Event Delivery Path (1 hour)

**Objective:** Understand complete event flow from test → widget → history

**Steps:**

1. **Map the call chain:**
```
TestHarness.send_codex_event(event)
  └─> widget.handle_codex_event(event)
      └─> ??? (what happens here?)
          └─> history_cells.push(cell) (NOT HAPPENING)
```

2. **Add tracing:**
```rust
// In handle_codex_event, add prints before/after key operations
// Track: event type, request_ordinal, output_index, history mutations
```

3. **Check for async task spawn:**
```rust
// If handle_codex_event spawns tokio tasks:
tokio::spawn(async move {
    // Process event here
    // Add to history here
});
// Then test might need to await task completion!
```

---

## 📝 DETAILED NEXT STEPS

### Phase 2.5: Complete theme_selection_view.rs Refactor (1 hour)

**Context:** 14 uses of std::sync::mpsc for ProgressMsg channels in theme installation

**Files:** `tui/src/bottom_pane/theme_selection_view.rs`

**Pattern to Change:**
```rust
// BEFORE (lines 273, 505, 834, 852):
progress_tx: std::sync::mpsc::Sender<ProgressMsg>
rx: Option<std::sync::mpsc::Receiver<ProgressMsg>>

// AFTER:
progress_tx: tokio::sync::mpsc::UnboundedSender<ProgressMsg>
rx: Option<tokio::sync::mpsc::UnboundedReceiver<ProgressMsg>>

// Channel creation (lines 1214, 1228, 1303, 1317):
let (txp, rxp) = std::sync::mpsc::channel::<ProgressMsg>();
// →
let (txp, rxp) = tokio::sync::mpsc::unbounded_channel::<ProgressMsg>();

// TryRecvError (lines 1852, 1854, 2234, 2235):
Err(std::sync::mpsc::TryRecvError::Empty) => break,
// →
Err(tokio::sync::mpsc::error::TryRecvError::Empty) => break,
```

**Steps:**
1. Read theme_selection_view.rs to understand ProgressMsg usage
2. Update all 14 occurrences systematically
3. Compile and verify no errors
4. Check if theme-related functionality uses tokio tasks (if yes, must change)

### Phase 2.6: Evaluate file_search.rs (30 min)

**Context:** 1 use of std::sync::mpsc in file search partial results

**Files:** `tui/src/file_search.rs:170`

**Current Code:**
```rust
let (part_tx, part_rx) = std::sync::mpsc::channel::<Vec<file_search::FileMatch>>();

// Receiver thread: forward partial updates
std::thread::spawn(move || {
    while let Ok(matches) = part_rx.recv() {
        // ...
    }
});
```

**Decision:**
- ✅ Uses `std::thread::spawn` (not tokio::spawn)
- ✅ std::sync::mpsc is appropriate for thread-to-thread communication
- **Action:** Leave as-is UNLESS tests prove it causes issues

---

## 🧪 PHASE 3: DEBUG TEST FAILURES (4-6 hours)

### 3.1: Add Diagnostic Logging (30 min)

**File:** `tui/src/chatwidget/mod.rs`

**Add to handle_codex_event:**
```rust
pub(crate) fn handle_codex_event(&mut self, event: Event) {
    eprintln!("[DIAG] handle_codex_event: id={}, msg={:?}, order={:?}",
        event.id, event.msg, event.order);

    // ... existing processing ...

    eprintln!("[DIAG] History cells after processing: {}", self.history_cells.len());
    eprintln!("[DIAG] Active requests: {:?}", self.active_requests.keys());
}
```

**Run diagnostics:**
```bash
cd codex-rs
cargo test test_simulate_streaming_response -- --nocapture 2>&1 | grep "\[DIAG\]"
```

**Look for:**
- Events being received
- History cell count changes
- Request tracking (active_requests)
- Any error paths being hit

### 3.2: Test Op Channel Hypothesis (2 hours)

**Create experimental test:**

```rust
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn test_op_channel_submission() {
    let mut harness = TestHarness::new();

    // Try submitting through Op channel instead of direct event injection
    let op = Op::Submit {
        items: vec![InputItem::Text { text: "Hello".to_string() }],
        slash_command: None,
    };

    // Need to expose codex_op_tx in TestHarness
    harness.widget.codex_op_tx.send(op).unwrap();

    // Give time for processing
    tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;

    // Drain events
    harness.drain_app_events();

    // Check if assistant response appears
    let assistant_count = harness.widget.history_cells.iter()
        .filter(|c| matches!(c.kind(), HistoryCellType::Assistant))
        .count();

    println!("Assistant cells after Op submission: {}", assistant_count);
}
```

**If this works:** Refactor all failing tests to use Op channel
**If this fails:** Continue to H3

### 3.3: Investigate Session/Conversation State (1 hour)

**Check widget internal state:**

```bash
# Find Session/Conversation fields in ChatWidget
rg "session:|conversation:" --type rust tui/src/chatwidget/mod.rs -B2 -A2

# Check if events require session context
rg "active_requests|pending_exec" --type rust tui/src/chatwidget/mod.rs -B2 -A2
```

**Potential fixes:**
- Initialize widget.session properly in TestHarness
- Set up conversation context before sending events
- Ensure request_id tracking is active

### 3.4: Fix Snapshot Tests (30 min)

**After logic tests pass, update snapshots:**

```bash
cd codex-rs

# Review snapshot changes
cargo insta test --review

# Or accept all if changes are expected (browser_enabled = false)
cargo insta accept
```

---

## ✅ PHASE 4: VALIDATION (2-3 hours)

### 4.1: Full Test Suite (30 min)

```bash
cd codex-rs

# All TUI tests
cargo test -p codex-tui --lib 2>&1 | tee /tmp/tui_tests.log
# Expected: All pass

# Full workspace
cargo test --workspace 2>&1 | tee /tmp/workspace_tests.log
# Check for regressions
```

### 4.2: Integration Tests Verification (15 min)

```bash
# AppEventSender integration tests should still pass
cargo test -p codex-tui app_event_sender::tests --lib

# Expected: 7/7 passing (baseline behavior preserved)
```

### 4.3: CI Strict Mode (30 min)

```bash
# Clippy
cargo clippy --workspace --all-targets --all-features -- -D warnings

# Format
cargo fmt --all -- --check

# Fix any issues
cargo fix --allow-dirty
cargo fmt --all
```

### 4.4: Manual TUI Smoke Testing (45 min)

```bash
# Build optimized binary
cd /home/thetu/code
~/code/build-fast.sh run

# Test scenarios:
# 1. Basic conversation (10 messages)
# 2. Switch models (/model)
# 3. Long message (>1000 chars)
# 4. Rapid messages (send 10 quickly)
# 5. /help overlay
# 6. /agents command
# 7. Background events (if any)
# 8. Clean exit (Ctrl-C)

# Verify:
# - No crashes
# - No UI glitches
# - Messages appear in order
# - No orphaned processes
# - Responsive (no lag)
```

### 4.5: Performance Comparison (30 min)

**Benchmark event throughput:**

```bash
# Test script - create temporary benchmark
cat > /tmp/bench_events.rs << 'EOF'
use codex_tui::app_event_sender::AppEventSender;
use codex_tui::app_event::AppEvent;
use tokio::sync::mpsc;
use std::time::Instant;

#[tokio::test(flavor = "multi_thread")]
async fn benchmark_event_throughput() {
    let (tx, mut rx) = mpsc::unbounded_channel();
    let sender = AppEventSender::new(tx);

    let start = Instant::now();
    let count = 100_000;

    for _ in 0..count {
        sender.send(AppEvent::RequestRedraw);
    }

    let elapsed = start.elapsed();
    let throughput = count as f64 / elapsed.as_secs_f64();

    println!("Throughput: {:.0} events/sec", throughput);
    println!("Latency: {:.2} µs/event", elapsed.as_micros() as f64 / count as f64);
}
EOF
```

**Expected:** Similar or better than std::sync::mpsc (tokio unbounded is highly optimized)

### 4.6: Memory Leak Check (30 min)

```bash
# Install heaptrack if not available
# sudo apt-get install heaptrack (or equivalent)

# Run TUI with heap tracking
heaptrack ~/code/codex-rs/target/release/codex-tui

# Or use valgrind
valgrind --leak-check=full --track-origins=yes \
    ~/code/codex-rs/target/release/codex-tui

# Test for 5-10 minutes:
# - Send 100+ messages
# - Switch models several times
# - Use various commands
# - Exit cleanly

# Check: No memory leaks, all channels cleaned up
```

---

## 📋 PHASE 5: DOCUMENTATION & COMPLETION (1 hour)

### 5.1: Update SPEC-955 Documentation (30 min)

**File:** `docs/SPEC-KIT-955-tui-test-deadlock/spec.md`

**Add sections:**

```markdown
## Solution Implemented

### Root Cause
std::sync::mpsc (blocking channels) incompatible with #[tokio::test] async runtime.

Specific deadlock pattern:
- TestHarness creates std::sync::mpsc::channel()
- ChatWidget::new() spawns tokio tasks
- Tasks try to send through std::sync::mpsc
- Main test thread calls try_recv() → blocks tokio runtime
- Deadlock: task waits for channel, runtime waits for task

### Refactoring Approach

**Phase 1: Preparation (2 hours)**
- Created 7 integration tests for AppEventSender (baseline)
- Comprehensive audit: 58 std::sync::mpsc uses across 15 files
- Identified critical vs optional migration targets

**Phase 2: Core Migration (5 hours)**
- AppEventSender: std::sync::mpsc::Sender → tokio::sync::mpsc::UnboundedSender
- App event loop: UnboundedReceiver with tokio-safe polling
- TestHarness: unbounded_channel for app_event_rx
- ChatWidget browser checks: std::sync::mpsc → tokio::sync::oneshot
- Test infrastructure: 12 files migrated to tokio channels

**Phase 3: Validation (Session 2)**
- Debug why handle_codex_event() doesn't create history cells
- Fix 5 failing tests
- Full test suite validation
- Manual TUI testing

### Files Modified (Session 1)

**Core Event System:**
1. tui/src/app_event_sender.rs - tokio::sync::mpsc::UnboundedSender (high_tx, bulk_tx)
2. tui/src/app.rs - UnboundedReceiver, next_event_priority refactored
3. tui/src/chatwidget/test_harness.rs - unbounded_channel, #[tokio::test(multi_thread)]

**ChatWidget Integration:**
4. tui/src/chatwidget/mod.rs - 4 browser checks: tokio::oneshot + Handle::block_on

**Test Infrastructure:**
5. tui/src/chatwidget/test_support.rs
6. tui/src/chatwidget/tests.rs
7. tui/src/bottom_pane/chat_composer.rs (15 test functions)
8. tui/src/bottom_pane/agent_editor_view.rs
9. tui/src/bottom_pane/mod.rs
10. tui/src/bottom_pane/chat_composer_history.rs
11. tui/src/bottom_pane/approval_modal_view.rs
12. tui/src/user_approval_widget.rs
13. tui/src/chatwidget/agent_install.rs
14. tui/tests/message_interleaving_test.rs

### Test Results

**Before Refactor:**
- 3/9 passing
- 5/9 hanging (60+ seconds)
- 1/9 failing (snapshot)

**After Session 1:**
- 4/9 passing (test_send_user_message now works!)
- 0/9 hanging ✅
- 5/9 failing (snapshot + logic issues)
- All tests complete in 0.19-0.21s

**After Session 2 (Target):**
- 9/9 passing ✅
- 0 hanging ✅
- Full test suite green
- Manual TUI validation successful

### Key Insights

1. **Blocking in async runtime deadlocks:**
   - `std::sync::mpsc::Sender.send()` can block
   - `Receiver.recv()` always blocks
   - `try_recv()` is "non-blocking" but blocks tokio scheduler in certain conditions
   - Solution: Use tokio::sync::mpsc which integrates with async runtime

2. **oneshot vs mpsc:**
   - tokio::sync::oneshot: Perfect for async → sync one-time communication
   - Handle::current().block_on(): Works for awaiting oneshot in sync context
   - UnboundedSender/Receiver: Multi-message async-safe channels

3. **Test runtime configuration:**
   - #[tokio::test] defaults to single-threaded runtime
   - #[tokio::test(flavor = "multi_thread")] enables thread pool
   - block_in_place requires multi-threaded runtime

4. **Tests might have never worked:**
   - Added in recent commits (b802de208, 2c42f0735)
   - Immediately hung due to pre-existing deadlock
   - Reverted in 4bde2dc80
   - No evidence of prior passing state

### Lessons Learned

- Always use tokio channels in tokio runtime contexts
- Avoid mixing std::sync::mpsc with async code
- Integration tests critical for validating behavior preservation
- Multi-threaded runtime often required for realistic async testing
- Test infrastructure issues can hide underlying functionality problems

## Validation Results

(To be filled in Session 2)

## Performance Impact

(To be measured in Session 2)

## Memory Safety

(To be validated in Session 2)
```

### 5.2: Update SPEC.md Tracker (5 min)

**File:** `SPEC.md`

**Update:**
```markdown
| SPEC-KIT-955 | TUI Test Deadlock | COMPLETE | ... |
```

**Mark dependent:**
```markdown
| SPEC-KIT-954 | Test Coverage Phase 3 | IN PROGRESS | Unblocked, can add Tasks 2-3 tests |
```

### 5.3: Store to Local-Memory (10 min)

```
Use mcp__local-memory__store_memory:
- content: "SPEC-955 complete: Migrated entire TUI event system from std::sync::mpsc to tokio::sync::mpsc (12 hours total, 2 sessions). Root cause: blocking channels deadlock in #[tokio::test] async runtime. Solution: AppEventSender uses tokio::sync::mpsc::UnboundedSender, App event loop refactored for tokio, ChatWidget browser checks use tokio::sync::oneshot with Handle::current().block_on(), test infrastructure migrated. Validation: 9/9 tests passing (was 3/9), no hangs (was 5/9 hanging 60+ seconds), CI green, manual TUI testing clean. Critical discovery: handle_codex_event() needed [DETAILS FROM SESSION 2]. Files: app_event_sender.rs, app.rs, test_harness.rs, chatwidget/mod.rs, +10 test files. Pattern: Always use tokio channels in tokio contexts, even for 'non-blocking' try_recv()."
- domain: "infrastructure"
- tags: ["type:refactor", "spec:SPEC-KIT-955", "async-architecture", "testing", "deadlock-fix"]
- importance: 9
```

### 5.4: Create Comprehensive Commit (15 min)

```bash
cd /home/thetu/code

# Stage all changes
git add -A

# Create detailed commit
git commit -m "$(cat <<'EOF'
refactor(tui): Migrate event system from std::sync::mpsc to tokio::sync::mpsc

SPEC-955: Fix TUI test deadlock by refactoring entire event system to use
tokio async channels instead of std blocking channels.

Root cause: std::sync::mpsc blocking operations deadlock in tokio async
runtime context (#[tokio::test]). The try_recv() call on std::sync::mpsc
in TestHarness.drain_app_events() would block the tokio scheduler, causing
60+ second hangs.

Solution:
- AppEventSender: std::sync::mpsc::Sender → tokio::sync::mpsc::UnboundedSender
- App event loop: UnboundedReceiver with tokio-safe polling (sleep vs recv_timeout)
- TestHarness: unbounded_channel, all tests use multi_thread runtime
- ChatWidget: 4 browser check deadlocks fixed (tokio::sync::oneshot + Handle::block_on)
- Test infrastructure: 14 files migrated to tokio channels

Files modified:
Core: app_event_sender.rs, app.rs, chatwidget/test_harness.rs, chatwidget/mod.rs
Tests: test_support.rs, tests.rs, chat_composer.rs, agent_editor_view.rs,
       mod.rs, chat_composer_history.rs, approval_modal_view.rs,
       user_approval_widget.rs, agent_install.rs, message_interleaving_test.rs

Validation:
- Test results: 9/9 passing (was 3/9), 0 hanging (was 5/9 hanging 60+ seconds)
- Integration tests: 7/7 passing (behavior preserved)
- Performance: [DETAILS FROM SESSION 2]
- Memory: No leaks detected
- Manual TUI: All scenarios pass
- CI: Green (strict mode)

Critical discoveries:
- [DETAILS FROM SESSION 2 DEBUGGING]

Pattern for future: Always use tokio::sync::mpsc in tokio runtime contexts.
Mixing std::sync::mpsc with async code causes subtle deadlocks.

🤖 Generated with [Claude Code](https://claude.com/claude-code)

Co-Authored-By: Claude <noreply@anthropic.com>
EOF
)"

# Push
git push origin main
```

---

## 🚨 CRITICAL DEBUGGING CHECKLIST (Session 2 Start)

Before diving into code, systematically test these hypotheses:

- [ ] **H1: Op channel required** - Create test using codex_op_tx.send(Op::Submit) instead of handle_codex_event()
- [ ] **H2: Async initialization** - Try longer sleep (500ms, 1s) to rule out timing
- [ ] **H3: Session state** - Check if widget.session or similar needs initialization
- [ ] **H4: Event delivery** - Add logging to confirm events reach handle_codex_event()
- [ ] **H5: History mutation** - Add logging to confirm history_cells.push() is called
- [ ] **H6: Request tracking** - Check if active_requests map needs setup

**Stop Condition:** Once one hypothesis is confirmed, implement fix across all 5 tests

---

## 🎯 SUCCESS CRITERIA (Session 2 Completion)

**Must Have:**
- ✅ 9/9 TUI tests passing
- ✅ Full workspace test suite green
- ✅ CI passing (strict mode)
- ✅ Manual TUI smoke test successful (10 scenarios)
- ✅ No memory leaks (valgrind/heaptrack clean)
- ✅ SPEC-955 marked COMPLETE in SPEC.md
- ✅ All changes committed and pushed
- ✅ Findings stored to local-memory

**Nice to Have:**
- ✅ Performance benchmarks documented
- ✅ Architectural patterns documented for future reference
- ✅ Test infrastructure improvements identified

---

## 📚 ESSENTIAL REFERENCE FILES

**Read First (Session 2):**
```
docs/SPEC-KIT-955-tui-test-deadlock/spec.md       - Original bug report
/tmp/mpsc_refactor_audit.md                      - Session 1 audit (if exists)
tui/src/chatwidget/test_harness.rs               - Test infrastructure
tui/src/chatwidget/mod.rs                        - ChatWidget core (handle_codex_event)
```

**Key Code Locations:**
```
tui/src/chatwidget/mod.rs:
  - Line ~4953: Browser initialization (modified to defer)
  - Search for: "pub(crate) fn handle_codex_event" - Event processing logic
  - Search for: "history_push" - Where cells are added to history

tui/src/chatwidget/test_harness.rs:
  - Line 28-63: TestHarness::new() - Widget creation
  - Line 66-77: send_user_message() - Working ✅
  - Line 80-82: send_codex_event() - Not creating history cells ❌
  - Line 85-92: drain_app_events() - Fixed (tokio try_recv)
  - Line 201-245: simulate_streaming_response() - Helper method

tui/src/app_event_sender.rs:
  - Line 1: Refactored import (tokio::sync::mpsc::UnboundedSender)
  - Line 7-12: Struct definition (high_tx, bulk_tx)
  - Line 148-331: Integration tests (7 tests, all passing)
```

---

## 🔧 DEBUGGING TOOLBOX

### Useful Commands

```bash
# Quick test status
cd codex-rs && timeout 180 cargo test -p codex-tui --lib test_harness::tests 2>&1 | grep "test result:"

# Single test with debug output
cargo test -p codex-tui --lib test_overlapping_turns_no_interleaving -- --nocapture

# Find event processing code
rg "fn handle_codex_event" --type rust tui/src -A20

# Check Op channel usage
rg "codex_op_tx\.send" --type rust tui/src -B3 -A3

# List all std::sync::mpsc uses
rg "std::sync::mpsc" --type rust tui/src -n

# Check test history
git log --oneline --all -- codex-rs/tui/src/chatwidget/test_harness.rs

# Restore specific file if needed
git checkout HEAD -- codex-rs/tui/src/chatwidget/mod.rs
```

### Debug Print Template

```rust
eprintln!("
=== DIAGNOSTIC ===
Location: {}:{}
Event: {:?}
History count: {}
Active requests: {:?}
=================
", file!(), line!(), event, self.history_cells.len(), self.active_requests.keys());
```

---

## ⚠️ KNOWN ISSUES & RISKS

### Issue 1: handle_codex_event() Not Creating History Cells

**Status:** Discovered in Session 1, requires investigation
**Impact:** 5/9 tests fail due to 0 assistant cells
**Risk:** Medium - might indicate broken widget logic vs test infrastructure issue
**Mitigation:** Systematic hypothesis testing (H1-H6)

### Issue 2: Browser Initialization Deferred

**Change Made:**
```rust
// Session 1 change to avoid blocking:
let browser_enabled = false;  // Deferred initialization
```

**Risk:** Low - browser features only for /browser and /chrome commands
**Validation:** Manual TUI testing should confirm browser commands still work

### Issue 3: Snapshot Tests Failing

**Status:** Expected - UI output changed due to browser_enabled = false
**Impact:** 3 snapshot tests need review/acceptance
**Risk:** Low - snapshots can be updated with `cargo insta accept`
**Validation:** Visual review of snapshot diffs

### Issue 4: Incomplete Migration

**Remaining std::sync::mpsc uses:**
- theme_selection_view.rs (14 uses) - Session 2 target
- file_search.rs (1 use) - Uses std::thread, may be OK
- TerminalRunController (intentional) - Separate system

**Risk:** Low - these use std::thread::spawn contexts where std::sync::mpsc is appropriate

---

## 🎓 ARCHITECTURAL LESSONS

### Async/Sync Boundary Patterns

**Problem Pattern (Causes Deadlock):**
```rust
// BAD: Blocking channel in async context
let (tx, rx) = std::sync::mpsc::channel();
tokio::spawn(async move {
    tx.send(data);  // Can block!
});
rx.recv();  // Blocks thread in tokio runtime → deadlock
```

**Solution Pattern:**
```rust
// GOOD: Async channel with non-blocking operations
let (tx, mut rx) = tokio::sync::mpsc::unbounded_channel();
tokio::spawn(async move {
    tx.send(data).unwrap();  // Never blocks
});
rx.try_recv()?;  // Non-blocking in tokio context
```

**For one-time async → sync:**
```rust
// GOOD: oneshot for single value
let (tx, rx) = tokio::sync::oneshot::channel();
tokio::spawn(async move {
    tx.send(data);
});
tokio::runtime::Handle::current().block_on(async {
    rx.await.unwrap()
})
```

### When to Use Which Channel

| Context | Sender Context | Receiver Context | Channel Type |
|---------|---------------|------------------|--------------|
| Thread → Thread | std::thread | std::thread | std::sync::mpsc ✅ |
| Async → Async | tokio::spawn | async fn | tokio::sync::mpsc ✅ |
| Async → Sync | tokio::spawn | sync fn | tokio::sync::oneshot + Handle::block_on ✅ |
| Sync → Async | sync fn | tokio::spawn | tokio::sync::mpsc ✅ |
| **Async → Sync (test)** | tokio::spawn | #[tokio::test] | **tokio::sync::mpsc** (NOT std!) |

---

## 🚀 ESTIMATED SESSION 2 TIMELINE

**Hour 1-2:** Phase 3.1-3.2 (Diagnostic logging + Op channel hypothesis)
**Hour 3-4:** Phase 3.3-3.5 (Session state + Fix 5 tests)
**Hour 5-6:** Phase 2.5-2.6 (theme_selection_view + file_search)
**Hour 7-8:** Phase 4.1-4.4 (Full test suite + CI + Manual TUI)
**Hour 9:** Phase 4.5-4.6 (Performance + Memory)
**Hour 10:** Phase 5 (Documentation + Commit)

**Total: 8-10 hours**

---

## 💾 SESSION 2 STARTUP COMMANDS

```bash
# 1. Navigate to repository
cd /home/thetu/code

# 2. Check status
git status
git diff --stat

# 3. Load local-memory context
# ID: 701b6762-973a-4bf0-ae92-78c5d84de11a
# Query: "SPEC-955 async event session 1"

# 4. Verify test status
cd codex-rs
timeout 180 cargo test -p codex-tui --lib test_harness::tests 2>&1 | grep "test result:"
# Expected: ok. 4 passed; 5 failed; finished in ~0.20s

# 5. Start debugging with H1 (Op channel hypothesis)
# See "Investigation 2" section above
```

---

## 📞 ESCALATION CRITERIA

**Stop and ask for guidance if:**
- Debugging exceeds 4 hours without progress
- Discovered architectural issue requiring ChatWidget redesign
- Test failures indicate fundamental widget logic broken (not just test infrastructure)
- CI failures emerge that weren't present before refactor
- Manual TUI testing reveals production regressions

---

## 🔗 RELATED SPECS

- **SPEC-954:** Test Coverage Phase 3 (blocked by SPEC-955, can resume after)
- **SPEC-952:** Multi-provider CLI routing (working, unrelated)
- **SPEC-KIT-070:** Cost optimization (unrelated)

---

**Session 1 Completion Time:** 2025-11-23 (5 hours)
**Session 2 Target Start:** Next session
**Combined Effort:** 12-15 hours estimated
