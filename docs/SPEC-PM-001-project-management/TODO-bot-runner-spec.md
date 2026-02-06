# TODO Spec: Devin-Style Bot Runner (NeedsResearch / NeedsReview)

**Status**: Draft (TODO)
**Parent**: `SPEC-PM-001`
**Date**: 2026-02-06

---

## Purpose

Define the product semantics and technical surface for "Devin-style" automation that a PM can trigger by manually placing a work item into:

- `NeedsResearch` (run research bots)
- `NeedsReview` (run review bots)

This is intentionally **not** part of the default automatic workflow; it is an explicit/manual state transition.

---

## Goals

- Define what "research bots" and "review bots" do (minimum viable behaviors).
- Define artifacts produced and how they are persisted to capsule + projected to filesystem.
- Define CLI/TUI/headless commands to trigger runs and to view results (Tier‑1 parity).
- Define failure semantics (exit codes + structured output for headless).

---

## Non-Goals (initial)

- Fully autonomous implementation selection or self-directed execution.
- Running bots automatically on every PR/spec by default.
- Cross-platform support (Linux-only remains the baseline expectation).

---

## Inputs

- Work item + PRD/intake form data.
- Local product knowledge artifacts (default), optional NotebookLM escalation.
- Web research (Tavily MCP default, fallback to client web search).

---

## Outputs (Artifacts)

Proposed artifact types (names TBD):

- `ResearchReport`: web research bundle + synthesis + recommended options/tradeoffs.
- `ReviewReport`: structured review notes with file/line references + risk assessment.
- `BotRunLog`: timing/cost summaries + tool usage + success/failure diagnostics.

All artifacts must respect capture mode (`none | prompts_only | full_io`) and export safety constraints.

---

## Open Questions

1. Bot runner execution model:
   - Is this a single "bot" or a fixed sequence (research → synthesis → report)?
   - Does it run within spec-kit stages, or as an independent PM command?
2. Scheduling:
   - Purely on-demand, or can a PM queue items for periodic processing?
3. Cost/latency budgets:
   - Default budget per run (hard cap) and how it's enforced.
4. Artifact projection:
   - Where do reports live on disk (stable paths)? How are they referenced from the work item and `SPEC.md`?
5. Security:
   - What tools are allowed during bot runs (web-only vs shell vs repo reads)?
6. Headless contract:
   - Exact exit codes for NEEDS_INPUT / NEEDS_APPROVAL / BLOCKED_SHIP / BOT_FAILED.

---

## Minimal MVP (suggested)

- A command to place an item into `NeedsResearch` and run a single research pass that emits:
  - `WebResearchBundle` + a short `ResearchReport` (structured JSON + Markdown projection).
- A command to place an item into `NeedsReview` and run a single deterministic/static review pass that emits:
  - `ReviewReport` with "must fix" vs "suggestions" plus a summarized risk list.

