# HANDOFF: SPEC-KIT-921 P6 Continuation

**Generated**: 2025-12-21
**Last Commit**: (pending P5 commit)
**Branch**: main

---

## Continue P6 Implementation

Copy this prompt to start the next session:

```
# Continue SPEC-KIT-921 P6 Implementation

Continue from docs/spec-kit/HANDOFF.md

## Context
- P0-P5 complete
- 37 CLI tests passing (25 original + 12 new for implement/validate/audit/unlock)
- All 6 stage commands have CLI support (plan, tasks, implement, validate, audit, unlock)
- TUI already routes all stage commands through executor
- Full 6-stage CI pipeline test added
- speckit specify deferred (different UX expectations)

## Key Types Reference

```rust
// spec-kit/src/executor/command.rs
SpeckitCommand::ValidateStage { spec_id, stage, dry_run }

// spec-kit/src/executor/mod.rs
StageOutcome { spec_id, stage, resolution, blocking_reasons, advisory_signals, evidence_refs, dry_run }
StageResolution::Ready | Blocked | Skipped
```

## P6 Tasks (in order)

### P6-A: Add speckit specify CLI command

First stage has different UX expectations:
- May need template selection/materialization
- Creates new SPEC directory structure
- Different validation semantics (no prereqs)

```bash
speckit specify --spec <ID> [--template <NAME>] [--dry-run] [--json]
```

Implementation:
1. Add SpecifyArgs struct to cli/src/speckit_cmd.rs
2. Add run_specify() with template handling
3. Add to SpeckitSubcommand enum
4. Add 3 tests: validates_spec_exits_0, creates_spec_dir, json_includes_schema_version

Files:
- cli/src/speckit_cmd.rs (add Specify subcommand)
- cli/tests/speckit.rs (add specify tests)

### P6-B: Add speckit run batch command

Sequential execution with stage range:

```bash
speckit run --spec <ID> [--from <STAGE>] [--to <STAGE>] [--dry-run] [--json]
```

Executes stages in order, stopping on first Blocked result.

Edge cases:
- --from without --to: run from specified stage to unlock
- --to without --from: run from specify to specified stage
- Neither: run full pipeline (specify→unlock)
- Invalid stage order: error

Implementation:
1. Add RunArgs struct with from/to stage options
2. Add run_batch() that iterates stages
3. Add comprehensive tests for stage range handling

### P6-C: Prerequisite hard-blocking option

Currently prereqs are advisory-only. Add --strict-prereqs flag:

```bash
speckit implement --spec <ID> --strict-prereqs [--dry-run] [--json]
```

With --strict-prereqs:
- Missing plan.md for implement → exit 2 (Blocked)
- Missing tasks.md for validate → exit 2 (Blocked)

### P6-D: Update CI and HANDOFF.md for P7

## Start Commands

```bash
git log --oneline -5
cargo test -p codex-cli --test speckit
cat codex-rs/cli/src/speckit_cmd.rs | head -100
```

## Exit Code Contract (Reference)

| Code | Meaning              |
|------|----------------------|
| 0    | Success / Ready      |
| 2    | Blocked / Escalation |
| 3    | Infrastructure error |

## Acceptance Criteria for P6

- [ ] `speckit specify` CLI command exists with 3 tests
- [ ] `speckit run` batch command exists with 5+ tests
- [ ] --strict-prereqs flag available on all stage commands
- [ ] 45+ CLI integration tests passing
- [ ] CI smoke: specify added to pipeline test
```

---

## Session Summary (P5 Completed)

### P5 Deliverables

| Task | Status | Deliverable |
|------|--------|-------------|
| P5-A: Implement CLI command | ✅ | `speckit implement --spec <ID> [--dry-run] [--json]` with 3 tests |
| P5-B: Validate CLI command | ✅ | `speckit validate --spec <ID> [--dry-run] [--json]` with 3 tests |
| P5-B: Audit CLI command | ✅ | `speckit audit --spec <ID> [--dry-run] [--json]` with 3 tests |
| P5-B: Unlock CLI command | ✅ | `speckit unlock --spec <ID> [--dry-run] [--json]` with 3 tests |
| P5-C: TUI routing verification | ✅ | All 6 stage commands route through executor |
| P5-D: CI 6-stage pipeline test | ✅ | plan→tasks→implement→validate→audit→unlock smoke test |
| P5-E: Handoff update | ✅ | This document |

**Tests**: 37 CLI integration tests passing (25 + 12 new)
**Parity**: TUI and CLI use same `SpeckitExecutor::execute()` for all commands

### Key Changes in P5

1. **Four new CLI stage commands**: implement, validate, audit, unlock
2. **Each follows existing pattern**: Args struct, run_* function, 3 tests
3. **Advisory prereq warnings**: Missing prereqs warn but don't block
4. **Full CI pipeline**: Tests all 6 stages in sequence
5. **specify deferred**: Different UX expectations warrant separate implementation

---

## Architecture After P5 (Current)

```
CLI (speckit_cmd.rs)
  ↓ SpeckitSubcommand::Status/Review/Plan/Tasks/Implement/Validate/Audit/Unlock
  ↓ → SpeckitCommand::ValidateStage { spec_id, stage, dry_run }
  ↓
SpeckitExecutor::execute()
  ↓
Outcome::Status/Review/ReviewSkipped/Stage(StageOutcome)/Error
  ↓
render_*() → stdout (with schema_version)

TUI (commands/plan.rs)
  ↓ execute_stage_command() for all stages
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

### Modified in P5

| File | Purpose |
|------|---------|
| `cli/src/speckit_cmd.rs` | Added Implement/Validate/Audit/Unlock subcommands |
| `cli/tests/speckit.rs` | 37 integration tests (12 new for P5 stage commands) |
| `.github/workflows/spec-kit-ci.yml` | Full 6-stage pipeline smoke test |

### To Modify in P6

| File | Change |
|------|--------|
| `cli/src/speckit_cmd.rs` | Add Specify and Run subcommands |
| `cli/tests/speckit.rs` | Add tests for specify and run commands |
| `spec-kit/src/executor/mod.rs` | Add --strict-prereqs support |
| `.github/workflows/spec-kit-ci.yml` | Add specify to pipeline test |

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
9. **Advisory prereq warnings** — missing prereqs warn but don't block (unless --strict-prereqs)

---

## Deferred to P7+

- **Full /speckit.auto orchestration**: After run batch command proven
- **CLI config-file support** (`.speckit.toml`): After Phase C/D requirements clarify
- **Checkpoint persistence**: After orchestration proven
- **Template library**: Built-in SPEC templates for common patterns
