# Plan: SPEC-PM-010 Reverse Sync

## Approach Options

1. **Manual drift review only**  
   Rejected: inconsistent and easy to skip.
2. **Rule-based drift analyzer + proposal generator (chosen)**  
   Deterministic, auditable, and aligned with packet contract.
3. **LLM-only drift interpretation**  
   Deferred: higher variance for Tier-1 contract logic.

## Chosen Path

Build deterministic drift scanners tied to milestone checkpoints, then generate packet patch proposals with strict sacred-anchor guards.

## Milestone Boundary Semantics

- Drift scans run at boundary checkpoints.
- Class 2 drift findings are marked `requires_boundary`.
- Sacred drift cannot be auto-applied and must route to epoch amendment.

## Rollout / Migration

- Start with read-only drift reports.
- Enable non-sacred packet patch proposals after precision threshold is met.
- Keep sacred-drift path decision-only throughout phase.

## Validation Mapping

| Requirement | Validation | Artifact |
| --- | --- | --- |
| AC1 | `cd codex-rs && cargo test -p codex-core drift::checkpoint_scan` | `docs/SPEC-PM-010-reverse-sync/artifacts/tests/checkpoint-scan.txt` |
| AC2 | `cd codex-rs && cargo test -p codex-core drift::classification` | `docs/SPEC-PM-010-reverse-sync/artifacts/tests/classification.txt` |
| AC3 | `cd codex-rs && cargo test -p codex-core drift::proposal_generation` | `docs/SPEC-PM-010-reverse-sync/artifacts/tests/proposal-generation.txt` |
| AC4 | `cd codex-rs && cargo test -p codex-core drift::sacred_guard` | `docs/SPEC-PM-010-reverse-sync/artifacts/tests/sacred-guard.txt` |
| AC5 | `python3 scripts/doc_lint.py` | `docs/SPEC-PM-010-reverse-sync/artifacts/drift/` |
