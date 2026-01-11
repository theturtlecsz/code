# SPEC-KIT-975 — Replayable Audits v1 (Deterministic)
**Date:** 2026-01-10  
**Status:** DRAFT  
**Owner (role):** Platform+Eval Eng

## Summary
Record the agent’s operations (retrieval, tool calls, decisions) so we can replay a run against the same capsule checkpoint and produce deterministic audit reports. Optional A/B model replay comes later.

## Goals
- Deliver the listed deliverables with tests and safe rollout.
- Keep Stage0 core abstracted (Memvid is an adapter).

## Non-Goals
- Hosted multi-tenant memory service.
- Removing the local-memory backend immediately (this is phased; see SPEC-KIT-979).

## Deliverables
- **Run Event Schema v1** (versioned JSON):
  - `StageTransition`, `PolicySnapshotRef`, `RetrievalRequest`, `RetrievalResponse`, `ToolCall`, `ToolResult`, `PatchApply`, `GateDecision`, `ErrorEvent`.
  - Optional: `ModelCallEnvelope` with capture modes (see below).
- **Capture pipeline**:
  - Instrument Spec‑Kit + Stage0 adapters to append `RunEvent`s into the workspace capsule (`track=events`) with canonical `mv2://.../event/<RUN_ID>/<SEQ>` URIs.
  - Commit events at stage boundaries (and on explicit `commit now`).
- **Capture modes for model I/O** (aligns with D15):
  - `audit.capture_llm_io = off|hash|summary|full` (default: `summary` + `hash`).
  - `hash/summary` must be safe‑export compatible (no raw secrets; allow redaction hooks).
- **Replay engine v1 (offline-first)**:
  - “Exact replay” replays deterministic steps **without re‑calling models**: retrieval, tool execution re‑application, gate evaluation.
  - For each `RetrievalRequest`, re-run retrieval against the same capsule checkpoint and compare:
    - hit set (URIs),
    - fused scores (within epsilon),
    - explainability fields.
- **Replay CLI/TUI**:
  - `speckit replay <RUN_ID> --as-of <CHECKPOINT>`
  - `/speckit.replay <RUN_ID> --as-of <CHECKPOINT>`
  - Optional: `--compare-to <CHECKPOINT>` to diff two points in time.
- **Deterministic report generator**:
  - Emit `replay_report.md` + `replay_report.json` (single artifact) with links to capsule URIs.
  - Store the report back into the capsule under `mv2://.../artifact/...`.
- **Stretch (explicitly optional)**: A/B replay that re-runs selected model calls with a different model and produces diffs (gated; may require network).

## Acceptance Criteria (testable)
- Given a completed run with `audit.capture_llm_io=summary`, the capsule contains:
  - an ordered `events` timeline for the run,
  - `PolicySnapshotRef` and model identifiers used,
  - retrieval requests/responses with explainability fields.
- `speckit replay <RUN_ID> --as-of <CHECKPOINT>` works fully offline and:
  - reproduces identical retrieval payloads (URIs + fused scores within epsilon),
  - reproduces identical decisions for deterministic steps.
- Replay output is a single report artifact (markdown + JSON) referencing capsule URIs.
- If `audit.capture_llm_io != full`, replay UI clearly indicates which model steps cannot be reconstructed exactly.

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
