# PRD: Capsule-Backed Project / Product Management (SPEC-PM-001)

**SPEC-ID**: SPEC-PM-001
**Status**: Draft
**Created**: 2026-02-05
**Author**: Architect session (manual)

---

## Problem Statement

Codex-RS currently has strong “one-shot” feature execution via Spec‑Kit (`/speckit.auto`), but it lacks a **first-class project/product management layer**:

- Feature work is tracked across multiple docs (roadmaps, PRDs, ad-hoc notes), causing drift.
- There is no capsule-backed system-of-record for “what are we building next?”, “what is in progress?”, “what is deprecated?”, etc.
- PRDs can exist without clear linkage to canonical tracking (`codex-rs/SPEC.md`) and without consistent lifecycle states (planned → in progress → shipped → deprecated/archived).

This is a product vision gap: the same tools used to build Codex-RS should also manage its roadmap, features, and documentation lifecycle.

---

## Goals

1. **Single system-of-record (SoR)** for features/specs/tasks and their lifecycle states, stored in the capsule.
2. **Canonical filesystem projections** for humans and tooling (including `codex-rs/SPEC.md` as the canonical tracker).
3. **Maieutic PRD sessions**: guided intake that produces a PRD artifact and links it to the tracker entry.
4. **Status surfaces across CLI/TUI/headless** so progress is visible without opening documents.
5. **Deprecation + archival lifecycle**: mark docs deprecated/superseded and produce archive packs with canonical pointers.

---

## Non-Goals (for initial iteration)

- Replacing GitHub Issues/Projects as a general-purpose PM tool.
- Full multi-repo portfolio management.
- Automating implementation work selection (this remains a human/architect decision).

---

## Scope & Assumptions

**In scope**:
- A capsule-backed registry of “work items” (at minimum: Specs/Features and Doc lifecycle records).
- Filesystem projections for trackers/registers (starting with `codex-rs/SPEC.md`; later `docs/DEPRECATIONS.md` projections/events).
- CLI/TUI/headless status views for the registry.
- A minimal maieutic PRD flow that produces a PRD artifact + links it to a tracker ID.

**Constraints**:
- Tier‑1 multi-surface parity (CLI/TUI/headless) for any workflow-impacting behavior.
- Headless must be deterministic and must not prompt.
- Local platform focus: Linux-only build/test expectations for CI.

---

## Lifecycle States (v1)

Work items have a single **state** at a time:

- `Backlog`: Not scheduled yet (default for new work items).
- `NeedsResearch`: Optional manual holding state to run "Devin-style" research automation.
- `Planned`: Scheduled / approved to start. Promotion to `Planned` is explicitly invoked and gated (see below).
- `InProgress`: Actively being worked.
- `NeedsReview`: Optional manual holding state to run "Devin-style" review automation.
- `Completed`: Done (definition depends on item type; for specs, typically "merged + verified").
- `Deprecated`: No longer applicable to product direction; retained for history with pointers.
- `Archived`: Terminal historical record (kept only as a pointer + archived pack).

**Allowed transitions (v1)**

- `Backlog` → `NeedsResearch` → `Backlog` or `Planned`
- `Planned` → `NeedsResearch` → `Backlog` or `Planned`
- `Planned` → `InProgress` → `Completed`
- `InProgress` → `NeedsReview` → `InProgress` or `Completed`
- Any non-terminal state → `Deprecated` → `Archived` (explicit/manual)

---

## Planned Promotion Gates (v1)

Promotion to `Planned` is **manual** (PM action) and must satisfy:

1. **Deterministic quality score ≥ 90/100**
   - Computed from deterministic checks only (no model variance).
   - A model-graded rubric may be generated as advisory feedback, but it does not affect the numeric gate.
2. **Open Questions must be empty**
   - If open questions are present, the work item cannot be marked `Planned`.

Headless must return structured output and product exit codes for blocking states (no clap default exit=2 fallbacks).

---

## Assisted Maieutic PRD Sessions (v1)

**Assisted** is the default UX:

- Interaction is chat-style (chat modal in TUI is acceptable).
- The system provides suggestions/recommendations (including multiple-choice with a recommended option).
- The session must still fully populate the existing intake "FORM" fields, including deep fields (no separate `--deep` mode for assisted).

**Inputs (high level)**

- Local product knowledge by default; configurable escalation to NotebookLM (Tier2).
- Web research via Tavily MCP by default; fallback to built-in client web search when Tavily is unavailable.

**Outputs**

- A PRD artifact linked to the work item.
- A deterministic score report (numeric gate).
- An advisory model rubric report (persisted for audit/review).

---

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

---

## Functional Requirements

| ID | Requirement | Acceptance Criteria | Priority |
| --- | --- | --- | --- |
| FR1 | Capsule-backed work registry | Create/read/update work items with stable IDs and states (`Backlog`, `NeedsResearch`, `Planned`, `InProgress`, `NeedsReview`, `Completed`, `Deprecated`, `Archived`). | P1 |
| FR2 | `codex-rs/SPEC.md` canonical updates | Promoting a work item to `Planned` inserts/updates a row in the `codex-rs/SPEC.md` Planned table deterministically (and demotion removes/updates). | P1 |
| FR3 | Assisted maieutic PRD session | Guided chat populates the full intake form (incl. deep fields), produces a PRD artifact linked to its work item, and emits deterministic score + advisory rubric artifacts. | P1 |
| FR4 | Status surfaces | CLI + TUI + headless can list work items and show state consistently (including NeedsResearch/NeedsReview). | P1 |
| FR5 | Deprecation/archival support | Work items/docs can be marked deprecated/superseded; archived packs are addressable from a canonical register. | P2 |

---

## Non-Functional Requirements

| ID | Requirement | Target | Validation |
| --- | --- | --- | --- |
| NFR1 | Deterministic projections | Same capsule state → identical projection output | Snapshot tests / hash comparison |
| NFR2 | Fast status | Status query < 1s on local machine | CLI timing in CI |
| NFR3 | Safety | No silent destructive actions in headless mode | Explicit flags + structured output |

---

## Success Metrics

- “What is planned/in progress?” is answerable from **one** canonical tracker (`codex-rs/SPEC.md`) kept in sync with capsule state.
- PRDs and deprecations are linked to tracker IDs and have explicit lifecycle state.
- CLI/TUI/headless show consistent status for Tier‑1 PM flows.

---

## Open Questions

- Exact schema for work items (feature vs spec vs task) and which fields must be immutable.
- Whether `docs/DEPRECATIONS.md` becomes a projection of capsule events in the first iteration or later.
- How to represent “archived packs” as first-class capsule artifacts (URI scheme, metadata).
- Deterministic scoring rubric definition (what contributes to the 0–100 score, and weights).
- Where and how web research artifacts are stored for audit/replay (hashes/IDs, retention, redaction).
- What automation "bot runner" semantics exist for `NeedsResearch` / `NeedsReview` (manual-only state vs queue semantics, scheduling, visibility in status surfaces).

---

## Supporting Docs

- `docs/SPEC-PM-001-project-management/ARCHITECT-BRIEF-maieutic-and-prd.md`: research + design drift analysis for assisted maieutics and PRD generation.
