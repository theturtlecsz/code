# Quality Gate Customization

Per-checkpoint agent selection and override rules.

---

## Overview

**Quality Gates** are checkpoints in the spec-kit workflow that ensure standards are met before proceeding.

**5 Quality Gates**:
1. **Plan** - Architectural planning (multi-agent consensus)
2. **Tasks** - Task decomposition (single-agent)
3. **Validate** - Test strategy validation (multi-agent)
4. **Audit** - Security/compliance review (premium agents)
5. **Unlock** - Ship/no-ship decision (premium agents)

**Configuration**: `[quality_gates]` section in `config.toml`

---

## Quality Gate Configuration

### Basic Configuration

```toml
# ~/.code/config.toml

[quality_gates]
plan = ["gemini", "claude", "code"]        # Multi-agent planning
tasks = ["gemini"]                          # Single-agent task breakdown
validate = ["gemini", "claude", "code"]    # Multi-agent test validation
audit = ["gemini", "claude", "gpt_codex"]  # Security/compliance
unlock = ["gemini", "claude", "gpt_codex"] # Ship decision
```

---

### Field Reference

| Field | Purpose | Recommended Agents | Cost Tier |
|-------|---------|-------------------|-----------|
| `plan` | Architectural decisions | 3 agents (diverse) | Tier 2 (~$0.35) |
| `tasks` | Task breakdown | 1 agent (cheap) | Tier 1 (~$0.10) |
| `validate` | Test strategy | 3 agents (diverse) | Tier 2 (~$0.35) |
| `audit` | Security/compliance | 3+ premium | Tier 3 (~$0.80) |
| `unlock` | Ship decision | 3 premium | Tier 3 (~$0.80) |

---

## Agent Selection Strategy

### Multi-Agent Consensus (Plan, Validate)

**Purpose**: Diverse perspectives on complex decisions

**Recommended Setup**:
```toml
[quality_gates]
plan = ["gemini", "claude", "code"]  # Fast + Balanced + Strategic
```

**Agent Roles**:
- `gemini` - Fast consensus, broad coverage (12.5x cheaper)
- `claude` - Balanced reasoning, edge case detection (12x cheaper)
- `code` (GPT-5) - Strategic planning, complex reasoning (baseline)

**Why 3 Agents**:
- 2 agents: Risk of tie (no consensus)
- 3 agents: Majority vote possible
- 4+ agents: Diminishing returns, higher cost

---

### Single-Agent Deterministic (Tasks)

**Purpose**: Straightforward decomposition without opinion diversity

**Recommended Setup**:
```toml
[quality_gates]
tasks = ["gemini"]  # Single cheap agent
```

**Why Single Agent**:
- Task breakdown is mechanical (not strategic)
- No benefit from consensus
- Cost savings (1 agent vs 3)

---

### Premium Consensus (Audit, Unlock)

**Purpose**: Critical decisions requiring maximum reasoning

**Recommended Setup**:
```toml
[quality_gates]
audit = ["gemini", "claude", "gpt_codex"]  # Security-focused
unlock = ["gemini", "claude", "gpt_codex"] # Ship decision
```

**Agent Selection**:
- `gemini` - Broad vulnerability scanning
- `claude` - Edge case security analysis
- `gpt_codex` - Code-specific security patterns

**Why Premium**:
- Audit prevents security incidents ($1000s in damages)
- Unlock prevents production bugs ($1000s in incidents)
- $0.80 cost per stage justifiable for critical gates

---

## Custom Configurations

### Cost-Optimized Setup

**Goal**: Minimize cost while maintaining quality

```toml
[quality_gates]
plan = ["gemini", "claude"]  # 2 cheap agents (no GPT-5)
tasks = ["gemini"]            # Single cheap agent
validate = ["gemini", "claude"]  # 2 cheap agents
audit = ["gemini", "claude", "code"]  # 2 cheap + 1 mid-tier
unlock = ["gemini", "claude", "code"] # 2 cheap + 1 mid-tier
```

**Cost Savings**: ~60% reduction (from $2.70 to ~$1.08 per full pipeline)

**Tradeoff**: Less strategic depth (no GPT-5 on plan/validate)

---

### Premium Quality Setup

**Goal**: Maximum quality, cost secondary

```toml
[quality_gates]
plan = ["gemini", "claude", "code", "gpt_pro"]  # 4 agents (premium)
tasks = ["code"]  # GPT-5 for task breakdown
validate = ["gemini", "claude", "code", "gpt_pro"]  # 4 agents
audit = ["gemini", "claude", "code", "gpt_codex", "gpt_pro"]  # 5 agents
unlock = ["gemini", "claude", "gpt_codex", "gpt_pro"]  # 4 premium
```

**Cost**: ~$4.50 per full pipeline (66% increase)

**Benefit**: Maximum reasoning, redundant validation

---

### Specialist Configuration

**Goal**: Assign specialists per gate

```toml
# Define specialized agents
[[agents]]
name = "security-specialist"
canonical_name = "security"
command = "claude"
instructions = "Focus on OWASP Top 10, cryptography, auth/authz."

[[agents]]
name = "test-specialist"
canonical_name = "test"
command = "gemini"
instructions = "Focus on test coverage, edge cases, property-based tests."

# Quality gates with specialists
[quality_gates]
plan = ["gemini", "claude", "code"]       # General agents
tasks = ["gemini"]                         # General agent
validate = ["test", "claude", "code"]     # Test specialist for validation
audit = ["security", "claude", "gpt_codex"]  # Security specialist for audit
unlock = ["gemini", "claude", "gpt_codex"]   # General agents
```

---

## Per-Checkpoint Overrides

### Override at Runtime

Quality gates can be overridden per-command:

```bash
# Override plan agents
/speckit.plan SPEC-KIT-065 --agents gemini,claude

# Override validate agents (premium quality)
/speckit.validate SPEC-KIT-065 --agents gemini,claude,code,gpt_pro

# Override audit agents (cost-optimized)
/speckit.audit SPEC-KIT-065 --agents gemini,claude
```

**Use Case**: One-off quality/cost tradeoffs

---

### Environment Variable Overrides

```bash
# Override plan agents via env var
export SPECKIT_QUALITY_GATES_PLAN="gemini,claude,code,gpt_pro"
/speckit.plan SPEC-KIT-065

# Override tasks agents
export SPECKIT_QUALITY_GATES_TASKS="code"
/speckit.tasks SPEC-KIT-065
```

**Precedence**: Env var > config.toml

---

## Consensus Thresholds

### Minimum Consensus

**Default**: 2/3 agents (66.7%)

**Configuration**:
```toml
[quality_gates]
plan = ["gemini", "claude", "code"]
consensus_threshold = 0.67  # 2/3 agents must agree
```

**Example**:
- 3 agents, 2 agree → ✅ Pass (2/3 = 66.7%)
- 3 agents, 1 agrees → ❌ Fail (1/3 = 33.3%)

---

### Strict Consensus

**Configuration**:
```toml
[quality_gates]
unlock = ["gemini", "claude", "gpt_codex"]
consensus_threshold = 1.0  # 100% agreement required
```

**Use Case**: Critical ship decisions (unlock gate)

**Behavior**: All agents must agree to pass

---

### Relaxed Consensus

**Configuration**:
```toml
[quality_gates]
plan = ["gemini", "claude", "code"]
consensus_threshold = 0.5  # 50% majority
```

**Use Case**: Exploratory planning (early stages)

**Behavior**: Simple majority sufficient

---

## Degradation Handling

### Agent Failure Behavior

**Scenario**: One agent fails (timeout, error)

**Default Behavior**:
1. Retry up to 3 times (AR-2)
2. If still fails, continue with remaining agents
3. Consensus valid if remaining agents ≥ threshold

**Example**:
```toml
[quality_gates]
plan = ["gemini", "claude", "code"]  # 3 agents
consensus_threshold = 0.67
```

**If `code` agent fails**:
- Remaining: `gemini`, `claude` (2 agents)
- If both agree: 2/2 = 100% ≥ 67% → ✅ Pass
- If disagree: 1/2 = 50% < 67% → ❌ Fail

---

### Empty Consensus Handling

**Scenario**: All agents fail

**Behavior**: Fall back to degraded mode

**Example**:
```bash
# All agents failed
❌ Quality gate failed: No agents returned valid consensus
⚠️ Continuing in degraded mode (manual review required)
```

**User Action**: Manual PRD review and approval

---

## Quality Gate Validation

### Startup Validation

**Validation Rules**:
1. All agent names must exist in `[[agents]]`
2. Agent `canonical_name` must match quality gate references
3. Agents must be enabled
4. Minimum 1 agent per gate

**Example Error**:
```
Config validation error:
  quality_gates.plan: Agent 'unknown-agent' not found
  quality_gates.audit: Agent 'gpt_pro' exists but is disabled

Fix: Check [[agents]] configuration
```

---

### Runtime Validation

**Per-command validation**:
```bash
/speckit.plan SPEC-KIT-065
```

**Validation**:
1. All specified agents are available
2. Agents can be spawned (commands exist)
3. Consensus threshold achievable

**Example Error**:
```
❌ Cannot execute /speckit.plan:
  - Agent 'claude' command not found
  - Consensus threshold 0.67 requires ≥2 agents, only 1 available

Fix: Install missing agent or adjust consensus_threshold
```

---

## Example Configurations

### Balanced (Default)

```toml
[quality_gates]
plan = ["gemini", "claude", "code"]        # 3 agents, diverse
tasks = ["gemini"]                          # 1 agent, cheap
validate = ["gemini", "claude", "code"]    # 3 agents, diverse
audit = ["gemini", "claude", "gpt_codex"]  # 3 agents, security-focused
unlock = ["gemini", "claude", "gpt_codex"] # 3 agents, ship decision
```

**Cost**: ~$2.70 per full pipeline

---

### Cost-Optimized

```toml
[quality_gates]
plan = ["gemini", "claude"]     # 2 agents (no GPT-5)
tasks = ["gemini"]               # 1 agent
validate = ["gemini", "claude"] # 2 agents
audit = ["gemini", "claude"]    # 2 agents (no premium)
unlock = ["gemini", "claude"]   # 2 agents
```

**Cost**: ~$0.80 per full pipeline (70% reduction)

---

### Premium Quality

```toml
[quality_gates]
plan = ["gemini", "claude", "code", "gpt_pro"]  # 4 agents
tasks = ["code"]  # GPT-5 for tasks
validate = ["gemini", "claude", "code", "gpt_pro"]  # 4 agents
audit = ["gemini", "claude", "code", "gpt_codex", "gpt_pro"]  # 5 agents
unlock = ["gemini", "claude", "gpt_codex", "gpt_pro"]  # 4 agents
```

**Cost**: ~$4.50 per full pipeline (66% increase)

---

### Single-Agent (Development)

```toml
[quality_gates]
plan = ["gemini"]      # Fast iteration
tasks = ["gemini"]
validate = ["gemini"]
audit = ["gemini"]
unlock = ["gemini"]
```

**Cost**: ~$0.20 per full pipeline (93% reduction)

**Use Case**: Rapid prototyping, development iteration

---

## Debugging Quality Gates

### Check Quality Gate Configuration

```bash
code --quality-gates-dump
```

**Output**:
```toml
[quality_gates]
plan = ["gemini", "claude", "code"]  # 3 agents
tasks = ["gemini"]  # 1 agent
validate = ["gemini", "claude", "code"]  # 3 agents
audit = ["gemini", "claude", "gpt_codex"]  # 3 agents
unlock = ["gemini", "claude", "gpt_codex"]  # 3 agents

# Consensus thresholds (effective)
plan.consensus_threshold = 0.67
validate.consensus_threshold = 0.67
unlock.consensus_threshold = 1.0  # Strict (100%)
```

---

### Validate Agent Availability

```bash
code --check-quality-gates
```

**Output**:
```
Validating quality gates...

plan:
  [✓] gemini (enabled, command found)
  [✓] claude (enabled, command found)
  [✓] code (enabled, command found)

tasks:
  [✓] gemini (enabled, command found)

validate:
  [✓] gemini (enabled, command found)
  [✓] claude (enabled, command found)
  [✓] code (enabled, command found)

audit:
  [✓] gemini (enabled, command found)
  [✓] claude (enabled, command found)
  [✗] gpt_codex (disabled)

unlock:
  [✓] gemini (enabled, command found)
  [✓] claude (enabled, command found)
  [✗] gpt_codex (disabled)

⚠️ Warning: gpt_codex is disabled but referenced in audit, unlock gates
```

---

## Best Practices

### 1. Use 3 Agents for Consensus

**Recommended**: 3 agents for plan, validate, audit, unlock

**Reason**: Allows majority vote, avoids ties

---

### 2. Use 1 Agent for Deterministic Tasks

**Recommended**: 1 agent for tasks

**Reason**: Task breakdown is mechanical, no consensus needed

---

### 3. Reserve Premium Agents for Critical Gates

**Good**:
```toml
[quality_gates]
plan = ["gemini", "claude", "code"]  # Mid-tier for planning
audit = ["gemini", "claude", "gpt_pro"]  # Premium for security
unlock = ["gemini", "claude", "gpt_pro"]  # Premium for ship decision
```

---

### 4. Test Quality Gate Configuration

```bash
# Dry-run to validate config
/speckit.plan SPEC-TEST-001 --dry-run
```

---

## Summary

**Quality Gate Customization** covers:
- 5 quality gates (plan, tasks, validate, audit, unlock)
- Agent selection strategies (multi-agent, single-agent, premium)
- Cost optimization (70-93% reduction possible)
- Consensus thresholds (50-100%)
- Degradation handling (agent failures)
- Runtime overrides (CLI, env vars)

**Best Practices**:
- 3 agents for consensus gates
- 1 agent for deterministic gates
- Premium agents for critical decisions
- Test configuration with dry-run

**Next**: [Hot-Reload](hot-reload.md)
