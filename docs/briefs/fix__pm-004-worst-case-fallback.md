# Branch Brief: fix/pm-004-worst-case-fallback

**Date**: 2026-02-13
**Spec**: SPEC-PM-004 (PM-UX-D15)
**Decisions**: D113, D138, D143

## Objective

Implement the worst-case fallback UI for the PM overlay when the service is
degraded and no PM data is available (cache missing, capsule inaccessible).

## Changes

* `codex-rs/tui/src/chatwidget/pm_overlay.rs`:
  * Added `render_worst_case_fallback()` helper rendering three diagnostic lines
    (Service/Cache/Capsule status) and two remedy commands (`/pm service doctor`,
    `systemctl --user start codex-pm-service`).
  * Wired into list-mode rendering: when `overlay.degraded && overlay.nodes.is_empty()`,
    renders the full-screen error state instead of summary+list.
  * Added `new_degraded_empty()` test constructor for empty/degraded overlays.
  * Added unit test asserting diagnostic and remedy lines are present in rendered buffer.

## Constraints

* PM-UX-D14 (degraded banner) behavior is preserved unchanged.
* PM views remain read-only (no state transitions, no run launching).
* No changes to pm-service protocol or CLI behavior.

<!-- BEGIN: SPECKIT_BRIEF_REFRESH -->

## Product Knowledge (manual)

* Query: `pm-004 worst-case fallback degraded empty`
* Domain: `spec-kit`
* Capsule URI: `mv2://default/WORKFLOW/brief-20260213T200800Z/artifact/briefs/fix__pm-004-worst-case-fallback/20260213T200800Z.md`
* Capsule checkpoint: `brief-fix__pm-004-worst-case-fallback-20260213T200800Z`

Locked refs: D113, D138, D143; PM-UX-D15.

<!-- END: SPECKIT_BRIEF_REFRESH -->
