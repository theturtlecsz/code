# HANDOFF: SPEC-KIT-921 P4 Continuation

**Generated**: 2025-12-21
**Last Commit**: (pending P3 commit)
**Branch**: main

---

## Session Summary

### P3 Completed

| Task | Status | Deliverable |
|------|--------|-------------|
| P3-A: Schema Versioning | ✅ | `schema_version: 1`, `tool_version` in all JSON |
| P3-B: Plan Migration | ✅ | `SpeckitCommand::Plan`, CLI `speckit plan` |
| P3-C: CI Smoke Test | ✅ | Plan→review pipeline test in CI |
| P3-D: Handoff Update | ✅ | This document |

**Tests**: 22 CLI integration tests passing (19 original + 3 plan tests)
**Parity**: TUI and CLI use same `SpeckitExecutor::execute()` for status, review, and plan

### P3 Key Deliverables

1. **Schema Versioning**: All JSON outputs now include:
   ```json
   {
     "schema_version": 1,
     "tool_version": "0.0.0+abc1234"
   }
   ```

2. **Plan Command**: New CLI command for model-free CI validation:
   ```bash
   code speckit plan --spec SPEC-ID --stage plan --dry-run --json
   ```

3. **Outcome Variants**: Added `Outcome::PlanReady` and `Outcome::PlanBlocked`:
   - `PlanReady`: Validation passed, ready for agent execution (TUI spawns agents)
   - `PlanBlocked`: Validation failed (exit 2)

4. **CI Smoke Test**: Pipeline validates plan→review workflow with fixture data

---

## P4 Scope (Recommended)

### Strategy: Stage Expansion

```
plan stage (P3) → tasks stage → implement stage → full pipeline
```

**Rationale**: With plan stage proven behind executor, expand to remaining stages incrementally.

### P4 Tasks

#### P4-A: Migrate /speckit.tasks Behind Executor

Same pattern as plan:
1. Add `SpeckitCommand::Tasks` variant (or reuse `Plan` with stage=Tasks)
2. Implement validation logic in executor
3. Add CLI `speckit plan --stage tasks` tests
4. Update CI with tasks stage smoke test

Files to modify:
- `cli/tests/speckit.rs`: Add tasks-specific tests
- `.github/workflows/spec-kit-ci.yml`: Add tasks stage test

#### P4-B: Migrate /speckit.implement Behind Executor

Same pattern as tasks:
1. Reuse `SpeckitCommand::Plan` with stage=Implement
2. Add implementation-specific validation (e.g., check tasks.md exists)
3. Add CLI tests for implement stage
4. Update CI with implement stage smoke test

#### P4-C: TUI Integration (Optional)

Refactor TUI `/speckit.plan` to use executor:
1. Call `executor.execute(SpeckitCommand::Plan { ... })`
2. On `PlanReady`: spawn agents via existing orchestrator
3. On `PlanBlocked`: display errors in chat

Note: This is optional because TUI already has its own guardrail logic. Full migration can be deferred if TUI works fine.

#### P4-D: Update Documentation

- Document `speckit plan` command in help
- Update HANDOFF.md for P5 (full pipeline orchestration)

---

## Files Reference

### Modified in P3

| File | Purpose |
|------|---------|
| `cli/src/speckit_cmd.rs` | Plan subcommand, schema versioning |
| `cli/tests/speckit.rs` | 22 integration tests (3 new) |
| `spec-kit/src/executor/command.rs` | Plan variant, parse_plan() |
| `spec-kit/src/executor/mod.rs` | execute_plan(), PlanReady/PlanBlocked |
| `tui/src/chatwidget/spec_kit/command_handlers.rs` | Exhaustive match for Plan outcomes |
| `.github/workflows/spec-kit-ci.yml` | Plan smoke test |

### Architecture After P3

```
CLI (speckit_cmd.rs)
  ↓ SpeckitCommand::Status/Review/Plan
  ↓
SpeckitExecutor::execute()
  ↓
Outcome::Status/Review/ReviewSkipped/PlanReady/PlanBlocked/Error
  ↓
render_*_dashboard() → stdout (with schema_version)

TUI (command_handlers.rs)
  ↓ SpeckitCommand::parse_*()
  ↓
SpeckitExecutor::execute()  ← SAME EXECUTOR
  ↓
Outcome::*
  ↓
render_*_dashboard() → ChatWidget
  ↓ (if PlanReady)
agent_orchestrator::auto_submit_spec_stage_prompt()  ← TUI-only
```

---

## Exit Code Contract (Reference)

| Code | Meaning | Trigger |
|------|---------|---------|
| 0 | Success/proceed | Clean consensus, plan ready |
| 1 | Soft fail | PassedWithWarnings + --strict-warnings |
| 2 | Hard fail | Conflicts, escalation, PlanBlocked |
| 3 | Infrastructure error | Parse/schema errors + --strict-schema |

---

## Deferred to P5+

- **Full /speckit.auto orchestration**: After all stages behind executor
- **CLI config-file support** (`.speckit.toml`): After Phase C/D requirements clarify
- **Evidence root auto-discovery**: Complexity multiplier; only after pipeline migration

---

## Start Commands

```bash
# Verify current state
cd /home/thetu/code
git log --oneline -5
cargo test -p codex-cli --test speckit

# Verify P3 deliverables
cargo run -p codex-cli -- speckit plan --spec SPEC-TEST --dry-run --json

# Start P4-A (tasks stage)
```

---

## Acceptance Criteria for P4

- [ ] Tasks stage (`--stage tasks`) validates through executor
- [ ] Implement stage (`--stage implement`) validates through executor
- [ ] CI smoke tests for tasks and implement stages
- [ ] 25+ CLI integration tests passing
- [ ] No regression in existing plan/review/status commands
