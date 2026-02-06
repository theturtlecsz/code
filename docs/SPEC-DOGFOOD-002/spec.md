# SPEC-DOGFOOD-002: Gold Run (Pipeline + Evidence)

## Status: PLANNED

## Overview

Canonical dogfood SPEC to validate the `/speckit.auto` pipeline completes end-to-end and produces a complete evidence chain on the local platform (Linux).

This SPEC exists to prove the happy path works in practice (not just in unit/integration tests):

1. All stages complete: Plan → Tasks → Implement → Validate → Audit → Unlock
2. Gates execute at each transition
3. Artifacts/evidence are persisted to the system-of-record (Memvid capsule) with filesystem projections as best-effort
4. `/speckit.verify` reports a coherent pass/fail with actionable diagnostics

Additionally, this SPEC is the canonical dogfood for **assisted maieutic + PM tracking**:

5. Assisted maieutic intake runs in **dry-run** mode (no new spec dirs) but still produces capsule artifacts + scoring output
6. PM lifecycle gates are exercised: Backlog → Planned promotion requires deterministic score ≥ 90/100 and Open Questions empty

## Scope

**In scope**
- Configure Tavily MCP (local pinned) for web research; fallback to default web search if unavailable
- Run assisted maieutic intake in dry-run mode for `SPEC-DOGFOOD-002` and capture:
  - Deterministic score report (numeric gate)
  - Advisory rubric report (persisted)
  - Web research artifact summary (query + sources + hashes/IDs)
- Verify PM status transitions and `codex-rs/SPEC.md` tracker insertion on Planned promotion
- Run `/speckit.auto SPEC-DOGFOOD-002` on Linux
- Verify expected stage artifacts exist under `docs/SPEC-DOGFOOD-002/`
- Verify evidence exists in the capsule and is sufficient for audit/replay (per `docs/PROGRAM.md` and `docs/DECISIONS.md`)
- Capture timing/cost summary if available

**Out of scope**
- Cross-platform compatibility (macOS/Windows)
- New orchestration patterns (committees/voting/etc.)

## Expected Artifacts

```
docs/SPEC-DOGFOOD-002/
├── spec.md
├── plan.md
├── tasks.md
├── implement.md
├── validate.md
├── audit.md
└── unlock.md
```

## Definition of Done

- Assisted intake dry-run completes and produces required artifacts
- Deterministic score is ≥ 90/100 and Open Questions are empty when promoting to `Planned`
- `/speckit.auto SPEC-DOGFOOD-002` completes without manual intervention beyond initiation
- `/speckit.verify SPEC-DOGFOOD-002` reports PASS (or a single well-scoped blocking failure with clear remediation)
- Evidence/artifacts are replay/audit friendly per program DoD (`docs/PROGRAM.md`)

## References

- Canonical tracker: `codex-rs/SPEC.md` (Planned: SPEC-DOGFOOD-002)
- Playbook (needs refresh): `codex-rs/docs/GOLD_RUN_PLAYBOOK.md`
