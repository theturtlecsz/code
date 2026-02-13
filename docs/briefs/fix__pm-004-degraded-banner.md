# Session Brief — fix/pm-004-degraded-banner

## Goal
Align PM degraded banner UX to PM-UX-D14 with exact text in both list and detail views.

## Scope / Constraints
- No pm-service protocol or CLI behavior changes.
- Keep PM detail view read-only.
- Touch only `codex-rs/tui/src/chatwidget/pm_overlay.rs` plus this brief.

## Decision / Spec Locks
- `docs/DECISIONS.md`: D113, D138, D143 (Already Locked)
- `docs/SPEC-PM-004-tui-ux/spec.md`: PM-UX-D14, PM-UX-D6/D12/D20

<!-- BEGIN: SPECKIT_BRIEF_REFRESH -->

## Product Knowledge (manual)

- Query: `pm-004 degraded banner parity read-only detail list`
- Domain: `spec-kit`
- Capsule URI: `mv2://default/WORKFLOW/brief-20260213T182100Z/artifact/briefs/fix__pm-004-degraded-banner/20260213T182100Z.md`
- Capsule checkpoint: `brief-fix__pm-004-degraded-banner-20260213T182100Z`

Locked refs: D113, D138, D143; PM-UX-D14.

<!-- END: SPECKIT_BRIEF_REFRESH -->
