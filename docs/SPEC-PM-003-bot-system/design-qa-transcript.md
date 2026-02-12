# PM Bot System Design Q\&A Transcript

**Session**: `bc4beabf-f2c6-4edf-bbe9-9e0a602bdd77`
**Date**: 2026-02-09
**Participants**: System Architect (Claude) + Product Owner (thetu)
**Scope**: SPEC-PM-001, SPEC-PM-002, SPEC-PM-003

***

## Table of Contents

* [Phase 1: Product/Design Q\&A](#phase-1-productdesign-qa)
  * [Phase 0 -- Shared Vision](#phase-0----shared-vision)
  * [PM-001: Work Items](#pm-001-work-items)
    * [PM-001 Follow-ups](#pm-001-follow-ups)
  * [PM-002: Bot Runner Promise](#pm-002-bot-runner-promise)
    * [PM-002 Duration Follow-ups](#pm-002-duration-follow-ups)
  * [PM-003: System Design Intent](#pm-003-system-design-intent)
  * [Cross-Spec Boundaries](#cross-spec-boundaries)
  * [End-of-Session Deliverable](#end-of-session-deliverable)
    * [SPEC-PM-001: Capsule-Backed Project Management](#spec-pm-001-capsule-backed-project-management)
    * [SPEC-PM-002: Bot Runner Interface Contract](#spec-pm-002-bot-runner-interface-contract)
    * [SPEC-PM-003: Bot System](#spec-pm-003-bot-system)
    * [Cross-Spec Alignment](#cross-spec-alignment)
  * [Handoff Items](#handoff-items)
  * [Final Locks](#final-locks)
* [Phase 2: Architecture/Implementation Q\&A](#phase-2-architectureimplementation-qa)
  * [Grounding Recap](#grounding-recap)
  * [Group A: Lifecycle Model](#group-a-lifecycle-model)
  * [Group B: Service Topology + Systemd](#group-b-service-topology--systemd)
  * [Group C: IPC + Protocol](#group-c-ipc--protocol)
  * [Group D: Job Model + Persistence](#group-d-job-model--persistence)
  * [Group E: Artifacts + Capture](#group-e-artifacts--capture)
  * [Group F: Review Write-Mode Finalization](#group-f-review-write-mode-finalization)
  * [Group G: CLI/TUI/Headless Parity](#group-g-clituiheadless-parity)
  * [Group H: Codebase Integration](#group-h-codebase-integration)
  * [Group I: Testing + Rollout](#group-i-testing--rollout)
  * [Synthesized Decisions](#synthesized-decisions)
    * [Phase 2 Additions (architecture-level)](#phase-2-additions-architecture-level)

## Phase 1: Product/Design Q\&A

Role: System Architect + Product/Design Consultant

### Phase 0 -- Shared Vision

**Architect Q1-Q4**: What is the #1 user outcome? Who is the primary user? What's the week-1 proof? What are the anti-goals?

**User**:

1. Manage all specs in a project through the TUI, assign statuses, trigger automation for review/research.
2. Solo user (answer A).
3. CRUD on work items through the TUI -- create (including maieutic intake), read, update, delete.
4. Unsure at this point.

**Locked**:

* TUI is the single control surface for managing all specs.
* Primary user: solo developer managing own roadmap.
* Week-1 proof: full CRUD on work items in TUI.
* Anti-goals: deferred to end of session.

***

### PM-001: Work Items

**Architect Q1**: What is a "work item"? Is the grain "one SPEC-ID = one work item" or finer/coarser?

**Architect Q2**: Of the 8 lifecycle states (Backlog, NeedsResearch, Planned, InProgress, NeedsReview, Completed, Deprecated, Archived), which 2-3 do you use most often?

**Architect Q3**: What must be true about a work item for you to trust it?

**Architect Q4**: What should you be able to answer in 30 seconds from the TUI?

**Architect Q5**: When a work item is created via maieutic intake, what's the minimum output that makes it feel "real"?

**User**:

* Hierarchy already defined: Projects > Features > SPECs > Tasks.
* State should always match SPEC.md and capsule (three-way consistency).
* 30-second answers: what's in progress, what's blocked, what needs attention, last bot run result -- all of these.
* Creation = real when: capsule record + PRD.md + SPEC.md row + docs/ directory. All required.

**Architect (restatement)**: Creation is an atomic multi-artifact operation. If any one output is missing, the item doesn't feel "created."

#### PM-001 Follow-ups

**Architect Q1**: Do all four levels share the same 8 lifecycle states?

**Architect Q2**: Which lifecycle transitions are hot paths (weekly) vs. rare?

**Architect Q3**: SPEC.md becoming a projection means you'd stop editing it directly. Ready for that?

**User**:

* Projects: container only (1:1 with repo), no lifecycle states.
* Features + SPECs: full lifecycle (all 8 states).
* Tasks: ternary -- open / completed / failed.
* SPEC.md: will be a deterministic projection going forward. Locked.
* Lifecycle: thinking out loud, hasn't used this lifecycle before. Wants pattern suggestions.

**Architect recommendation on lifecycle** (drawing from Linear, Shortcut, GitHub Projects):

The 8-state machine is reasonable, but NeedsResearch and NeedsReview are **action triggers, not natural stages**. In tools like Linear:

* **Core flow**: Backlog -> Planned -> InProgress -> Completed
* **Actions**: "run research", "request review"
* **Terminal**: Deprecated, Archived

The question: should NeedsResearch/NeedsReview be **states you move into** or **actions you invoke from certain states**? Marked as Proposed decision for end of session.

***

### PM-002: Bot Runner Promise

**Architect Q1**: When you trigger a research bot, what does "a good result" look like?

**Architect Q2**: When you trigger a review bot, what does "a good result" look like?

**Architect Q3**: NotebookLM is a hard requirement for research. What should happen when it's down?

**Architect Q4**: How long is an acceptable bot run?

**User**:

1. Structured report with options/tradeoffs PLUS confidence levels and citations.
2. Combination of risk + security analysis + patch suggestions.
3. NLM is important but should NOT block research. Used for understanding the project before research. If unavailable, manual local processes should be used.
4. Open to 24-48 hours or more. Wants something comprehensive.

**Architect (significant departures from current specs)**:

* NotebookLM is NOT a hard blocker (PRD change needed). Current spec says hard-requires NLM -> BLOCKED. User wants: preferred, with local fallback.
* Run duration 24-48+ hours: changes everything. Crash recovery, streaming progress, background execution, status polling go from "nice to have" to "load-bearing."

#### PM-002 Duration Follow-ups

**Architect Q1**: If a run takes 24-48 hours, what happens when your machine sleeps/reboots/TUI closes?

**Architect Q2**: During a multi-hour run, what do you want to see in the TUI?

**Architect Q3**: Should a research run be interruptible + resumable, or cancel-and-restart only?

**Architect Q4**: Does 24-48 hours apply to review runs too?

**User**:

1. (c) Run must survive reboots -- implies persistent state + resume.
2. (c) Periodic checkpoint summaries, shown only via status command or spec detail view in TUI. Not a live tail.
3. Want the option for both interrupt-and-resume AND cancel-and-restart.
4. Reviews would be shorter, but wants configurable intensity for each, plus options for what to perform during the run.

**Architect (three new requirements not in any spec)**:

* **Reboot-survivable runs**: biggest tension with D38 (no-daemon). Proposed: amend D38 interpretation.
* **Status on demand, not streaming**: checkpoint summaries visible when you ask.
* **Configurable intensity + scope**: run profiles or configuration flags.

***

### PM-003: System Design Intent

**Architect Q1**: How do you expect to resume a run after reboot?

**Architect Q2**: What's your stance on the "optional on-demand runner service" now?

**Architect Q3**: For configurable intensity, what's the right UX?

**Architect Q4**: When a review bot suggests patches, what level of isolation?

**Architect Q5**: What external integrations matter for v1?

**User**:

1. A systemd unit should pick up the job and continue. No user interaction required.
2. A service that the TUI interacts with makes more sense now.
3. (a) and (b) -- named presets plus granular flags. Checkboxes in the TUI.
4. Definitely requires rebasing and understanding what has changed.
5. No Linear -- we're building a local Linear. v2 of speckit TUI will have a web UI.

**Architect (architecture shifted fundamentally)**:

* **Systemd auto-resume**: hard product requirement, not optional.
* **Service is now baseline, not optional**: TUI talks to persistent service. Ephemeral CLI becomes fallback/debug mode.
* **D38 needs amendment**: reinterpret from "no daemon" to "lightweight persistent service for job management only."
* **Presets + granular checkboxes in TUI** for intensity/scope.
* **Worktree must rebase during long runs**.
* **No Linear in scope**: deferred to v2 web UI.

***

### Cross-Spec Boundaries

**Architect Q1**: What belongs ONLY in PM-001 vs PM-002 vs PM-003?

Proposed split:

* **PM-001**: Work item schema, lifecycle states, SPEC.md projection, maieutic PRD intake, quality scoring, status surfaces.
* **PM-002**: Caller promise -- commands, inputs, outputs, exit codes, artifact schemas, JSON contract.
* **PM-003**: Service, systemd integration, checkpointing, resume, locking, worktree management, permission enforcement, rebase logic.

**Architect Q2**: Canonical definitions to lock?

**Architect Q3**: Does the TUI need to function when the service is down?

**Architect Q4**: Maieutic intake (PM-001) vs research bot (PM-002) -- what's the boundary?

**Architect Q5**: Sequencing -- what must be decided first?

**User**:

1. Split looks good. Questioning whether TUI interaction design needs its own PRD or can be rolled into PM-001.
2. Definitions sound good as proposed.
3. Should degrade gracefully with service management commands: start, stop, doctor.
4. Unsure on boundary. PM-001 could be calling create/update feature or create/update spec, and THIS starts a maieutic intake. The bot creates data separately.
5. Sequencing confirmed: PM-001 -> PM-002 -> PM-003.

**Architect recommendation**: Roll UX into PM-001 as a dedicated section. If it grows too large, extract later.

***

### End-of-Session Deliverable

#### SPEC-PM-001: Capsule-Backed Project Management

**Intention**: PM-001 defines the work item model (Projects > Features > SPECs > Tasks), lifecycle states, and canonical surfaces for managing the roadmap through the TUI. SPEC.md becomes a deterministic projection. CRUD operations (including maieutic intake on create/update) are the primary user actions.

**Decisions to lock**:

| ID    | Decision                                                                                   | Status   |
| ----- | ------------------------------------------------------------------------------------------ | -------- |
| PM-D1 | Work item hierarchy is Projects > Features > SPECs > Tasks                                 | Lock now |
| PM-D2 | Projects have no lifecycle state (container only)                                          | Lock now |
| PM-D3 | Features + SPECs share the full lifecycle state set                                        | Lock now |
| PM-D4 | Tasks have ternary state: open / completed / failed                                        | Lock now |
| PM-D5 | SPEC.md is a deterministic projection from capsule (never hand-edited)                     | Lock now |
| PM-D6 | Creation is atomic multi-artifact: capsule record + PRD.md + SPEC.md row + docs/ directory | Lock now |
| PM-D7 | Maieutic intake is triggered by create/update CRUD operations, not a separate flow         | Lock now |
| PM-D8 | UX/TUI interaction design is owned by PM-001 (dedicated section, not separate PRD)         | Proposed |

**Open Questions**:

1. States vs. actions for NeedsResearch/NeedsReview?
2. Exact fields per hierarchy level?
3. Scoring rubric weights (20/20/20/15/10/15)?
4. Deprecation/archival mechanics -- v1 or deferred?

#### SPEC-PM-002: Bot Runner Interface Contract

**Intention**: The caller promise. Research produces structured options with confidence levels and verifiable citations. Review produces risk/security analysis with patch suggestions. Runs are long-lived (hours to days), configurable, and report progress via checkpoint summaries on demand.

**Decisions to lock**:

| ID     | Decision                                                                             | Status                |
| ------ | ------------------------------------------------------------------------------------ | --------------------- |
| PM-D9  | Research output: structured options + confidence levels + citations                  | Lock now              |
| PM-D10 | Review output: risk + security analysis + patch suggestions                          | Lock now              |
| PM-D11 | NotebookLM is preferred but NOT a hard blocker; local fallback with "degraded" label | Lock now (PRD change) |
| PM-D12 | Runs are long-lived (hours to days); checkpoint summaries on demand                  | Lock now              |
| PM-D13 | Both cancel-and-restart and interrupt-and-resume must be supported                   | Lock now              |
| PM-D14 | Run intensity configurable via named presets + granular include/exclude flags        | Lock now              |
| PM-D15 | Status queried on demand, not live-streamed                                          | Lock now              |

**Open Questions**:

1. Exit code for BLOCKED?
2. Named intensity presets?
3. Granular scope options?
4. Filesystem projection root?

#### SPEC-PM-003: Bot System

**Intention**: Persistent but lightweight service managing long-running bot jobs, surviving reboots via systemd, providing TUI with status and control. Enforces capability boundaries, checkpoints to capsule, handles rebase, degrades gracefully when down.

**Decisions to lock**:

| ID     | Decision                                                                        | Status                |
| ------ | ------------------------------------------------------------------------------- | --------------------- |
| PM-D16 | Primary runtime is a persistent service (not ephemeral CLI); managed by systemd | Lock now (amends D38) |
| PM-D17 | D38 amendment: lightweight persistent service acceptable for job management     | Lock now              |
| PM-D18 | Runs must survive machine reboots via systemd auto-resume                       | Lock now              |
| PM-D19 | TUI degrades to read-only (capsule direct) when service is down                 | Lock now              |
| PM-D20 | Service management commands: start, stop, doctor                                | Lock now              |
| PM-D21 | Review worktrees must rebase against main during long runs                      | Lock now              |
| PM-D22 | Linear integration out of scope (deferred to v2 web UI)                         | Lock now              |
| PM-D23 | No Linear; the system IS the local project management tool                      | Lock now              |

**Open Questions**:

1. IPC transport?
2. Checkpoint frequency?
3. Rebase strategy (continuous vs. periodic)?
4. Conflict resolution during review?
5. Systemd unit: user-level or system-level? Socket activation or always-running?
6. Initial allowlisted commands?

#### Cross-Spec Alignment

**Canonical definitions**:

| Term       | Definition                                                                                        |
| ---------- | ------------------------------------------------------------------------------------------------- |
| Work Item  | Any entity in Projects > Features > SPECs > Tasks hierarchy with a lifecycle state                |
| Bot Run    | Single execution of a research or review pass against one work item, identified by unique run\_id |
| Projection | Any filesystem artifact derived from capsule state, overwritten on sync, not authoritative        |
| Service    | Lightweight persistent process managing bot run lifecycle, checkpointing, and status              |

**Sequencing**: PM-001 -> PM-002 -> PM-003.

**MVP demo narrative**: Create a Feature in TUI -> capsule record + SPEC.md row + PRD.md generated via maieutic intake -> set to NeedsResearch -> trigger research run with "standard" intensity -> service checkpoints periodically -> produces ResearchReport with options + confidence + citations -> review in TUI -> promote to Planned -> SPEC.md updates automatically -> trigger review with security enabled -> bot creates worktree, rebases, produces ReviewReport with patches -> apply patch -> mark Completed.

***

### Handoff Items

Five additional items for the architect:

1. **Anti-goals (still open)**: candidates: "not a CI/CD pipeline orchestrator", "not multi-user collaboration (v1 is solo)", "not something that auto-merges without approval."
2. **D38 amendment is policy-level**: needs formal D135 with rationale.
3. **Three unresolved design tensions**: states vs. actions, rebase during long runs, checkpoint granularity.
4. **Existing building blocks**: PM-003 spec lines 268-286 list what already exists in codex-rs.
5. **Work item capsule schema v0**: architect's first deliverable should be a draft schema.

***

### Final Locks

**User confirmed D135 lock**: docs/DECISIONS.md:240. ADR-004 marked Accepted. D38/D126 carve-out explicitly scoped by D135.

**User locked `pm bot run` semantics**: Default submit-and-exit (exit 0 = submitted, returns run\_id + initial status JSON). `--wait` is opt-in, exits by terminal state.

**User locked review freshness strategy**: Rebase only at finalization. Conflicts yield terminal `needs_attention` with original patch + conflict details + manual resolution instructions.

**Architect on remaining open items**:

1. States vs. actions for NeedsResearch/NeedsReview (highest priority -- affects capsule schema, state machine, TUI, PM-002 commands).
2. IPC transport: recommends Unix socket (locked as D136).
3. Work item capsule schema v0: architect's first deliverable.

***

## Phase 2: Architecture/Implementation Q\&A

Role: System Architect + Principal Rust Reviewer

### Grounding Recap

12 locked invariants confirmed:

* D7: single-writer capsule
* D113/D133: Tier-1 parity, headless never prompts
* D114: capsule is SoR, artifacts authoritative
* D119/D131: capture enforcement, over-capture hard-block
* D130: maieutic gate mandatory pre-execution
* D135: service-first runtime (systemd-managed, auto-resume, exit-when-idle)
* D136: Unix socket IPC, prefer systemd socket activation
* `pm bot run`: async submit-and-exit default, `--wait` opt-in
* Review freshness: rebase at finalization only
* Existing building blocks: `core::pm::bot`, `app-server-protocol`, `memvid_adapter`, `AgentManager`

***

### Group A: Lifecycle Model

**A1: NeedsResearch/NeedsReview -- states or actions?**

Options presented:

* **States**: item moves into them. Simpler schema, clear in status views. Item "leaves" InProgress.
* **Actions**: item stays in current state. Runs are annotations. More flexible, needs separate `active_runs` field.
* **Hybrid: states + re-enter**: enter the holding state, auto-return to previous state on run completion.

**Decision: Hybrid**. Schema needs a `return_state` field stored when entering a holding state. Bot service auto-transitions back on completion. This is the one exception to "bots recommend, PM acts" -- the return is automatic since the PM already expressed intent.

**A2: Work item schema -- immutable fields?**

Options: minimal immutable, strict immutable, wide immutable.

**Decision: Minimal immutable set**. Immutable: id, type, created\_at, parent\_id. Everything else mutable. Shared fields: id, type, title, description, created\_at, updated\_at, parent\_id, state. Level-specific: Features get acceptance\_criteria + priority; SPECs get prd\_uri + quality\_score; Tasks get assignee + result.

**A3: Projection failure recovery?**

Options: capsule-is-truth + re-project, rollback capsule, leave partial + surface error.

**Decision: Capsule is truth, re-project**. Consistent with D114 (artifacts authoritative, projections rebuildable).

***

### Group B: Service Topology + Systemd

**B1: Per-user vs. per-workspace service?**

Options: per-workspace, per-user, per-user with workspace namespacing.

**Decision: Per-user**. One service manages all workspaces. IPC protocol must include `workspace_path` in every request for routing. Single user-scoped socket.

**B2: Resume trigger after reboot?**

Options: socket activation + stale check, WantedBy=default.target, systemd timer.

**Decision: WantedBy=default.target**. Service starts automatically on login. Checks for incomplete runs immediately.

**B3: Idle-exit definition?**

Options: no active runs for 30min, exit immediately when last run finishes, never exit, configurable.

**Decision: Never exit (stay running)**. Since it starts on login, keep running until logout/shutdown. Simplest, minimal resource cost. Note: D135 "exit-when-idle" reinterpreted as "no always-*processing* daemon."

**B4: Service crate location?**

Options: new `codex-pm-service` crate, subcommand of `code` CLI, module in core crate.

**Decision: New crate `codex-rs/pm-service/`**. Own Cargo.toml, produces `codex-pm-service` binary. Systemd unit runs this binary directly.

***

### Group C: IPC + Protocol

**C1: Socket path convention?**

Options: `$XDG_RUNTIME_DIR/codex-pm.sock`, `~/.codex/pm-service.sock`, systemd-managed ListenStream.

**Decision: `$XDG_RUNTIME_DIR/codex-pm.sock`**. Standard XDG path, user-scoped via filesystem permissions.

**C2: IPC protocol?**

Options: JSON-RPC-lite (reuse existing), NDJSON, gRPC/protobuf.

**Decision: JSON-RPC-lite**. Reuse existing `app-server-protocol/jsonrpc_lite.rs`. Methods like `bot.run`, `bot.status`, `bot.cancel`.

**C3: Protocol versioning?**

Options: handshake hello, per-message schema\_version, both.

**Decision: Handshake hello**. On connect, client sends `hello` with protocol\_version + client\_version. Service responds with version + capabilities. Incompatible versions rejected early.

**C4: Auth boundary?**

Options: filesystem permissions only, filesystem + workspace token, filesystem + write-op confirmation.

**Decision: Filesystem permissions only**. Unix socket with 0700 permissions. Sufficient for local single-user tool.

***

### Group D: Job Model + Persistence

**D1: Run queue location?**

Options: capsule-backed, service-local SQLite, filesystem queue.

**Decision: Capsule-backed**. Each BotRunRequest persisted as capsule artifact before execution. Run state transitions are capsule events. Consistent with D114.

**D2: Run ID format?**

Options: UUID (keep current), human-readable with UUID suffix, sequential per work item.

**Decision: UUID (keep current)**. Already implemented, globally unique, no coordination needed.

**D3: Cancel -- persist partial artifacts?**

Options: BotRunLog + partial artifacts, BotRunLog only, BotRunLog + checkpoint summary only.

**Decision: BotRunLog + partial artifacts**. Persist everything with `partial: true` flag. Half-finished research with 2 of 5 options is still valuable.

**D4: Duplicate request handling?**

Options: reject duplicate + allow cross-kind, queue behind active, cancel active + start new, reject all duplicates.

**Decision: Reject duplicate, allow cross-kind**. Second research request while one running -> BLOCKED. But research + review can run simultaneously (research is read-only, review uses own worktree).

***

### Group E: Artifacts + Capture

**E1: Minimum artifact set per terminal status?**

Options: proposed set (differentiated by terminal status), failed includes partial, all states get everything.

**Decision: Proposed set is correct**.

* succeeded: BotRunLog + report + optional bundles
* failed: BotRunLog with error details only
* blocked: BotRunLog with blocked\_reason only
* cancelled: BotRunLog + partial artifacts
* needs\_attention: BotRunLog + ReviewReport + PatchBundle with conflict details

**E2: Checkpoint frequency?**

Options: hybrid event + 30min floor, event-driven only, time-driven only, configurable.

**Decision: Hybrid event + 30min floor**. Checkpoint after each significant phase/finding AND at least every 30 minutes. Best for crash recovery + status queries.

**E3: Should service accept bot runs with capture=none?**

Options: reject, accept but warn, accept silently.

**Decision: Reject capture=none**. Bot runs require capture >= `prompts_only`. Exit code 10 (needs\_input) with message.

**E4: Projection root?**

Options: `docs/specs/<ID>/runs/` + service writes, `.speckit/pm/runs/`, docs + separate projection-builder.

**Decision: `docs/specs/<ID>/runs/` + service writes**. Visible in repo tree, consistent with docs/ convention.

***

### Group F: Review Write-Mode Finalization

**F1: Rebase target branch?**

Options: configurable default main, always main (v1), infer from work item.

**Decision: Configurable, default main**. `--rebase-target <branch>` flag, optional.

**F2: Conflict artifact schema?**

Proposed: `{ analysis_base_commit, rebase_target_commit, rebase_succeeded: false, conflict_files: [{path, conflict_markers}], original_diff_uri, resolution_instructions }`

Options: sufficient for v1, add suggested\_resolution per file, minimal.

**Decision: Add suggested\_resolution per file**. Review engine performs a conflict resolution pass after rebase failure. Adds complexity but high value.

**F3: `needs_attention` UX?**

Options: distinct badge + detail on select, same as other terminal, auto-open detail view.

**Decision: Distinct badge + detail on select**. Yellow/warning indicator in status column. TUI auto-shows conflict summary + resolution instructions when user selects the item.

***

### Group G: CLI/TUI/Headless Parity

**G1: Complete v1 command set?**

Proposed: `bot run`, `bot status`, `bot runs`, `bot show`, `bot cancel`, `bot resume` + service `start`/`stop`/`status`/`doctor`.

Options: complete as proposed, add `bot logs`, add `bot retry`, add both.

**Decision: Complete as proposed**. Covers full lifecycle.

**G2: JSON schema publication?**

Options: schema\_version in payload only (v1), publish via schema-gen, publish via docs.

**Decision: schema\_version in payload only (v1)**. Publish separately when schema stabilizes.

**G3: `--wait` polling mechanism?**

Options: IPC keepalive with push, polling with reconnect, file watch.

**Decision: IPC keepalive with push**. CLI holds Unix socket connection open. Service pushes JSON-RPC notifications on status changes. CLI exits on terminal status.

***

### Group H: Codebase Integration

**H1: Crate boundary split?**

Proposed: (a) `core::pm` = shared types, (b) new `codex-rs/pm-service/` = service binary, (c) CLI adds `pm` subcommand group, (d) TUI adds PM widget.

Options: proposed split, merge into core, separate shared types crate.

**Decision: Proposed split**. Clean dependency: pm-service depends on core::pm; CLI depends on core::pm; TUI depends on core::pm.

**H2: Reuse AgentManager?**

Options: don't reuse (build BotRunManager), wrap AgentManager, fork AgentManager.

**Decision: Don't reuse AgentManager**. Bot runs have fundamentally different lifecycle (hours/days, checkpoint/resume). Build purpose-built BotRunManager. Reuse event patterns but not the singleton.

**H3: Extract capsule adapter?**

Options: extract to `codex-rs/capsule/`, duplicate minimal API, keep in TUI.

**Decision: Extract to `codex-rs/capsule/`**. Both TUI and pm-service depend on it. One-time refactor.

***

### Group I: Testing + Rollout

**I1: Walking-skeleton acceptance test?**

Proposed: Submit research via CLI -> service accepts -> writes BotRunLog checkpoint -> marks succeeded -> CLI status shows completed -> capsule contains artifact. No actual research engine.

Options: proposed test, add projection test, add cancel test.

**Decision: Proposed test is correct**. Proves full plumbing with stub engines.

**I2: Systemd integration testing?**

Options: mock systemd + test logic, test in systemd namespace, manual only.

**Decision: Mock systemd, test service logic**. Unit-test lifecycle logic without real systemd. Integration test via direct Unix socket.

**I3-I5: Lock contention, reboot resume, capture compliance?**

Options: all three proposed, adjust I4 to SIGTERM first, defer to Phase 2.

**Decision: All three proposed approaches are good**.

* I3: Two concurrent CLI processes, verify one BLOCKED.
* I4: Start run, SIGKILL service, restart, verify resume from checkpoint.
* I5: capture=prompts\_only -> no full\_io artifacts; capture=none -> rejection.

***

### Synthesized Decisions

24 proposed decisions from this session:

| ID     | Decision                                                     | Group |
| ------ | ------------------------------------------------------------ | ----- |
| PM-D1  | Work item hierarchy: Projects > Features > SPECs > Tasks     | A     |
| PM-D2  | Projects have no lifecycle state (container only)            | A     |
| PM-D3  | Features + SPECs share the full lifecycle state set          | A     |
| PM-D4  | Tasks: ternary state (open / completed / failed)             | A     |
| PM-D5  | SPEC.md is deterministic projection from capsule             | A     |
| PM-D6  | Creation is atomic multi-artifact                            | A     |
| PM-D7  | Maieutic intake triggered by CRUD, not separate flow         | A     |
| PM-D8  | UX/TUI interaction design owned by PM-001                    | A     |
| PM-D9  | Research output: structured options + confidence + citations | --    |
| PM-D10 | Review output: risk + security analysis + patches            | --    |
| PM-D11 | NotebookLM preferred, not hard blocker; local fallback       | --    |
| PM-D12 | Runs long-lived (hours/days); checkpoint summaries on demand | --    |
| PM-D13 | Cancel-and-restart AND interrupt-and-resume both supported   | --    |
| PM-D14 | Intensity via named presets + granular flags                 | --    |
| PM-D15 | Status queried on demand, not streamed                       | --    |
| PM-D16 | Primary runtime is persistent service (amends D38)           | B     |
| PM-D17 | D38 reinterpreted: lightweight persistent service OK         | B     |
| PM-D18 | Runs survive reboots via systemd auto-resume                 | B     |
| PM-D19 | TUI degrades to read-only when service down                  | B     |
| PM-D20 | Service management: start, stop, doctor                      | B     |
| PM-D21 | Review worktrees rebase against main                         | F     |
| PM-D22 | Linear integration deferred to v2                            | --    |
| PM-D23 | System IS the local PM tool                                  | --    |
| PM-D24 | `--wait` uses IPC keepalive with push notifications          | G     |

#### Phase 2 Additions (architecture-level)

| Topic               | Decision                                                         | Group |
| ------------------- | ---------------------------------------------------------------- | ----- |
| Lifecycle model     | Hybrid states + auto-return with `return_state` field            | A     |
| Schema              | Minimal immutable set (id, type, created\_at, parent\_id)        | A     |
| Projection recovery | Capsule is truth, re-project on failure                          | A     |
| Service scope       | Per-user (single socket, workspace\_path in requests)            | B     |
| Resume trigger      | WantedBy=default.target (starts on login)                        | B     |
| Idle behavior       | Never exit (stay running; D135 = no always-processing)           | B     |
| Service crate       | New `codex-rs/pm-service/` crate                                 | B     |
| Socket path         | `$XDG_RUNTIME_DIR/codex-pm.sock`                                 | C     |
| IPC protocol        | JSON-RPC-lite (reuse existing)                                   | C     |
| Versioning          | Handshake hello with protocol\_version                           | C     |
| Auth                | Filesystem permissions (0700) only                               | C     |
| Run queue           | Capsule-backed (consistent with D114)                            | D     |
| Run ID              | UUID (keep current)                                              | D     |
| Cancel artifacts    | BotRunLog + partials with `partial: true` flag                   | D     |
| Concurrency         | Reject duplicate same-kind; allow cross-kind                     | D     |
| Artifact sets       | Differentiated by terminal status                                | E     |
| Checkpoints         | Hybrid: event-driven + 30min floor                               | E     |
| capture=none        | Reject for bot runs (require >= prompts\_only)                   | E     |
| Projection root     | `docs/specs/<ID>/runs/` + service writes                         | E     |
| Rebase target       | Configurable, default main                                       | F     |
| Conflict schema     | Include suggested\_resolution per file                           | F     |
| needs\_attention UX | Distinct badge + detail on select                                | F     |
| Command set         | run/status/runs/show/cancel/resume + start/stop/status/doctor    | G     |
| JSON schema         | schema\_version in payload only (v1)                             | G     |
| Crate split         | core::pm types / pm-service binary / CLI subcommand / TUI widget | H     |
| Bot manager         | Purpose-built BotRunManager (not AgentManager)                   | H     |
| Capsule crate       | Extract to codex-rs/capsule/                                     | H     |
| Skeleton test       | CLI -> IPC -> service -> capsule -> status (stub engines)        | I     |
| Systemd testing     | Mock systemd, test service logic                                 | I     |
| Additional tests    | Lock contention, SIGKILL resume, capture compliance              | I     |

***

*Extracted from Claude Code session `bc4beabf-f2c6-4edf-bbe9-9e0a602bdd77` on 2026-02-12. Original session ran 2026-02-09.*
