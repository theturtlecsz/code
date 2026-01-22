# CLI Reference (v1.0.0)

**Binary**: `code speckit` (alias: `code sk`)
**Last Updated**: 2026-01-22
**Total Commands**: 23 command structs, 40 names (primary + aliases)

***

## Table of Contents

* [Overview](#overview)
* [Quick Start](#quick-start)
* [Core Commands Reference](#core-commands-reference)
  * [status](#status)
  * [review](#review)
  * [specify](#specify)
  * [Stage Commands](#stage-commands)
  * [run](#run)
  * [migrate](#migrate)
* [Command Inventory](#command-inventory)
  * [Intake Commands (2)](#intake-commands-2)
  * [Quality Commands (3)](#quality-commands-3)
  * [Stage Commands (6)](#stage-commands-6)
  * [Automation Commands (2)](#automation-commands-2)
  * [Guardrail Commands (7)](#guardrail-commands-7)
  * [Project Commands (1)](#project-commands-1)
  * [Utility Commands (2)](#utility-commands-2)
* [Command Types](#command-types)
* [Tiered Model Strategy](#tiered-model-strategy)
* [Workflows & Patterns](#workflows--patterns)
  * [Quick SPEC Creation](#quick-spec-creation)
  * [Full Automation](#full-automation)
  * [Manual Stage-by-Stage](#manual-stage-by-stage)
  * [Quality Checks](#quality-checks)
  * [Diagnostics](#diagnostics)
* [Templates](#templates)
  * [Template Inventory (14)](#template-inventory-14)
  * [Command → Template Mapping](#command--template-mapping)
* [/speckit.new Deep Dive](#speckitnew-deep-dive)
  * [Execution Phases](#execution-phases)
  * [Output Structure](#output-structure)
  * [Next Steps After /speckit.new](#next-steps-after-speckitnew)
* [CI/CD Integration](#cicd-integration)
  * [GitHub Actions Example](#github-actions-example)
  * [Exit Code Contract](#exit-code-contract)
  * [JSON Schema Versioning](#json-schema-versioning)
* [Global Options & Parity](#global-options--parity)
  * [Global Options](#global-options)
  * [TUI/CLI Parity](#tuicli-parity)
  * [Alias Mapping (Legacy Names)](#alias-mapping-legacy-names)
* [Change History](#change-history)

## Overview

The Spec-Kit CLI provides headless access to all spec-kit functionality for:

* CI/CD automation (model-free validation)
* Scripting and batch operations
* JSON output for tool integration
* Exit code contracts for automation

All commands use the shared `SpeckitExecutor` core, ensuring **CLI/TUI parity**.

***

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

***

## Core Commands Reference

### status

Show SPEC status dashboard with stage progress and evidence footprint.

```bash
code speckit status --spec <SPEC-ID> [--stale-hours N] [--json]
```

| Flag            | Default  | Description                          |
| --------------- | -------- | ------------------------------------ |
| `--spec, -s`    | required | SPEC identifier                      |
| `--stale-hours` | 24       | Hours after which telemetry is stale |
| `--json, -j`    | false    | Output as JSON                       |

**Exit Codes**: 0 (success), 3 (infrastructure error)

### review

Evaluate stage gate artifacts and determine pass/fail/escalation.

```bash
code speckit review --spec <SPEC-ID> --stage <STAGE> [OPTIONS]
```

| Flag                 | Default  | Description                                 |
| -------------------- | -------- | ------------------------------------------- |
| `--spec, -s`         | required | SPEC identifier                             |
| `--stage`            | required | Stage to review                             |
| `--strict-artifacts` | false    | Fail if expected artifacts missing (exit 2) |
| `--strict-warnings`  | false    | Treat PassedWithWarnings as exit 1          |
| `--strict-schema`    | false    | Fail on parse/schema errors (exit 3)        |
| `--explain`          | false    | Show human-readable exit explanation        |
| `--json, -j`         | false    | Output as JSON                              |

**Exit Code Contract**:

| Code | Meaning        | Scenario                                            |
| ---- | -------------- | --------------------------------------------------- |
| 0    | Proceed        | No conflicts, or warnings without --strict-warnings |
| 1    | Soft fail      | Warnings with --strict-warnings enabled             |
| 2    | Hard fail      | Blocking conflicts or escalation required           |
| 3    | Infrastructure | Parse/schema errors with --strict-schema            |

### specify

Create a new SPEC directory structure with PRD.md template.

```bash
code speckit specify --spec <SPEC-ID> [--execute] [--json]
```

| Flag         | Default  | Description                                |
| ------------ | -------- | ------------------------------------------ |
| `--spec, -s` | required | SPEC identifier to create                  |
| `--execute`  | false    | Actually create files (default is dry-run) |
| `--json, -j` | false    | Output as JSON                             |

### Stage Commands

Validate SPEC prerequisites and check readiness for a stage.

```bash
code speckit plan --spec <SPEC-ID> [--dry-run] [--strict-prereqs] [--json]
code speckit tasks --spec <SPEC-ID> [--dry-run] [--strict-prereqs] [--json]
code speckit implement --spec <SPEC-ID> [--dry-run] [--strict-prereqs] [--json]
code speckit validate --spec <SPEC-ID> [--dry-run] [--strict-prereqs] [--json]
code speckit audit --spec <SPEC-ID> [--dry-run] [--strict-prereqs] [--json]
code speckit unlock --spec <SPEC-ID> [--dry-run] [--strict-prereqs] [--json]
```

| Flag               | Default  | Description                             |
| ------------------ | -------- | --------------------------------------- |
| `--spec, -s`       | required | SPEC identifier                         |
| `--dry-run`        | true     | Validate only, don't trigger agents     |
| `--strict-prereqs` | false    | Treat missing prerequisites as blocking |
| `--json, -j`       | false    | Output as JSON                          |

**Exit Codes**: 0 (ready), 2 (blocked), 3 (infrastructure error)

### run

Batch validate multiple stages in sequence.

```bash
code speckit run --spec <SPEC-ID> --from <STAGE> --to <STAGE> [--json]
```

| Flag         | Default  | Description                |
| ------------ | -------- | -------------------------- |
| `--spec, -s` | required | SPEC identifier            |
| `--from`     | required | Starting stage (inclusive) |
| `--to`       | required | Ending stage (inclusive)   |
| `--json, -j` | false    | Output as JSON             |

### migrate

Migrate legacy spec.md to PRD.md format.

```bash
code speckit migrate --spec <SPEC-ID> [--dry-run] [--json]
```

***

## Command Inventory

### Intake Commands (2)

| Command            | Description                                                  | Type         | Agents               |
| ------------------ | ------------------------------------------------------------ | ------------ | -------------------- |
| `/speckit.new`     | Create new SPEC from description (55% faster with templates) | Orchestrator | gemini, claude, code |
| `/speckit.specify` | Generate PRD with multi-agent consensus                      | Orchestrator | gemini, claude, code |

**Aliases**: `/new-spec` → `/speckit.new`

### Quality Commands (3)

| Command              | Description                                | Type          | Agents               | Time     |
| -------------------- | ------------------------------------------ | ------------- | -------------------- | -------- |
| `/speckit.clarify`   | Resolve spec ambiguities (max 5 questions) | Prompt-expand | gemini, claude, code | 8-12 min |
| `/speckit.analyze`   | Check cross-artifact consistency           | Prompt-expand | gemini, claude, code | 8-12 min |
| `/speckit.checklist` | Evaluate requirement quality (scores)      | Prompt-expand | claude, code         | 5-8 min  |

### Stage Commands (6)

| Command              | Description                                | Template              | Agents                               | Time      |
| -------------------- | ------------------------------------------ | --------------------- | ------------------------------------ | --------- |
| `/speckit.plan`      | Create work breakdown                      | plan-template.md      | gemini, claude, gpt\_pro             | 8-12 min  |
| `/speckit.tasks`     | Generate task list with validation mapping | tasks-template.md     | gemini, claude, gpt\_pro             | 8-12 min  |
| `/speckit.implement` | Write code with multi-agent consensus      | implement-template.md | gemini, claude, gpt\_codex, gpt\_pro | 15-20 min |
| `/speckit.validate`  | Run test strategy                          | validate-template.md  | gemini, claude, gpt\_pro             | 10-12 min |
| `/speckit.audit`     | Compliance review                          | audit-template.md     | gemini, claude, gpt\_pro             | 10-12 min |
| `/speckit.unlock`    | Final approval for merge                   | unlock-template.md    | gemini, claude, gpt\_pro             | 10-12 min |

**Aliases**: `/spec-plan`, `/spec-tasks`, `/spec-implement`, `/spec-validate`, `/spec-audit`, `/spec-unlock`

### Automation Commands (2)

| Command           | Description                                 | Type     | Time     | Cost  |
| ----------------- | ------------------------------------------- | -------- | -------- | ----- |
| `/speckit.auto`   | Full 6-stage pipeline with auto-advancement | Pipeline | \~60 min | \~$11 |
| `/speckit.status` | Show SPEC progress dashboard                | Native   | <1s      | $0    |

**Usage**: `/speckit.auto SPEC-ID [--from stage] [--hal mock|live]`

### Guardrail Commands (7)

| Command                | Description                        | Script                  |
| ---------------------- | ---------------------------------- | ----------------------- |
| `/guardrail.plan`      | Guardrail validation for plan      | spec\_ops\_plan.sh      |
| `/guardrail.tasks`     | Guardrail validation for tasks     | spec\_ops\_tasks.sh     |
| `/guardrail.implement` | Guardrail validation for implement | spec\_ops\_implement.sh |
| `/guardrail.validate`  | Guardrail validation for validate  | spec\_ops\_validate.sh  |
| `/guardrail.audit`     | Guardrail validation for audit     | spec\_ops\_audit.sh     |
| `/guardrail.unlock`    | Guardrail validation for unlock    | spec\_ops\_unlock.sh    |
| `/guardrail.auto`      | Full guardrail pipeline            | spec\_auto.sh           |

### Project Commands (1)

| Command            | Description                                 | Types                                 |
| ------------------ | ------------------------------------------- | ------------------------------------- |
| `/speckit.project` | Scaffold new project with spec-kit workflow | rust, python, typescript, go, generic |

**Created files**: CLAUDE.md, AGENTS.md, GEMINI.md, SPEC.md, docs/, memory/constitution.md

### Utility Commands (2)

| Command                | Description                                  |
| ---------------------- | -------------------------------------------- |
| `/spec-consensus`      | Check multi-agent consensus via local-memory |
| `/spec-evidence-stats` | Summarize guardrail/consensus evidence sizes |

***

## Command Types

| Type                 | Commands                                                                     | Behavior                                                  |
| -------------------- | ---------------------------------------------------------------------------- | --------------------------------------------------------- |
| **Prompt-Expanding** | clarify, analyze, checklist, plan, tasks, implement, validate, audit, unlock | Expand prompt → submit to agents → consensus              |
| **Guardrail**        | guardrail.\* (7)                                                             | Execute bash script → parse telemetry → validate → report |
| **Orchestrator**     | speckit.new, speckit.specify                                                 | Format subagent command → orchestrator handles            |
| **Pipeline**         | speckit.auto                                                                 | State machine → sequential stages → auto-advance          |
| **Native**           | speckit.status, speckit.project                                              | Pure Rust → instant response → $0 cost                    |
| **Diagnostic**       | spec-consensus, spec-evidence-stats                                          | Query evidence/local-memory → display                     |

***

## Tiered Model Strategy

| Tier           | Commands                                                             | Agents                                    | Time      | Cost         |
| -------------- | -------------------------------------------------------------------- | ----------------------------------------- | --------- | ------------ |
| **0: Native**  | status, project                                                      | 0                                         | <1s       | $0           |
| **2-lite**     | checklist                                                            | 2 (claude, code)                          | 5-8 min   | \~$0.35      |
| **2: Triple**  | new, specify, clarify, analyze, plan, tasks, validate, audit, unlock | 3                                         | 8-12 min  | \~$0.60-1.00 |
| **3: Quad**    | implement                                                            | 4 (gemini, claude, gpt\_codex, gpt\_pro)  | 15-20 min | \~$2.00      |
| **4: Dynamic** | auto                                                                 | 3-5 (adaptive, adds arbiter if conflicts) | \~60 min  | \~$11        |

***

## Workflows & Patterns

### Quick SPEC Creation

```bash
/speckit.new Add OAuth2 authentication
/speckit.status
```

### Full Automation

```bash
/speckit.auto SPEC-KIT-065
/speckit.auto SPEC-KIT-065 --from tasks  # Resume
/speckit.auto SPEC-KIT-065 --hal live    # Live HAL validation
```

### Manual Stage-by-Stage

```bash
/speckit.plan SPEC-KIT-065
/speckit.tasks SPEC-KIT-065
/speckit.implement SPEC-KIT-065
/speckit.validate SPEC-KIT-065
/speckit.audit SPEC-KIT-065
/speckit.unlock SPEC-KIT-065
```

### Quality Checks

```bash
/speckit.clarify SPEC-KIT-065   # Resolve ambiguities
/speckit.analyze SPEC-KIT-065   # Check consistency
/speckit.checklist SPEC-KIT-065 # Evaluate requirements
```

### Diagnostics

```bash
/spec-consensus SPEC-KIT-065 plan
/spec-evidence-stats --spec SPEC-KIT-065
```

***

## Templates

### Template Inventory (14)

| Category          | Templates                                       | Count |
| ----------------- | ----------------------------------------------- | ----- |
| **Stages**        | plan, tasks, implement, validate, audit, unlock | 6     |
| **Quality Gates** | clarify, analyze, checklist                     | 3     |
| **Documents**     | prd, spec                                       | 2     |
| **Instructions**  | claude, agents, gemini                          | 3     |

### Command → Template Mapping

| Command           | Template File                                              |
| ----------------- | ---------------------------------------------------------- |
| speckit.new       | spec-template.md, PRD-template.md                          |
| speckit.project   | CLAUDE-template.md, AGENTS-template.md, GEMINI-template.md |
| speckit.clarify   | clarify-template.md                                        |
| speckit.analyze   | analyze-template.md                                        |
| speckit.checklist | checklist-template.md                                      |
| speckit.plan      | plan-template.md                                           |
| speckit.tasks     | tasks-template.md                                          |
| speckit.implement | implement-template.md                                      |
| speckit.validate  | validate-template.md                                       |
| speckit.audit     | audit-template.md                                          |
| speckit.unlock    | unlock-template.md                                         |

**All 14 templates embedded in binary and actively used.**

***

## /speckit.new Deep Dive

**Command**: `/speckit.new <feature-description>` (formerly `/new-spec`)
**Tier**: 2 (Triple agent: gemini, claude, code)
**Performance**: \~13 min, \~$0.60 (55% faster than baseline)

### Execution Phases

**Phase 1: Generate SPEC-ID**

* Runs `generate_spec_id.py` to create ID (e.g., SPEC-KIT-020-dark-mode-toggle)
* Creates `docs/SPEC-{ID}/` directory
* Adds row to SPEC.md table

**Phase 2: Multi-Agent PRD Generation**

* Uses `templates/PRD-template.md`
* 3 agents draft PRD.md with consensus
* P1/P2/P3 scenarios structure

**Phase 3: Multi-Agent Planning**

* Uses `templates/plan-template.md`
* 3 agents create work breakdown
* GitHub-style plan structure

**Phase 4: Task Decomposition**

* Uses `templates/tasks-template.md`
* Checkbox task lists with dependencies
* Updates SPEC.md Tasks table

**Phase 5: Present Package**

* Shows created files and evidence paths
* Suggests quality checks or automation

### Output Structure

```
docs/SPEC-KIT-020-dark-mode-toggle/
├── PRD.md          (acceptance criteria, P1/P2/P3 scenarios)
├── plan.md         (work breakdown, consensus, acceptance mapping)
└── tasks.md        (checkbox task list, dependencies)

SPEC.md             (table row added)
```

### Next Steps After /speckit.new

**Quality Checks (Recommended)**:

```bash
/speckit.clarify SPEC-KIT-020
/speckit.analyze SPEC-KIT-020
/speckit.checklist SPEC-KIT-020
```

**Full Automation**:

```bash
/speckit.auto SPEC-KIT-020
```

***

## CI/CD Integration

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

### Exit Code Contract

| Exit Code | Meaning                 | Action              |
| --------- | ----------------------- | ------------------- |
| 0         | Success / Ready         | Proceed             |
| 1         | Soft failure (warnings) | Review, may proceed |
| 2         | Hard failure (blocked)  | Fix blockers, retry |
| 3         | Infrastructure error    | Debug/escalate      |

### JSON Schema Versioning

All JSON outputs include:

* `schema_version`: Integer, bumped only on **breaking** changes
* `tool_version`: Cargo version + git SHA (format: `0.0.0+abc123`)

**Compatibility policy**:

* Additive changes (new fields) do NOT bump version
* Removed/renamed fields bump version
* Semantic changes to existing fields bump version

***

## Global Options & Parity

### Global Options

Available on all commands:

| Flag                       | Description                             |
| -------------------------- | --------------------------------------- |
| `-C, --cwd <DIR>`          | Working directory (defaults to current) |
| `-c, --config <key=value>` | Override configuration value            |
| `-h, --help`               | Print help                              |

### TUI/CLI Parity

The CLI uses the same `SpeckitExecutor` as TUI slash commands:

| TUI Command               | CLI Equivalent                       |
| ------------------------- | ------------------------------------ |
| `/speckit.status SPEC-ID` | `code speckit status --spec SPEC-ID` |
| `/speckit.plan SPEC-ID`   | `code speckit plan --spec SPEC-ID`   |
| `/review plan`            | `code speckit review --stage plan`   |

**Parity verified by unit tests** in `spec-kit/src/executor/mod.rs`.

### Alias Mapping (Legacy Names)

| Legacy Name     | Modern Name        |
| --------------- | ------------------ |
| /new-spec       | /speckit.new       |
| /spec-plan      | /speckit.plan      |
| /spec-tasks     | /speckit.tasks     |
| /spec-implement | /speckit.implement |
| /spec-validate  | /speckit.validate  |
| /spec-audit     | /speckit.audit     |
| /spec-unlock    | /speckit.unlock    |
| /spec-status    | /speckit.status    |
| /project        | /speckit.project   |

**All legacy names work for backward compatibility.**

***

## Change History

| Version | Date       | Changes                                                                                                    |
| ------- | ---------- | ---------------------------------------------------------------------------------------------------------- |
| v1.0.0  | 2026-01-22 | Initial canonical version (consolidated from CLI-REFERENCE.md, COMMAND\_INVENTORY.md, new-spec-command.md) |

***

**Navigation**: [INDEX.md](INDEX.md) | [POLICY.md](POLICY.md) | [KEY\_DOCS.md](KEY_DOCS.md)
