# Precedence System

5-tier configuration precedence with examples.

---

## Overview

The configuration system implements **5-tier precedence** (highest to lowest):

1. **CLI Flags** (highest priority) - Command-line arguments
2. **Shell Environment** - Environment variables
3. **Profile** - Named configuration sets
4. **Config File** - `~/.code/config.toml`
5. **Defaults** (lowest priority) - Built-in fallback values

**Rule**: Higher tiers override lower tiers

---

## Precedence Order

### Tier 1: CLI Flags (Highest)

**Priority**: Highest

**Usage**:
```bash
# Specific model flags
code --model o3 "task description"
code --profile premium "task"

# Generic config flag
code --config model="gpt-5"
code --config approval_policy=never
code -c model_reasoning_effort=high

# Deep config paths (dot notation)
code --config model_providers.openai.wire_api="chat"
code --config shell_environment_policy.include_only='["PATH", "HOME"]'
```

**Characteristics**:
- Overrides all other tiers
- Session-specific (not persisted)
- Supports dot notation for nested values
- Values in TOML format (not JSON)

**Examples**:
```bash
# Override model
code --model o3

# Override approval policy
code --config approval_policy=never

# Override provider config
code --config model_providers.openai.base_url="https://custom.api.com"
```

---

### Tier 2: Shell Environment

**Priority**: 2nd highest

**Patterns**:
- `CODEX_HOME`, `CODE_HOME` - Installation directory
- `<PROVIDER>_API_KEY` - API keys (e.g., `OPENAI_API_KEY`)
- `OPENAI_BASE_URL` - Provider base URL override
- `OPENAI_WIRE_API` - Wire protocol override (`"responses"` or `"chat"`)
- `CODEX_MODEL`, `CODEX_PROVIDER` - Model/provider overrides

**Usage**:
```bash
# API keys (most common)
export OPENAI_API_KEY="sk-proj-..."
export ANTHROPIC_API_KEY="sk-ant-..."
export GOOGLE_API_KEY="..."

# Home directory
export CODEX_HOME="/custom/path"

# Provider overrides
export OPENAI_BASE_URL="https://custom.openai.com/v1"
export OPENAI_WIRE_API="responses"

# Model overrides
export CODEX_MODEL="gpt-5"
export CODEX_PROVIDER="anthropic"
```

**Characteristics**:
- Persistent for session duration
- Useful for secrets (API keys)
- Environment-specific overrides
- Case-insensitive for most values

---

### Tier 3: Profile

**Priority**: 3rd highest

**Activation**:
```bash
# Via CLI
code --profile premium "task"

# Via config.toml
profile = "premium"
```

**Definition**:
```toml
# ~/.code/config.toml

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
```

**Characteristics**:
- Named configuration sets
- Overrides config.toml base values
- Can be selected per-session via CLI
- Useful for different workflows

---

### Tier 4: Config File

**Priority**: 4th highest

**Location**: `~/.code/config.toml`

**Example**:
```toml
model = "gpt-5"
model_provider = "openai"
approval_policy = "on-request"
sandbox_mode = "workspace-write"

[quality_gates]
plan = ["gemini", "claude", "code"]
tasks = ["gemini"]
```

**Characteristics**:
- Persistent across sessions
- User-specific configuration
- Hot-reloadable (changes apply without restart)
- TOML format (human-readable)

---

### Tier 5: Defaults (Lowest)

**Priority**: Lowest

**Source**: Built-in code defaults

**Example**:
```rust
impl Default for AppConfig {
    fn default() -> Self {
        Self {
            model: "gpt-5-codex".to_string(),
            model_provider: "openai".to_string(),
            approval_policy: ApprovalPolicy::OnRequest,
            sandbox_mode: SandboxMode::ReadOnly,
            // ... 30+ more fields
        }
    }
}
```

**Characteristics**:
- Fallback values when no other tier specifies
- Hardcoded in Rust source
- Guaranteed sensible defaults
- Work out-of-the-box without configuration

---

## Precedence Examples

### Example 1: Model Selection

**Setup**:
```toml
# ~/.code/config.toml
model = "gpt-5"

[profiles.premium]
model = "o3"
```

```bash
export CODEX_MODEL="gpt-4o"
```

**Scenarios**:

| Command | Effective Model | Why |
|---------|-----------------|-----|
| `code "task"` | `gpt-4o` | Env var (Tier 2) > config.toml (Tier 4) |
| `code --profile premium "task"` | `o3` | Profile (Tier 3) > env var (Tier 2) |
| `code --model o1 "task"` | `o1` | CLI flag (Tier 1) > all others |
| `code --profile premium --model o1 "task"` | `o1` | CLI flag (Tier 1) wins |

---

### Example 2: API Key

**Setup**:
```toml
# ~/.code/config.toml
# (no API key specified)
```

```bash
export OPENAI_API_KEY="sk-proj-env-key"
```

**Scenarios**:

| Command | Effective Key | Why |
|---------|---------------|-----|
| `code "task"` | `sk-proj-env-key` | Env var (Tier 2) > defaults (Tier 5) |
| `code --config model_providers.openai.env_key="sk-proj-cli-key" "task"` | `sk-proj-cli-key` | CLI flag (Tier 1) > env var (Tier 2) |

**Note**: API keys should **always** be stored in environment variables, never in `config.toml`.

---

### Example 3: Approval Policy

**Setup**:
```toml
# ~/.code/config.toml
approval_policy = "on-request"

[profiles.ci]
approval_policy = "never"
```

```bash
# No environment overrides
```

**Scenarios**:

| Command | Effective Policy | Why |
|---------|------------------|-----|
| `code "task"` | `on-request` | config.toml (Tier 4) > defaults (Tier 5) |
| `code --profile ci "task"` | `never` | Profile (Tier 3) > config.toml (Tier 4) |
| `code --profile ci --config approval_policy=untrusted "task"` | `untrusted` | CLI flag (Tier 1) > profile (Tier 3) |

---

### Example 4: Complex Nested Config

**Setup**:
```toml
# ~/.code/config.toml
[model_providers.openai]
base_url = "https://api.openai.com/v1"
wire_api = "responses"
```

```bash
export OPENAI_BASE_URL="https://custom.openai.com/v1"
```

**Scenarios**:

| Command | Effective URL | Wire API | Why |
|---------|---------------|----------|-----|
| `code "task"` | `https://custom.openai.com/v1` | `responses` | Env var (Tier 2) for URL, config.toml (Tier 4) for wire_api |
| `code --config model_providers.openai.wire_api="chat" "task"` | `https://custom.openai.com/v1` | `chat` | CLI flag (Tier 1) for wire_api, env var (Tier 2) for URL |

---

## Special Cases

### Shell Environment Policy Override

**Warning**: `shell_environment_policy.set` can override config values at runtime.

**Example**:
```toml
# ~/.code/config.toml
approval_policy = "always"

[shell_environment_policy]
set = { APPROVAL_POLICY = "never" }  # ⚠️ OVERRIDES top-level setting!
```

**Behavior**: `APPROVAL_POLICY=never` wins at runtime (subprocess environment)

**Best Practice**: Avoid using `shell_environment_policy.set` for keys that exist as top-level config options.

---

### Profile Selection Precedence

**Priority**: CLI `--profile` > `config.toml` `profile` field > no profile

**Example**:
```toml
# ~/.code/config.toml
profile = "fast"  # Default profile

[profiles.fast]
model = "gpt-4o-mini"

[profiles.premium]
model = "o3"
```

| Command | Effective Profile | Model | Why |
|---------|-------------------|-------|-----|
| `code "task"` | `fast` | `gpt-4o-mini` | config.toml `profile` field |
| `code --profile premium "task"` | `premium` | `o3` | CLI `--profile` overrides config.toml |

---

## Precedence Table

**Summary**:

| Tier | Source | Example | Persistence | Override Method |
|------|--------|---------|-------------|-----------------|
| 1 | CLI Flags | `--model o3` | Session-only | Command-line args |
| 2 | Environment | `OPENAI_API_KEY=...` | Session/shell | `export VAR=value` |
| 3 | Profile | `[profiles.premium]` | Persistent (in config.toml) | `--profile name` or `profile = "name"` |
| 4 | Config File | `model = "gpt-5"` | Persistent | Edit `~/.code/config.toml` |
| 5 | Defaults | `"gpt-5-codex"` | Built-in | (Cannot override) |

---

## Debugging Precedence

### Check Effective Configuration

**Command**:
```bash
code --config-dump
```

**Output**:
```toml
# Effective configuration (after precedence resolution)
model = "o3"  # From: CLI flag (--model o3)
model_provider = "openai"  # From: config.toml
approval_policy = "never"  # From: profile 'premium'
# ... full effective config
```

---

### Trace Configuration Source

**Example**:
```bash
# With verbose logging
export RUST_LOG=debug
code --model o3 "task"
```

**Log Output**:
```
[DEBUG] Config layer 5 (defaults): model=gpt-5-codex
[DEBUG] Config layer 4 (config.toml): model=gpt-5
[DEBUG] Config layer 3 (profile 'premium'): model=o3
[DEBUG] Config layer 1 (CLI flag): model=o3
[INFO] Effective model: o3 (source: CLI flag)
```

---

## Best Practices

### 1. Use Environment Variables for Secrets

**Good**:
```bash
export OPENAI_API_KEY="sk-proj-..."
```

**Bad**:
```toml
# DON'T: API keys should NOT be in config.toml
[model_providers.openai]
api_key = "sk-proj-..."  # ❌ Security risk!
```

---

### 2. Use Profiles for Workflows

**Example**:
```toml
# Fast iteration
[profiles.fast]
model = "gpt-4o-mini"
approval_policy = "never"

# Premium quality
[profiles.premium]
model = "o3"
model_reasoning_effort = "high"

# CI/automation
[profiles.ci]
model = "gpt-4o"
approval_policy = "never"
sandbox_mode = "read-only"
```

**Usage**:
```bash
code --profile fast "quick formatting"
code --profile premium "complex refactor"
code --profile ci "generate report"
```

---

### 3. Use CLI Flags for One-Off Overrides

**Example**:
```bash
# One-time model override
code --model o3 "complex task"

# One-time approval policy override
code --config approval_policy=never "trusted script"
```

---

### 4. Keep config.toml for Persistent Preferences

**Example**:
```toml
# ~/.code/config.toml

# Personal preferences (persistent)
model = "gpt-5"
approval_policy = "on-request"
sandbox_mode = "workspace-write"
file_opener = "vscode"

[tui.theme]
name = "dark-carbon-night"
```

---

## Summary

**5-Tier Precedence** (highest to lowest):
1. CLI Flags - Session-specific overrides
2. Environment Variables - Secrets and env-specific config
3. Profiles - Named configuration sets
4. Config File - Persistent user preferences
5. Defaults - Built-in fallback values

**Rule**: Higher tiers override lower tiers

**Best Practices**:
- Secrets → Environment variables
- Workflows → Profiles
- One-off overrides → CLI flags
- Persistent preferences → config.toml

**Next**: [Model Configuration](model-configuration.md)
