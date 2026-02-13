# Branch Brief: feat/pm-004-needs-attention

**Date**: 2026-02-13
**Spec**: SPEC-PM-004 (PM-UX-D11)
**Decisions**: D113, D138, D143

## Objective

Implement PM-UX-D11 needs_attention indicator and detail auto-scroll (read-only).

## Changes

- Added `latest_run_status` field to `TreeNode` for tracking run state.
- Added `detail_auto_scroll_pending` flag to `PmOverlay`.
- Wide list view: `"! "` warning-color prefix on Latest Run column for needs_attention nodes.
- Medium/narrow list views: `" !"` warning-color suffix on State column.
- Detail view: conditional status text (`needs_attn`), conflict summary, resolution instructions.
- Auto-scroll: detail view scrolls to Run History section when opening a needs_attention node.
- Added 6 unit tests covering indicators, auto-scroll, and detail content.

## Constraints

- No pm-service protocol or CLI changes.
- PM overlay remains read-only: no state transitions, no run launching.

## Files

- `codex-rs/tui/src/chatwidget/pm_overlay.rs`

<!-- BEGIN: SPECKIT_BRIEF_REFRESH -->

## Product Knowledge (manual)

- Query: `pm-004 needs_attention indicator auto-scroll PM-UX-D11`
- Domain: `spec-kit`
- Capsule URI: `mv2://default/WORKFLOW/brief-20260213T204000Z/artifact/briefs/feat__pm-004-needs-attention/20260213T204000Z.md` (placeholder)
- Capsule checkpoint: `brief-feat__pm-004-needs-attention-20260213T204000Z` (placeholder)

Locked refs: D113, D138, D143; PM-UX-D11.

<!-- END: SPECKIT_BRIEF_REFRESH -->
