# PRD: PM Bot Runner Interface Contract (SPEC-PM-002)

**SPEC-ID**: SPEC-PM-002  
**Status**: Draft  
**Created**: 2026-02-09  
**Author**: Architect session (manual)

---

## Problem Statement

Codex-RS introduces optional, manual “Devin-style” automation for PM holding states (`NeedsResearch`, `NeedsReview`). Without a Tier‑1 interface contract for bot runs, we risk:

- **Parity drift** between TUI/CLI/headless (automation behavior diverges across surfaces).
- **Non-deterministic headless behavior** (bots hanging/prompting in CI or scripts).
- **Unsafe defaults** (silent writes, unclear evidence/artifact persistence, capture-mode violations).
- **Unclear caller responsibilities** (what inputs are required, how to interpret outcomes, what artifacts exist).

`SPEC-PM-002` defines the *caller-facing* contract so the PM bot system can be invoked predictably and audited/replayed.

---

## Goals

1. Define Tier‑1 bot runner commands and their semantics across CLI/TUI/headless.
2. Define headless behavior guarantees (never prompt; structured output; stable exit codes).
3. Define the minimum artifact set required for auditability and human review (capsule as SoR).
4. Make safety boundaries caller-visible (read-only by default; explicit write isolation for reviews).
5. Ensure the interface contract composes with capsule-backed PM state (`SPEC-PM-001`).

---

## Non-Goals (v1)

- Defining the internal runner/service/queueing architecture (tracked in `SPEC-PM-003`).
- Automatically starting bots on every work item or on PM state transitions.
- Auto-committing/pushing/merging changes.
- Auto-transitioning work item states (bots recommend; PM acts).

---

## Primary Users / Callers (Tier‑1)

- **TUI**: spawns the canonical CLI command and renders progress/results (viewer model).
- **CLI**: interactive developer/PM usage.
- **Headless/CI**: deterministic, non-interactive usage in scripts and automation.

---

## Constraints

### Already Locked (must not change)

These constraints are locked in `docs/DECISIONS.md` and apply to the bot runner interface:

- **Tier‑1 parity**: D113 + D133 (`docs/DECISIONS.md` → “Product & Parity (A1-A2)” and “ACE + Maieutics (H0-H7)”).
- **Headless never prompts**: D133 (`docs/DECISIONS.md` → “ACE + Maieutics (H0-H7)”).
- **Maieutic gate is mandatory pre-execution**: D130 (`docs/DECISIONS.md` → “ACE + Maieutics (H0-H7)”).
- **Capture-mode governs persistence; over-capture hard-blocked**: D131 + D119 (`docs/DECISIONS.md` → “Capture Mode (C1-C2)” and “ACE + Maieutics (H0-H7)”).
- **Artifacts are authoritative; projections are rebuildable**: D114 (`docs/DECISIONS.md` → “Evidence Store (B1-B2)”).
- **Single-writer capsule**: D7 (`docs/DECISIONS.md` → “Core Capsule Decisions (D1-D20)”).

### Proposed (needs confirmation)

- Whether bot progress should be **final JSON only** or optionally **NDJSON streaming** on stdout (see `docs/SPEC-PM-003-bot-system/research-digest.md` for the proposal).
- The canonical, product-wide **exit code registry** for blocked/needs-input/needs-approval states (numbers vs named codes).

---

## Requirements (v1)

### Functional Requirements

| ID | Requirement | Acceptance Criteria |
| --- | --- | --- |
| FR1 | Run bots manually | Caller can invoke a bot run for a `(work_item_id, kind)` and receive a terminal result. |
| FR2 | Deterministic headless | Headless never prompts; missing prerequisites returns structured output + non-zero exit code. |
| FR3 | Stable artifacts | Each run produces capsule artifacts (SoR) appropriate to kind/capture mode (e.g. `BotRunLog`, `ResearchReport`/`ReviewReport`, optional `PatchBundle`, optional `WebResearchBundle`). |
| FR4 | Safe defaults | Default is read-only; review write mode is explicit and isolated (bot-owned worktree/branch). |
| FR5 | Status visibility | Caller can query latest run status/results across CLI/TUI/headless using the same contract. |

### Non-Functional Requirements

| ID | Requirement | Target | Validation |
| --- | --- | --- | --- |
| NFR1 | Tier‑1 parity | Same inputs → same semantics across surfaces | Integration tests + doc review vs `SPEC-PM-002` |
| NFR2 | Replay/audit readiness | Run outputs are reconstructible from capsule artifacts | Artifact schema + projection rebuildability checks |
| NFR3 | Capture compliance | Never persist more than policy allows | Unit tests at artifact writer boundaries |
| NFR4 | Safety posture | No silent writes or destructive actions | Explicit flags + headless blocks |

---

## Deliverables (v1)

- A stable interface contract in `docs/SPEC-PM-002-bot-runner/spec.md` (commands, inputs, outputs, exit codes, artifact set).
- A minimal JSON result schema for headless usage, including terminal status + artifact URIs.
- Documentation of safe write isolation semantics (`--write-mode worktree`) and required result fields.

---

## Success Metrics

- A bot run can be invoked from CLI, TUI, and headless with identical semantics (Tier‑1 parity).
- Headless invocations never hang; missing prerequisites surface as structured “blocked/needs input” outcomes.
- For review write mode, the user can inspect proposed changes without their primary worktree being modified.

---

## Open Questions

- Should `BLOCKED` be a dedicated exit code or “exit 2 with structured `blocked_reason`”?
- Do we require optional **streaming progress** (NDJSON) for long runs, or is final JSON sufficient for v1?
- What is the canonical filesystem projection root for PM bot run outputs (`docs/` vs `.speckit/`)?

---

## References

- Interface contract: `docs/SPEC-PM-002-bot-runner/spec.md`
- Bot system design: `docs/SPEC-PM-003-bot-system/spec.md`
- PM system PRD: `docs/SPEC-PM-001-project-management/PRD.md`

