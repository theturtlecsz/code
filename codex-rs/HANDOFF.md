# Dead Code Cleanup - Session 17 Handoff

**Date**: 2025-12-26
**Task**: SPEC-DOGFOOD-001 Dead Code Audit & Cleanup
**Session**: 17 (partial work, build broken)
**Plan File**: `~/.claude/plans/stateful-floating-anchor.md`

---

## Session 17 Summary

### Work Completed (Build Currently Broken)

#### Dead Code Deleted (~1,000 LOC)
1. **chatwidget/mod.rs**:
   - `set_mouse_status_message()` - never called
   - `notify_login_claude_failed()`, `notify_login_gemini_failed()` - never called
   - `coalesce_read_ranges_in_lines()` - 80 LOC, never called
   - Consensus infrastructure methods with hardcoded paths (~700 LOC)
   - Thin wrapper methods that just delegated to spec_kit

2. **spec_kit modules**:
   - Deleted `evidence_cleanup.rs` module (~180 LOC) - 0 usages
   - Removed `handle_spec_consensus_impl` from consensus_coordinator.rs
   - Removed unused re-export from handler.rs

3. **login_accounts_view.rs**:
   - Deleted redundant `on_claude_failed()`, `on_gemini_failed()` methods

#### Trait Changes
- Removed `run_spec_consensus()` from `SpecKitContext` trait (context.rs)
- Removed implementation from ChatWidget

### Build Status: BROKEN

**Remaining Issues**:
1. 3 deprecated tests still call `.run_spec_consensus()` - need deletion
2. Unused imports in `consensus_coordinator.rs`: `parse_consensus_stage`, `HistoryCellType`
3. Unused import in `chatwidget/mod.rs`: `sha2::{Digest, Sha256}`
4. Unused import in `spec_kit/mod.rs`: `advance_spec_auto`

---

## Session 18 Tasks (Next Session)

### Priority 1: Fix Build Errors

1. **Delete deprecated tests** (chatwidget/mod.rs, ~200 LOC):
   - `run_spec_consensus_writes_verdict_and_local_memory()`
   - `run_spec_consensus_reports_missing_agents()`
   - `run_spec_consensus_persists_telemetry_bundle_when_enabled()`

2. **Clean unused imports**:
   - `consensus_coordinator.rs`: Remove `parse_consensus_stage`, `HistoryCellType`
   - `chatwidget/mod.rs`: Remove `sha2::{Digest, Sha256}`
   - `spec_kit/mod.rs`: Remove unused `advance_spec_auto`

3. **Build and verify**:
   ```bash
   cargo build --workspace
   cargo test -p codex-tui
   ```

4. **Commit with message**:
   ```
   fix(tui): dead code cleanup - Session 17+18

   - Delete ~1,200 LOC of dead consensus infrastructure
   - Remove deprecated tests using old LocalMemoryMock
   - Clean unused imports
   - Remove run_spec_consensus from SpecKitContext trait
   ```

---

## Multi-Session Plan Overview

| Session | Goal | Est. LOC |
|---------|------|----------|
| 18 | Fix build, commit current work | -200 |
| 19 | Delete native_consensus_executor.rs | -407 |
| 20 | Type alias migration (Consensus* → Gate*) | -50 |
| 21-23 | Analyze undocumented annotations | -300 |
| 24 | Final audit and documentation | 0 |

**Total estimated deletion**: ~957 LOC

---

## Critical Files

| File | Purpose |
|------|---------|
| `tui/src/chatwidget/mod.rs` | Main ChatWidget, has deprecated tests |
| `tui/src/chatwidget/spec_kit/context.rs` | SpecKitContext trait (modified) |
| `tui/src/chatwidget/spec_kit/consensus_coordinator.rs` | Has unused imports |
| `tui/src/chatwidget/spec_kit/gate_evaluation.rs` | Active consensus implementation |
| `~/.claude/plans/stateful-floating-anchor.md` | Full multi-session plan |

---

## User Decisions (From Session 17)

- **Scope**: tui crate only (not tui2, per ADR-002)
- **Undocumented annotations**: Analyze, delete truly dead, tag infrastructure
- **Type aliases**: Include Consensus* → Gate* migration in cleanup
- **Old tests**: Delete deprecated tests using LocalMemoryMock

---

## Continuation Prompt for Session 18

```
Continue SPEC-DOGFOOD-001 Dead Code Cleanup - Session 18 **ultrathink**

## Context
Session 17 deleted ~1,000 LOC of dead code but left the build broken.
See HANDOFF.md and ~/.claude/plans/stateful-floating-anchor.md for full plan.

## Immediate Tasks
1. Delete 3 deprecated tests calling run_spec_consensus (~200 LOC)
2. Clean unused imports (consensus_coordinator.rs, chatwidget/mod.rs, spec_kit/mod.rs)
3. Build and verify: cargo build --workspace
4. Run tests: cargo test -p codex-tui
5. Commit the complete Session 17+18 dead code cleanup

## Files to Modify
- tui/src/chatwidget/mod.rs - Delete tests, clean imports
- tui/src/chatwidget/spec_kit/consensus_coordinator.rs - Clean imports
- tui/src/chatwidget/spec_kit/mod.rs - Clean imports

## Success Criteria
- [ ] cargo build --workspace - 0 errors, minimal warnings
- [ ] cargo test -p codex-tui - passes
- [ ] Commit pushed

## After Session 18
Continue with Session 19: Delete native_consensus_executor.rs (407 LOC)
```

---

_Last updated: 2025-12-26 (Session 17 handoff)_
