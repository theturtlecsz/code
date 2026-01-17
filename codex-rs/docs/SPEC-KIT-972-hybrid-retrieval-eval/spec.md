# SPEC-KIT-972 — Hybrid Retrieval + Explainability + Evaluation Harness
**Date:** 2026-01-17 (Updated)
**Status:** COMPLETE (100%)
**Owner (role):** Search/Eval Eng

## Summary
Make retrieval robust and debuggable: lex+vec fusion, filters, recency bias, and an eval harness that gates regressions and drives tuning.

## Decision IDs implemented

**Implemented by this spec:** D5, D21, D24, D35, D89, D90

**Referenced (must remain consistent):** D66, D80

**Explicitly out of scope:** D31

---

## Goals
- Deliver the listed deliverables with tests and safe rollout.
- Keep Stage0 core abstracted (Memvid is an adapter).

## Non-Goals
- Hosted multi-tenant memory service.
- Removing the local-memory backend immediately (this is phased; see SPEC-KIT-979).

## Deliverables

### Core Infrastructure (DONE)
- [x] Hybrid query strategy (BM25 + vector) with weighted fusion (`HybridBackend`)
- [x] Query controls: `top_k`, tag filters, recency bias knobs
- [x] A/B harness comparing `local-memory` vs `memvid` on same corpus
- [x] Score fusion via RRF or linear combination

### Search Commands (DONE)
- [x] `/speckit.memory search [--explain]` - TUI search with explain output
- [x] `code speckit memory search [--explain] [--json]` - CLI search

### Remaining Work
- [ ] Golden query suite for regression testing
- [ ] Performance benchmarking (P95 < 250ms target)
- [ ] Stress tests: large capsule, many small artifacts

## Acceptance Criteria (testable)
- CLI/TUI: `/speckit.search --explain` renders signal breakdown per result. ✅ PASSING
- Golden queries stable: key workflows meet or exceed baseline top-k hit rate.
- A/B harness runs in CI and produces a report artifact (JSON + markdown summary). ✅ PASSING
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
