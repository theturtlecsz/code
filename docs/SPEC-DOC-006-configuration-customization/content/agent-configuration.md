# Agent Configuration

Multi-agent setup, subagent commands, and agent profiles.

---

## Overview

The **multi-agent system** enables consensus-driven decision-making through parallel execution of multiple AI agents.

**Use Cases**:
- **Consensus Planning** - 3+ agents agree on architecture decisions
- **Quality Gates** - Multiple agents validate test strategies
- **Diverse Perspectives** - Combine strengths of different models

**Configuration**: `[[agents]]` array in `config.toml`

---

## Agent Configuration Schema

### Agent Fields

```toml
[[agents]]
name = "gemini"                    # Display name
canonical_name = "gemini"          # Canonical identifier (for quality gates)
command = "gemini"                 # Executable command
args = []                          # Command arguments
read_only = false                  # Force read-only mode
enabled = true                     # Enable/disable agent
description = "Google Gemini"      # Human-readable description
env = {}                           # Environment variables
args_read_only = []                # Args for read-only mode (optional)
args_write = []                    # Args for write mode (optional)
instructions = ""                  # Per-agent instructions (optional)
```

---

## Default Agent Configuration

### 5-Agent Setup

```toml
# ~/.code/config.toml

# ============================================================================
# Agent 1: Gemini (Fast, Cheap Consensus)
# ============================================================================

[[agents]]
name = "gemini"
canonical_name = "gemini"
command = "gemini"
args = []
read_only = false
enabled = true
description = "Google Gemini Flash - Fast consensus agent (12.5x cheaper than GPT-5)"

# ============================================================================
# Agent 2: Claude (Balanced Reasoning)
# ============================================================================

[[agents]]
name = "claude"
canonical_name = "claude"
command = "claude"
args = []
read_only = false
enabled = true
description = "Anthropic Claude Haiku - Balanced reasoning (12x cheaper than GPT-5)"

# ============================================================================
# Agent 3: Code (Strategic Planning)
# ============================================================================

[[agents]]
name = "code"
canonical_name = "gpt_pro"
command = "code"
args = ["--model", "gpt-5"]
read_only = false
enabled = true
description = "OpenAI GPT-5 - Strategic planning and complex reasoning"

# ============================================================================
# Agent 4: GPT-Codex (Code Generation)
# ============================================================================

[[agents]]
name = "gpt_codex"
canonical_name = "gpt_codex"
command = "code"
args = ["--model", "gpt-5-codex"]
read_only = false
enabled = true
description = "OpenAI GPT-5-Codex - Specialized code generation"

# ============================================================================
# Agent 5: GPT-Pro (Premium Reasoning)
# ============================================================================

[[agents]]
name = "gpt_pro"
canonical_name = "gpt_pro"
command = "code"
args = ["--model", "o3", "--config", "model_reasoning_effort=high"]
read_only = false
enabled = false  # Disabled by default (premium cost)
description = "OpenAI o3 - Premium reasoning for critical decisions"
```

---

## Agent Properties

### name vs canonical_name

**`name`**: Display name, can change

**`canonical_name`**: Stable identifier used in quality gates

**Example**:
```toml
[[agents]]
name = "claude-sonnet"          # Display name (can evolve)
canonical_name = "claude"       # Canonical name (stable)
command = "anthropic"
```

**Quality gate reference**:
```toml
[quality_gates]
plan = ["claude"]  # Uses canonical_name, not name
```

**Benefit**: Can rename display names without breaking quality gate configs

---

### read_only Flag

**Purpose**: Force agent to run in read-only mode

**Use Case**: Agents that should never write files

**Example**:
```toml
[[agents]]
name = "readonly-advisor"
canonical_name = "advisor"
command = "gemini"
read_only = true  # Never allow writes
enabled = true
```

---

### enabled Flag

**Purpose**: Temporarily disable agent without removing config

**Use Case**: Testing, cost control, debugging

**Example**:
```toml
[[agents]]
name = "gpt_pro"
canonical_name = "gpt_pro"
command = "code"
args = ["--model", "o3"]
enabled = false  # Disable premium agent to save cost
```

---

## Advanced Agent Configuration

### args_read_only vs args_write

**Purpose**: Different arguments for read vs write modes

**Example**:
```toml
[[agents]]
name = "claude"
canonical_name = "claude"
command = "anthropic"
args = []  # Default args

# Read-only mode: Use faster, cheaper model
args_read_only = ["--model", "claude-3-haiku"]

# Write mode: Use more capable model
args_write = ["--model", "claude-3-5-sonnet"]
```

**Behavior**: Automatically selects appropriate args based on operation mode

---

### Environment Variables

**Purpose**: Pass environment variables to agent process

**Example**:
```toml
[[agents]]
name = "custom-agent"
canonical_name = "custom"
command = "/path/to/agent"
args = []
env = {
    LOG_LEVEL = "debug",
    CUSTOM_CONFIG = "/path/to/config.json",
    FEATURE_FLAGS = "experimental"
}
```

**Use Case**: Custom agents, debugging, feature flags

---

### Per-Agent Instructions

**Purpose**: Prepend instructions to every prompt sent to this agent

**Example**:
```toml
[[agents]]
name = "security-focused"
canonical_name = "security"
command = "claude"
args = []
instructions = """
You are a security-focused code reviewer. Always prioritize:
1. Input validation and sanitization
2. Authentication and authorization checks
3. Secure cryptographic practices
4. Protection against OWASP Top 10 vulnerabilities

Flag any potential security issues with HIGH severity.
"""
```

---

## Subagent Commands

### Default Commands

The spec-kit framework provides **13 slash commands** that use agents:

**Native (Tier 0 - Zero agents, FREE)**:
- `/speckit.new` - SPEC creation (template-based, no agents)
- `/speckit.clarify` - Ambiguity detection (heuristics)
- `/speckit.analyze` - Consistency checking (structural diff)
- `/speckit.checklist` - Quality scoring (rubric)
- `/speckit.status` - Status dashboard (native)

**Single-Agent (Tier 1 - 1 agent, ~$0.10)**:
- `/speckit.specify` - PRD drafting (gpt5-low)
- `/speckit.tasks` - Task decomposition (gpt5-low)

**Multi-Agent (Tier 2 - 2-3 agents, ~$0.35)**:
- `/speckit.plan` - Architectural planning (gemini-flash, claude-haiku, gpt5-medium)
- `/speckit.validate` - Test strategy (gemini-flash, claude-haiku, gpt5-medium)
- `/speckit.implement` - Code generation (gpt_codex HIGH, claude-haiku validator)

**Premium (Tier 3 - 3 premium agents, ~$0.80)**:
- `/speckit.audit` - Compliance/security (gemini-pro, claude-sonnet, gpt5-high)
- `/speckit.unlock` - Ship decision (gemini-pro, claude-sonnet, gpt5-high)

**Full Pipeline (Tier 4 - Strategic routing, ~$2.70)**:
- `/speckit.auto` - Full 6-stage pipeline

---

### Subagent Command Configuration

**Table Format**: `[[subagents.commands]]`

```toml
[[subagents.commands]]
name = "plan"                # Command name (/speckit.plan)
read_only = true             # Force read-only mode
agents = ["gemini", "claude", "code"]  # Agents to use
orchestrator_instructions = "Focus on architectural decisions and trade-offs."
agent_instructions = "Provide detailed reasoning for all recommendations."
```

**Fields**:
- `name` (string): Command name (matches `/speckit.<name>`)
- `read_only` (boolean): Force read-only mode (default: command-specific)
- `agents` (array): Agent names to enable (default: all enabled agents)
- `orchestrator_instructions` (string): Extra instructions for orchestrator
- `agent_instructions` (string): Instructions appended to each agent prompt

---

### Custom Subagent Command

**Example**: Add custom consensus command

```toml
# ~/.code/config.toml

[[subagents.commands]]
name = "review"  # Creates /speckit.review command
read_only = true
agents = ["claude", "gpt_pro"]
orchestrator_instructions = """
Focus on:
1. Code quality and maintainability
2. Performance implications
3. Security concerns
4. Test coverage adequacy
"""
agent_instructions = """
Provide specific, actionable feedback with code examples.
"""
```

**Usage**:
```bash
/speckit.review SPEC-KIT-065
```

---

## Quality Gate Integration

### Agent Selection for Quality Gates

Quality gates reference agents by **canonical_name**:

```toml
# Agents configuration
[[agents]]
name = "gemini-flash"
canonical_name = "gemini"  # ← Used in quality gates
# ...

[[agents]]
name = "claude-haiku"
canonical_name = "claude"  # ← Used in quality gates
# ...

# Quality gates configuration
[quality_gates]
plan = ["gemini", "claude", "code"]  # Uses canonical_name
tasks = ["gemini"]
validate = ["gemini", "claude", "code"]
```

**Validation**: Config loader checks that all quality gate agents exist

---

## Agent Cost Tiers

### Cost Comparison

**Based on OpenAI GPT-5 baseline (1.0x)**:

| Agent | Model | Cost per 1M tokens | Relative Cost |
|-------|-------|-------------------|---------------|
| `gemini` | Gemini Flash | $0.40 | 12.5x cheaper |
| `claude` | Claude Haiku | $0.40 | 12x cheaper |
| `code` | GPT-5 | $5.00 | 1.0x (baseline) |
| `gpt_codex` | GPT-5-Codex | $5.00 | 1.0x |
| `gpt_pro` | o3 (high effort) | $20.00 | 4x more expensive |

---

### Strategic Agent Routing

**SPEC-KIT-070**: Cost optimization via tiered agent selection

**Principle**: "Agents for reasoning, NOT transactions"

**Tier 0 (Native)**: Pattern matching → FREE
```toml
# No agents needed for:
- /speckit.new (template-based SPEC-ID generation)
- /speckit.clarify (regex-based ambiguity detection)
- /speckit.analyze (structural consistency checking)
```

**Tier 1 (Single Agent)**: Simple reasoning → $0.10
```toml
[[subagents.commands]]
name = "specify"
agents = ["gpt5-low"]  # Single cheap agent
```

**Tier 2 (Multi-Agent)**: Complex decisions → $0.35
```toml
[quality_gates]
plan = ["gemini", "claude", "gpt5-medium"]  # 3 agents, diverse perspectives
```

**Tier 3 (Premium)**: Critical decisions → $0.80
```toml
[quality_gates]
unlock = ["gemini-pro", "claude-sonnet", "gpt5-high"]  # Quality over cost
```

---

## Example Configurations

### Minimal (Single Agent)

```toml
[[agents]]
name = "gemini"
canonical_name = "gemini"
command = "gemini"
args = []
enabled = true
```

**Use Case**: Cost-conscious setup, simple tasks

---

### Balanced (3 Agents)

```toml
# Cheap consensus
[[agents]]
name = "gemini"
canonical_name = "gemini"
command = "gemini"

# Balanced reasoning
[[agents]]
name = "claude"
canonical_name = "claude"
command = "claude"

# Strategic planning
[[agents]]
name = "code"
canonical_name = "gpt_pro"
command = "code"
args = ["--model", "gpt-5"]

[quality_gates]
plan = ["gemini", "claude", "gpt_pro"]
tasks = ["gemini"]
validate = ["gemini", "claude", "gpt_pro"]
```

**Use Case**: Most production workloads

---

### Premium (5 Agents + Specialist)

```toml
# Full 5-agent setup with premium reasoning
[[agents]]
name = "gemini"
canonical_name = "gemini"
command = "gemini"
enabled = true

[[agents]]
name = "claude"
canonical_name = "claude"
command = "claude"
enabled = true

[[agents]]
name = "code"
canonical_name = "gpt_pro"
command = "code"
args = ["--model", "gpt-5"]
enabled = true

[[agents]]
name = "gpt_codex"
canonical_name = "gpt_codex"
command = "code"
args = ["--model", "gpt-5-codex"]
enabled = true

[[agents]]
name = "gpt_pro"
canonical_name = "gpt_pro"
command = "code"
args = ["--model", "o3", "--config", "model_reasoning_effort=high"]
enabled = true  # Enable for critical decisions

[quality_gates]
plan = ["gemini", "claude", "gpt_pro"]
tasks = ["gemini"]
validate = ["gemini", "claude", "gpt_pro"]
audit = ["gemini", "claude", "gpt_codex", "gpt_pro"]  # 4 agents for security
unlock = ["gemini", "claude", "gpt_pro"]
```

**Use Case**: Critical projects, maximum quality

---

## Debugging Agent Configuration

### List Configured Agents

```bash
code --agents-list
```

**Output**:
```
Configured Agents (5):
  [✓] gemini       - Google Gemini Flash (enabled)
  [✓] claude       - Anthropic Claude Haiku (enabled)
  [✓] code         - OpenAI GPT-5 (enabled)
  [✓] gpt_codex    - OpenAI GPT-5-Codex (enabled)
  [✗] gpt_pro      - OpenAI o3 (disabled)
```

---

### Validate Agent Commands

```bash
code --check-agents
```

**Output**:
```
Checking agent commands...
[✓] gemini: command 'gemini' found
[✓] claude: command 'claude' found
[✓] code: command 'code' found
[✓] gpt_codex: command 'code' found
[✗] gpt_pro: command 'code' found, but agent disabled

All enabled agents have valid commands.
```

---

## Best Practices

### 1. Use Canonical Names Consistently

**Good**:
```toml
[[agents]]
canonical_name = "gemini"  # Stable

[quality_gates]
plan = ["gemini"]  # Matches canonical_name
```

**Bad**:
```toml
[[agents]]
name = "gemini-flash-2024"  # Display name

[quality_gates]
plan = ["gemini-flash-2024"]  # ❌ Breaks if name changes
```

---

### 2. Enable Minimum Required Agents

**Good**:
```toml
# Enable only what you need
[[agents]]
canonical_name = "gemini"
enabled = true

[[agents]]
canonical_name = "claude"
enabled = true

[[agents]]
canonical_name = "gpt_pro"
enabled = false  # Disable premium agent unless needed
```

---

### 3. Use args_read_only for Cost Savings

**Example**:
```toml
[[agents]]
name = "claude"
canonical_name = "claude"
command = "anthropic"
args_read_only = ["--model", "claude-3-haiku"]  # Cheap for read-only
args_write = ["--model", "claude-3-5-sonnet"]   # Capable for writes
```

---

### 4. Leverage Per-Agent Instructions

**Example**:
```toml
[[agents]]
name = "security-agent"
canonical_name = "security"
command = "claude"
instructions = "Focus on security. Flag OWASP Top 10 vulnerabilities."
```

---

## Summary

**Agent Configuration** covers:
- 5-agent default setup (gemini, claude, code, gpt_codex, gpt_pro)
- Agent properties (name, canonical_name, command, args, enabled)
- Advanced features (args_read_only, env, instructions)
- Subagent commands (13 built-in commands)
- Quality gate integration
- Cost tiers (Tier 0-4, $0 to $0.80 per stage)

**Best Practices**:
- Use canonical_name for stability
- Enable minimum required agents
- Leverage args_read_only for cost savings
- Use per-agent instructions for specialization

**Next**: [Quality Gate Customization](quality-gate-customization.md)
