# Spec-Kit Architecture (v1.0.0)

**Last Updated**: 2026-01-22
**Status**: Consolidated from MULTI-AGENT-ARCHITECTURE.md, model-strategy.md, consensus-runner-design.md, HERMETIC-ISOLATION.md

***

## Table of Contents

* [Overview](#overview)
* [Tiered Model Strategy](#tiered-model-strategy)
  * [Tier 0: Native (0 agents)](#tier-0-native-0-agents)
  * [Tier 2-lite: Dual Agent (2 agents)](#tier-2-lite-dual-agent-2-agents)
  * [Tier 2: Triple Agent (3 agents)](#tier-2-triple-agent-3-agents)
  * [Tier 3: Quad Agent (4 agents)](#tier-3-quad-agent-4-agents)
  * [Tier 4: Dynamic (3-5 agents)](#tier-4-dynamic-3-5-agents)
  * [Command → Tier Mapping](#command--tier-mapping)
* [Agent Roster & Responsibilities](#agent-roster--responsibilities)
* [Consensus Workflow](#consensus-workflow)
  * [5-Step Consensus Process](#5-step-consensus-process)
  * [Classification Rules](#classification-rules)
  * [Retry Logic](#retry-logic)
  * [Escalation Rules](#escalation-rules)
* [Hermetic Isolation](#hermetic-isolation)
  * [Design Principles](#design-principles)
  * [Template Resolution Order](#template-resolution-order)
  * [Pre-Spawn Validation](#pre-spawn-validation)
  * [Environment Variables](#environment-variables)
  * [Project Scaffolding](#project-scaffolding)
* [Implementation Details](#implementation-details)
  * [Technical Architecture](#technical-architecture)
  * [Template System](#template-system)
  * [Evidence Repository](#evidence-repository)
  * [Prompt Metadata Requirements](#prompt-metadata-requirements)
* [Operational Reference](#operational-reference)
  * [Consensus Runner](#consensus-runner)
  * [Multi-IDE Integration](#multi-ide-integration)
  * [ACE Playbook Integration](#ace-playbook-integration)
* [Troubleshooting](#troubleshooting)
* [Change History](#change-history)

## Overview

Spec-kit uses a multi-agent consensus system for complex development stages. Agents from different providers (OpenAI, Anthropic, Google) collaborate to produce validated outputs.

**Key Metrics**:

| Metric                 | Value                                        |
| ---------------------- | -------------------------------------------- |
| Native MCP integration | 5.3x faster than subprocess (8.7ms typical)  |
| Commands automated     | 13 `/speckit.*` commands                     |
| Cost reduction         | 75% via tiered model strategy (SPEC-KIT-070) |
| Full pipeline cost     | \~$2.70 (down from $11)                      |

**Architecture Principle**: "Agents for reasoning, NOT transactions"

* Pattern matching → Native Rust (FREE, instant)
* Strategic decisions → Multi-agent consensus (justified cost)
* Code generation → Specialist model (gpt-5-codex)

***

## Tiered Model Strategy

### Tier 0: Native (0 agents)

**Commands**: `/speckit.status`, `/speckit.project`

| Metric        | Value |
| ------------- | ----- |
| Response time | <1s   |
| Cost          | $0    |
| Token usage   | 0     |

**Implementation**: Pure Rust in `codex-rs/tui/src/spec_status.rs`. Reads evidence directory directly, no API calls.

### Tier 2-lite: Dual Agent (2 agents)

**Command**: `/speckit.checklist`

**Agents**:

* **Synthesizer**: `claude-4.5-sonnet` (requirement analysis)
* **Evaluator**: `code` (Claude Code - scoring and validation)

| Metric   | Value                             |
| -------- | --------------------------------- |
| Duration | 5-8 minutes                       |
| Cost     | \~$0.35                           |
| Mode     | Sequential (Claude → Code review) |

### Tier 2: Triple Agent (3 agents)

**Commands**: `/speckit.new`, `.specify`, `.clarify`, `.analyze`, `.plan`, `.tasks`, `.validate`, `.audit`, `.unlock`

**Agents**:

* **Research**: `gemini-2.5-pro` - Breadth, exploration, tool use
* **Synthesizer**: `claude-4.5-sonnet` - Precision, analysis, coding
* **Arbiter**: `gpt-5` or `code` - Conflict resolution, policy enforcement

| Metric   | Value                                |
| -------- | ------------------------------------ |
| Duration | 8-12 minutes                         |
| Cost     | \~$0.80-1.00                         |
| Mode     | Parallel spawn → Consensus synthesis |

**Agent allocation by command**:

* `.new`, `.specify`, `.clarify`, `.analyze`: gemini, claude, code
* `.plan`, `.tasks`, `.validate`, `.audit`, `.unlock`: gemini, claude, gpt\_pro

### Tier 3: Quad Agent (4 agents)

**Command**: `/speckit.implement` (code generation only)

**Agents**:

* **Research**: `gemini-2.5-pro` - Retrieve refs, APIs, prior art
* **Code Ensemble** (two-vote system):
  * `gpt-5-codex` (OpenAI implementation)
  * `claude-4.5-sonnet` (Anthropic implementation)
* **Arbiter**: `gpt-5` with `--reasoning high` - Merges best elements

| Metric   | Value                                      |
| -------- | ------------------------------------------ |
| Duration | 15-20 minutes                              |
| Cost     | \~$2.00                                    |
| Mode     | Parallel ensemble → Synthesis → Validation |

**Why Quad?** Code generation benefits from diverse tool stacks (OpenAI + Anthropic) producing stronger diffs than single-agent approaches.

### Tier 4: Dynamic (3-5 agents)

**Command**: `/speckit.auto` (full 6-stage pipeline)

**Strategy**:

* Most stages: Use Tier 2 (3 agents)
* Code generation: Use Tier 3 (4 agents)
* Conflict resolution: Add arbiter dynamically (+1 if consensus fails)

| Metric   | Value                    |
| -------- | ------------------------ |
| Duration | 40-60 minutes (6 stages) |
| Cost     | \~$2.70 (previously $11) |
| Mode     | Adaptive per stage       |

**Cost breakdown**:

* 5 × Tier 2 stages: 5 × $0.35 = $1.75
* 1 × Tier 3 stage (implement): \~$0.60
* Orchestration overhead: \~$0.35
* **Total**: \~$2.70

### Command → Tier Mapping

| Tier            | Commands                                                                                                |
| --------------- | ------------------------------------------------------------------------------------------------------- |
| **Tier 0**      | `/speckit.status`, `/speckit.project`                                                                   |
| **Tier 2-lite** | `/speckit.checklist`                                                                                    |
| **Tier 2**      | `/speckit.new`, `.specify`, `.clarify`, `.analyze`, `.plan`, `.tasks`, `.validate`, `.audit`, `.unlock` |
| **Tier 3**      | `/speckit.implement`                                                                                    |
| **Tier 4**      | `/speckit.auto`                                                                                         |

***

## Agent Roster & Responsibilities

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

**Model Responsibilities**:

* **Gemini 2.5 Pro**: Breadth, tool use, wide context windows. Flash mode for quick pre-scans.
* **Claude 4.5 Sonnet**: Precision, code quality, autonomous sessions. Default synthesizer.
* **GPT-5**: Conflict resolution, policy enforcement. Escalate with `--reasoning high`.
* **GPT-5-Codex**: Code generation, implementation diffs. Combines with Claude for two-vote system.
* **Code (Claude Code)**: Orchestration, fallback, broad capability.

***

## Consensus Workflow

### 5-Step Consensus Process

1. **Spawn**: Agents execute in parallel for stage
2. **Store**: Each agent writes analysis to local-memory with tags `spec:SPEC-ID`, `stage:NAME`
3. **Fetch**: `check_consensus_and_advance_spec_auto()` retrieves via MCP (8.7ms avg)
4. **Validate**: Check participation, extract gpt\_pro consensus, detect conflicts
5. **Advance**: Move to next stage or retry (max 3x)

### Classification Rules

| Status           | Condition                        | Action               |
| ---------------- | -------------------------------- | -------------------- |
| **OK**           | All agents present, no conflicts | Advance              |
| **Degraded**     | 2/3 agents, no conflicts         | Advance with warning |
| **Conflict**     | Non-empty conflicts array        | Retry or escalate    |
| **No consensus** | <50% participation               | Retry                |

### Retry Logic

| Trigger               | Max Attempts | Backoff                  |
| --------------------- | ------------ | ------------------------ |
| Empty/invalid results | 3            | 100→200→400ms            |
| MCP "not initialized" | 3            | 100→200→400ms            |
| Validation failures   | 2            | Implement→Validate cycle |

### Escalation Rules

**Consensus degraded**: Rerun with `gemini-2.5-pro` (thinking budget 0.6), reissue arbiter with `gpt-5 --reasoning high`

**Thinking budget exhausted**: Promote `gemini-2.5-flash` to Pro, log retry

**Guardrail parsing failure**: Retry with `gpt-5-codex`, escalate to `gpt-5` (low effort), tag `guardrail_escalated=true`

**Agent unavailability**: Continue with 2/3 agents (minimum 2 required), document participation

**Offline mode**: Use on-prem fallbacks, record `"offline": true` in metadata

***

## Hermetic Isolation

Hermetic isolation (SPEC-KIT-964) ensures spawned agents operate in controlled, reproducible environments independent of user-specific global configurations.

### Design Principles

1. **Project-local first**: Templates resolve from `./templates/` before embedded
2. **No global fallback**: `~/.config/code/templates/` intentionally excluded
3. **Pre-spawn validation**: Check instruction files exist before agent spawn
4. **Graceful degradation**: Warn on missing files but don't block execution

### Template Resolution Order

```
1. Project-local:  ./templates/{name}-template.md  (highest priority)
2. Embedded:       Compiled into binary            (always available)

NOT checked:     ~/.config/code/templates/       (breaks hermeticity)
```

**Implementation**: `codex-rs/tui/src/templates/mod.rs`

### Pre-Spawn Validation

**Required instruction files**:

* `CLAUDE.md` - Claude/Anthropic agent instructions
* `AGENTS.md` - Multi-agent coordination rules
* `GEMINI.md` - Google Gemini agent instructions

**Implementation**: `codex-rs/tui/src/chatwidget/spec_kit/isolation_validator.rs`

**Call site**: `agent_orchestrator.rs:767` - Logs warning but doesn't block execution

### Environment Variables

| Variable                  | Values             | Effect                    |
| ------------------------- | ------------------ | ------------------------- |
| `SPEC_KIT_SKIP_ISOLATION` | `1`, `true`, `yes` | Skip pre-spawn validation |

**Use cases**: Development/testing, CI environments, legacy project migration

### Project Scaffolding

Create required instruction files with `/speckit.project`:

```bash
/speckit.project rust my-project    # Rust-specific CLAUDE.md
/speckit.project python my-project  # Python-specific CLAUDE.md
/speckit.project go my-project      # Go-specific CLAUDE.md
```

***

## Implementation Details

### Technical Architecture

**Main Files** (7,883 LOC total):

| File               | LOC   | Purpose                |
| ------------------ | ----- | ---------------------- |
| `handler.rs`       | 2,038 | Orchestration          |
| `consensus.rs`     | 992   | MCP native integration |
| `quality.rs`       | 807   | Quality gates          |
| `evidence.rs`      | 499   | Persistence            |
| `templates/mod.rs` | —     | Template resolution    |

**Key Functions**:

```rust
pub async fn run_spec_consensus(...) -> Result<(Vec<Line>, bool)>
async fn fetch_memory_entries(...) -> Result<Vec<LocalMemorySearchResult>>
async fn remember_consensus_verdict(...) -> Result<()>
pub fn resolve_template(name: &str) -> Result<String>
```

**MCP Timeouts**: Search 30s, Store 10s

### Template System

14 templates embedded in binary:

| Category          | Templates                                       |
| ----------------- | ----------------------------------------------- |
| **Stages**        | plan, tasks, implement, validate, audit, unlock |
| **Quality Gates** | clarify, analyze, checklist                     |
| **Documents**     | prd, spec                                       |
| **Instructions**  | claude, agents, gemini                          |

### Evidence Repository

```
docs/SPEC-OPS-004-integrated-coder-hooks/evidence/
├── consensus/SPEC-ID/
│   └── {stage}_{timestamp}_verdict.json
└── commands/SPEC-ID/
    └── {stage}_{timestamp}_telemetry.json
```

**Limit**: 25 MB soft limit per SPEC. Monitor with `/spec-evidence-stats`.

### Prompt Metadata Requirements

Every agent response for consensus must include:

```json
{
  "model": "<provider-model-id>",
  "model_release": "YYYY-MM-DD",
  "prompt_version": "YYYYMMDD-stage-suffix",
  "reasoning_mode": "fast|thinking|auto",
  "consensus": { "agreements": [], "conflicts": [] }
}
```

**Validation**: Consensus checker rejects artifacts missing these fields.

***

## Operational Reference

### Consensus Runner

**Entry point**: `scripts/spec_ops_004/consensus_runner.sh`

**Command flags**:

| Flag                    | Required | Description                                        |
| ----------------------- | -------- | -------------------------------------------------- |
| `--stage <stage>`       | Yes      | One of spec-plan, spec-tasks, spec-implement, etc. |
| `--spec <SPEC-ID>`      | Yes      | SPEC identifier                                    |
| `--context-file <path>` | No       | Additional context file                            |
| `--dry-run`             | No       | Render prompts only                                |
| `--execute`             | No       | Run agents (requires credentials)                  |
| `--allow-conflict`      | No       | Exit 0 even if conflicts detected                  |

**Template variables**: `${SPEC_ID}`, `${PROMPT_VERSION}`, `${MODEL_ID}`, `${CONTEXT}`, `${PREVIOUS_OUTPUTS.*}`

### Multi-IDE Integration

| IDE/CLI     | Config Location         | Model Default    |
| ----------- | ----------------------- | ---------------- |
| Claude Code | `~/.claude/`            | claude-opus-4-5  |
| Gemini CLI  | `.gemini/settings.json` | gemini-2.5-flash |
| Planner TUI | `~/.config/code/`       | gpt-5            |

**Note**: Gemini CLI uses its configured model directly; multi-agent consensus requires TUI or Claude Code.

### ACE Playbook Integration

The Agentic Context Engine (ACE) provides execution learning via `ace-playbook` MCP server.

**Workflow**:

1. `playbook_slice()` → Fetch relevant bullets for scope
2. Inject bullets into agent prompts
3. Execute stage agents
4. Collect execution feedback
5. `learn()` → Update bullet scores

**Scoring**: Success +1.0, Failure -0.6, Clamp range \[-2.0, +5.0]

***

## Troubleshooting

| Issue                                    | Resolution                                                                               |
| ---------------------------------------- | ---------------------------------------------------------------------------------------- |
| **"MCP manager not initialized"**        | Auto-handled by retry logic. Verify: `local-memory --version`                            |
| **"No consensus artifacts found"**       | Check: `/spec-evidence-stats --spec SPEC-ID`, `local-memory search "SPEC-ID stage:plan"` |
| **"Consensus degraded: missing agents"** | 2/3 agents still valid. Check TUI history for errors.                                    |
| **"Evidence footprint exceeds 25MB"**    | Archive old SPECs. Run `/spec-evidence-stats`.                                           |
| **"Template not found"**                 | Check resolution order: `./templates/` → embedded (no global config)                     |

***

## Change History

| Version | Date       | Changes                                                                                                                                         |
| ------- | ---------- | ----------------------------------------------------------------------------------------------------------------------------------------------- |
| v1.0.0  | 2026-01-22 | Initial canonical version (consolidated from MULTI-AGENT-ARCHITECTURE.md, model-strategy.md, consensus-runner-design.md, HERMETIC-ISOLATION.md) |

***

**Navigation**: [INDEX.md](INDEX.md) | [POLICY.md](POLICY.md) | [SPEC-KIT-CLI.md](SPEC-KIT-CLI.md) | [KEY\_DOCS.md](KEY_DOCS.md)
