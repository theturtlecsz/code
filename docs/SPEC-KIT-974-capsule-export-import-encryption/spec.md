# SPEC-KIT-974 — Capsule Export/Import + Encryption + Safe Export
**Date:** 2026-01-10  
**Status:** DRAFT  
**Owner (role):** Platform+Security Eng

## Summary
Make capsules shareable and enterprise-safe: encrypted exports, reproducible imports, and safe-export redaction + audit logging.

## Goals
- Deliver the listed deliverables with tests and safe rollout.
- Keep Stage0 core abstracted (Memvid is an adapter).

## Non-Goals
- Hosted multi-tenant memory service.
- Removing the local-memory backend immediately (this is phased; see SPEC-KIT-979).

## Deliverables
- Export: `speckit capsule export --run <RUN_ID> --out <PATH> [--encrypt]` producing `.mv2e` by default for run exports.
- Import: `speckit capsule import <PATH>` to reproduce retrieval context offline.
- Password/key UX: env var + prompt; optional OS keychain integration later.
- Safe export pipeline: redact secrets/PII in rendered views and export bundles; log export actions into evidence trail.

## Acceptance Criteria (testable)
- Import on a second machine reproduces same retrieval results for checkpointed queries (deterministic context).
- Encrypted capsule requires password; wrong password fails safely.
- Every export writes an audit event (who/when/what) into workspace capsule evidence.

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
