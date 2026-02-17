# Plan: SPEC-PM-011 Hysteresis Engine

## Approach Options

1. **No hysteresis (replace on any higher score)**  
   Rejected: high thrash.
2. **Static dominance margin (chosen)**  
   Simple, transparent, and aligned with vision.
3. **Adaptive/learned margin**  
   Deferred until sufficient production data exists.

## Chosen Path

Implement deterministic dominance-margin evaluation with explicit confidence/evidence gating. Integrate with proposal ranking and plan selection.

## Milestone Boundary Semantics

- Hysteresis governs plan replacement, not boundary legality.
- Class 2 plan replacements still require milestone boundary approval.
- Class E path bypasses hysteresis with audit annotation.

## Rollout / Migration

- Enable read-only score comparison first.
- Activate hold/replace decisions after baseline calibration.
- Expose threshold configuration after stability baseline is validated.

## Validation Mapping

| Requirement | Validation | Artifact |
| --- | --- | --- |
| AC1 | `cd codex-rs && cargo test -p codex-core hysteresis::score_formula` | `docs/SPEC-PM-011-hysteresis-engine/artifacts/tests/score-formula.txt` |
| AC2 | `cd codex-rs && cargo test -p codex-core hysteresis::dominance_margin` | `docs/SPEC-PM-011-hysteresis-engine/artifacts/tests/dominance-margin.txt` |
| AC3 | `cd codex-rs && cargo test -p codex-core hysteresis::autopick_threshold` | `docs/SPEC-PM-011-hysteresis-engine/artifacts/tests/autopick-threshold.txt` |
| AC4 | `cd codex-rs && cargo test -p codex-tui hysteresis::near_tie_behavior` | `docs/SPEC-PM-011-hysteresis-engine/artifacts/tests/near-tie.txt` |
| AC5 | `cd codex-rs && cargo test -p codex-core hysteresis::audit_log` | `docs/SPEC-PM-011-hysteresis-engine/artifacts/decisions/<timestamp>.md` |
