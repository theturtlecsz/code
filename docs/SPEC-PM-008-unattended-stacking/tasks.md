# Tasks: SPEC-PM-008 Unattended Stacking

## Priority Tasks

- [ ] **T001** Implement attended/unattended state detection and persistence.
  **Validation**: `cd codex-rs && cargo test -p codex-core unattended::state_machine`
  **Artifact**: `docs/SPEC-PM-008-unattended-stacking/artifacts/state/unattended-state.json`

- [ ] **T002** Enforce hard merge disable while unattended.
  **Validation**: `cd codex-rs && cargo test -p codex-core unattended::no_merge`
  **Artifact**: `docs/SPEC-PM-008-unattended-stacking/artifacts/tests/no-merge.txt`

- [ ] **T003** Add bounded unattended queue for research/review work.
  **Validation**: `cd codex-rs && cargo test -p codex-core unattended::queue_bounds`
  **Artifact**: `docs/SPEC-PM-008-unattended-stacking/artifacts/tests/queue-bounds.txt`

- [ ] **T004** Implement Morning Brief assembler with evidence links.
  **Validation**: `cd codex-rs && cargo test -p codex-tui unattended::morning_brief_content`
  **Artifact**: `docs/SPEC-PM-008-unattended-stacking/artifacts/morning-brief/<date>.md`

- [ ] **T005** Surface Morning Brief on session resume and daily recap.
  **Validation**: `cd codex-rs && cargo test -p codex-tui unattended::resume_surface`
  **Artifact**: `docs/SPEC-PM-008-unattended-stacking/artifacts/tests/resume-surface.txt`

- [ ] **T006** Validate docs/policy references.
  **Validation**: `python3 scripts/doc_lint.py`
  **Artifact**: `docs/SPEC-PM-008-unattended-stacking/artifacts/docs/doc-lint.txt`

## Definition of Done

- Unattended mode never merges and still produces useful next-day output.
- Morning Brief is concise, evidence-backed, and deterministic.
- Notification policy remains compliant with ADR-011.
