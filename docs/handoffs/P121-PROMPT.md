# P121 Session Prompt

```
load HANDOFF.md **ultrathink**
```

Continue P121: MAINT-11 Phase 10 (history_handlers extraction) + documentation update.

## Context
- P120 completed: agents_terminal.rs extraction (-719 LOC)
- Commit: (pending)
- mod.rs: 19,073 LOC remaining
- Cumulative MAINT-11: -4,340 LOC (-18.5%)

---

## Primary Task: MAINT-11 Phase 10 - History Handlers

### Goal
Extract history cell management from `mod.rs` into `history_handlers.rs`.

### Target
~600 LOC extraction

### Phase 1: Investigation
Analyze history handler code boundaries:
```bash
cd codex-rs

# Find history-related functions
grep -n "fn history_\|fn.*history" tui/src/chatwidget/mod.rs | head -30

# Find history cell operations
grep -n "history_cells\|history\.push\|history\.replace" tui/src/chatwidget/mod.rs | head -30

# Find HistoryCell trait usage
grep -n "HistoryCell\|dyn HistoryCell" tui/src/chatwidget/mod.rs | head -20
```

### Phase 2: Define Module Boundaries
Expected `history_handlers.rs` contents (~600 LOC target):
- History cell push/replace operations
- History merging logic
- History clear/truncate operations
- Related helper functions

### Phase 3: Staged Extraction
1. Create `chatwidget/history_handlers.rs` with proper imports
2. Move functions one at a time, updating imports
3. Add `pub(crate)` visibility where needed
4. Update callers in mod.rs
5. Add tests if appropriate

### Phase 4: Verification
```bash
cargo test -p codex-tui
cargo clippy -p codex-tui -- -D warnings
cargo test --workspace
```

---

## Secondary Task: Update MAINT-11 Extraction Plan

### Goal
Update `docs/MAINT-11-EXTRACTION-PLAN.md` with P120 completion status.

### Updates Required
- Verify Phase 9 (agents_terminal) marked complete
- Add actual LOC counts vs estimates
- Update cumulative metrics
- Add Phase 10 (history_handlers) as in-progress

---

## Deliverables Checklist

### Required
- [ ] `history_handlers.rs` module created (~600 LOC)
- [ ] mod.rs reduced by ~600 LOC
- [ ] All TUI tests pass
- [ ] Clippy clean (0 warnings)
- [ ] MAINT-11-EXTRACTION-PLAN.md updated
- [ ] HANDOFF.md updated for P122

### Optional (if time permits)
- [ ] Investigate event handlers for P122 planning
- [ ] Update architecture diagram with history_handlers

---

## Safety Checks

- [ ] Search for ALL usages before moving any function
- [ ] Check for trait implementations that depend on moved code
- [ ] Verify no circular imports after extraction
- [ ] Run clippy to catch dead code and unused imports
- [ ] All TUI tests pass
- [ ] Full workspace tests pass

---

## File References

| Component | File | Current Lines |
|-----------|------|---------------|
| ChatWidget Monolith | `tui/src/chatwidget/mod.rs` | 19,073 |
| Agents Terminal | `tui/src/chatwidget/agents_terminal.rs` | 759 |
| Architecture Diagram | `docs/architecture/chatwidget-structure.md` | - |
| MAINT-11 Plan | `docs/MAINT-11-EXTRACTION-PLAN.md` | - |
| Extraction Guide | `docs/EXTRACTION-GUIDE.md` | - |

---

## Work Queue (Post-P121)

| Session | Target | Est. LOC | Complexity |
|---------|--------|----------|------------|
| P122+ | event_handlers.rs | ~1,000 | High |
| P123+ | config_handlers.rs | ~400 | Medium |

---

## Expected Outcomes

| Deliverable | Target |
|-------------|--------|
| `history_handlers.rs` | ~600 LOC extracted |
| mod.rs reduction | 19,073 → ~18,500 LOC |
| Tests | All passing |
| Clippy warnings | 0 |
| MAINT-11 progress | 18.5% → ~21% |

---

## Extraction Pattern Reference

Following the established pattern from P118/P119/P120:

1. **Investigate** - grep for related functions, understand boundaries
2. **Create module** - New file with minimal imports
3. **Extract functions** - Move one at a time, fix imports
4. **Update visibility** - `pub(crate)` for cross-module access
5. **Add tests** - Unit tests for pure functions
6. **Verify** - clippy + tests
7. **Document** - Update HANDOFF.md

---

_Generated: 2025-12-16 for P121 session_
