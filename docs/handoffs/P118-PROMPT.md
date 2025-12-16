# P118 Session Prompt

```
load HANDOFF.md **ultrathink**
```

Continue P118: MAINT-11 Phase 7 (Review/Merge handlers extraction).

## Context
- P117 completed: Browser/chrome dead code removal (-2,094 LOC)
- Commits: `15aa783a7` (code removal), `d66893fa2` (handoff)
- mod.rs: 20,758 LOC remaining
- **User guidance**: Focus on review/merge handlers extraction first

## Primary Task: MAINT-11 Phase 7

### Goal
Extract review/merge handler functionality from `mod.rs` into a dedicated module.

### Phase 1: Investigation
Analyze review/merge code boundaries:
```bash
cd codex-rs

# Find all review-related functions
grep -n "fn.*review\|Review\|review_" tui/src/chatwidget/mod.rs | head -40

# Find all merge-related functions
grep -n "fn.*merge\|Merge\|merge_" tui/src/chatwidget/mod.rs | head -30

# Check for review event handlers
grep -n "EnteredReviewMode\|ExitedReviewMode\|ReviewRequest" tui/src/chatwidget/mod.rs

# Find open_review_dialog and related
grep -n "open_review_dialog\|StartReviewCommitPicker\|StartReviewBranchPicker" tui/src/chatwidget/mod.rs
```

### Phase 2: Define Module Boundaries
Expected `review_handlers.rs` contents:
- `open_review_dialog()` - Show review options modal
- `handle_review_mode_entered()` - Process EnteredReviewMode event
- `handle_review_mode_exited()` - Process ExitedReviewMode event
- `start_review_commit_picker()` - Commit selection UI
- `start_review_branch_picker()` - Branch selection UI
- Related types and helpers

### Phase 3: Staged Extraction
1. Create `chatwidget/review_handlers.rs` with function stubs
2. Move functions one at a time, updating imports
3. Add `pub(crate) use` re-exports in `mod.rs`
4. Update callers to use new module
5. Add tests for extracted functions

### Phase 4: Verification
```bash
cargo test -p codex-tui
cargo clippy -p codex-tui -- -D warnings
cargo test --workspace  # Full workspace validation
```

---

## Secondary Tasks

### 2A: Update Architecture Docs
After extraction, update `docs/architecture/` with:
- Current module structure diagram
- ChatWidget component breakdown
- Extraction progress metrics

### 2B: Create Extraction Guide
Document the extraction pattern in `docs/EXTRACTION-GUIDE.md`:
- Step-by-step process used in Phases 1-6
- Common pitfalls (import cycles, visibility)
- Test patterns for extracted modules
- Checklist for future extractions

---

## Safety Checks
- [ ] Search for ALL usages before moving any function
- [ ] Check for trait implementations that depend on moved code
- [ ] Verify no circular imports after extraction
- [ ] Run clippy to catch dead code and unused imports
- [ ] All TUI tests pass
- [ ] Full workspace tests pass

---

## Expected Outcomes

| Deliverable | Target |
|-------------|--------|
| `review_handlers.rs` | ~500 LOC extracted |
| mod.rs reduction | 20,758 → ~20,250 LOC |
| Tests | All passing |
| Clippy warnings | 0 |
| Architecture docs | Updated |
| Extraction guide | Created |

---

## File References

| Component | File | Current Lines |
|-----------|------|---------------|
| ChatWidget Monolith | `tui/src/chatwidget/mod.rs` | 20,758 |
| Review events | `tui/src/app_event.rs` | (RunReviewCommand, etc.) |
| Review protocol | `core/src/protocol.rs` | (ReviewRequest, ReviewOutput) |
| MAINT-11 Tracker | `SPEC.md:186` | - |

---

## MAINT-11 Progress Summary

| Phase | Session | LOC Change | Total mod.rs |
|-------|---------|------------|--------------|
| 1 | P110 | -200 extracted | 23,213 |
| 2 | P113 | -65 extracted | 23,151 |
| 3 | P114 | -300 extracted | 22,911 |
| 4 | P115 | -5 removed | 22,906 |
| 5 | P116 | -54 extracted | 22,852 |
| 6 | P117 | -2,094 removed | 20,758 |
| 7 | P118 | ~-500 (target) | ~20,250 |

---

## Remaining Extraction Candidates (Post-P118)

| Target | Est. LOC | Complexity | Notes |
|--------|----------|------------|-------|
| Session management | ~800 | Medium | save/load/resume |
| Agents terminal | ~300 | Low | AgentsTerminalState |
| History management | ~600 | Medium | push/replace/merge |
| Event handlers | ~1,000 | High | Multiple categories |

---

_Generated: 2025-12-16 for P118 session_
