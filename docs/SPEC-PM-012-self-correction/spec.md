# SPEC-PM-012: Self-Correction (Build Failure Recovery)

**Status**: Planned  
**Phase**: Learning Loop (Days 61-90)  
**Owner**: Implementer Runtime

## Problem Statement

Agent workflows can escalate too early on build/test failures, creating avoidable human interruption and reducing autonomous throughput.

## Goals

- Retry failed build/test loops automatically before escalating.
- Use each failed attempt as context for the next correction attempt.
- Escalate with a concise, high-signal failure package only after bounded retries.

## Non-Goals

- Infinite autonomous debugging loops.
- Silent skipping of failing tests.
- Auto-merge of self-correction changes in unattended mode.

## Acceptance Criteria

- **AC1**: Build/test failures trigger bounded retry loop (default max attempts: 3).
- **AC2**: Each retry includes failure context from previous attempt.
- **AC3**: Retry loop stops early on success and emits completion evidence.
- **AC4**: Final escalation packet includes root-cause summary, attempted fixes, and next-best options.
- **AC5**: Loop guard prevents repeated retries on unchanged deterministic failure signatures.

## Constraints (ADR + Constitution)

- VISION: self-correction before asking for help.
- ADR-007/008: no unattended merge side-effects.
- ADR-011: notifications only on final escalation/high-signal events.
- Constitution: reproducible artifacts and single source-of-truth trackers.

## Interfaces / Behavior

- `RetryContext`: command, failure signature, logs, prior patch summary.
- `RetryOutcome`: `success | retry | escalate`.
- Artifacts:
  - `docs/SPEC-PM-012-self-correction/artifacts/retries/<run_id>.json`
  - `docs/SPEC-PM-012-self-correction/artifacts/escalations/<run_id>.md`

## Risk Register

- **Risk**: retry loop wastes time on impossible failures.  
  **Mitigation**: signature-based early stop.
- **Risk**: retries mutate too broadly.  
  **Mitigation**: patch scope caps + diff size limits.
- **Risk**: escalation packet too verbose/low signal.  
  **Mitigation**: fixed concise escalation schema.

## Decision IDs

- ADR-007 (merge safety constraints)
- ADR-008 (unattended non-merge operation)
- ADR-011 (escalation notification policy)
