# SPEC-PM-008: Unattended Stacking + Morning Brief

**Status**: Planned  
**Phase**: Autonomous Lab (Days 31-60)  
**Owner**: PM Runtime

## Problem Statement

Unattended operation currently risks one of two failures: surprise merges or stalled progress. We need bounded autonomous progress without changing shipping state while user is away.

## Goals

- Enforce no-merge policy in unattended mode.
- Allow productive unattended research/prototyping/review runs.
- Generate a high-signal Morning Brief with ready-to-review outputs.

## Non-Goals

- Auto-merging code while unattended.
- Unlimited unattended run fan-out.
- Replacing attended decision checkpoints.

## Acceptance Criteria

- **AC1**: When unattended, merge actions are hard-disabled on primary thread.
- **AC2**: System can queue and run research/review tasks with bounded concurrency.
- **AC3**: Morning Brief summarizes verified stack items with evidence links and risk labels.
- **AC4**: Morning Brief is generated once per unattended window and visible at session resume.
- **AC5**: Notification behavior stays high-signal (critical only immediate; otherwise recap/digest).

## Constraints (ADR + Constitution)

- ADR-007: only primary thread may merge, and only under attended conditions.
- ADR-008: unattended mode = no merges; progress via non-merge work.
- ADR-011: notification contract must stay sparse and high-signal.
- Constitution: no duplicate runtime tracks, no rewrite scaffolding.

## Interfaces / Behavior

- Presence model: `attended | unattended` from explicit toggle + inactivity timeout.
- Queue model: `StackItem { item_id, type, confidence, evidence_uri }`.
- Brief artifact:
  - `docs/SPEC-PM-008-unattended-stacking/artifacts/morning-brief/<date>.md`
  - `docs/SPEC-PM-008-unattended-stacking/artifacts/morning-brief/<date>.json`

## Risk Register

- **Risk**: unattended queue grows unbounded.  
  **Mitigation**: max active runs + backlog caps.
- **Risk**: morning brief too noisy.  
  **Mitigation**: include only verified items with ranking threshold.
- **Risk**: implicit merges via side-path.  
  **Mitigation**: hard merge disable in unattended state at core gate.

## Decision IDs

- ADR-007 (primary thread merge policy)
- ADR-008 (unattended mode behavior)
- ADR-011 (high-signal notifications)
