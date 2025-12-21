# HANDOFF: SPEC-KIT-921 P5 Continuation

**Generated**: 2025-12-21
**Last Commit**: b7549eb99 (docs: update HANDOFF.md for P5 continuation)
**Branch**: main

---

## Continue P5 Implementation

Copy this prompt to start the next session:

```
# Continue SPEC-KIT-921 P5 Implementation

Continue from docs/spec-kit/HANDOFF.md

## Context
- P0-P4 complete (commits c127dee66, 1f216920a)
- 25 CLI tests passing, CI workflow with plan/tasks smoke tests
- StageOutcome envelope with Ready/Blocked/Skipped resolution
- SpeckitCommand::ValidateStage is stage-neutral (handles all stages)
- TUI routes /speckit.plan and /speckit.tasks through executor
- Architectural review complete: naming, dry_run preservation, build-time determinism

## Key Types Reference

```rust
// spec-kit/src/executor/command.rs
SpeckitCommand::ValidateStage { spec_id, stage, dry_run }

// spec-kit/src/executor/mod.rs
StageOutcome { spec_id, stage, resolution, blocking_reasons, advisory_signals, evidence_refs, dry_run }
StageResolution::Ready | Blocked | Skipped
```

## P5 Tasks (in order)

### P5-A: Add speckit implement CLI command
New subcommand following plan/tasks pattern:

```bash
speckit implement --spec <ID> [--dry-run] [--json]
```

Implementation:
1. Add ImplementArgs struct to cli/src/speckit_cmd.rs
2. Add run_implement() following run_tasks() pattern
3. Add to SpeckitSubcommand enum
4. Add 3 tests: validates_spec_exits_0, warns_when_tasks_missing, json_includes_schema_version

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

Each follows same pattern: Args struct, run_* function, 3 tests.

Files:
- cli/src/speckit_cmd.rs (add subcommands)
- cli/tests/speckit.rs (add tests per stage)

NOTE: `speckit specify` deferred to P6 (scope creep risk, different UX expectations)

### P5-C: TUI already routes all stages
Verify existing TUI routing:
- /speckit.implement already in plan.rs
- /speckit.validate already in plan.rs
- /speckit.audit already in plan.rs
- /speckit.unlock already in plan.rs

Files:
- tui/src/chatwidget/spec_kit/commands/plan.rs (verify only)

### P5-D: CI smoke tests for all stages
Add smoke tests for full pipeline (6 stages, specify deferred):

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
git log --oneline -5
cargo test -p codex-cli --test speckit
cat codex-rs/cli/src/speckit_cmd.rs | head -100  # Understand current structure
```

## Exit Code Contract (Reference)

| Code | Meaning              |
|------|----------------------|
| 0    | Success / Ready      |
| 2    | Blocked / Escalation |
| 3    | Infrastructure error |

## Acceptance Criteria for P5

- [ ] `speckit implement` CLI command exists with 3 tests
- [ ] `speckit validate` CLI command exists with 3 tests
- [ ] `speckit audit` CLI command exists with 3 tests
- [ ] `speckit unlock` CLI command exists with 3 tests
- [ ] TUI all stage commands verified routing through executor
- [ ] CI smoke: full 6-stage pipeline test passes (plan→tasks→implement→validate→audit→unlock)
- [ ] 37+ CLI integration tests passing (25 current + 12 new = 37)

## P5 Architectural Decisions

1. **Defer `speckit specify` to P6**: First stage has different UX expectations and scope creep risk
2. **Advisory signals only**: Stages add advisory warnings for missing prerequisites (e.g., "no tasks found"), but do NOT hard-block
3. **Defer `speckit run` batch to P6**: Complete individual commands first; batch orchestration has complex edge cases
4. **Single commit**: All 4 stage commands in one atomic commit for easier atomic rollback
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

### P4 Architectural Fixes (commit 1f216920a)

| Issue | Fix |
|-------|-----|
| P0: "consensus" vocabulary | Changed to "gate review" in TUI descriptions |
| P1-A: Misleading naming | `SpeckitCommand::Plan` → `ValidateStage` |
| P1-B: dry_run lost on Blocked/Skipped | Added dry_run param to blocked()/skipped() constructors |
| P1-C: Runtime git in tool_version | Now build-time only via `SPECKIT_GIT_SHA`/`GIT_SHA` env vars |
| P2: Lossy stage mapping | `spec_stage_to_stage()` returns `Option<Stage>`, None for quality commands |

---

## Architecture After P4 (Current)

```
CLI (speckit_cmd.rs)
  ↓ SpeckitSubcommand::Status/Review/Plan/Tasks
  ↓ → SpeckitCommand::ValidateStage { spec_id, stage, dry_run }
  ↓
SpeckitExecutor::execute()
  ↓
Outcome::Status/Review/ReviewSkipped/Stage(StageOutcome)/Error
  ↓
render_*() → stdout (with schema_version)

TUI (commands/plan.rs)
  ↓ execute_stage_command()
  ↓ → SpeckitCommand::ValidateStage { spec_id, stage, dry_run: false }
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
| `cli/src/speckit_cmd.rs` | Tasks subcommand, removed --stage, StageResolution handling, build-time tool_version |
| `cli/tests/speckit.rs` | 25 integration tests (3 new for tasks) |
| `spec-kit/src/executor/mod.rs` | StageOutcome, StageResolution, execute_validate_stage |
| `spec-kit/src/executor/command.rs` | Renamed Plan→ValidateStage, parse_validate_stage |
| `tui/src/chatwidget/spec_kit/commands/plan.rs` | Executor integration, gate review descriptions, Option<Stage> mapping |
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
7. **ValidateStage is stage-neutral** — single command variant handles all stage validation
8. **Build-time determinism** — tool_version uses compile-time env vars only, no runtime git

---

## Deferred to P6+

- **`speckit specify` command**: First stage has different UX expectations (template selection, spec materialization)
- **`speckit run` batch command**: Sequential execution with `--from`/`--to` flags
- **Full /speckit.auto orchestration**: After all stages behind executor
- **CLI config-file support** (`.speckit.toml`): After Phase C/D requirements clarify
- **Checkpoint persistence**: After orchestration proven
- **Prerequisite hard-blocking**: Currently advisory-only; hard-blocking requires defining "completed" semantics
