# HANDOFF: SPEC-KIT-921 P5 Continuation

**Generated**: 2025-12-21
**Last Commit**: (pending P4 commit)
**Branch**: main

---

## Continue P5 Implementation

Copy this prompt to start the next session:

```
# Continue SPEC-KIT-921 P5 Implementation

Continue from docs/spec-kit/HANDOFF.md

## Context
- P0-P4 complete
- 25 CLI tests passing, CI workflow with plan/tasks smoke tests
- StageOutcome envelope replaces PlanReady/PlanBlocked
- TUI routes /speckit.plan and /speckit.tasks through executor
- Architect approved: Implement stage migration + remaining stages

## P5 Tasks (in order)

### P5-A: Add speckit implement CLI command
New subcommand following plan/tasks pattern:

```bash
speckit implement --spec <ID> [--dry-run] [--json]
```

Files:
- cli/src/speckit_cmd.rs (add Implement subcommand)
- cli/tests/speckit.rs (add implement tests)

### P5-B: Add remaining stage commands
Add validate, audit, unlock CLI commands:

```bash
speckit validate --spec <ID> [--dry-run] [--json]
speckit audit --spec <ID> [--dry-run] [--json]
speckit unlock --spec <ID> [--dry-run] [--json]
```

Files:
- cli/src/speckit_cmd.rs (add subcommands)
- cli/tests/speckit.rs (add tests)

### P5-C: Refactor TUI for all stages
Ensure all stage commands route through executor:
- /speckit.implement
- /speckit.validate
- /speckit.audit
- /speckit.unlock

Files:
- tui/src/chatwidget/spec_kit/commands/plan.rs

### P5-D: CI smoke tests for all stages
Add smoke tests for full pipeline:

```yaml
- name: "[PIPELINE] full stage progression test"
  run: |
    $CLI speckit plan --spec SPEC-CI-001-clean --dry-run
    $CLI speckit tasks --spec SPEC-CI-001-clean --dry-run
    $CLI speckit implement --spec SPEC-CI-001-clean --dry-run
    $CLI speckit validate --spec SPEC-CI-001-clean --dry-run
    $CLI speckit audit --spec SPEC-CI-001-clean --dry-run
    $CLI speckit unlock --spec SPEC-CI-001-clean --dry-run
```

### P5-E: Update HANDOFF.md for P6

## Start Commands

```bash
git log --oneline -3
cargo test -p codex-cli --test speckit
```

## Exit Code Contract (Reference)

| Code | Meaning              |
|------|----------------------|
| 0    | Success / Ready      |
| 2    | Blocked / Escalation |
| 3    | Infrastructure error |

## Acceptance Criteria for P5

- [ ] `speckit implement` CLI command exists
- [ ] `speckit validate/audit/unlock` CLI commands exist
- [ ] TUI all stage commands route through executor
- [ ] CI smoke: full pipeline test passes
- [ ] 30+ CLI integration tests passing
```

---

## Session Summary (P4 Completed)

### P4 Deliverables

| Task | Status | Deliverable |
|------|--------|-------------|
| P4-A: Remove --stage flag | ✅ | `speckit plan` no longer accepts `--stage` |
| P4-B: StageOutcome envelope | ✅ | `StageOutcome` struct with Ready/Blocked/Skipped |
| P4-C: Tasks CLI command | ✅ | `speckit tasks --spec <ID> [--dry-run] [--json]` |
| P4-D: TUI integration | ✅ | `/speckit.plan` and `/speckit.tasks` route through executor |
| P4-E: CI smoke test | ✅ | plan→tasks→review pipeline test |
| P4-F: Handoff update | ✅ | This document |

**Tests**: 25 CLI integration tests passing
**Parity**: TUI and CLI use same `SpeckitExecutor::execute()` for status, review, plan, tasks

### Key Changes in P4

1. **Removed --stage from plan command**: Plan command is locked to Stage::Plan
2. **StageOutcome envelope**: Generic outcome for all stage validation
   - `StageResolution::Ready` - validation passed
   - `StageResolution::Blocked` - validation failed
   - `StageResolution::Skipped` - stage not applicable
3. **Tasks command**: New CLI command following plan pattern
4. **TUI executor integration**: Stage commands now route through executor

---

## Architecture After P4 (Current)

```
CLI (speckit_cmd.rs)
  ↓ SpeckitSubcommand::Status/Review/Plan/Tasks
  ↓
SpeckitExecutor::execute()
  ↓
Outcome::Status/Review/ReviewSkipped/Stage(StageOutcome)/Error
  ↓
render_*() → stdout (with schema_version)

TUI (commands/plan.rs)
  ↓ execute_stage_command()
  ↓
SpeckitExecutor::execute()  ← SAME EXECUTOR
  ↓
Outcome::Stage(StageOutcome)
  ↓
If Ready: agent_orchestrator::auto_submit_spec_stage_prompt()
If Blocked: display errors in chat
```

---

## Files Reference

### Modified in P4

| File | Purpose |
|------|---------|
| `cli/src/speckit_cmd.rs` | Tasks subcommand, removed --stage, StageResolution handling |
| `cli/tests/speckit.rs` | 25 integration tests (3 new for tasks) |
| `spec-kit/src/executor/mod.rs` | StageOutcome, StageResolution, refactored execute_plan |
| `tui/src/chatwidget/spec_kit/commands/plan.rs` | Executor integration |
| `tui/src/chatwidget/spec_kit/command_handlers.rs` | Updated match arms |
| `.github/workflows/spec-kit-ci.yml` | Tasks smoke test, plan→tasks→review pipeline |

### To Modify in P5

| File | Change |
|------|--------|
| `cli/src/speckit_cmd.rs` | Add implement/validate/audit/unlock subcommands |
| `cli/tests/speckit.rs` | Add tests for all remaining stages |
| `.github/workflows/spec-kit-ci.yml` | Full pipeline smoke test |

---

## Invariants to Preserve

1. **No env/config reads in executor core** — resolve at adapter boundary
2. **All paths repo-relative** in JSON output and evidence refs
3. **Deterministic evidence selection** — tests cover ordering/timestamp logic
4. **Strict modes behave predictably**: artifacts→2, schema→3, escalation→2
5. **TUI is an adapter** — slash commands call executor, not legacy implementations
6. **Stage-specific commands** — each stage has its own CLI command (no generic `--stage` flag)

---

## Deferred to P6+

- **Full /speckit.auto orchestration**: After all stages behind executor
- **CLI config-file support** (`.speckit.toml`): After Phase C/D requirements clarify
- **Generic `speckit run` command**: After all stages migrated, as optional alias
- **Checkpoint persistence**: After orchestration proven
