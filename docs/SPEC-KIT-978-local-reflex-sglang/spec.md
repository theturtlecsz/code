# SPEC-KIT-978 — Implementer.Reflex Mode via SGLang (RTX 5090) + Bakeoff
**Date:** 2026-01-10  
**Status:** DRAFT  
**Owner (role):** Infra/LLM Eng

## Summary

**Important:** "Reflex" is **not** a new Stage0 role. Treat it as `role=Implementer` + `mode=reflex` (for telemetry + routing), used only inside the Implement stage.
Wire a local OpenAI-compatible inference server for sub-second compiler loops using SGLang with RadixAttention and JSON schema enforcement. Establish bakeoff gates for reflex promotion.

## Goals
- Deliver the listed deliverables with tests and safe rollout.
- Keep Stage0 core abstracted (local inference is an adapter behind an OpenAI-compatible client).

## Non-Goals
- Hosted multi-tenant memory service.
- Removing the local-memory backend immediately (this is phased; see SPEC-KIT-979).

## Deliverables
- SGLang server runbook for RTX 5090; default model `gpt-oss-20b` (MXFP4) + FP8 KV cache.
- Standardize reflex **backup** model(s): `qwen3-coder-30b-a3b` (primary backup) and optionally `qwen2.5-coder-32b` (dense last resort).
- Config: allow `reflex.model_primary` + ordered `reflex.model_fallbacks[]` with bakeoff gating before promotion.
- Client wiring: OpenAI-compatible endpoint selection via config.
- Reflex loop: compile → parse errors → patch JSON → apply patch; with retry + escalation.
- Bakeoff harness: TTFT, tokens/sec, pass@1 on Rust error sets; cache-hit verification.

## Acceptance Criteria (testable)
- Reflex patch loop produces valid JSON args (schema enforced) with no markdown preamble.
- TTFT and throughput meet targets on 5090 (targets defined in harness doc).
- Fallback to cloud Implementer triggers after N failures and is logged.

## Dependencies
- Memvid crate(s) pinned behind adapter boundary.
- Decision Register: `docs/DECISION_REGISTER.md`
- Architecture: `docs/MEMVID_FIRST_WORKBENCH.md`

## Rollout / Rollback
- Roll out behind config flags with dual-backend fallback.
- Roll back by switching `memory_backend` back to `local-memory` and disabling Memvid features.

## Risks & Mitigations
- **Reflex quality variance** (repo-specific Rust crates) → bakeoff on *your* error corpus; keep Qwen3-A3B as gated fallback; escalate to cloud after N fails.
- **Memvid API churn** → pin versions; wrap behind traits; contract tests.
- **Single-file contention** → single-writer + lock + writer queue.
- **Retrieval regressions** → eval harness + A/B parity gates.
- **Data leakage** → safe export redaction + optional sanitize-on-ingest mode.
