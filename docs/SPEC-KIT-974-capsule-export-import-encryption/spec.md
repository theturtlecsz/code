# SPEC-KIT-974 — Capsule Export/Import + Encryption + Safe Export

**Date:** 2026-01-10\
**Status:** DRAFT\
**Owner (role):** Platform+Security Eng

## Summary

Make capsules shareable and enterprise-safe: encrypted exports, reproducible imports, and safe-export redaction + audit logging.

## Decision IDs implemented

**Implemented by this spec:** D2, D8, D9, D16, D23, D46, D54, D70, D71

**Referenced (must remain consistent):** D79

**Explicitly out of scope:** D60

***

## Goals

* Deliver the listed deliverables with tests and safe rollout.
* Keep Stage0 core abstracted (Memvid is an adapter).

## Non-Goals

* Hosted multi-tenant memory service.
* Removing the local-memory backend immediately (this is phased; see SPEC-KIT-979).

## Deliverables

* Export command:
  * `speckit capsule export --run <RUN_ID> --out <PATH> [--encrypt|--no-encrypt] [--safe|--unsafe]`
  * Default: `--encrypt --safe` for per-run exports (`.mv2e`).
* Import command:
  * `speckit capsule import <PATH> [--mount-as <NAME>]`
  * Imports are read-only mounts by default (no mutation of imported capsule).
* Export triggers (D16):
  * Config: `capsule.export.mode = manual | risk | always` (default: `risk`).
  * `risk` means auto-export is performed only when: (a) Spec classification is high-risk, (b) Judge requests export, or (c) an Unlock gate requires an audit handoff.
* Retention/GC (D20, D116):
  * Config: `capsule.export.retention_days = 30` (default) + `capsule.export.keep_pinned = true`.
  * Command: `speckit capsule gc` removes expired run exports and orphaned temp files.
* Password/key UX (D54):
  * Support env var (`SPECKIT_MEMVID_PASSPHRASE`) and interactive prompt.
  * Optional OS keychain integration is a later enhancement (not required in v1).
* Safe export scope (D23, D9, D124):
  * Include: run artifacts, evidence logs, checkpoints, PolicySnapshotRef, RetrievalRequest/Response, GateDecisions, ErrorEvents, and manifests.
  * Exclude raw LLM I/O by default unless `capture.mode = "full_io"` (SPEC-KIT-975).
  * Apply redaction/masking to rendered views and exported bundle outputs.
* Audit logging (D23):
  * Every export writes a `CapsuleExported` event into the workspace capsule (who/when/what/safe-mode/encryption/digest).
  * Every import writes a `CapsuleImported` event into the workspace capsule and stores provenance metadata (source path, digest, policy version, timestamp).
* Import verification (D70, D103, D104):
  * `speckit capsule import` MUST run `speckit capsule doctor` checks on the imported capsule before mounting.
  * Enforce version compatibility; warn on unsigned/unverified capsules; hard-fail if `--require-verified` is set.

## Acceptance Criteria (testable)

* Export produces a single file artifact (`.mv2` or `.mv2e`) with no sidecar files.
* Encrypted capsule requires a passphrase; wrong passphrase fails safely without partial mounts.
* Import on a second machine reproduces identical retrieval results for checkpointed golden queries (within tolerance for floating scoring), using the imported capsule context.
* `capsule.export.mode=risk` only auto-exports when the configured risk conditions are met; otherwise it remains manual.
* Safe export redaction:
  * Secrets/PII are masked in rendered exports and replay reports by default.
  * Raw LLM I/O is excluded unless explicitly enabled via `capture.mode = "full_io"`.
* Every export writes a `CapsuleExported` event into the workspace capsule evidence timeline, including: run\_id, spec\_id, digest, encryption flag, safe flag, included tracks.
* Every import writes a `CapsuleImported` event into the workspace capsule evidence timeline, including: source digest, mount name, and validation result.
* Retention:
  * `speckit capsule gc` deletes expired exports older than `retention_days` unless pinned; leaves an audit trail event for deletions.

## Dependencies

* Memvid crate(s) pinned behind adapter boundary.
* Decision Register: `docs/DECISIONS.md`
* Architecture: `docs/ARCHITECTURE.md`

## Rollout / Rollback

* Roll out behind config flags with dual-backend fallback.
* Roll back by switching `memory_backend` back to `local-memory` and disabling Memvid features.

## Risks & Mitigations

* **Memvid API churn** → pin versions; wrap behind traits; contract tests.
* **Single-file contention** → single-writer + lock + writer queue.
* **Retrieval regressions** → eval harness + A/B parity gates.
* **Data leakage** → safe export redaction + optional sanitize-on-ingest mode.
