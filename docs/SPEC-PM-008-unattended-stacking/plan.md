# Plan: SPEC-PM-008 Unattended Stacking

## Approach Options

1. **Pause all work when unattended**  
   Rejected: no progress.
2. **Allow full autonomy including merges**  
   Rejected: violates ADR-008.
3. **Stack non-merge outputs + morning brief (chosen)**  
   Preserves safety while maintaining momentum.

## Chosen Path

Use unattended mode to run bounded research/review/prototype work. Accumulate high-confidence outputs into a single Morning Brief package for attended review.

## Milestone Boundary Semantics

- Unattended outputs cannot cross ship boundary automatically.
- Class 2 recommendations remain proposals until attended milestone boundary decisions.
- Morning Brief includes boundary status for each proposed change.

## Rollout / Migration

- Introduce unattended state machine and merge-disable guard first.
- Add stacking queue with simple prioritization.
- Add morning brief assembly + resume-time surfacing.

## Validation Mapping

| Requirement | Validation | Artifact |
| --- | --- | --- |
| AC1 | `cd codex-rs && cargo test -p codex-core unattended::no_merge` | `docs/SPEC-PM-008-unattended-stacking/artifacts/tests/no-merge.txt` |
| AC2 | `cd codex-rs && cargo test -p codex-core unattended::queue_bounds` | `docs/SPEC-PM-008-unattended-stacking/artifacts/tests/queue-bounds.txt` |
| AC3 | `cd codex-rs && cargo test -p codex-tui unattended::morning_brief_content` | `docs/SPEC-PM-008-unattended-stacking/artifacts/tests/brief-content.txt` |
| AC4 | `cd codex-rs && cargo test -p codex-tui unattended::resume_surface` | `docs/SPEC-PM-008-unattended-stacking/artifacts/tests/resume-surface.txt` |
| AC5 | `cd codex-rs && cargo test -p codex-core notifications::unattended_policy` | `docs/SPEC-PM-008-unattended-stacking/artifacts/tests/notification-policy.txt` |
