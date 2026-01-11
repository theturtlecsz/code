# SPEC-KIT-973 — Time‑Travel UX (Timeline / As‑Of / Diff)
**Date:** 2026-01-10  
**Status:** DRAFT  
**Owner (role):** TUI Eng

## Summary
Turn capsule history into a product feature: timeline view, as-of queries, and diffs between checkpoints so auditors/devs can reproduce “what the agent knew.”

## Goals
- Deliver the listed deliverables with tests and safe rollout.
- Keep Stage0 core abstracted (Memvid remains behind the memory adapter; TUI consumes only the trait).

## Non-Goals
- Hosted multi-tenant memory service.
- Removing the local-memory backend immediately (this is phased; see SPEC-KIT-979).

## Deliverables
- TUI commands: `/speckit.timeline`, `/speckit.asof <checkpoint>`, `/speckit.diff <a> <b>`.
- Branching: `/speckit.branch <checkpoint> --name <branch>` creates a new writable branch from a checkpoint.
- Branch implementation: **prefer native Memvid branching if available**; otherwise implement as `capsule copy + branch pointer metadata` (workspace keeps `active_branch`).
- Checkpoint identity: stage boundary checkpoint IDs (primary) + timestamps (secondary).
- As-of retrieval: query capsule “view” at a checkpoint.
- Diff: summarize changed artifacts, added/removed items, and changed extracted facts (if available).

## Acceptance Criteria (testable)
- Creating a branch from checkpoint `X` and writing new artifacts does **not** mutate the base timeline; switching branches changes retrieval scope.
- Given two checkpoints, diff lists added/changed/removed artifacts deterministically.
- As-of search returns results constrained to selected checkpoint (no leakage from later stages).
- Timeline includes: stage, timestamp, run_id/spec_id, policy snapshot id, model versions (if available).

## Dependencies
- Memvid crate(s) pinned behind adapter boundary.
- Decision Register: `docs/DECISION_REGISTER.md`
- Architecture: `docs/MEMVID_FIRST_WORKBENCH.md`

## Rollout / Rollback
- Roll out behind config flags with dual-backend fallback.
- Roll back by switching `memory_backend` back to `local-memory` and disabling Memvid features.

## Risks & Mitigations
- **Branch bloat / copy cost** → branch by reference if Memvid supports it; otherwise compress + warn on branch size; add `speckit branch gc` later.
- **Memvid API churn** → pin versions; wrap behind traits; contract tests.
- **Single-file contention** → single-writer + lock + writer queue.
- **Retrieval regressions** → eval harness + A/B parity gates.
- **Data leakage** → safe export redaction + optional sanitize-on-ingest mode.
