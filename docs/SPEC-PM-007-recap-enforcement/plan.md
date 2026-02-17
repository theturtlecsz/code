# Plan: SPEC-PM-007 Recap Enforcement

## Approach Options

1. **Soft reminder only**  
   Rejected: not enforceable.
2. **Hard gate with structured recap (chosen)**  
   Enforces contract, preserves audit trail.
3. **Long-form narrative reports**  
   Rejected: high overhead, low signal.

## Chosen Path

Add a recap guard to execution-shift and merge paths. Guard verifies recap freshness against packet epoch and milestone context.

## Milestone Boundary Semantics

- Recap does not replace boundary gating.
- Class 2 attempts still require milestone boundary through Gatekeeper.
- Recap artifact must reference current milestone and class decision context.

## Rollout / Migration

- Start in warning mode for one iteration.
- Move to hard-fail once recap artifacts are emitted reliably.
- Include daily recap generation in unattended scheduler.

## Validation Mapping

| Requirement | Validation | Artifact |
| --- | --- | --- |
| AC1 | `cd codex-rs && cargo test -p codex-core recap::gate_required` | `docs/SPEC-PM-007-recap-enforcement/artifacts/tests/gate-required.txt` |
| AC2 | `cd codex-rs && cargo test -p codex-core recap::schema` | `docs/SPEC-PM-007-recap-enforcement/artifacts/tests/schema.txt` |
| AC3 | `cd codex-rs && cargo test -p codex-core recap::stale_detection` | `docs/SPEC-PM-007-recap-enforcement/artifacts/tests/stale.txt` |
| AC4 | `cd codex-rs && cargo test -p codex-cli recap::parity` | `docs/SPEC-PM-007-recap-enforcement/artifacts/tests/parity.txt` |
| AC5 | `cd codex-rs && cargo test -p codex-tui recap::daily_digest` | `docs/SPEC-PM-007-recap-enforcement/artifacts/tests/daily-digest.txt` |
