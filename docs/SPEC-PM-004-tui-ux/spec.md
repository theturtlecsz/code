# SPEC-PM-004: TUI PM UX/UI

## Status: PLANNED (interaction design)

## Design / Research Inputs (supporting)

These documents capture durable context from design sessions. They are **not** part
of the locked decision register unless also listed in `docs/DECISIONS.md`.

* Design Q\&A transcript (2026-02-12): `design-qa-transcript.md` -- 15 UX decisions (PM-UX-D1 through PM-UX-D15)
* Follow-up resolutions (2026-02-12): PM-UX-D16 through PM-UX-D25 (this spec; previously Open Questions)

## Overview

Define the **TUI interaction design** for the PM layer: views, navigation flows, information hierarchy, key behaviors, and degraded-mode UX.

Extracted from SPEC-PM-001 (D138/PM-D26). PM-001 owns the data model; PM-004 owns how it's presented and interacted with in the TUI.

## Goals

* Define the PM "home screen" (list view) and what information it surfaces.
* Define the work item detail view (state, PRD links, bot run history, checkpoint summaries).
* Define navigation between views, filtering, sorting, and keyboard-driven workflows.
* Define run configuration UX (preset selection + scope toggles as checkboxes).
* Define degraded-mode behavior when the PM service is unavailable.
* Define status indicators (holding states, active runs, degraded mode, needs\_attention).

## Non-Goals (v1)

* Visual design system / styling (that's implementation detail).
* Web UI (deferred to v2).
* Multi-user / collaboration (v1 is solo developer only; PM-UX-D2).
* Mobile or remote access.

## Constraints

* **Tier-1 parity** (D113/D133): TUI PM behavior must match CLI/headless semantics for all Tier-1 operations.
* **Headless never prompts** (D133): TUI degradation must not block headless/CLI workflows.
* **Safety signals**: explicit indicators when write mode is enabled or a run is operating in degraded mode.
* **Accessibility** (PM-UX-D25): provide a high-contrast mode toggle (theme-level); do not rely on color alone for meaning.

## Dependencies

* PM-001: work item schema (fields determine what's shown in list/detail views).
* PM-002: run config surface (presets/scopes determine checkbox options).
* PM-003: service status + checkpoint data (determines what's available for display).

## Views

### List View (PM "home screen")

The primary entry point for PM (PM-UX-D1). Must answer "what's the state of my project?" in under 30 seconds.

**"30 seconds to truth"** (PM-UX-D3): Six questions answerable from the list view without opening detail:

1. What's in progress?
2. What's blocked / needs attention?
3. Last bot run result per item
4. What changed most recently?
5. Count by lifecycle state (summary bar)
6. Which items have active runs (spinner/indicator)

**Hierarchy**: Tree/outline with expand/collapse (PM-UX-D4). Items shown hierarchically with indentation (Project > Feature > SPEC > Task). The hierarchy is the primary information architecture.

**Default expand state** (PM-UX-D16): All nodes are collapsed on first load. The user expands nodes explicitly (Right arrow) to reveal children.

**Adaptive columns** (PM-UX-D5): Column set adapts to terminal width. Type is implied by tree indentation with icon/badge prefix.

| Priority | Column       | Source                 | Notes                                          | Visible     |
| -------- | ------------ | ---------------------- | ---------------------------------------------- | ----------- |
| 1        | ID           | work\_item.id          | SPEC-ID format; Enter opens detail view        | Always      |
| 2        | Title        | work\_item.title       | Truncated with ellipsis                        | Always      |
| 3        | State        | work\_item.state       | Color-coded badge (holding states highlighted) | Always      |
| 4        | Last Updated | work\_item.updated\_at | Relative time ("2h ago")                       | >= 80 cols  |
| 5        | Latest Run   | latest bot run summary | Kind + status + short summary (if any)         | >= 120 cols |

**Summary bar** (PM-UX-D17): Two-line summary bar at the top of list view.

* **Line 1**: lifecycle state count chips + active run count + degraded indicator (when applicable)
* **Line 2**: compact active-run meter (aggregate progress if available, otherwise count-only)

**Behaviors**:

* Filter by: hierarchy level (type), lifecycle state, has active run
* Filter/tree interaction (PM-UX-D18): hide non-matching nodes, but keep ancestors of matches visible so results remain in context
* Sort by: updated\_at (default), state, type
* Keyboard: arrow keys navigate up/down, Left/Right collapse/expand tree, Enter opens detail, Esc exits (PM-UX-D12)

**Performance** (PM-UX-D19): Always-virtualize the list rows (render only visible rows) so the view remains responsive at large item counts.

### Detail View

Full-screen replacement (PM-UX-D6). `Esc` returns to list with cursor position preserved. Matches established TUI patterns (lazygit, tig).

Selected work item's full context. Must answer "what's happening with this item?" immediately.

**Layout** (PM-UX-D20): Fixed header at top + pinned run configuration section, with a scrollable middle region (metadata, state controls, run history, checkpoints).

**Sections**:

1. **Header**: ID + title + state badge + last updated
2. **Metadata**: type, parent, priority (if feature), quality score (if spec), prd\_uri (if spec)
3. **State Controls**: three hot-path transition shortcuts (PM-UX-D8):
   * **promote** -- moves item forward one natural step (Backlog -> Planned -> InProgress -> Completed)
   * **hold** -- enters NeedsResearch or NeedsReview (prompts which), records return\_state (D140)
   * **complete** -- marks Completed (or Failed for tasks)
   * Deprecated/Archived via command menu only (rare operations)
4. **Bot Run History**: table of runs (run\_id, kind, preset, status, started\_at, duration)
5. **Latest Checkpoint** (if run active): phase, summary, percent, resume\_hint
6. **Run Configuration** (PM-UX-D9/PM-UX-D21): pinned section, always visible. Preset as radio group (`quick`/`standard`/`deep`/`exhaustive`), scopes as checkboxes (all default ON). Defaults: `standard` preset, all scopes enabled
   * **Launch actions** (PM-UX-D21): two explicit launch actions: **Run Research** and **Run Review** (no bot-kind radio toggle)
   * **Write mode toggle** (PM-UX-D10): inline toggle in run config section, defaults OFF. When ON: visual danger indicator. Launch requires double-press confirmation ("Confirm write mode?" on first press, submit on second)

**Behaviors**:

* Keyboard: arrow-only navigation (PM-UX-D12), Enter to interact, Esc to return to list
* State transitions: promote/hold/complete hot keys (see Keyboard Model)
* Run launch: pinned run config section (preset + scopes) + explicit **Run Research** / **Run Review** actions

### Degraded Mode

**Service unavailable** (PM-UX-D14): Progressive fallback with clear indication:

1. TUI reads **local cache** if present and non-stale
2. Falls back to **capsule direct read** if cache is missing/stale
3. Persistent banner at top of list/detail: "PM service unavailable -- read-only"
4. All mutation controls (state transitions, run launch, write-mode toggle) **visually disabled** (greyed out)
5. `/pm service status` and `/pm service doctor` remain functional
6. Summary bar shows a degraded indicator alongside state counts

Concretizes D143 (cache reads, capsule writes) and PM-D19 (read-only when service down).

**Worst-case fallback** (PM-UX-D15): When service is down AND cache is missing/stale AND capsule access fails, the PM tree/detail views are completely unavailable. A full-screen error state displays three diagnostic lines:

1. Service status (not running / connection refused)
2. Cache status (missing / stale / corrupt)
3. Capsule status (locked / unreadable / missing)

Below diagnostics: actionable remedies ("Run `/pm service doctor`", "Run `systemctl --user start codex-pm-service`"). No partial/stale data shown -- clear diagnostics help user self-recover.

## Run Configuration UX (PM-D31)

**Preset selector**: dropdown or radio group

* `quick`: surface-level checks, minutes
* `standard` (default): balanced, minutes to hours
* `deep`: thorough, hours
* `exhaustive`: comprehensive multi-pass, hours to days

**Scope toggles**: checkboxes (all checked by default)

* `correctness`
* `security`
* `performance`
* `style`
* `architecture`

## Status Indicators

| State              | Indicator                                                        | Notes                                                                                                                                                            |
| ------------------ | ---------------------------------------------------------------- | ---------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Normal lifecycle   | State badge (green/blue/grey)                                    | Backlog=grey, Planned=blue, InProgress=green, Completed=green-check                                                                                              |
| Holding state      | Amber badge + "NeedsResearch"/"NeedsReview" + return destination | Shows return state (PM-UX-D8/D140): e.g., `NeedsReview (-> InProgress)`                                                                                          |
| Active run         | Spinner or progress bar                                          | Visible in both list and detail                                                                                                                                  |
| needs\_attention   | Warning badge in Latest Run column (PM-UX-D11)                   | At narrow widths: secondary indicator on State column. Detail auto-scrolls to attention-requiring run, expands conflict summary + resolution instructions inline |
| Degraded mode      | Banner: "PM service unavailable -- read-only"                    | Persistent until service reconnects; summary bar degraded indicator                                                                                              |
| Write mode enabled | Danger indicator on inline toggle (PM-UX-D10)                    | Explicit safety signal at point of action; double-press confirmation                                                                                             |

## Keyboard Model (PM-UX-D12)

Arrow-only navigation model. No vim-style `j`/`k` keys.

* **List view**: Arrow Up/Down to navigate, Left/Right to collapse/expand tree, Enter to open detail, Esc to exit
* **Detail view**: Arrow Up/Down to scroll sections, Esc to return to list
* **Action keys** (PM-UX-D22): function keys (no ctrl-letter bindings in PM views)

**Default action key map** (PM-UX-D22):

* `F5`: Promote
* `F6`: Hold (NeedsResearch/NeedsReview selector)
* `F7`: Complete
* `F8`: Run Research
* `F9`: Run Review
* `F10`: Cancel active run (if applicable)

## Slash Command Surface (PM-UX-D13)

**Interactive entrypoint** (PM-UX-D23): `/pm open` opens the interactive PM list/detail views. This is TUI-only (no CLI equivalent). All `/pm bot ...` commands remain text-mode output to chat history.

Full v1 CLI parity. All 8 PM-002 commands remain available as `/pm` slash commands (text-mode):

| Command              | CLI equivalent                                     | Output mode |
| -------------------- | -------------------------------------------------- | ----------- |
| `/pm bot run`        | `code speckit pm bot run --id <ID> ...`            | Text        |
| `/pm bot status`     | `code speckit pm bot status --id <ID>`             | Text        |
| `/pm bot runs`       | `code speckit pm bot runs --id <ID>`               | Text        |
| `/pm bot show`       | `code speckit pm bot show --id <ID> --run <RID>`   | Text        |
| `/pm bot cancel`     | `code speckit pm bot cancel --id <ID> --run <RID>` | Text        |
| `/pm bot resume`     | `code speckit pm bot resume --id <ID> --run <RID>` | Text        |
| `/pm service status` | `code speckit pm service status`                   | Text        |
| `/pm service doctor` | `code speckit pm service doctor`                   | Text        |

These are text-mode (output to chat history), separate from the interactive tree/detail views. Strict 1:1 semantic match with `code speckit pm ...` CLI (D113/D133).

## Empty State (PM-UX-D7)

When there are zero work items, the list view triggers a guided onboarding wizard (PM-UX-D24):

1. Confirm project container (default: repo name)
2. Choose the first work item type (Feature or SPEC)
3. Enter title/name, then begin maieutic intake

Wizard must respect D130 (maieutic step) and D113 (Tier-1 parity). TUI flow is sugar over CLI semantics.

## Open Questions

None (as of 2026-02-12).

## References

* Data model: `docs/SPEC-PM-001-project-management/PRD.md`
* Caller contract: `docs/SPEC-PM-002-bot-runner/spec.md`
* Service/runtime: `docs/SPEC-PM-003-bot-system/spec.md`
* PM-003 design transcript: `docs/SPEC-PM-003-bot-system/design-qa-transcript.md`
* PM-004 design transcript: `design-qa-transcript.md`
