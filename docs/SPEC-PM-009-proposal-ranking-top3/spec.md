# SPEC-PM-009: Proposal Ranking (Top 3 Signal)

**Status**: Planned  
**Phase**: Autonomous Lab (Days 31-60)  
**Owner**: Product Intelligence

## Problem Statement

Proposal streams can overwhelm users and degrade trust. We need bounded, high-signal surfacing that proves the system can prioritize what matters.

## Goals

- Rank proposals with explicit scoring factors from ADR-010.
- Show only top 3 proposals per category by default.
- Keep broader visibility bounded (top 10), deduplicated, and archived.

## Non-Goals

- Fully autonomous acceptance of proposals.
- Replacing human decision on major changes.
- Building a complex ML ranker in this phase.

## Acceptance Criteria

- **AC1**: Ranking score includes intent/success alignment, expected gain, security impact, cost, and evidence quality.
- **AC2**: UI default shows top 3 per category (`Architecture/Refactor`, `Spec/Template`).
- **AC3**: Duplicate proposals are merged with provenance retained.
- **AC4**: Proposals older than 7 days are archived unless pinned.
- **AC5**: Top 10 per category remain discoverable on demand.

## Constraints (ADR + Constitution)

- ADR-010: category model, ranking factors, pruning rules.
- ADR-006: sacred anchors must dominate alignment scoring.
- ADR-011: avoid notification spam; inbox is primary surfacing channel.
- Constitution: evidence-driven docs/tests before implementation.

## Interfaces / Behavior

- `ProposalScore` fields: `alignment`, `gain`, `security`, `cost`, `evidence`, `total`.
- `ProposalCategory`: `architecture_refactor | spec_template`.
- Artifacts:
  - `docs/SPEC-PM-009-proposal-ranking-top3/artifacts/ranking/<timestamp>.json`
  - `docs/SPEC-PM-009-proposal-ranking-top3/artifacts/archive/<date>.json`

## Risk Register

- **Risk**: Scoring bias hides important ideas.  
  **Mitigation**: transparent score breakdown + manual pin.
- **Risk**: Duplicate merge loses context.  
  **Mitigation**: provenance list in merged proposal record.
- **Risk**: Top-3 cap appears arbitrary.  
  **Mitigation**: retain top-10 expansion path and evidence.

## Decision IDs

- ADR-010 (proposal inbox governance)
- ADR-006 (intent/success alignment weighting)
- ADR-011 (notification noise limits)
