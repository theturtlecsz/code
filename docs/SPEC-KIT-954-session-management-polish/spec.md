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

### Task 1: Message Interleaving Investigation ⏳
**Status**: Debug logging added, investigation pending

**Problem**: User reports "questions and responses separate" instead of proper Q&A interleaving

**Acceptance Criteria**:
- [ ] Reproduce issue with test conversation
- [ ] Analyze debug logs to identify root cause
- [ ] Determine if it's key-based ordering, async timing, or display logic
- [ ] Implement fix
- [ ] Verify Q&A pairs display in correct order

**Files**:
- `tui/src/chatwidget/mod.rs:5595` (user message handling)
- `tui/src/chatwidget/mod.rs:11247-11303` (streaming handlers)
- `tui/src/chatwidget/mod.rs:4379, 4465` (history_push with keys)

**Debug Logging**: Emoji-tagged logs ready (🔵 user, 🟢 stream, 🟡 complete, 🟠 assistant, 📝 history)

**Estimated Effort**: 30-60 minutes

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

### Task 4: Model-Switching Limitation Documentation ⏳
**Status**: Known limitation, not documented

**Problem**: Global providers use single model, can't switch between opus/sonnet/haiku in session mode

**Root Cause**:
```rust
// Global provider with empty model (uses CLI default)
static CLAUDE_PROVIDER: OnceLock<ClaudePipesProvider> = OnceLock::new();
CLAUDE_PROVIDER.get_or_init(|| ClaudePipesProvider::with_cwd("", &cwd))
```

**Acceptance Criteria**:
- [ ] Document limitation in SPEC-952 notes or README
- [ ] Describe workaround (use ChatGPT for model variety)
- [ ] Note fix requires multi-instance providers (keyed by model)
- [ ] Estimate effort for future fix (~2-3 hours)

**Estimated Effort**: 15 minutes

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
