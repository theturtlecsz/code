# SPEC-PM-003: Bot System (Runner/Service/Tooling)

## Status: PLANNED (design draft)

## Overview

Define the internal **bot system** that executes optional, manual automation for PM holding states:

- `NeedsResearch` (research bots)
- `NeedsReview` (validator/reviewer bots)

`SPEC-PM-002` is the **interface contract** (how callers interact with bot runs across CLI/TUI/headless).  
`SPEC-PM-003` is the **system design** (how the bot runner/service/tooling actually works).

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
- **Prefer single-binary, no-daemon posture** (D38): avoid always-on daemons; background services must be optional and short-lived.
- **Explainability follows capture mode** (D131) + **over-capture hard-block** (D119): the system must never persist more than policy allows; `capture=none` persists no explainability artifacts.
- **No silent destructive actions in headless** (`SPEC-PM-001` NFR3): write operations require explicit user intent and must be auditable.
- **Single-writer capsule** (D7): capsule writes are serialized; runner must not violate lock/queue invariants.

## Three-Surface Truth Model (ADR-003)

The bot system may consult multiple “truth surfaces”, but only one is authoritative:

- **Capsule (`mv2://`)**: **System of Record (SoR)** for:
  - PM work item state and lifecycle,
  - bot run artifacts + logs,
  - replay/audit inputs (evidence packs, web research bundles).
- **OverlayDb (Stage0)**: ephemeral operational cache (policy/vision/session data). Helpful for performance, but **not** an SoR.
- **Local-memory (`codex-product`)**: consultative knowledge graph. If used as an input, the bot must **snapshot** the used records into the capsule (e.g., `ProductKnowledgeEvidencePack`) so runs remain replayable even if local-memory changes.

External tools and filesystem views (e.g. `docs/` tree, Linear) are **projections** derived from capsule state and artifacts. They must not become competing sources of truth.

## System Responsibilities (v1)

- Accept bot run requests for a given work item + kind (`research | review`).
- Build a deterministic context bundle from capsule + repo state.
- Execute the appropriate engine:
  - Research: NotebookLM synthesis (required) + web research
  - Review: validator/reviewer pass + optional patch staging
- Persist immutable artifacts (capsule SoR) + emit events.
- Provide progress + final summaries to Tier‑1 surfaces (TUI + CLI + headless JSON).

## Architecture Options Considered (v1)

We want strong fault isolation and Tier‑1 parity without violating the “no daemon” preference (D38). Options:

- **In-process runner library**:
  - Pros: low overhead, easy local invocation.
  - Cons: poor fault isolation (runner panic can crash TUI); parity drift risk if UI-specific assumptions leak in.
- **On-demand runner service (TUI-spawned + IPC)**:
  - Pros: good isolation + queueing + cancellation; single execution endpoint for TUI/CLI.
  - Cons: introduces IPC + lifecycle complexity; must remain optional/short-lived per D38 to avoid a permanent daemon posture.
- **Ephemeral CLI runner (recommended baseline)**:
  - Pros: Tier‑1 parity by construction (TUI spawns the same CLI command); clean-slate determinism; easy debugging; no long-lived daemon.
  - Cons: process spawn overhead (negligible for minute-scale runs).
- **Always-on daemon**:
  - Generally discouraged (D38 posture + larger attack surface + stale state risk).

## Runtime Model (v1)

### Placement + Lifecycle

Baseline runtime: **ephemeral CLI runner** (D38-friendly).

- The bot system executes as a **single command** (`code speckit pm bot run ...`) that:
  - Loads context from the capsule + repo,
  - Executes the bot kind (`research | review`),
  - Persists artifacts to the capsule (SoR) + best-effort projections,
  - Exits deterministically with product exit codes (see `SPEC-PM-002`).
- TUI triggers bot runs by spawning the same CLI command as a child process and consuming its structured output stream (Tier‑1 parity by construction).
- CLI/headless uses the same command directly; no “TUI-only magic”.

Optional runtime (future): **on-demand runner service** (tertiary, not required).

- Allowed only if it remains **optional** and **short-lived**:
  - Start on demand, exit when idle, and never become a required always-on daemon.
  - Must not introduce semantic divergence vs the ephemeral CLI runner.
  - May provide convenience features (queueing, cancellation, progress fan-out).

### CLI/Headless Integration

- CLI/headless must achieve Tier‑1 parity with the TUI runner semantics.
- Baseline: CLI/headless invokes the same ephemeral runner as the TUI (`code speckit pm bot run ...`).
- If an optional service exists, CLI/headless may connect via IPC (exact transport TBD), but the service must preserve identical semantics and artifacts.

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

For remote triggers (e.g., Linear) and for durable audit, a bot run may be initiated by persisting a request record into the capsule before execution.

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
- `cancelled`

Mapping to headless exit codes is defined in `SPEC-PM-002` (e.g., `0` success, `1` failure, `2` blocked, `130` cancelled).

## Locking + Concurrency (v1)

The capsule is a single-writer system (D7). The bot system must ensure it never creates split-brain writes or corrupts projections.

Recommended enforcement:

- Per-item constraint: at most one active run per `(work_item_id, kind)`.
- Lock target: `work_item_id` (recommended) with a best-effort stale-lock recovery mechanism.
- Preemption: interactive requests (TUI user waiting) may cancel/lower priority background runs (best-effort) to avoid UX deadlocks.

## Components (conceptual)

- **Ephemeral Runner**: executes a single run (`code speckit pm bot run ...`) and exits.
- **Optional BotRunnerService**: run request API + job scheduling + progress reporting (must remain optional/short-lived).
- **Context Builder**: assembles deterministic inputs (work item fields + linked capsule artifacts + repo snapshot metadata).
- **Research Engine**:
  - NotebookLM client (required for `NeedsResearch`)
  - Web research client(s): Tavily MCP preferred; generic web tooling fallback allowed
- **Review Engine**:
  - Local validation execution (allowlisted commands)
  - Optional patch staging via worktree/branch
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
- **NotebookLM hard requirement**:
  - If unavailable/unconfigured, the run terminates as `blocked` with structured output (no fallback research).
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

## Persistence: Events, Artifacts, Projections (v1)

- Artifacts are authoritative SoR (D114); projections are rebuildable mirrors.
- All persistence must be capture-mode compliant (D131/D119).
- `prompts_only` should prefer export-safe templates + hashes (see `WebResearchBundle` contract in `SPEC-PM-001`).
- `full_io` may store extracted content, but must remain excluded from safe export per policy.

## Product Knowledge Integration (ADR-003, v1)

`NeedsResearch` runs should be able to consult `codex-product` to reduce redundant Tier‑2 work *without* breaking determinism:

- **Pre-check**: query local-memory for relevant prior decisions/patterns (best-effort; never a hard dependency).
- **Snapshot for replay**: if local-memory results are used, write a capsule artifact (e.g. `ProductKnowledgeEvidencePack`) capturing the exact records/URIs used, and feed NotebookLM from that snapshot (not from live local-memory).
- **Post-run give-back (optional)**: propose durable, high-confidence memories for curation into `codex-product` after the user accepts the report (never implicit auto-write as part of the bot run).

## Optional Remote UI Bridge: Linear (projection + trigger, not SoR)

Linear can be used as a **remote UI** for PM work items, but it must remain a **projection/trigger surface**:

- **Capsule is always the system-of-record** for work item state and bot artifacts.
- Linear webhooks may **request** a bot run (and/or request a state change), but the bridge must first create a capsule-backed request/event so the action is auditable and replayable.
- Linear should be updated only as a **projection** from capsule state + artifacts (e.g., comments summarizing `ResearchReport` / `ReviewReport` plus `mv2://...` URIs).

### Recommended trigger pattern (v1)

Use an explicit, ephemeral UI signal (label or custom field), rather than implicitly running on any status change:

1. User applies a label like `Bot: Research Requested` (or sets a `bot_request=research` field).
2. Linear webhook hits a bridge endpoint; the bridge validates signature and dedupes retries.
3. Bridge resolves Linear issue → `work_item_id` (via a required `CapsuleID` custom field or a canonical `mv2://...` link in the description).
4. Bridge writes a capsule-backed `BotRunRequest` (or event) and either:
   - spawns the ephemeral CLI runner, or
   - submits the request to the optional runner service.
5. Runner produces capsule artifacts (`BotRunLog`, `WebResearchBundle`, `ResearchReport`, etc).
6. Bridge posts a projection comment back to Linear and clears the trigger label/field (ack).

This preserves the `SPEC-PM-002` contract that holding states are manual and bot runs are explicit.

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

- What is the concrete IPC transport and auth boundary for CLI/headless → runner service (Unix socket vs stdio bridge vs in-process only)?
- Where should a persistent run queue live (in-memory only vs capsule-backed queue events vs filesystem queue)?
- Should `BLOCKED` become a dedicated headless exit code, or remain “exit 2 with structured blocked_reason” (per `SPEC-PM-002`)?
- Which commands are on the initial allowlist for `NeedsReview`, and how do we represent policy overrides without violating D119/D125?

## Implementation Sequencing (proposed)

- **Phase 1 (walking skeleton)**: `pm bot run` stubs + artifact schemas + capsule writes + deterministic exit codes; no NotebookLM required.
- **Phase 2 (parity + brains)**: wire NotebookLM + web research + evidence-pack snapshotting + permission enforcement; TUI spawns runner and streams progress.
- **Phase 3 (integrations + hardening)**: Linear bridge shim + patch staging + game-day drills (offline, auth down, lock contention, partial failures).

## Risks (initial)

- NotebookLM availability: `NeedsResearch` becomes a hard BLOCKED path; requires clear remediation output and fast-fail timeouts.
- Capture policy regressions: any over-capture violates export safety; enforce at artifact-writer boundaries and via tests.
- Lock contention: interactive UX must not deadlock behind background jobs; prioritize TUI-triggered runs.

## References

- Interface contract: `docs/SPEC-PM-002-bot-runner/spec.md`
- PM system PRD: `docs/SPEC-PM-001-project-management/PRD.md`
- External research digest (informational): `docs/SPEC-PM-003-bot-system/research-digest.md`
