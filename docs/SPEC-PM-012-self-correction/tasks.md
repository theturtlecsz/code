# Tasks: SPEC-PM-012 Self-Correction

## Priority Tasks

- [ ] **T001** Define retry context and failure signature schema.
  **Validation**: `cd codex-rs && cargo test -p codex-core retry::schema`
  **Artifact**: `docs/SPEC-PM-012-self-correction/artifacts/schema/retry-context.json`

- [ ] **T002** Implement bounded retry loop orchestration.
  **Validation**: `cd codex-rs && cargo test -p codex-core retry::bounded_loop`
  **Artifact**: `docs/SPEC-PM-012-self-correction/artifacts/tests/bounded-loop.txt`

- [ ] **T003** Inject failure-context feedback into subsequent retries.
  **Validation**: `cd codex-rs && cargo test -p codex-core retry::context_feedback`
  **Artifact**: `docs/SPEC-PM-012-self-correction/artifacts/tests/context-feedback.txt`

- [ ] **T004** Add loop guard for unchanged deterministic failures.
  **Validation**: `cd codex-rs && cargo test -p codex-core retry::signature_guard`
  **Artifact**: `docs/SPEC-PM-012-self-correction/artifacts/tests/signature-guard.txt`

- [ ] **T005** Generate escalation packet after max attempts.
  **Validation**: `cd codex-rs && cargo test -p codex-core retry::escalation_packet`
  **Artifact**: `docs/SPEC-PM-012-self-correction/artifacts/escalations/<run_id>.md`

- [ ] **T006** Validate docs and command references.
  **Validation**: `python3 scripts/doc_lint.py`
  **Artifact**: `docs/SPEC-PM-012-self-correction/artifacts/docs/doc-lint.txt`

## Definition of Done

- Agents retry safely before escalating.
- Escalations are concise and actionable.
- No unbounded or repetitive retry loops occur.
