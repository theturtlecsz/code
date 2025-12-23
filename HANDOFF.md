# Session Handoff — MAINT-11 Phase 9 + Convergence

**Last updated:** 2025-12-23
**Status:** MAINT-12 Complete, MAINT-11 Phase 9 Ready

---

## Session Summary (2025-12-23)

### Completed This Session

| Task | Status | Commit | Notes |
|------|--------|--------|-------|
| MAINT-12: Stage0 HTTP-only | ✅ | `db3985846` | Removed dead `Tier2McpAdapter` (~70 LOC) |
| SPEC.md update | ✅ | `15ac70e5c` | Marked MAINT-12 complete |
| Convergence docs | ✅ | `ada1eed7c` | Added MEMO/PROMPT pointer files |
| MAINT-11 assessment | ✅ | — | mod.rs: 19,070 LOC (down 18.5%) |
| Convergence verification | ✅ | — | All 6 criteria verified passing |

### Convergence Verification Results

| Requirement | Status | Evidence |
|-------------|--------|----------|
| Pointer docs | ✅ | `codex-rs/docs/convergence/README.md`, `MEMO_codex-rs.md`, `PROMPT_codex-rs.md` |
| Tier2 fail-closed | ✅ | `stage0_integration.rs:119-161` |
| `code doctor` command | ✅ | `cli/src/main.rs:926-1191` |
| System pointer memory | ✅ | `system_memory.rs` + `stage0_integration.rs:506-610` |
| Tier1 excludes system:true | ✅ | `dcc.rs:414-418` |
| Acceptance tests | ✅ | 6 tests in `convergence_acceptance.rs` |

---

## Next Session: MAINT-11 Phase 9

### Primary Goal: Extract Undo/Snapshots Module

**Target:** Extract ~460 LOC to `tui/src/chatwidget/undo_snapshots.rs`

**Functions to Extract:**

| Function | Line | Purpose |
|----------|------|---------|
| `GhostSnapshot` struct | 589 | Snapshot data structure |
| `capture_ghost_snapshot` | 4666 | Create snapshot after commit |
| `current_conversation_snapshot` | 4712 | Get current state |
| `conversation_delta_since` | 4730 | Calculate changes |
| `snapshot_preview` | 4756 | UI preview generation |
| `handle_undo_command` | 4772 | Main `/undo` handler |
| `show_undo_snapshots_disabled` | 4795 | Disabled state UI |
| `show_undo_empty_state` | 4821 | Empty state UI |
| `show_undo_status_popup` | 4836 | Status popup |
| `show_undo_snapshot_picker` | 4891 | Picker UI |
| `show_undo_restore_options` | 4956 | Restore options modal |
| `perform_undo_restore` | 5013 | Execute restore |
| `reset_after_conversation_restore` | 5124 | Post-restore cleanup |
| `undo_jump_back` | 10882 | Quick undo |

**Expected Result:** mod.rs < 18,600 LOC

### Secondary Goals

| Task | Priority | Notes |
|------|----------|-------|
| Run full test suite | P1 | Verify 604+ tests pass after extraction |
| Document P3 recall command | P2 | `code stage0 recall SPEC-ID` as future work |

### Deferred (Separate Session)

- AD-001: Async/blocking unification
- AD-006: Backpressure for unbounded channels
- MAINT-13: Config inheritance

---

## MAINT-11 Progress Tracker

| Phase | Module | LOC | Status |
|-------|--------|-----|--------|
| 1 | `command_render.rs` | 303 | ✅ |
| 2 | `agent_status.rs` | 123 | ✅ |
| 3 | `submit_helpers.rs` | 302 | ✅ |
| 4 | Dead code cleanup | -2,094 | ✅ |
| 5 | `input_helpers.rs` | 175 | ✅ |
| 6 | Browser/chrome removal | -2,094 | ✅ |
| 7 | `review_handlers.rs` | 462 | ✅ |
| 8 | `session_handlers.rs` | 619 | ✅ |
| **9** | **`undo_snapshots.rs`** | **~460** | **NEXT** |
| 10 | `pro_overlay.rs` | ~200 | Planned |
| 11 | `validation_config.rs` | ~200 | Planned |
| 12 | `model_presets.rs` | ~120 | Planned |

**mod.rs trajectory:** 23,413 → 20,758 → 19,070 → target 18,600

---

## P3 Future Work: Pointer Recall Command

**Goal:** Query stored Stage0 pointers for traceability

```bash
code stage0 recall SPEC-KIT-102
# Shows: task brief hash, divine truth hash, paths, tier2 status, commit SHA
```

**Implementation Notes:**
- Query local-memory REST API: `GET /api/v1/search?tags=spec:SPEC-ID,stage:0`
- Include `system:true` explicitly (normally excluded)
- Display in table format with timestamps

**Status:** Documented for future implementation (not in next session scope)

---

## Quick Verify Commands

```bash
# Stage0 tests
cargo test -p codex-stage0 -- convergence

# Full TUI tests (verify extraction didn't break anything)
cargo test -p codex-tui

# Health check
code doctor

# Check mod.rs line count
wc -l codex-rs/tui/src/chatwidget/mod.rs
```

---

## Next Session Start Prompt

Copy this into a new session:

```
load HANDOFF.md

## Session Goal: MAINT-11 Phase 9 — Undo/Snapshots Extraction

Previous session completed:
- MAINT-12: Stage0 HTTP-only (removed dead Tier2McpAdapter)
- Convergence verification: All 6 criteria passing
- MAINT-11 assessment: mod.rs at 19,070 LOC

## Primary Task

Extract Undo/Snapshots module (~460 LOC) from mod.rs:
- Target file: `tui/src/chatwidget/undo_snapshots.rs`
- Key functions: GhostSnapshot, capture_ghost_snapshot, handle_undo_command,
  show_undo_*, perform_undo_restore, undo_jump_back
- Goal: mod.rs < 18,600 LOC

## Extraction Pattern (follow existing modules)

1. Create new file with module doc and imports
2. Move struct definitions first (GhostSnapshot, GhostSnapshotsDisabledReason)
3. Move helper functions
4. Move impl methods (may need trait or extension pattern)
5. Add `pub(crate) mod undo_snapshots;` to mod.rs
6. Re-export needed types
7. Run tests: `cargo test -p codex-tui`

## After Extraction

1. Verify tests: `cargo test -p codex-tui` (expect 604+ passing)
2. Check line count: `wc -l tui/src/chatwidget/mod.rs`
3. Update SPEC.md with Phase 9 completion
4. Commit with conventional format

## Reference Files

- Existing extraction: `tui/src/chatwidget/session_handlers.rs` (619 LOC, Phase 8)
- Target functions: mod.rs lines 4666-5140, 10882
- Struct definitions: mod.rs lines 309-340, 589-680
```

---

## Key Files Reference

| File | Purpose |
|------|---------|
| `tui/src/chatwidget/mod.rs` | Main refactor target (19,070 LOC) |
| `tui/src/chatwidget/session_handlers.rs` | Reference extraction (Phase 8) |
| `stage0/tests/convergence_acceptance.rs` | Convergence test suite |
| `cli/src/main.rs` | `code doctor` implementation |
| `SPEC.md` | Task tracking (MAINT-11 row) |

---

## Commits This Session

```
ada1eed7c docs(convergence): add codex-rs MEMO and PROMPT pointers
15ac70e5c docs(spec): mark MAINT-12 complete
db3985846 refactor(stage0): remove dead Tier2McpAdapter code (MAINT-12)
```

Push when ready: `git push`
