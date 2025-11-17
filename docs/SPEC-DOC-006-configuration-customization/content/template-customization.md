# Template Customization

Installing, modifying, and versioning custom templates.

---

## Overview

**Templates** provide pre-configured settings for common workflows.

**Use Cases**:
- Team-specific default configurations
- Project-specific quality gate settings
- Environment-specific profiles (dev, staging, production)
- Organization-wide standards

**Location**: `~/.code/templates/`

---

## Template Structure

### Template Format

```toml
# ~/.code/templates/premium-quality.toml

[template]
name = "Premium Quality"
version = "1.0.0"
description = "Premium quality configuration with maximum reasoning"
author = "Your Name"
created = "2025-11-17"

# Template configuration (will be merged with config.toml)
[config]
model = "o3"
model_reasoning_effort = "high"
model_reasoning_summary = "detailed"
approval_policy = "never"

[config.quality_gates]
plan = ["gemini", "claude", "code", "gpt_pro"]
tasks = ["code"]
validate = ["gemini", "claude", "code", "gpt_pro"]
audit = ["gemini", "claude", "code", "gpt_codex", "gpt_pro"]
unlock = ["gemini", "claude", "gpt_codex", "gpt_pro"]

[config.hot_reload]
enabled = true
debounce_ms = 2000

[[config.agents]]
name = "gpt_pro"
canonical_name = "gpt_pro"
command = "code"
args = ["--model", "o3", "--config", "model_reasoning_effort=high"]
enabled = true
```

---

## Installing Templates

### Method 1: Manual Installation

**Steps**:
```bash
# Create templates directory
mkdir -p ~/.code/templates

# Copy template file
cp premium-quality.toml ~/.code/templates/

# List installed templates
code --templates-list
```

---

### Method 2: Install from URL

```bash
code --template-install https://example.com/templates/premium-quality.toml
```

**Behavior**:
1. Download template file
2. Validate template structure
3. Save to `~/.code/templates/`
4. Confirm installation

---

### Method 3: Install from Git Repository

```bash
code --template-install github:theturtlecsz/code-templates/premium-quality.toml
```

**Behavior**:
1. Clone/fetch from GitHub
2. Extract template file
3. Validate and install

---

## Using Templates

### Apply Template Once

```bash
code --template premium-quality "task"
```

**Behavior**: Merges template config with `config.toml` for this session only

---

### Set Default Template

```toml
# ~/.code/config.toml

template = "premium-quality"  # Apply on every session
```

**Behavior**: Template config merged on startup

---

### Template Precedence

**Precedence** (highest to lowest):
1. CLI flags (`--model o3`)
2. Environment variables (`CODEX_MODEL=o3`)
3. **Template config** (new tier)
4. Profile (`[profiles.premium]`)
5. config.toml
6. Defaults

**Example**:
```toml
# ~/.code/config.toml
model = "gpt-5"

# ~/.code/templates/premium.toml
[config]
model = "o3"

# Usage:
code --template premium "task"
# Effective model: "o3" (template > config.toml)

code --template premium --model gpt-4o "task"
# Effective model: "gpt-4o" (CLI > template)
```

---

## Creating Custom Templates

### Step 1: Define Template Metadata

```toml
[template]
name = "My Custom Template"
version = "1.0.0"
description = "Custom configuration for my team"
author = "Team Lead"
created = "2025-11-17"
tags = ["team", "production"]  # Optional
```

---

### Step 2: Define Configuration

```toml
[config]
# Model configuration
model = "gpt-5"
model_provider = "openai"
approval_policy = "on-request"

# Quality gates
[config.quality_gates]
plan = ["gemini", "claude", "code"]
tasks = ["gemini"]
validate = ["gemini", "claude", "code"]
audit = ["gemini", "claude", "gpt_codex"]
unlock = ["gemini", "claude", "gpt_codex"]

# Agents
[[config.agents]]
name = "gemini"
canonical_name = "gemini"
command = "gemini"
enabled = true

# ... more configuration
```

---

### Step 3: Test Template

```bash
# Save template
cp my-template.toml ~/.code/templates/

# Test application
code --template my-template --dry-run "test task"

# Check effective configuration
code --template my-template --config-dump
```

---

### Step 4: Version and Document

**Version Incrementing**:
- Major: Breaking changes (agent names changed, quality gates restructured)
- Minor: New features (new agents, new quality gates)
- Patch: Bug fixes, clarifications

**Documentation**:
```toml
[template]
name = "My Template"
version = "2.1.0"  # Incremented version
changelog = """
2.1.0 (2025-11-17):
  - Added gpt_pro agent for premium reasoning
  - Increased audit quality gate to 4 agents

2.0.0 (2025-11-10):
  - BREAKING: Renamed 'code' agent to 'gpt_pro'
  - Added cost optimization profile

1.0.0 (2025-11-01):
  - Initial release
"""
```

---

## Template Examples

### Cost-Optimized Template

```toml
# ~/.code/templates/cost-optimized.toml

[template]
name = "Cost Optimized"
version = "1.0.0"
description = "Minimize cost while maintaining quality"

[config]
model = "gpt-4o-mini"
model_reasoning_effort = "low"
approval_policy = "never"

[config.quality_gates]
plan = ["gemini", "claude"]  # 2 cheap agents
tasks = ["gemini"]
validate = ["gemini", "claude"]
audit = ["gemini", "claude"]
unlock = ["gemini", "claude"]

[[config.agents]]
name = "gemini"
canonical_name = "gemini"
command = "gemini"
enabled = true

[[config.agents]]
name = "claude"
canonical_name = "claude"
command = "claude"
enabled = true
```

**Usage**:
```bash
code --template cost-optimized "task"
```

---

### CI/CD Template

```toml
# ~/.code/templates/ci-cd.toml

[template]
name = "CI/CD"
version = "1.0.0"
description = "Configuration optimized for CI/CD pipelines"

[config]
model = "gpt-4o"
approval_policy = "never"
sandbox_mode = "read-only"
disable_response_storage = false

[config.quality_gates]
plan = ["gemini", "claude"]
tasks = ["gemini"]
validate = ["gemini", "claude"]
audit = ["gemini", "claude"]
unlock = ["gemini", "claude"]

[config.hot_reload]
enabled = false  # No hot-reload in CI

[config.history]
persistence = "none"  # Don't persist history in CI
```

**Usage** (in CI):
```bash
code --template ci-cd "generate report"
```

---

### Team Standard Template

```toml
# ~/.code/templates/team-standard.toml

[template]
name = "Team Standard"
version = "1.2.0"
description = "Standard configuration for our team"
author = "Engineering Team"
organization = "ACME Corp"

[config]
model = "gpt-5"
model_provider = "openai"
approval_policy = "on-request"

# Custom quality gates for our workflow
[config.quality_gates]
plan = ["gemini", "claude", "code"]
tasks = ["gemini"]
validate = ["gemini", "claude", "code"]
audit = ["gemini", "claude", "gpt_codex"]
unlock = ["gemini", "claude", "gpt_codex"]

# Team-specific agents
[[config.agents]]
name = "team-security"
canonical_name = "security"
command = "claude"
instructions = """
Focus on ACME Corp security standards:
- OWASP Top 10 compliance
- PCI-DSS requirements for payment processing
- GDPR compliance for user data
"""
enabled = true

# Use team security agent for audit
[config.quality_gates]
audit = ["security", "gemini", "gpt_codex"]
```

---

## Template Versioning

### Version Schema

**Format**: `MAJOR.MINOR.PATCH`

**Versioning Rules**:
- **MAJOR**: Breaking changes (incompatible with previous versions)
- **MINOR**: New features (backward compatible)
- **PATCH**: Bug fixes, documentation updates

---

### Version Compatibility

**Check Template Version**:
```bash
code --template-info premium-quality
```

**Output**:
```
Template: Premium Quality
Version: 2.1.0
Compatible with: codex-rs >= 0.5.0
Author: Your Name
Description: Premium quality configuration with maximum reasoning

Changelog:
  2.1.0 (2025-11-17):
    - Added gpt_pro agent
    - Increased audit quality gate to 4 agents
  2.0.0 (2025-11-10):
    - BREAKING: Renamed agents
  1.0.0 (2025-11-01):
    - Initial release
```

---

### Automatic Template Updates

**Enable Auto-Update**:
```toml
# ~/.code/config.toml

template_auto_update = true  # Check for updates on startup
template_update_channel = "stable"  # stable, beta, nightly
```

**Manual Update**:
```bash
code --template-update premium-quality
```

**Output**:
```
Checking for updates...
New version available: 2.2.0 (current: 2.1.0)

Changelog:
  2.2.0 (2025-11-20):
    - Added performance optimizations
    - Fixed quality gate configuration bug

Update? [Y/n]: y

Downloading... ✓
Installing... ✓
Template updated successfully.
```

---

## Template Repositories

### Official Template Repository

**URL**: https://github.com/theturtlecsz/code-templates

**Templates**:
- `premium-quality.toml` - Maximum quality, high cost
- `cost-optimized.toml` - Minimum cost, acceptable quality
- `ci-cd.toml` - CI/CD pipelines
- `team-standard.toml` - Team collaboration
- `solo-developer.toml` - Individual productivity

---

### Install from Repository

```bash
# Install from official repository
code --template-install official:premium-quality

# Install from GitHub
code --template-install github:theturtlecsz/code-templates/premium-quality.toml

# Install from URL
code --template-install https://raw.githubusercontent.com/.../template.toml
```

---

### Create Your Own Repository

**Structure**:
```
my-templates/
├── README.md
├── templates.json  # Template index
└── templates/
    ├── premium.toml
    ├── cost.toml
    └── ci.toml
```

**templates.json**:
```json
{
  "repository": "my-templates",
  "version": "1.0.0",
  "templates": [
    {
      "name": "premium",
      "file": "templates/premium.toml",
      "description": "Premium quality template",
      "version": "1.0.0"
    },
    {
      "name": "cost",
      "file": "templates/cost.toml",
      "description": "Cost-optimized template",
      "version": "1.0.0"
    }
  ]
}
```

---

## Debugging Templates

### Validate Template

```bash
code --template-validate ~/.code/templates/my-template.toml
```

**Output**:
```
Validating template...

Template Metadata:
  ✓ name: "My Template"
  ✓ version: "1.0.0"
  ✓ description: Present

Configuration:
  ✓ model: "gpt-5" (valid)
  ✓ quality_gates.plan: 3 agents (valid)
  ✓ agents: 3 configured (all valid)

Template is valid ✓
```

---

### Dry-Run Template

```bash
code --template my-template --dry-run "task"
```

**Output**:
```
Dry-run mode: No actions will be executed

Effective configuration (with template "my-template"):
  model: o3 (from template)
  model_reasoning_effort: high (from template)
  quality_gates.plan: ["gemini", "claude", "code", "gpt_pro"] (from template)

Would execute: [task description]
```

---

### Compare Templates

```bash
code --template-diff premium-quality cost-optimized
```

**Output**:
```
Comparing templates:
  premium-quality v2.1.0
  cost-optimized v1.0.0

Differences:

model:
  - premium-quality: "o3"
  + cost-optimized: "gpt-4o-mini"

model_reasoning_effort:
  - premium-quality: "high"
  + cost-optimized: "low"

quality_gates.plan:
  - premium-quality: ["gemini", "claude", "code", "gpt_pro"] (4 agents)
  + cost-optimized: ["gemini", "claude"] (2 agents)
```

---

## Best Practices

### 1. Version Templates Semantically

**Good**:
```toml
[template]
version = "2.1.0"
changelog = """
2.1.0: Added gpt_pro agent
2.0.0: BREAKING: Renamed agents
1.0.0: Initial release
"""
```

---

### 2. Document Template Usage

**Include README**:
```markdown
# Premium Quality Template

**Purpose**: Maximum reasoning quality for critical projects

**Cost**: ~$4.50 per full pipeline (66% increase over default)

**When to Use**:
- Critical production features
- Security-sensitive code
- Architecture decisions

**When NOT to Use**:
- Simple formatting tasks
- Routine bug fixes
- Development iteration
```

---

### 3. Test Templates Before Distribution

```bash
# Validate template
code --template-validate my-template.toml

# Dry-run test
code --template my-template --dry-run "test task"

# Full test with real task
code --template my-template "simple test task"
```

---

### 4. Use Templates for Team Consistency

**Team workflow**:
```bash
# Install team template
code --template-install github:myorg/code-templates/team-standard.toml

# Set as default
# Add to ~/.code/config.toml:
template = "team-standard"
```

---

## Summary

**Template Customization** provides:
- Pre-configured settings for common workflows
- Team-specific default configurations
- Environment-specific profiles (dev, staging, production)
- Organization-wide standards

**Features**:
- Template installation (URL, GitHub, local)
- Template versioning (semantic versioning)
- Template repositories (official + custom)
- Automatic updates
- Template validation and dry-run

**Usage**:
```bash
# Install template
code --template-install official:premium-quality

# Use template once
code --template premium-quality "task"

# Set as default
# Add to config.toml:
template = "premium-quality"
```

**Best Practices**:
- Version templates semantically
- Document template usage (purpose, cost, when to use)
- Test templates before distribution
- Use templates for team consistency

**Next**: [Theme System](theme-system.md)
