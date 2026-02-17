# Plan: SPEC-PM-006 Packet Persistence

## Approach Options

1. **YAML file contract (chosen)**  
   Minimal operational complexity; human-auditable.
2. **SQLite-backed packet state**  
   Deferred; unnecessary for current phase.
3. **In-memory + projection only**  
   Rejected: violates durability requirement.

## Chosen Path

Implement `.speckit/packet.yaml` as authoritative packet contract with shared serializer/parser and strict schema validation.

## Milestone Boundary Semantics

- Milestone state in packet is the only boundary source used by Gatekeeper.
- Packet writes for milestone transitions must be atomic and auditable.
- Class 2 decisions update packet only after explicit boundary transition.

## Rollout / Migration

- Add packet bootstrap command for existing workspaces.
- Backfill existing milestone/intent metadata into packet once.
- Flip runtime to read packet as first source, then remove legacy fallback reads.

## Validation Mapping

| Requirement | Validation | Artifact |
| --- | --- | --- |
| AC1 | `cd codex-rs && cargo test -p codex-core packet::atomic_write` | `docs/SPEC-PM-006-packet-persistence/artifacts/tests/atomic-write.txt` |
| AC2 | `cd codex-rs && cargo test -p codex-tui packet::restore_on_startup` | `docs/SPEC-PM-006-packet-persistence/artifacts/tests/restart-restore.txt` |
| AC3 | `cd codex-rs && cargo test -p codex-core packet::sacred_anchor_guard` | `docs/SPEC-PM-006-packet-persistence/artifacts/tests/anchor-guard.txt` |
| AC4 | `cd codex-rs && cargo test -p codex-cli packet::recovery_messages` | `docs/SPEC-PM-006-packet-persistence/artifacts/tests/recovery.txt` |
| AC5 | `cd codex-rs && cargo test -p codex-cli packet::surface_parity` | `docs/SPEC-PM-006-packet-persistence/artifacts/tests/parity.txt` |
