# Session Handoff — SYNC-028 TUI v2 Port

**Last updated:** 2025-12-24
**Status:** SYNC-028 In Progress - Protocol Sync Needed
**Priority:** SYNC-028 (TUI v2) → SPEC-KIT-926 (Progress Visibility)

---

## Session Summary (2025-12-24 - Session 4)

### Completed This Session

| Task | Status | Notes |
|------|--------|-------|
| Architecture Analysis | ✅ | Researched two-workspace model vs direct port |
| User Decision | ✅ | Direct port (not two-workspace), full dependencies |
| Crate Port: utils/absolute-path | ✅ | 195 LOC, copied from upstream |
| Crate Port: codex-backend-openapi-models | ✅ | Generated code, copied |
| Crate Port: app-server-protocol | ✅ | ~1K LOC, copied |
| Crate Port: backend-client | ✅ | Small, copied |
| Crate Port: windows-sandbox-rs | ✅ | Windows-only, copied |
| Crate Port: tui2 | ✅ | Main crate, copied |
| Workspace Cargo.toml | ✅ | Added 6 new members + dependencies |
| External Dependencies | ✅ | Added ratatui-macros, tree-sitter-highlight, etc. |
| core/Cargo.toml features | ✅ | Added test-support feature |

### Blocker Identified

**Protocol Divergence:** The `app-server-protocol` crate requires 6 files (655 LOC) missing from fork's `codex-protocol`:

| File | Lines | Purpose |
|------|-------|---------|
| `account.rs` | 20 | Account/plan types |
| `approvals.rs` | 95 | Approval workflow types |
| `conversation_id.rs` | 81 | ConversationId type |
| `items.rs` | 163 | Item types |
| `openai_models.rs` | 266 | ReasoningEffort, model configs |
| `user_input.rs` | 30 | User input types |

**User Decision:** Port the protocol files (not stub).

### Key Decisions Made

| Question | Decision |
|----------|----------|
| Two-workspace model? | No - direct port, current isolation is good |
| Full or minimal deps? | Full - port all 5 dependency crates |
| Protocol divergence? | Port the 655 LOC of missing files |

---

## Next Session: Complete SYNC-028

### Continuation Prompt

```
Continue SYNC-028 (TUI v2 port) **ultrathink**

## Context
Session 4 ported tui2 + 5 dependency crates but hit protocol divergence.
User decided to port missing protocol files (655 LOC).

## Remaining Tasks (in order)
1. Port 6 missing protocol files from ~/old/code/codex-rs/protocol/src/:
   - account.rs (20 LOC)
   - approvals.rs (95 LOC)
   - conversation_id.rs (81 LOC)
   - items.rs (163 LOC)
   - openai_models.rs (266 LOC)
   - user_input.rs (30 LOC)

2. Update codex-rs/protocol/src/lib.rs to export new modules

3. Check for conflicts with existing fork customizations (mcp_protocol.rs)

4. Build and fix any remaining compile errors:
   cargo check -p codex-tui2

5. Run basic tests:
   cargo test -p codex-tui2

6. Add CLI --tui2 flag for opt-in launch

7. Add features.tui2 config option (may integrate with SYNC-019)

8. Create UPSTREAM_SYNC.md tracking document

## Conflict Strategy
If config_types.rs conflicts arise: **Ask during session** (user preference)

## Files Modified So Far
- codex-rs/Cargo.toml (workspace members + deps)
- codex-rs/core/Cargo.toml (added [features] section)
- NEW: codex-rs/utils/absolute-path/
- NEW: codex-rs/codex-backend-openapi-models/
- NEW: codex-rs/app-server-protocol/
- NEW: codex-rs/backend-client/
- NEW: codex-rs/windows-sandbox-rs/
- NEW: codex-rs/tui2/

## Source Reference
Upstream: ~/old/code/codex-rs/ (just-every/code)

## Plan File
~/.claude/plans/deep-gathering-cosmos.md

## Success Criteria
- [ ] cargo build -p codex-tui2 succeeds
- [ ] cargo test -p codex-tui2 passes (some failures OK due to fork)
- [ ] codex-tui2 binary runs
- [ ] Existing codex-tui still works
```

---

## Architecture Decision Record

### Question: Adopt Two-Workspace Model?

**Decision:** No

**Rationale:**
1. Fork has diverged significantly (14K+ LOC custom crates, 150K+ TUI)
2. MAINT-11 refactoring already achieved good isolation (<100 LOC conflict surface)
3. Migration cost (2-4 weeks) outweighs benefits
4. Every Code's model is for tracking OpenAI Codex; our effective upstream is Every Code

**Alternative adopted:** Direct port of tui2 into existing workspace.

See: `~/.claude/plans/deep-gathering-cosmos.md` for full analysis.

---

## Upstream Relationship

```
openai/codex (archived)
    ↓
just-every/code ("Every Code" - active, has tui2)
    ↓
~/code (your fork - "Planner" with Spec-Kit)
```

**Key insight:** Your fork is essentially a distinct product, not just customizations.

---

## Protocol Crate Divergence

| Your Fork Has | Upstream Has | Status |
|---------------|--------------|--------|
| mcp_protocol.rs | (moved elsewhere) | Fork-specific |
| — | account.rs | **MISSING** |
| — | approvals.rs | **MISSING** |
| — | conversation_id.rs | **MISSING** |
| — | items.rs | **MISSING** |
| — | openai_models.rs | **MISSING** |
| — | user_input.rs | **MISSING** |

---

## Files Reference

| File | Purpose |
|------|---------|
| `~/.claude/plans/deep-gathering-cosmos.md` | Architecture analysis + implementation plan |
| `~/old/code/codex-rs/tui2/docs/tui_viewport_and_history.md` | TUI2 design document |
| `~/old/code/codex-rs/protocol/src/` | Source for missing protocol files |
| `codex-rs/protocol/src/lib.rs` | Needs update to export new modules |

---

## Rollback Plan

If issues arise:
```bash
cd ~/code/codex-rs
rm -rf utils/absolute-path codex-backend-openapi-models app-server-protocol backend-client windows-sandbox-rs tui2
git checkout Cargo.toml core/Cargo.toml
```
