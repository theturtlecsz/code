# ⚡ SPEC-955 Session 2 - START HERE

## 30-Second Context

**What:** Fix TUI test deadlock (async refactor)
**Status:** Deadlock fixed ✅, 5 tests failing ❌, need debugging
**Time:** Session 1: 5h done | Session 2: 8-10h needed

---

## 3-Step Start

```bash
# 1. Navigate + verify
cd /home/thetu/code/codex-rs
git status  # 14 modified files
timeout 180 cargo test -p codex-tui --lib test_harness::tests 2>&1 | grep "test result:"
# Expect: 4 passed; 5 failed; ~0.20s

# 2. Load context
# Local-memory ID: 701b6762-973a-4bf0-ae92-78c5d84de11a

# 3. Start debugging
cat ../docs/SPEC-955-SESSION-2-QUICKSTART.md
```

---

## The One Thing to Fix

**THE BUG:**
```rust
harness.send_codex_event(Event { msg: AgentMessageDelta {...} });
// → Should create assistant history cell
// → Actually creates: 0 assistant cells ❌
```

**WHY IT MATTERS:**
- 5/9 tests fail because they expect assistant cells
- Even "passing" test has 0 assistant cells (weak assertion)
- Indicates handle_codex_event() not working properly

**FIRST HYPOTHESIS TO TEST:**
Maybe tests should use `codex_op_tx.send(Op::Submit)` instead of `handle_codex_event()`?

---

## Session 2 Checklist

**Hour 1-2:** Debug handle_codex_event (logging + Op channel hypothesis)
**Hour 3-4:** Fix 5 failing tests
**Hour 5-6:** Complete refactoring (theme_selection, file_search)
**Hour 7-8:** Validation (9/9 tests, full suite, manual TUI)
**Hour 9:** CI + Performance + Memory
**Hour 10:** Docs + Commit

**Success:** 9/9 tests green, committed, SPEC-955 COMPLETE

---

## Full Docs

- 📖 `SPEC-955-SESSION-2-PROMPT.md` - Comprehensive guide (300+ lines)
- 🚀 `SPEC-955-SESSION-2-QUICKSTART.md` - Quick start (detailed debugging)
- 📊 `SPEC-955-PROGRESS-TRACKER.md` - Visual progress tracking
- 🤝 `SPEC-955-SESSION-HANDOFF.md` - Session handoff checklist

---

**Ready to debug? Read QUICKSTART.md →**
