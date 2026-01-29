# SPEC-KIT-976 — Logic Mesh / Graph v1 (Memvid Memory Cards)

**Date:** 2026-01-10\
**Status:** COMPLETED\
**Completed:** 2026-01-19\
**Owner (role):** Platform Data Eng + Librarian/Knowledge Eng

> **Implementation status:** Canonical completion tracker: `codex-rs/SPEC.md` Completed (Recent).

## Summary

Promote Spec‑Kit from “searchable blobs” to a **queryable project knowledge graph** stored inside the Memvid capsule. We will ingest **Memory Cards** (normalized entities + facts) and **Logic Mesh edges** (relationships) with full provenance + time‑travel.

This is the foundation for:

* `/speckit.state` (O(1) current state views)
* “what changed” fact history per entity
* relationship-aware retrieval (graph‑expanded recall)
* replayable audits that can explain *why* a decision was made

## Decision IDs implemented

**Implemented by this spec:** D31, D10, D62, D63, D64, D75, D97

**Referenced (must remain consistent):** D58, D77

**Explicitly out of scope:** D60

***

## Goals

* A **v1 schema** for Cards + Edges that is stable, versioned, and time‑travelable.
* A minimal extraction/enrichment pipeline that produces useful Cards/Edges for the 2026-Q1 specs (971–980).
* TUI/CLI query surfaces that make the graph **usable** (not just stored).
* Keep Stage0 abstracted via traits; Memvid remains a backend adapter.

## Non‑Goals

* A full “enterprise ontology” or auto‑reasoning system.
* Hosted graph service / multi-tenant graph.
* Perfect extraction quality on day one (we ship incremental coverage + confidence).

## Deliverables

### 1) Schema v1: Memory Cards

Define `MemoryCardV1` as a versioned JSON document stored in the capsule (track: `cards`).

**Core fields (required):**

* `card_id`: stable ID (UUID or deterministic hash)
* `card_type`: one of `spec|decision|task|risk|component|person|artifact|run` (extensible)
* `title`: short human label
* `facts`: list of `{ key, value, value_type, confidence, source_uris[] }`
* `provenance`: `{ created_at, created_by, spec_id?, run_id?, stage?, commit_hash? }`
* `version`: `1`

**Card lifecycle rule:** append-only updates. “Edits” create a new card frame that supersedes prior facts (by `card_id` + `created_at`).

### 2) Schema v1: Logic Mesh Edges

Define `LogicEdgeV1` as a versioned JSON document stored in the capsule (track: `edges`).

**Core fields (required):**

* `edge_id`: stable ID
* `edge_type`: `depends_on|blocks|implements|references|owns|risks|related_to` (extensible)
* `from_uri`: canonical `mv2://...` URI or `card:<card_id>`
* `to_uri`: canonical `mv2://...` URI or `card:<card_id>`
* `weight`: optional numeric weight/confidence
* `provenance`: `{ created_at, created_by, spec_id?, run_id?, stage? , source_uris[] }`
* `version`: `1`

### 3) Extraction/Enrichment Pipeline (v1)

* Implement a `GraphExtractor` service that:
  * reads capsule artifacts for a given spec/run/stage window
  * produces Cards + Edges (JSON) with provenance
  * writes them back to the capsule + commits a checkpoint
* **Default model choice**: use SidecarCritic (cheap cloud) for extraction; fallback to Architect cloud if confidence is low.
  * *(Assumption to validate)*: SidecarCritic quality is sufficient for task/decision/risk extraction.

### 4) Query APIs + UX

Expose graph queries to users:

* `/speckit.state --spec <ID>`: show current tasks/decisions/risks summary (from Cards)
* `/speckit.facts <card_id>`: show fact history across checkpoints
* `/speckit.graph --from <card_id> --depth 2`: show outgoing edges
* CLI equivalents under `speckit graph ...`

### 5) Graph‑Aware Retrieval Hooks

* Add optional retrieval expansion mode in Spec‑Kit:
  * when enabled, expand a query’s top hits with 1-hop graph neighbors (bounded by type + depth)
  * record expansion events into audit timeline (SPEC‑KIT‑975)

### 6) Librarian Rebase (from SPEC‑KIT‑103)

* Rebase “Librarian” repair/enrichment jobs to operate on `cards/edges` in the capsule:
  * dedup/merge cards
  * normalize task status fields
  * maintain “current state” indices

## Acceptance Criteria (testable)

* Given a spec run containing a plan + tasks artifact, the extractor generates:
  * at least one `spec` card and ≥5 `task` cards
  * edges linking tasks to the spec (`implements` or `references`)
* `/speckit.state --spec <ID>` returns a stable view of tasks/decisions/risks.
* “As‑of checkpoint” graph queries produce deterministic results:
  * same card/edge IDs and values for the same checkpoint.
* Graph extraction + queries are fully functional offline **as long as** the cards already exist in the capsule.

## Dependencies

* SPEC‑KIT‑971 (capsule foundation + checkpointing)
* SPEC‑KIT‑972 (retrieval + explainability; graph expansion hooks)
* SPEC‑KIT‑975 (event log schema for recording extraction + query events)
* Decision Register: `docs/DECISIONS.md`

## Rollout / Rollback

* Roll out behind `graph.enabled=true`.
* If extraction quality or performance is problematic:
  * disable extraction, keep storage schema,
  * fall back to blob retrieval only.

## Risks & Mitigations

* **Extraction quality variance** → confidence scores + allow manual edits; incremental coverage (tasks first).
* **Graph explosion / bloat** → bounded types, depth limits, retention rules, compaction.
* **Inconsistent IDs** → deterministic ID rules + contract tests + schema versioning.
* **Policy/privacy** → safe-export redaction applies to cards/edges; do not store secrets as facts by default.
