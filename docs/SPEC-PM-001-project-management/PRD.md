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

## Functional Requirements

| ID | Requirement | Acceptance Criteria | Priority |
| --- | --- | --- | --- |
| FR1 | Capsule-backed work registry | Create/read/update work items with stable IDs and lifecycle states. | P1 |
| FR2 | `codex-rs/SPEC.md` projection | Regenerate `codex-rs/SPEC.md` from capsule state with deterministic output. | P1 |
| FR3 | Maieutic PRD session | Produce a PRD file and register it in the capsule linked to its work item ID. | P1 |
| FR4 | Status surfaces | CLI + TUI + headless can list work items and show status consistently. | P1 |
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

- “What is planned/in progress?” is answerable from **one** canonical tracker (`codex-rs/SPEC.md`) generated from capsule state.
- PRDs and deprecations are linked to tracker IDs and have explicit lifecycle state.
- CLI/TUI/headless show consistent status for Tier‑1 PM flows.

---

## Open Questions

- Exact schema for work items (feature vs spec vs task) and which fields must be immutable.
- Whether `docs/DEPRECATIONS.md` becomes a projection of capsule events in the first iteration or later.
- How to represent “archived packs” as first-class capsule artifacts (URI scheme, metadata).
