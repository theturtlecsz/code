# Plan: SPEC-PM-005 Gatekeeper

## Approach Options

1. **Inline ad-hoc checks in each surface**  
   Rejected: high drift risk and duplicate logic.
2. **Shared core gate evaluator (chosen)**  
   Single evaluator library invoked by TUI/CLI/headless adapters.
3. **External policy service**  
   Deferred: adds operational complexity too early.

## Chosen Path

Implement a shared gate evaluator in core flow, then wire adapters in TUI/CLI/headless. Keep emergency path explicit and auditable.

## Milestone Boundary Semantics

- Milestone boundary = milestone state is `Done` in Packet contract.
- Class 2 adoption attempts before boundary return hard block.
- Class E bypass ignores boundary but requires:
  - emergency trigger data,
  - snapshot artifact,
  - rollback path,
  - immediate notification emission.

## Rollout / Migration

- Phase A: report-only mode with decision artifacts (no hard block).
- Phase B: hard-block enforcement for Class 2 on primary merge train.
- Phase C: enforce parity assertions in headless/CLI CI checks.

## Validation Mapping

| Requirement | Validation | Artifact |
| --- | --- | --- |
| AC1 | `cd codex-rs && cargo test -p codex-core gatekeeper::classification` | `docs/SPEC-PM-005-gatekeeper-class2-boundary/artifacts/tests/classification.txt` |
| AC2 | `cd codex-rs && cargo test -p codex-tui gatekeeper::boundary_block` | `docs/SPEC-PM-005-gatekeeper-class2-boundary/artifacts/tests/boundary-block.txt` |
| AC3 | `cd codex-rs && cargo test -p codex-core gatekeeper::emergency_bypass` | `docs/SPEC-PM-005-gatekeeper-class2-boundary/artifacts/tests/emergency.txt` |
| AC4 | `cd codex-rs && cargo test -p codex-cli pm_gatekeeper_parity` | `docs/SPEC-PM-005-gatekeeper-class2-boundary/artifacts/tests/parity.txt` |
| AC5 | `python3 scripts/doc_lint.py` | `docs/SPEC-PM-005-gatekeeper-class2-boundary/artifacts/gate-decisions/` |
