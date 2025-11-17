# Configuration Reference

Complete `config.toml` schema reference.

---

## Overview

**Location**: `~/.code/config.toml`

**Alternative**: `~/.codex/config.toml` (legacy, read-only)

**Format**: TOML (Tom's Obvious, Minimal Language)

**Validation**: Schema validation on load, old config preserved on error

---

## File Structure

### Minimal Example

```toml
# ~/.code/config.toml (minimal)

model = "gpt-5"
model_provider = "openai"
approval_policy = "on-request"
```

---

### Complete Example

```toml
# ~/.code/config.toml (comprehensive)

# ============================================================================
# Model Configuration
# ============================================================================

model = "gpt-5"
model_provider = "openai"
model_reasoning_effort = "medium"  # minimal, low, medium, high
model_reasoning_summary = "auto"   # auto, concise, detailed, none
model_verbosity = "medium"         # low, medium, high (GPT-5 only)
model_context_window = 128000      # Override context window size
model_max_output_tokens = 16384    # Override max output tokens
model_supports_reasoning_summaries = false

# ============================================================================
# Model Providers
# ============================================================================

[model_providers.openai]
name = "OpenAI"
base_url = "https://api.openai.com/v1"
env_key = "OPENAI_API_KEY"
wire_api = "responses"  # or "chat"
request_max_retries = 4
stream_max_retries = 10
stream_idle_timeout_ms = 300000  # 5 minutes

[model_providers.anthropic]
name = "Anthropic"
base_url = "https://api.anthropic.com"
env_key = "ANTHROPIC_API_KEY"
wire_api = "chat"

[model_providers.google]
name = "Google"
base_url = "https://generativelanguage.googleapis.com/v1beta"
env_key = "GOOGLE_API_KEY"
wire_api = "chat"

[model_providers.ollama]
name = "Ollama"
base_url = "http://localhost:11434/v1"
# No env_key needed for local Ollama

# ============================================================================
# Agents (Multi-Agent Configuration)
# ============================================================================

[[agents]]
name = "gemini"
canonical_name = "gemini"
command = "gemini"
args = []
read_only = false
enabled = true
description = "Google Gemini Flash (fast, cheap consensus)"

[[agents]]
name = "claude"
canonical_name = "claude"
command = "claude"
args = []
read_only = false
enabled = true
description = "Anthropic Claude Haiku (balanced reasoning)"

[[agents]]
name = "code"
canonical_name = "gpt_pro"
command = "code"
args = ["--model", "gpt-5"]
read_only = false
enabled = true
description = "OpenAI GPT-5 (strategic planning)"

[[agents]]
name = "gpt_codex"
canonical_name = "gpt_codex"
command = "code"
args = ["--model", "gpt-5-codex"]
read_only = false
enabled = true
description = "OpenAI GPT-5-Codex (code generation)"

# ============================================================================
# Quality Gates (Spec-Kit Framework)
# ============================================================================

[quality_gates]
plan = ["gemini", "claude", "code"]        # Multi-agent planning
tasks = ["gemini"]                          # Single-agent task breakdown
validate = ["gemini", "claude", "code"]    # Multi-agent test validation
audit = ["gemini", "claude", "gpt_codex"]  # Security/compliance review
unlock = ["gemini", "claude", "gpt_codex"] # Ship decision

# ============================================================================
# Hot-Reload Configuration
# ============================================================================

[hot_reload]
enabled = true
debounce_ms = 2000  # Wait 2s after last change before reloading
watch_paths = ["config.toml"]  # Additional paths to watch

# ============================================================================
# Validation Configuration
# ============================================================================

[validation]
check_api_keys = true      # Validate API keys on startup
check_commands = true      # Validate agent commands exist
strict_schema = true       # Enforce strict TOML schema
patch_harness = false      # Run patch validation harness

[validation.groups]
functional = true   # Functional checks (cargo, tsc, etc.)
stylistic = false   # Stylistic checks (prettier, shfmt)

[validation.tools]
shellcheck = true
cargo-check = true
# ... other tools (see Validation section below)

# ============================================================================
# Approval Policy
# ============================================================================

approval_policy = "on-request"  # untrusted, on-failure, on-request, never

# ============================================================================
# Confirm Guard (Destructive Commands)
# ============================================================================

[[confirm_guard.patterns]]
regex = "(?i)^\\s*git\\s+reset\\b"
message = "Blocked git reset. Reset rewrites the working tree/index."

[[confirm_guard.patterns]]
regex = "(?i)^\\s*(?:sudo\\s+)?rm\\s+-[a-z-]*rf[a-z-]*\\s+"
message = "Blocked rm -rf. Force-recursive delete requires confirmation."

# ============================================================================
# Sandbox Configuration
# ============================================================================

sandbox_mode = "workspace-write"  # read-only, workspace-write, danger-full-access

[sandbox_workspace_write]
exclude_tmpdir_env_var = false
exclude_slash_tmp = false
writable_roots = []  # Additional writable paths
network_access = false
allow_git_writes = true  # Allow .git/ folder writes

# ============================================================================
# Shell Environment Policy
# ============================================================================

[shell_environment_policy]
inherit = "all"  # all, core, none
ignore_default_excludes = false  # If true, include *KEY*, *TOKEN* vars
exclude = ["AWS_*", "AZURE_*"]   # Additional exclusion patterns
set = { CI = "1" }                # Force-set environment variables
include_only = []                 # If non-empty, only these patterns survive

# ============================================================================
# MCP Servers
# ============================================================================

[mcp_servers.local-memory]
command = "npx"
args = ["-y", "@modelcontextprotocol/server-memory"]
startup_timeout_ms = 10000  # 10 seconds

[mcp_servers.git-status]
command = "npx"
args = ["-y", "@just-every/mcp-server-git"]
env = { LOG_LEVEL = "info" }

# ============================================================================
# ACE (Agentic Context Engine)
# ============================================================================

[ace]
enabled = true
mode = "auto"  # auto, always, never
slice_size = 8  # Max 8 playbook bullets
db_path = "~/.code/ace/playbooks_normalized.sqlite3"
use_for = ["speckit.constitution", "speckit.specify", "speckit.tasks"]
complex_task_files_threshold = 4
rerun_window_minutes = 30

# ============================================================================
# TUI Configuration
# ============================================================================

[tui]
alternate_screen = true        # Use alternate screen mode
show_reasoning = false         # Show reasoning content by default

[tui.theme]
name = "dark-carbon-night"     # See Theme section for all themes
# Optional custom color overrides
colors = {}

[tui.highlight]
theme = "auto"  # auto, or specific syntect theme

[tui.stream]
answer_header_immediate = false
show_answer_ellipsis = true
commit_tick_ms = 50
soft_commit_timeout_ms = 400
soft_commit_chars = 160
relax_list_holdback = false
relax_code_holdback = false
responsive = false  # Enable snappier preset

[tui.spinner]
name = "diamond"  # Spinner style from cli-spinners

[tui.notifications]
# false (disabled), true (all), or array of specific notifications
notifications = false

# ============================================================================
# History Configuration
# ============================================================================

[history]
persistence = "save-all"  # save-all, none
max_bytes = 10485760      # 10 MB (not currently enforced)

# ============================================================================
# Browser Configuration (Screenshot Tool)
# ============================================================================

[browser]
enabled = false
fullpage = true
segments_max = 10
idle_timeout_ms = 30000
format = "png"  # png, webp

[browser.viewport]
width = 1280
height = 720
device_scale_factor = 2.0
mobile = false

[browser.wait]
delay_ms = 1000  # Wait 1s before screenshot

# ============================================================================
# GitHub Integration
# ============================================================================

[github]
check_workflows_on_push = true
actionlint_on_patch = false
actionlint_strict = false

# ============================================================================
# Project Hooks
# ============================================================================

[[project_hooks]]
event = "session.start"
name = "install-deps"
command = ["npm", "install"]
timeout_ms = 60000

[[project_hooks]]
event = "file.after_write"
command = ["cargo", "fmt", "--all"]

# ============================================================================
# Profiles
# ============================================================================

profile = "default"  # Active profile

[profiles.premium]
model = "o3"
model_reasoning_effort = "high"
model_reasoning_summary = "detailed"
approval_policy = "never"

[profiles.fast]
model = "gpt-4o-mini"
model_reasoning_effort = "low"
approval_policy = "never"

[profiles.ci]
model = "gpt-4o"
approval_policy = "never"
sandbox_mode = "read-only"
disable_response_storage = false

# ============================================================================
# Miscellaneous
# ============================================================================

disable_response_storage = false  # Required for ZDR accounts
file_opener = "vscode"  # vscode, vscode-insiders, cursor, windsurf, none
hide_agent_reasoning = false
show_raw_agent_reasoning = false
project_doc_max_bytes = 32768  # 32 KiB
notify = []  # Command to execute for notifications
```

---

## Configuration Sections

### Model Configuration

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `model` | string | `"gpt-5"` | Model name to use |
| `model_provider` | string | `"openai"` | Provider ID from `model_providers` |
| `model_reasoning_effort` | string | `"medium"` | Reasoning effort: minimal, low, medium, high |
| `model_reasoning_summary` | string | `"auto"` | Summary mode: auto, concise, detailed, none |
| `model_verbosity` | string | `"medium"` | Verbosity level: low, medium, high (GPT-5 only) |
| `model_context_window` | integer | `128000` | Context window size in tokens |
| `model_max_output_tokens` | integer | `16384` | Max output tokens |
| `model_supports_reasoning_summaries` | boolean | `false` | Force reasoning support |

---

### Model Providers

**Table Format**: `[model_providers.<id>]`

**Required Fields**:
- `name` (string): Display name
- `base_url` (string): API base URL
- `env_key` (string, optional): Environment variable for API key

**Optional Fields**:
- `wire_api` (string): `"chat"` or `"responses"` (default: `"chat"`)
- `query_params` (table): Additional query parameters (e.g., Azure `api-version`)
- `http_headers` (table): Static HTTP headers
- `env_http_headers` (table): HTTP headers from environment variables
- `request_max_retries` (integer): HTTP request retries (default: 4)
- `stream_max_retries` (integer): SSE stream retries (default: 10)
- `stream_idle_timeout_ms` (integer): Idle timeout in ms (default: 300000)

**Example**:
```toml
[model_providers.azure]
name = "Azure OpenAI"
base_url = "https://YOUR_PROJECT.openai.azure.com/openai"
env_key = "AZURE_OPENAI_API_KEY"
query_params = { api-version = "2025-04-01-preview" }
```

---

### Agents

**Array Format**: `[[agents]]`

| Field | Type | Required | Description |
|-------|------|----------|-------------|
| `name` | string | Yes | Agent name (display) |
| `canonical_name` | string | No | Canonical identifier (default: same as `name`) |
| `command` | string | Yes | Command to execute |
| `args` | array | No | Command arguments |
| `read_only` | boolean | No | Force read-only mode (default: false) |
| `enabled` | boolean | No | Enable agent (default: true) |
| `description` | string | No | Agent description |
| `env` | table | No | Environment variables |
| `args_read_only` | array | No | Args for read-only mode |
| `args_write` | array | No | Args for write mode |
| `instructions` | string | No | Per-agent instructions |

---

### Quality Gates

**Table Format**: `[quality_gates]`

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `plan` | array | `[]` | Agent names for plan stage |
| `tasks` | array | `[]` | Agent names for tasks stage |
| `validate` | array | `[]` | Agent names for validate stage |
| `audit` | array | `[]` | Agent names for audit stage |
| `unlock` | array | `[]` | Agent names for unlock stage |

**Agent names** must match `canonical_name` from `[[agents]]`.

---

### Hot-Reload

**Table Format**: `[hot_reload]`

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `enabled` | boolean | `true` | Enable hot-reload |
| `debounce_ms` | integer | `2000` | Debounce window in ms (default: 2s) |
| `watch_paths` | array | `[]` | Additional paths to watch |

---

### Validation

**Table Format**: `[validation]`

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `check_api_keys` | boolean | `true` | Validate API keys on startup |
| `check_commands` | boolean | `true` | Validate agent commands exist |
| `strict_schema` | boolean | `true` | Enforce strict TOML schema |
| `patch_harness` | boolean | `false` | Run patch validation harness |
| `tools_allowlist` | array | `null` | Restrict allowed tools |
| `timeout_seconds` | integer | `null` | Tool execution timeout |

**Groups** (`[validation.groups]`):
- `functional` (boolean): Functional checks (cargo, tsc, eslint)
- `stylistic` (boolean): Stylistic checks (prettier, shfmt)

**Tools** (`[validation.tools]`):
- `shellcheck`, `markdownlint`, `hadolint`, `yamllint` (stylistic)
- `cargo-check`, `tsc`, `eslint`, `mypy`, `pyright`, `golangci-lint` (functional)
- `shfmt`, `prettier` (stylistic)

---

### Sandbox Configuration

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `sandbox_mode` | string | `"read-only"` | Sandbox mode: read-only, workspace-write, danger-full-access |

**`[sandbox_workspace_write]`** (only applies when `sandbox_mode = "workspace-write"`):

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `exclude_tmpdir_env_var` | boolean | `false` | Exclude `$TMPDIR` from writable roots |
| `exclude_slash_tmp` | boolean | `false` | Exclude `/tmp` from writable roots |
| `writable_roots` | array | `[]` | Additional writable paths |
| `network_access` | boolean | `false` | Allow network access |
| `allow_git_writes` | boolean | `true` | Allow `.git/` folder writes |

---

### MCP Servers

**Table Format**: `[mcp_servers.<name>]`

| Field | Type | Required | Description |
|-------|------|----------|-------------|
| `command` | string | Yes | Command to execute |
| `args` | array | No | Command arguments |
| `env` | table | No | Environment variables |
| `startup_timeout_ms` | integer | No | Startup timeout in ms (default: 10000) |

**Example**:
```toml
[mcp_servers.custom-tool]
command = "/path/to/mcp-server"
args = ["--port", "8080"]
env = { API_KEY = "secret" }
startup_timeout_ms = 15000
```

---

### TUI Configuration

**Theme** (`[tui.theme]`):
- `name` (string): Theme name (see Theme System guide)
- `colors` (table): Custom color overrides
- `label` (string, optional): Custom theme label
- `is_dark` (boolean, optional): Dark theme hint

**Highlight** (`[tui.highlight]`):
- `theme` (string): Syntax highlighting theme (default: "auto")

**Stream** (`[tui.stream]`):
- `answer_header_immediate` (boolean): Show header immediately
- `show_answer_ellipsis` (boolean): Show ellipsis while waiting
- `commit_tick_ms` (integer): Animation commit rate (default: 50ms)
- `soft_commit_timeout_ms` (integer): Soft-commit timeout
- `soft_commit_chars` (integer): Soft-commit character threshold
- `relax_list_holdback` (boolean): Relax list marker hold-back
- `relax_code_holdback` (boolean): Relax code block hold-back
- `responsive` (boolean): Enable snappier preset

---

### Profiles

**Table Format**: `[profiles.<name>]`

Profiles can override any top-level config field. See [Precedence System](precedence-system.md) for details.

**Example**:
```toml
[profiles.premium]
model = "o3"
model_reasoning_effort = "high"
approval_policy = "never"
```

**Activation**: Set `profile = "premium"` or use `--profile premium` flag.

---

## Validation Rules

### Required Fields

**None** - All fields have defaults

### Type Validation

- Strings: Non-empty (whitespace trimmed)
- Integers: Must be positive (where applicable)
- Booleans: `true` or `false`
- Arrays: Can be empty unless semantically invalid

### Semantic Validation

1. **Model provider must exist**: `model_provider` must be a key in `model_providers`
2. **Quality gate agents must exist**: Agent names in `quality_gates.*` must match `canonical_name` in `[[agents]]`
3. **Evidence size must be reasonable**: `evidence.max_size_mb` ≤ 1000
4. **Debounce must be reasonable**: `hot_reload.debounce_ms` ≥ 100

---

## Error Handling

**On validation failure**:
1. Old config is **preserved** (no reload)
2. `ReloadFailed` event emitted with error message
3. TUI shows notification with error details

**Example error**:
```
Config validation failed: Agent 'unknown-agent' not found in quality_gates.plan
Old config preserved.
```

---

## Summary

**Config File**: `~/.code/config.toml`

**Sections**: 20+ configuration sections covering:
- Model/provider configuration
- Multi-agent setup
- Quality gates
- Hot-reload settings
- Validation rules
- Sandbox policy
- MCP servers
- TUI customization
- Profiles

**Validation**: Schema validation with old config preservation on error

**Next**: [Precedence System](precedence-system.md)
