# Tasks: SPEC-PM-010 Reverse Sync

## Priority Tasks

- [ ] **T001** Define drift report schema and severity model.
  **Validation**: `cd codex-rs && cargo test -p codex-core drift::schema`
  **Artifact**: `docs/SPEC-PM-010-reverse-sync/artifacts/schema/drift-report.json`

- [ ] **T002** Implement milestone checkpoint drift scanner.
  **Validation**: `cd codex-rs && cargo test -p codex-core drift::checkpoint_scan`
  **Artifact**: `docs/SPEC-PM-010-reverse-sync/artifacts/tests/checkpoint-scan.txt`

- [ ] **T003** Implement sacred vs non-sacred drift classification.
  **Validation**: `cd codex-rs && cargo test -p codex-core drift::classification`
  **Artifact**: `docs/SPEC-PM-010-reverse-sync/artifacts/tests/classification.txt`

- [ ] **T004** Generate packet patch proposals for non-sacred drift.
  **Validation**: `cd codex-rs && cargo test -p codex-core drift::proposal_generation`
  **Artifact**: `docs/SPEC-PM-010-reverse-sync/artifacts/proposals/<timestamp>.md`

- [ ] **T005** Enforce sacred anchor write-block + epoch amendment routing.
  **Validation**: `cd codex-rs && cargo test -p codex-core drift::sacred_guard`
  **Artifact**: `docs/SPEC-PM-010-reverse-sync/artifacts/tests/sacred-guard.txt`

- [ ] **T006** Validate doc/policy consistency.
  **Validation**: `python3 scripts/doc_lint.py`
  **Artifact**: `docs/SPEC-PM-010-reverse-sync/artifacts/docs/doc-lint.txt`

## Definition of Done

- Drift is detected and classified at deterministic checkpoints.
- Non-sacred updates become reviewable packet proposals.
- Sacred drift never mutates packet fields silently.
