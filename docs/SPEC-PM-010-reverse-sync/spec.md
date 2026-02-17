# SPEC-PM-010: Reverse Sync (Code Intent Drift Detection)

**Status**: Planned  
**Phase**: Autonomous Lab (Days 31-60)  
**Owner**: Runtime + Spec Governance

## Problem Statement

Code and packet intent can diverge over time. Without systematic reverse sync, the contract becomes stale and trust in autonomy degrades.

## Goals

- Detect drift between implementation state and packet contract.
- Generate explicit packet update proposals when drift is non-sacred.
- Route sacred-anchor drift into epoch amendment decisions.

## Non-Goals

- Auto-updating sacred fields.
- Inferring product intent from code without user review.
- Full semantic code understanding beyond milestone scope.

## Acceptance Criteria

- **AC1**: Drift scan runs at milestone boundaries and merge-train checkpoints.
- **AC2**: Drift reports classify issues as `non_sacred_update` or `epoch_amendment_required`.
- **AC3**: Non-sacred drift produces packet update proposals with evidence.
- **AC4**: Sacred drift triggers a Class 2 decision path (no silent edits).
- **AC5**: Drift evidence and proposed patch are auditable.

## Constraints (ADR + Constitution)

- ADR-006: sacred anchor changes require epoch handling.
- ADR-009: major changes follow boundary semantics.
- ADR-005: packet remains execution contract.
- Constitution: docs/tests/sync discipline and no rewrite branch.

## Interfaces / Behavior

- `DriftReport`:
  - `drift_id`, `milestone_id`, `severity`, `class`, `summary`, `evidence`
- `PacketPatchProposal`:
  - non-sacred update draft + review checklist
- Artifacts:
  - `docs/SPEC-PM-010-reverse-sync/artifacts/drift/<timestamp>.json`
  - `docs/SPEC-PM-010-reverse-sync/artifacts/proposals/<timestamp>.md`

## Risk Register

- **Risk**: false positive drift reports.  
  **Mitigation**: scoped analyzers + confidence threshold.
- **Risk**: drift ignored by users.  
  **Mitigation**: include in recap + morning brief channels.
- **Risk**: sacred drift auto-edited by bug.  
  **Mitigation**: hard guard against sacred field writes.

## Decision IDs

- ADR-006 (epoch amendment for sacred drift)
- ADR-005 (packet as execution contract)
- ADR-009 (boundary-aware adoption)
