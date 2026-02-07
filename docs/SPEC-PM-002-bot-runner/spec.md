# SPEC-PM-002: Devin-Style Bot Runner (NeedsResearch / NeedsReview)

## Status: PLANNED (stub)

## Overview

Define the product semantics and Tier‑1 command surfaces for **manual** "Devin-style" automation bots that a PM can trigger by placing a work item into:

- `NeedsResearch` (run research bots)
- `NeedsReview` (run review bots)

This SPEC is intentionally **not** part of the default automatic workflow; it is an explicit/manual state transition used for optional automation.

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
- Fully autonomous implementation execution (coding/committing/merging) as a default mode.

## Inputs

- Work item + attached PRD/intake form data.
- Local product knowledge artifacts (default); optional NotebookLM escalation (configurable).
- Web research via Tavily MCP (default; pinned locally), with fallback to client default web research when Tavily is unavailable.

## Outputs (Artifacts)

Proposed artifact types (names and schemas TBD in this SPEC):

- `ResearchReport`: web research bundle + synthesis + recommended options/tradeoffs.
- `ReviewReport`: structured review notes with file/line references + risk assessment.
- `BotRunLog`: timing/cost summary + tool usage + success/failure diagnostics.

All artifacts must respect capture mode (`none | prompts_only | full_io`) and export safety constraints (locked by policy).

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
  - Artifact schemas + filesystem projection locations.
  - Command surface proposal under `code speckit pm ...` (plus TUI alias mapping 1:1).
  - Headless exit-code + JSON contract for bot runs.
  - Safety boundaries (tool allowlist) and capture-mode compliance.

## References

- PM system PRD: `docs/SPEC-PM-001-project-management/PRD.md`
- Historical stub: `docs/SPEC-PM-001-project-management/TODO-bot-runner-spec.md`
