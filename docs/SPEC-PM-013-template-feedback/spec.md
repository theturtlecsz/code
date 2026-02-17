# SPEC-PM-013: Template Feedback Loop (Pattern Promotion)

**Status**: Planned  
**Phase**: Learning Loop (Days 61-90)  
**Owner**: Spec-Kit Platform

## Problem Statement

Successful delivery patterns are discovered repeatedly but are not systematically promoted into reusable templates, causing repeated reinvention.

## Goals

- Identify successful recurring patterns from completed specs.
- Generate candidate template updates with evidence.
- Promote approved patterns into global templates with versioned changelog.

## Non-Goals

- Fully automatic template mutation without approval.
- Replacing manual template authoring entirely.
- Promoting low-confidence or single-run patterns.

## Acceptance Criteria

- **AC1**: Pattern extractor identifies candidate patterns from completed spec/task artifacts.
- **AC2**: Candidates require minimum confidence/evidence thresholds before surfacing.
- **AC3**: Promotion flow requires explicit approval and records decision rationale.
- **AC4**: Approved promotions update template files and changelog with traceable references.
- **AC5**: Regression checks run on updated templates before release.

## Constraints (ADR + Constitution)

- VISION: learning loop should improve future execution quality.
- ADR-010: proposal governance applies to template improvement proposals.
- Constitution: template changes require docs updates and passing tests.

## Interfaces / Behavior

- `TemplatePatternCandidate`:
  - `pattern_id`, `source_specs[]`, `confidence`, `evidence_uris[]`, `proposed_change`
- `TemplatePromotionDecision`:
  - `approved | rejected`, `rationale`, `reviewer`, `timestamp`
- Artifacts:
  - `docs/SPEC-PM-013-template-feedback/artifacts/candidates/<timestamp>.json`
  - `docs/SPEC-PM-013-template-feedback/artifacts/promotions/<timestamp>.md`

## Risk Register

- **Risk**: promotes brittle patterns.  
  **Mitigation**: require multi-spec evidence and regression tests.
- **Risk**: template churn increases complexity.  
  **Mitigation**: batch releases and changelog review gate.
- **Risk**: missing provenance for promoted pattern.  
  **Mitigation**: mandatory source-spec references.

## Decision IDs

- ADR-010 (proposal governance for template candidates)
- ADR-006 (sacred intent alignment preserved)
- Constitution guardrail: template changes require docs + tests
