# HANDOFF: SPEC-KIT-921 P7 Continuation

**Generated**: 2025-12-21
**Last Commit**: a2ef529fa (centralize prereq matrix with architect gate tests)
**Branch**: main

---

## Continue P7 Implementation

Copy this prompt to start the next session:

```
# Continue SPEC-KIT-921 P7 Implementation

Continue from docs/spec-kit/HANDOFF.md

## Context
- P0-P6 complete (with architectural improvements)
- 53 CLI tests passing (50 speckit + 3 helpers)
- All 7 stage commands have CLI support (specify, plan, tasks, implement, validate, audit, unlock)
- --strict-prereqs flag available on all stage validation commands
- Centralized prereq matrix in check_stage_prereqs() helper
- Test helpers module available for reduced boilerplate
- speckit specify creates SPEC directory with PRD.md (idempotent)

## P6 Completed Deliverables

| Task | Status | Tests |
|------|--------|-------|
| P6-C: --strict-prereqs | ✅ | 6 tests |
| P6-D: Test helpers | ✅ | 3 tests (speckit_helpers.rs) |
| P6-A: speckit specify | ✅ | 7 tests (4 original + 3 architect gate) |
| P6-B: speckit run batch | ⏳ Deferred | - |

## P7 Priority Order

1. **P7-A: speckit run batch command** (--from/--to stage ranges)
2. **P7-B: Template library** (built-in SPEC templates)
3. **P7-C: Full /speckit.auto orchestration**

## Start Commands

```bash
git log --oneline -5
cargo test -p codex-cli --test speckit
cargo test -p codex-cli --test speckit_helpers
```

## Acceptance Criteria for P7

- [ ] `speckit run` batch command with --from/--to support
- [ ] Template library for common SPEC patterns
- [ ] Full 8-stage pipeline test (specify→unlock)
- [ ] 60+ CLI integration tests passing
```

---

## Session Summary (P6 Completed + Architectural Improvements)

### P6 Deliverables

| Task | Status | Deliverable |
|------|--------|-------------|
| P6-C: --strict-prereqs flag | ✅ | `--strict-prereqs` on all 6 stage commands, 6 tests |
| P6-D: Test helpers | ✅ | `speckit_helpers.rs` module with TestContext, CliResult, 3 tests |
| P6-A: speckit specify | ✅ | `speckit specify --spec <ID> [--execute] [--json]`, 7 tests |
| P6-B: speckit run batch | ⏳ Deferred | Complex orchestration, moved to P7 |

**Tests**: 53 CLI tests passing (50 speckit + 3 helpers)

### Architectural Improvements (P6 Continuation)

Centralized prereq checking into single `check_stage_prereqs()` helper:

```rust
/// Returns (required_missing, recommended_missing)
/// Required: blocks with --strict-prereqs
/// Recommended: never blocks, always advisory
fn check_stage_prereqs(spec_dir, spec_id, stage) -> (Vec<String>, Vec<String>)
```

Added 3 architect gate tests:
1. `specify_idempotent_never_overwrites` - verify specify doesn't clobber existing PRD.md
2. `specify_then_plan_strict_prereqs_succeeds` - verify specify→plan chain works
3. `plan_strict_prereqs_blocks_without_spec_dir` - verify Plan blocks when SPEC dir missing

### Key Additions

#### --strict-prereqs (P6-C)

Added to all stage validation commands:

```bash
speckit tasks --spec <ID> --strict-prereqs [--dry-run] [--json]
```

Behavior:
- Default: warnings are advisory, command succeeds
- With --strict-prereqs: missing prereqs → Blocked (exit 2)

Prereq matrix:
| Stage     | Required Prereqs                    |
|-----------|-------------------------------------|
| Specify   | (none - first stage)                |
| Plan      | SPEC directory exists               |
| Tasks     | plan.md exists                      |
| Implement | plan.md exists                      |
| Validate  | tasks.md OR implement.md exists     |
| Audit     | tasks.md OR implement.md exists     |
| Unlock    | tasks.md OR implement.md exists     |

#### Test Helpers (P6-D)

New `speckit_helpers.rs` module:

```rust
// Create test context with isolated dirs
let ctx = TestContext::new()?;

// Setup SPEC directory with files
ctx.setup_spec_dir("SPEC-001", &[("plan.md", "# Plan")])?;

// Run CLI and get result wrapper
let result = ctx.run_cli(&["speckit", "tasks", "--spec", "SPEC-001", "--json"])?;

// Assertion helpers
result.assert_success();
result.assert_schema_version(1);
result.assert_stage("Tasks");
result.assert_status("ready");
```

#### speckit specify (P6-A)

```bash
# Dry-run (default): report what would be created
speckit specify --spec SPEC-TEST-001 [--json]

# Execute: actually create directory and PRD.md
speckit specify --spec SPEC-TEST-001 --execute [--json]
```

Creates:
- `docs/<SPEC-ID>/` directory
- `docs/<SPEC-ID>/PRD.md` with template content

---

## Architecture After P6 (Current)

```
CLI (speckit_cmd.rs)
  ↓ SpeckitSubcommand::Status/Review/Specify/Plan/Tasks/Implement/Validate/Audit/Unlock
  ↓
SpeckitExecutor::execute()
  ↓ SpeckitCommand::Specify { spec_id, dry_run }
  ↓ SpeckitCommand::ValidateStage { spec_id, stage, dry_run, strict_prereqs }
  ↓
Outcome::Status/Review/ReviewSkipped/Stage(StageOutcome)/Specify(SpecifyOutcome)/Error
  ↓
render_*() → stdout (with schema_version)
```

Key types:

```rust
// spec-kit/src/executor/command.rs
SpeckitCommand::Specify { spec_id, dry_run }
SpeckitCommand::ValidateStage { spec_id, stage, dry_run, strict_prereqs }

// spec-kit/src/executor/mod.rs
SpecifyOutcome { spec_id, dry_run, spec_dir, already_existed, created_files }
StageOutcome { spec_id, stage, resolution, blocking_reasons, advisory_signals, evidence_refs, dry_run }
```

---

## Files Reference

### Modified in P6

| File | Purpose |
|------|---------|
| `cli/src/speckit_cmd.rs` | Added Specify subcommand, --strict-prereqs to all stages |
| `cli/tests/speckit.rs` | 50 tests (37 P5 + 6 strict-prereqs + 7 specify) |
| `cli/tests/speckit_helpers.rs` | NEW: 3 test helper tests |
| `spec-kit/src/executor/command.rs` | Added Specify command variant, strict_prereqs to ValidateStage |
| `spec-kit/src/executor/mod.rs` | Added execute_specify(), SpecifyOutcome, strict_prereqs handling |
| `tui/src/chatwidget/spec_kit/commands/plan.rs` | Added Outcome::Specify handling |
| `tui/src/chatwidget/spec_kit/command_handlers.rs` | Added Outcome::Specify handling |

### To Modify in P7

| File | Change |
|------|--------|
| `cli/src/speckit_cmd.rs` | Add Run subcommand with --from/--to |
| `spec-kit/src/executor/mod.rs` | Add execute_run_batch() |
| `cli/tests/speckit.rs` | Add run batch tests |
| `templates/` | Add built-in SPEC templates |

---

## Invariants to Preserve

1. **No env/config reads in executor core** — resolve at adapter boundary
2. **All paths repo-relative** in JSON output and evidence refs
3. **Deterministic evidence selection** — tests cover ordering/timestamp logic
4. **Strict modes behave predictably**: artifacts→2, schema→3, escalation→2, prereqs→2
5. **TUI is an adapter** — slash commands call executor, not legacy implementations
6. **Stage-specific commands** — each stage has its own CLI command (no generic `--stage` flag)
7. **ValidateStage is stage-neutral** — single command variant handles all stage validation
8. **Build-time determinism** — tool_version uses compile-time env vars only, no runtime git
9. **Advisory by default** — missing prereqs warn but don't block (unless --strict-prereqs)
10. **Specify is separate** — creation command, not validation (uses --execute, not --dry-run=false)
11. **Centralized prereq matrix** — all prereq checks go through `check_stage_prereqs()`, not scattered logic
12. **Idempotent scaffolding** — `speckit specify --execute` never overwrites existing PRD.md content

---

## Exit Code Contract (Reference)

| Code | Meaning              |
|------|----------------------|
| 0    | Success / Ready      |
| 2    | Blocked / Escalation |
| 3    | Infrastructure error |

---

## Deferred to P7+

- **speckit run batch command**: Stage range execution with --from/--to
- **Template library**: Built-in SPEC templates for common patterns
- **Full /speckit.auto orchestration**: After run batch command proven
- **CLI config-file support** (`.speckit.toml`): After Phase C/D requirements clarify
- **Checkpoint persistence**: After orchestration proven
- **Custom template directory**: User-defined templates in .speckit/templates/

---

## Progress Tracking

| Phase | Status | Tests | Key Deliverable |
|-------|--------|-------|-----------------|
| P0-P3 | ✅ | 22 | Status, Review commands |
| P4 | ✅ | 25 | Tasks command, StageOutcome envelope |
| P5 | ✅ | 37 | Implement/Validate/Audit/Unlock commands |
| P6 | ✅ | 53 | --strict-prereqs, specify, test helpers, prereq matrix |
| P7+ | ⏳ | - | Run batch, templates, auto orchestration |
