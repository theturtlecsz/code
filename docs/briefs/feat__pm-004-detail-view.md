# Brief: PM-004 Detail View (Read-Only)

## Scope
Land the PM-004 TUI detail-view slice only: read-only detail rendering and key handling.

## What Changed
- Added PM overlay detail-mode rendering and scroll behavior in `codex-rs/tui/src/chatwidget/pm_overlay.rs`.
- Added mode-aware key handling (list vs detail) in `codex-rs/tui/src/chatwidget/pm_handlers.rs`.

## Constraints Applied
- No pm-service protocol or CLI changes in this branch.
- No write actions from detail view (no state transitions, no run launch).

## Decision/Spec References
- `docs/DECISIONS.md`: D137 (full PM hierarchy), D140 (hybrid holding states).
- `docs/SPEC-PM-004-tui-ux/spec.md`: PM-UX-D6 (overlay behavior), PM-UX-D20 (detail panel structure/behavior).

<!-- BEGIN: SPECKIT_BRIEF_REFRESH -->

## Product Knowledge (manual)

- Query: `pm-004 detail view read-only overlay key handling`
- Domain: `spec-kit`
- Capsule URI: `mv2://default/WORKFLOW/brief-20260213T170400Z/artifact/briefs/feat__pm-004-detail-view/20260213T170400Z.md`
- Capsule checkpoint: `brief-feat__pm-004-detail-view-20260213T170400Z`

Locked refs: D137, D138, D140; PM-UX-D6, PM-UX-D20.

<!-- END: SPECKIT_BRIEF_REFRESH -->
