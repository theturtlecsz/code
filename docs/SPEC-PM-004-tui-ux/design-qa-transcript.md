# PM-004 TUI UX Design Q\&A Transcript

**Session**: `f3e54c6e-819b-4173-9ce5-588944d1e4f4`
**Date**: 2026-02-12
**Participants**: System Architect (Claude) + Product Owner (thetu)
**Scope**: SPEC-PM-004 (TUI PM UX/UI)

***

## Table of Contents

* [Already Locked Constraints](#already-locked-constraints)
  * [From docs/DECISIONS.md](#from-docsdecisionsmd)
  * [From PM-003 Design Transcript (locked but not D-numbered)](#from-pm-003-design-transcript-locked-but-not-d-numbered)
* [Phase 1: North Star + Week-1 Proof](#phase-1-north-star--week-1-proof)
  * [Q1: Single Most Important Week-1 Outcome](#q1-single-most-important-week-1-outcome)
  * [Q2: Anti-Goals for v1](#q2-anti-goals-for-v1)
  * [Q3: "30 Seconds to Truth"](#q3-30-seconds-to-truth)
* [Phase 2: Information Architecture](#phase-2-information-architecture)
  * [Q4: 4-Level Hierarchy Representation](#q4-4-level-hierarchy-representation)
  * [Q5: List View Columns](#q5-list-view-columns)
  * [Q6: Detail View Layout](#q6-detail-view-layout)
  * [Q7: Empty State UX](#q7-empty-state-ux)
* [Phase 3: Workflows](#phase-3-workflows)
  * [Q8: State Transition Hot Paths](#q8-state-transition-hot-paths)
  * [Q9: Bot Run Launch Flow](#q9-bot-run-launch-flow)
  * [Q10: Review Write-Mode Safety Signals](#q10-review-write-mode-safety-signals)
  * [Q11: needs\_attention UX](#q11-needs_attention-ux)
* [Phase 4: Keyboard UX + Slash Commands](#phase-4-keyboard-ux--slash-commands)
  * [Q12: Keyboard Shortcut Map](#q12-keyboard-shortcut-map)
  * [Q13: Slash Command Parity](#q13-slash-command-parity)
* [Phase 5: Degraded Mode + Cache Trust Model](#phase-5-degraded-mode--cache-trust-model)
  * [Q14: Service Unavailable Behavior](#q14-service-unavailable-behavior)
  * [Q15: Offline/Partial Data Fallback](#q15-offlinepartial-data-fallback)
* [Phase 6: Closeout](#phase-6-closeout)
  * [Decision Summary](#decision-summary)
  * [Open Questions](#open-questions)
  * [Next READY Work Package](#next-ready-work-package)

## Already Locked Constraints

These locked decisions from `docs/DECISIONS.md` and the PM-003 design transcript constrained the design:

### From docs/DECISIONS.md

| D#       | Constraint                                                   | Impact on PM-004                                                        |
| -------- | ------------------------------------------------------------ | ----------------------------------------------------------------------- |
| **D7**   | Single-writer capsule                                        | All mutations serialized through service; TUI never writes directly     |
| **D113** | Tier-1 parity (TUI/CLI/headless)                             | Every TUI PM operation must have a CLI equivalent                       |
| **D114** | Events + artifacts are authoritative SoR                     | List/detail views may use cache but capsule is truth                    |
| **D119** | Over-capture hard-blocked                                    | TUI must never surface raw LLM I/O that violates capture policy         |
| **D130** | Maieutic step mandatory                                      | TUI create/update flows must not bypass maieutic gates                  |
| **D133** | Headless never prompts; multi-surface parity                 | TUI degradation must not block CLI/headless; safety signals visual only |
| **D135** | Lightweight persistent service (systemd)                     | TUI talks to a service, not directly to capsule for writes              |
| **D136** | Unix domain socket IPC                                       | TUI IPC is the same socket as CLI                                       |
| **D137** | Full 4-level hierarchy (Projects > Features > SPECs > Tasks) | List view must represent all 4 levels                                   |
| **D138** | TUI PM UX = SPEC-PM-004                                      | This spec owns the interaction design; PM-001 owns data model only      |
| **D139** | BLOCKED = exit 2 + structured reason                         | TUI must surface blocked reason visually                                |
| **D140** | Hybrid holding states with `return_state` auto-return        | TUI must show holding state badge AND the return destination            |
| **D141** | Service starts on login                                      | TUI should never need to "first start" the service in normal flow       |
| **D142** | Service stays running (idle OK)                              | No "service sleeping" states to worry about                             |
| **D143** | Cache reads, capsule writes                                  | TUI reads local cache for speed; writes go through service              |

### From PM-003 Design Transcript (locked but not D-numbered)

| PM-D#      | Constraint                                      | Notes                                   |
| ---------- | ----------------------------------------------- | --------------------------------------- |
| **PM-D2**  | Projects = container only (no lifecycle state)  | List view: projects have no state badge |
| **PM-D3**  | Features + SPECs share full 8-state lifecycle   | Same state badge set for both           |
| **PM-D4**  | Tasks = ternary (open/completed/failed)         | Simpler badge for tasks                 |
| **PM-D12** | Runs long-lived; checkpoint summaries on demand | Status is pulled, not pushed            |
| **PM-D15** | Status queried on demand, not live-streamed     | No live tail in v1                      |
| **PM-D19** | TUI degrades to read-only when service down     | Must clearly indicate degraded mode     |
| **PM-D20** | Service management: start, stop, doctor         | Available even in degraded mode         |
| **PM-D31** | Presets + scopes as checkboxes                  | Run config UX must surface both         |

***

## Phase 1: North Star + Week-1 Proof

### Q1: Single Most Important Week-1 Outcome

**Decision being made**: What the PM TUI must deliver in its first usable release to prove value.

**Options**:

* A) **List + detail views with real data** (Recommended) -- the "30 seconds to truth" promise is the north star; run-launch and mutation UX come after the read path is solid
* B) Run bots + see status
* C) State transitions + review queue
* D) CRUD lifecycle

**User chose: A**

**PM-UX-D1 (Proposed)**: Week-1 proof is the list + detail views with real data. The "30 seconds to truth" promise is the north star; run-launch and mutation UX come after the read path is solid.

***

### Q2: Anti-Goals for v1

**Decision being made**: What is explicitly out of scope for PM-004 v1 UX.

**Options**:

* A) Conservative v1 -- exclude: multi-user, drag-drop, inline editing, complex sorting, live-streaming
* B) **Minimal exclusions** -- only multi-user and web UI (Recommended by user)
* C) Aggressive exclusions -- all of A, plus no run-launch or state transitions from TUI

**User chose: B**

**PM-UX-D2 (Proposed)**: Anti-goals for v1 are limited to: (1) no multi-user/collaboration, (2) no web UI. Everything else is fair game if time permits.

***

### Q3: "30 Seconds to Truth"

**Decision being made**: Exactly which questions the list view must answer without opening detail view.

**Options**:

* A) Four questions -- in-progress, blocked/needs-attention, last run result, what changed
* B) **Six questions** -- all of A, plus state counts and active runs
* C) Two questions -- only in-progress and needs-attention

**User chose: B**

**PM-UX-D3 (Proposed)**: "30 seconds to truth" means six questions answerable from the list view without opening detail:

1. What's in progress?
2. What's blocked/needs attention?
3. Last bot run result per item
4. What changed most recently?
5. Count by lifecycle state (summary bar)
6. Which items have active runs (spinner/indicator)

**UX implication**: Requires both per-row indicators (state badge, run status, updated\_at) AND a summary/count bar visible in the list view.

***

## Phase 2: Information Architecture

### Q4: 4-Level Hierarchy Representation

**Decision being made**: How the Projects > Features > SPECs > Tasks hierarchy appears in the list view.

**Options**:

* A) Flat table + type column + filter chips
* B) **Tree/outline with expand/collapse** (Recommended)
* C) Tabs per type
* D) Grouped flat list

**User chose: B**

**PM-UX-D4 (Proposed)**: List view uses a tree/outline with expand/collapse. Items shown hierarchically with indentation (Project > Feature > SPEC > Task). Expand/collapse via keyboard. The hierarchy is the primary information architecture.

**Follow-up implications**: Keyboard behavior for expand/collapse, default expand state, filter interaction with tree.

***

### Q5: List View Columns

**Decision being made**: Which columns appear in the list view, their order, and how they adapt to terminal width.

**Options**:

* A) Six columns as proposed -- ID, Type, Title, State, Last Updated, Latest Run
* B) Five columns, drop Type -- tree indentation implies type
* C) Four columns, minimal
* D) **Adaptive** -- 4-6 columns depending on terminal width

**User chose: D**

**PM-UX-D5 (Proposed)**: Adaptive column layout based on terminal width. Full set is ID, Title, State, Last Updated, Latest Run (5 columns, Type implied by tree indentation with icon/badge prefix). Narrow terminals drop Latest Run first, then Last Updated. Minimum viable: ID, Title, State.

**Column priority order**:

1. ID (always)
2. Title (always)
3. State (always)
4. Last Updated (drop first at narrow widths)
5. Latest Run (drop second at narrow widths)

***

### Q6: Detail View Layout

**Decision being made**: Whether the detail view is a sidebar, overlay, or full-screen replacement.

**Options**:

* A) **Full-screen replacement** (Recommended) -- Esc returns to list
* B) Right sidebar (30-40% width)
* C) Bottom panel
* D) Overlay/modal

**User chose: A**

**PM-UX-D6 (Proposed)**: Detail view is full-screen replacement. `Esc` returns to list with cursor position preserved. Matches established TUI patterns (lazygit, tig).

**Detail view sections**:

1. **Header**: ID + title + state badge + last updated
2. **Metadata**: type, parent, priority (feature), quality score (spec), prd\_uri (spec)
3. **State controls**: transition shortcuts
4. **Bot run history**: table of runs
5. **Latest checkpoint** (if run active)
6. **Run configuration** (for launching new runs)

***

### Q7: Empty State UX

**Decision being made**: What the user sees when there are zero work items.

**Options**:

* A) Actionable empty state -- show command to create first item
* B) **Guided onboarding** -- multi-step wizard
* C) Silent empty -- just show empty tree

**User chose: B**

**PM-UX-D7 (Proposed)**: Empty state triggers a guided onboarding wizard: prompts for project name, creates it, and shows the list with the first item.

**Implementation constraint**: Wizard must respect D130 (maieutic step) and D113 (Tier-1 parity). TUI flow is sugar over CLI semantics.

***

## Phase 3: Workflows

### Q8: State Transition Hot Paths

**Decision being made**: Which lifecycle transitions get single-keystroke shortcuts vs. command menu.

**Options**:

* A) **Promote/hold/complete as hot keys** (Recommended)
* B) Direct state shortcuts -- one key per target state
* C) Command palette only

**User chose: A**

**PM-UX-D8 (Proposed)**: Three hot-path transition shortcuts in detail view:

1. **promote** -- moves item forward one natural step (Backlog -> Planned, Planned -> InProgress, InProgress -> Completed)
2. **hold** -- enters NeedsResearch or NeedsReview (prompts which), records return\_state
3. **complete** -- marks Completed (or Failed for tasks)

Deprecated/Archived via command menu only (rare operations).

**D140 visualization**: Holding state badge shows return destination (e.g., `NeedsReview (-> InProgress)`).

**Note**: Actual key bindings deferred to Q12.

***

### Q9: Bot Run Launch Flow

**Decision being made**: Where the run configuration (preset + scope toggles) lives in the UX.

**Options**:

* A) **Inline section in detail view** (Recommended)
* B) Modal overlay
* C) Pre-configured defaults, confirm only

**User chose: A**

**PM-UX-D9 (Proposed)**: Run configuration is an inline section in the detail view, always visible. Preset as radio group, scopes as checkboxes. Launch key submits. Defaults: `standard` preset, all scopes enabled. No modal or context switch needed.

**Tier-1 parity note**: CLI equivalent is `code speckit pm bot run --id <ID> --kind <kind> --preset standard` with `--scope`/`--no-scope` flags.

***

### Q10: Review Write-Mode Safety Signals

**Decision being made**: How the TUI signals that write mode is enabled and what confirmation is required.

**Options**:

* A) **Inline toggle + confirmation step** (Recommended)
* B) Modal confirmation
* C) Persistent banner
* D) A + C combined

**User chose: A**

**PM-UX-D10 (Proposed)**: Write mode is an inline toggle in the run config section, defaults OFF. When ON: visual danger indicator on the toggle. Launch requires double-press confirmation ("Confirm write mode?" on first press, submit on second). No persistent banner -- safety signal lives at the point of action.

***

### Q11: needs\_attention UX

**Decision being made**: How `needs_attention` appears in list vs. detail and what happens on selection.

**Options**:

* A) **Badge in Latest Run column + auto-expand in detail** (Recommended)
* B) Promote to work item state
* C) Separate "attention queue" view

**User chose: A**

**PM-UX-D11 (Proposed)**: `needs_attention` shows as a distinct warning badge in the Latest Run column. At narrow widths where that column is hidden, the State column gets a secondary indicator. Detail view auto-scrolls to the attention-requiring run and expands conflict summary + resolution instructions inline. No modal, no state conflation.

**Already Locked alignment**: Consistent with PM-003 design transcript F3 decision ("distinct badge + detail on select").

***

## Phase 4: Keyboard UX + Slash Commands

### Q12: Keyboard Shortcut Map

**Decision being made**: Whether to use vim-style navigation, arrow-only, or hybrid.

**Options**:

* A) Vim-style primary + arrows as alias (Recommended by architect)
* B) **Arrow-only navigation** (Chosen by user)
* C) Hybrid with leader key

**User chose: B**

**PM-UX-D12 (Proposed)**: Arrow-only navigation model. No vim-style `j`/`k` keys. Arrow keys for up/down/collapse/expand in tree. Enter to open detail. Esc to go back. Action keys use ctrl-based or function-key patterns to avoid accidental triggers.

**Deferred**: Specific function/ctrl key bindings deferred to implementation (will be tested against existing TUI shortcut conflicts before finalizing).

***

### Q13: Slash Command Parity

**Decision being made**: Which `/pm` slash commands exist in v1 and whether they match CLI exactly.

**Options**:

* A) **Full v1 CLI parity** (Recommended) -- all PM-002 commands
* B) Service + status only
* C) Slash commands mirror interactive views

**User chose: A**

**PM-UX-D13 (Proposed)**: Full v1 CLI parity for `/pm` slash commands. All 8 PM-002 commands available:

1. `bot run`
2. `bot status`
3. `bot runs`
4. `bot show`
5. `bot cancel`
6. `bot resume`
7. `service status`
8. `service doctor`

These are text-mode (output to chat history), separate from the interactive tree/detail views. Strict 1:1 semantic match with `code speckit pm ...` CLI.

**Implementation note**: `/pm service status` and `/pm service doctor` already partially implemented. Remaining 6 bot commands need work item + run management RPC methods.

***

## Phase 5: Degraded Mode + Cache Trust Model

### Q14: Service Unavailable Behavior

**Decision being made**: What data source to use and what to show when the PM service is unavailable.

**Options**:

* A) **Cache-first, capsule fallback, clear banner** (Recommended)
* B) Capsule-only in degraded mode
* C) Show nothing, prompt to start service

**User chose: A**

**PM-UX-D14 (Proposed)**: Degraded mode behavior when service unavailable:

1. TUI reads local cache if present and non-stale
2. Falls back to capsule direct read if cache is missing/stale
3. Persistent banner at top of list/detail: "PM service unavailable -- read-only"
4. All mutation controls (state transitions, run launch, write-mode toggle) visually disabled (greyed out)
5. `/pm service status` and `/pm service doctor` remain functional
6. Summary bar shows a degraded indicator alongside state counts

Concretizes Already Locked D143 + PM-D19 for the UI layer.

***

### Q15: Offline/Partial Data Fallback

**Decision being made**: What happens when service is down AND cache is missing/stale AND capsule access fails.

**Options**:

* A) **Error state with diagnostics** (Recommended)
* B) Stale data with strong warning
* C) Fallback to SPEC.md parsing

**User chose: A**

**PM-UX-D15 (Proposed)**: Worst-case fallback is a full-screen error state with three diagnostic lines:

1. Service status (not running / connection refused)
2. Cache status (missing / stale / corrupt)
3. Capsule status (locked / unreadable / missing)

Below diagnostics: actionable remedies ("Run `/pm service doctor`", "Run `systemctl --user start codex-pm-service`").

PM tree/detail views completely unavailable -- no partial/stale data shown.

**Rationale**: Showing potentially corrupt or incomplete data is worse than showing nothing. Clear diagnostics help user self-recover.

***

## Phase 6: Closeout

### Decision Summary

| ID            | Decision                                             | User Choice | Status   |
| ------------- | ---------------------------------------------------- | ----------- | -------- |
| **PM-UX-D1**  | Week-1 proof = list + detail views with real data    | A           | Proposed |
| **PM-UX-D2**  | Anti-goals: only multi-user + web UI excluded        | B           | Proposed |
| **PM-UX-D3**  | "30s to truth" = 6 questions from list view          | B           | Proposed |
| **PM-UX-D4**  | Tree/outline with expand/collapse                    | B           | Proposed |
| **PM-UX-D5**  | Adaptive columns (5 columns, responsive drop)        | D           | Proposed |
| **PM-UX-D6**  | Detail = full-screen replacement, Esc returns        | A           | Proposed |
| **PM-UX-D7**  | Empty state = guided onboarding wizard               | B           | Proposed |
| **PM-UX-D8**  | Three hot-path transitions: promote/hold/complete    | A           | Proposed |
| **PM-UX-D9**  | Run config = inline section in detail view           | A           | Proposed |
| **PM-UX-D10** | Write mode = inline toggle, double-press confirm     | A           | Proposed |
| **PM-UX-D11** | needs\_attention = badge + auto-expand in detail     | A           | Proposed |
| **PM-UX-D12** | Arrow-only navigation + function/ctrl keys           | B           | Proposed |
| **PM-UX-D13** | Full v1 CLI parity for `/pm` slash commands          | A           | Proposed |
| **PM-UX-D14** | Degraded mode: cache-first, capsule fallback, banner | A           | Proposed |
| **PM-UX-D15** | Worst-case: full-screen error with diagnostics       | A           | Proposed |

### Open Questions

Resolved via follow-up (2026-02-12). See addendum below; `docs/SPEC-PM-004-tui-ux/spec.md` is now the canonical UX source.

| #      | Question                                                                                               | Priority | Blocking? |
| ------ | ------------------------------------------------------------------------------------------------------ | -------- | --------- |
| **1**  | Specific function/ctrl key bindings for actions (PM-UX-D12)                                            | High     | No        |
| **2**  | Tree default expand state on first load (all expanded? expand to SPEC level?)                          | High     | No        |
| **3**  | How do filters interact with tree? (hide non-matching branches? grey them? collapse to matching?)      | Medium   | No        |
| **4**  | Summary bar exact layout (state count chips? progress bar? text?)                                      | Medium   | No        |
| **5**  | Detail view section ordering and scrolling behavior (fixed header? all scrollable?)                    | Medium   | No        |
| **6**  | Run config section: how to select bot kind (research vs review) -- radio inline or part of launch key? | Medium   | No        |
| **7**  | Onboarding wizard exact flow (PM-UX-D7): how many steps? what fields?                                  | Low      | No        |
| **8**  | Pagination/virtual scroll threshold for large item counts (>100 items)                                 | Low      | No        |
| **9**  | Accessibility: screen reader compatibility for tree view                                               | Low      | No        |
| **10** | Should `/pm list` or `/pm open` open the interactive views, or only via a dedicated PM mode key?       | Medium   | **Yes**   |

### Addendum (2026-02-12): Open Questions Resolved

These resolutions were incorporated as PM-UX-D16 through PM-UX-D25 in `docs/SPEC-PM-004-tui-ux/spec.md`.

1. **Action keys** (PM-UX-D22): function keys (default map: F5 Promote, F6 Hold, F7 Complete, F8 Run Research, F9 Run Review, F10 Cancel)
2. **Tree default expand** (PM-UX-D16): all nodes collapsed on first load
3. **Filter/tree interaction** (PM-UX-D18): hide non-matching nodes; keep ancestors of matches visible
4. **Summary bar layout** (PM-UX-D17): two-line (chips + compact active-run meter)
5. **Detail scrolling/layout** (PM-UX-D20): fixed header + pinned run config; middle region scrolls
6. **Bot kind selection** (PM-UX-D21): two explicit launch actions (Run Research / Run Review), not a kind radio toggle
7. **Onboarding wizard flow** (PM-UX-D24): guided 2–3 steps before maieutic intake
8. **Large lists** (PM-UX-D19): always-virtualize list rows
9. **Accessibility** (PM-UX-D25): high-contrast mode toggle (theme-level); no color-only meaning
10. **Interactive entrypoint** (PM-UX-D23): `/pm open` opens interactive PM views; `/pm bot ...` remains text-mode output

### Next READY Work Package

**WP1: List View Tree Widget with Adaptive Columns**

**Scope**: Implement the PM list view tree widget that reads work items from the PM service (or cache/capsule in degraded mode) and renders a hierarchical tree with adaptive columns.

**Prerequisites**: PM service running with `service.status` and a `work_item.list` RPC method (or stub returning test data).

**Files to create/modify**:

| File                                                  | Action | Description                                                          |
| ----------------------------------------------------- | ------ | -------------------------------------------------------------------- |
| `codex-rs/tui/src/chatwidget/spec_kit/commands/pm.rs` | Modify | Add `open` subcommand that opens the PM tree view                    |
| `codex-rs/tui/src/pm_view/mod.rs`                     | Create | PM view module root: tree state, render, input handling              |
| `codex-rs/tui/src/pm_view/tree.rs`                    | Create | Tree data structure with expand/collapse, parent/child, indent level |
| `codex-rs/tui/src/pm_view/columns.rs`                 | Create | Adaptive column layout: measure width, select column set, format     |
| `codex-rs/tui/src/pm_view/summary_bar.rs`             | Create | Summary bar: state counts + active run count + degraded indicator    |
| `codex-rs/tui/src/pm_view/ipc.rs`                     | Create | IPC client for `work_item.list` RPC                                  |

**Acceptance criteria**:

1. `/pm open` opens a full-screen tree view showing all 4 hierarchy levels with indentation
2. Arrow keys navigate up/down; Left/Right collapse/expand tree nodes
3. Esc returns to main TUI
4. Columns adapt: at >= 120 chars show all 5 columns; at < 120 drop Latest Run; at < 80 drop Last Updated
5. Summary bar visible at top showing item count per state (two-line layout)
6. When service is unavailable, shows "PM service unavailable -- read-only" banner
7. When no data source available, shows error state with diagnostics (PM-UX-D15)
8. `cargo clippy -p codex-tui --all-targets -- -D warnings` passes
9. `cargo test -p codex-tui` passes

**Not in scope for WP1**: Detail view, state transitions, run launch, slash commands beyond `/pm open`, onboarding wizard.

***

*Extracted from Claude Code session `f3e54c6e-819b-4173-9ce5-588944d1e4f4` on 2026-02-12.*
