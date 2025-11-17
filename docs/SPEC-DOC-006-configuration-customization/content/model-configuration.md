# Model Configuration

Provider setup, reasoning effort, and model tuning.

---

## Overview

Model configuration controls:
1. **Provider Selection** - Which AI service to use (OpenAI, Anthropic, Google, Ollama)
2. **Model Selection** - Which specific model (GPT-5, o3, Claude, Gemini)
3. **Reasoning Configuration** - Effort level, summaries, verbosity
4. **Network Tuning** - Retries, timeouts, streaming

---

## Basic Model Configuration

### Minimal Setup

```toml
# ~/.code/config.toml

model = "gpt-5"
model_provider = "openai"
```

**Environment**:
```bash
export OPENAI_API_KEY="sk-proj-..."
```

---

### Model Selection

**Available Models** (OpenAI):
- `gpt-5` - Default, balanced reasoning and cost
- `gpt-5-codex` - Optimized for code generation
- `o3` - Maximum reasoning capability (premium)
- `o4-mini` - Fast reasoning model
- `gpt-4o` - Legacy model
- `gpt-4o-mini` - Fast, cheap legacy model

**Configuration**:
```toml
model = "o3"  # Use premium reasoning model
```

**CLI Override**:
```bash
code --model o3 "complex task"
```

---

## Provider Configuration

### OpenAI (Default)

```toml
model_provider = "openai"

[model_providers.openai]
name = "OpenAI"
base_url = "https://api.openai.com/v1"
env_key = "OPENAI_API_KEY"
wire_api = "responses"  # or "chat"
request_max_retries = 4
stream_max_retries = 10
stream_idle_timeout_ms = 300000  # 5 minutes
```

**Environment Variables**:
```bash
export OPENAI_API_KEY="sk-proj-..."

# Optional overrides
export OPENAI_BASE_URL="https://custom.openai.com/v1"
export OPENAI_WIRE_API="chat"  # Force chat completions API
```

---

### Anthropic (Claude)

```toml
model_provider = "anthropic"
model = "claude-3-5-sonnet"

[model_providers.anthropic]
name = "Anthropic"
base_url = "https://api.anthropic.com"
env_key = "ANTHROPIC_API_KEY"
wire_api = "chat"
```

**Environment**:
```bash
export ANTHROPIC_API_KEY="sk-ant-..."
```

---

### Google (Gemini)

```toml
model_provider = "google"
model = "gemini-2.0-flash-001"

[model_providers.google]
name = "Google"
base_url = "https://generativelanguage.googleapis.com/v1beta"
env_key = "GOOGLE_API_KEY"
wire_api = "chat"
```

**Environment**:
```bash
export GOOGLE_API_KEY="..."
```

---

### Ollama (Local)

```toml
model_provider = "ollama"
model = "mistral"

[model_providers.ollama]
name = "Ollama"
base_url = "http://localhost:11434/v1"
# No env_key needed for local Ollama
```

**Setup**:
```bash
# Install Ollama
curl -fsSL https://ollama.com/install.sh | sh

# Pull model
ollama pull mistral

# Start server
ollama serve
```

---

### Azure OpenAI

```toml
model_provider = "azure"
model = "gpt-5"

[model_providers.azure]
name = "Azure OpenAI"
base_url = "https://YOUR_PROJECT.openai.azure.com/openai"
env_key = "AZURE_OPENAI_API_KEY"
wire_api = "chat"
query_params = { api-version = "2025-04-01-preview" }
```

**Environment**:
```bash
export AZURE_OPENAI_API_KEY="..."
```

---

### Custom Provider

```toml
[model_providers.custom]
name = "Custom Provider"
base_url = "https://custom.api.com/v1"
env_key = "CUSTOM_API_KEY"
wire_api = "chat"

# Optional: Static HTTP headers
http_headers = { "X-Custom-Header" = "value" }

# Optional: Dynamic HTTP headers from environment
env_http_headers = { "X-Features" = "CUSTOM_FEATURES" }

# Network tuning
request_max_retries = 3
stream_max_retries = 5
stream_idle_timeout_ms = 180000  # 3 minutes
```

---

## Reasoning Configuration

### Reasoning Effort

Controls how much computational effort the model uses for reasoning.

**Options**:
- `minimal` - Fastest, least reasoning (previously "none")
- `low` - Light reasoning
- `medium` - Balanced (default)
- `high` - Maximum reasoning (premium cost)

**Configuration**:
```toml
model_reasoning_effort = "high"
```

**Use Cases**:

| Effort | Use Case | Cost | Speed |
|--------|----------|------|-------|
| `minimal` | Simple formatting, trivial tasks | Lowest | Fastest |
| `low` | Straightforward code changes | Low | Fast |
| `medium` | Moderate complexity tasks | Medium | Moderate |
| `high` | Complex refactoring, architecture | Highest | Slowest |

**Example**:
```toml
# Premium profile for complex tasks
[profiles.premium]
model = "o3"
model_reasoning_effort = "high"

# Fast profile for simple tasks
[profiles.fast]
model = "gpt-4o-mini"
model_reasoning_effort = "minimal"
```

---

### Reasoning Summary

Controls summarization of reasoning process.

**Options**:
- `auto` - Model decides (default)
- `concise` - Brief summary
- `detailed` - Comprehensive summary
- `none` - No summary

**Configuration**:
```toml
model_reasoning_summary = "detailed"
```

**Example Output**:

**`auto`**:
```
Reasoning: Analyzing code structure...
```

**`concise`**:
```
Reasoning: Identified 3 refactoring opportunities.
```

**`detailed`**:
```
Reasoning: Analyzed codebase structure. Identified 3 refactoring opportunities:
1. Extract duplicate validation logic into shared function
2. Replace switch statement with strategy pattern
3. Simplify nested conditionals with early returns
```

**`none`**:
```
(No reasoning summary shown)
```

---

### Model Verbosity (GPT-5 Only)

Controls output length/detail for GPT-5 family models.

**Options**:
- `low` - Concise output
- `medium` - Balanced (default)
- `high` - Detailed explanations

**Configuration**:
```toml
model = "gpt-5"
model_verbosity = "low"
```

**Example**:

**`low`**:
```
Refactored validation logic. See main.rs:42.
```

**`medium`**:
```
Refactored validation logic into shared function `validate_input()`
in main.rs:42. Updated 3 call sites.
```

**`high`**:
```
Refactored validation logic to improve maintainability:
1. Extracted duplicate validation code into new function `validate_input()`
   - Location: main.rs:42-58
   - Parameters: &str input, bool strict_mode
   - Returns: Result<(), ValidationError>
2. Updated call sites: handler.rs:15, api.rs:33, cli.rs:67
3. Added unit tests: tests/validation_test.rs:10-45
```

---

## Context Window Configuration

### Context Window Size

**Default**: Auto-detected based on model

**Manual Override**:
```toml
model_context_window = 128000  # 128K tokens
```

**Use Case**: New models not yet recognized by Codex

---

### Max Output Tokens

**Default**: Auto-detected based on model

**Manual Override**:
```toml
model_max_output_tokens = 16384  # 16K tokens
```

**Use Case**: Limit output length for cost control

---

## Network Tuning

### Request Retries

**Default**: 4 retries

**Configuration**:
```toml
[model_providers.openai]
request_max_retries = 6  # Increase for unreliable networks
```

**Behavior**: Exponential backoff (1s, 2s, 4s, 8s, 16s, 32s)

---

### Stream Retries

**Default**: 10 retries

**Configuration**:
```toml
[model_providers.openai]
stream_max_retries = 15  # Increase for flaky connections
```

**Use Case**: Unstable network, frequent disconnects

---

### Stream Idle Timeout

**Default**: 300,000 ms (5 minutes)

**Configuration**:
```toml
[model_providers.openai]
stream_idle_timeout_ms = 600000  # 10 minutes for slow models
```

**Use Case**: Very slow models or complex tasks

---

## Wire API Selection

### Responses API (Default for GPT-5/o3)

**Features**:
- Native reasoning support
- Reasoning summaries
- Verbosity control
- Optimized for GPT-5 family

**Configuration**:
```toml
[model_providers.openai]
wire_api = "responses"
```

---

### Chat Completions API (Legacy)

**Features**:
- Compatible with all OpenAI models
- Compatible with most third-party providers
- Simpler protocol

**Configuration**:
```toml
[model_providers.openai]
wire_api = "chat"
```

**Use Case**: Third-party providers, older models

---

## Advanced Configuration

### Force Reasoning Support

**Use Case**: Custom models that support reasoning but aren't auto-detected

**Configuration**:
```toml
model_supports_reasoning_summaries = true
```

---

### Disable Response Storage (ZDR Accounts)

**Use Case**: Zero Data Retention accounts

**Configuration**:
```toml
disable_response_storage = true
```

**Effect**: Forces Chat Completions API instead of Responses API

---

## Configuration Examples

### Premium Quality Setup

```toml
# Maximum reasoning quality
model = "o3"
model_provider = "openai"
model_reasoning_effort = "high"
model_reasoning_summary = "detailed"
model_verbosity = "high"

[model_providers.openai]
wire_api = "responses"
```

---

### Fast Iteration Setup

```toml
# Speed over quality
model = "gpt-4o-mini"
model_provider = "openai"
model_reasoning_effort = "minimal"
model_reasoning_summary = "none"
model_verbosity = "low"

[model_providers.openai]
wire_api = "chat"
```

---

### Local Development Setup

```toml
# Ollama for offline development
model = "mistral"
model_provider = "ollama"

[model_providers.ollama]
name = "Ollama"
base_url = "http://localhost:11434/v1"
```

---

### Multi-Provider Setup

```toml
# Default to OpenAI
model = "gpt-5"
model_provider = "openai"

# OpenAI provider
[model_providers.openai]
name = "OpenAI"
base_url = "https://api.openai.com/v1"
env_key = "OPENAI_API_KEY"
wire_api = "responses"

# Anthropic provider
[model_providers.anthropic]
name = "Anthropic"
base_url = "https://api.anthropic.com"
env_key = "ANTHROPIC_API_KEY"
wire_api = "chat"

# Ollama provider (local)
[model_providers.ollama]
name = "Ollama"
base_url = "http://localhost:11434/v1"

# Profiles for quick switching
[profiles.openai]
model_provider = "openai"
model = "gpt-5"

[profiles.claude]
model_provider = "anthropic"
model = "claude-3-5-sonnet"

[profiles.local]
model_provider = "ollama"
model = "mistral"
```

**Usage**:
```bash
code --profile openai "task"
code --profile claude "task"
code --profile local "task"
```

---

## Debugging Model Configuration

### Check Effective Configuration

```bash
code --config-dump | grep -A 10 "model"
```

**Output**:
```toml
model = "o3"  # From: CLI flag
model_provider = "openai"  # From: config.toml
model_reasoning_effort = "high"  # From: profile 'premium'
model_reasoning_summary = "detailed"  # From: profile 'premium'
```

---

### Test Provider Connection

```bash
# Enable debug logging
export RUST_LOG=debug
code "Hello world"
```

**Log Output**:
```
[DEBUG] Model provider: openai
[DEBUG] Base URL: https://api.openai.com/v1
[DEBUG] Wire API: responses
[DEBUG] Model: o3
[DEBUG] Reasoning effort: high
[INFO] Connection successful
```

---

## Summary

**Model Configuration** covers:
- Provider selection (OpenAI, Anthropic, Google, Ollama, Azure, custom)
- Model selection (GPT-5, o3, Claude, Gemini, etc.)
- Reasoning effort (minimal, low, medium, high)
- Reasoning summaries (auto, concise, detailed, none)
- Model verbosity (low, medium, high)
- Network tuning (retries, timeouts)
- Wire API selection (responses, chat)

**Best Practices**:
- Use profiles for different quality/speed tradeoffs
- Store API keys in environment variables
- Tune network settings for your connection quality
- Use local providers (Ollama) for offline development

**Next**: [Agent Configuration](agent-configuration.md)
