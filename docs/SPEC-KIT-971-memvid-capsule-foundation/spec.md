# SPEC-KIT-971 — Memvid Capsule Foundation + Single-Writer Adapter
**Date:** 2026-01-10  
**Status:** DRAFT  
**Owner (role):** Platform Eng

## Summary
Introduce Memvid as an in-process backend behind Stage0 memory traits with a single-writer capsule coordinator, checkpoint commits aligned to Spec‑Kit stages, and crash-safe reopen/search.

## Goals
- Deliver the listed deliverables with tests and safe rollout.
- Keep Stage0 core abstracted (Memvid is an adapter).

## Non-Goals
- Hosted multi-tenant memory service.
- Removing the local-memory backend immediately (this is phased; see SPEC-KIT-979).

## Deliverables
- New `MemvidMemoryAdapter` implementing existing Stage0 memory traits (search + ingest baseline).
- `speckit capsule doctor` command: verify capsule readability, lock health, and last-good checkpoint; emit actionable repair steps.
- `speckit capsule stats` command: size, frame counts, index status, and dedup ratio (if enabled).
- Enable and validate Memvid dedup tracks (BLAKE3 exact + SimHash near-dup) for evidence/artifact ingest; add contract tests.
- Capsule path conventions: `./.speckit/memvid/workspace.mv2` (+ optional `.mv2e`).
- Checkpoint API: stage boundary commit + manual commit; stored as metadata.
- Canonical URI scheme: every artifact/checkpoint/policy snapshot/event stored in the capsule must get a stable `mv2://...` URI (round-trippable, unique, and stable across reopen).
- Event track plumbing (foundation): create an `events` track in the capsule and a minimal `RunEventEnvelope` (v1) for append-only event capture. Emit at least `StageTransition` + `PolicySnapshotRef` events at stage commits (more event types land in SPEC-KIT-975).
- `speckit capsule checkpoints` command: list checkpoints (time, stage, spec_id/run_id) and their IDs for time-travel/replay workflows.
- Single-writer lock + write queue; safe reopen after crash.
- Config switch: `memory_backend = memvid | local-memory` with fallback when capsule corrupt/unreadable.

## Acceptance Criteria (testable)
- End-to-end test: create capsule → ingest artifact → commit checkpoint → reopen → search returns artifact.
- `speckit capsule doctor` detects: missing capsule, locked capsule, corrupted footer, and version mismatch; returns non-zero exit on failure.
- Crash recovery test: simulate crash mid-write; capsule reopens; last committed checkpoint is readable.
- Local-memory fallback test: if capsule missing/corrupt, system falls back and records evidence.
- All Memvid types stay behind adapter boundary (no Memvid dependency in Stage0 core crates).
- `speckit capsule checkpoints` returns a non-empty list after at least one stage commit, and includes both stage checkpoints and manual commits.
- Every `put` returns a `mv2://` URI; URIs remain stable after reopen and are unique per stored object.
- At least one `StageTransition` event is appended on stage commit; it has a stable `mv2://...` URI and can be retrieved after reopen.

## Dependencies
- Memvid crate(s) pinned behind adapter boundary.
- Decision Register: `docs/DECISION_REGISTER.md`
- Architecture: `docs/MEMVID_FIRST_WORKBENCH.md`

## Rollout / Rollback
- Roll out behind config flags with dual-backend fallback.
- Roll back by switching `memory_backend` back to `local-memory` and disabling Memvid features.

## Risks & Mitigations
- **Single-file corruption / partial writes** → enforce single-writer + lockfile; commit barriers at stage boundaries; `capsule doctor` + backups + contract tests.
- **Memvid API churn** → pin versions; wrap behind traits; contract tests.
- **Single-file contention** → single-writer + lock + writer queue.
- **Retrieval regressions** → eval harness + A/B parity gates.
- **Data leakage** → safe export redaction + optional sanitize-on-ingest mode.
