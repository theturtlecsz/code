**SPEC-ID**: SPEC-KIT-955
**Feature**: TUI Test Infrastructure Deadlock Investigation & Fix
**Status**: Backlog
**Created**: 2025-11-23
**Priority**: P1 - HIGH
**Type**: Bug Fix - Critical Testing Infrastructure
**Blocks**: SPEC-KIT-954 (Tasks 2-3), All future TUI testing expansion

---

## Problem Statement

**Symptom**: 5/9 TUI tests hang indefinitely (>60s timeout) when running test suite.

**Impact**:
- Cannot validate TUI behavior with automated tests
- Blocks SPEC-954 completion (Tasks 2-3 require working tests)
- Prevents test suite expansion
- CI cannot validate TUI changes

**Severity**: CRITICAL - Testing infrastructure completely broken

---

## Investigation Findings (2025-11-23)

### Tests That Hang (>60s timeout):
1. `test_overlapping_turns_no_interleaving`
2. `test_send_user_message`
3. `test_three_overlapping_turns_extreme_adversarial`
4. `test_chatwidget_single_exchange_snapshot`
5. `test_chatwidget_two_turns_snapshot`

### Tests That Pass (<1s):
1. `test_harness_creation` (0.05s)
2. `test_simulate_streaming_response`
3. `test_history_cells_debug`

### Pattern Identified:

**Working tests**:
```rust
let mut harness = TestHarness::new();
harness.simulate_streaming_response(...); // ← Uses helper
// NO drain_app_events() call
let debug = harness.history_cells_debug(); // ← Works fine
```

**Hanging tests**:
```rust
let mut harness = TestHarness::new();
harness.send_codex_event(...); // ← Manual event sending
harness.drain_app_events(); // ← Suspected deadlock point
let debug = harness.history_cells_debug(); // ← Never reaches here
```

### Root Cause Hypotheses:

**1. Async/Sync Boundary Issue** (Most Likely):
- TestHarness::new() spawns ChatWidget with background tasks
- handle_codex_event() may trigger async operations
- drain_app_events() or subsequent state access deadlocks waiting for async task

**2. Channel Deadlock**:
- send_codex_event() → handle_codex_event() may send to app_event_tx
- drain_app_events() tries to receive from app_event_rx
- If widget is blocked waiting to send, and test is blocked waiting to receive → deadlock

**3. Tokio Runtime Conflict**:
- Tests run in tokio::test runtime
- ChatWidget may create its own runtime or spawn tasks
- Runtime nesting or task scheduling conflict

---

## Scope

### In Scope:
- Root cause identification (async/sync boundaries, channels, tokio)
- Fix deadlock in test infrastructure
- Verify all 9 tests pass
- Add regression test for the fix
- Document async testing patterns for future

### Out of Scope:
- Production TUI changes (unless required for fix)
- New test features beyond fixing existing tests
- Performance optimization (focus on correctness)

---

## Acceptance Criteria

- [ ] All 9 existing tests pass without timeout
- [ ] Test suite completes in <30s total
- [ ] No async/tokio warnings or errors
- [ ] Can add new tests without deadlock
- [ ] Documented fix and testing patterns

---

## Estimated Effort

**Investigation**: 2-3 hours (async debugging, tokio profiling)
**Fix Implementation**: 2-3 hours (depending on root cause)
**Testing & Validation**: 1-2 hours
**Documentation**: 1 hour

**Total**: 6-9 hours (1-1.5 days)

---

## Dependencies

**Blocked By**: None
**Blocks**:
- SPEC-KIT-954 (Tasks 2-3)
- Future TUI test expansion
- CI test automation

---

## Files Involved

**Test Infrastructure**:
- `codex-rs/tui/src/chatwidget/test_harness.rs` (1,017 LOC, 9 tests)
- `codex-rs/tui/src/chatwidget/mod.rs` (handle_codex_event)

**Potential Fixes**:
- Tokio runtime configuration in tests
- Channel buffer sizes or async boundaries
- Event processing sync/async separation

---

## Notes

**Discovery Context**: Found during SPEC-954 when attempting to add automated tests for Tasks 2-3. Initially appeared to be new regression, investigation revealed pre-existing bug from previous session (commit 54f76a6f2).

**Previous Session Claim**: "72 tests passing locally ✅" - **FALSE**. Tests were not actually run or different subset tested.

**Workaround**: Use `simulate_streaming_response()` helper instead of manual event sending.

---

---

## Session 1 Progress (2025-11-23, 5 hours)

### ✅ Major Achievement: Deadlock FIXED

**Primary Objective Achieved:**
- 0/9 tests hanging (was 5/9 hanging 60+ seconds) ✅
- All tests complete in 0.19-0.21s ✅
- Root cause identified and partially resolved ✅

### Root Cause Analysis

**THE BUG:**
```rust
// TestHarness (BEFORE):
use std::sync::mpsc;  // Blocking channel
let (app_tx_raw, app_rx) = mpsc::channel();  // In tokio context!

#[tokio::test]  // Async runtime
async fn test_send_user_message() {
    let harness = TestHarness::new();  // Creates std::sync::mpsc
    harness.drain_app_events();  // try_recv() blocks tokio → DEADLOCK
}
```

**WHY IT DEADLOCKS:**
1. std::sync::mpsc is designed for thread-to-thread communication
2. In tokio runtime, try_recv() can block the scheduler
3. ChatWidget spawns tokio tasks that send to the channel
4. Main test thread blocks waiting for events
5. Tokio runtime can't schedule the tasks → deadlock

**THE FIX:**
```rust
// TestHarness (AFTER):
use tokio::sync::mpsc;  // Async channel
let (app_tx_raw, app_rx) = mpsc::unbounded_channel();  // Tokio-safe!

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]  // Multi-thread
async fn test_send_user_message() {
    let harness = TestHarness::new();  // Creates tokio::sync::mpsc
    harness.drain_app_events();  // tokio try_recv() - no deadlock ✅
}
```

### Files Refactored (14 files)

**Core Event System:**
1. `tui/src/app_event_sender.rs` - tokio::sync::mpsc::UnboundedSender
2. `tui/src/app.rs` - UnboundedReceiver, event loop refactored
3. `tui/src/chatwidget/test_harness.rs` - unbounded_channel, multi_thread tests
4. `tui/src/chatwidget/mod.rs` - 4 browser check deadlocks fixed (tokio::oneshot)

**Test Infrastructure:**
5-14. Multiple test files migrated to tokio channels

### Test Results

**Before:** 3/9 passing, 5/9 hanging, 1/9 failing
**After Session 1:** 4/9 passing, 0/9 hanging, 5/9 failing
**Progress:** +1 passing, -5 hanging, +4 failing (tests fail fast instead of hang)

### Critical Discovery

**UNEXPECTED:** Even "passing" test has 0 assistant cells!

```bash
$ cargo test test_simulate_streaming_response -- --nocapture

Assistant cells: 0
test ... ok  # Passes because it only checks !debug.is_empty()
```

**Implication:** handle_codex_event() not creating history cells at all
**Next Session Focus:** Debug event processing logic

### Session 1 Deliverables

- ✅ Comprehensive audit: 58 std::sync::mpsc uses across 15 files
- ✅ 7 integration tests added (AppEventSender baseline validation)
- ✅ Core refactor complete (AppEventSender, App, TestHarness)
- ✅ ChatWidget deadlocks eliminated (tokio::oneshot pattern)
- ✅ Test infrastructure 90% migrated
- ✅ All changes compile successfully
- ⚠️ Test failures require Session 2 investigation

### Remaining Work (Session 2)

**Phase 2 (Complete Refactoring):**
- theme_selection_view.rs (14 uses)
- file_search.rs (1 use, evaluation needed)

**Phase 3 (Debug Test Failures):**
- Investigate why handle_codex_event() doesn't create history cells
- Test 6 hypotheses (Op channel, async init, session state, etc.)
- Fix all 5 failing tests

**Phase 4 (Validation):**
- 9/9 tests passing
- Full workspace test suite
- Manual TUI smoke testing
- Performance + memory validation

**Phase 5 (Completion):**
- Documentation
- Single comprehensive commit
- Local-memory storage

**Estimated Time:** 8-10 hours

---

**SPEC-KIT-955: TUI Test Deadlock - Critical Infrastructure Bug**
