# SPEC-PM-003: Bot System (Runner/Service/Tooling)

## Status: PLANNED (design draft)

## Overview

Define the internal **bot system** that executes optional, manual automation for PM holding states:

- `NeedsResearch` (research bots)
- `NeedsReview` (validator/reviewer bots)

`SPEC-PM-002` is the **interface contract** (how callers interact with bot runs across CLI/TUI/headless).  
`SPEC-PM-003` is the **system design** (how the bot runner/service/tooling actually works).

PRD: `docs/SPEC-PM-003-bot-system/PRD.md`

## Design / Research Inputs (supporting)

These documents capture durable context from design and research sessions. They are **not** part of the locked decision register unless a point is also locked in `docs/DECISIONS.md`.

- Design Q&A transcript (2026-02-09): `docs/SPEC-PM-003-bot-system/design-qa-transcript.md`
- Product thinking for PM-as-spec-management (SPEC-PM-001): `docs/SPEC-PM-003-bot-system/product-analysis.md`
- External research digest (informational): `docs/SPEC-PM-003-bot-system/research-digest.md`

## Goals

- Define the bot runner/service architecture and lifecycle.
- Define scheduling/queueing, idempotency, and cancellation semantics.
- Define the permission model and enforcement for tool execution + filesystem writes.
- Define worktree/branch management for write-enabled review runs.
- Define capsule event/artifact persistence and filesystem projection implementation.
- Define observability (status events, logs, diagnostics) without violating capture-mode policy.

## Non-Goals (initial)

- Automatically starting bots on every work item by default.
- Automatic PM state transitions (bots recommend; PM acts).
- Cross-platform support (Linux-only remains baseline).
- Auto-commit/push/merge as a default behavior.

## Constraints (Already Locked)

- **Tier‑1 multi-surface parity** (D113/D133): automation-critical behavior must match across CLI/TUI/headless.
- **Headless never prompts** (D133): missing requirements → structured output + product exit codes.
- **Maieutic step always mandatory** (D130): bot automation must not bypass required gates (especially `/speckit.auto`).
- **Operational footprint posture** (D38): prefer single-binary, no-daemon design; any persistent runtime must be tightly scoped and justified.
- **No permanent daemon (maintenance posture)** (D126): maintenance triggers are event/scheduled/on-demand; avoid “always processing” background loops.
- **Bot job management service runtime** (D135): lightweight persistent service is allowed for bot job management; must be systemd-managed, auto-resume on reboot, and exit-when-idle (no heavyweight frameworks or always-processing daemons).
- **Bot service IPC** (D136): IPC between Tier‑1 callers (TUI/CLI/headless) and the service uses a Unix domain socket; prefer systemd socket activation.
- **Explainability follows capture mode** (D131) + **over-capture hard-block** (D119): the system must never persist more than policy allows; `capture=none` persists no explainability artifacts.
- **No silent destructive actions in headless** (`SPEC-PM-001` NFR3): write operations require explicit user intent and must be auditable.
- **Single-writer capsule** (D7): capsule writes are serialized; runner must not violate lock/queue invariants.

## Current Intended Direction (from design transcript)

The 2026-02-09 design session produced additional decisions that are **not locked** in `docs/DECISIONS.md` (unless they match a D-number above). Where the transcript conflicts with locked decisions, the locked decision wins.

**Proposed (implemented in `codex-rs/pm-service/` + `codex-rs/cli/src/pm_cmd.rs`)**

- **Service scope**: per-user service (single socket) that can manage multiple workspaces; every IPC request includes `workspace_path` for routing.
- **Socket path**: `$XDG_RUNTIME_DIR/codex-pm.sock` (systemd `%t/codex-pm.sock`).
- **IPC protocol**: newline-delimited JSON-RPC-lite with an explicit `hello` handshake and protocol versioning.
- **`--wait` behavior**: keepalive connection with push notifications (`bot.terminal`) rather than polling.
- **Duplicate handling**: reject duplicate active runs for the same `(workspace_path, work_item_id, kind)`; allow cross-kind concurrency.
- **Capture posture for bot runs**: reject `capture=none` for `pm bot run` (require `>= prompts_only`).

**Proposed (not yet fully implemented)**

- **Checkpoint cadence**: hybrid event-driven checkpoints with a time floor (e.g., every 30 minutes).
- **Filesystem projection root**: `docs/specs/<WORK_ITEM_ID>/runs/` (service writes best-effort projections; capsule remains SoR).

**Conflict resolved by locked decisions**

- The transcript contains a “never exit” idle posture; this is superseded by **D135 exit-when-idle**.

## Service-First Runtime (D135)

Bot runs may be **long-lived** (hours → days) and must survive TUI restarts, process crashes, and machine reboots.

Baseline runtime (D135; ADR-004): a lightweight **PM bot service** managed by a **systemd user unit** that can resume incomplete runs without user interaction and **exit when idle**. The ephemeral CLI runner remains as fallback/debug, not the primary execution model.

## Three-Surface Truth Model (ADR-003)

The bot system may consult multiple “truth surfaces”, but only one is authoritative:

- **Capsule (`mv2://`)**: **System of Record (SoR)** for:
  - PM work item state and lifecycle,
  - bot run artifacts + logs,
  - replay/audit inputs (evidence packs, web research bundles).
- **OverlayDb (Stage0)**: ephemeral operational cache (policy/vision/session data). Helpful for performance, but **not** an SoR.
- **Local-memory (`codex-product`)**: consultative knowledge graph. If used as an input, the bot must **snapshot** the used records into the capsule (e.g., `ProductKnowledgeEvidencePack`) so runs remain replayable even if local-memory changes.

External tools and filesystem views (e.g. `docs/` tree) are **projections** derived from capsule state and artifacts. They must not become competing sources of truth.

## System Responsibilities (v1)

- Accept bot run requests for a given work item + kind (`research | review`).
- Build a deterministic context bundle from capsule + repo state.
- Execute the appropriate engine:
  - Research: NotebookLM synthesis (preferred; policy-defined) + web research
  - Review: validator/reviewer pass + optional patch staging
- Persist immutable artifacts (capsule SoR) + emit events.
- Provide progress + final summaries to Tier‑1 surfaces (TUI + CLI + headless JSON).

## Architecture Options Considered (v1)

We want strong fault isolation and Tier‑1 parity while preserving the “no permanent daemon” posture. Options:

- **In-process runner library**:
  - Pros: low overhead, easy local invocation.
  - Cons: poor fault isolation (runner panic can crash TUI); parity drift risk if UI-specific assumptions leak in.
- **Service-first runtime (systemd-managed)** (baseline; D135, ADR-004):
  - Pros: supports long-lived runs + reboot survival; isolates runs from TUI lifetime; enables resume/cancel semantics.
  - Cons: introduces IPC + lifecycle management; must strictly enforce D135 scope (exit-when-idle; no always-processing loops; no heavy frameworks).
- **Ephemeral CLI runner** (fallback/debug):
  - Pros: easy debugging; parity by construction; minimal operational footprint.
  - Cons: not sufficient alone for multi-day runs and reboot survival.
- **Always-on daemon**:
  - Generally discouraged (D38 posture + larger attack surface + stale state risk).

## Runtime Model (v1)

### Placement + Lifecycle

Baseline runtime: **PM BotRunnerService** + **systemd user unit** (D135; ADR-004).

- The service owns run lifecycle (start/status/cancel/resume) and performs the work.
- The service persists run artifacts and **checkpoints** into the capsule so incomplete runs can resume after restart/reboot.
- The service must remain “lightweight” and job-scoped: **exit when idle**, no heavy agent framework, and no always-processing loops.

Fallback runtime: **ephemeral CLI runner**.

- Executes a single bot run as a one-shot process (internal engine), producing the same artifacts as the service.
- Used for debugging, emergency “no service” scenarios, and parity testing.

### CLI/Headless Integration

- CLI/headless must achieve Tier‑1 parity with the TUI semantics (D113/D133).
- Baseline: CLI/headless talks to the same service endpoint as the TUI (Unix domain socket; systemd socket-activated per D136).
- Fallback: CLI/headless may execute the ephemeral runner directly, but must preserve identical artifacts and exit codes.

### TUI Degraded Mode

If the service is unavailable, the TUI must degrade gracefully:

- read-only status views from capsule artifacts,
- explicit service management actions (start/stop/doctor),
- no “hanging” interactive flows.

### Idempotency + Concurrency

- Each run is identified by a unique `run_id`.
- Recommended constraints:
  - At most one active run per `(work_item_id, kind)`.
  - Global concurrency bounded (CPU, network, capsule single-writer).
- Cancellation:
  - Best-effort, cooperative cancellation.
  - Partial results must still be persisted with terminal `cancelled` status and a `BotRunLog`.

## Internal Job Model (v1)

This section defines internal structures for queueing/bridging. It does not replace the caller-facing contract in `SPEC-PM-002`.

### BotRunRequest (capsule-backed, internal)

For durable audit (and any future external triggers), a bot run may be initiated by persisting a request record into the capsule before execution.

Reference implementation type: `codex-rs/core/src/pm/bot.rs` (`BotRunRequest`).

Minimal fields (v0):

- `schema_version` (e.g., `bot_run_request@1.0`)
- `run_id`
- `work_item_id`
- `kind`: `research | review`
- `capture_mode`: `none | prompts_only | full_io`
- `write_mode`: `none | worktree` (review only)
- `requested_at` (RFC3339)
- `trigger` (optional): `source`, `dedupe_key`, `url`

### BotRunState (FSM)

The runner must be deterministic in headless mode (D133): never hang or wait for interactive input.

Proposed states:

- `queued`
- `running`
- `succeeded`
- `failed`
- `blocked`
- `needs_attention` (terminal; manual resolution required, e.g., rebase conflicts)
- `cancelled`

Mapping to headless exit codes is defined in `SPEC-PM-002` (noting that `pm bot run` is async by default and `--wait` is the opt-in mode for “exit reflects terminal state”).

## Locking + Concurrency (v1)

The capsule is a single-writer system (D7). The bot system must ensure it never creates split-brain writes or corrupts projections.

Recommended enforcement:

- Per-item constraint: at most one active run per `(work_item_id, kind)`.
- Lock target: `work_item_id` (recommended) with a best-effort stale-lock recovery mechanism.
- Preemption: interactive requests (TUI user waiting) may cancel/lower priority background runs (best-effort) to avoid UX deadlocks.

## Components (conceptual)

- **BotRunnerService** (baseline; D135, ADR-004): job lifecycle + scheduling + progress + resume.
- **systemd user unit** (baseline; D135, ADR-004): ensures incomplete runs can resume after reboot without interactive input.
- **Ephemeral Runner** (fallback): executes a single run (`code speckit pm bot run ...`) and exits.
- **Context Builder**: assembles deterministic inputs (work item fields + linked capsule artifacts + repo snapshot metadata).
- **Research Engine**:
  - NotebookLM client (preferred; policy-defined)
  - Web research client(s): Tavily MCP preferred; generic web tooling fallback allowed
- **Review Engine**:
  - Local validation execution (allowlisted commands)
  - Patch staging via worktree/branch + rebase strategy (for long-lived runs)
- **Permission Enforcer**: centrally enforces read/write/network/tool allowlists and “no destructive actions” posture.
- **Artifact Writer**: writes artifacts + emits capsule events (capture-mode aware).
- **Projection Builder**: filesystem projection implementation (best-effort; rebuildable).
- **UI Bridge**: publishes progress + summaries to TUI surfaces and CLI/headless JSON.

## Tool Execution + Permissions (v1)

### Default Posture

- Default is **read-only** with respect to the user’s primary working tree.
- Write capability must be explicitly enabled via interface flags (see `SPEC-PM-002`) and constrained to bot-owned worktrees.

### NeedsResearch

- Network allowed for web research (Tavily MCP preferred; generic fallback allowed).
- NotebookLM posture is **policy-defined**:
  - If `allow_degraded=true` (current implementation default), proceed in degraded mode when NotebookLM is unavailable, label outputs degraded, and persist replay/audit inputs accordingly.
  - If `allow_degraded=false`, the run terminates as `blocked` with structured output.
- Must not create worktrees/branches or modify repo files.

### NeedsReview

- May run local validation commands under a strict allowlist (policy-defined).
- May create a worktree/branch and stage changes when write mode is enabled.
- Must never commit/push/merge by default.

## Worktree + Branch Management (NeedsReview, v1)

### Goals

- Preserve user safety: never mutate the primary working tree by default.
- Provide a reviewable, inspectable staging area for suggested changes.
- Provide a patch artifact so review is possible without checkout.

### Proposed Defaults

- Worktree root: `./.speckit/worktrees/<work_item_id>/<run_id>/`
- Branch name: `bot/<work_item_id>/<run_id>`

### Required Outputs

When write mode is enabled and changes are staged, the bot system must output:

- `worktree_path`
- `branch_name`
- A `PatchBundle` artifact (includes a diff/patch reference + apply/inspect instructions)

### Long-Run Freshness (rebase intent, v1)

For long-lived review runs, “stale patches” are a UX failure. The bot system should ensure proposed changes remain reviewable against a reasonably current base:

- **Run-start snapshot**: record the base commit used for analysis/checkpoints (`analysis_base_commit`).
- **Deterministic checkpoints**: all checkpoint artifacts must reference `analysis_base_commit` so “resume” preserves deterministic progress semantics.
- **Finalization-only rebase**: immediately before producing the final `PatchBundle`, rebase the bot branch/worktree onto the current target branch (default: `main`) and record:
  - `rebase_target_commit` (the commit SHA the patch was rebased onto), and
  - whether the rebase succeeded cleanly.
- **Conflict posture**: if the finalization rebase conflicts, the run transitions to terminal `needs_attention` and must persist:
  - the original patch (based on `analysis_base_commit`),
  - a structured conflict summary, and
  - manual resolution instructions (headless never prompts).

Checkpoint artifacts remain valid as analysis even if the final patch requires manual conflict resolution.

## Persistence: Events, Artifacts, Projections (v1)

- Artifacts are authoritative SoR (D114); projections are rebuildable mirrors.
- All persistence must be capture-mode compliant (D131/D119).
- `prompts_only` should prefer export-safe templates + hashes (see `WebResearchBundle` contract in `SPEC-PM-001`).
- `full_io` may store extracted content, but must remain excluded from safe export per policy.
- Local disk persistence may be used as a **fast resume cache**, but the capsule remains the source of record for audit/replay (D114).

## Product Knowledge Integration (ADR-003, v1)

`NeedsResearch` runs should be able to consult `codex-product` to reduce redundant Tier‑2 work *without* breaking determinism:

- **Pre-check**: query local-memory for relevant prior decisions/patterns (best-effort; never a hard dependency).
- **Snapshot for replay**: if local-memory results are used, write a capsule artifact (e.g. `ProductKnowledgeEvidencePack`) capturing the exact records/URIs used, and feed NotebookLM from that snapshot (not from live local-memory).
- **Post-run give-back (optional)**: propose durable, high-confidence memories for curation into `codex-product` after the user accepts the report (never implicit auto-write as part of the bot run).

## Out of Scope: Linear / Remote PM UI (v1)

No Linear bridge is in scope for v1. PM-003 is building a **local-first** PM/bot system; any web UI is deferred to a later iteration.

## NotebookLM Reference Notebook (roadmap)

Maintain a dedicated NotebookLM notebook for developing Spec‑Kit itself (e.g., `spec-kit-dev`, currently titled **“Codex-RS Spec-Kit Decision Register 1.0.0”**):

- Include Tier‑1 policy + decision docs (`docs/POLICY.md`, `docs/DECISIONS.md`, ADR‑003, etc.).
- Include PM specs (`SPEC-PM-001`, `SPEC-PM-002`, `SPEC-PM-003`) and related architecture references.
- Treat it as a fast “decision register” surface for product development; refresh sources when these docs change.

## Existing Building Blocks (already in Codex‑RS today)

The bot system should reuse existing primitives rather than inventing new ones:

- Agent runtime supports `read_only`, plus optional `worktree_path` and `branch_name`.
  - `codex-rs/core/src/agent_tool.rs`
- Agent status updates already flow into the main event stream.
  - `codex-rs/core/src/codex.rs`
- Capsule-backed grounding artifacts exist (good deterministic context bundle inputs).
  - `codex-rs/tui/src/chatwidget/spec_kit/grounding.rs`
- Structured Project Intel snapshot model exists.
  - Schema: `codex-rs/stage0/src/project_intel/types.rs`
  - TUI commands: `codex-rs/tui/src/chatwidget/spec_kit/commands/intel.rs`
- NotebookLM integration surfaces exist.
  - `codex-rs/cli/src/architect_cmd.rs`
  - `codex-rs/cli/src/stage0_cmd.rs`
- Local validation gates exist.
  - `.githooks/pre-commit`
  - `scripts/doc_lint.py`

## Open Questions

- Should `BLOCKED` become a dedicated headless exit code, or remain “exit 2 with structured blocked_reason” (per `SPEC-PM-002`)?
- Which commands are on the initial allowlist for `NeedsReview`, and how do we represent policy overrides without violating D119/D125?
- Should “resume after reboot/login” be timer-driven, socket-activation-driven, or an explicit user action that systemd replays?
- What is the exact artifact schema for “finalization rebase conflict” details and manual resolution instructions?

## Implementation Sequencing (proposed)

- **Phase 1 (walking skeleton)**: service skeleton + run request persistence + locking + checkpoint artifacts + deterministic terminal results; ephemeral runner fallback.
- **Phase 2 (parity + engines)**: wire research/review engines + evidence-pack snapshotting + permission enforcement + worktree staging + rebase boundaries.
- **Phase 3 (hardening)**: game-day drills (offline, auth down, lock contention, partial failures, reboot resume).

## Risks (initial)

- Service/runtime drift: the bot service must remain strictly within D135 scope (systemd-managed, exit-when-idle; no always-processing daemon loops or heavy frameworks).
- NotebookLM availability: clarify whether `NeedsResearch` is hard BLOCKED or can operate in a degraded mode.
- Capture policy regressions: any over-capture violates export safety; enforce at artifact-writer boundaries and via tests.
- Lock contention: interactive UX must not deadlock behind background jobs; prioritize TUI-triggered runs.

## References

- Interface contract: `docs/SPEC-PM-002-bot-runner/spec.md`
- PM system PRD: `docs/SPEC-PM-001-project-management/PRD.md`
- External research digest (informational): `docs/SPEC-PM-003-bot-system/research-digest.md`
- Runtime ADR (accepted): `docs/adr/ADR-004-pm-bot-service-runtime.md`
