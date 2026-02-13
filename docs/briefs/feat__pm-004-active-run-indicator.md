# feat/pm-004-active-run-indicator

## Objective

Add a read-only "active run" indicator to the PM overlay list and detail views,
and make the summary bar's "Active runs" count reflect computed data instead of
a hardcoded zero.

## Changes

### `codex-rs/tui/src/chatwidget/pm_overlay.rs`

* **Demo tree**: TASK-005 (node 10) now has `latest_run_status: "running"` with
  `latest_run: "run-038"` to provide at least one active-run node.
* **Summary bar**: `Active runs: N` computed by counting nodes with
  `latest_run_status == "running"`. Run meter line 2 uses the same count.
  Non-zero counts styled with accent color.
* **Wide list (>=120 cols)**: Latest Run column shows `▶ run-ID` for running
  nodes, distinct from `! run-ID` used by needs\_attention.
* **Medium/narrow (<120 cols)**: State column shows ` ▶` suffix for running
  nodes, distinct from ` !` used by needs\_attention.
* **Detail Run History**: Running status displayed as "running" with accent
  color styling (no conflict summary — that's needs\_attention only).
* **Tests**: 7 new tests covering demo tree running node, summary bar count,
  wide/medium/narrow indicators, and detail content for running status.

## Spec References

* PM-UX-D3 (30s-to-truth Q6: active runs)
* PM-UX-D17 (summary bar)
* Status Indicators table (Active run row)

## Decisions

* D113, D138, D143

## Touch Budget

* Files: 2 (pm\_overlay.rs + this brief)
* LOC delta: \~130

<!-- BEGIN: SPECKIT_BRIEF_REFRESH -->

## Product Knowledge (manual)

* Query: `pm-004 active-run indicator summary bar PM-UX-D3 PM-UX-D17`
* Domain: `spec-kit`
* Capsule URI: `mv2://default/WORKFLOW/brief-20260213T214000Z/artifact/briefs/feat__pm-004-active-run-indicator/20260213T214000Z.md` (placeholder)
* Capsule checkpoint: `brief-feat__pm-004-active-run-indicator-20260213T214000Z` (placeholder)

Locked refs: D113, D138, D143; PM-UX-D3, PM-UX-D17.

<!-- END: SPECKIT_BRIEF_REFRESH -->
