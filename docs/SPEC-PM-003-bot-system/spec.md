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
- **Explainability follows capture mode** (D131) + **over-capture hard-block** (D119): the system must never persist more than policy allows; `capture=none` persists no explainability artifacts.
- **No silent destructive actions in headless** (`SPEC-PM-001` NFR3): write operations require explicit user intent and must be auditable.
- **Single-writer capsule** (D7): capsule writes are serialized; runner must not violate lock/queue invariants.

## System Responsibilities (v1)

- Accept bot run requests for a given work item + kind (`research | review`).
- Build a deterministic context bundle from capsule + repo state.
- Execute the appropriate engine:
  - Research: NotebookLM synthesis (required) + web research
  - Review: validator/reviewer pass + optional patch staging
- Persist immutable artifacts (capsule SoR) + emit events.
- Provide progress + final summaries to Tier‑1 surfaces (TUI + CLI + headless JSON).

## Runtime Model (v1)

### Placement + Lifecycle

- Primary runtime: **background service spawned by the TUI** (tertiary service).
- Service lifecycle expectations:
  - Start on demand (when a bot run is triggered).
  - Exit when idle (policy/configurable) to avoid “permanent daemon” posture.
  - Expose a health/status endpoint for CLI/headless.

### CLI/Headless Integration

- CLI/headless must achieve Tier‑1 parity with the TUI runner semantics.
- Preferred approach: CLI/headless connects to the runner service via an IPC protocol (exact transport TBD).
- Acceptable fallback (if IPC is unavailable): in-process runner library with identical semantics.

### Idempotency + Concurrency

- Each run is identified by a unique `run_id`.
- Recommended constraints:
  - At most one active run per `(work_item_id, kind)`.
  - Global concurrency bounded (CPU, network, capsule single-writer).
- Cancellation:
  - Best-effort, cooperative cancellation.
  - Partial results must still be persisted with terminal `cancelled` status and a `BotRunLog`.

## Components (conceptual)

- **BotRunnerService**: run request API + job scheduling + progress reporting.
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

## References

- Interface contract: `docs/SPEC-PM-002-bot-runner/spec.md`
- PM system PRD: `docs/SPEC-PM-001-project-management/PRD.md`
