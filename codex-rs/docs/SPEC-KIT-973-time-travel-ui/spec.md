# SPEC-KIT-973 — Time‑Travel UX (Timeline / As‑Of / Diff)
**Date:** 2026-01-10  
**Status:** DRAFT  
**Owner (role):** TUI Eng

## Summary
Turn capsule history into a product feature: timeline view, as-of queries, and diffs between checkpoints so auditors/devs can reproduce “what the agent knew.”

## Decision IDs implemented

**Implemented by this spec:** D3, D18, D32, D61, D73, D74, D96, D66

**Referenced (must remain consistent):** D71, D79

**Explicitly out of scope:** D39

---

## Goals
- Deliver the listed deliverables with tests and safe rollout.
- Keep Stage0 core abstracted (Memvid remains behind the memory adapter; TUI consumes only the trait).

## Non-Goals
- Hosted multi-tenant memory service.
- Removing the local-memory backend immediately (this is phased; see SPEC-KIT-979).

## Deliverables
- TUI commands: `/speckit.timeline`, `/speckit.asof <checkpoint>`, `/speckit.diff <a> <b>`.
- Branch UX:
  - `/speckit.branch <checkpoint> --name <branch>` creates a new writable branch from a checkpoint.
  - `/speckit.branches` lists branches (main + run branches) with size, last checkpoint, and "merged?" status.
  - `/speckit.branch switch <branch>`
  - `/speckit.branch merge <branch> [--merge curated|full]` (writes a merge checkpoint on main).
- Run branch lifecycle (default, per D73/D74):
  - When a Spec‑Kit run starts, create/select a capsule branch `run/<RUN_ID>` from `main` at the run‑start checkpoint.
  - All artifacts/events/cards written during the run MUST be tagged with `branch_id=run/<RUN_ID>` (or use native Memvid branches).
  - "Main" retrieval excludes non‑merged run branches by default; run‑scoped retrieval uses the active run branch.
  - On successful Unlock, auto‑merge the run branch into `main` using curated semantics and record a `BranchMerged` event referencing the run_id.
- Branch implementation:
  - Prefer native Memvid branching/merging if available.
  - Otherwise implement **app‑level branching**:
    - Branch = `branch_id` metadata + default filters.
    - Merge = "promote" artifacts into main by re‑ingesting with `branch_id=main` (dedup ensures minimal storage bloat).
- Checkpoint identity: stage boundary checkpoint IDs (primary) + timestamps (secondary) + optional named checkpoints (manual commits).
- As‑of retrieval: query capsule "view" at a checkpoint (and optionally branch_id).
- Diff: summarize changed artifacts, added/removed items, and changed extracted facts (if available).

## Merge semantics (normative)

This section exists because Git merge terminology caused repeated confusion.
We use **product semantics**, not Git semantics.

### Merge modes

| Mode | What is promoted to `main` | What stays run-isolated | Primary use |
|------|-----------------------------|--------------------------|-------------|
| `curated` (default) | **Curated** artifacts + **graph deltas** needed for long-term state (Cards/Edges) + **summary** audit events | High-volume debug/telemetry (full retrieval traces, verbose tool logs, raw model I/O unless explicitly configured) | Default “keep main clean” posture |
| `full` | All artifacts + all events + all Cards/Edges | Nothing (run becomes fully visible in main) | Deep audit / incident review / research |

Concrete rules (v1):
- **Artifacts:** promote all artifacts tagged `promote_to_main=true` (and any required dependencies they reference).
  - Default tagging: stage deliverables for `plan`, `tasks`, `validate`, `audit`, `unlock` are `promote_to_main=true`.
  - Debug artifacts default to `promote_to_main=false` unless explicitly opted in.
- **Events:**  
  - `curated`: promote *summary events* only: `StageTransition`, `PolicySnapshotRef`, `GateDecision`, `ErrorEvent`, `BranchMerged`, `CapsuleExported`, `CapsuleImported`.  
  - `full`: promote the complete event timeline including `RetrievalRequest/Response` (and optional `ModelCallEnvelope` if capture mode is `full`).
- **Cards/Edges:**  
  - `curated`: promote extracted Cards/Edges that represent the new “current state” (delta since branch base).  
  - `full`: promote all Cards/Edges produced during the run.
- **URIs:** logical `mv2://…` URIs are immutable (see SPEC‑KIT‑971).  
  - If merge implementation requires “promotion writes” (append-only), those writes **must preserve the same logical URI** and supersede prior branch-scoped visibility via metadata.

### How merge is recorded

- Merge emits `BranchMerged` event with:
  - `run_id`, `from_branch`, `to_branch=main`, `merge_mode`, `checkpoint_base`, `checkpoint_head`
  - lists of promoted URIs (or a hash of the list if too large)

---

## Acceptance Criteria (testable)
- `/speckit.timeline` shows ordered checkpoints and stage transitions for a run, including: stage, timestamp, run_id/spec_id, policy snapshot id, active branch, and (if available) model versions.
- As‑of queries:
  - `/speckit.asof <checkpoint>` constrains retrieval to that checkpoint (no leakage from later).
  - `/speckit.asof <checkpoint> --branch <branch>` works for non‑main branches.
- Diff:
  - Given two checkpoints, `/speckit.diff <a> <b>` lists added/changed/removed artifacts deterministically.
  - Diff includes structured deltas for Cards/Edges when present (SPEC‑KIT‑976).
- Branching semantics:
  - Creating a branch from checkpoint `X` and writing new artifacts does **not** mutate `main`.
  - Default run lifecycle creates `run/<RUN_ID>` branch and writes all run artifacts/events to it.
  - Before merge, main-branch searches MUST NOT return artifacts from `run/<RUN_ID>` unless explicitly requested.
  - After merge, main search DOES return the promoted artifacts, and the merge is recorded as an event with run_id.

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
