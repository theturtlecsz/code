# P120 Session Prompt

```
load HANDOFF.md **ultrathink**
```

Continue P120: MAINT-11 Phase 9 (agents_terminal extraction) + documentation update.

## Context
- P119 completed: session_handlers.rs extraction (-558 LOC)
- Commit: `7ce0d4111`
- mod.rs: 19,792 LOC remaining
- Cumulative MAINT-11: -3,621 LOC (-15.5%)
- Architecture diagram: docs/architecture/chatwidget-structure.md

---

## Primary Task: MAINT-11 Phase 9 - Agents Terminal

### Goal
Extract agents terminal functionality from `mod.rs` into `agents_terminal.rs`.

### Target
~300 LOC extraction

### Phase 1: Investigation
Analyze agents terminal code boundaries:
```bash
cd codex-rs

# Find agents_terminal related code
grep -n "agents_terminal\|AgentsTerminal" tui/src/chatwidget/mod.rs | head -30

# Find agent terminal state struct
grep -n "struct.*AgentsTerminal\|agents_terminal:" tui/src/chatwidget/mod.rs | head -20

# Find agent terminal render/update functions
grep -n "fn.*agents_terminal\|render_agents_terminal\|update_agents" tui/src/chatwidget/mod.rs | head -20
```

### Phase 2: Define Module Boundaries
Expected `agents_terminal.rs` contents (~300 LOC target):
- AgentsTerminalState struct (if applicable)
- Agent terminal overlay rendering
- Agent terminal input handling
- Related helper functions

### Phase 3: Staged Extraction
1. Create `chatwidget/agents_terminal.rs` with proper imports
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
Update `docs/MAINT-11-EXTRACTION-PLAN.md` with P119 completion status.

### Updates Required
- Mark Phase 8 (session_handlers) as complete
- Add actual LOC counts vs estimates
- Update cumulative metrics
- Add Phase 9 (agents_terminal) as in-progress

---

## Deliverables Checklist

### Required
- [ ] `agents_terminal.rs` module created (~300 LOC)
- [ ] mod.rs reduced by ~300 LOC
- [ ] All TUI tests pass
- [ ] Clippy clean (0 warnings)
- [ ] MAINT-11-EXTRACTION-PLAN.md updated
- [ ] HANDOFF.md updated for P121

### Optional (if time permits)
- [ ] Add integration test for session save/load cycle
- [ ] Update architecture diagram with agents_terminal

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
| ChatWidget Monolith | `tui/src/chatwidget/mod.rs` | 19,792 |
| Session handlers | `tui/src/chatwidget/session_handlers.rs` | 624 |
| Architecture Diagram | `docs/architecture/chatwidget-structure.md` | - |
| MAINT-11 Plan | `docs/MAINT-11-EXTRACTION-PLAN.md` | - |
| Extraction Guide | `docs/EXTRACTION-GUIDE.md` | - |

---

## Work Queue (Post-P120)

| Session | Target | Est. LOC | Complexity |
|---------|--------|----------|------------|
| P121 | history_handlers.rs | ~600 | Medium |
| P122+ | event_handlers.rs | ~1,000 | High |
| P123+ | config_handlers.rs | ~400 | Medium |

---

## Expected Outcomes

| Deliverable | Target |
|-------------|--------|
| `agents_terminal.rs` | ~300 LOC extracted |
| mod.rs reduction | 19,792 → ~19,500 LOC |
| Tests | All passing |
| Clippy warnings | 0 |
| MAINT-11 progress | 15.5% → ~17% |

---

## Extraction Pattern Reference

Following the established pattern from P118/P119:

1. **Investigate** - grep for related functions, understand boundaries
2. **Create module** - New file with minimal imports
3. **Extract functions** - Move one at a time, fix imports
4. **Update visibility** - `pub(crate)` for cross-module access
5. **Add tests** - Unit tests for pure functions
6. **Verify** - clippy + tests
7. **Document** - Update HANDOFF.md

---

_Generated: 2025-12-16 for P120 session_
