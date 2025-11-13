# PRD: Configuration Management

**SPEC-ID**: SPEC-KIT-939
**Created**: 2025-11-13
**Status**: Draft - **MEDIUM PRIORITY**
**Priority**: **P2** (User Experience + Flexibility)
**Owner**: Code
**Estimated Effort**: 22-32 hours (1-1.5 weeks)
**Dependencies**: None
**Blocks**: None

---

## 🔥 Executive Summary

**Current State**: Configuration changes require TUI restart (loses session state). Agent naming confusion (3-4 names per agent: config name, command name, model ID, agent_name field). Typos discovered late (during execution, not startup). Hardcoded quality gate agents (can't experiment with cost/quality tradeoffs). No JSON Schema (no IDE autocomplete or validation).

**Proposed State**: Hot-reload config when idle (preserves session state). Canonical name field per agent (single source of truth). Startup config validation (catch typos immediately). Configurable quality gate agents per checkpoint (experiment with agent selection). Pluggable validation layers per agent (adjust quality/cost tradeoff). JSON Schema documentation (IDE autocomplete, validation). Clear API key naming guide.

**Impact**:
- ✅ Better UX (no restarts for config changes)
- ✅ Clearer agent naming (canonical name eliminates confusion)
- ✅ Faster error detection (startup validation vs runtime discovery)
- ✅ Flexibility (configurable agents, pluggable validation)
- ✅ Better documentation (JSON Schema, API key guide)

**Source**: SPEC-931B configuration analysis identified hot-reload need (D3), canonical name (D4), and 8 configuration improvements (Q7, Q44, Q80-Q81, Q84-Q87).

---

## 1. Problem Statement

### Issue #1: Config Changes Require TUI Restart (HIGH)

**Current Behavior**:
```bash
# User edits ~/.code/config.json (add new agent)
vim ~/.code/config.json

# Must restart TUI to apply changes
# Problem: Loses current session state
# - Active quality gate progress
# - SPEC routing history
# - TUI layout preferences
```

**Impact**:
- User loses context (what SPEC was I working on?)
- Active quality gates interrupted (must re-run)
- Friction (edit → save → restart cycle)

**Proposed** (SPEC-931B D3):
```
TUI detects config file changes (filesystem watch)
→ Prompts user: "Config changed. Reload? [Y/n]"
→ If idle: Hot-reload (no restart)
→ If active quality gate: Defer until completion
```

**Benefit**: Edit config.json, TUI auto-reloads, session preserved.

---

### Issue #2: Agent Naming Confusion (MEDIUM)

**Current Reality** (SPEC-931B analysis):

Each agent has 3-4 different names:

| Agent | Config Name | Command Name | Model ID | agent_name Field |
|-------|-------------|--------------|----------|------------------|
| Gemini | `gemini` | `gemini` | `models/gemini-1.5-flash` | missing |
| Claude | `claude` | `anthropic` | `claude-sonnet-4` | missing |
| Code | `code` | `code` | `gpt-5-preview` | missing |

**Problems**:
- Documentation uses different names (confusing for users)
- Code references `config.agent_name` but field doesn't exist (should be `config.canonical_name`)
- Logs show model ID (e.g., "gemini-1.5-flash") but config uses short name ("gemini")
- CLI commands use different convention (e.g., `anthropic` vs `claude`)

**Proposed** (SPEC-931B D4):
```json
{
  "agents": [
    {
      "canonical_name": "gemini",  // NEW: Single source of truth
      "model": "models/gemini-1.5-flash",
      "command": "gemini",
      "display_name": "Gemini 1.5 Flash"
    }
  ]
}
```

**Benefit**: Code references `agent.canonical_name` everywhere, no confusion.

---

### Issue #3: Config Errors Discovered Late (MEDIUM)

**Current Behavior**:
```bash
# User has typo in config.json
{
  "agents": [
    {
      "name": "gemini",
      "model": "gemini-1.5-flash",  // Missing "models/" prefix
      "command": "gemni"              // Typo: "gemni" not "gemini"
    }
  ]
}

# TUI starts successfully (no validation)
# User triggers quality gate
# Runtime error: "Command 'gemni' not found"
# → Wasted time, frustration
```

**Impact**:
- Users waste time debugging runtime errors
- Quality gates fail unnecessarily (fixable config issues)
- No immediate feedback (error happens minutes after config edit)

**Proposed** (Q85):
```rust
// On TUI startup
fn validate_config_on_startup(config: &Config) -> Result<(), ConfigError> {
    for agent in &config.agents {
        // Check required fields
        if agent.canonical_name.is_empty() {
            return Err("Agent missing canonical_name");
        }

        // Check command exists
        if !which(&agent.command).is_ok() {
            return Err(format!("Command '{}' not found. Install {} CLI.", agent.command, agent.canonical_name));
        }

        // Check API key present
        if agent.requires_auth && !env::var(&agent.api_key_env).is_ok() {
            return Err(format!("API key '{}' not set. Run: export {}=<key>", agent.api_key_env, agent.api_key_env));
        }
    }

    Ok(())
}
```

**Benefit**: Catch typos/errors immediately on startup, not during execution.

---

### Issue #4: Hardcoded Quality Gate Agents (MEDIUM)

**Current Behavior** (quality_gate_handler.rs):
```rust
// Hardcoded agent selection
fn get_quality_gate_agents(checkpoint: QualityCheckpoint) -> Vec<&'static str> {
    match checkpoint {
        QualityCheckpoint::Plan => vec!["gemini", "claude", "code"],
        QualityCheckpoint::Tasks => vec!["gemini", "claude", "code"],
        QualityCheckpoint::Validate => vec!["gemini", "claude", "code"],
        // Always same 3 agents, no flexibility
    }
}
```

**Problem**:
- Can't experiment with different agent combinations (e.g., use cheaper agents for tasks, premium for audit)
- Can't adjust cost/quality tradeoff per checkpoint
- Requires code change to try new agent selection

**Proposed** (Q7):
```json
{
  "quality_gates": {
    "plan": {
      "agents": ["gemini", "claude", "code"],
      "required_consensus": 2
    },
    "tasks": {
      "agents": ["gemini"],  // Single cheap agent for simple stage
      "required_consensus": 1
    },
    "validate": {
      "agents": ["gemini", "claude", "code"],
      "required_consensus": 3  // Full consensus for critical stage
    },
    "audit": {
      "agents": ["gemini-pro", "claude-opus", "gpt-5-high"],  // Premium agents
      "required_consensus": 3
    }
  }
}
```

**Benefit**: Users can tune cost/quality per checkpoint (cheap for tasks, premium for audit).

---

### Issue #5: No Pluggable Validation (LOW)

**Current Behavior**:
All agents use same validation depth (hardcoded):
```rust
// Same validation for all agents
fn validate_agent_response(response: &AgentResponse) -> ValidationResult {
    // Basic checks: non-empty, JSON parseable
    if response.content.is_empty() {
        return ValidationResult::Failed("Empty response");
    }

    // No customization per agent
    ValidationResult::Passed
}
```

**Problem**:
- Can't adjust validation strictness per agent (e.g., cheaper agents → lighter validation)
- Can't experiment with validation layers (syntax only vs full semantic validation)
- One-size-fits-all (doesn't match diverse agent capabilities)

**Proposed** (Q44):
```json
{
  "agents": [
    {
      "canonical_name": "gemini",
      "validation": {
        "layers": ["syntax", "schema", "semantic"],  // Full validation
        "timeout_ms": 5000
      }
    },
    {
      "canonical_name": "cheap-agent",
      "validation": {
        "layers": ["syntax"],  // Minimal validation (faster, cheaper)
        "timeout_ms": 1000
      }
    }
  ]
}
```

**Benefit**: Tune validation strictness per agent (cost/quality tradeoff).

---

### Issue #6: No JSON Schema Documentation (MEDIUM)

**Current State**:
- Users edit config.json manually (no autocomplete)
- No validation until runtime (easy to make typos)
- No documentation of config structure (must read code)

**Proposed** (Q87):
```json
// config.schema.json
{
  "$schema": "http://json-schema.org/draft-07/schema#",
  "title": "Codex Configuration",
  "type": "object",
  "properties": {
    "agents": {
      "type": "array",
      "items": {
        "type": "object",
        "properties": {
          "canonical_name": {
            "type": "string",
            "description": "Unique identifier for this agent (used throughout codebase)"
          },
          "model": {
            "type": "string",
            "description": "Provider model ID (e.g., 'models/gemini-1.5-flash')"
          },
          "command": {
            "type": "string",
            "description": "CLI command to execute (e.g., 'gemini', 'anthropic')"
          }
        },
        "required": ["canonical_name", "model", "command"]
      }
    }
  }
}
```

**Benefit**: IDE autocomplete, validation, inline docs.

---

### Issue #7: API Key Naming Confusion (LOW)

**Context** (Q84):
Users confused by different API key env var conventions:

| Provider | Expected Var | Actual Var | Confusion |
|----------|--------------|------------|-----------|
| Google Gemini | `GOOGLE_API_KEY` | `GEMINI_API_KEY` | Which one? |
| Anthropic | `ANTHROPIC_API_KEY` | `CLAUDE_API_KEY` | Provider or model name? |
| OpenAI | `OPENAI_API_KEY` | Correct | No confusion |

**Proposed** (Q84):
Document clear convention in `docs/authentication.md`:
```markdown
# API Key Naming Convention

Use **provider name** (not model name):
- ✅ `GOOGLE_API_KEY` (for Gemini)
- ✅ `ANTHROPIC_API_KEY` (for Claude)
- ✅ `OPENAI_API_KEY` (for GPT)
- ❌ `GEMINI_API_KEY` (wrong)
- ❌ `CLAUDE_API_KEY` (wrong)
```

**Benefit**: Clear guidance, fewer support requests.

---

### Issue #8: Poor Config Error Messages (LOW)

**Current Behavior** (Q86):
```rust
// Generic error
return Err("Invalid config");

// User sees:
// Error: Invalid config
// (No context: which field? which agent? what's wrong?)
```

**Proposed**:
```rust
// Specific error with context
return Err(ConfigError::MissingField {
    agent: "gemini",
    field: "canonical_name",
    hint: "Add 'canonical_name': 'gemini' to agent config"
});

// User sees:
// Error: Agent 'gemini' missing required field 'canonical_name'
// Hint: Add "canonical_name": "gemini" to agent config in ~/.code/config.json:15
```

**Benefit**: Users can fix errors immediately (no guessing).

---

## 2. Proposed Solution

### Component Group 1: Core (D3+D4) - Hot-Reload + Canonical Name (5-6h)

#### Component 1a: Hot-Reload Config When Idle (3-4h)

**Implementation**:
```rust
// config_watcher.rs
use notify::{Watcher, RecursiveMode, Result};

pub struct ConfigWatcher {
    watcher: RecommendedWatcher,
    config_path: PathBuf,
    reload_tx: mpsc::Sender<ConfigReloadEvent>,
}

impl ConfigWatcher {
    pub fn new(config_path: PathBuf) -> Result<Self> {
        let (reload_tx, reload_rx) = mpsc::channel(10);

        let mut watcher = notify::recommended_watcher(move |res: Result<Event>| {
            if let Ok(event) = res {
                if event.kind.is_modify() {
                    reload_tx.blocking_send(ConfigReloadEvent::FileChanged).ok();
                }
            }
        })?;

        watcher.watch(&config_path, RecursiveMode::NonRecursive)?;

        Ok(ConfigWatcher { watcher, config_path, reload_tx })
    }
}

// In TUI event loop
async fn handle_config_reload_event() {
    if let Some(ConfigReloadEvent::FileChanged) = reload_rx.recv().await {
        // Check if TUI is idle (no active quality gates)
        if !is_quality_gate_active() {
            // Prompt user
            show_prompt("Config changed. Reload? [Y/n]");
            if user_confirms() {
                reload_config().await?;
                tracing::info!("Config hot-reloaded successfully");
            }
        } else {
            // Defer until quality gate completes
            tracing::info!("Config change detected, will reload after quality gate completes");
            defer_reload_until_idle();
        }
    }
}
```

---

#### Component 1b: Canonical Name Field (2h)

**Schema Change**:
```json
// OLD
{
  "agents": [
    {
      "name": "gemini",  // Ambiguous
      "model": "models/gemini-1.5-flash"
    }
  ]
}

// NEW
{
  "agents": [
    {
      "canonical_name": "gemini",  // Explicit, used everywhere
      "model": "models/gemini-1.5-flash",
      "display_name": "Gemini 1.5 Flash"  // Optional, for UI
    }
  ]
}
```

**Migration**:
```rust
// Auto-migrate on load
fn migrate_config_v1_to_v2(config: &mut Config) {
    for agent in &mut config.agents {
        if agent.canonical_name.is_none() {
            // Use existing "name" field
            agent.canonical_name = agent.name.clone();
        }
    }
}
```

---

### Component Group 2: Flexibility (Q7+Q44) - Configurable Agents + Validation (10-15h)

#### Component 2a: Configurable Quality Gate Agents (8-12h)

**Schema**:
```json
{
  "quality_gates": {
    "plan": {
      "agents": ["gemini", "claude", "code"],
      "required_consensus": 2,
      "timeout_ms": 300000
    },
    "tasks": {
      "agents": ["gemini"],
      "required_consensus": 1,
      "timeout_ms": 180000
    }
  }
}
```

**Implementation**:
```rust
// quality_gate_handler.rs
fn get_quality_gate_agents(checkpoint: QualityCheckpoint, config: &Config) -> Vec<String> {
    // Load from config instead of hardcoding
    config.quality_gates
        .get(&checkpoint.to_string())
        .map(|gate_config| gate_config.agents.clone())
        .unwrap_or_else(|| default_agents_for_checkpoint(checkpoint))
}
```

---

#### Component 2b: Pluggable Validation Layers (2-3h)

**Schema**:
```json
{
  "agents": [
    {
      "canonical_name": "gemini",
      "validation": {
        "layers": ["syntax", "schema", "semantic"],
        "timeout_ms": 5000,
        "strict_mode": true
      }
    }
  ]
}
```

**Implementation**:
```rust
// validation.rs
pub enum ValidationLayer {
    Syntax,    // JSON parseable, non-empty
    Schema,    // Matches expected structure
    Semantic,  // Content makes sense (LLM-based)
}

pub async fn validate_agent_response(
    response: &AgentResponse,
    layers: &[ValidationLayer],
) -> ValidationResult {
    for layer in layers {
        match layer {
            ValidationLayer::Syntax => validate_syntax(response)?,
            ValidationLayer::Schema => validate_schema(response)?,
            ValidationLayer::Semantic => validate_semantic(response).await?,
        }
    }
    Ok(ValidationResult::Passed)
}
```

---

### Component Group 3: Quality (Q85+Q86+Q87) - Startup Validation + Errors + Schema (6-9h)

#### Component 3a: Startup Config Validation (3-4h)

**Implementation**:
```rust
// On TUI startup
fn validate_config_on_startup(config: &Config) -> Result<(), ConfigError> {
    // 1. Required fields
    for (idx, agent) in config.agents.iter().enumerate() {
        if agent.canonical_name.is_empty() {
            return Err(ConfigError::MissingField {
                location: format!("agents[{}]", idx),
                field: "canonical_name",
            });
        }
    }

    // 2. Command availability
    for agent in &config.agents {
        if which(&agent.command).is_err() {
            return Err(ConfigError::CommandNotFound {
                agent: agent.canonical_name.clone(),
                command: agent.command.clone(),
                hint: format!("Install {} CLI or update 'command' field", agent.canonical_name),
            });
        }
    }

    // 3. API keys
    for agent in &config.agents {
        if agent.requires_auth {
            let key_var = &agent.api_key_env;
            if env::var(key_var).is_err() {
                return Err(ConfigError::MissingApiKey {
                    agent: agent.canonical_name.clone(),
                    env_var: key_var.clone(),
                    hint: format!("Run: export {}=<your-key>", key_var),
                });
            }
        }
    }

    // 4. Quality gate agents exist
    for (checkpoint, gate_config) in &config.quality_gates {
        for agent_name in &gate_config.agents {
            if !config.agents.iter().any(|a| a.canonical_name == *agent_name) {
                return Err(ConfigError::UnknownAgent {
                    checkpoint: checkpoint.clone(),
                    agent: agent_name.clone(),
                    hint: "Add agent to 'agents' array or remove from quality gate",
                });
            }
        }
    }

    Ok(())
}
```

---

#### Component 3b: Better Error Messages (1-2h)

**Error Enum**:
```rust
#[derive(Debug, thiserror::Error)]
pub enum ConfigError {
    #[error("Config file not found: {path}\nHint: Run 'codex-tui --init-config' to create default config")]
    FileNotFound { path: String },

    #[error("Agent '{agent}' missing required field '{field}'\nLocation: {location}\nHint: {hint}")]
    MissingField {
        agent: String,
        field: String,
        location: String,
        hint: String,
    },

    #[error("Command '{command}' not found for agent '{agent}'\nHint: {hint}")]
    CommandNotFound {
        agent: String,
        command: String,
        hint: String,
    },

    #[error("API key '{env_var}' not set for agent '{agent}'\nHint: {hint}")]
    MissingApiKey {
        agent: String,
        env_var: String,
        hint: String,
    },
}
```

---

#### Component 3c: JSON Schema Documentation (2-3h)

**Create Schema**:
```json
// config.schema.json
{
  "$schema": "http://json-schema.org/draft-07/schema#",
  "title": "Codex Configuration",
  "description": "Configuration for codex-tui multi-agent orchestration",
  "type": "object",
  "properties": {
    "agents": {
      "type": "array",
      "description": "List of available agents for quality gates",
      "items": { "$ref": "#/definitions/Agent" }
    },
    "quality_gates": {
      "type": "object",
      "description": "Agent selection per quality checkpoint",
      "additionalProperties": { "$ref": "#/definitions/QualityGate" }
    }
  },
  "definitions": {
    "Agent": {
      "type": "object",
      "required": ["canonical_name", "model", "command"],
      "properties": {
        "canonical_name": {
          "type": "string",
          "description": "Unique identifier (used throughout codebase)",
          "pattern": "^[a-z0-9-]+$"
        },
        "model": {
          "type": "string",
          "description": "Provider model ID (e.g., 'models/gemini-1.5-flash')"
        },
        "command": {
          "type": "string",
          "description": "CLI command to execute (e.g., 'gemini')"
        }
      }
    }
  }
}
```

**IDE Integration** (VS Code `.vscode/settings.json`):
```json
{
  "json.schemas": [
    {
      "fileMatch": ["**/config.json"],
      "url": "./config.schema.json"
    }
  ]
}
```

---

### Component Group 4: Documentation (Q84) - API Key Naming Guide (1-2h)

**Create Guide** (`docs/authentication.md`):
```markdown
# Authentication Guide

## API Key Naming Convention

Use **provider name** (not model name) for environment variables:

| Provider | Environment Variable | Example |
|----------|---------------------|---------|
| Google (Gemini) | `GOOGLE_API_KEY` | `export GOOGLE_API_KEY=AIza...` |
| Anthropic (Claude) | `ANTHROPIC_API_KEY` | `export ANTHROPIC_API_KEY=sk-ant-...` |
| OpenAI (GPT) | `OPENAI_API_KEY` | `export OPENAI_API_KEY=sk-proj-...` |

## Common Mistakes

❌ `GEMINI_API_KEY` (use `GOOGLE_API_KEY` instead)
❌ `CLAUDE_API_KEY` (use `ANTHROPIC_API_KEY` instead)

## Configuration

Update `config.json` to specify required API keys:

```json
{
  "agents": [
    {
      "canonical_name": "gemini",
      "model": "models/gemini-1.5-flash",
      "command": "gemini",
      "requires_auth": true,
      "api_key_env": "GOOGLE_API_KEY"
    }
  ]
}
```

## Verification

Check authentication status:
```bash
codex-tui --auth-status
```
```

---

## 3. Acceptance Criteria

### AC1: Hot-Reload ✅
- [ ] TUI detects config file changes (filesystem watch)
- [ ] Prompts user to reload when idle
- [ ] Defers reload if quality gate active
- [ ] Session state preserved after reload

### AC2: Canonical Name ✅
- [ ] All agents have `canonical_name` field
- [ ] Code references `canonical_name` everywhere (not `name`)
- [ ] Auto-migration for old configs

### AC3: Startup Validation ✅
- [ ] Config validated on TUI startup
- [ ] Required fields checked (canonical_name, model, command)
- [ ] Command availability verified
- [ ] API keys checked (if requires_auth)
- [ ] Clear error messages with hints

### AC4: Configurable Agents ✅
- [ ] Quality gate agents loaded from config (not hardcoded)
- [ ] Users can customize agent selection per checkpoint
- [ ] Validation: Quality gate agents must exist in agents array

### AC5: Pluggable Validation ✅
- [ ] Validation layers configurable per agent (syntax, schema, semantic)
- [ ] Timeout configurable per agent
- [ ] Strict mode toggle

### AC6: JSON Schema ✅
- [ ] config.schema.json created
- [ ] IDE autocomplete works (VS Code tested)
- [ ] Schema validation on file save

### AC7: Documentation ✅
- [ ] API key naming guide (`docs/authentication.md`)
- [ ] Clear provider → env var mapping
- [ ] Common mistakes documented

### AC8: Error Messages ✅
- [ ] All config errors include context (agent, field, location)
- [ ] Hints provided (how to fix)
- [ ] File path + line number (when applicable)

---

## 4. Technical Implementation

### Week 1: Core + Flexibility (16-21h)

**Day 1-2: Hot-Reload + Canonical Name (5-6h)**:
- Implement config file watcher (notify crate)
- Add reload prompt in TUI event loop
- Add canonical_name field to Agent struct
- Auto-migration for old configs

**Day 3-4: Configurable Agents (8-12h)**:
- Add quality_gates config section
- Update quality_gate_handler.rs to read from config
- Validation: Ensure quality gate agents exist
- Tests: Custom agent selection per checkpoint

**Day 5: Pluggable Validation (2-3h)**:
- Add validation config to Agent struct
- Implement ValidationLayer enum
- Update validation logic to use layers

**Files**:
- `codex-core/src/config.rs` (+200 LOC)
- `codex-core/src/config_watcher.rs` (+150 LOC, new)
- `codex-tui/src/chatwidget/spec_kit/quality_gate_handler.rs` (+100 LOC)

---

### Week 2: Quality + Documentation (6-11h)

**Day 1: Startup Validation (3-4h)**:
- Implement validate_config_on_startup()
- Check required fields, command availability, API keys
- Integration tests

**Day 2: Error Messages (1-2h)**:
- Create ConfigError enum with helpful messages
- Update all config loading to use ConfigError
- Test error messages (verify hints)

**Day 3: JSON Schema (2-3h)**:
- Create config.schema.json
- VS Code settings.json for IDE integration
- Test autocomplete in VS Code

**Day 4: Documentation (1-2h)**:
- Write docs/authentication.md (API key guide)
- Update README.md with configuration examples
- Code review, PR preparation

**Files**:
- `config.schema.json` (new, ~200 lines)
- `docs/authentication.md` (new, ~100 lines)
- `codex-core/src/config_error.rs` (new, ~100 LOC)

---

## 5. Success Metrics

### User Experience Metrics
- **Restart Frequency**: 90% reduction (config changes don't require restart)
- **Config Error Time-to-Fix**: 80% faster (startup validation vs runtime discovery)
- **Agent Naming Confusion**: 0 support requests (canonical name clarity)

### Flexibility Metrics
- **Custom Agent Configs**: 50%+ users experiment with custom quality gate agents
- **Validation Tuning**: 30%+ users adjust validation layers per agent

### Documentation Metrics
- **Schema Adoption**: 100% (all users see autocomplete in IDE)
- **API Key Errors**: 80% reduction (clear guide prevents mistakes)

---

## 6. Risk Analysis

### Risk 1: Hot-Reload Edge Cases (MEDIUM)

**Scenario**: Config hot-reload during quality gate transition (idle → active race condition).

**Mitigation**:
- Lock quality gate state during reload check
- If quality gate becomes active during reload, abort and defer
- Comprehensive concurrency tests

**Likelihood**: Low (tight synchronization)

---

### Risk 2: Breaking Config Changes (MEDIUM)

**Scenario**: Users with old config.json format (no canonical_name) break on upgrade.

**Mitigation**:
- Auto-migration (use "name" field if canonical_name missing)
- Clear upgrade guide in CHANGELOG.md
- Deprecation warning (v1.2.0), hard break (v2.0.0)

**Likelihood**: Medium (inevitable schema evolution)

---

## 7. Open Questions

### Q1: Should hot-reload apply immediately or require confirmation?

**Context**: Filesystem watch triggers reload, but should TUI reload automatically or prompt user?

**Decision**: PROMPT - User might have unsaved work, confirmation is safer.

---

### Q2: Should JSON Schema be bundled or external file?

**Context**: config.schema.json can be embedded in binary or external file.

**Decision**: EXTERNAL - Easier to update, users can customize.

---

## 8. Implementation Strategy

### Week 1: Core + Flexibility (21h)
- **Mon**: Hot-reload implementation (config_watcher.rs)
- **Tue**: Canonical name field + migration
- **Wed-Thu**: Configurable quality gate agents (8-12h)
- **Fri**: Pluggable validation layers (2-3h)

### Week 2: Quality + Documentation (11h)
- **Mon**: Startup validation (3-4h)
- **Tue**: Error messages (1-2h)
- **Wed**: JSON Schema (2-3h)
- **Thu**: Documentation (1-2h)
- **Fri**: Testing, PR preparation

**Total**: 32h (within 22-32h estimate, upper bound)

---

## 9. Deliverables

1. **Code Changes**:
   - `codex-core/src/config.rs` - Canonical name, quality gates config
   - `codex-core/src/config_watcher.rs` - Hot-reload logic
   - `codex-core/src/config_error.rs` - Helpful error messages
   - `codex-core/src/validation.rs` - Pluggable validation layers

2. **Configuration**:
   - `config.schema.json` - JSON Schema for IDE integration
   - `.vscode/settings.json` - VS Code integration example

3. **Documentation**:
   - `docs/authentication.md` - API key naming guide
   - `docs/configuration.md` - Config structure, hot-reload, validation

4. **Tests**:
   - Unit tests (config validation, hot-reload, error messages)
   - Integration tests (quality gates with custom agents, validation layers)

---

## 10. Validation Plan

### Unit Tests (20 tests)
- Config validation (required fields, command availability, API keys)
- Hot-reload (idle detection, defer during quality gate)
- Canonical name migration (old → new format)
- Error messages (verify context, hints)
- Validation layers (syntax, schema, semantic)

### Integration Tests (10 tests)
- Quality gates with custom agents (1 agent, 3 agents, 5 agents)
- Hot-reload during active quality gate (defer until completion)
- Startup validation failures (missing field, command not found)

### Schema Tests (3 tests)
- JSON Schema validation (valid config, invalid config)
- IDE autocomplete (VS Code integration)

**Total**: 33 tests

---

## 11. Conclusion

SPEC-939 improves configuration management through hot-reload, canonical naming, startup validation, configurable agents, pluggable validation, JSON Schema, and clear documentation. **Estimated effort: 22-32 hours over 1-1.5 weeks.**

**Key Benefits**:
- ✅ Better UX (no restarts for config changes)
- ✅ Clearer naming (canonical_name eliminates confusion)
- ✅ Faster error detection (startup validation)
- ✅ Flexibility (configurable agents, pluggable validation)
- ✅ Better documentation (JSON Schema, API key guide)

**Next Steps**:
1. Review and approve SPEC-939
2. Schedule Week 1 kickoff (hot-reload + configurable agents)
3. Coordinate JSON Schema release with v1.2.0
