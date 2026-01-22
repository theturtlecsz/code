# Operations Reference

> **Version**: 1.1.0 (2026-01-22)
>
> **Purpose**: Consolidated operational guidance and configuration reference for AI agents and developers.
>
> **Supersedes**: `docs/OPERATIONAL-PLAYBOOK.md`, `docs/config.md`

***

## Table of Contents

* [Part I: Operations Playbook](#part-i-operations-playbook)
  * [1. Agent Behavioral Guidance](#1-agent-behavioral-guidance)
    * [When To Pause And Ask](#when-to-pause-and-ask)
    * [Escalate Early](#escalate-early)
    * [Memory Workflow](#memory-workflow)
    * [NotebookLM Integration (SPEC-KIT-102)](#notebooklm-integration-spec-kit-102)
    * [Evidence & Validation Ritual](#evidence--validation-ritual)
    * [Telemetry Expectations](#telemetry-expectations)
    * [Deliverable Formats](#deliverable-formats)
      * [Plans (`docs/SPEC-<id>-<slug>/plan.md`)](#plans-docsspec-id-slugplanmd)
      * [Tasks (`docs/SPEC-<id>-<slug>/tasks.md` + SPEC.md)](#tasks-docsspec-id-slugtasksmd--specmd)
    * [Multi-Agent Expectations](#multi-agent-expectations)
    * [Config Isolation (SPEC-KIT-964)](#config-isolation-spec-kit-964)
    * [Reference Documents](#reference-documents)
  * [2. Runbook: CI & Gating](#2-runbook-ci--gating)
    * [Pre-Commit Validation](#pre-commit-validation)
    * [Quality Gate Commands](#quality-gate-commands)
    * [Evidence Capture](#evidence-capture)
  * [3. Runbook: Troubleshooting](#3-runbook-troubleshooting)
    * [If HAL Telemetry Fails](#if-hal-telemetry-fails)
    * [If Consensus Drift Occurs](#if-consensus-drift-occurs)
    * [If Dirty Tree Blocks Guardrails](#if-dirty-tree-blocks-guardrails)
    * [If Evidence Exceeds Limits](#if-evidence-exceeds-limits)
    * [If NotebookLM Is Unavailable](#if-notebooklm-is-unavailable)
    * [If MCP Server Fails to Start](#if-mcp-server-fails-to-start)
* [Part II: Configuration Reference](#part-ii-configuration-reference)
  * [4. Configuration Overview](#4-configuration-overview)
    * [Config File Location](#config-file-location)
    * [Generic Config Flag Syntax](#generic-config-flag-syntax)
  * [5. Model Settings](#5-model-settings)
    * [model](#model)
    * [model\_provider](#model_provider)
    * [model\_providers](#model_providers)
      * [Azure Example](#azure-example)
      * [Per-Provider Network Tuning](#per-provider-network-tuning)
    * [model\_reasoning\_effort](#model_reasoning_effort)
    * [model\_reasoning\_summary](#model_reasoning_summary)
    * [model\_verbosity](#model_verbosity)
    * [model\_context\_window / model\_max\_output\_tokens](#model_context_window--model_max_output_tokens)
  * [6. Execution Policies](#6-execution-policies)
    * [approval\_policy](#approval_policy)
    * [sandbox\_mode](#sandbox_mode)
      * [Workspace-Write Options](#workspace-write-options)
    * [profiles](#profiles)
    * [disable\_response\_storage](#disable_response_storage)
  * [7. MCP Servers](#7-mcp-servers)
    * [Configuration](#configuration)
    * [CLI Management](#cli-management)
  * [8. Validation & Hooks](#8-validation--hooks)
    * [Validation Groups](#validation-groups)
    * [GitHub Actions Lint](#github-actions-lint)
    * [Shell Environment Policy](#shell-environment-policy)
    * [Project Hooks](#project-hooks)
    * [Project Commands](#project-commands)
    * [Stage0 (NotebookLM) Config](#stage0-notebooklm-config)
  * [9. Config Key Reference](#9-config-key-reference)
    * [Nested Keys](#nested-keys)
* [Part III: Stage 0 Observability](#part-iii-stage-0-observability)
  * [10. Stage 0 Metrics](#10-stage-0-metrics)
    * [Core Metrics](#core-metrics)
    * [Suggested Dashboards](#suggested-dashboards)
  * [11. Stage 0 Events](#11-stage-0-events)
    * [Correlation IDs](#correlation-ids)
    * [Stage 0 Run Event](#stage-0-run-event)
    * [Guardian Events](#guardian-events)
    * [Cache Invalidation Events](#cache-invalidation-events)
    * [Explainability Snapshots](#explainability-snapshots)
    * [Storage Notes](#storage-notes)
* [Appendices](#appendices)
  * [A. Related Documentation](#a-related-documentation)
  * [B. Change History](#b-change-history)

# Part I: Operations Playbook

## 1. Agent Behavioral Guidance

### When To Pause And Ask

Stop and request clarification when:

* Missing or ambiguous acceptance criteria
* Spec requires external services unavailable here
* Security/privacy implications are unclear
* Legacy `specs/**` artifact touched—plan migration before editing
* Large refactor emerges unexpectedly
* Required reference documents (`product-requirements.md`, `PLANNING.md`, relevant spec files) are absent

### Escalate Early

* State blockers, degraded guardrails, or missing telemetry explicitly
* When HAL telemetry is missing or malformed, pause and re-run the relevant guardrail command (e.g., `/guardrail.plan`) with `SPEC_OPS_TELEMETRY_HAL=1` after restoring prerequisites
* For consensus drift (agents missing, conflicting verdicts), re-run the stage or run `/spec-consensus <SPEC-ID> <stage>` and include findings in the report

### Memory Workflow

Use **local-memory via CLI + REST only** for high-value knowledge (importance >= 8). Do not configure or call `local-memory` via MCP.

**Full protocol**: See `~/.local-memory/PROTOCOL.md` and `MEMORY-POLICY.md`.

**Quick reference**:

```bash
lm search "query" --limit 5      # Search
lm remember "WHAT: ...\nWHY: ...\nEVIDENCE: ...\nOUTCOME: ..." --type decision --importance 8  # Store
~/.claude/hooks/lm-dashboard.sh --compact  # Health
```

**Store**: Architecture decisions, reusable patterns, critical discoveries, milestones.
**Don't store**: Session summaries, progress updates, routine operations.

**Mandatory Session Workflow**:

1. **Session Start**: Query local-memory for project context, recent decisions, architecture state
2. **Before Tasks**: Search local-memory for relevant prior work, patterns, solutions
3. **During Work**: Store key decisions, architecture changes, bug discoveries (importance >= 8)
4. **After Milestones**: Store outcomes, file locations, validation results, lessons learned

### NotebookLM Integration (SPEC-KIT-102)

NotebookLM provides "Tier 2" reasoning for complex synthesis queries.

**When to Use**: Stage 0 planning, deep context synthesis, "WHY" questions.
**Rate Limit**: 50 queries/day (free tier). Cache aggressively.

**Quick reference**:

```bash
# Verify service (must be running)
curl -s localhost:3456/health | jq .authenticated

# Ask a question
curl -X POST localhost:3456/api/ask \
  -H "Content-Type: application/json" \
  -d '{"notebook": "...", "question": "..."}'  # aliases: notebook_id, notebook_url
```

**Service Management**:

```bash
notebooklm service start   # Start HTTP daemon
notebooklm service status  # Check status
notebooklm health --deep   # Verify authentication
```

**Full documentation**: See `docs/SPEC-KIT-102-notebooklm-integration/`.

### Evidence & Validation Ritual

* Guardrail runs must have a clean tree unless specifically allowed (`SPEC_OPS_ALLOW_DIRTY=1`)
* Capture both success and failure artifacts; `/speckit.auto` includes automatic retry (AR-2, AR-3) but document degradations
* After `/implement`, run the full validation harness (fmt, clippy, build/tests, doc validators). Attach logs or cite evidence files in local-memory and user reports
* Evidence growth policy: 25 MB soft limit per SPEC, monitor with `/spec-evidence-stats`. See `POLICY.md#3-evidence-policy` for retention/archival

### Telemetry Expectations

Telemetry schema v1 requires every JSON to have: `command`, `specId`, `sessionId`, `timestamp`, `schemaVersion`, `artifacts[]`

**Stage-specific fields**:

* **Plan**: `baseline.mode`, `baseline.artifact`, `baseline.status`, `hooks.session.start`
* **Tasks**: `tool.status`
* **Implement**: `lock_status`, `hook_status`
* **Validate/Audit**: `scenarios[{name,status}]` (`passed|failed|skipped`)
* **Unlock**: `unlock_status`

Enable `SPEC_OPS_TELEMETRY_HAL=1` during HAL smoke tests to capture `hal.summary.{status,failed_checks,artifacts}`.

`/guardrail.auto` halts on schema violations or missing artifacts. Investigate immediately.

**Evidence root**: `docs/SPEC-OPS-004-integrated-coder-hooks/evidence/`

### Deliverable Formats

#### Plans (`docs/SPEC-<id>-<slug>/plan.md`)

```markdown
# Plan: <feature / spec-id>

## Inputs
- Spec: docs/<id>-<slug>/spec.md (version/hash)
- Constitution: memory/constitution.md (version/hash)

## Work Breakdown
1. ...
2. ...

## Acceptance Mapping
| Requirement (Spec) | Validation Step | Test/Check Artifact |
| --- | --- | --- |
| R1: ... | ... | ... |

## Risks & Unknowns
- ...

## Consensus & Risks (Multi-AI)
- Agreement: ...
- Disagreement & resolution: ...

## Exit Criteria (Done)
- All acceptance checks pass
- Docs updated (list)
- Changelog/PR prepared
```

#### Tasks (`docs/SPEC-<id>-<slug>/tasks.md` + SPEC.md)

* Update SPEC.md's Tasks table every time a `/tasks` or `/implement` run changes state
* Columns: Order | Task ID | Title | Status | PRD | Branch | PR | Notes
* Status values: `Backlog`, `In Progress`, `In Review`, `Blocked`, `Done`
* On PR open: Status -> `In Review`, populate `Branch`
* On merge: Status -> `Done`, fill `PR`, add dated note referencing evidence

### Multi-Agent Expectations

* **Consensus is fully automated** via native integration (ARCH-002, 5.3x faster). All 13 `/speckit.*` commands operational
* **Agent roster**: Tier 2 uses gemini/claude/code (or gpt\_pro for dev stages), Tier 3 adds gpt\_codex, Tier 4 dynamically selects 3-5 agents
* **Degradation handling**: If agent fails, retry up to 3 times (AR-2). If still fails, continue with remaining agents (2/3 consensus still valid)
* **Consensus metadata**: Automatically records `agent`, `version`, `content` in local-memory. Synthesis includes `consensus_ok`, `degraded`, `missing_agents`, `conflicts[]`
* **Memory System**: Use local-memory via **CLI + REST only** (no MCP). Byterover deprecated 2025-10-18
* **Validation**: `/implement` runs `cargo fmt`, `cargo clippy`, build checks, tests before returning

See `docs/spec-kit/MULTI-AGENT-ARCHITECTURE.md` for detailed system documentation.

### Config Isolation (SPEC-KIT-964)

This project uses hermetic agent isolation:

* Templates resolve: `./templates/` -> embedded (NO global `~/.config/code/templates/`)
* Agents receive context ONLY from:
  * Project files (CLAUDE.md, AGENTS.md, GEMINI.md)
  * prompts.json (embedded)
  * MCP queries scoped by `project:theturtlecsz/code`

This ensures reproducible behavior regardless of user's global configuration.

### Reference Documents

Load these every session:

* `MEMORY-POLICY.md` - mandatory memory system policy
* `memory/constitution.md` - non-negotiable project charter
* `product-requirements.md` - canonical product scope
* `PLANNING.md` - high-level architecture, goals, constraints
* `SPEC.md` - single source of truth for task tracking

***

## 2. Runbook: CI & Gating

### Pre-Commit Validation

```bash
# From codex-rs/
cargo fmt --all -- --check
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo build --workspace --all-features
cargo test -p codex-core
```

### Quality Gate Commands

```bash
# Spec-kit pipeline
/speckit.auto SPEC-ID           # Full 6-stage pipeline
/speckit.status SPEC-ID         # Check SPEC status
/spec-evidence-stats            # Monitor evidence footprint

# Guardrails
/guardrail.plan                 # Plan stage validation
/guardrail.auto                 # Full validation (halts on schema violations)
```

### Evidence Capture

```bash
# Evidence structure
docs/SPEC-<id>-<slug>/evidence/
  ├── baselines/                # Pre-run state
  ├── artifacts/                # Generated outputs
  └── logs/                     # Command output

# Monitor size
/spec-evidence-stats            # 25 MB soft limit per SPEC
```

***

## 3. Runbook: Troubleshooting

### If HAL Telemetry Fails

1. Check `HAL_SECRET_KAVEDARR_API_KEY` is set (or set `SPEC_OPS_HAL_SKIP=1`)
2. Re-run with `SPEC_OPS_TELEMETRY_HAL=1`
3. Verify telemetry JSON has required fields

### If Consensus Drift Occurs

1. Check which agents are missing/failed
2. Re-run: `/spec-consensus <SPEC-ID> <stage>`
3. Document degradation in report
4. 2/3 consensus is still valid (AR-2 retry logic)

### If Dirty Tree Blocks Guardrails

```bash
# Option 1: Commit or stash changes
git stash

# Option 2: Allow dirty (not recommended for production)
SPEC_OPS_ALLOW_DIRTY=1 /guardrail.plan
```

### If Evidence Exceeds Limits

1. Check: `/spec-evidence-stats`
2. Archive old baselines: `scripts/docs-archive-pack.sh create <dir>`
3. See `POLICY.md#3-evidence-policy` for retention rules

### If NotebookLM Is Unavailable

1. Check service: `curl -s localhost:3456/health | jq`
2. Restart: `notebooklm service start`
3. Verify auth: `notebooklm health --deep`
4. Fallback: Use local-memory + web search for context

### If MCP Server Fails to Start

1. Check startup timeout: default is 10 seconds
2. Increase via `startup_timeout_sec` in config
3. Check logs for connection errors
4. Verify command path and args

***

# Part II: Configuration Reference

## 4. Configuration Overview

Planner supports several mechanisms for setting config values (highest precedence first):

1. **Command-line flags**: `--model o3`
2. **Generic config flag**: `-c key=value` or `--config key=value`
3. **Profile-scoped values**: `--profile <name>` selecting from `[profiles.<name>]`
4. **Config file**: `$CODE_HOME/config.toml` (defaults to `~/.code`; also reads `~/.codex` for compatibility)
5. **Built-in defaults**

### Config File Location

```
$CODE_HOME/config.toml   # Primary (defaults to ~/.code/)
$CODEX_HOME/config.toml  # Legacy fallback (~/.codex/)
```

### Generic Config Flag Syntax

```bash
# Simple key=value
code --config model="o3"

# Nested keys with dots
code --config model_providers.openai.wire_api="chat"

# TOML objects
code --config shell_environment_policy.include_only='["PATH", "HOME", "USER"]'
```

Values use TOML format. If parsing fails, the value is treated as a string.

***

## 5. Model Settings

### model

The model that Planner should use.

```toml
model = "o3"  # overrides the default of "gpt-5-codex"
```

### model\_provider

Identifies which provider from `model_providers` to use. Defaults to `"openai"`.

```toml
model_provider = "ollama"
model = "mistral"
```

### model\_providers

Override or extend the default model providers:

```toml
[model_providers.openai-chat-completions]
name = "OpenAI using Chat Completions"
base_url = "https://api.openai.com/v1"
env_key = "OPENAI_API_KEY"
wire_api = "chat"  # or "responses"
query_params = {}

[model_providers.ollama]
name = "Ollama"
base_url = "http://localhost:11434/v1"

[model_providers.mistral]
name = "Mistral"
base_url = "https://api.mistral.ai/v1"
env_key = "MISTRAL_API_KEY"
```

#### Azure Example

```toml
[model_providers.azure]
name = "Azure"
base_url = "https://YOUR_PROJECT_NAME.openai.azure.com/openai"
env_key = "AZURE_OPENAI_API_KEY"
query_params = { api-version = "2025-04-01-preview" }
wire_api = "responses"
```

#### Per-Provider Network Tuning

```toml
[model_providers.openai]
name = "OpenAI"
base_url = "https://api.openai.com/v1"
env_key = "OPENAI_API_KEY"
request_max_retries = 4            # HTTP retry count (default: 4)
stream_max_retries = 10            # SSE stream retry (default: 5)
stream_idle_timeout_ms = 300000    # 5m idle timeout (default: 300000)
```

### model\_reasoning\_effort

For models supporting reasoning (`o3`, `o4-mini`, `codex-*`, `gpt-5`, `gpt-5-codex`):

```toml
model_reasoning_effort = "medium"  # minimal | low | medium (default) | high
```

### model\_reasoning\_summary

```toml
model_reasoning_summary = "auto"  # auto (default) | concise | detailed | none
```

### model\_verbosity

Controls output length on GPT-5 family (Responses API only):

```toml
model_verbosity = "medium"  # low | medium (default) | high
```

### model\_context\_window / model\_max\_output\_tokens

Override context window or max output tokens for custom models:

```toml
model_context_window = 128000
model_max_output_tokens = 16384
```

***

## 6. Execution Policies

### approval\_policy

When to prompt for command approval:

```toml
approval_policy = "untrusted"  # Prompt for non-trusted commands (default)
approval_policy = "on-failure"  # Prompt when sandbox fails
approval_policy = "on-request"  # Model decides when to escalate
approval_policy = "never"       # Never prompt (exec mode default)
```

### sandbox\_mode

OS-level sandboxing for shell commands:

```toml
sandbox_mode = "read-only"           # Read any file, block writes/network (default)
sandbox_mode = "workspace-write"     # cwd + $TMPDIR writable
sandbox_mode = "danger-full-access"  # No sandbox (use in containers)
```

#### Workspace-Write Options

```toml
[sandbox_workspace_write]
exclude_tmpdir_env_var = false  # Exclude $TMPDIR from writable
exclude_slash_tmp = false       # Exclude /tmp from writable
allow_git_writes = true         # Allow writes to .git/ (default: true)
writable_roots = ["/Users/YOU/.pyenv/shims"]  # Additional writable paths
network_access = false          # Allow outbound network (default: false)
```

### profiles

Define named configuration profiles:

```toml
profile = "o3"  # Default profile

[profiles.o3]
model = "o3"
model_provider = "openai"
approval_policy = "never"
model_reasoning_effort = "high"

[profiles.gpt3]
model = "gpt-3.5-turbo"
model_provider = "openai-chat-completions"
```

Use with: `code --profile o3`

### disable\_response\_storage

Required for Zero Data Retention (ZDR) accounts:

```toml
disable_response_storage = true
```

***

## 7. MCP Servers

### Configuration

```toml
[mcp_servers.server-name]
command = "npx"
args = ["-y", "mcp-server"]
env = { "API_KEY" = "value" }
startup_timeout_sec = 20   # Default: 10
tool_timeout_sec = 30      # Default: 60
```

### CLI Management

```bash
code mcp add docs -- docs-server --port 4000
code mcp list
code mcp list --json
code mcp get docs
code mcp remove docs
```

***

## 8. Validation & Hooks

### Validation Groups

```toml
[validation.groups]
functional = true   # Catch regressions (default: on)
stylistic = false   # Formatting feedback (default: off)

[validation.tools]
shellcheck = true
markdownlint = true
cargo-check = true
tsc = true
eslint = true
mypy = true
pyright = true
golangci-lint = true
```

### GitHub Actions Lint

```toml
[github]
actionlint_on_patch = true
actionlint_path = "/usr/local/bin/actionlint"  # Optional
```

### Shell Environment Policy

```toml
[shell_environment_policy]
inherit = "all"                    # all (default) | core | none
ignore_default_excludes = false    # Skip KEY/SECRET/TOKEN filter
exclude = ["AWS_*", "AZURE_*"]     # Additional exclusions
set = { CI = "1" }                 # Force-set values
include_only = ["PATH", "HOME"]   # Whitelist (if non-empty)
```

### Project Hooks

```toml
[projects."/path/to/project"]
trust_level = "trusted"

[[projects."/path/to/project".hooks]]
name = "bootstrap"
event = "session.start"
run = ["./scripts/bootstrap.sh"]
timeout_ms = 60000

[[projects."/path/to/project".hooks]]
event = "tool.after"
run = "npm run lint -- --changed"
```

**Hook Events**:

* `session.start` - After session configured
* `session.end` - Before shutdown
* `tool.before` - Before each exec/tool command
* `tool.after` - After each exec/tool command
* `file.before_write` - Before apply\_patch
* `file.after_write` - After apply\_patch

**Environment Variables** (provided to hooks):

* `CODE_HOOK_EVENT`, `CODE_HOOK_NAME`, `CODE_HOOK_INDEX`
* `CODE_HOOK_CALL_ID`, `CODE_HOOK_PAYLOAD` (JSON)
* `CODE_SESSION_CWD`, `CODE_HOOK_SOURCE_CALL_ID`

### Project Commands

```toml
[[projects."/path/to/project".commands]]
name = "setup"
description = "Install dependencies"
run = ["pnpm", "install"]

[[projects."/path/to/project".commands]]
name = "unit"
run = "cargo test --lib"
```

Use with: `/cmd <name>` in TUI.

### Stage0 (NotebookLM) Config

```toml
[projects."/path/to/project".stage0]
notebook = "your-notebook-id-or-url"
notebooklm_base_url = "http://127.0.0.1:3456"  # Optional override
```

***

## 9. Config Key Reference

| Key                                  | Type          | Default          | Description                                           |
| ------------------------------------ | ------------- | ---------------- | ----------------------------------------------------- |
| `model`                              | string        | `gpt-5-codex`    | Model to use                                          |
| `model_provider`                     | string        | `openai`         | Provider from `model_providers`                       |
| `model_context_window`               | number        | (model-specific) | Context window tokens                                 |
| `model_max_output_tokens`            | number        | (model-specific) | Max output tokens                                     |
| `model_reasoning_effort`             | string        | `medium`         | `minimal`/`low`/`medium`/`high`                       |
| `model_reasoning_summary`            | string        | `auto`           | `auto`/`concise`/`detailed`/`none`                    |
| `model_verbosity`                    | string        | `medium`         | `low`/`medium`/`high` (GPT-5)                         |
| `model_supports_reasoning_summaries` | boolean       | false            | Force reasoning support                               |
| `approval_policy`                    | string        | `untrusted`      | `untrusted`/`on-failure`/`on-request`/`never`         |
| `sandbox_mode`                       | string        | `read-only`      | `read-only`/`workspace-write`/`danger-full-access`    |
| `disable_response_storage`           | boolean       | false            | Required for ZDR                                      |
| `profile`                            | string        | -                | Active profile name                                   |
| `notify`                             | array         | -                | External notification program                         |
| `file_opener`                        | string        | `vscode`         | `vscode`/`vscode-insiders`/`windsurf`/`cursor`/`none` |
| `hide_agent_reasoning`               | boolean       | false            | Hide reasoning events                                 |
| `show_raw_agent_reasoning`           | boolean       | false            | Show raw chain-of-thought                             |
| `project_doc_max_bytes`              | number        | 32768            | Max bytes from AGENTS.md                              |
| `history.persistence`                | string        | `save-all`       | `save-all`/`none`                                     |
| `tui.notifications`                  | boolean/array | false            | Desktop notifications                                 |
| `tools.web_search`                   | boolean       | false            | Enable web search tool                                |

### Nested Keys

| Key                          | Description                 |
| ---------------------------- | --------------------------- |
| `model_providers.<id>.*`     | Provider configuration      |
| `mcp_servers.<id>.*`         | MCP server configuration    |
| `profiles.<name>.*`          | Profile-scoped overrides    |
| `projects.<path>.*`          | Project-scoped settings     |
| `sandbox_workspace_write.*`  | Workspace-write options     |
| `shell_environment_policy.*` | Environment filtering       |
| `validation.groups.*`        | Validation group toggles    |
| `validation.tools.*`         | Per-tool validation toggles |

***

# Part III: Stage 0 Observability

## 10. Stage 0 Metrics

Stage 0 metrics monitor and tune the overlay engine's performance and quality.

### Core Metrics

**Stage 0 Runs**:

* `stage0_runs_total` - Counter with label `result` (success, degraded\_config, degraded\_db, etc.)
* `stage0_run_latency_ms` - Histogram with labels `tier2_used`, `cache_hit`

**Tier 2 Usage & Cache**:

* `stage0_tier2_calls_total` - Counter with label `outcome` (success, timeout, error)
* `stage0_tier2_cache_hits_total` - Counter
* `stage0_tier2_cache_misses_total` - Counter
* `stage0_tier2_cache_entries` - Gauge (optional)

**DCC Stats**:

* `stage0_dcc_candidate_count` - Histogram (candidates after pre-filter)
* `stage0_dcc_selected_count` - Histogram (memories in TASK\_BRIEF)
* `stage0_dcc_combined_score_selected` - Histogram (optional)

**Error Rates**:

* `stage0_errors_total` - Counter with label `category` (CONFIG\_ERROR, OVERLAY\_DB\_ERROR, etc.)

**Memory Scoring**:

* `stage0_memory_dynamic_score` - Histogram (distribution across overlay)
* `stage0_memory_usage_count` - Histogram (0 uses, 1–5, 6–20, 21+)

### Suggested Dashboards

1. **Stage 0 Health Dashboard**
   * `stage0_runs_total` by `result`
   * `stage0_run_latency_ms` (p50/p95)
   * `stage0_errors_total` by `category`

2. **Tier 2 Performance Dashboard**
   * `stage0_tier2_calls_total` by `outcome`
   * Cache hit/miss ratio
   * Tier 2 latency distribution

3. **DCC & Context Quality Dashboard**
   * `stage0_dcc_candidate_count` & `stage0_dcc_selected_count`
   * `stage0_memory_dynamic_score` distribution

***

## 11. Stage 0 Events

### Correlation IDs

For each `run_stage0` invocation, generate a `request_id` (UUID). Include it in all logs:

* DCC steps
* Cache hits/misses
* NotebookLM calls
* Causal link ingestion

### Stage 0 Run Event

```json
{
  "timestamp": "2025-11-30T15:42:01Z",
  "event_type": "stage0_run",
  "request_id": "3c1e31c4-9f13-4f2b-9e9a-0a034f3b9c5b",
  "spec_id": "SPEC-KIT-102",
  "tier2_used": true,
  "cache_hit": false,
  "tier2_latency_ms": 12450,
  "dcc": {
    "candidate_count": 87,
    "top_k": 15,
    "token_count": 3982
  },
  "result": {
    "status": "success",
    "error": null
  }
}
```

### Guardian Events

**Metadata Guardian Warning**:

```json
{
  "timestamp": "2025-11-30T15:39:01Z",
  "event_type": "metadata_guardian_warning",
  "memory_id": "mem-123",
  "reason": "auto-filled created_at",
  "fields": {
    "created_at_before": null,
    "created_at_after": "2025-11-30T15:39:01Z"
  }
}
```

**Template Guardian Error**:

```json
{
  "timestamp": "2025-11-30T15:40:55Z",
  "event_type": "template_guardian_error",
  "memory_id": "mem-123",
  "error": "LLM request timeout"
}
```

### Cache Invalidation Events

```json
{
  "timestamp": "2025-11-30T15:45:22Z",
  "event_type": "tier2_cache_invalidation",
  "memory_id": "mem-789",
  "cache_hash": "sha256:abc123...",
  "reason": "memory_update"
}
```

### Explainability Snapshots

When Stage 0 runs with explainability enabled:

```json
{
  "timestamp": "2025-11-30T15:43:10Z",
  "event_type": "dcc_explain_snapshot",
  "request_id": "3c1e31c4-9f13-4f2b-...",
  "spec_id": "SPEC-KIT-102",
  "top_memories": [
    {
      "id": "mem-001",
      "combined_score": 0.88,
      "components": {
        "similarity": 0.91,
        "dynamic_score": 0.82
      }
    }
  ]
}
```

### Storage Notes

V1 does not mandate a dedicated metrics backend. Options:

* Emit JSON logs and collect with existing stack
* Write logs to overlay DB for offline analysis

Consistency and structure matter more than the sink.

***

# Appendices

## A. Related Documentation

* **CLAUDE.md** - Quick reference for commands and project structure
* **POLICY.md** - Model policy, gate policy, evidence policy, testing policy
* **STAGE0-REFERENCE.md** - Stage 0 integration, types, DCC, configuration
* **AGENTS.md** - Agent orchestration and MCP server config
* **PLANNING.md** - High-level architecture, goals, constraints
* **memory/constitution.md** - Project charter and guardrails
* **docs/spec-kit/MULTI-AGENT-ARCHITECTURE.md** - Multi-agent system documentation

## B. Change History

| Version | Date       | Changes                                                        |
| ------- | ---------- | -------------------------------------------------------------- |
| 1.1.0   | 2026-01-22 | Added Part III: Stage 0 Observability (metrics, events)        |
| 1.0.0   | 2026-01-21 | Initial consolidation from OPERATIONAL-PLAYBOOK.md + config.md |

***

*Last Updated: 2026-01-22*
