# SPEC-PM-005: Gatekeeper (Class 2 Boundary + Class E Bypass)

**Status**: Planned  
**Phase**: Trust Foundation (Days 1-30)  
**Owner**: Architecture + Runtime

## Problem Statement

Major changes are currently not enforced against milestone boundaries, which allows architecture churn mid-milestone and increases merge-train instability.

## Goals

- Enforce a hard gate: Class 2 changes are blocked unless the current milestone is `Done`.
- Enforce merge-train policy consistently across TUI, CLI, and headless paths.
- Support emergency Class E bypass with strict blast-radius controls.

## Non-Goals

- Replacing the existing milestone model.
- Introducing a parallel runtime or rewrite path.
- Auto-approving Class 2 changes in unattended mode.

## Acceptance Criteria

- **AC1**: Every candidate change is classified as Class 0/1/2/E with an auditable reason payload.
- **AC2**: Class 2 adoption is blocked when milestone state is not `Done`; user receives boundary reason + required next action.
- **AC3**: Class E bypass requires emergency trigger evidence, pre-change snapshot, rollback script reference, and immediate notification event.
- **AC4**: Gate decisions are parity-consistent across TUI/CLI/headless (same result semantics and machine-readable fields).
- **AC5**: Gate decision artifacts are emitted for every blocked/override path.

## Constraints (ADR + Constitution)

- ADR-007: one primary merge thread; research/review threads never merge.
- ADR-009: Class 2 adoption only at milestone boundaries.
- ADR-012: Class E bypass allowed only with strict constraints.
- `memory/constitution.md`: `tui` is primary; no second-system rewrite.

## Interfaces / Behavior

- `GatekeeperInput`: change metadata, milestone state, thread type, posture, emergency context.
- `GateDecision`: `allow | block | bypass_emergency`, `class`, `reason`, `required_actions`.
- Evidence artifacts:
  - `docs/SPEC-PM-005-gatekeeper-class2-boundary/artifacts/gate-decisions/<timestamp>.json`
  - `docs/SPEC-PM-005-gatekeeper-class2-boundary/artifacts/emergency/<timestamp>.md`

## Risk Register

- **Risk**: Misclassification causes false blocks.  
  **Mitigation**: deterministic classifier tests with golden fixtures.
- **Risk**: Class E abused to bypass process.  
  **Mitigation**: mandatory trigger evidence + snapshot + rollback + notification.
- **Risk**: Surface drift (TUI/CLI/headless).  
  **Mitigation**: shared core evaluator + parity tests.

## Decision IDs

- ADR-007 (thread model and merge privileges)
- ADR-009 (Class 2 boundary semantics)
- ADR-012 (Class E emergency protocol)
