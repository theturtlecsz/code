# SPEC-KIT-975 — Replayable Audits v1 (Deterministic)
**Date:** 2026-01-18
**Status:** COMPLETE
**Owner (role):** Platform+Eval Eng

## Summary
Record the agent’s operations (retrieval, tool calls, decisions) so we can replay a run against the same capsule checkpoint and produce deterministic audit reports. Optional A/B model replay comes later.

## Decision IDs implemented

**Implemented by this spec:** D33, D65, D66, D72, D76, D95

**Referenced (must remain consistent):** D51, D59

**Explicitly out of scope:** D60

---

## Goals
- Deliver the listed deliverables with tests and safe rollout.
- Keep Stage0 core abstracted (Memvid is an adapter).

## Non-Goals
- Hosted multi-tenant memory service.
- Removing the local-memory backend immediately (this is phased; see SPEC-KIT-979).

## Deliverables
- **Run Event Schema v1** (versioned JSON):
  - `StageTransition`, `PolicySnapshotRef`, `RetrievalRequest`, `RetrievalResponse`, `ToolCall`, `ToolResult`, `PatchApply`, `GateDecision`, `ErrorEvent`, `BranchMerged`, `CapsuleExported`, `CapsuleImported`.
  - Optional: `ModelCallEnvelope` with capture modes (see below).
- **Capture pipeline**:
  - Instrument Spec‑Kit + Stage0 adapters to append `RunEvent`s into the workspace capsule (`track=events`) with canonical `mv2://.../event/<RUN_ID>/<SEQ>` URIs.
  - Commit events at stage boundaries (and on explicit `commit now`).
- **Capture modes for model I/O** (aligns with D15 and Model Policy v2):
  - `[capture] mode = none|prompts_only|full_io` (default: `prompts_only`).
  - `prompts_only` stores prompt content + response hash (safe for export).
  - `full_io` stores full prompt + response (NOT safe for export, may contain sensitive data).
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

## Replay Truth Table (read this before implementing)

We use the term **“exact replay”** in a precise way:

> **Exact = retrieval + events.**  
> **Model I/O depends on capture mode.**  
> Offline replay **never** reissues remote model calls.

| Step type | Offline "exact"? | Condition |
|----------:|:------------------|:----------|
| Retrieval requests/responses | ✅ Yes | Same capsule checkpoint + same retrieval config → same hit set (within epsilon) |
| Event timeline | ✅ Yes | Events are captured at commit time and replayed verbatim |
| Tool outputs | ✅ Yes | Only if the tool output was captured (ToolResult events) |
| Model prompts | ⚠️ Depends | Only if `mode != none` (captured in `prompts_only` and `full_io`) |
| Model responses | ⚠️ Depends | Only if `mode = full_io` |
| Remote model calls | ❌ Never | Offline mode never reissues network calls |

---

## Acceptance Criteria (testable)
- Given a completed run with `[capture] mode = prompts_only`, the capsule contains:
  - an ordered `events` timeline for the run,
  - `PolicySnapshotRef` and model identifiers used,
  - retrieval requests/responses with explainability fields.
- `speckit replay <RUN_ID> --as-of <CHECKPOINT>` works fully offline and:
  - reproduces identical retrieval payloads (URIs + fused scores within epsilon),
  - reproduces identical decisions for deterministic steps.
- Replay output is a single report artifact (markdown + JSON) referencing capsule URIs.
- If `mode != full_io`, replay UI clearly indicates which model steps cannot be reconstructed exactly.

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

---

## Implementation Summary (2026-01-18)

### CLI Commands

**Capsule Events** (`speckit capsule events`):
- `--type <TYPE>` - Filter by event type
- `--branch <BRANCH>` - Filter by branch
- `--since-checkpoint <ID>` - Events after checkpoint
- `--audit-only` - Audit-critical events only
- `--curated-only` - Curated-eligible events only

**Replay Run** (`speckit replay run`):
- `--run <RUN_ID>` (required)
- `--branch <BRANCH>` (default: `run/<RUN_ID>`)
- `--types <TYPES>` (comma-separated filter)
- `--json` (machine-readable output)
- `--capsule <PATH>` (override capsule path)

**Replay Verify** (`speckit replay verify`):
- `--run <RUN_ID>` (required)
- `--check-retrievals` (validate URI resolution)
- `--check-sequence` (validate monotonic sequence)
- `--json` (machine-readable output)
- `--capsule <PATH>` (override capsule path)

### Tests

Location: `codex-rs/tui/src/memvid_adapter/tests.rs`

- `test_replay_timeline_deterministic()` (lines 3555-3704): Verifies event emission order preserved across capsule reopen
- `test_replay_offline_retrieval_exact()` (lines 3707-3844): Verifies retrieval results captured with exact precision
