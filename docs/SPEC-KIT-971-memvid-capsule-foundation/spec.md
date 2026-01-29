# SPEC-KIT-971 — Memvid Capsule Foundation + Single-Writer Adapter

**Date:** 2026-01-10\
**Status:** COMPLETED\
**Completed:** 2026-01-17\
**Owner (role):** Platform Eng

> **Implementation status:** Canonical completion tracker: `codex-rs/SPEC.md` Completed (Recent).

## Summary

Introduce Memvid as an in-process backend behind Stage0 memory traits with a single-writer capsule coordinator, checkpoint commits aligned to Spec‑Kit stages, and crash-safe reopen/search.

## Decision IDs implemented

**Implemented by this spec:** D1, D2, D3, D4, D6, D7, D18, D20, D45, D53, D70

**Referenced (must remain consistent):** D8, D22, D52

**Explicitly out of scope:** D79

***

## Goals

* Deliver the listed deliverables with tests and safe rollout.
* Keep Stage0 core abstracted (Memvid is an adapter).

## Non-Goals

* Hosted multi-tenant memory service.
* Removing the local-memory backend immediately (this is phased; see SPEC-KIT-979).

## Deliverables

* New `MemvidMemoryAdapter` implementing existing Stage0 memory traits (search + ingest baseline).
* `speckit capsule doctor` command: verify capsule readability, lock health, and last-good checkpoint; emit actionable repair steps.
* `speckit capsule stats` command: size, frame counts, index status, and dedup ratio (if enabled).
* Enable and validate Memvid dedup tracks (BLAKE3 exact + SimHash near-dup) for evidence/artifact ingest; add contract tests.
* Capsule path conventions: `./.speckit/memvid/workspace.mv2` (+ optional `.mv2e`).
* Checkpoint API: stage boundary commit + manual commit; stored as metadata (checkpoint\_id, label, stage, spec\_id, run\_id, commit\_hash, timestamp).
* `speckit capsule commit --label <LABEL>` (and optional TUI `/speckit.commit <LABEL>`) creates a manual checkpoint that is visible in the checkpoint list and usable for time-travel/replay.
* Canonical URI scheme: every artifact/checkpoint/policy snapshot/event stored in the capsule must get a stable `mv2://...` URI (round-trippable, unique, and stable across reopen).
* Event track plumbing (foundation): create an `events` track in the capsule and a minimal `RunEventEnvelope` (v1) for append-only event capture. Emit at least `StageTransition` + `PolicySnapshotRef` events at stage commits (more event types land in SPEC-KIT-975).
* `speckit capsule checkpoints` command: list checkpoints (time, stage, spec\_id/run\_id) and their IDs for time-travel/replay workflows.
* Single-writer lock + write queue; safe reopen after crash.
* Config switch: `memory_backend = memvid | local-memory` with fallback when capsule corrupt/unreadable.

## URI invariants (normative)

URIs are the addressing primitive for:

* replay/events (`SPEC-KIT-975`)
* Cards/Edges graph endpoints (`SPEC-KIT-976`)
* export/import bundles (`SPEC-KIT-974`)

If URIs drift, **replay breaks** and **graph edges orphan**. This section is intentionally strict.

### Definitions

* **Logical URI**: the stable identifier we expose externally (examples start with `mv2://...`)
* **Physical frame address**: Memvid’s internal frame/fragment identity (may change with append-only writes)
* **Resolution**: mapping `(logical_uri, branch, as_of)` → the correct physical record/revision

### Invariants

1. **Logical URIs are immutable.** Once a URI is returned to the caller, it MUST remain valid across:
   * reopen,
   * time-travel queries,
   * branch merges/promotions,
   * export/import.
2. **Logical URIs are stable keys, not “frame IDs”.** A URI may have multiple revisions over time.
3. **All cross-object references use logical URIs.**
   * Events (`source_uris`) use logical URIs.
   * Graph edges (`from_uri`, `to_uri`) use logical URIs.
4. **Promotion/merge writes MUST preserve the same logical URI.**
   * Because the capsule is append-only, merges are modeled as “superseding” writes that update visibility/metadata.
   * Do not mint new URIs for the same conceptual object during merge.
5. **Alias map is the emergency escape hatch only.**
   * If we ever must change a URI due to a bug, we record `old_uri → new_uri` in a dedicated `uri_aliases` track.
   * Resolution follows aliases transitively and reports alias usage in `--explain` mode.

### Implementation posture (how to build this)

* `CapsuleStore::put(...)` **generates** the logical URI (do not rely on Memvid internal IDs).
* Store the logical URI in object metadata as `uri=<mv2://...>`.
* Maintain a `uri_index` track that maps `uri → latest_physical_pointer` per `(branch_id, checkpoint)`.
  * Update this index at commit barriers (stage boundary commits + manual commits).
* Provide `resolve_uri(uri, branch, as_of)` API and use it for:
  * replay,
  * graph traversal,
  * export materialization.

***

## Acceptance Criteria (testable)

* End-to-end test: create capsule → ingest artifact → commit checkpoint → reopen → search returns artifact.
* `speckit capsule doctor` detects: missing capsule, locked capsule, corrupted footer, and version mismatch; returns non-zero exit on failure.
* Crash recovery test: simulate crash mid-write; capsule reopens; last committed checkpoint is readable.
* Local-memory fallback test: if capsule missing/corrupt, system falls back and records evidence.
* All Memvid types stay behind adapter boundary (no Memvid dependency in Stage0 core crates).
* `speckit capsule checkpoints` returns a non-empty list after at least one stage commit, and includes both stage checkpoints and manual commits.
* Every `put` returns a `mv2://` URI; URIs remain stable after reopen and are unique per stored object.
* At least one `StageTransition` event is appended on stage commit; it has a stable `mv2://...` URI and can be retrieved after reopen.

## Dependencies

* Memvid crate(s) pinned behind adapter boundary.
* Decision Register: `docs/DECISIONS.md`
* Architecture: `docs/ARCHITECTURE.md`

## Rollout / Rollback

* Roll out behind config flags with dual-backend fallback.
* Roll back by switching `memory_backend` back to `local-memory` and disabling Memvid features.

## Risks & Mitigations

* **Single-file corruption / partial writes** → enforce single-writer + lockfile; commit barriers at stage boundaries; `capsule doctor` + backups + contract tests.
* **Memvid API churn** → pin versions; wrap behind traits; contract tests.
* **Single-file contention** → single-writer + lock + writer queue.
* **Retrieval regressions** → eval harness + A/B parity gates.
* **Data leakage** → safe export redaction + optional sanitize-on-ingest mode.
