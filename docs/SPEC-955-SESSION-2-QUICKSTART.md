# SPEC-955 Session 2 Quick Start

## TL;DR - What You Need to Know

**Primary Goal:** Debug why handle_codex_event() doesn't create history cells (0 assistant cells in tests)

**Current State:**
- ✅ Deadlock FIXED (no more 60+ second hangs!)
- ✅ 4/9 tests passing (up from 3/9)
- ❌ 5/9 tests failing (need assistant cells)
- ⚠️ All changes uncommitted (waiting for 9/9 green)

---

## 🚀 Session 2 Start (3 commands)

```bash
# 1. Navigate
cd /home/thetu/code/codex-rs

# 2. Confirm state
timeout 180 cargo test -p codex-tui --lib test_harness::tests 2>&1 | grep "test result:"
# Expect: ok. 4 passed; 5 failed; finished in ~0.20s

# 3. Start debugging
cargo test test_simulate_streaming_response -- --nocapture 2>&1 | grep -A5 "Assistant cells:"
# Expect: "Assistant cells: 0" (THE BUG)
```

---

## 🔍 Debug Plan (Pick One Path)

### Path A: Op Channel Hypothesis (RECOMMENDED - 2 hours)

**Theory:** Tests should use Op channel, not direct event injection

```rust
// Current (doesn't work):
harness.send_codex_event(Event { ... });  // → 0 history cells

// Try this:
harness.widget.codex_op_tx.send(Op::Submit { ... })?;
harness.drain_app_events();  // → might create history cells?
```

**Action:** Test in test_simulate_streaming_response first

### Path B: Add Logging (1 hour)

**Theory:** Events aren't reaching history mutation code

**Add to `tui/src/chatwidget/mod.rs`:**
```rust
pub(crate) fn handle_codex_event(&mut self, event: Event) {
    eprintln!("[TRACE] Event: {:?}", event.msg);
    // ...
    eprintln!("[TRACE] History after: {}", self.history_cells.len());
}
```

**Run:** `cargo test test_simulate --nocapture 2>&1 | grep TRACE`

### Path C: Check Conversation State (30 min)

**Theory:** Widget needs active session/conversation

**Check:**
```bash
rg "struct ChatWidget" tui/src/chatwidget/mod.rs -A50 | grep "session\|conversation"
```

---

## 📊 Key Metrics to Track

**Test Results:**
- [ ] 9/9 TUI tests passing (currently 4/9)
- [ ] 7/7 AppEventSender integration tests passing (currently ✅)
- [ ] Full workspace test suite green

**Performance:**
- [ ] Event throughput: __ events/sec (measure after fix)
- [ ] Test suite time: ~0.20s (currently ✅)
- [ ] Manual TUI responsiveness: Good

**Memory:**
- [ ] No leaks in valgrind
- [ ] Long-running TUI stable (30+ min test)

---

## 🎯 Session 2 Completion Checklist

**Before Committing:**
- [ ] All 9 TUI tests green
- [ ] Full workspace tests green
- [ ] CI strict mode clean (clippy + fmt)
- [ ] Manual TUI tested (10 scenarios)
- [ ] Performance: Same or better
- [ ] Memory: No leaks
- [ ] SPEC-955 docs updated
- [ ] Local-memory updated with solution details

**Commit Criteria:**
- Single comprehensive commit
- Detailed message with root cause + solution
- All validation evidence included
- Push to main after final checks

---

## 📖 Read Full Details

See `docs/SPEC-955-SESSION-2-PROMPT.md` for:
- Complete file modification list
- Detailed debugging strategy (6 hypotheses)
- Step-by-step instructions for each phase
- Architectural lessons learned
- Success criteria and escalation paths

**Local-Memory ID:** `701b6762-973a-4bf0-ae92-78c5d84de11a`

---

**Created:** 2025-11-23
**Session 1 Time:** 5 hours
**Session 2 Estimate:** 8-10 hours
**Total Estimate:** 12-15 hours
