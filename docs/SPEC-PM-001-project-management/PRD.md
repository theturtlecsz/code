# PRD: Capsule-Backed Project / Product Management (SPEC-PM-001)

**SPEC-ID**: SPEC-PM-001
**Status**: Draft
**Created**: 2026-02-05
**Author**: Architect session (manual)

***

## Problem Statement

Codex-RS currently has strong “one-shot” feature execution via Spec‑Kit (`/speckit.auto`), but it lacks a **first-class project/product management layer**:

* Feature work is tracked across multiple docs (roadmaps, PRDs, ad-hoc notes), causing drift.
* There is no capsule-backed system-of-record for “what are we building next?”, “what is in progress?”, “what is deprecated?”, etc.
* PRDs can exist without clear linkage to canonical tracking (`codex-rs/SPEC.md`) and without consistent lifecycle states (planned → in progress → shipped → deprecated/archived).

This is a product vision gap: the same tools used to build Codex-RS should also manage its roadmap, features, and documentation lifecycle.

***

## Goals

1. **Single system-of-record (SoR)** for features/specs/tasks and their lifecycle states, stored in the capsule.
2. **Canonical filesystem projections** for humans and tooling (including `codex-rs/SPEC.md` as the canonical tracker).
3. **Maieutic PRD sessions**: guided intake that produces a PRD artifact and links it to the tracker entry.
4. **Status surfaces across CLI/TUI/headless** so progress is visible without opening documents.
5. **Deprecation + archival lifecycle**: mark docs deprecated/superseded and produce archive packs with canonical pointers.

***

## Non-Goals (for initial iteration)

* Replacing GitHub Issues/Projects as a general-purpose PM tool.
* Full multi-repo portfolio management.
* Automating implementation work selection (this remains a human/architect decision).

***

## Scope & Assumptions

**In scope**:

* A capsule-backed registry of “work items” (at minimum: Specs/Features and Doc lifecycle records).
* Filesystem projections for trackers/registers (starting with `codex-rs/SPEC.md`; later `docs/DEPRECATIONS.md` projections/events).
* CLI/TUI/headless status views for the registry.
* A minimal maieutic PRD flow that produces a PRD artifact + links it to a tracker ID.

**Constraints**:

* Tier‑1 multi-surface parity (CLI/TUI/headless) for any workflow-impacting behavior.
* Headless must be deterministic and must not prompt.
* Local platform focus: Linux-only build/test expectations for CI.

### Deferred (confirmed but not in next cycle)

* **SPEC.md deterministic projection** (PM-D5/D27): confirmed as eventual target but deferred. TUI experience is the priority for the next cycle.
* **DEPRECATIONS.md projection**: deferred indefinitely. No timeline.
* **Archived pack URI scheme**: deferred.

### Anti-Goals

* **Not a JIRA clone**: solo developer workflow only. No multi-user collaboration in v1.
* **No auto-merge**: bots never auto-commit/push/merge without explicit human approval.

***

## Lifecycle States (v1)

Work items have a single **state** at a time:

* `Backlog`: Not scheduled yet (default for new work items).
* `NeedsResearch`: Optional manual holding state to run "Devin-style" research automation (semantics tracked in `SPEC-PM-002`; dependency posture is policy-defined).
* `Planned`: Scheduled / approved to start. Promotion to `Planned` is explicitly invoked and gated (see below).
* `InProgress`: Actively being worked.
* `NeedsReview`: Optional manual holding state to run "Devin-style" review automation (semantics tracked in `SPEC-PM-002`).
* `Completed`: Done (definition depends on item type; for specs, typically "merged + verified").
* `Deprecated`: No longer applicable to product direction; retained for history with pointers.
* `Archived`: Terminal historical record (kept only as a pointer + archived pack).

**Allowed transitions (v1)**

* `Backlog` → `NeedsResearch` → `Backlog` or `Planned`
* `Planned` → `NeedsResearch` → `Backlog` or `Planned`
* `Planned` → `InProgress` → `Completed`
* `InProgress` → `NeedsReview` → `InProgress` or `Completed`
* Any non-terminal state → `Deprecated` → `Archived` (explicit/manual)

***

## Work Item Schema (v1) — PM-D25

All four hierarchy levels must be schema-defined. Shared and level-specific fields below.

### Shared Fields (all levels)

| Field         | Type                                       | Immutable | Notes                                   |
| ------------- | ------------------------------------------ | --------- | --------------------------------------- |
| `id`          | String (SPEC-ID format)                    | Yes       | Unique identifier                       |
| `type`        | Enum: `project \| feature \| spec \| task` | Yes       | Hierarchy level                         |
| `title`       | String                                     | No        | Human-readable name                     |
| `description` | String (optional)                          | No        | Brief description                       |
| `created_at`  | String (RFC3339)                           | Yes       | Creation timestamp                      |
| `updated_at`  | String (RFC3339)                           | No        | Last modification                       |
| `parent_id`   | String (optional)                          | Yes       | Parent in hierarchy (null for projects) |
| `state`       | Enum (level-dependent)                     | No        | Current lifecycle state                 |

### Level-Specific Fields

**Project** (container only; PM-D2):

* No additional fields. No lifecycle state (state field is always `null`).

**Feature** (full lifecycle; PM-D3):

* `acceptance_criteria`: String\[] (optional)
* `priority`: Enum `p0 | p1 | p2 | p3` (optional)
* `state`: Enum `backlog | needs_research | planned | in_progress | needs_review | completed | deprecated | archived`
* `return_state`: Enum (optional; set when entering a holding state; PM-D28)

**SPEC** (full lifecycle; PM-D3):

* `prd_uri`: String (optional; capsule URI of linked PRD artifact)
* `quality_score`: u32 (optional; deterministic score 0-100)
* `state`: same as Feature
* `return_state`: same as Feature

**Task** (ternary; PM-D4):

* `assignee`: String (optional)
* `result`: String (optional; completion notes)
* `state`: Enum `open | completed | failed`

### Immutability Rules (PM-D from session 1: minimal immutable set)

Only `id`, `type`, `created_at`, and `parent_id` are immutable after creation. All other fields are mutable.

***

## Command Surface (v1)

Canonical CLI namespace: `code speckit pm ...`

TUI should provide a short alias (e.g., `/pm ...`) that maps 1:1 onto the canonical CLI behavior (Tier‑1 parity requirement).

***

## TUI Interaction Design

> **Extracted to SPEC-PM-004** (D138/PM-D26).
>
> PM-001 owns the work item data model. The TUI PM interaction design (views, navigation, information hierarchy, behaviors) is defined in `docs/SPEC-PM-004-tui-ux/spec.md`.

***

## Planned Promotion Gates (v1)

Promotion to `Planned` is **manual** (PM action) and must satisfy:

1. **Deterministic quality score ≥ 90/100**
   * Computed from deterministic checks only (no model variance).
   * A model-graded rubric may be generated as advisory feedback, but it does not affect the numeric gate.
2. **Open Questions must be empty**
   * If open questions are present, the work item cannot be marked `Planned`.

Headless must return structured output and product exit codes for blocking states (no clap default exit=2 fallbacks).

***

## Bot Automation Holding States (v1) — tracked in `SPEC-PM-002` / `SPEC-PM-003`

These are **manual** holding states used for optional automation (not part of the default `/speckit.auto` workflow):

* Entering a holding state does **not** automatically start a bot run; it makes the work item eligible for a manual run (see `SPEC-PM-002`).
* Per D140, entering a holding state must record a `return_state`. When the bot run reaches a terminal state, the system auto-transitions the work item back to `return_state` (this is not a "bot recommendation" transition).

* `NeedsResearch`:
  * NotebookLM (or equivalent Tier‑2 grounding) posture is **policy-defined**:
    * if required but unavailable, the run returns a BLOCKED result with structured output,
    * if degraded operation is allowed, the run proceeds with explicit degraded labeling.
  * Network access is allowed (Tavily MCP preferred; generic web research allowed).
* `NeedsReview`:
  * Validator/reviewer runs with tool access and may create worktrees/branches to stage suggested changes.
  * Bot output must be able to write back a summarized response into TUI status surfaces.

See:

* Interface contract (commands/exit codes/artifacts): `docs/SPEC-PM-002-bot-runner/spec.md`
* Bot system design (runner/service/tooling internals): `docs/SPEC-PM-003-bot-system/spec.md`

***

## Deterministic PRD Quality Score (v1)

The numeric quality score is **deterministic-only** (no model variance) and is computed from the completed intake form.

**Purpose**

* Provide a deterministic "ready to plan" threshold (≥ 90/100).
* Keep the model-graded rubric advisory (persisted for audit/review but not gating).

**Proposed deterministic rubric (v0)**

Score ranges from 0–100 and is the sum of the following components:

1. **Problem + Outcome clarity (20)**
   * Problem statement present and non-trivial length (10)
   * Outcome present and non-trivial length (10)
2. **Scope + Non-goals quality (20)**
   * Scope-in list meets min count and contains non-placeholder entries (10)
   * Non-goals list meets min count and contains non-placeholder entries (10)
3. **Acceptance criteria verifiability (20)**
   * Minimum number of criteria present (10)
   * Each criterion includes an explicit verification method (10)
4. **Integration + Constraints coverage (15)**
   * Integration points present and not "unknown"/placeholder (10)
   * Constraints present and non-placeholder (5)
5. **Risk coverage (10)**
   * Risks present and non-placeholder (5)
   * Assumptions present if applicable (or explicitly "none") (5)
6. **Deep fields completeness (15)** (assisted default)
   * Architecture components + dataflows present (5)
   * Integration mapping + test plan present (5)
   * Threat model + rollout plan + risk register + non-goals rationale present (5)

**Hard caps**

* If any required form field is missing/invalid under existing validation, score is capped at 0 (cannot be promoted).
* If open questions are non-empty, promotion is blocked regardless of score (see Planned Promotion Gates).

***

## Assisted Maieutic PRD Sessions (v1)

**Assisted** is the default UX:

* Interaction is chat-style (chat modal in TUI is acceptable).
* The system provides suggestions/recommendations (including multiple-choice with a recommended option).
* The session must still fully populate the existing intake "FORM" fields, including deep fields (no separate `--deep` mode for assisted).

**Inputs (high level)**

* Local product knowledge by default; configurable escalation to NotebookLM (Tier2).
* Web research via Tavily MCP by default; fallback to built-in client web search when Tavily is unavailable.

**Outputs**

* A PRD artifact linked to the work item.
* A deterministic score report (numeric gate).
* An advisory model rubric report (persisted for audit/review).

***

## Tavily Web Research (via MCP)

Web research is provided via a pinned local MCP server (`tavily-mcp`) configured in user config (not repo `.env`):

```toml
[mcp_servers.tavily]
command = "npx"
args = ["-y", "tavily-mcp@0.2.16"]
env = { TAVILY_API_KEY = "tvly-REDACTED" }
```

If Tavily is down/unreachable, assisted intake falls back to the client's default web search tool.

All web research used to form recommendations should be captured into capsule artifacts (query + params + source list + hashes/IDs) to preserve auditability.

***

## Web Research Artifacts (v1)

**Goal**: capture enough information to support audit/replay without violating capture-mode or export safety constraints.

**Proposed artifact: `WebResearchBundle` (v0)**

* Stored in capsule as structured JSON.
* Filesystem projection is best-effort (e.g., under `docs/<SPEC_ID>/research/web_research_bundle.json`).

**Fields (high level)**

* `schema_version`
* `spec_id`, `run_id`
* `provider`: `tavily-mcp` or `client-web-search`
* `capture_mode`: `prompts_only` or `full_io` (mirrors PolicySnapshot capture mode)
* `requests[]`:
  * `tool` (e.g., `tavily-search`, `tavily-extract`)
  * `query` / `urls`
  * `params` (depth, max results, filters)
  * `retrieved_at`
  * `results[]` (minimal metadata)
  * `selected[]` (URLs actually used to form recommendations)
  * `errors[]`
* `bundle_hash` (sha256 of canonicalized bundle JSON)

**Capture-mode rules**

* `prompts_only` (export-safe):
  * Store query + params and a **full result template** (not just titles):
    * URL + domain
    * title (if available)
    * ranking/score (if provided)
    * published/updated timestamp (if available)
    * snippet/summary fields (if provided), with size caps
    * hashes for any larger text fields to support audit/replay without requiring raw storage
  * Optionally store a **temporary non-export-safe cache** for selected sources to avoid refetching during the same session:
    * extracted content is stored separately (by URI) with explicit TTL/expiry metadata
    * cache artifacts must be excluded from safe export and are not required for deterministic gating
* `full_io` (not export-safe):
  * May store extracted page text as separate capsule artifacts referenced by URI.

***

## Functional Requirements

| ID  | Requirement                          | Acceptance Criteria                                                                                                                                                         | Priority |
| --- | ------------------------------------ | --------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | -------- |
| FR1 | Capsule-backed work registry         | Create/read/update work items with stable IDs and states (`Backlog`, `NeedsResearch`, `Planned`, `InProgress`, `NeedsReview`, `Completed`, `Deprecated`, `Archived`).       | P1       |
| FR2 | `codex-rs/SPEC.md` canonical updates | Promoting a work item to `Planned` inserts/updates a row in the `codex-rs/SPEC.md` Planned table deterministically (and demotion removes/updates).                          | P1       |
| FR3 | Assisted maieutic PRD session        | Guided chat populates the full intake form (incl. deep fields), produces a PRD artifact linked to its work item, and emits deterministic score + advisory rubric artifacts. | P1       |
| FR4 | Status surfaces                      | CLI + TUI + headless can list work items and show state consistently (including NeedsResearch/NeedsReview).                                                                 | P1       |
| FR5 | Deprecation/archival support         | Work items/docs can be marked deprecated/superseded; archived packs are addressable from a canonical register.                                                              | P2       |

***

## Non-Functional Requirements

| ID   | Requirement               | Target                                           | Validation                         |
| ---- | ------------------------- | ------------------------------------------------ | ---------------------------------- |
| NFR1 | Deterministic projections | Same capsule state → identical projection output | Snapshot tests / hash comparison   |
| NFR2 | Fast status               | Status query < 1s on local machine               | CLI timing in CI                   |
| NFR3 | Safety                    | No silent destructive actions in headless mode   | Explicit flags + structured output |

***

## Success Metrics

* “What is planned/in progress?” is answerable from **one** canonical tracker (`codex-rs/SPEC.md`) kept in sync with capsule state.
* PRDs and deprecations are linked to tracker IDs and have explicit lifecycle state.
* CLI/TUI/headless show consistent status for Tier‑1 PM flows.

***

## Open Questions

* ~~Exact schema for work items~~ Resolved: see "Work Item Schema (v1)" section above.
* Whether `docs/DEPRECATIONS.md` becomes a projection of capsule events in the first iteration or later. **Deferred** (PM-D27).
* How to represent "archived packs" as first-class capsule artifacts (URI scheme, metadata). **Deferred**.
* Confirm deterministic scoring rubric weights/threshold behavior (see "Deterministic PRD Quality Score"). **Not blocking; iterate.**
* Confirm web research template size caps + cache retention/TTL for `prompts_only` temporary content cache.
* ~~Bot runner interface + system design for NeedsResearch/NeedsReview~~ Resolved: tracked in SPEC-PM-002/003.

***

## Supporting Docs

* `docs/SPEC-PM-001-project-management/ARCHITECT-BRIEF-maieutic-and-prd-generation.md`: research + design drift analysis for assisted maieutics and PRD generation.
* `docs/SPEC-PM-002-bot-runner/spec.md`: SPEC stub for Devin-style research/review bot runner semantics.
