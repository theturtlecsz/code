# HANDOFF: SPEC-KIT-921 P7 Continuation

**Generated**: 2025-12-21
**Last Commit**: 0ddd9e61e (P7 foundational improvements)
**Branch**: main

---

## Continue P7 Implementation

Copy this prompt to start the next session:

```
# Continue SPEC-KIT-921 P7-A Implementation

Continue from docs/spec-kit/HANDOFF.md

## Context
- P0-P6 + P7 foundations complete
- 57 CLI tests passing (54 speckit + 3 helpers)
- Canonical packet contract defined (PRD.md → plan.md → tasks.md → ...)
- Spec ID validation with path traversal protection
- Suffix directory resolution (SPEC-ID-suffix patterns)
- Prereq matrix aligned with artifact dependency DAG

## P7-A Task: speckit run batch command

Implement `speckit run --spec <ID> --from <stage> --to <stage> [--json]`

### Architect-Approved Design
1. **Validation only** - no agent spawning (readiness check for CI)
2. **Named stages only** - specify/plan/tasks/implement/validate/audit/unlock
3. **Aggregated output** - single JSON with per-stage outcomes
4. **Exit codes** - 0=all ready, 2=any blocked, 3=infrastructure error

### Implementation Steps
1. Add `Run` subcommand to `cli/src/speckit_cmd.rs`
2. Add `SpeckitCommand::Run { spec_id, from, to }` to executor
3. Add `execute_run()` that iterates stages and collects outcomes
4. Add `RunOutcome` with aggregated results
5. Add spec.md fallback logic with deprecation warning
6. Add 8+ tests for run command

### Also in P7
- spec.md fallback with `packet_source: "spec_md_legacy"` warning
- Full 7-stage validation pipeline test

## P7 Foundational Deliverables (Complete)

| Task | Status | Tests |
|------|--------|-------|
| Spec ID validation | ✅ | 2 tests (path traversal, format) |
| Directory resolution | ✅ | 2 tests (suffix, determinism) |
| Prereq matrix alignment | ✅ | Updated all stages |
| Packet contract docs | ✅ | In executor/mod.rs |

## P7 Remaining Priority Order

1. **P7-A: speckit run batch command** (validation only, --from/--to)
2. **P7-B: spec.md fallback** (legacy compatibility with deprecation warning)
3. **P7-C: Full pipeline test** (specify→unlock validation chain)

**Deferred to P8**: Template library, full auto orchestration with agent spawning

## P7-A Design Decisions (Architect-Approved)

### Run Command Semantics
- **Validation only** - no agent spawning (that's /speckit.auto territory)
- Iterate stages from `--from` to `--to`
- Run executor validation for each stage
- Optionally run canonical reviews at boundaries (AfterPlan, AfterTasks, BeforeUnlock)
- Emit single aggregated JSON summary + per-stage outcomes
- Deterministic readiness report + exit code (CI-friendly)

### Stage Granularity
- **Named stages only**: `--from plan --to validate`
- No numeric IDs (prevents vocabulary drift)
- Valid names: specify, plan, tasks, implement, validate, audit, unlock

### Legacy Compatibility
- PRD.md is canonical input to Plan
- If PRD.md missing but spec.md exists:
  - Proceed with warning (non-strict mode)
  - Emit `packet_source: "spec_md_legacy"` in output
  - Mark as deprecated
- Future: `--strict-packet` flag to block on missing PRD.md

### Output Schema
```json
{
  "schema_version": 1,
  "spec_id": "SPEC-XXX",
  "from_stage": "plan",
  "to_stage": "validate",
  "overall_status": "ready|blocked|partial",
  "stages": [
    { "stage": "plan", "status": "ready", "warnings": [], "errors": [] },
    { "stage": "tasks", "status": "blocked", "warnings": [], "errors": ["..."] }
  ],
  "exit_code": 0|2|3
}
```

## Start Commands

```bash
git log --oneline -5
cargo test -p codex-cli --test speckit
cargo test -p codex-cli --test speckit_helpers
```

## Acceptance Criteria for P7

- [ ] `speckit run --spec <ID> --from <stage> --to <stage> [--json]`
- [ ] spec.md fallback with deprecation warning
- [ ] Full 7-stage validation test (specify→unlock)
- [ ] Aggregated JSON output with per-stage outcomes
- [ ] 65+ CLI integration tests passing
```

---

## Session Summary (P7 Foundations)

### P7 Foundational Deliverables

| Task | Status | Deliverable |
|------|--------|-------------|
| Canonical packet contract | ✅ | Artifact DAG documented in executor/mod.rs |
| Spec ID validation | ✅ | Path traversal + format validation, 2 tests |
| Directory resolution | ✅ | resolve_spec_dir() with suffix support, 2 tests |
| Prereq matrix alignment | ✅ | Matches artifact DAG |

**Tests**: 57 CLI tests passing (54 speckit + 3 helpers)

### Canonical Packet Contract

```
Stage     | Input Required       | Output Created
----------|---------------------|------------------
Specify   | (none)              | PRD.md
Plan      | PRD.md              | plan.md
Tasks     | plan.md             | tasks.md
Implement | tasks.md            | implement.md
Validate  | implement.md        | validate.md
Audit     | validate.md         | audit.md
Unlock    | audit.md            | (approval)
```

### Key Utilities Added

```rust
// Spec ID validation (security)
pub fn validate_spec_id(spec_id: &str) -> Result<(), SpecIdError>

// Canonical directory resolution (supports suffixes)
pub fn resolve_spec_dir(repo_root: &Path, spec_id: &str) -> Option<ResolvedSpecDir>

// Creation path (no suffix)
pub fn default_spec_dir_for_creation(repo_root: &Path, spec_id: &str) -> PathBuf
```

### P6 Session Summary (Previous)

| Task | Status | Tests |
|------|--------|-------|
| P6-C: --strict-prereqs | ✅ | 6 tests |
| P6-D: Test helpers | ✅ | 3 tests |
| P6-A: speckit specify | ✅ | 7 tests |
| Architect gate tests | ✅ | 3 tests |

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
| P6 | ✅ | 53 | --strict-prereqs, specify, test helpers |
| P7 (foundations) | ✅ | 57 | Packet contract, spec ID validation, directory resolution |
| P7+ | ⏳ | - | Run batch, templates, auto orchestration |
