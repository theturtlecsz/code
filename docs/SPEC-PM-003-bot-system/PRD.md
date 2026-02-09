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

This PRD defines the product requirements for the runner/service/tooling internals that realize the PM bot contract without introducing a daemon-oriented “agent framework”.

---

## Goals

1. Implement the baseline runtime model for bot runs as a deterministic, crash-safe execution unit (“one run = one bounded execution”).
2. Provide safe mutation for `NeedsReview` via bot-owned worktrees/branches (never touch the user’s active worktree by default).
3. Enforce capability boundaries (read/write/network/tool allowlists) centrally and audibly.
4. Persist authoritative bot outputs as capsule artifacts; treat filesystem/Linear as projections only.
5. Define concurrency/locking and best-effort cancellation semantics compatible with the capsule single-writer model.
6. Make replay/audit possible by snapshotting any non-capsule inputs used (e.g., local-memory retrievals, web research).

---

## Non-Goals (v1)

- Always-on background daemon/service.
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
- Whether to ship an **optional on-demand runner service** (short-lived, non-required) for queueing/progress fan-out.
- Whether to formalize retention/cleanup policies for large projections (worktrees, caches) as a separate spec/decision.

---

## Requirements (v1)

### Functional Requirements

| ID | Requirement | Acceptance Criteria |
| --- | --- | --- |
| FR1 | Ephemeral baseline runner | Bot runs execute as a bounded command invocation whose semantics match headless/CLI/TUI (Tier‑1). |
| FR2 | Deterministic context bundle | Runner snapshots required inputs (capsule artifacts + repo metadata); detects drift at apply/merge boundaries rather than mid-run. |
| FR3 | Locking + concurrency | Enforce “at most one active run per `(work_item_id, kind)`”; respect capsule single-writer discipline. |
| FR4 | Permission enforcement | Central allowlist checks for network/tool/write; deny-by-default with explicit escalation flags. |
| FR5 | Research physiology | `NeedsResearch` hard-requires NotebookLM; missing config → blocked result (no silent downgrade). |
| FR6 | Review physiology | `NeedsReview` write mode stages changes in bot-owned worktree/branch; emits a `PatchBundle` artifact for review/apply. |
| FR7 | Artifact persistence | Persist `BotRunLog` and kind-specific outputs as capsule artifacts in a capture-mode-compliant way. |
| FR8 | Best-effort projections | Write rebuildable filesystem projections when allowed; projections never become SoR. |
| FR9 | Cancellation | Cooperative cancellation marks run terminal state and persists a log artifact with partial context (within capture policy). |

### Non-Functional Requirements

| ID | Requirement | Target | Validation |
| --- | --- | --- | --- |
| NFR1 | Fault isolation | Runner crashes don’t crash TUI | TUI spawns runner as child; non-zero exits handled |
| NFR2 | Replay / audit | Decisions can be replayed from capsule evidence | Evidence packs + artifact schemas |
| NFR3 | Capture compliance | No over-capture across any codepath | Policy tests + artifact writer checks |
| NFR4 | Minimal operational burden | No required background service | Runs work via CLI invocation only |

---

## Deliverables (v1)

- A system design in `docs/SPEC-PM-003-bot-system/spec.md` that maps directly to the `SPEC-PM-002` interface contract.
- A concrete definition of the runner lifecycle, locking model, and permission boundaries.
- Worktree/branch naming conventions and staging semantics for write-enabled review runs.
- A deterministic evidence snapshot policy for any non-capsule inputs used during a run (local-memory + web research).

---

## Success Metrics

- A PM bot run can execute end-to-end without a daemon and without semantic divergence across surfaces.
- Concurrent invocations do not corrupt capsule state; lock contention resolves cleanly (blocked/queued behavior is explicit).
- Review write mode never mutates the user’s primary worktree and yields reviewable patches/worktrees.
- Capture-mode restrictions are enforced strictly; any over-capture attempt hard-blocks.

---

## Open Questions

- What IPC transport (if any) is acceptable for an optional on-demand runner service (Unix socket vs stdio bridge)?
- Where should a persistent run queue live, if we add one (capsule-backed events vs ephemeral in-memory)?
- What are the initial allowlisted local commands for `NeedsReview` validation, and where is that policy defined?

---

## References

- PM system PRD: `docs/SPEC-PM-001-project-management/PRD.md`
- Bot runner contract: `docs/SPEC-PM-002-bot-runner/spec.md`
- Bot system design: `docs/SPEC-PM-003-bot-system/spec.md`

