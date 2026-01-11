# SPEC-KIT-979 — Migration: local-memory → Memvid + Sunset
**Date:** 2026-01-10  
**Status:** DRAFT  
**Owner (role):** Platform Eng

## Summary
Migrate fully off the local-memory daemon after parity gates: import corpus, dual-run A/B, then deprecate and remove local-memory dependencies.

## Goals
- Deliver the listed deliverables with tests and safe rollout.
- Keep Stage0 core abstracted (Memvid is an adapter).

## Non-Goals
- Hosted multi-tenant memory service.
- Removing the local-memory backend immediately (this is phased; see SPEC-KIT-979).

## Deliverables
- Import/migration tool: ingest existing local-memory corpus into workspace capsule.
- Dual-backend runtime flag; ability to A/B compare retrieval results in same run.
- Parity gates: golden queries + workflow pass rates must meet thresholds.
- Deprecation plan: mark local-memory docs as legacy; remove daemon requirement when safe.

## Acceptance Criteria (testable)
- Parity gate passes for agreed workloads; failures have tracked remediation issues.
- Local-memory backend can be removed behind a feature flag (rollback available).
- Docs updated: memvid-first is default; local-memory described as legacy fallback.

## Dependencies
- Memvid crate(s) pinned behind adapter boundary.
- Decision Register: `docs/DECISION_REGISTER.md`
- Architecture: `docs/MEMVID_FIRST_WORKBENCH.md`

## Rollout / Rollback
- Roll out behind config flags with dual-backend fallback.
- Roll back by switching `memory_backend` back to `local-memory` and disabling Memvid features.

## Risks & Mitigations
- **Memvid API churn** → pin versions; wrap behind traits; contract tests.
- **Single-file contention** → single-writer + lock + writer queue.
- **Retrieval regressions** → eval harness + A/B parity gates.
- **Data leakage** → safe export redaction + optional sanitize-on-ingest mode.
