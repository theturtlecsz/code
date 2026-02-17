# Tasks: SPEC-PM-005 Gatekeeper

## Priority Tasks

- [ ] **T001** Define deterministic change classification schema (0/1/2/E) and decision payload.
  **Validation**: `cd codex-rs && cargo test -p codex-core gatekeeper::classification`
  **Artifact**: `docs/SPEC-PM-005-gatekeeper-class2-boundary/artifacts/schema/gate-classification.json`

- [ ] **T002** Implement milestone-boundary block for Class 2 in shared evaluator.
  **Validation**: `cd codex-rs && cargo test -p codex-core gatekeeper::boundary_block`
  **Artifact**: `docs/SPEC-PM-005-gatekeeper-class2-boundary/artifacts/tests/boundary-block.txt`

- [ ] **T003** Implement Class E bypass constraints (snapshot + rollback + notify).
  **Validation**: `cd codex-rs && cargo test -p codex-core gatekeeper::emergency`
  **Artifact**: `docs/SPEC-PM-005-gatekeeper-class2-boundary/artifacts/emergency/class-e-checklist.md`

- [ ] **T004** Wire shared evaluator into TUI/CLI/headless merge paths.
  **Validation**: `cd codex-rs && cargo test -p codex-cli pm_gatekeeper_parity`
  **Artifact**: `docs/SPEC-PM-005-gatekeeper-class2-boundary/artifacts/tests/parity.txt`

- [ ] **T005** Emit decision artifacts for blocks and overrides.
  **Validation**: `cd codex-rs && cargo test -p codex-core gatekeeper::artifacts`
  **Artifact**: `docs/SPEC-PM-005-gatekeeper-class2-boundary/artifacts/gate-decisions/<timestamp>.json`

- [ ] **T006** Update operator docs and run doc contract checks.
  **Validation**: `python3 scripts/doc_lint.py`
  **Artifact**: `docs/SPEC-PM-005-gatekeeper-class2-boundary/artifacts/docs/doc-lint.txt`

## Definition of Done

- All acceptance criteria AC1-AC5 pass.
- No Class 2 change can merge mid-milestone in primary thread.
- Emergency bypass path is auditable and rollback-capable.
