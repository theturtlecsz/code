# SPEC-KIT-972 — Hybrid Retrieval + Explainability + Evaluation Harness
**Date:** 2026-01-10  
**Status:** DRAFT  
**Owner (role):** Search/Eval Eng

## Summary
Make retrieval robust and debuggable: lex+vec fusion, filters, recency bias, and an eval harness that gates regressions and drives tuning.

## Goals
- Deliver the listed deliverables with tests and safe rollout.
- Keep Stage0 core abstracted (Memvid is an adapter).

## Non-Goals
- Hosted multi-tenant memory service.
- Removing the local-memory backend immediately (this is phased; see SPEC-KIT-979).

## Deliverables
- Hybrid query strategy (BM25 + vector + optional graph/state signal) with weighted fusion.
- Query controls: `top_k`, tag filters, URI scope, recency bias knobs.
- Explain output (lex score, vec score, recency contribution, tag matches, final score).
- Golden query suite + A/B harness comparing `local-memory` vs `memvid` on the same corpus.
- Stress tests: large capsule, many small artifacts, frequent checkpoints.

## Acceptance Criteria (testable)
- CLI/TUI: `/speckit.search --explain` renders signal breakdown per result.
- Golden queries stable: key workflows meet or exceed baseline top-k hit rate.
- A/B harness runs in CI and produces a report artifact (JSON + markdown summary).
- Performance: retrieval P95 < 250ms on warm cache for typical queries (assumption to validate).

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
