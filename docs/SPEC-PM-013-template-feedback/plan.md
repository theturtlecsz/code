# Plan: SPEC-PM-013 Template Feedback

## Approach Options

1. **Manual-only template updates**  
   Rejected: slow and inconsistent learning.
2. **Assisted extraction + approval gate (chosen)**  
   Preserves control while scaling learning.
3. **Fully automatic promotion**  
   Rejected: too risky for shared templates.

## Chosen Path

Implement candidate extraction from successful specs, then route through proposal review and approval before applying template updates.

## Milestone Boundary Semantics

- Template promotions are treated as spec/template proposals.
- Class 2 template-impacting changes follow milestone boundary rules.
- Promotions include rollback plan for template version regressions.

## Rollout / Migration

- Start with read-only candidate generation.
- Enable approval workflow with explicit artifacts.
- Enable template write path after regression harness is stable.

## Validation Mapping

| Requirement | Validation | Artifact |
| --- | --- | --- |
| AC1 | `cd codex-rs && cargo test -p codex-core templates::candidate_extraction` | `docs/SPEC-PM-013-template-feedback/artifacts/tests/candidate-extraction.txt` |
| AC2 | `cd codex-rs && cargo test -p codex-core templates::confidence_gate` | `docs/SPEC-PM-013-template-feedback/artifacts/tests/confidence-gate.txt` |
| AC3 | `cd codex-rs && cargo test -p codex-tui templates::approval_flow` | `docs/SPEC-PM-013-template-feedback/artifacts/tests/approval-flow.txt` |
| AC4 | `cd codex-rs && cargo test -p codex-core templates::promotion_apply` | `docs/SPEC-PM-013-template-feedback/artifacts/promotions/<timestamp>.md` |
| AC5 | `cd codex-rs && cargo test -p codex-core templates::regression_harness` | `docs/SPEC-PM-013-template-feedback/artifacts/tests/regression.txt` |
