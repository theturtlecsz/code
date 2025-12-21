# HANDOFF: SPEC-KIT-921 P4 Continuation

**Generated**: 2025-12-21
**Last Commit**: b27417cfd (feat(spec-kit): implement P3 schema versioning and plan command)
**Branch**: main

---

## Continue P4 Implementation

Copy this prompt to start the next session:

```
# Continue SPEC-KIT-921 P4 Implementation

Continue from docs/spec-kit/HANDOFF.md (commit b27417cfd)

## Context
- P0-P3 complete (commits through b27417cfd)
- 22 CLI tests passing, CI workflow in place
- Plan stage behind executor with PlanReady/PlanBlocked outcomes
- Architect approved: Tasks stage + StageOutcome envelope + TUI parity

## P4 Tasks (in order)

### P4-A: Remove --stage flag from speckit plan
**HARD REQUIREMENT**: `speckit plan` must not accept `--stage` other than `plan`.
- Remove `--stage` flag entirely from PlanArgs, OR
- Accept only `--stage plan`, reject others with clear error
- Update tests accordingly

Files:
- cli/src/speckit_cmd.rs (PlanArgs struct)
- cli/tests/speckit.rs (update plan tests)

### P4-B: Introduce shared StageOutcome envelope
Replace per-stage outcomes (PlanReady/PlanBlocked) with generic envelope:

```rust
pub struct StageOutcome {
    pub spec_id: String,
    pub stage: Stage,
    pub checkpoint: Option<Checkpoint>,
    pub resolution: StageResolution, // Ready | Blocked | Skipped
    pub blocking_reasons: Vec<String>,
    pub advisory_signals: Vec<String>,
    pub evidence_refs: Option<EvidenceRefs>,
    pub dry_run: bool,
}

pub enum StageResolution {
    Ready,      // Validation passed, proceed
    Blocked,    // Validation failed, needs intervention
    Skipped,    // Stage not applicable
}
```

Keep `StageReviewResult` separate (review has extra semantics).
Share common sub-structs (signals, evidence refs) to avoid duplication.

Files:
- spec-kit/src/executor/mod.rs (add StageOutcome, refactor Outcome enum)
- cli/src/speckit_cmd.rs (update run_plan to use StageOutcome)
- tui/src/chatwidget/spec_kit/command_handlers.rs (update match arms)

### P4-C: Add speckit tasks CLI command
New subcommand following plan pattern:

```bash
speckit tasks --spec <ID> [--dry-run] [--json]
```

- Add SpeckitCommand::Tasks variant (or reuse Plan with fixed stage)
- Implement execute_tasks() with validation logic
- Add TasksArgs struct and run_tasks() in CLI
- Add CLI tests

Files:
- spec-kit/src/executor/command.rs (add Tasks variant if needed)
- spec-kit/src/executor/mod.rs (add execute_tasks or extend execute_plan)
- cli/src/speckit_cmd.rs (add Tasks subcommand)
- cli/tests/speckit.rs (add tasks tests)

### P4-D: TUI integration for Plan and Tasks
Refactor TUI slash commands to use executor:

1. `/speckit.plan SPEC-ID`:
   - Parse → SpeckitCommand::Plan
   - Execute → StageOutcome
   - If Ready: spawn agents via orchestrator
   - If Blocked: display errors in chat

2. `/speckit.tasks SPEC-ID`:
   - Same pattern as plan

**Do NOT move agent execution into executor.** Executor decides readiness;
TUI orchestrator handles interactive model/tool loops.

Files:
- tui/src/chatwidget/spec_kit/commands/plan.rs (refactor to use executor)
- tui/src/chatwidget/spec_kit/command_handlers.rs (add handle_tasks if needed)

### P4-E: CI smoke test for Tasks
Add to spec-kit-ci.yml:

```yaml
- name: "[TASKS] Validate tasks dry-run exits 0"
  run: |
    $CLI speckit tasks --spec SPEC-CI-001-clean --dry-run --json

- name: "[PIPELINE] plan → tasks → review smoke test"
  run: |
    $CLI speckit plan --spec SPEC-CI-001-clean --dry-run
    $CLI speckit tasks --spec SPEC-CI-001-clean --dry-run
    $CLI speckit review --spec SPEC-CI-001-clean --stage tasks --json
```

### P4-F: Update HANDOFF.md for P5

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

## Acceptance Criteria for P4

- [ ] `speckit plan --stage X` rejects non-plan stages (or flag removed)
- [ ] StageOutcome envelope replaces PlanReady/PlanBlocked
- [ ] `speckit tasks` CLI command exists with same exit codes
- [ ] TUI `/speckit.plan` routes through executor
- [ ] TUI `/speckit.tasks` routes through executor
- [ ] CI smoke: plan→tasks→review pipeline passes
- [ ] 25+ CLI integration tests passing
- [ ] No ChatWidget coupling in stage validation logic
```

---

## Session Summary (P3 Completed)

### P3 Deliverables

| Task | Status | Deliverable |
|------|--------|-------------|
| P3-A: Schema Versioning | ✅ | `schema_version: 1`, `tool_version` in all JSON |
| P3-B: Plan Migration | ✅ | `SpeckitCommand::Plan`, CLI `speckit plan` |
| P3-C: CI Smoke Test | ✅ | Plan→review pipeline test in CI |
| P3-D: Handoff Update | ✅ | This document |

**Tests**: 22 CLI integration tests passing
**Parity**: TUI and CLI use same `SpeckitExecutor::execute()` for status, review

### Architect Review Feedback (Applied to P4 Scope)

1. **CLI Rename Decision**: Keep separate commands (`speckit plan`, `speckit tasks`), not `speckit run --stage`. Remove/lock the `--stage` flag on plan command.

2. **StageOutcome Envelope**: Introduce now while refactor cost is low. Prevents outcome proliferation across 6+ stages.

3. **TUI Integration**: Required for parity. TUI must route through executor to prevent drift.

4. **Stage Scope**: Tasks only. Prove pattern before expanding to Implement.

---

## Architecture After P4 (Target)

```
CLI (speckit_cmd.rs)
  ↓ SpeckitCommand::Status/Review/Plan/Tasks
  ↓
SpeckitExecutor::execute()
  ↓
Outcome::Status/Review/ReviewSkipped/Stage(StageOutcome)/Error
  ↓
render_*() → stdout (with schema_version)

TUI (command_handlers.rs + commands/plan.rs)
  ↓ SpeckitCommand::parse_*()
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

### Modified in P3

| File | Purpose |
|------|---------|
| `cli/src/speckit_cmd.rs` | Plan subcommand, schema versioning |
| `cli/tests/speckit.rs` | 22 integration tests |
| `spec-kit/src/executor/command.rs` | Plan variant, parse_plan() |
| `spec-kit/src/executor/mod.rs` | execute_plan(), PlanReady/PlanBlocked |
| `tui/src/chatwidget/spec_kit/command_handlers.rs` | Exhaustive match |
| `.github/workflows/spec-kit-ci.yml` | Plan smoke test |

### To Modify in P4

| File | Change |
|------|--------|
| `cli/src/speckit_cmd.rs` | Remove --stage from plan, add tasks subcommand |
| `spec-kit/src/executor/mod.rs` | StageOutcome envelope, execute_tasks() |
| `tui/src/chatwidget/spec_kit/commands/plan.rs` | Route through executor |
| `.github/workflows/spec-kit-ci.yml` | Tasks smoke test |

---

## Invariants to Preserve

1. **No env/config reads in executor core** — resolve at adapter boundary
2. **All paths repo-relative** in JSON output and evidence refs
3. **Deterministic evidence selection** — tests cover ordering/timestamp logic
4. **Strict modes behave predictably**: artifacts→2, schema→3, escalation→2
5. **TUI is an adapter** — slash commands call executor, not legacy implementations

---

## Deferred to P5+

- **Implement stage migration**: After Tasks proven
- **Full /speckit.auto orchestration**: After all stages behind executor
- **CLI config-file support** (`.speckit.toml`): After Phase C/D requirements clarify
- **Generic `speckit run` command**: After 3+ stages migrated, as optional alias
