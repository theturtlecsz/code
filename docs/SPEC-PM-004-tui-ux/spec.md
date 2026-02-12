# SPEC-PM-004: TUI PM UX/UI

## Status: PLANNED (interaction design)

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
* Mobile or remote access.

## Constraints

* **Tier-1 parity** (D113/D133): TUI PM behavior must match CLI/headless semantics for all Tier-1 operations.
* **Headless never prompts** (D133): TUI degradation must not block headless/CLI workflows.
* **Safety signals**: explicit indicators when write mode is enabled or a run is operating in degraded mode.

## Dependencies

* PM-001: work item schema (fields determine what's shown in list/detail views).
* PM-002: run config surface (presets/scopes determine checkbox options).
* PM-003: service status + checkpoint data (determines what's available for display).

## Views

### List View (PM "home screen")

The primary entry point for PM. Must answer "what's the state of my project?" in under 30 seconds.

**Columns** (proposed):

| Column       | Source                 | Notes                                          |
| ------------ | ---------------------- | ---------------------------------------------- |
| ID           | work\_item.id          | SPEC-ID format; clickable to detail view       |
| Type         | work\_item.type        | Icon or badge: project/feature/spec/task       |
| Title        | work\_item.title       | Truncated with ellipsis                        |
| State        | work\_item.state       | Color-coded badge (holding states highlighted) |
| Last Updated | work\_item.updated\_at | Relative time ("2h ago")                       |
| Latest Run   | latest bot run summary | Kind + status + short summary (if any)         |

**Behaviors**:

* Filter by: hierarchy level (type), lifecycle state, has active run
* Sort by: updated\_at (default), state, type
* Keyboard: arrow keys navigate, Enter opens detail, `/` to filter, `q` to exit

### Detail View

Selected work item's full context. Must answer "what's happening with this item?" immediately.

**Sections** (proposed):

1. **Header**: ID + title + state badge + last updated
2. **Metadata**: type, parent, priority (if feature), quality score (if spec), prd\_uri (if spec)
3. **State Controls**: lifecycle transition buttons/shortcuts (e.g., "Move to Planned", "Start Research Run")
4. **Bot Run History**: table of runs (run\_id, kind, preset, status, started\_at, duration)
5. **Latest Checkpoint** (if run active): phase, summary, percent, resume\_hint
6. **Run Configuration**: preset selector + scope checkboxes (for launching new runs)

**Behaviors**:

* Keyboard: `r` to start research run, `v` to start review run, `c` to cancel active run, `Esc` to return to list
* State transitions: keyboard shortcuts for common transitions
* Run launch: opens config panel (preset + scopes), then submits

### Degraded Mode

When PM service is unavailable:

* List and detail views switch to **read-only** (data from capsule or local cache; PM-D35)
* State controls and run launch are **disabled** with a visible "Service unavailable" indicator
* Service management actions available: start, stop, doctor

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

| State              | Indicator                                   | Notes                                                               |
| ------------------ | ------------------------------------------- | ------------------------------------------------------------------- |
| Normal lifecycle   | State badge (green/blue/grey)               | Backlog=grey, Planned=blue, InProgress=green, Completed=green-check |
| Holding state      | Amber badge + "NeedsResearch"/"NeedsReview" | Distinct from normal states                                         |
| Active run         | Spinner or progress bar                     | Visible in both list and detail                                     |
| needs\_attention   | Yellow/warning badge + detail on select     | Auto-shows conflict summary in detail view                          |
| Degraded mode      | Banner: "Service unavailable — read-only"   | Persistent until service reconnects                                 |
| Write mode enabled | Warning badge on run entry                  | Explicit safety signal                                              |

## Open Questions

* Exact keyboard shortcut map (conflicts with existing TUI shortcuts?)
* How to represent the 4-level hierarchy in the list view (tree vs flat with indentation vs tabs?)
* Should the detail view be a sidebar, overlay, or full-screen replacement?
* How to handle work items that span hundreds of entries (pagination vs virtual scroll?)

## References

* Data model: `docs/SPEC-PM-001-project-management/PRD.md`
* Caller contract: `docs/SPEC-PM-002-bot-runner/spec.md`
* Service/runtime: `docs/SPEC-PM-003-bot-system/spec.md`
* Design transcript: `docs/SPEC-PM-003-bot-system/design-qa-transcript.md`
