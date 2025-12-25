# Spec: SPEC-DOGFOOD-001 Golden Path Dogfooding Validation

## Context

- The code TUI (`~/code`) is ready for dogfooding as the default development workflow, with all P0 blockers resolved in Session 14.
- Stage0 infrastructure is in place: local-memory daemon healthy, NotebookLM service authenticated and ready with 5 core documentation sources.
- Configuration (`~/.config/codex/stage0.toml`) exists with Tier2 enabled, pointing to notebook `code-project-docs` (ID: `4e80974f-789d-43bd-abe9-7b1e76839506`).
- This spec validates the full golden path: `/speckit.auto` invoking Stage0 with Tier1 (local-memory) + Tier2 (NotebookLM) to produce evidence artifacts.

## Objectives

1. **Validate Tier2 Integration**: Confirm NotebookLM is queried during Stage0 execution and contributes to Divine Truth synthesis.
2. **Verify Evidence Production**: Ensure `TASK_BRIEF.md` and `DIVINE_TRUTH.md` artifacts are generated in the spec evidence directory.
3. **Confirm System Pointer Storage**: Validate that Stage0 stores a system pointer memory in local-memory with `system:true` tag.
4. **Demonstrate End-to-End Flow**: Run `/speckit.auto SPEC-DOGFOOD-001` and observe successful pipeline completion.

## Scope

- Run `code doctor` to verify Stage0 health checks pass (local-memory, NotebookLM, notebook-mapping all OK).
- Execute `/speckit.auto SPEC-DOGFOOD-001` within the TUI.
- Examine Stage0 logs/output for `tier2_used=true` indicator.
- Verify evidence artifacts exist in `docs/SPEC-DOGFOOD-001/evidence/`.
- Query local-memory for system pointer artifact related to this spec.

## Non-Goals

- Validating downstream stages (1-6) of the spec-kit pipeline; focus is Stage0 only.
- Performance benchmarking or Tier2 cache optimization.
- Comprehensive NotebookLM source seeding beyond core docs already added.
- Code changes to Stage0 engine; this is a validation spec, not implementation.

## Acceptance Criteria

| ID | Criterion | Verification Command |
|----|-----------|---------------------|
| A1 | Doctor Ready | `code doctor` shows all [OK], no stage0.toml warning |
| A2 | Tier2 Used | `/speckit.auto SPEC-DOGFOOD-001` logs show `tier2_used=true` or similar indicator |
| A3 | Evidence Exists | `ls docs/SPEC-DOGFOOD-001/evidence/` contains `TASK_BRIEF.md` and/or `DIVINE_TRUTH.md` |
| A4 | System Pointer | `lm search "SPEC-DOGFOOD-001"` returns memory with `system:true` tag |

## Dependencies

| Dependency | Status | Notes |
|------------|--------|-------|
| local-memory daemon | OK | Verified via `lm health` |
| NotebookLM service | OK | Verified via health endpoint |
| stage0.toml | OK | Created at `~/.config/codex/stage0.toml` |
| NotebookLM sources | OK | 5 sources in `code-project-docs` notebook |

## Risks

| Risk | Mitigation |
|------|------------|
| NotebookLM rate limiting | Tier2 fails closed; Tier1 continues |
| Memory pressure on service | Monitor via health endpoint; service auto-recovers |
| Stage0 engine not wired | Verify via logs; escalate if Stage0 is skipped entirely |

## Success Metrics

- All 4 acceptance criteria pass on first execution.
- No manual intervention required during pipeline run.
- Evidence artifacts are human-readable and contain synthesized context from project docs.
