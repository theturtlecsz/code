# Plan: SPEC-PM-009 Proposal Ranking

## Approach Options

1. **Chronological inbox only**  
   Rejected: low signal-to-noise.
2. **Rule-based weighted scoring (chosen)**  
   Transparent and testable in current phase.
3. **ML ranking model**  
   Deferred until stable feedback data exists.

## Chosen Path

Implement deterministic weighted scoring and ranking with explicit explanation fields. Default view is top 3; extended view exposes top 10.

## Milestone Boundary Semantics

- Ranking does not authorize adoption.
- Class 2-ranked proposals still require milestone boundary decisions.
- Proposal status includes `requires_boundary` marker for Class 2 impacts.

## Rollout / Migration

- Introduce ranking engine with side-by-side old/new ordering for calibration.
- Enable top-3 default after score sanity checks.
- Enable dedupe/archive job once provenance retention is verified.

## Validation Mapping

| Requirement | Validation | Artifact |
| --- | --- | --- |
| AC1 | `cd codex-rs && cargo test -p codex-core proposal::scoring` | `docs/SPEC-PM-009-proposal-ranking-top3/artifacts/tests/scoring.txt` |
| AC2 | `cd codex-rs && cargo test -p codex-tui proposal::top3_view` | `docs/SPEC-PM-009-proposal-ranking-top3/artifacts/tests/top3-view.txt` |
| AC3 | `cd codex-rs && cargo test -p codex-core proposal::dedupe` | `docs/SPEC-PM-009-proposal-ranking-top3/artifacts/tests/dedupe.txt` |
| AC4 | `cd codex-rs && cargo test -p codex-core proposal::archive_ttl` | `docs/SPEC-PM-009-proposal-ranking-top3/artifacts/tests/archive.txt` |
| AC5 | `cd codex-rs && cargo test -p codex-tui proposal::top10_expand` | `docs/SPEC-PM-009-proposal-ranking-top3/artifacts/tests/top10-expand.txt` |
