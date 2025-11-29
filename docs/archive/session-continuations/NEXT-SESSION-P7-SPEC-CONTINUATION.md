# Next Session: P7-SPEC Continuation

**Generated**: 2025-11-29
**Previous Session**: Completed P6-SYNC verification, P7-SPEC Tasks 1-3, dead code cleanup
**Focus**: Finish SPEC-KIT-900, cleanup, validation

---

## Session Context

### Commits from Previous Session
```
43f242094 feat(spec-kit): Complete run_id propagation and log tagging (P7-SPEC)
078c0247c chore(tui): Remove dead code and update audit TODO
```

### Current State
- **P6-SYNC**: Complete (Phases 1-7 done, token refresh not needed)
- **P7-SPEC Tasks 1-3**: Complete (run_id, log tagging, completions)
- **P7-SPEC Tasks 4-5**: Need analysis before implementation
- **Build**: Clean (no warnings)
- **Git**: Up to date with origin/main

---

## Task 1: Analyze SPEC-KIT-900 Tasks 4-5 (~30 min) **ultrathink**

Before implementing `/speckit.verify`, analyze whether it's still needed:

### Questions to Answer
1. **What problem does /speckit.verify solve?**
   - Is there a current pain point it addresses?
   - Can existing tools (`/speckit.status`, SQLite queries) achieve the same goal?

2. **Current Implementation State**
   - Check if any partial implementation exists
   - Review `docs/SPEC-KIT-900-AUDIT-INFRASTRUCTURE-TODO.md` Task 4-5 specs
   - Search for existing verify-related code

3. **PRD Completeness**
   - Is there a proper PRD for this command?
   - If not, should we create one via `/speckit.new` or `/speckit.specify`?

### Decision Points
- **Option A**: Implement as specified (1.5-2 hours)
- **Option B**: Simplify scope (e.g., just query SQLite, no fancy report)
- **Option C**: Defer/close as won't-do (document rationale)
- **Option D**: Create proper PRD first, then implement

### Files to Examine
```
docs/SPEC-KIT-900-AUDIT-INFRASTRUCTURE-TODO.md  # Task 4-5 specs
codex-rs/tui/src/chatwidget/spec_kit/           # Existing commands
~/.code/consensus_artifacts.db                   # SQLite schema
```

---

## Task 2: Archive Old Continuation Docs (~15 min)

17 NEXT-SESSION-*.md files to clean up:

```bash
# List all continuation docs
ls -la ~/code/docs/NEXT-SESSION-*.md

# Create archive directory
mkdir -p ~/code/docs/archive/session-continuations

# Move old docs (keep only current one)
mv ~/code/docs/NEXT-SESSION-*.md ~/code/docs/archive/session-continuations/

# Move this doc back to active location
mv ~/code/docs/archive/session-continuations/NEXT-SESSION-P7-SPEC-CONTINUATION.md ~/code/docs/
```

**Commit**: `chore(docs): Archive 17 old session continuation docs`

---

## Task 3: Quick Test Suite Check (~15 min)

```bash
# Verify build
cd ~/code/codex-rs && cargo build -p codex-tui

# Run core tests
cargo test -p codex-core

# Run TUI tests
cargo test -p codex-tui

# Run login tests
cargo test -p codex-login
```

**Expected**: All tests pass, no new failures
**If failures**: Document in commit, create issue if blocking

---

## Task 4: Implement or Close Tasks 4-5 (Based on Task 1 Analysis)

If implementing `/speckit.verify`:

### Minimal Implementation (Option B)
```rust
// New file: codex-rs/tui/src/chatwidget/spec_kit/commands/verify.rs

/// Query SQLite for pipeline run data and format as report
pub fn handle_verify_command(widget: &mut ChatWidget, spec_id: &str) {
    // 1. Query agent_executions for spec_id
    // 2. Group by run_id, stage
    // 3. Format as simple text report
    // 4. Display in chat history
}
```

### Full Implementation (Option A)
- Follow spec in TODO.md Task 4
- Create `SpecKitVerifyCommand` struct
- Rich formatting with stage timelines
- SQLite verification checks

---

## Commit Sequence

1. `chore(docs): Archive old session continuation docs`
2. `docs(spec-kit): Add analysis notes for /speckit.verify decision`
3. (If implementing) `feat(spec-kit): Add /speckit.verify command`
4. Final status commit with any remaining notes

---

## Success Criteria

- [ ] SPEC-KIT-900 Tasks 4-5 resolved (implemented OR closed with rationale)
- [ ] Old continuation docs archived
- [ ] Test suite passes (quick check)
- [ ] Working tree clean
- [ ] No pending P7-SPEC work

---

## Commands Reference

```bash
# Start session
cd ~/code
git status
cargo check -p codex-tui

# Check SQLite schema
sqlite3 ~/.code/consensus_artifacts.db ".schema"

# Query recent runs
sqlite3 ~/.code/consensus_artifacts.db \
  "SELECT DISTINCT spec_id, run_id, stage FROM agent_executions ORDER BY spawned_at DESC LIMIT 20"

# Build and test
~/code/build-fast.sh
cargo test -p codex-tui
```
