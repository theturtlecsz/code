# HANDOFF: SPEC-KIT-921 P3 Continuation

**Generated**: 2025-12-21
**Last Commit**: 61bebcf3b (feat(spec-kit): implement P2 CI workflow and CLI enhancements)
**Branch**: main

---

## Session Summary

### P2 Completed (61bebcf3b)

| Task | Status | Deliverable |
|------|--------|-------------|
| P2-A: CI Workflow | ✅ | `.github/workflows/spec-kit-ci.yml` |
| P2-B: Status JSON | ✅ | Enhanced with stages, packet, repo-relative paths |
| P2-C: --explain flag | ✅ | Exit code debugging for review command |

**Tests**: 16 CLI integration tests passing
**Parity**: TUI `/spec-review` and CLI `speckit review` use same `SpeckitExecutor::execute()` + `render_review_dashboard()`

### Architect Review Feedback Applied

- CI hardening: `permissions: contents: read`, `pipefail`, `--explain` in failure diagnostics
- Repo-relative paths verified in all JSON outputs
- TUI/CLI parity confirmed via code inspection

---

## P3 Scope (Architect-Approved)

### Strategy: Incremental Migration

```
plan stage → CI validation → expand to tasks/implement → full pipeline
```

**Rationale**: Proves the hard part (stage execution behind executor without ChatWidget coupling) with minimal risk. CI smoke test prevents invariant breakage before expansion.

### P3 Tasks

#### P3-A: Schema Versioning for JSON Outputs

Add to all JSON outputs (status, review, future stage outputs):

```json
{
  "schema_version": 1,
  "tool_version": "0.1.0+abc1234",
  ...
}
```

Rules:
- `schema_version`: Bump only on breaking changes; additive fields don't require bumps
- `tool_version`: Cargo version + git sha (if available) for forensic debugging

Files to modify:
- `cli/src/speckit_cmd.rs`: Add version fields to status and review JSON
- Add test for version fields in `cli/tests/speckit.rs`

#### P3-B: Migrate /speckit.plan Behind Executor

Move plan stage execution from ChatWidget into `SpeckitExecutor`:

1. Add `SpeckitCommand::Plan { spec_id, ... }` variant
2. Implement `execute_plan()` in executor
3. Refactor TUI `/speckit.plan` to call `executor.execute(SpeckitCommand::Plan { ... })`
4. Add CLI `speckit plan --spec <ID>` command

Files to modify:
- `spec-kit/src/executor/command.rs`: Add Plan variant
- `spec-kit/src/executor/mod.rs`: Add execute_plan()
- `tui/src/chatwidget/spec_kit/command_handlers.rs`: Delegate to executor
- `cli/src/speckit_cmd.rs`: Add plan subcommand

Parity check: TUI and CLI must produce identical artifacts and exit codes.

#### P3-C: CI Pipeline Smoke Test

Add to `.github/workflows/spec-kit-ci.yml`:

```yaml
- name: "[PIPELINE] plan → review smoke test"
  run: |
    # Run plan on fixture SPEC
    $CLI speckit plan --spec SPEC-CI-001-clean --dry-run

    # Verify review still works after plan
    $CLI speckit review --spec SPEC-CI-001-clean --stage plan --json
```

**Key constraint**: Model-free CI. Use fixture-driven or stubbed execution—no live LLM calls.

Validate:
- TUI and CLI both route `/speckit.plan` → `SpeckitExecutor::execute()`
- Evidence/artifact outputs land in expected topology (repo-relative refs)
- Exit code contract stable for: success, escalation, infra failure

#### P3-D: Update Documentation

- Update CONTINUATION-PROMPT for P4
- Document schema versioning contract in CLI help or README

---

## Files Reference

### Modified in P2

| File | Purpose |
|------|---------|
| `.github/workflows/spec-kit-ci.yml` | CI exit code contract + smoke tests |
| `cli/src/speckit_cmd.rs` | Status JSON, --explain flag |
| `cli/tests/speckit.rs` | 16 integration tests |

### To Modify in P3

| File | Change |
|------|--------|
| `spec-kit/src/executor/command.rs` | Add Plan variant |
| `spec-kit/src/executor/mod.rs` | Add execute_plan() |
| `tui/src/chatwidget/spec_kit/command_handlers.rs` | Delegate plan to executor |
| `cli/src/speckit_cmd.rs` | Add plan subcommand + schema versioning |

### Existing Architecture (for reference)

```
CLI (speckit_cmd.rs)
  ↓ SpeckitCommand::Review/Status
  ↓
SpeckitExecutor::execute()
  ↓
Outcome::Review/Status/Error
  ↓
render_*_dashboard() → stdout

TUI (command_handlers.rs)
  ↓ SpeckitCommand::parse_review()
  ↓
SpeckitExecutor::execute()  ← SAME EXECUTOR
  ↓
Outcome::Review/Status/Error
  ↓
render_*_dashboard() → ChatWidget
```

---

## Exit Code Contract (Reference)

| Code | Meaning | Trigger |
|------|---------|---------|
| 0 | Success/proceed | Clean consensus, no blocking signals |
| 1 | Soft fail | PassedWithWarnings + --strict-warnings |
| 2 | Hard fail | Conflicts, escalation, missing artifacts + --strict-artifacts |
| 3 | Infrastructure error | Parse/schema errors + --strict-schema |

---

## Deferred to P4

- **CLI config-file support** (`.speckit.toml`): Defer until Phase C/D requirements clarify
- **Evidence root auto-discovery**: Complexity multiplier; only after pipeline migration
- **Full Phase D** (`/speckit.auto` orchestration): After plan/tasks/implement proven

---

## Start Commands

```bash
# Verify current state
cd /home/thetu/code
git log --oneline -5
cargo test -p codex-cli --test speckit

# Start P3-A (schema versioning)
# Then P3-B (plan migration)
# Then P3-C (CI smoke test)
```

---

## Acceptance Criteria for P3

- [ ] All JSON outputs include `schema_version` and `tool_version`
- [ ] `/speckit.plan` routes through `SpeckitExecutor` in both TUI and CLI
- [ ] CLI `speckit plan` command exists with same exit codes as TUI
- [ ] CI pipeline smoke test passes (fixture-driven, model-free)
- [ ] 18+ CLI integration tests passing (16 existing + 2 new)
- [ ] No ChatWidget coupling in plan execution logic
