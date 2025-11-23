# Cost Model: Tiered Pricing and Optimization

## Executive Summary

Spec-Kit achieved 75% cost reduction through strategic model routing:
- **Before**: $11 per full pipeline run
- **After**: $2.70 per full pipeline run
- **Annual savings**: $10,536 (at 100 SPECs/month)

## Tiered Model Strategy

### Tier 0: Native Rust ($0)

**Zero cost, instant execution** for deterministic tasks.

| Command | Method | Time |
|---------|--------|------|
| `/speckit.new` | Template generation | <1s |
| `/speckit.clarify` | Pattern matching | <1s |
| `/speckit.analyze` | Structural diff | <1s |
| `/speckit.checklist` | Rubric scoring | <1s |
| `/speckit.status` | State lookup | <1s |

**Key Insight**: "Agents for reasoning, NOT transactions"
- Pattern matching → Native Rust
- File operations → Native Rust
- ID generation → Native Rust

### Tier 1: Single Agent (~$0.10)

**Low-cost single model** for structured tasks.

| Command | Agent | Cost |
|---------|-------|------|
| `/speckit.specify` | gpt5-low | $0.10 |
| `/speckit.tasks` | gpt5-low | $0.10 |

**Use Case**: Tasks that need AI but don't benefit from consensus.

### Tier 2: Multi-Agent (~$0.35)

**Balanced cost-quality** with cheap models.

| Command | Agents | Cost |
|---------|--------|------|
| `/speckit.plan` | gemini-flash, claude-haiku, gpt5-medium | $0.35 |
| `/speckit.validate` | gemini-flash, claude-haiku, gpt5-medium | $0.35 |
| `/speckit.implement` | gpt_codex, claude-haiku | $0.11 |

**Cost Breakdown (Plan)**:
- Gemini 2.5 Flash: $0.008 (~2K input, 1K output)
- Claude 3.5 Haiku: $0.012 (~2K input, 1K output)
- GPT-5 Medium: $0.35 (fixed rate)
- **Total**: $0.37

### Tier 3: Premium (~$0.80)

**High-capability models** for critical decisions.

| Command | Agents | Cost |
|---------|--------|------|
| `/speckit.audit` | gemini-pro, claude-sonnet, gpt5-high | $0.80 |
| `/speckit.unlock` | gemini-pro, claude-sonnet, gpt5-high | $0.80 |

**Justification**: Security and ship decisions require maximum reasoning capability.

### Tier 4: Full Pipeline (~$2.70)

**Strategic routing** through all tiers.

```
/speckit.auto SPEC-KIT-065

Execution:
├─ specify: $0.10 (Tier 1)
├─ clarify: $0 (Tier 0)
├─ checklist: $0 (Tier 0)
├─ plan: $0.35 (Tier 2)
├─ analyze: $0 (Tier 0)
├─ tasks: $0.10 (Tier 1)
├─ analyze: $0 (Tier 0)
├─ implement: $0.11 (Tier 2)
├─ validate: $0.35 (Tier 2)
├─ audit: $0.80 (Tier 3)
└─ unlock: $0.80 (Tier 3)

Total: $2.61 + ~$0.09 overhead = $2.70
```

## Model Pricing Reference

### Per-Token Costs (API Pricing)

| Model | Input/1M | Output/1M |
|-------|----------|-----------|
| Gemini 2.5 Flash | $0.075 | $0.30 |
| Claude 3.5 Haiku | $0.25 | $1.25 |
| Gemini 2.5 Pro | $1.25 | $5.00 |
| Claude 4.5 Sonnet | $3.00 | $15.00 |

### Fixed-Rate Agents

| Agent | Cost | Tokens (approx) |
|-------|------|-----------------|
| gpt5-low | $0.10 | 3K in / 1K out |
| gpt5-medium | $0.35 | 5K in / 2K out |
| gpt5-high | $0.80 | 8K in / 3K out |
| gpt_codex | $0.11 | 4K in / 2K out |

## Cost Optimization Strategies

### 1. Stage Skipping (SPEC-948)

Skip expensive stages when not needed:

```bash
# Rapid prototyping (-75% cost)
/speckit.auto SPEC-KIT-065 --skip-validate --skip-audit --skip-unlock
# Cost: $0.66

# Documentation only (-57% cost)
/speckit.auto SPEC-KIT-065 --stages=specify,plan,unlock
# Cost: $1.15

# Code refactoring (-61% cost)
/speckit.auto SPEC-KIT-065 --stages=implement,validate,unlock
# Cost: $1.06

# Debug single stage (-87% cost)
/speckit.auto SPEC-KIT-065 --stages=plan
# Cost: $0.35
```

### 2. Native Commands First

Before spawning agents, check if native command works:

```bash
# Free quality checks before expensive planning
/speckit.clarify SPEC-KIT-065    # $0
/speckit.analyze SPEC-KIT-065    # $0
/speckit.checklist SPEC-KIT-065  # $0

# Only if quality is good:
/speckit.plan SPEC-KIT-065       # $0.35
```

### 3. Selective Premium

Reserve premium agents for critical stages:

| Stage | Question | Default | Override |
|-------|----------|---------|----------|
| Plan | Critical architecture? | Tier 2 | Use Tier 3 |
| Implement | Security-sensitive? | Tier 2 | Use Tier 3 |
| Audit | Required? | Tier 3 | Skip if low-risk |
| Unlock | Required? | Tier 3 | Skip if internal-only |

### 4. Budget Enforcement

Per-SPEC cost tracking with alerts:

```rust
// Set budget in pipeline.toml
[budget]
limit = 3.00  # $3.00 max per SPEC
warn_at = 0.80  # Warn at 80%
```

**Alert Levels**:
- **Info** (80%): "On track for $2.40"
- **Warning** (95%): "Approaching $2.85 limit"
- **Critical** (100%): "Budget exceeded, pausing pipeline"

## Cost Tracking

### Per-SPEC Tracking

```bash
/speckit.status SPEC-KIT-065

# Shows:
# Cost Summary:
#   Spent: $1.86 / $5.00 budget
#   Stages:
#     - specify: $0.10
#     - plan: $0.35
#     - tasks: $0.10
#     - implement: $0.11
#     - validate: $0.35
#     - audit: $0.80
#   Remaining: $3.14
```

### Aggregate Tracking

Evidence telemetry enables reporting:

```bash
# Monthly cost report
grep -r '"cost_usd"' docs/SPEC-OPS-004-integrated-coder-hooks/evidence/ | \
  jq -s 'map(.cost_usd) | add'

# Per-model breakdown
grep -r '"agent":' evidence/ | \
  jq -s 'group_by(.agent) | map({agent: .[0].agent, cost: map(.cost_usd) | add})'
```

## ROI Analysis

### Before Optimization

| Metric | Value |
|--------|-------|
| Cost per pipeline | $11.00 |
| SPECs per month | 100 |
| Monthly cost | $1,148 |
| Annual cost | $13,776 |

### After Optimization

| Metric | Value |
|--------|-------|
| Cost per pipeline | $2.70 |
| SPECs per month | 100 |
| Monthly cost | $270 |
| Annual cost | $3,240 |

### Savings

| Period | Savings | % Reduction |
|--------|---------|-------------|
| Per pipeline | $8.30 | 75% |
| Monthly | $878 | 76% |
| Annual | $10,536 | 76% |

## Cost-Quality Tradeoffs

### When to Use Full Pipeline ($2.70)

- Production features
- Security-sensitive code
- External-facing APIs
- Regulated industries

### When to Skip Stages ($0.66-1.15)

- Rapid prototyping
- Internal tools
- Documentation updates
- Low-risk refactoring

### When to Use Premium Only ($0.80)

- Critical bug fixes
- Security audits
- Architecture reviews
- Final release decisions

## Budget Planning

### Small Team (10 SPECs/month)

| Scenario | Monthly Cost |
|----------|--------------|
| All full pipeline | $27 |
| 50% skipped stages | $17 |
| All rapid prototype | $7 |

### Medium Team (50 SPECs/month)

| Scenario | Monthly Cost |
|----------|--------------|
| All full pipeline | $135 |
| 50% skipped stages | $85 |
| All rapid prototype | $33 |

### Large Team (200 SPECs/month)

| Scenario | Monthly Cost |
|----------|--------------|
| All full pipeline | $540 |
| 50% skipped stages | $340 |
| All rapid prototype | $132 |

## Cost Alerts Configuration

### Per-SPEC Budget

```toml
# docs/SPEC-KIT-065/pipeline.toml
[budget]
limit = 5.00
warn_percent = 80
critical_percent = 95
action_on_exceed = "pause"  # or "continue", "abort"
```

### Global Defaults

```toml
# ~/.code/config.toml
[budget.defaults]
limit = 10.00
warn_percent = 80
monthly_limit = 500.00
```

### Alert Actions

| Threshold | Action |
|-----------|--------|
| 80% | Log warning, continue |
| 95% | Display alert, prompt user |
| 100% (pause) | Pause pipeline, require confirmation |
| 100% (abort) | Stop pipeline, report costs |
| 100% (continue) | Log critical, continue anyway |

## Optimizing Specific Workflows

### Feature Development

```bash
# Full quality for production features
/speckit.auto SPEC-KIT-065
# Cost: $2.70, Time: 45 min
```

### Bug Fixes

```bash
# Skip planning stages
/speckit.auto SPEC-KIT-065 --stages=implement,validate,unlock
# Cost: $1.06, Time: 25 min
```

### Documentation

```bash
# Skip implementation and validation
/speckit.auto SPEC-KIT-065 --stages=specify,plan,unlock
# Cost: $1.15, Time: 20 min
```

### Prototyping

```bash
# Minimal validation
/speckit.auto SPEC-KIT-065 --skip-validate --skip-audit --skip-unlock
# Cost: $0.66, Time: 15 min
```

### Security Review

```bash
# Run audit separately with premium agents
/speckit.audit SPEC-KIT-065
# Cost: $0.80, Time: 12 min
```
