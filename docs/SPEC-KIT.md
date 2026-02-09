# Spec-Kit Reference (v1.0.1)

**Last Updated**: 2026-01-29
**Status**: Canonical (consolidated from SPEC-KIT-QUALITY-GATES.md, SPEC-KIT-CLI.md, SPEC-KIT-ARCHITECTURE.md)

***

## Table of Contents

* [What is Spec-Kit](#what-is-spec-kit)
  * [Product Promise](#product-promise)
  * [Invariants](#invariants)
  * [Surfaces](#surfaces)
* [Commands and Workflows](#commands-and-workflows)
  * [Quick Start](#quick-start)
  * [Command Inventory](#command-inventory)
    * [Intake (2)](#intake-2)
    * [Quality (3)](#quality-3)
    * [Stages (6)](#stages-6)
    * [Automation (2)](#automation-2)
    * [Guardrails (7)](#guardrails-7)
    * [Utility (3)](#utility-3)
  * [Workflows](#workflows)
* [Execution Model](#execution-model)
  * [6-Stage Pipeline](#6-stage-pipeline)
  * [Quality Gate Checkpoints](#quality-gate-checkpoints)
  * [Multi-Agent Synthesis](#multi-agent-synthesis)
  * [Tiered Model Strategy](#tiered-model-strategy)
* [Policies and Capture](#policies-and-capture)
  * [Resolution Logic](#resolution-logic)
  * [Escalation Rules](#escalation-rules)
  * [Hermetic Isolation](#hermetic-isolation)
  * [Evidence Storage](#evidence-storage)
* [Troubleshooting](#troubleshooting)
* [Reference Appendix](#reference-appendix)
  * [CLI Flags Reference](#cli-flags-reference)
  * [Exit Code Contract](#exit-code-contract)
  * [Environment Variables](#environment-variables)
  * [Template Inventory](#template-inventory)
  * [Agent Roster](#agent-roster)
  * [Telemetry Schema](#telemetry-schema)
* [Change History](#change-history)

## What is Spec-Kit

### Product Promise

Spec-kit provides single-owner multi-agent development automation for feature specifications. It transforms natural language feature descriptions into validated, implemented code through a 6-stage pipeline.

**Key Value**:

* Autonomous automation (\~55% auto-resolution)
* Multi-agent coordination (3-5 agents)
* Quality gates at 3 checkpoints
* Full pipeline: \~$2.70, \~60 min

### Invariants

| Principle                       | Description                                                       |
| ------------------------------- | ----------------------------------------------------------------- |
| **Agents for reasoning**        | Strategic decisions use agents; pattern matching uses native Rust |
| **Quality gates before commit** | No code changes without passing constitution checks and gates     |
| **Hermetic isolation**          | Agents operate in controlled, reproducible environments           |
| **Evidence-driven**             | All decisions logged with metadata for audit                      |
| **Tiered parity (D113/D133)**   | Tier 1 automation semantics match across TUI/CLI/headless (artifacts, gating, exit codes); visualization may be TUI-first |

### Surfaces

| Surface | Access                | Description                                 |
| ------- | --------------------- | ------------------------------------------- |
| **TUI** | `/speckit.*` commands | Interactive spec-kit in terminal UI         |
| **CLI** | `code speckit`        | Headless automation, CI/CD integration      |
| **MCP** | Native integration    | 5.3x faster than subprocess (8.7ms typical) |

> Parity note: per D113/D133, Tier 1 automation features are required to work with full parity across TUI/CLI/headless.

### Terminology: Work Items and "SPEC Packets"

In Codex-RS, the **uniform unit of work** is a **work item** identified by a `SPEC-*` ID and tracked in the canonical tracker (`codex-rs/SPEC.md`).

A work item’s documentation is stored as a **SPEC packet**: a directory under `docs/` containing **multiple artifacts**, for example:

- `PRD.md` — product requirements (“what/why”)
- `spec.md` — design / interface / architecture (“how”)
- `plan.md`, `tasks.md`, etc. — pipeline outputs

This is *not* “PRDs vs SPECS” as competing concepts — both are just artifacts within the same work item packet.

***

## Commands and Workflows

### Quick Start

```bash
# Create new SPEC from description
/speckit.new Add OAuth2 authentication

# Check status
/speckit.status SPEC-KIT-065

# Run full automation
/speckit.auto SPEC-KIT-065

# CLI equivalent (for CI/CD)
code speckit status --spec SPEC-KIT-065 --json
code speckit run --spec SPEC-KIT-065 --from plan --to audit --json
```

### Command Inventory

**Total**: 23 command structs, 40 names (primary + aliases)

#### Intake (2)

| Command            | Description                  | Agents               | Time     |
| ------------------ | ---------------------------- | -------------------- | -------- |
| `/speckit.new`     | Create SPEC from description | gemini, claude, code | \~13 min |
| `/speckit.specify` | Generate PRD with synthesis  | gemini, claude, code | 8-12 min |

#### Quality (3)

| Command              | Description                      | Agents               | Time     |
| -------------------- | -------------------------------- | -------------------- | -------- |
| `/speckit.clarify`   | Resolve spec ambiguities         | gemini, claude, code | 8-12 min |
| `/speckit.analyze`   | Check cross-artifact consistency | gemini, claude, code | 8-12 min |
| `/speckit.checklist` | Evaluate requirement quality     | claude, code         | 5-8 min  |

#### Stages (6)

| Command              | Description                | Template              | Time      |
| -------------------- | -------------------------- | --------------------- | --------- |
| `/speckit.plan`      | Create work breakdown      | plan-template.md      | 8-12 min  |
| `/speckit.tasks`     | Generate task list         | tasks-template.md     | 8-12 min  |
| `/speckit.implement` | Write code with validation | implement-template.md | 15-20 min |
| `/speckit.validate`  | Run test strategy          | validate-template.md  | 10-12 min |
| `/speckit.audit`     | Compliance review          | audit-template.md     | 10-12 min |
| `/speckit.unlock`    | Final approval for merge   | unlock-template.md    | 10-12 min |

#### Automation (2)

| Command           | Description             | Time     | Cost    |
| ----------------- | ----------------------- | -------- | ------- |
| `/speckit.auto`   | Full 6-stage pipeline   | \~60 min | \~$2.70 |
| `/speckit.status` | SPEC progress dashboard | <1s      | $0      |

#### Guardrails (7)

| Command                | Script                  |
| ---------------------- | ----------------------- |
| `/guardrail.plan`      | spec\_ops\_plan.sh      |
| `/guardrail.tasks`     | spec\_ops\_tasks.sh     |
| `/guardrail.implement` | spec\_ops\_implement.sh |
| `/guardrail.validate`  | spec\_ops\_validate.sh  |
| `/guardrail.audit`     | spec\_ops\_audit.sh     |
| `/guardrail.unlock`    | spec\_ops\_unlock.sh    |
| `/guardrail.auto`      | spec\_auto.sh           |

#### Utility (3)

| Command                | Description                                      |
| ---------------------- | ------------------------------------------------ |
| `/speckit.project`     | Scaffold new project (rust/python/go/typescript) |
| `/spec-consensus`      | Check stage synthesis status                     |
| `/spec-evidence-stats` | Summarize evidence sizes                         |

### Workflows

**Full Automation**:

```bash
/speckit.auto SPEC-KIT-065
/speckit.auto SPEC-KIT-065 --from tasks  # Resume
```

**Manual Stage-by-Stage**:

```bash
/speckit.plan SPEC-KIT-065
/speckit.tasks SPEC-KIT-065
/speckit.implement SPEC-KIT-065
/speckit.validate SPEC-KIT-065
/speckit.audit SPEC-KIT-065
/speckit.unlock SPEC-KIT-065
```

**Quality Checks**:

```bash
/speckit.clarify SPEC-KIT-065
/speckit.analyze SPEC-KIT-065
/speckit.checklist SPEC-KIT-065
```

***

## Execution Model

### 6-Stage Pipeline

```
/speckit.auto SPEC-KIT-065

┌─────────────────┐    ┌─────────────────┐    ┌─────────────────┐
│ QG: Pre-Plan    │    │ QG: Post-Plan   │    │ QG: Post-Tasks  │
│ (clarify+check) │    │ (analyze)       │    │ (analyze)       │
└────────┬────────┘    └────────┬────────┘    └────────┬────────┘
         │                      │                      │
         v                      v                      v
    ┌────────┐            ┌────────┐            ┌──────────┐
    │  Plan  │ ────────>  │ Tasks  │ ────────>  │Implement │
    └────────┘            └────────┘            └──────────┘
                                                      │
         ┌────────────────────────────────────────────┘
         v
    ┌──────────┐      ┌───────┐      ┌────────┐
    │ Validate │ ──>  │ Audit │ ──>  │ Unlock │
    └──────────┘      └───────┘      └────────┘
```

**Expected per pipeline**:

* 3 interruption points (batched questions)
* \~5 questions total
* 12-17 auto-resolutions applied
* \~60 min total

### Quality Gate Checkpoints

| Checkpoint   | When                          | Gates               | Purpose                                 |
| ------------ | ----------------------------- | ------------------- | --------------------------------------- |
| Pre-Planning | After SPEC, before plan       | Clarify + Checklist | Resolve ambiguities, score requirements |
| Post-Plan    | After plan, before tasks      | Analyze             | Check plan ↔ spec consistency           |
| Post-Tasks   | After tasks, before implement | Analyze             | Verify task coverage                    |

**Quality Gate Details**:

**QG1 (Clarify)**: Identifies ambiguous requirements, classifies by confidence/magnitude/resolvability

**QG2 (Checklist)**: Scores requirements (0-10) on specificity, testability, completeness, clarity

**QG3/QG4 (Analyze)**: Checks consistency between artifacts, identifies coverage gaps

### Multi-Agent Synthesis

**GR-001 Single-Owner Model**:

Spec-Kit follows the GR-001 policy: single-owner stages with quality gates, not multi-agent voting.

| Component          | Description                                              |
| ------------------ | -------------------------------------------------------- |
| **Single Owner**   | Each stage has one authoritative agent                   |
| **Quality Gates**  | Constitution checks, compiler, tests                     |
| **Critic Sidecar** | Optional non-authoritative feedback (risk triggers only) |
| **Synthesis**      | Agent outputs are synthesized, not voted on              |

**Pipeline Flow**:

```
Stage 0 -> Single Architect -> Single Implementer -> Single Judge
               (optional critic sidecar if triggered)
```

**Synthesis Quorum** (for multi-agent stages):

| Status       | Condition                               | Action               |
| ------------ | --------------------------------------- | -------------------- |
| **OK**       | Stage owner confident, gates pass       | Advance              |
| **Degraded** | 2/3 agents completed (Tier 2)           | Advance with warning |
| **Blocked**  | Gate failure or owner < 0.75 confidence | Escalate             |

### Tiered Model Strategy

| Tier           | Commands                                                             | Agents                                   | Time      | Cost         |
| -------------- | -------------------------------------------------------------------- | ---------------------------------------- | --------- | ------------ |
| **0: Native**  | status, project                                                      | 0                                        | <1s       | $0           |
| **2-lite**     | checklist                                                            | 2 (claude, code)                         | 5-8 min   | \~$0.35      |
| **2: Triple**  | new, specify, clarify, analyze, plan, tasks, validate, audit, unlock | 3                                        | 8-12 min  | \~$0.60-1.00 |
| **3: Quad**    | implement                                                            | 4 (gemini, claude, gpt\_codex, gpt\_pro) | 15-20 min | \~$2.00      |
| **4: Dynamic** | auto                                                                 | 3-5 (adaptive)                           | \~60 min  | \~$2.70      |

**Why Quad for implement?** Code generation benefits from diverse tool stacks (OpenAI + Anthropic) producing stronger diffs.

***

## Policies and Capture

### Resolution Logic

**Classification Dimensions**:

* **Confidence**: high (>90%), medium (70-90%), low (<70%)
* **Magnitude**: critical, important, minor
* **Resolvability**: auto-fix, suggest-fix, need-human

**Decision Matrix**:

| Confidence | Magnitude          | Resolvable       | Action   |
| ---------- | ------------------ | ---------------- | -------- |
| high       | minor              | auto-fix/suggest | AUTO     |
| high       | important          | auto-fix         | AUTO     |
| high       | important          | suggest          | CONFIRM  |
| high       | critical           | any              | ESCALATE |
| medium     | minor              | auto-fix         | AUTO     |
| medium     | minor              | suggest          | CONFIRM  |
| medium     | important/critical | any              | ESCALATE |
| low        | any                | any              | ESCALATE |

**Resolution Algorithm**:

* Unanimous (3/3) → Auto-apply
* Majority (2/3) → GPT-5 validate → Auto-apply or escalate
* No quorum → Escalate

**Expected**: \~55% auto-apply, \~45% escalate

### Escalation Rules

| Trigger                   | Action                                                                                           |
| ------------------------- | ------------------------------------------------------------------------------------------------ |
| Synthesis quorum degraded | Rerun with `gemini-2.5-pro` (thinking budget 0.6), reissue arbiter with `gpt-5 --reasoning high` |
| Thinking budget exhausted | Promote `gemini-2.5-flash` to Pro                                                                |
| Guardrail parsing failure | Retry with `gpt-5-codex`, escalate to `gpt-5`                                                    |
| Agent unavailability      | Continue with 2/3 agents (minimum 2 required)                                                    |

### Hermetic Isolation

**Principle**: Agents operate in controlled, reproducible environments independent of user-specific global configurations.

**Template Resolution Order**:

```
1. Project-local:  ./templates/{name}-template.md  (highest priority)
2. Embedded:       Compiled into binary            (always available)

NOT checked:     ~/.config/code/templates/       (breaks hermeticity)
```

**Required Instruction Files**: `CLAUDE.md`, `AGENTS.md`, `GEMINI.md`

**Scaffolding**: `/speckit.project rust my-project`

### Evidence Storage

```
docs/SPEC-KIT-*/evidence/
├── synthesis/
│   └── {stage}_{timestamp}_verdict.json
└── commands/
    └── {stage}_{timestamp}_telemetry.json
```

**Limit**: 25 MB soft limit per SPEC. Monitor with `/spec-evidence-stats`.

***

## Troubleshooting

| Issue                                    | Resolution                                                    |
| ---------------------------------------- | ------------------------------------------------------------- |
| **"MCP manager not initialized"**        | Auto-handled by retry logic. Verify: `local-memory --version` |
| **"No consensus artifacts found"**       | Check: `/spec-evidence-stats --spec SPEC-ID`                  |
| **"Consensus degraded: missing agents"** | 2/3 agents still valid. Check TUI history for errors.         |
| **"Evidence footprint exceeds 25MB"**    | Archive old SPECs. Run `/spec-evidence-stats`.                |
| **"Template not found"**                 | Check: `./templates/` → embedded (no global config)           |
| **GPT-5 validation fails**               | Export `OPENAI_API_KEY`. Check API status.                    |
| **Quality gate hangs**                   | Check agent logs, retry pipeline.                             |
| **Too many escalations**                 | SPEC poorly specified. Improve before automation.             |

For deep runbooks, see [OPERATIONS.md](OPERATIONS.md).

***

## Reference Appendix

### CLI Flags Reference

**Global Options**:

| Flag                       | Description            |
| -------------------------- | ---------------------- |
| `-C, --cwd <DIR>`          | Working directory      |
| `-c, --config <key=value>` | Override configuration |
| `-h, --help`               | Print help             |

**Status Command**:

```bash
code speckit status --spec <SPEC-ID> [--stale-hours N] [--json]
```

**Review Command**:

```bash
code speckit review --spec <SPEC-ID> --stage <STAGE> \
  [--strict-artifacts] [--strict-warnings] [--strict-schema] \
  [--explain] [--json]
```

**Stage Commands**:

```bash
code speckit {plan|tasks|implement|validate|audit|unlock} \
  --spec <SPEC-ID> [--dry-run] [--strict-prereqs] [--json]
```

**Run Command**:

```bash
code speckit run --spec <SPEC-ID> --from <STAGE> --to <STAGE> [--json]
```

### Exit Code Contract

| Code | Meaning                 | Action              |
| ---- | ----------------------- | ------------------- |
| 0    | Success / Ready         | Proceed             |
| 1    | Soft failure (warnings) | Review, may proceed |
| 2    | Hard failure (blocked)  | Fix blockers, retry |
| 3    | Infrastructure error    | Debug/escalate      |

### Environment Variables

| Variable                          | Values             | Effect                        |
| --------------------------------- | ------------------ | ----------------------------- |
| `OPENAI_API_KEY`                  | `sk-...`           | Required for GPT-5 validation |
| `SPEC_KIT_QUALITY_GATES_DISABLED` | `1`                | Disable quality gates         |
| `SPEC_KIT_SKIP_ISOLATION`         | `1`, `true`, `yes` | Skip pre-spawn validation     |
| `SPEC_OPS_HAL_SKIP`               | `1`                | Skip HAL validation           |
| `SPEC_OPS_ALLOW_DIRTY`            | `1`                | Allow dirty git tree          |

### Template Inventory

| Category          | Templates                                       | Count |
| ----------------- | ----------------------------------------------- | ----- |
| **Stages**        | plan, tasks, implement, validate, audit, unlock | 6     |
| **Quality Gates** | clarify, analyze, checklist                     | 3     |
| **Documents**     | prd, spec                                       | 2     |
| **Instructions**  | claude, agents, gemini                          | 3     |

**Total**: 14 templates embedded in binary

### Agent Roster

| Agent            | Model                 | Role                 | Used In                   |
| ---------------- | --------------------- | -------------------- | ------------------------- |
| gemini-25-flash  | gemini-2.5-flash      | Cheap research       | plan, validate            |
| claude-haiku-45  | claude-3.5-haiku      | Cheap validation     | plan, validate, implement |
| gpt5-low         | gpt-5 (low effort)    | Simple analysis      | specify, tasks            |
| gpt5-medium      | gpt-5 (medium effort) | Planning             | plan, validate            |
| gpt5-high        | gpt-5 (high effort)   | Critical decisions   | audit, unlock             |
| gpt\_codex       | gpt-5-codex (HIGH)    | Code generation      | implement only            |
| gemini-25-pro    | gemini-2.5-pro        | Premium reasoning    | audit, unlock             |
| claude-sonnet-45 | claude-4.5-sonnet     | Premium analysis     | audit, unlock             |
| code             | Native Rust           | Zero-cost heuristics | Tier 0 commands           |

### Telemetry Schema

```json
{
  "command": "quality-gate",
  "specId": "SPEC-KIT-065",
  "checkpoint": "pre-planning",
  "gates": ["clarify", "checklist"],
  "timestamp": "2025-10-16T20:00:00Z",
  "schemaVersion": "v1.1",
  "agents": ["gemini", "claude", "code"],
  "results": { ... },
  "summary": {
    "total_issues": 7,
    "auto_resolved": 5,
    "escalated": 2
  }
}
```

**JSON outputs include**: `schema_version` (integer, breaking changes only), `tool_version` (cargo + git SHA)

***

## Change History

| Version | Date       | Changes                                                                                                            |
| ------- | ---------- | ------------------------------------------------------------------------------------------------------------------ |
| v1.0.1  | 2026-01-29 | Document Tier 1 multi-surface parity requirement (D113/D133)                                                       |
| v1.0.0  | 2026-01-22 | Initial canonical version (consolidated from SPEC-KIT-QUALITY-GATES.md, SPEC-KIT-CLI.md, SPEC-KIT-ARCHITECTURE.md) |

***

**Navigation**: [INDEX.md](INDEX.md) | [POLICY.md](POLICY.md) | [OPERATIONS.md](OPERATIONS.md) | [KEY\_DOCS.md](KEY_DOCS.md)
