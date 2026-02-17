# Tasks: SPEC-PM-011 Hysteresis Engine

## Priority Tasks

- [ ] **T001** Implement score model and weight normalization.
  **Validation**: `cd codex-rs && cargo test -p codex-core hysteresis::score_formula`
  **Artifact**: `docs/SPEC-PM-011-hysteresis-engine/artifacts/schema/plan-score.json`

- [ ] **T002** Implement dominance margin evaluator (default 0.15).
  **Validation**: `cd codex-rs && cargo test -p codex-core hysteresis::dominance_margin`
  **Artifact**: `docs/SPEC-PM-011-hysteresis-engine/artifacts/tests/dominance-margin.txt`

- [ ] **T003** Enforce auto-pick confidence/evidence thresholds.
  **Validation**: `cd codex-rs && cargo test -p codex-core hysteresis::autopick_threshold`
  **Artifact**: `docs/SPEC-PM-011-hysteresis-engine/artifacts/tests/autopick-threshold.txt`

- [ ] **T004** Integrate hold/replace outcomes with proposal inbox state.
  **Validation**: `cd codex-rs && cargo test -p codex-tui hysteresis::inbox_integration`
  **Artifact**: `docs/SPEC-PM-011-hysteresis-engine/artifacts/tests/inbox-integration.txt`

- [ ] **T005** Emit audit artifacts for all hysteresis decisions.
  **Validation**: `cd codex-rs && cargo test -p codex-core hysteresis::audit_log`
  **Artifact**: `docs/SPEC-PM-011-hysteresis-engine/artifacts/decisions/<timestamp>.md`

- [ ] **T006** Validate docs consistency and update references.
  **Validation**: `python3 scripts/doc_lint.py`
  **Artifact**: `docs/SPEC-PM-011-hysteresis-engine/artifacts/docs/doc-lint.txt`

## Definition of Done

- Plan-of-record remains stable unless dominance criteria are met.
- Decision explanations are audit-ready.
- Emergency path remains available and explicitly marked.
