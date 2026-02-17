# Tasks: SPEC-PM-007 Recap Enforcement

## Priority Tasks

- [ ] **T001** Define recap schema and required fields.
  **Validation**: `cd codex-rs && cargo test -p codex-core recap::schema`
  **Artifact**: `docs/SPEC-PM-007-recap-enforcement/artifacts/schema/recap-schema.json`

- [ ] **T002** Implement recap generation from packet + milestone context.
  **Validation**: `cd codex-rs && cargo test -p codex-core recap::generation`
  **Artifact**: `docs/SPEC-PM-007-recap-enforcement/artifacts/recaps/sample-recap.md`

- [ ] **T003** Add hard gate for missing/stale recap in execution shifts/merges.
  **Validation**: `cd codex-rs && cargo test -p codex-core recap::gate_required`
  **Artifact**: `docs/SPEC-PM-007-recap-enforcement/artifacts/tests/gate-required.txt`

- [ ] **T004** Enforce parity for TUI/CLI/headless recap checks.
  **Validation**: `cd codex-rs && cargo test -p codex-cli recap::parity`
  **Artifact**: `docs/SPEC-PM-007-recap-enforcement/artifacts/tests/parity.txt`

- [ ] **T005** Implement unattended daily recap digest output.
  **Validation**: `cd codex-rs && cargo test -p codex-tui recap::daily_digest`
  **Artifact**: `docs/SPEC-PM-007-recap-enforcement/artifacts/recaps/daily-digest.md`

- [ ] **T006** Validate documentation consistency.
  **Validation**: `python3 scripts/doc_lint.py`
  **Artifact**: `docs/SPEC-PM-007-recap-enforcement/artifacts/docs/doc-lint.txt`

## Definition of Done

- Merge and execution-shift operations are impossible without valid recap artifacts.
- Recap output is concise, structured, and tied to current packet epoch.
- Unattended mode produces daily recap with no extra notification noise.
