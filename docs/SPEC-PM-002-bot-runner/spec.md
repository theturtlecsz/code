# SPEC-PM-002: Devin-Style Bot Runner (NeedsResearch / NeedsReview)

## Status: PLANNED (stub)

## Overview

Define the product semantics and Tier‑1 command surfaces for **manual** "Devin-style" automation bots that a PM can trigger by placing a work item into:

- `NeedsResearch` (run research bots)
- `NeedsReview` (run review bots)

This SPEC is intentionally **not** part of the default automatic workflow; it is an explicit/manual state transition used for optional automation.

This automation is expected to be **in-depth** and may need to evolve into a dedicated “bot system” (runner/service/tooling) tracked separately from the PM semantics (see Open Questions).

## Goals

- Define minimum viable behaviors for:
  - Research bots (`NeedsResearch`)
  - Review bots (`NeedsReview`)
- Define artifacts produced and how they are persisted to capsule + projected to filesystem.
- Define CLI/TUI/headless commands to trigger runs and to view results (Tier‑1 parity).
- Define headless behavior contract (structured output + product exit codes; never prompt).
- Define tool and safety boundaries for bot runs (what can be read/executed).

## Non-Goals (initial)

- Running bots automatically on every PR/spec by default.
- Cross-platform support (Linux-only remains baseline).
- Auto-committing/pushing/merging as a default mode.

## Inputs

- Work item + attached PRD/intake form data.
- Capsule artifacts linked to the work item (intake/grounding/reports/evidence).
- NotebookLM is **required** for `NeedsResearch` runs; if unavailable, the run hard-fails as **BLOCKED** with structured output (no fallback research).
- Web research is allowed via both:
  - Tavily MCP (preferred; pinned locally), and
  - the client’s default/generic web research tooling.

## Outputs (Artifacts)

Proposed artifact types (names and schemas TBD in this SPEC):

- `ResearchReport`: web research bundle + synthesis + recommended options/tradeoffs.
- `ReviewReport`: structured review notes with file/line references + risk assessment.
- `BotRunLog`: timing/cost summary + tool usage + success/failure diagnostics.

All artifacts must respect capture mode (`none | prompts_only | full_io`) and export safety constraints (locked by policy).

## Execution Model (v1)

- Implemented as a **background service spawned by the TUI** (tertiary service).
- Must still provide Tier‑1 parity across CLI/TUI/headless for automation-critical behavior (commands, artifacts, gating semantics, exit codes).
- Validator/reviewer bot may create **worktrees/branches** to stage suggested changes and persist patch context.
- Bot results must be able to write back a summarized response into the main TUI conversation/status surfaces.

## Tier‑1 Constraints (Already Locked)

- **Multi-surface parity** (D133): CLI/TUI/headless must share semantics for Tier‑1 behavior.
- **Headless never prompts** (D133): missing required inputs must return product exit codes + structured output.
- **Maieutic step is mandatory for `/speckit.auto`** (D130): bot runner must not create a bypass of required gates.

## Minimal MVP (suggested)

- A command to place an item into `NeedsResearch` and run a single research pass that emits:
  - `WebResearchBundle` + a short `ResearchReport` (structured JSON + Markdown projection).
- A command to place an item into `NeedsReview` and run a single deterministic/static review pass that emits:
  - `ReviewReport` with "must fix" vs "suggestions" plus a summarized risk list.

## Definition of Done

- PRD/design doc produced for this SPEC with:
  - Bot runner execution model (on-demand vs queued), idempotency, and visibility in status surfaces.
  - NotebookLM hard dependency behavior for `NeedsResearch` (blocked exit code + structured output; no fallback).
  - Worktree/branch creation semantics and write boundaries (what may be written, and what is forbidden by default).
  - TUI write-back mechanism for bot outputs (status/events + report linking).
  - Artifact schemas + filesystem projection locations.
  - Command surface proposal under `code speckit pm ...` (plus TUI alias mapping 1:1).
  - Headless exit-code + JSON contract for bot runs.
  - Safety boundaries (tool allowlist) and capture-mode compliance.

## Open Questions

- Should the “bot runner/service” be tracked as its own dedicated SPEC (separate from PM semantics), with `SPEC-PM-002` focused on product semantics and surfaces?

## References

- PM system PRD: `docs/SPEC-PM-001-project-management/PRD.md`
- Historical stub: `docs/SPEC-PM-001-project-management/TODO-bot-runner-spec.md`
