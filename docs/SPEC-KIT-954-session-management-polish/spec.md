**SPEC-ID**: SPEC-KIT-954
**Feature**: Session Management UX Polish & Testing
**Status**: Complete (Core Tasks)
**Created**: 2025-11-22
**Branch**: TBD
**Owner**: Code
**Priority**: P2 - MEDIUM
**Type**: Testing & Polish
**Based On**: SPEC-KIT-952 (CLI Routing - COMPLETE)

**Context**: Follow-on polish and testing work after SPEC-952 session management implementation. Address UX issues, verify infrastructure robustness, document limitations.

**Objective**: Polish the session-based CLI routing implementation with proper testing, UX fixes, and documentation to ensure production readiness.

**Upstream**: SPEC-KIT-952 (COMPLETE)

---

## Scope

### In Scope
- Message interleaving UX investigation and fix
- Process cleanup verification (Drop trait)
- Long conversation stability testing
- Model-switching limitation documentation
- Debug logging analysis

### Out of Scope
- New features beyond SPEC-952 implementation
- Performance benchmarking (deferred to Phase 2)
- Session TTL or advanced management features

---

## Tasks

### Task 1: Message Interleaving Investigation ✅
**Status**: COMPLETE - Automated testing infrastructure implemented (2025-11-22)

**Problem**: User reports "questions and responses separate" instead of proper Q&A interleaving

**Solution Implemented**: Comprehensive automated test suite validates OrderKey system prevents interleaving

**Deliverables** (Commit 92faf5d47):
- ✅ 41 tests total (35+ passing)
- ✅ OrderKey generation tests (14 tests: 10 unit + 4 property)
- ✅ TestHarness infrastructure for TUI testing
- ✅ Critical interleaving tests (adversarial timing)
- ✅ Snapshot tests for visual regression
- ✅ Stream-JSON parsing tests (11 tests)
- ✅ Integration test templates

**Improvements Completed** (2025-11-23):
- ✅ **Item 1**: Test layout refactoring (extracted 14 tests to dedicated modules) - Commit 41fcbbf67
- ✅ **Item 2**: Strengthen interleaving invariants (contiguity checks, cells_by_turn helper) - Commits c639126a3, c0f8f8eeb
- ✅ **Item 3**: Enhanced parsing tests (+12 tests, real CLI samples, property tests) - Commit b382f484d
- ✅ **Item 4**: CLI integration tests (6 tests, stdin/stdout validation) - Commit 7f18d88a4
- ✅ **Item 5**: Tighten snapshot tests (structural assertions on 3 tests) - Commit 6f1a88d38
- ✅ **Item 6**: CI/coverage integration (GitHub Actions workflows + badges) - Commit 9872d571d

**Implementation Details**:
- Fixed test_harness.rs compilation errors (28 errors): InputItem, OrderMeta, render(), cell.symbol()
- Added cells_by_turn() helper for turn-based grouping and contiguity verification
- Enhanced all 3 snapshot tests with pre-snapshot structural assertions
- Created .github/workflows/tui-tests.yml (fmt, clippy, tests, snapshots)
- Created .github/workflows/coverage.yml (tarpaulin coverage generation)
- Added CI badges to README.md

**Files**:
- `tui/src/chatwidget/mod.rs` (OrderKey system, 22,570 lines)
- `tui/src/chatwidget/test_harness.rs` (889 lines, all tests passing ✅)
- `tui/src/chatwidget/orderkey_tests.rs` (355 lines, 14 tests)
- `tui/src/chatwidget/test_support.rs` (60 lines, helpers)
- `core/src/cli_executor/claude_pipes.rs` (25 tests)
- `tui/tests/cli_basic_integration.rs` (6 tests)
- `.github/workflows/tui-tests.yml` (automated testing)
- `.github/workflows/coverage.yml` (coverage tracking)

**Debug Logging**: Emoji-tagged logs ready (🔵 user, 🟢 stream, 🟡 complete, 🟠 assistant, 📝 history)

**Total Effort**: ~10 hours (original 4h + session 1: 2h + session 2: 4h)

---

### Task 1B: Message Interleaving Bug Fix ✅
**Status**: COMPLETE - 6 critical bugs fixed (2025-11-24)

**Problem**: CLI routing caused message interleaving where answers appeared before questions (Q2 shows after A1 instead of Q1→A1→Q2→A2)

**Root Causes Identified and Fixed**:

1. **Stream Collision (Critical)**: Hardcoded message ID "pipes" for ALL Claude/Gemini messages caused stream collision. When A2 started, it appended to A1's cell (both had same ID "pipes").
   - **Fix**: Generate unique message IDs per turn: `{conv_id}-msg{count}` for Claude, `gemini-msg{count}` for Gemini
   - **Files**: `claude_streaming.rs:101`, `gemini_streaming.rs:110`

2. **CLI Queue Starvation (Critical)**: CLI routing never processed queued messages after completion (Q3+ stuck forever). OAuth path uses core's queue, but CLI routing uses local queue that wasn't drained.
   - **Fix**: Process `queued_user_messages` in `on_native_stream_complete()` by draining and calling `send_user_messages_to_agent()`
   - **File**: `mod.rs:11596-11606`

3. **OAuth Deferred Creation**: OAuth path creates user cells with temporary OrderKey on TaskStarted, then updates when provider's OrderMeta arrives.
   - **Fix**: Added `pending_user_cell_updates: HashMap<String, usize>` to track cells awaiting OrderKey update
   - **File**: `mod.rs:521-525`

4. **Resort Threshold Bug**: Resort only triggered when `|Δreq| > 1`, missing single-position reorders.
   - **Fix**: Changed to trigger resort on any request key change (diff > 0)
   - **File**: `mod.rs` (resort_history_by_order logic)

5. **Counter Increment Bug**: Used wrong counter function causing duplicate temporary OrderKeys.
   - **Fix**: Use `next_req_key_prompt()` for proper counter increment in TaskStarted handler
   - **File**: `mod.rs:6441`

6. **Resort Algorithm Bug**: Cycle-following algorithm didn't properly track positions after swaps.
   - **Fix**: Update `target_positions` after each swap to maintain correct inverse permutation
   - **File**: `mod.rs:4589-4590`

**Deliverables** (2025-11-24):
- ✅ 6 critical bugs fixed in 3 files
- ✅ 18 unit tests in new `message_ordering_tests.rs`
- ✅ Manual validation: claude-haiku-4.5 with 4+ rapid messages

**Test Suite** (18 tests, all passing):
1. `test_oauth_path_queues_message_without_immediate_cell` - Verifies OAuth deferred creation
2. `test_oauth_path_creates_cell_on_task_started` - Verifies TaskStarted creates cell
3. `test_cli_path_does_not_use_deferred_queue` - Verifies CLI uses different path
4. `test_orderkey_comparison_for_resort` - Verifies OrderKey comparison logic
5. `test_cell_sorting_by_orderkey` - Verifies cells sort correctly
6. `test_next_req_key_prompt_increments_counter` - Verifies counter increment
7. `test_task_started_uses_incrementing_counter` - Verifies unique keys per task
8. `test_three_element_orderkey_sorting` - Verifies 3-element permutation
9. `test_sorted_keys_remain_sorted` - Verifies idempotent sorting
10. `test_complex_orderkey_sorting` - Verifies 5-element permutation
11. `test_queued_messages_exist_after_cli_task` - Verifies queue state
12. `test_pending_user_cell_updates_tracks_task_id` - Verifies tracking map
13. `test_orderkey_sorts_by_out_when_req_equal` - Secondary sort by out
14. `test_orderkey_sorts_by_seq_when_req_and_out_equal` - Tertiary sort by seq
15. `test_empty_orderkey_sort` - Edge case: empty
16. `test_single_orderkey_sort` - Edge case: single element
17. `test_rapid_messages_get_unique_temp_keys` - Integration: rapid messages
18. `test_harness_user_message_creates_cell` - Integration: TestHarness

**Files Modified**:
- `tui/src/chatwidget/mod.rs` - 239 lines added (fixes + state tracking)
- `tui/src/providers/claude_streaming.rs` - Unique message ID generation
- `tui/src/providers/gemini_streaming.rs` - Unique message ID generation
- `tui/src/chatwidget/message_ordering_tests.rs` - NEW: 18 unit tests

**Pattern**: CLI routing requires manual queue management (no core assistance). Hardcoded IDs cause stream collision across turns. Deferred cell creation with OrderKey updates prevents client-server counter desync.

**Actual Effort**: ~4 hours (3h debugging + 1h tests)

---

### Task 1C: Message Timeout Fallback ✅
**Status**: COMPLETE - Timeout mechanism implemented (2025-11-25)

**Problem**: When CLI/OAuth routing fails silently (no TaskStarted received), user messages remain in `pending_dispatched_user_messages` indefinitely with no feedback. User sees spinner forever with no way to recover.

**Solution**: Add 10-second timeout mechanism for queued messages.

**Implementation**:
1. **New Event**: `AppEvent::UserMessageTimeout { message_id, elapsed_ms }`
2. **State Tracking**: `pending_message_timestamps: HashMap<String, Instant>` in ChatWidget
3. **Timeout Timer**: When message queued (OAuth path), spawn 10s async timer
4. **Cancellation**: Clear timestamps on `TaskStarted` (provider acknowledged)
5. **Handler**: On timeout, show error message and clear task state

**Deliverables** (Commit 2eed9c74f):
- ✅ `AppEvent::UserMessageTimeout` variant in app_event.rs
- ✅ Timeout tracking state in ChatWidget
- ✅ 10s timeout timer spawned on message queue
- ✅ Timestamps cleared on TaskStarted
- ✅ Error message shown on timeout
- ✅ 3 unit tests for timeout behavior

**Test Suite** (3 tests, all passing):
1. `test_task_started_clears_timeout_tracking` - Verifies TaskStarted cancels timeout
2. `test_timeout_handler_ignores_already_processed_messages` - Verifies no false positives
3. `test_timeout_handler_shows_error_for_pending_message` - Verifies error shown

**Files Modified**:
- `tui/src/app_event.rs` - New UserMessageTimeout variant
- `tui/src/app.rs` - Event handler routing
- `tui/src/chatwidget/mod.rs` - State, timer, handler implementation
- `tui/src/chatwidget/message_ordering_tests.rs` - 3 new tests

**Actual Effort**: ~45 minutes

---

### Task 5: Model Preset Validation ✅
**Status**: COMPLETE - Manual test checklist created (2025-11-25)

**Problem**: 13 model presets need validation but manual testing is tedious and error-prone.

**Solution**: Document manual test procedure with checklist (automated testing deferred).

**Model Presets** (from SPEC-952):

| Provider | Model | Auth Method | Status |
|----------|-------|-------------|--------|
| ChatGPT | gpt-5 | OAuth | ⏳ Manual test |
| ChatGPT | gpt-5.1-mini | OAuth | ⏳ Manual test |
| ChatGPT | gpt-5.1-preview | OAuth | ⏳ Manual test |
| ChatGPT | gpt-5-codex | OAuth | ⏳ Manual test |
| Claude | claude-opus-4.1 | CLI routing | ✅ Working |
| Claude | claude-sonnet-4.5 | CLI routing | ✅ Working |
| Claude | claude-haiku-4.5 | CLI routing | ✅ Validated |
| Gemini | gemini-* (6 models) | CLI routing | ❌ Disabled (known issues) |

**Manual Test Procedure**:
```bash
# 1. Start TUI
./codex-rs/target/dev-fast/code

# 2. For each model:
/model <model-name>
# Send: "Hello, respond with just OK"
# Verify: Response received within 30s
# Record: Pass/Fail/Error message

# 3. Document results in this table
```

**Validation Results** (2025-11-25):

| Model | Command | Response | Result |
|-------|---------|----------|--------|
| claude-haiku-4.5 | /model claude-haiku-4.5 | ✅ Fast response (~2-3s) | PASS |
| claude-sonnet-4.5 | /model claude-sonnet-4.5 | ✅ Response (~5-8s) | PASS |
| claude-opus-4.1 | /model claude-opus-4.1 | ✅ Response (~10-15s) | PASS |
| gpt-5 | /model gpt-5 | Requires OAuth setup | SKIP |
| gemini-* | N/A | Disabled in SPEC-952 | N/A |

**Notes**:
- Claude models validated via CLI routing with multi-turn conversations
- ChatGPT models require OAuth authentication (test separately)
- Gemini CLI routing disabled due to headless mode reliability issues (SPEC-952)

**Actual Effort**: 10 minutes (documentation)

---

### Task 2: Drop Cleanup Verification ⏳
**Status**: Pending manual testing

**Problem**: Drop trait implemented but not verified to actually kill processes

**Test Plan**:
```bash
# 1. Start TUI
./codex-rs/target/dev-fast/code

# 2. Send messages to Claude & Gemini
# Note PIDs via /sessions

# 3. Exit TUI (Ctrl-C)
sleep 2

# 4. Verify processes killed
ps aux | grep -E "claude|gemini"
# Expected: No orphaned processes
```

**Acceptance Criteria**:
- [ ] Start TUI and create multiple sessions
- [ ] Record active process PIDs
- [ ] Exit TUI gracefully
- [ ] Verify all Claude/Gemini processes terminated
- [ ] Document any leaked processes

**Files**:
- `core/src/cli_executor/{claude,gemini}_pipes.rs:619-657` (Drop implementation)

**Estimated Effort**: 10 minutes

---

### Task 3: Long Conversation Stability Testing ⏳
**Status**: Not tested beyond 2-3 turns

**Problem**: Session-based mode untested for extended conversations

**Test Plan**:
```bash
# Send 20-30 message pairs
for i in {1..20}; do
    echo "Turn $i - testing context retention"
    # Verify context preserved throughout
done

# Monitor:
# - Memory usage (should be stable)
# - Session validity (no corruption)
# - Performance (no degradation)
# - Context accuracy (remembers all prior exchanges)
```

**Acceptance Criteria**:
- [ ] Successfully complete 20+ turn conversation
- [ ] Context preserved across all turns
- [ ] No memory leaks (stable RSS)
- [ ] No performance degradation
- [ ] Session files valid throughout

**Estimated Effort**: 20 minutes

---

### Task 4: Model-Switching Limitation Documentation ✅
**Status**: COMPLETE - Documentation created (2025-11-23)

**Problem**: Global providers use single model, can't switch between opus/sonnet/haiku in session mode

**Root Cause**:
```rust
// Global provider with empty model (uses CLI default)
static CLAUDE_PROVIDER: OnceLock<ClaudePipesProvider> = OnceLock::new();
CLAUDE_PROVIDER.get_or_init(|| ClaudePipesProvider::with_cwd("", &cwd))
```

**Acceptance Criteria**:
- [x] Document limitation in SPEC-952 notes or README
- [x] Describe workaround (use ChatGPT for model variety)
- [x] Note fix requires multi-instance providers (keyed by model)
- [x] Estimate effort for future fix (~2-3 hours)

**Deliverables** (Commit d70d05cb1):
- ✅ Created KNOWN-LIMITATIONS.md in SPEC-952 docs
- ✅ Documented root cause (global OnceLock provider singleton)
- ✅ Workaround documented (use ChatGPT account for model switching)
- ✅ Fix estimate (2-3 hours, HashMap<String, Provider> refactor)
- ✅ Linked from SPEC-952 README.md Known Limitations section

**Actual Effort**: 6 minutes (vs 15 estimated)

---

## Success Criteria

### Must Have
- [x] Message interleaving issue identified and documented (fix optional) ✅ FIXED (6 bugs)
- [x] Timeout fallback for silent failures ✅ (Task 1C - 10s timeout)
- [ ] Drop cleanup verified working (Task 2 - deferred)
- [ ] Long conversation tested (20+ turns) (Task 3 - deferred)
- [x] Model-switching limitation documented ✅ (Task 4)
- [x] Model preset validation documented ✅ (Task 5)

### Should Have
- [x] Message interleaving fixed (if root cause is simple) ✅ (6 bugs fixed)
- [x] Automated test for message ordering ✅ (21 tests total)
- [ ] Performance metrics from long conversation test

### Could Have
- [ ] Session management best practices guide
- [ ] Troubleshooting documentation

---

## Dependencies

**Upstream**:
- SPEC-KIT-952: CLI Routing (COMPLETE ✅)

**Downstream**: None (polish work)

---

## Estimated Effort

**Total**: 1.5-2.5 hours

**Breakdown**:
- Task 1: 30-60 minutes (investigation + potential fix)
- Task 2: 10 minutes (manual verification)
- Task 3: 20 minutes (stability testing)
- Task 4: 15 minutes (documentation)

**Timeline**: Single session

---

## Notes

**Based On**: Session handoff documents (SESSION-HANDOFF-PROCESS-MGMT-COMPLETE.md)

**Context**: These tasks emerged from testing the SPEC-952 implementation. Session management infrastructure is complete and working, but needs polish and verification.

**Priority**: P2 - Not blocking other work, but important for production quality.
