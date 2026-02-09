# PRD: PM Bot System (Runner/Service/Tooling) (SPEC-PM-003)

**SPEC-ID**: SPEC-PM-003  
**Status**: Draft  
**Created**: 2026-02-09  
**Author**: Architect session (manual)

---

## Problem Statement

`SPEC-PM-002` defines the Tier‑1 interface contract for manual PM bot runs (`NeedsResearch`, `NeedsReview`). We still need an internal bot system that can execute those runs **safely**, **deterministically**, and **locally** while preserving Codex-RS’s core invariants:

- Capsule (`mv2://`) is the system-of-record for run evidence and artifacts.
- Tier‑1 parity across TUI/CLI/headless must be enforced structurally (not by convention).
- Headless automation must not prompt or silently take destructive actions.
- Capture-mode must be enforced with an over-capture hard-block.

Additionally, bot runs may be **long-lived** (hours → days). The product must ensure they can survive:

- TUI restarts / disconnects,
- process crashes,
- machine reboots.

This PRD defines the product requirements for the runner/service/tooling internals that realize the PM bot contract while remaining local-first and sovereignty-preserving.

---

## Goals

1. Provide a service-first runtime that can manage long-lived bot runs and resume incomplete runs after reboot.
2. Provide safe mutation for `NeedsReview` via bot-owned worktrees/branches (never touch the user’s active worktree by default).
3. Enforce capability boundaries (read/write/network/tool allowlists) centrally and audibly.
4. Persist authoritative bot outputs as capsule artifacts; treat the filesystem as a projection only.
5. Define concurrency/locking and best-effort cancellation semantics compatible with the capsule single-writer model.
6. Make replay/audit possible by snapshotting any non-capsule inputs used (e.g., local-memory retrievals, web research).

---

## Non-Goals (v1)

- A heavy, always-processing “agent framework” daemon.
- Fully automatic scheduling/queueing across the whole machine.
- Auto-merge/auto-push behavior.
- Cross-platform support (Linux-first remains baseline).

---

## Constraints

### Already Locked (must not change)

These constraints are locked in `docs/DECISIONS.md` and apply to the bot system internals:

- **Prefer no-daemon posture**: D38 (`docs/DECISIONS.md` → “Retrieval & Storage (D21-D40)”).
- **Tier‑1 parity + headless contract**: D113 + D133 (`docs/DECISIONS.md` → “Product & Parity (A1-A2)” and “ACE + Maieutics (H0-H7)”).
- **Maieutic pre-execution gate**: D130 (`docs/DECISIONS.md` → “ACE + Maieutics (H0-H7)”).
- **Capture-mode enforcement + over-capture hard-block**: D131 + D119 (`docs/DECISIONS.md` → “ACE + Maieutics (H0-H7)” and “Capture Mode (C1-C2)”).
- **Artifacts are authoritative SoR**: D114 (`docs/DECISIONS.md` → “Evidence Store (B1-B2)”).
- **Policy sovereignty hard constraints**: D125 (`docs/DECISIONS.md` → “Pipeline & Gates (D1-D2, E1-E2)”).
- **Capsule single-writer**: D7 (`docs/DECISIONS.md` → “Core Capsule Decisions (D1-D20)”).
- **No permanent daemon for maintenance**: D126 (`docs/DECISIONS.md` → “Maintenance (F1-F2)”).

### Proposed (needs confirmation)

- Whether to standardize on **NDJSON streaming progress events** (stdout) vs “final JSON only” for headless callers.
- Clarify the scope of acceptable “persistence” for a lightweight job-management service vs the D38/D126 no-daemon posture (see ADR-004).
- Whether to formalize retention/cleanup policies for large projections (worktrees, caches) as a separate spec/decision.

---

## Requirements (v1)

### Functional Requirements

| ID | Requirement | Acceptance Criteria |
| --- | --- | --- |
| FR1 | Service-first runtime | Bot runs are managed by a lightweight local service whose semantics match TUI/CLI/headless (Tier‑1). |
| FR2 | Reboot survival | Incomplete runs can resume after reboot via systemd user unit behavior without interactive prompts. |
| FR3 | Graceful degradation | When the service is down, the TUI provides read-only status from capsule artifacts and explicit service management actions. |
| FR4 | Deterministic context + checkpoints | The system snapshots required inputs and persists checkpoints so resume does not lose work or violate determinism/audit. |
| FR5 | Locking + concurrency | Enforce “at most one active run per `(work_item_id, kind)`”; respect capsule single-writer discipline. |
| FR6 | Permission enforcement | Central allowlist checks for network/tool/write; deny-by-default with explicit escalation flags. |
| FR7 | Research physiology | Research runs must be source-grounded and auditable; dependency posture (NotebookLM required vs degraded) is policy-defined. |
| FR8 | Review physiology | Review write mode stages changes in bot-owned worktree/branch; outputs remain reviewable against a reasonably current base. |
| FR9 | Artifact persistence | Persist `BotRunLog` and kind-specific outputs as capsule artifacts in a capture-mode-compliant way. |
| FR10 | Best-effort projections | Write rebuildable filesystem projections when allowed; projections never become SoR. |
| FR11 | Cancellation | Cooperative cancellation marks run terminal state and persists a log artifact with partial context (within capture policy). |

### Non-Functional Requirements

| ID | Requirement | Target | Validation |
| --- | --- | --- | --- |
| NFR1 | Fault isolation | Runner crashes don’t crash TUI | TUI spawns runner as child; non-zero exits handled |
| NFR2 | Replay / audit | Decisions can be replayed from capsule evidence | Evidence packs + artifact schemas |
| NFR3 | Capture compliance | No over-capture across any codepath | Policy tests + artifact writer checks |
| NFR4 | Minimal operational burden | Service is lightweight + user-scoped | systemd user unit + “idle when no jobs” posture |

---

## Deliverables (v1)

- A system design in `docs/SPEC-PM-003-bot-system/spec.md` that maps directly to the `SPEC-PM-002` interface contract.
- A concrete definition of the runner lifecycle, locking model, and permission boundaries.
- Worktree/branch naming conventions and staging semantics for write-enabled review runs.
- A deterministic evidence snapshot policy for any non-capsule inputs used during a run (local-memory + web research).

---

## Success Metrics

- A PM bot run can execute end-to-end without semantic divergence across surfaces.
- A long-lived run can survive a reboot and continue (resume) without bypassing maieutic gates.
- Concurrent invocations do not corrupt capsule state; lock contention resolves cleanly (blocked/queued behavior is explicit).
- Review write mode never mutates the user’s primary worktree and yields reviewable patches/worktrees.
- Capture-mode restrictions are enforced strictly; any over-capture attempt hard-blocks.

---

## Open Questions

- What IPC transport is acceptable between TUI/CLI/headless and the service (Unix socket vs stdio bridge vs in-process only)?
- What triggers resume after reboot (timer-driven, socket-activated, explicit “resume all incomplete”)?
- How should the system handle “freshness” for long-lived review runs (rebase/refresh boundaries and conflict posture)?
- What are the initial allowlisted local commands for `NeedsReview` validation, and where is that policy defined?

---

## References

- PM system PRD: `docs/SPEC-PM-001-project-management/PRD.md`
- Bot runner contract: `docs/SPEC-PM-002-bot-runner/spec.md`
- Bot system design: `docs/SPEC-PM-003-bot-system/spec.md`
- Runtime ADR (proposed): `docs/adr/ADR-004-pm-bot-service-runtime.md`
