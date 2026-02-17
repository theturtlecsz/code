# SPEC-PM-007: Recap Enforcement (Explain Before Act)

**Status**: Planned  
**Phase**: Trust Foundation (Days 1-30)  
**Owner**: UX + Runtime

## Problem Statement

Execution shifts and merges can happen without a consistent recap artifact, reducing operator trust and making decisions hard to audit.

## Goals

- Mechanically enforce recap output before execution shifts and merges.
- Standardize recap structure: Intent, Plan, Gates, Rollback.
- Keep recap output visible in attended and unattended flows.

## Non-Goals

- Replacing full planning docs.
- Using recap as a substitute for milestone decisions.
- Introducing long-form narrative requirements.

## Acceptance Criteria

- **AC1**: Merge or execution shift is blocked if a valid recap artifact is missing.
- **AC2**: Recap includes required fields: `intent`, `plan`, `gates`, `rollback`.
- **AC3**: Recap validity is bound to packet epoch and milestone context; stale recaps are rejected.
- **AC4**: Recap policy is enforced consistently in TUI/CLI/headless flows.
- **AC5**: Unattended mode emits daily recap digest without interrupting users.

## Constraints (ADR + Constitution)

- ADR-005: recap before major execution shifts.
- ADR-007: merge behavior constrained to primary thread.
- ADR-008/011: unattended mode with non-interrupting recap cadence.
- Constitution: keep docs/tests/guardrails synchronized.

## Interfaces / Behavior

- Recap model:
  - `recap_id`, `packet_epoch`, `milestone_id`, `intent`, `plan`, `gates`, `rollback`
- Required artifacts:
  - `docs/SPEC-PM-007-recap-enforcement/artifacts/recaps/<timestamp>.md`
  - `docs/SPEC-PM-007-recap-enforcement/artifacts/recaps/<timestamp>.json`
- Gate condition: operations requiring recap fail closed when artifact missing/stale.

## Risk Register

- **Risk**: Recap generated but not meaningful.  
  **Mitigation**: schema validation + min content checks.
- **Risk**: Recap friction slows flow.  
  **Mitigation**: concise template and auto-fill from packet context.
- **Risk**: unattended notification spam.  
  **Mitigation**: daily digest policy + high-signal alert rules.

## Decision IDs

- ADR-005 (recap before execution shift)
- ADR-007 (merge-train constraints)
- ADR-011 (notification/recap contract)
