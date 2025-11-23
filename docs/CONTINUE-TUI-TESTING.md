# CONTINUE: TUI Testing Improvements (Quick Start)

**Copy this entire block into your next Claude Code session:**

---

## Context Load

I'm continuing TUI testing improvements for **SPEC-KIT-954** (Session Management UX Polish).

**Previous session completed** (2025-11-23):
- ✅ Item 1: Test layout refactoring (commit 41fcbbf67)
- ✅ Item 3: Enhanced parsing tests (commit b382f484d) 
- ✅ Item 4: CLI integration tests (commit 7f18d88a4)

**Remaining work**:
- ⏸️ Item 2: Strengthen invariants (BLOCKED - test_harness.rs has 28 errors)
- ⏸️ Item 5: Tighten snapshots (BLOCKED - same)
- ⏳ Item 6: CI/coverage (READY - not blocked)

**Context files**:
```bash
cat /home/thetu/code/docs/SPEC-KIT-954-session-management-polish/spec.md
cat /home/thetu/code/docs/NEXT-SESSION-TUI-IMPROVEMENTS-CONTINUE.md
cat /home/thetu/code/codex-rs/TESTING-CRITIQUE.md
```

**Local-memory**:
```
Search: "TUI testing improvements 2025-11-23"
# Will find 3 memories:
# - Parsing tests (54a5079b-762c-4230-a70e-f43643a72672)
# - CLI integration (69ddb935-9549-407f-a705-89fb06120d44)
# - Test refactoring (64e5e599-c291-4629-b1e6-bcba23229507)
```

---

## Choose Your Path

### Option A: Fix Blocker (Recommended)
**Task**: Fix test_harness.rs compilation errors (28 errors)  
**Effort**: 1-2 hours  
**Unblocks**: Items 2 & 5

Say: "I want to fix the test_harness.rs compilation errors so we can complete Items 2 and 5"

### Option B: CI First
**Task**: Implement CI/coverage (Item 6)  
**Effort**: 2-3 hours  
**Independent**: Doesn't need test_harness

Say: "I want to implement Item 6 (CI/coverage) since it's not blocked"

### Option C: Complete All
**Task**: Fix blocker + finish all remaining items  
**Effort**: 3-5 hours  
**Comprehensive**: Full completion

Say: "I want to complete all remaining items (2, 5, 6) - fix test_harness.rs first, then strengthen invariants, tighten snapshots, and add CI/coverage"

---

## Quick Validation

```bash
cd /home/thetu/code/codex-rs

# Verify recent commits
git log --oneline -3

# Try compiling tests (will show errors)
cargo test -p codex-tui --lib 2>&1 | grep "error\[E" | wc -l
# Expected: ~28 errors

# Library should compile fine
cargo build -p codex-tui --lib
# Expected: Success ✅
```

---

**Ready to continue!** Paste this prompt and choose your path (A, B, or C).
