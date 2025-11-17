# Environment Variables

Complete reference for all environment variables and override behavior.

---

## Overview

Environment variables provide **Tier 2 precedence** (higher than config.toml, lower than CLI flags).

**Use Cases**:
- API keys and secrets
- Environment-specific overrides (dev, staging, production)
- CI/CD configuration
- Temporary configuration changes

---

## Core Environment Variables

### CODEX_HOME / CODE_HOME

**Purpose**: Installation directory

**Default**: `~/.code`

**Legacy**: `~/.codex` (read-only, deprecated)

**Usage**:
```bash
export CODEX_HOME="/custom/path"
# or
export CODE_HOME="/custom/path"
```

**Precedence**: `CODE_HOME` > `CODEX_HOME` > `~/.code`

**Files Stored**:
```
$CODEX_HOME/
├── config.toml          # Configuration file
├── history.jsonl        # Session history
├── debug.log            # Debug logs
├── mcp-memory/          # MCP memory database
├── mcp-cache/           # MCP tool cache
└── ace/                 # ACE playbook database
    └── playbooks_normalized.sqlite3
```

---

## API Keys

### OPENAI_API_KEY

**Purpose**: OpenAI API authentication

**Required**: When using `model_provider = "openai"`

**Usage**:
```bash
export OPENAI_API_KEY="sk-proj-..."
```

**Security**: Never commit to git, never store in config.toml

---

### ANTHROPIC_API_KEY

**Purpose**: Anthropic API authentication

**Required**: When using `model_provider = "anthropic"`

**Usage**:
```bash
export ANTHROPIC_API_KEY="sk-ant-..."
```

---

### GOOGLE_API_KEY

**Purpose**: Google Gemini API authentication

**Required**: When using `model_provider = "google"`

**Usage**:
```bash
export GOOGLE_API_KEY="..."
```

---

### AZURE_OPENAI_API_KEY

**Purpose**: Azure OpenAI API authentication

**Required**: When using Azure model provider

**Usage**:
```bash
export AZURE_OPENAI_API_KEY="..."
```

**Alternative**: `OPENAI_API_KEY` also works for Azure

---

### Custom Provider API Keys

**Pattern**: `<PROVIDER_NAME>_API_KEY`

**Example**:
```toml
[model_providers.custom]
env_key = "CUSTOM_API_KEY"
```

```bash
export CUSTOM_API_KEY="..."
```

---

## Model Configuration Overrides

### CODEX_MODEL

**Purpose**: Override default model

**Precedence**: Env var > config.toml

**Usage**:
```bash
export CODEX_MODEL="o3"
code "task"
```

**Equivalent**:
```bash
code --model o3 "task"
```

---

### CODEX_PROVIDER

**Purpose**: Override model provider

**Usage**:
```bash
export CODEX_PROVIDER="anthropic"
code "task"
```

**Equivalent**:
```bash
code --config model_provider=anthropic "task"
```

---

### OPENAI_BASE_URL

**Purpose**: Override OpenAI base URL

**Use Case**: Custom proxy, Azure, local endpoint

**Usage**:
```bash
export OPENAI_BASE_URL="https://custom.openai.com/v1"
```

**Overrides**: `model_providers.openai.base_url`

---

### OPENAI_WIRE_API

**Purpose**: Force OpenAI wire protocol

**Options**: `"responses"` or `"chat"`

**Usage**:
```bash
export OPENAI_WIRE_API="chat"  # Force chat completions
```

**Overrides**: `model_providers.openai.wire_api`

---

## Spec-Kit Environment Variables

### SPEC_OPS_CARGO_MANIFEST

**Purpose**: Override cargo manifest path for workspace commands

**Default**: Auto-detected (`codex-rs/Cargo.toml`)

**Usage**:
```bash
export SPEC_OPS_CARGO_MANIFEST="/path/to/Cargo.toml"
```

---

### SPEC_OPS_ALLOW_DIRTY

**Purpose**: Allow guardrail commands with dirty git tree

**Default**: `0` (require clean tree)

**Usage**:
```bash
export SPEC_OPS_ALLOW_DIRTY=1
/guardrail.auto SPEC-KIT-065
```

**Use Case**: Testing, development iteration

---

### SPEC_OPS_TELEMETRY_HAL

**Purpose**: Enable HAL telemetry collection

**Default**: `0` (disabled)

**Usage**:
```bash
export SPEC_OPS_TELEMETRY_HAL=1
/guardrail.plan SPEC-KIT-065
```

**Output**: Captures `hal.summary.{status,failed_checks,artifacts}` in telemetry

---

### SPEC_OPS_HAL_SKIP

**Purpose**: Skip HAL validation (when secrets unavailable)

**Default**: `0` (run HAL validation)

**Usage**:
```bash
export SPEC_OPS_HAL_SKIP=1
/guardrail.audit SPEC-KIT-065
```

**Use Case**: Development without HAL secrets

---

### SPECKIT_QUALITY_GATES_*

**Purpose**: Override quality gate agent selection

**Pattern**: `SPECKIT_QUALITY_GATES_<STAGE>=agent1,agent2,agent3`

**Usage**:
```bash
export SPECKIT_QUALITY_GATES_PLAN="gemini,claude,code,gpt_pro"
export SPECKIT_QUALITY_GATES_TASKS="code"
export SPECKIT_QUALITY_GATES_VALIDATE="gemini,claude,code"
export SPECKIT_QUALITY_GATES_AUDIT="gemini,claude,gpt_codex,gpt_pro"
export SPECKIT_QUALITY_GATES_UNLOCK="gemini,claude,gpt_pro"
```

**Precedence**: Env var > config.toml

---

## Logging and Debugging

### RUST_LOG

**Purpose**: Rust logging level

**Options**: `error`, `warn`, `info`, `debug`, `trace`

**Usage**:
```bash
export RUST_LOG=debug
code
```

**Module-Specific**:
```bash
export RUST_LOG=codex_tui::chatwidget::spec_kit=debug
code
```

**Multiple Modules**:
```bash
export RUST_LOG=codex_mcp_client=debug,codex_spec_kit=trace
code
```

---

### RUST_BACKTRACE

**Purpose**: Enable backtraces on panic

**Usage**:
```bash
export RUST_BACKTRACE=1  # Short backtrace
export RUST_BACKTRACE=full  # Full backtrace
code
```

**Use Case**: Debugging crashes

---

## Sandbox and Security

### CODEX_SANDBOX_NETWORK_DISABLED

**Purpose**: Disable network access in sandbox

**Auto-Set**: When `sandbox_mode = "read-only"` or `sandbox_mode = "workspace-write"` with `network_access = false`

**Usage** (manual override):
```bash
export CODEX_SANDBOX_NETWORK_DISABLED=1
```

---

## CI/CD Environment Variables

### CI

**Purpose**: Detect CI environment

**Auto-Set**: By most CI systems (GitHub Actions, GitLab CI, CircleCI, etc.)

**Usage**:
```toml
[shell_environment_policy]
set = { CI = "1" }
```

**Effect**: Triggers CI-specific behavior (non-interactive mode, strict validation)

---

### GITHUB_ACTIONS

**Purpose**: Detect GitHub Actions environment

**Auto-Set**: By GitHub Actions

**Usage**:
```bash
if [ "$GITHUB_ACTIONS" = "true" ]; then
  export CODEX_MODEL="gpt-4o"  # Use cheaper model in CI
fi
```

---

### CODEX_AUTO_UPGRADE

**Purpose**: Enable/disable auto-upgrade

**Options**: `true`/`false`, `1`/`0`, `yes`/`no`, `on`/`off`

**Usage**:
```bash
export CODEX_AUTO_UPGRADE=false  # Disable auto-upgrade in CI
```

**Overrides**: `auto_upgrade_enabled` in config.toml

---

## Shell Environment Policy

### Shell Environment Inheritance

**Configuration**:
```toml
[shell_environment_policy]
inherit = "all"  # all, core, none
ignore_default_excludes = false
exclude = ["AWS_*", "AZURE_*"]
set = { CI = "1" }
include_only = []
```

**Default Excludes** (when `ignore_default_excludes = false`):
- `*KEY*` (case-insensitive)
- `*TOKEN*` (case-insensitive)
- `*SECRET*` (case-insensitive)

**Example**:
```bash
# These are excluded by default:
export AWS_ACCESS_KEY="..."       # Excluded (*KEY*)
export GITHUB_TOKEN="..."         # Excluded (*TOKEN*)
export DB_SECRET="..."            # Excluded (*SECRET*)

# These are included (no KEY/TOKEN/SECRET):
export PATH="/usr/bin"            # Included
export HOME="/home/user"          # Included
```

---

### Override Shell Environment Policy

**Usage**:
```bash
export SHELL_ENV_INHERIT="core"  # Override inherit mode
export SHELL_ENV_IGNORE_DEFAULT_EXCLUDES="1"  # Include KEY/TOKEN vars
```

---

## MCP Server Environment Variables

### MCP-Specific Variables

**Pattern**: Set in `env` field of `[mcp_servers.<name>]`

**Example**:
```toml
[mcp_servers.database]
command = "/path/to/db-server"
env = {
    DB_HOST = "localhost",
    DB_PORT = "5432",
    DB_NAME = "mydb"
}
```

**Scope**: Only available to that specific MCP server

---

### Global MCP Environment

**Pattern**: `MCP_*` prefix

**Usage**:
```bash
export MCP_LOG_LEVEL="debug"
export MCP_TIMEOUT="30000"
```

**Scope**: Available to all MCP servers

---

## HAL Secret Environment Variables

### HAL_SECRET_KAVEDARR_API_KEY

**Purpose**: Kavedarr API key for HAL validation

**Required**: When running HAL smoke tests or policy validation

**Usage**:
```bash
export HAL_SECRET_KAVEDARR_API_KEY="..."
```

**Security**: Never commit, never store in config

---

## Testing Environment Variables

### PRECOMMIT_FAST_TEST

**Purpose**: Skip test compilation in pre-commit hook

**Default**: `1` (skip test compilation)

**Usage**:
```bash
export PRECOMMIT_FAST_TEST=0  # Run test compilation
git commit
```

---

### PREPUSH_FAST

**Purpose**: Skip pre-push hooks

**Default**: `1` (run hooks)

**Usage**:
```bash
export PREPUSH_FAST=0  # Skip pre-push hooks
git push
```

**Warning**: Only use for emergencies

---

## Complete Environment Variable Reference

### Core Variables

| Variable | Purpose | Default | Example |
|----------|---------|---------|---------|
| `CODEX_HOME` | Installation directory | `~/.code` | `/custom/path` |
| `CODE_HOME` | Alt. installation directory | (uses CODEX_HOME) | `/custom/path` |
| `RUST_LOG` | Logging level | `info` | `debug` |
| `RUST_BACKTRACE` | Backtrace on panic | `0` | `1`, `full` |

---

### API Keys

| Variable | Purpose | Required For |
|----------|---------|--------------|
| `OPENAI_API_KEY` | OpenAI authentication | `model_provider = "openai"` |
| `ANTHROPIC_API_KEY` | Anthropic authentication | `model_provider = "anthropic"` |
| `GOOGLE_API_KEY` | Google Gemini authentication | `model_provider = "google"` |
| `AZURE_OPENAI_API_KEY` | Azure OpenAI authentication | Azure model provider |
| `<PROVIDER>_API_KEY` | Custom provider authentication | Custom providers |

---

### Model Overrides

| Variable | Overrides | Example |
|----------|-----------|---------|
| `CODEX_MODEL` | `model` | `o3` |
| `CODEX_PROVIDER` | `model_provider` | `anthropic` |
| `OPENAI_BASE_URL` | `model_providers.openai.base_url` | `https://custom.api.com` |
| `OPENAI_WIRE_API` | `model_providers.openai.wire_api` | `chat`, `responses` |
| `CODEX_AUTO_UPGRADE` | `auto_upgrade_enabled` | `true`, `false` |

---

### Spec-Kit Variables

| Variable | Purpose | Default | Example |
|----------|---------|---------|---------|
| `SPEC_OPS_CARGO_MANIFEST` | Cargo manifest path | Auto-detected | `/path/to/Cargo.toml` |
| `SPEC_OPS_ALLOW_DIRTY` | Allow dirty git tree | `0` | `1` |
| `SPEC_OPS_TELEMETRY_HAL` | Enable HAL telemetry | `0` | `1` |
| `SPEC_OPS_HAL_SKIP` | Skip HAL validation | `0` | `1` |
| `SPECKIT_QUALITY_GATES_*` | Override quality gate agents | (from config) | `gemini,claude,code` |

---

### Sandbox and Security

| Variable | Purpose | Auto-Set | Manual Override |
|----------|---------|----------|-----------------|
| `CODEX_SANDBOX_NETWORK_DISABLED` | Disable network in sandbox | Yes (when `network_access = false`) | `1` |

---

### CI/CD Variables

| Variable | Purpose | Auto-Set By | Example |
|----------|---------|-------------|---------|
| `CI` | CI environment detection | Most CI systems | `1`, `true` |
| `GITHUB_ACTIONS` | GitHub Actions detection | GitHub Actions | `true` |

---

### Testing Variables

| Variable | Purpose | Default | Example |
|----------|---------|---------|---------|
| `PRECOMMIT_FAST_TEST` | Skip test compilation in pre-commit | `1` | `0` |
| `PREPUSH_FAST` | Skip pre-push hooks | `1` | `0` |

---

## Best Practices

### 1. Store Secrets in Environment Variables

**Good**:
```bash
export OPENAI_API_KEY="sk-proj-..."
export ANTHROPIC_API_KEY="sk-ant-..."
```

**Bad**:
```toml
# DON'T: Never store secrets in config.toml
[model_providers.openai]
api_key = "sk-proj-..."  # ❌ Security risk!
```

---

### 2. Use .env Files (Local Development)

**`.env` file** (git-ignored):
```bash
# .env
OPENAI_API_KEY=sk-proj-...
ANTHROPIC_API_KEY=sk-ant-...
GOOGLE_API_KEY=...
```

**Load with direnv**:
```bash
# Install direnv
brew install direnv  # macOS
apt install direnv   # Linux

# Enable for shell
echo 'eval "$(direnv hook bash)"' >> ~/.bashrc

# Allow .envrc
echo 'dotenv' > .envrc
direnv allow
```

---

### 3. Use Profiles for Environment-Specific Config

**config.toml**:
```toml
[profiles.dev]
model = "gpt-4o-mini"
approval_policy = "never"

[profiles.staging]
model = "gpt-5"
approval_policy = "on-request"

[profiles.production]
model = "o3"
approval_policy = "on-failure"
model_reasoning_effort = "high"
```

**Usage**:
```bash
# Development
code --profile dev "task"

# Staging
code --profile staging "task"

# Production
code --profile production "task"
```

---

### 4. Document Required Environment Variables

**README.md**:
```markdown
## Required Environment Variables

- `OPENAI_API_KEY` - OpenAI API key
- `ANTHROPIC_API_KEY` - Anthropic API key (optional)
- `CODEX_HOME` - Installation directory (optional, default: ~/.code)
```

---

## Debugging Environment Variables

### List Active Environment Variables

```bash
# All CODEX_* and *_API_KEY variables
env | grep -E 'CODEX|API_KEY'
```

**Output**:
```
CODEX_HOME=/home/user/.code
CODEX_MODEL=gpt-5
OPENAI_API_KEY=sk-proj-***
ANTHROPIC_API_KEY=sk-ant-***
```

---

### Check Effective Configuration

```bash
code --config-dump | grep -A 5 "# Source:"
```

**Output**:
```toml
model = "o3"  # Source: Environment variable (CODEX_MODEL)
model_provider = "openai"  # Source: config.toml
approval_policy = "never"  # Source: Profile 'premium'
```

---

## Summary

**Environment Variables** provide:
- Tier 2 precedence (env var > config.toml)
- API key storage (secure, never in config)
- Environment-specific overrides (dev, staging, production)
- CI/CD configuration
- Temporary configuration changes

**Categories**:
- Core (CODEX_HOME, RUST_LOG)
- API Keys (OPENAI_API_KEY, ANTHROPIC_API_KEY, GOOGLE_API_KEY)
- Model Overrides (CODEX_MODEL, CODEX_PROVIDER)
- Spec-Kit (SPEC_OPS_*, SPECKIT_*)
- Sandbox (CODEX_SANDBOX_NETWORK_DISABLED)
- CI/CD (CI, GITHUB_ACTIONS)
- Testing (PRECOMMIT_FAST_TEST, PREPUSH_FAST)

**Best Practices**:
- Store secrets in environment variables
- Use .env files (git-ignored) for local development
- Use profiles for environment-specific config
- Document required variables in README

**Next**: [Template Customization](template-customization.md)
