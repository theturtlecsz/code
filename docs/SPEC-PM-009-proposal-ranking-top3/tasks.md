# Tasks: SPEC-PM-009 Proposal Ranking

## Priority Tasks

- [ ] **T001** Define ranking schema and weighted scoring function.
  **Validation**: `cd codex-rs && cargo test -p codex-core proposal::scoring`
  **Artifact**: `docs/SPEC-PM-009-proposal-ranking-top3/artifacts/schema/proposal-score.json`

- [ ] **T002** Implement scoring explanations for each proposal.
  **Validation**: `cd codex-rs && cargo test -p codex-core proposal::explainability`
  **Artifact**: `docs/SPEC-PM-009-proposal-ranking-top3/artifacts/ranking/sample-score.json`

- [ ] **T003** Implement top-3 default view and top-10 expansion.
  **Validation**: `cd codex-rs && cargo test -p codex-tui proposal::top3_view`
  **Artifact**: `docs/SPEC-PM-009-proposal-ranking-top3/artifacts/tests/top3-view.txt`

- [ ] **T004** Implement duplicate merge with provenance retention.
  **Validation**: `cd codex-rs && cargo test -p codex-core proposal::dedupe`
  **Artifact**: `docs/SPEC-PM-009-proposal-ranking-top3/artifacts/tests/dedupe.txt`

- [ ] **T005** Implement 7-day archive job with pin override.
  **Validation**: `cd codex-rs && cargo test -p codex-core proposal::archive_ttl`
  **Artifact**: `docs/SPEC-PM-009-proposal-ranking-top3/artifacts/archive/<date>.json`

- [ ] **T006** Run docs quality gates.
  **Validation**: `python3 scripts/doc_lint.py`
  **Artifact**: `docs/SPEC-PM-009-proposal-ranking-top3/artifacts/docs/doc-lint.txt`

## Definition of Done

- Inbox defaults to top 3 with transparent scoring.
- Deduplication and archival keep proposal volume bounded.
- Users can still inspect top 10 and provenance details.
