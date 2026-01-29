# SPEC-KIT-978 — Implementer.Reflex Mode via SGLang (RTX 5090) + Bakeoff

**Date:** 2026-01-18
**Status:** COMPLETED
**Completed:** 2026-01-18
**Owner (role):** Infra/LLM Eng

> **Implementation status:** Canonical completion tracker: `codex-rs/SPEC.md` Completed (Recent).

## Summary

**Important:** "Reflex" is **not** a new Stage0 role. Treat it as `role=Implementer` + `mode=reflex` (for telemetry + routing), used only inside the Implement stage.
Wire a local OpenAI-compatible inference server for sub-second compiler loops using SGLang with RadixAttention and JSON schema enforcement. Establish bakeoff gates for reflex promotion.

## Decision IDs implemented

**Implemented by this spec:** D13, D25, D43, D55, D78, D93, D110, D112

**Referenced (must remain consistent):** D49, D50

**Explicitly out of scope:** D27

***

## Goals

* Deliver the listed deliverables with tests and safe rollout.
* Keep Stage0 core abstracted (local inference is an adapter behind an OpenAI-compatible client).

## Non-Goals

* Hosted multi-tenant memory service.
* Removing the local-memory backend immediately (this is phased; see SPEC-KIT-979).

## Deliverables

* SGLang server runbook for RTX 5090; default model `gpt-oss-20b` (MXFP4) + FP8 KV cache.
* Standardize reflex **backup** model(s): `qwen3-coder-30b-a3b` (primary backup) and optionally `qwen2.5-coder-32b` (dense last resort).
* Config: allow `reflex.model_primary` + ordered `reflex.model_fallbacks[]` with bakeoff gating before promotion.
* Client wiring: OpenAI-compatible endpoint selection via config.
* Reflex loop: compile → parse errors → patch JSON → apply patch; with retry + escalation.
* Bakeoff harness: TTFT, tokens/sec, pass\@1 on Rust error sets; cache-hit verification.

## Acceptance Criteria (testable)

* Reflex patch loop produces valid JSON args (schema enforced) with no markdown preamble.
* TTFT and throughput meet targets on 5090 (targets defined in harness doc).
* Fallback to cloud Implementer triggers after N failures and is logged.

## Dependencies

* Memvid crate(s) pinned behind adapter boundary.
* Decision Register: `docs/DECISIONS.md`
* Architecture: `docs/ARCHITECTURE.md`

## Rollout / Rollback

* Roll out behind config flags with dual-backend fallback.
* Roll back by switching `memory_backend` back to `local-memory` and disabling Memvid features.

## Risks & Mitigations

* **Reflex quality variance** (repo-specific Rust crates) → bakeoff on *your* error corpus; keep Qwen3-A3B as gated fallback; escalate to cloud after N fails.
* **Memvid API churn** → pin versions; wrap behind traits; contract tests.
* **Single-file contention** → single-writer + lock + writer queue.
* **Retrieval regressions** → eval harness + A/B parity gates.
* **Data leakage** → safe export redaction + optional sanitize-on-ingest mode.

***

## Implementation Summary (2026-01-18)

### TUI Commands (`/speckit.reflex <cmd>`)

| Command              | Description                                |
| -------------------- | ------------------------------------------ |
| `health`             | Check reflex server health                 |
| `status`             | Display reflex configuration               |
| `models`             | List available models                      |
| `bakeoff [duration]` | Show reflex vs cloud metrics (default 24h) |
| `check [duration]`   | Validate bakeoff thresholds                |
| `e2e [--stub]`       | Run E2E routing tests                      |

### Headless CLI (`code reflex <cmd>`)

| Command  | Args                                              | Exit Codes                             |
| -------- | ------------------------------------------------- | -------------------------------------- |
| `health` | `--json`, `--policy`, `--timeout`                 | 0=Healthy, 1=Unhealthy, 2=Config Error |
| `models` | `--json`, `--policy`                              | 0=Success, 1=Failed, 2=Config Error    |
| `status` | `--json`, `--policy`                              | 0=Success, 2=Config Error              |
| `e2e`    | `--stub`, `--endpoint`, `--model`, `--json`, `-v` | 0=Pass, 1=Fail                         |

### Circuit Breaker Scope

**Implemented (this spec):**

* `BreakerState` enum: `Closed`, `Open`, `HalfOpen`
* `BreakerStateChangedPayload` with failure metrics + probe tracking
* `EventType::BreakerStateChanged` integrated with curated/audit-critical flags

**Follow-on (SPEC-945C deferred):**

* Runtime state machine transitions
* 50% failure threshold + 30s cooldown
* Half-open test probes
