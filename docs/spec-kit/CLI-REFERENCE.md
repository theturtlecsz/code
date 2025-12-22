# Spec-Kit CLI Reference

**Last Updated:** 2025-12-22
**Status:** SPEC-KIT-921 Complete
**Binary:** `code speckit` (alias: `code sk`)

---

## Overview

The Spec-Kit CLI provides headless access to all spec-kit functionality for:
- CI/CD automation (model-free validation)
- Scripting and batch operations
- JSON output for tool integration
- Exit code contracts for automation

All commands use the shared `SpeckitExecutor` core, ensuring **CLI/TUI parity**.

---

## Quick Start

```bash
# Show help
code speckit --help

# Check SPEC status (JSON for CI)
code speckit status --spec SPEC-KIT-921 --json

# Validate plan stage (dry-run, no agents)
code speckit plan --spec SPEC-KIT-921 --dry-run --json

# Batch validate stages
code speckit run --spec SPEC-KIT-921 --from plan --to audit --json

# Review stage gate artifacts
code speckit review --spec SPEC-KIT-921 --stage plan --explain
```

---

## Commands

### status

Show SPEC status dashboard with stage progress and evidence footprint.

```bash
code speckit status --spec <SPEC-ID> [--stale-hours N] [--json]
```

**Options:**
| Flag | Default | Description |
|------|---------|-------------|
| `--spec, -s` | required | SPEC identifier (e.g., SPEC-KIT-921) |
| `--stale-hours` | 24 | Hours after which telemetry is considered stale |
| `--json, -j` | false | Output as JSON instead of text |

**Exit Codes:**
- 0: Success
- 3: Infrastructure error

**Example Output (JSON):**
```json
{
  "schema_version": 1,
  "tool_version": "0.0.0+abc123",
  "spec_id": "SPEC-KIT-921",
  "generated_at": "2025-12-22T04:41:27Z",
  "stages": [...],
  "evidence": {...}
}
```

---

### review

Evaluate stage gate artifacts and determine pass/fail/escalation.

```bash
code speckit review --spec <SPEC-ID> --stage <STAGE> [OPTIONS]
```

**Options:**
| Flag | Default | Description |
|------|---------|-------------|
| `--spec, -s` | required | SPEC identifier |
| `--stage` | required | Stage to review (plan, tasks, implement, validate, audit, unlock) |
| `--strict-artifacts` | false | Fail if expected artifacts are missing (exit 2) |
| `--strict-warnings` | false | Treat PassedWithWarnings as exit 1 |
| `--strict-schema` | false | Fail on parse/schema errors (exit 3) |
| `--evidence-root` | auto | Override evidence root path |
| `--explain` | false | Show human-readable exit code explanation |
| `--json, -j` | false | Output as JSON |

**Exit Code Contract:**
| Code | Meaning | Scenario |
|------|---------|----------|
| 0 | Proceed | No conflicts, or warnings without --strict-warnings |
| 1 | Soft fail | Warnings with --strict-warnings enabled |
| 2 | Hard fail | Blocking conflicts or escalation required |
| 3 | Infrastructure | Parse/schema errors with --strict-schema |

**Example:**
```bash
# Strict CI validation
code speckit review --spec SPEC-KIT-921 --stage plan \
  --strict-artifacts --strict-schema --json
```

---

### specify

Create a new SPEC directory structure with PRD.md template.

```bash
code speckit specify --spec <SPEC-ID> [--execute] [--json]
```

**Options:**
| Flag | Default | Description |
|------|---------|-------------|
| `--spec, -s` | required | SPEC identifier to create |
| `--execute` | false | Actually create files (default is dry-run) |
| `--json, -j` | false | Output as JSON |

**Example:**
```bash
# Check what would be created
code speckit specify --spec SPEC-KIT-999

# Actually create the SPEC
code speckit specify --spec SPEC-KIT-999 --execute
```

---

### Stage Commands (plan, tasks, implement, validate, audit, unlock)

Validate SPEC prerequisites and check readiness for a stage.

```bash
code speckit plan --spec <SPEC-ID> [--dry-run] [--strict-prereqs] [--json]
code speckit tasks --spec <SPEC-ID> [--dry-run] [--strict-prereqs] [--json]
code speckit implement --spec <SPEC-ID> [--dry-run] [--strict-prereqs] [--json]
code speckit validate --spec <SPEC-ID> [--dry-run] [--strict-prereqs] [--json]
code speckit audit --spec <SPEC-ID> [--dry-run] [--strict-prereqs] [--json]
code speckit unlock --spec <SPEC-ID> [--dry-run] [--strict-prereqs] [--json]
```

**Options:**
| Flag | Default | Description |
|------|---------|-------------|
| `--spec, -s` | required | SPEC identifier |
| `--dry-run` | true | Validate only, don't trigger agent execution |
| `--strict-prereqs` | false | Treat missing prerequisites as blocking |
| `--json, -j` | false | Output as JSON |

**Exit Codes:**
| Code | Meaning |
|------|---------|
| 0 | Stage ready |
| 2 | Stage blocked (prerequisites missing with --strict-prereqs) |
| 3 | Infrastructure error |

**Example:**
```bash
# CI validation for plan stage
code speckit plan --spec SPEC-KIT-921 --dry-run --strict-prereqs --json
```

---

### run

Batch validate multiple stages in sequence.

```bash
code speckit run --spec <SPEC-ID> --from <STAGE> --to <STAGE> [--json]
```

**Options:**
| Flag | Default | Description |
|------|---------|-------------|
| `--spec, -s` | required | SPEC identifier |
| `--from` | required | Starting stage (inclusive) |
| `--to` | required | Ending stage (inclusive) |
| `--json, -j` | false | Output as JSON |

**Exit Codes:**
| Code | Meaning |
|------|---------|
| 0 | All stages ready |
| 2 | One or more stages blocked |
| 3 | Infrastructure error |

**Example:**
```bash
# Validate entire pipeline in one command
code speckit run --spec SPEC-KIT-921 --from plan --to audit --json
```

**Output (JSON):**
```json
{
  "schema_version": 1,
  "overall_status": "ready",
  "from_stage": "plan",
  "to_stage": "audit",
  "stages": [
    {"stage": "Plan", "status": "ready", "warnings": [], "errors": []},
    {"stage": "Tasks", "status": "ready", "warnings": [], "errors": []},
    ...
  ],
  "exit_code": 0
}
```

---

### migrate

Migrate legacy spec.md to PRD.md format.

```bash
code speckit migrate --spec <SPEC-ID> [--dry-run] [--json]
```

**Options:**
| Flag | Default | Description |
|------|---------|-------------|
| `--spec, -s` | required | SPEC identifier |
| `--dry-run` | false | Check what would be migrated |
| `--json, -j` | false | Output as JSON |

**Exit Codes:**
| Code | Meaning |
|------|---------|
| 0 | Success or already migrated |
| 1 | Error (no source file, etc.) |

---

## CI Integration

### GitHub Actions Example

```yaml
- name: Build CLI
  working-directory: codex-rs
  run: cargo build --release -p codex-cli

- name: Validate SPEC pipeline
  run: |
    ./codex-rs/target/release/code speckit run \
      --spec SPEC-KIT-921 \
      --from plan \
      --to audit \
      --json | jq -e '.overall_status == "ready"'

- name: Review stage gates
  run: |
    ./codex-rs/target/release/code speckit review \
      --spec SPEC-KIT-921 \
      --stage plan \
      --strict-artifacts \
      --strict-schema \
      --json
```

### Exit Code Contract Summary

| Exit Code | Meaning | Action |
|-----------|---------|--------|
| 0 | Success / Ready | Proceed |
| 1 | Soft failure (warnings) | Review, may proceed |
| 2 | Hard failure (blocked) | Fix blockers, retry |
| 3 | Infrastructure error | Debug/escalate |

---

## JSON Schema Versioning

All JSON outputs include:
- `schema_version`: Integer, bumped only on **breaking** changes
- `tool_version`: Cargo version + git SHA (format: `0.0.0+abc123`)

**Compatibility policy:**
- Additive changes (new fields) do NOT bump version
- Removed/renamed fields bump version
- Semantic changes to existing fields bump version

---

## Global Options

Available on all commands:

| Flag | Description |
|------|-------------|
| `-C, --cwd <DIR>` | Working directory (defaults to current) |
| `-c, --config <key=value>` | Override configuration value |
| `-h, --help` | Print help |

---

## TUI/CLI Parity

The CLI uses the same `SpeckitExecutor` as TUI slash commands:

| TUI Command | CLI Equivalent |
|-------------|----------------|
| `/speckit.status SPEC-ID` | `code speckit status --spec SPEC-ID` |
| `/speckit.plan SPEC-ID` | `code speckit plan --spec SPEC-ID` |
| `/review plan` | `code speckit review --stage plan` |

**Parity is verified by unit tests** — see `spec-kit/src/executor/mod.rs`.

---

## Related Documentation

- [COMMAND_INVENTORY.md](COMMAND_INVENTORY.md) — TUI slash command reference
- [GATE_POLICY.md](GATE_POLICY.md) — Gate policy vocabulary and semantics
- [REVIEW-CONTRACT.md](../../codex-rs/spec-kit/docs/REVIEW-CONTRACT.md) — Exit code contract spec
