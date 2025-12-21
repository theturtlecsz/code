# HANDOFF: SPEC-KIT-921 P6 Continuation

**Generated**: 2025-12-21
**Last Commit**: cf26d91e9 (feat(spec-kit): implement P5 CLI commands for all stages)
**Branch**: main

---

## Continue P6 Implementation

Copy this prompt to start the next session:

```
# Continue SPEC-KIT-921 P6 Implementation

Continue from docs/spec-kit/HANDOFF.md

## Context
- P0-P5 complete (commit cf26d91e9)
- 37 CLI tests passing (25 original + 12 new for implement/validate/audit/unlock)
- All 6 stage commands have CLI support (plan, tasks, implement, validate, audit, unlock)
- TUI already routes all stage commands through executor
- Full 6-stage CI pipeline test passing
- speckit specify deferred to P6 (different UX expectations)

## P6 Priority Order (Revised)

Based on architectural analysis, P6 order is:
1. **P6-C first**: --strict-prereqs flag (makes CI trustworthy)
2. **P6-A second**: speckit specify (minimal, no templates)
3. **P6-B third**: speckit run batch (benefits from strict prereqs)
4. **P6-D**: Test helpers infrastructure (reduces boilerplate)

## Key Types Reference

```rust
// spec-kit/src/executor/command.rs
SpeckitCommand::ValidateStage { spec_id, stage, dry_run }

// spec-kit/src/executor/mod.rs
StageOutcome { spec_id, stage, resolution, blocking_reasons, advisory_signals, evidence_refs, dry_run }
StageResolution::Ready | Blocked | Skipped
```

## P6 Tasks (in priority order)

### P6-C: --strict-prereqs flag (FIRST PRIORITY)

**Rationale**: Existing pattern (--strict-artifacts, --strict-schema) + makes batch execution meaningful.

Add --strict-prereqs to all stage commands:

```bash
speckit implement --spec <ID> --strict-prereqs [--dry-run] [--json]
```

Prereq matrix (stage → required artifacts):
| Stage     | Required Prereqs                    |
|-----------|-------------------------------------|
| Specify   | (none - first stage)                |
| Plan      | SPEC directory exists               |
| Tasks     | plan.md exists                      |
| Implement | plan.md exists                      |
| Validate  | tasks.md OR implement.md exists     |
| Audit     | tasks.md OR implement.md exists     |
| Unlock    | tasks.md OR implement.md exists     |

Implementation:
1. Add --strict-prereqs flag to all stage Args structs
2. Modify execute_validate_stage() to check strict_prereqs param
3. Return Blocked (exit 2) instead of Ready+warnings when strict
4. Add 6 tests: one per stage verifying strict behavior

Files:
- cli/src/speckit_cmd.rs (add --strict-prereqs to all Args)
- spec-kit/src/executor/mod.rs (update execute_validate_stage)
- cli/tests/speckit.rs (add strict_prereqs tests)

### P6-D: Test helpers infrastructure (WITH P6-C)

Add shared test helpers to reduce boilerplate:

```rust
// cli/tests/speckit_helpers.rs (new file)

/// Create SPEC directory with optional files
fn setup_spec_dir(root: &Path, spec_id: &str, files: &[(&str, &str)]) -> PathBuf;

/// Run CLI command and capture output
fn run_cli(root: &Path, args: &[&str]) -> CliResult;

/// Assert JSON has schema_version = 1
fn assert_schema_version(json: &JsonValue, expected: u32);

/// Assert stage field contains expected string
fn assert_stage(json: &JsonValue, expected: &str);
```

Files:
- cli/tests/speckit_helpers.rs (new)
- cli/tests/speckit.rs (use helpers, reduce duplication)

### P6-A: speckit specify command (MINIMAL)

First stage with minimal scope (no template library):

```bash
speckit specify --spec <ID> [--dry-run] [--json]
```

Behavior:
1. Create docs/<SPEC-ID>/ directory if not exists
2. Create minimal skeleton files for Plan prereqs:
   - PRD.md (empty/placeholder)
   - Or just validate directory structure
3. Return Ready with spec_id

NO template support in P6. Just establish minimum contract for downstream stages.

Implementation:
1. Add SpecifyArgs struct (no --template flag)
2. Add run_specify() that creates directory structure
3. Add SpeckitCommand::Specify variant (not ValidateStage)
4. Add 3 tests: creates_dir, validates_existing_dir, json_includes_schema_version

Files:
- cli/src/speckit_cmd.rs (add Specify subcommand)
- spec-kit/src/executor/mod.rs (add Specify command handling)
- cli/tests/speckit.rs (add specify tests)

### P6-B: speckit run batch command (LAST)

Sequential execution with stage range:

```bash
speckit run --spec <ID> [--from <STAGE>] [--to <STAGE>] [--strict-prereqs] [--dry-run] [--json]
```

Edge cases:
- --from without --to: run from specified stage to unlock
- --to without --from: run from specify to specified stage
- Neither: run full pipeline (specify→unlock)
- Invalid stage order: error
- Stops on first Blocked result

Implementation:
1. Add RunArgs struct with from/to/strict_prereqs options
2. Add run_batch() that iterates stages in order
3. JSON output: array of stage outcomes
4. Add 5+ tests for stage range handling

## Start Commands

```bash
git log --oneline -5
cargo test -p codex-cli --test speckit
cat codex-rs/spec-kit/src/executor/mod.rs | head -150  # See prereq checking
```

## Exit Code Contract (Reference)

| Code | Meaning              |
|------|----------------------|
| 0    | Success / Ready      |
| 2    | Blocked / Escalation |
| 3    | Infrastructure error |

## Acceptance Criteria for P6

- [ ] --strict-prereqs flag on all 6 stage commands
- [ ] Test helpers module reduces test duplication
- [ ] `speckit specify` creates SPEC directory (minimal, no templates)
- [ ] `speckit run` batch command with --from/--to support
- [ ] 50+ CLI integration tests passing
- [ ] CI smoke: full 7-stage pipeline test (specify→unlock)
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
| `cli/src/speckit_cmd.rs` | Add --strict-prereqs to all, add Specify and Run |
| `cli/tests/speckit_helpers.rs` | NEW: shared test helpers |
| `cli/tests/speckit.rs` | Use helpers, add strict/specify/run tests |
| `spec-kit/src/executor/mod.rs` | Add strict_prereqs handling, Specify command |
| `.github/workflows/spec-kit-ci.yml` | Add specify to pipeline, strict-prereqs tests |

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

---

## Deferred to P7+

- **Template library**: Built-in SPEC templates for common patterns
- **Full /speckit.auto orchestration**: After run batch command proven
- **CLI config-file support** (`.speckit.toml`): After Phase C/D requirements clarify
- **Checkpoint persistence**: After orchestration proven
- **Custom template directory**: User-defined templates in .speckit/templates/
