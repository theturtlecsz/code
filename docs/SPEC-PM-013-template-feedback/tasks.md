# Tasks: SPEC-PM-013 Template Feedback

## Priority Tasks

- [ ] **T001** Define candidate extraction schema and source requirements.
  **Validation**: `cd codex-rs && cargo test -p codex-core templates::schema`
  **Artifact**: `docs/SPEC-PM-013-template-feedback/artifacts/schema/pattern-candidate.json`

- [ ] **T002** Implement successful-pattern extractor from completed specs.
  **Validation**: `cd codex-rs && cargo test -p codex-core templates::candidate_extraction`
  **Artifact**: `docs/SPEC-PM-013-template-feedback/artifacts/candidates/<timestamp>.json`

- [ ] **T003** Add confidence/evidence gating for candidate surfacing.
  **Validation**: `cd codex-rs && cargo test -p codex-core templates::confidence_gate`
  **Artifact**: `docs/SPEC-PM-013-template-feedback/artifacts/tests/confidence-gate.txt`

- [ ] **T004** Implement approval workflow and decision artifacts.
  **Validation**: `cd codex-rs && cargo test -p codex-tui templates::approval_flow`
  **Artifact**: `docs/SPEC-PM-013-template-feedback/artifacts/promotions/<timestamp>.md`

- [ ] **T005** Implement template update + changelog + rollback metadata.
  **Validation**: `cd codex-rs && cargo test -p codex-core templates::promotion_apply`
  **Artifact**: `docs/SPEC-PM-013-template-feedback/artifacts/promotions/template-diff.patch`

- [ ] **T006** Run template regression checks and doc lint.
  **Validation**: `cd codex-rs && cargo test -p codex-core templates::regression_harness && python3 scripts/doc_lint.py`
  **Artifact**: `docs/SPEC-PM-013-template-feedback/artifacts/tests/regression.txt`

## Definition of Done

- Template learning loop is evidence-backed and approval-gated.
- Promotions are traceable to successful source specs.
- Template updates are regression-tested before release.
