# SPEC-PM-011: Hysteresis Engine (Stability Bias)

**Status**: Planned  
**Phase**: Learning Loop (Days 61-90)  
**Owner**: Decision Engine

## Problem Statement

The system can churn on marginal improvements, causing plan thrash and instability. We need explicit dominance requirements before replacing the plan-of-record.

## Goals

- Implement score-based hysteresis with dominance margin checks.
- Prevent plan replacement on low-value deltas.
- Keep emergency/security paths exempt where required.

## Non-Goals

- Replacing baseline scoring model in ADR vision.
- ML optimization of score weights in this phase.
- Blocking user-directed overrides with explicit approvals.

## Acceptance Criteria

- **AC1**: Score formula includes intent match, performance, simplicity, and thrash penalty.
- **AC2**: Candidate plan must exceed current plan by configured dominance margin (default 15%).
- **AC3**: Auto-pick requires confidence > 0.85 and high evidence quality.
- **AC4**: Near-tie proposals remain in inbox and do not replace plan-of-record.
- **AC5**: Hysteresis decisions are logged with factor breakdown for audit.

## Constraints (ADR + Constitution)

- VISION scoring contract and hysteresis rule.
- ADR-010: ranking integration.
- ADR-012: emergency path can bypass hysteresis when required.
- Constitution: test-backed decisions and synchronized docs.

## Interfaces / Behavior

- `PlanScore`: `intent`, `performance`, `simplicity`, `thrash_penalty`, `total`.
- `HysteresisDecision`: `replace | hold | bypass_emergency` + explanation.
- Artifacts:
  - `docs/SPEC-PM-011-hysteresis-engine/artifacts/scoring/<timestamp>.json`
  - `docs/SPEC-PM-011-hysteresis-engine/artifacts/decisions/<timestamp>.md`

## Risk Register

- **Risk**: overly strict margin blocks useful changes.  
  **Mitigation**: configurable threshold + review override.
- **Risk**: scoring inputs are noisy.  
  **Mitigation**: normalize metrics and keep provenance.
- **Risk**: emergency wrongly blocked by hysteresis.  
  **Mitigation**: explicit Class E bypass lane.

## Decision IDs

- ADR-010 (proposal ranking integration)
- ADR-012 (emergency bypass lane)
- VISION v1.1.0 scoring/hysteresis contract
