**SPEC-ID**: SPEC-KIT-954
**Feature**: Session Management UX Polish & Testing
**Status**: Backlog
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
- [ ] Message interleaving issue identified and documented (fix optional)
- [ ] Drop cleanup verified working
- [ ] Long conversation tested (20+ turns)
- [ ] Model-switching limitation documented

### Should Have
- [ ] Message interleaving fixed (if root cause is simple)
- [ ] Automated test for message ordering
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
