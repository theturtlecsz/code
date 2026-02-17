# SPEC-PM-006: Packet Persistence (`.speckit/packet.yaml`)

**Status**: Planned  
**Phase**: Trust Foundation (Days 1-30)  
**Owner**: Product Runtime

## Problem Statement

The project contract is not durably persisted as a first-class Packet, which causes restart-time drift and weakens milestone/gate enforcement.

## Goals

- Persist Packet contract in `.speckit/packet.yaml` with durable read/write semantics.
- Preserve sacred anchors and epoch metadata across restarts.
- Provide deterministic loading behavior for all Tier-1 surfaces.

## Non-Goals

- Moving packet storage to a database.
- Auto-mutating sacred fields without explicit amendment flow.
- Replacing existing spec artifacts.

## Acceptance Criteria

- **AC1**: Packet file is created/updated atomically with schema versioning.
- **AC2**: Startup restores packet state (milestones, anchors, epoch) with no user prompt.
- **AC3**: Sacred fields (`intent_summary`, `success_criteria`) cannot change without explicit amendment workflow.
- **AC4**: Packet corruption or missing file returns deterministic recovery guidance.
- **AC5**: Packet API semantics are identical for TUI/CLI/headless reads.

## Constraints (ADR + Constitution)

- ADR-005: Packet is execution contract, not a summary.
- ADR-006: sacred anchors + epoch changes for material drift.
- ADR-009: milestone contract must drive boundary checks.
- Constitution: running system first; no parallel rewrite track.

## Interfaces / Behavior

- Canonical file: `.speckit/packet.yaml`
- Core structs:
  - `PacketHeader` (schema/version/epoch)
  - `SacredAnchors`
  - `MilestoneContract[]`
  - `ExecutionState`
- Error classes: `PacketMissing`, `PacketInvalid`, `PacketWriteFailed`.

## Risk Register

- **Risk**: Packet/schema drift over time.  
  **Mitigation**: schema version + migration validator.
- **Risk**: partial writes create unreadable packets.  
  **Mitigation**: temp-file + fsync + atomic rename.
- **Risk**: multiple surfaces diverge in parsing rules.  
  **Mitigation**: single parser/writer library.

## Decision IDs

- ADR-005 (consultant-first packet contract)
- ADR-006 (sacred anchors and epochs)
- ADR-009 (milestone contract boundaries)
