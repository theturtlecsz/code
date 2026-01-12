# SPEC-KIT-977 — Model Policy v2 (Lifecycle + Enforcement)
**Date:** 2026-01-10  
**Status:** DRAFT  
**Owner (role):** Platform+Security Eng

## Summary
Turn model policy into an executable system: authored in repo, validated in CI, snapshotted into capsules, enforced at routing and gates, monitored and auditable.

## Decision IDs implemented

**Implemented by this spec:** D12, D17, D36, D56, D57, D44

**Referenced (must remain consistent):** D30, D59

**Explicitly out of scope:** D60

---

## Goals
- Deliver the listed deliverables with tests and safe rollout.
- Keep Stage0 core abstracted (Memvid is an adapter).

## Non-Goals
- Hosted multi-tenant memory service.
- Removing the local-memory backend immediately (this is phased; see SPEC-KIT-979).

## Deliverables
- Policy authoring format (TOML/YAML) + schema validation.
- Policy compiler emits `PolicySnapshot.json` stored into capsule per run/checkpoint.
- Router enforcement: role→model mapping, escalation rules, provider fallbacks.
- Policy tests: golden scenarios; regression suite; change log and versioning.
- Governance: warn-only signed approvals initially; hard enforcement later.

## Acceptance Criteria (testable)
- Any run contains a PolicySnapshot referenced by ID in the capsule timeline.
- Policy test suite blocks merge on invalid/untested changes.
- Router logs “why this model” as structured evidence for each call.

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
