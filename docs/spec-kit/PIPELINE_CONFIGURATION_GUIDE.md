# Pipeline Configuration Guide

**SPEC-948: Modular Pipeline Logic**
**Version**: 1.0
**Last Updated**: 2025-11-17

---

## Table of Contents

1. [Overview](#1-overview)
2. [Configuration Schema](#2-configuration-schema)
3. [CLI Flags Reference](#3-cli-flags-reference)
4. [Dependency Rules](#4-dependency-rules)
5. [Quality Gate Interaction](#5-quality-gate-interaction)
6. [Troubleshooting](#6-troubleshooting)

---

## 1. Overview

### What is Pipeline Configuration?

Pipeline configuration allows selective execution of spec-kit pipeline stages, enabling customized workflows for different development scenarios. Instead of always running all 8 stages (new → specify → plan → tasks → implement → validate → audit → unlock), you can:

- **Skip expensive stages** during prototyping (save 60-76% cost and time)
- **Run only relevant stages** for specific workflows (docs-only, code refactoring, debugging)
- **Balance quality vs speed** by choosing appropriate validation depth
- **Customize per-SPEC** or globally for consistent team workflows

### 3-Tier Precedence System

Configuration is determined by a 3-tier precedence hierarchy (highest to lowest):

```
1. CLI Flags           (--skip-validate, --stages=plan,tasks)
   ↓ Overrides
2. Per-SPEC Config     (docs/SPEC-*/pipeline.toml)
   ↓ Overrides
3. Global User Config  (~/.code/config.toml)
   ↓ Overrides
4. Built-in Defaults   (all 8 stages enabled)
```

**When to use each level**:

- **CLI Flags**: One-off overrides, quick experiments, debugging single stages
- **Per-SPEC Config**: Workflow-specific requirements (prototype vs production SPEC)
- **Global Config**: Personal or team-wide defaults (always skip certain stages)
- **Defaults**: No config needed, full pipeline runs automatically

**Precedence Example**:
```bash
# Global config: Skip audit stage for all SPECs
# Per-SPEC config: Skip validate and audit
# CLI flag: --skip-unlock

# Result: Skips validate, audit, unlock (union of all three)
```

### Use Cases by Workflow Type

| Workflow | Stages | Cost | Time | Use Case |
|----------|--------|------|------|----------|
| **Full Pipeline** | All 8 | $2.46 | ~50 min | Production-ready features, critical changes |
| **Rapid Prototyping** | new, specify, plan, tasks, implement | $0.56 | ~20 min | Quick POC, experimental features, low-risk changes |
| **Docs-Only** | new, specify, plan, unlock | $1.18 | ~15 min | Documentation updates, guides, planning exercises |
| **Code Refactoring** | new, tasks, implement, validate, unlock | $1.86 | ~25 min | Code cleanup, refactoring, technical debt |
| **Debug Single Stage** | plan (or any single) | $0.30 | ~11 min | Debugging specific stage, testing changes |

**Cost Baseline**: Assumes GPT-5 migration (SPEC-949) complete. Pre-SPEC-949 baseline was $2.71.

---

## 2. Configuration Schema

### TOML File Structure

Pipeline configuration uses TOML format with the following structure:

```toml
# docs/SPEC-XXX/pipeline.toml (per-SPEC config)
# OR ~/.code/config.toml with [pipeline.defaults] section (global)

spec_id = "SPEC-XXX"  # Required: SPEC identifier

# Core configuration: Which stages to execute
enabled_stages = ["new", "specify", "plan", "tasks", "implement", "validate", "audit", "unlock"]

# Quality gate configuration
[quality_gates]
enabled = true        # Enable quality gate checkpoints (default: true)
auto_resolve = true   # Auto-resolve low-severity issues (default: true)

# Optional: Model overrides per stage
[stage_models]
# plan = ["gpt-5", "claude-sonnet", "gemini-pro"]  # Override default models

# Optional: Conditional skip rules
[skip_conditions]
# validate = { type = "no_tests" }  # Skip if no test files exist

# Optional: Human-readable skip reasons (for documentation/telemetry)
[skip_reasons]
validate = "Prototype, no tests needed yet"
audit = "Low-risk experimental feature"

# Metadata (auto-generated)
created = "2025-11-17T10:00:00Z"
modified = "2025-11-17T14:30:00Z"
```

### Field Reference

#### `spec_id` (String, Required)
- **Purpose**: SPEC identifier this configuration applies to
- **Format**: `SPEC-XXX` or `SPEC-KIT-XXX`
- **Example**: `"SPEC-948"`, `"SPEC-KIT-070"`

#### `enabled_stages` (Array\<String\>, Required)
- **Purpose**: Stages to execute (in order)
- **Valid Values**: `"new"`, `"specify"`, `"plan"`, `"tasks"`, `"implement"`, `"validate"`, `"audit"`, `"unlock"`
- **Order**: Must be in logical sequence (e.g., tasks before implement)
- **Default**: All 8 stages
- **Example**:
  ```toml
  enabled_stages = ["new", "specify", "plan", "tasks", "implement"]  # Skip validate, audit, unlock
  ```

#### `quality_gates` (Table, Optional)
- **Purpose**: Configure quality gate behavior
- **Fields**:
  - `enabled` (Boolean, default: `true`): Enable/disable quality checkpoints
  - `auto_resolve` (Boolean, default: `true`): Automatically resolve low-severity issues
- **Example**:
  ```toml
  [quality_gates]
  enabled = true
  auto_resolve = false  # Require manual review of all issues
  ```

#### `stage_models` (Table, Optional)
- **Purpose**: Override default AI models for specific stages
- **Format**: `stage_name = ["model1", "model2", ...]`
- **Example**:
  ```toml
  [stage_models]
  plan = ["gpt-5", "claude-haiku", "gemini-flash"]  # Custom model selection
  ```

#### `skip_conditions` (Table, Optional)
- **Purpose**: Conditional skip rules (advanced)
- **Supported Conditions**:
  - `no_tests`: Skip if no test files exist
  - `low_risk`: Skip if SPEC priority is "low"
  - `file_count_below`: Skip if file count < threshold
  - `always`: Always skip (same as removing from enabled_stages)
  - `never`: Never skip (force execution)
- **Example**:
  ```toml
  [skip_conditions]
  validate = "no_tests"  # Skip validation if no test files
  ```

#### `skip_reasons` (Table, Optional)
- **Purpose**: Human-readable explanations for skipped stages
- **Use**: Documentation and telemetry (not functional)
- **Example**:
  ```toml
  [skip_reasons]
  validate = "Prototype phase, tests not required"
  audit = "Low-risk documentation update"
  ```

### Global Configuration Format

Global configuration in `~/.code/config.toml` uses a nested `[pipeline.defaults]` section:

```toml
# ~/.code/config.toml

[pipeline.defaults]
spec_id = "GLOBAL"
enabled_stages = ["new", "specify", "plan", "tasks", "implement", "validate", "unlock"]
# Note: Skipping audit globally (team policy)

[pipeline.defaults.quality_gates]
enabled = true
auto_resolve = true
```

**Note**: Global config applies to all SPECs unless overridden by per-SPEC config or CLI flags.

---

## 3. CLI Flags Reference

### Supported Flags

CLI flags provide the highest precedence for quick overrides without editing TOML files.

#### `--skip-{stage}`

**Purpose**: Disable a specific stage
**Syntax**: `--skip-validate`, `--skip-audit`, `--skip-unlock`
**Multiple**: Can use multiple flags: `--skip-validate --skip-audit`

**Examples**:
```bash
# Skip validation (for prototyping)
/speckit.auto SPEC-948 --skip-validate

# Skip expensive stages (validate + audit)
/speckit.auto SPEC-948 --skip-validate --skip-audit

# Skip all quality stages
/speckit.auto SPEC-948 --skip-validate --skip-audit --skip-unlock
```

**Cost Impact**:
```
Full pipeline:    $2.46
--skip-validate:  $2.16  (save $0.30, 12% reduction)
--skip-audit:     $1.66  (save $0.80, 33% reduction)
--skip-unlock:    $1.66  (save $0.80, 33% reduction)
Combined skip:    $0.56  (save $1.90, 77% reduction)
```

#### `--only-{stage}`

**Purpose**: Run ONLY the specified stage(s)
**Syntax**: `--only-plan`, `--only-tasks`, `--only-implement`
**Multiple**: Can use multiple flags to run subset

**Examples**:
```bash
# Run only plan stage (debugging)
/speckit.auto SPEC-948 --only-plan

# Run planning stages only (docs-only workflow)
/speckit.auto SPEC-948 --only-specify --only-plan --only-unlock

# Run code stages only (skip planning)
/speckit.auto SPEC-948 --only-implement --only-validate --only-unlock
```

**Behavior**:
- `--only-*` flags **replace** enabled_stages entirely
- All non-specified stages are disabled
- Equivalent to setting `enabled_stages = [specified stages]`

#### `--stages={list}`

**Purpose**: Run comma-separated list of stages (compact syntax)
**Syntax**: `--stages=plan,tasks,implement`
**Multiple**: Only use once (last wins if multiple)

**Examples**:
```bash
# Rapid prototyping workflow
/speckit.auto SPEC-948 --stages=new,specify,plan,tasks,implement

# Docs-only workflow
/speckit.auto SPEC-948 --stages=specify,plan,unlock

# Debug plan stage only
/speckit.auto SPEC-948 --stages=plan
```

**Equivalent Commands**:
```bash
# These are equivalent:
/speckit.auto SPEC-948 --stages=plan,tasks,implement
/speckit.auto SPEC-948 --only-plan --only-tasks --only-implement
```

### Flag Precedence Rules

When multiple CLI flags are used:

1. **`--stages=` wins**: Replaces enabled_stages entirely, ignores `--only-*`
2. **`--only-*` before `--skip-*`**: `--only-*` sets enabled stages, then `--skip-*` removes from that set
3. **Multiple `--skip-*` flags**: Union (skip all specified stages)
4. **Multiple `--only-*` flags**: Union (run all specified stages)

**Examples**:
```bash
# Case 1: --stages wins
/speckit.auto SPEC-948 --only-plan --stages=implement,validate
# Result: Run implement, validate (--only-plan ignored)

# Case 2: --only then --skip
/speckit.auto SPEC-948 --only-plan --only-tasks --skip-plan
# Result: Run tasks only (plan added by --only-plan, then removed by --skip-plan)

# Case 3: --skip with existing config
# Per-SPEC config: enabled_stages = ["plan", "tasks", "implement", "validate"]
/speckit.auto SPEC-948 --skip-validate
# Result: Run plan, tasks, implement (validate skipped)
```

### Common Workflows with CLI Flags

```bash
# 1. Rapid Prototyping (~60% time savings)
/speckit.auto SPEC-XXX --skip-validate --skip-audit --skip-unlock

# 2. Code Review Preparation (skip planning, focus on code)
/speckit.auto SPEC-XXX --stages=implement,validate,unlock

# 3. Documentation Updates (no code changes)
/speckit.auto SPEC-XXX --stages=specify,plan,unlock

# 4. Debug Single Stage (plan troubleshooting)
/speckit.auto SPEC-XXX --stages=plan

# 5. Security Audit Only (re-run audit with fixes)
/speckit.auto SPEC-XXX --stages=audit,unlock

# 6. Skip Quality Gates Entirely (use with caution!)
/speckit.auto SPEC-XXX --skip-validate --skip-audit
```

---

## 4. Dependency Rules

### Stage Dependencies

Each stage has **dependencies** on prior stages. Dependencies are categorized as:

- **Hard Dependencies**: MUST be enabled or have existing artifacts (causes validation error)
- **Soft Dependencies**: CAN use existing artifacts if prior stage skipped (causes warning)

#### Dependency Graph

```
new          (no dependencies)
  ↓
specify      ← depends on: new (soft)
  ↓
plan         ← depends on: specify (soft)
  ↓
tasks        ← depends on: plan (HARD)
  ↓
implement    ← depends on: tasks (HARD)
  ↓
validate     ← depends on: implement (soft)
  ↓
audit        ← depends on: implement (soft)
  ↓
unlock       ← depends on: implement (soft)
```

### Hard Dependencies (Validation Errors)

These dependencies **MUST** be satisfied or validation fails:

| Stage | Hard Dependency | Reason |
|-------|-----------------|--------|
| **tasks** | **plan** | Task decomposition requires plan structure |
| **implement** | **tasks** | Code generation requires task breakdown |

**Error Example**:
```toml
enabled_stages = ["new", "implement"]  # Missing tasks

# Validation Error:
# "Error: implement requires tasks to be enabled"
```

**Fix**:
```toml
# Option 1: Add tasks stage
enabled_stages = ["new", "specify", "plan", "tasks", "implement"]

# Option 2: Ensure tasks.md artifact exists from prior run
# (artifact discovery allows skipping if file exists)
```

### Soft Dependencies (Warnings Only)

These dependencies generate **warnings** but allow execution:

| Stage | Soft Dependency | Artifact Fallback |
|-------|-----------------|-------------------|
| specify | new | Uses spec.md (raw spec) |
| plan | specify | Uses spec.md directly |
| validate | implement | Validates existing code |
| audit | implement | Audits existing code |
| unlock | implement | Reviews existing implementation |

**Warning Example**:
```toml
enabled_stages = ["new", "plan", "tasks", "implement"]  # Missing specify

# Validation Warning:
# "Warning: plan without specify: will use existing artifacts"
```

**Artifact Discovery**:

If a stage is skipped but its artifact exists from a prior run, dependent stages use the existing file:

```bash
# First run: Full pipeline
/speckit.auto SPEC-948

# Second run: Skip plan (uses existing plan.md)
/speckit.auto SPEC-948 --skip-plan  # Tasks uses docs/SPEC-948/plan.md from first run
```

### Dependency Validation Process

Validation occurs when configuration is loaded:

```rust
// Pseudocode
for stage in enabled_stages:
    for dependency in stage.dependencies():
        if NOT dependency.is_enabled():
            if stage.is_hard_dependency(dependency):
                ERROR("stage requires dependency")
            else:
                WARNING("stage without dependency: will use existing artifacts")
```

**Validation Output**:
```
# Valid configuration
✅ All dependencies satisfied

# Configuration with warnings
⚠️  Warning: plan without specify: will use existing artifacts
⚠️  Warning: validate without implement: will use existing artifacts
✅ Validation passed (2 warnings)

# Invalid configuration
❌ Error: implement requires tasks to be enabled
❌ Configuration has 1 error(s)
```

### Dependency Best Practices

1. **Always include hard dependencies** (tasks with plan, implement with tasks)
2. **Use artifact discovery** for soft dependencies (skip stages if artifacts exist)
3. **Review warnings** to ensure existing artifacts are up-to-date
4. **Re-run skipped stages** if artifacts are stale or incorrect

---

## 5. Quality Gate Interaction

### Quality Gate Checkpoints

The pipeline includes **3 quality gate checkpoints** that run native heuristic validation:

| Checkpoint | When | Stage | Command | Purpose |
|------------|------|-------|---------|---------|
| **Pre-Planning** | After specify, before plan | specify → plan | `/speckit.clarify` | Ambiguity detection, missing requirements |
| **Post-Plan** | After plan | plan | `/speckit.checklist` | Plan completeness, rubric scoring |
| **Post-Tasks** | After tasks | tasks | `/speckit.analyze` | Consistency validation, coverage analysis |

**Quality Gates are FREE** (Tier 0: native Rust, $0, <1s each)

### Quality Gate Bypass Warnings

Skipping stages that contain quality checkpoints triggers bypass warnings:

```toml
enabled_stages = ["new", "specify", "implement"]  # Skips plan and tasks

# Warnings:
# ⚠ Skipping plan disables 2 quality gate checkpoints
# ⚠ Skipping tasks disables 1 quality gate checkpoint
```

**Checkpoint Mapping**:

| Skipped Stage | Bypassed Checkpoints | Impact |
|---------------|----------------------|--------|
| **specify** | Pre-Planning (1) | No clarity check before planning |
| **plan** | Pre-Planning + Post-Plan (2) | No plan quality assessment |
| **tasks** | Post-Tasks (1) | No task consistency validation |

### Active Checkpoint Calculation

Checkpoints are calculated based on enabled stages:

```rust
// Pre-Planning: Requires both specify AND plan
if is_enabled(specify) AND is_enabled(plan):
    run /speckit.clarify

// Post-Plan: Requires plan
if is_enabled(plan):
    run /speckit.checklist

// Post-Tasks: Requires tasks
if is_enabled(tasks):
    run /speckit.analyze
```

**Examples**:

```bash
# Full pipeline: All 3 checkpoints
/speckit.auto SPEC-948
# Checkpoints: Pre-Planning ✅, Post-Plan ✅, Post-Tasks ✅

# Skip validate/audit: All 3 checkpoints still run
/speckit.auto SPEC-948 --skip-validate --skip-audit
# Checkpoints: Pre-Planning ✅, Post-Plan ✅, Post-Tasks ✅

# Skip plan: Bypass 2 checkpoints
/speckit.auto SPEC-948 --skip-plan
# Checkpoints: Post-Tasks ✅ (Pre-Planning ❌, Post-Plan ❌)

# Skip tasks: Bypass 1 checkpoint
/speckit.auto SPEC-948 --skip-tasks
# Checkpoints: Pre-Planning ✅, Post-Plan ✅ (Post-Tasks ❌)

# Docs-only workflow: Only plan checkpoint
/speckit.auto SPEC-948 --stages=specify,plan,unlock
# Checkpoints: Pre-Planning ✅, Post-Plan ✅ (Post-Tasks ❌)
```

### Quality Gate Configuration

Quality gate behavior is controlled via `[quality_gates]` section:

```toml
[quality_gates]
enabled = true        # Enable checkpoints (default: true)
auto_resolve = true   # Auto-resolve low-severity issues (default: true)
```

**Settings**:

- **`enabled = false`**: Disable ALL quality checkpoints (not recommended)
- **`auto_resolve = false`**: Require manual review of all issues (stricter)

**Example**:
```toml
# Strict quality mode (no auto-resolution)
[quality_gates]
enabled = true
auto_resolve = false  # Human must review all clarify/checklist/analyze issues
```

### Quality vs Speed Trade-offs

| Workflow | Checkpoints | Quality Level | Use Case |
|----------|-------------|---------------|----------|
| **Full Pipeline** | 3/3 | Maximum | Production features, critical changes |
| **Skip Validation Stages** | 3/3 | High | Prototyping with full planning quality |
| **Skip Tasks** | 2/3 | Medium | Docs-only, planning exercises |
| **Skip Plan** | 1/3 | Low | Code-only refactoring, quick fixes |
| **Docs-Only (specify, plan, unlock)** | 2/3 | Medium | Documentation updates |

**Recommendation**: Always run quality gates unless extreme time constraints require skipping planning stages.

---

## 6. Troubleshooting

### Common Validation Errors

#### Error: "implement requires tasks to be enabled"

**Cause**: Hard dependency violation (implement depends on tasks)

**Solution**:
```toml
# Before (invalid)
enabled_stages = ["new", "specify", "plan", "implement"]

# After (valid)
enabled_stages = ["new", "specify", "plan", "tasks", "implement"]
```

**Or**: Ensure `docs/SPEC-XXX/tasks.md` exists from prior run (artifact discovery)

#### Error: "tasks requires plan to be enabled"

**Cause**: Hard dependency violation (tasks depends on plan)

**Solution**:
```toml
# Before (invalid)
enabled_stages = ["new", "specify", "tasks", "implement"]

# After (valid)
enabled_stages = ["new", "specify", "plan", "tasks", "implement"]
```

#### Error: "Failed to parse TOML"

**Cause**: TOML syntax error in configuration file

**Common Mistakes**:
```toml
# Incorrect: Missing quotes
enabled_stages = [new, plan, tasks]

# Correct: Quoted strings
enabled_stages = ["new", "plan", "tasks"]

# Incorrect: Wrong section syntax
quality_gates.enabled = true

# Correct: TOML table syntax
[quality_gates]
enabled = true
```

**Solution**: Validate TOML syntax using an online validator or fix quoted strings.

#### Error: "Configuration file not found"

**Cause**: `pipeline.toml` referenced but doesn't exist

**Solution**:
```bash
# Create per-SPEC config
cat > docs/SPEC-948/pipeline.toml << 'EOF'
spec_id = "SPEC-948"
enabled_stages = ["new", "specify", "plan", "tasks", "implement"]
EOF

# Or use CLI flags instead of config file
/speckit.auto SPEC-948 --skip-validate --skip-audit
```

### Common Warnings

#### Warning: "plan without specify: will use existing artifacts"

**Meaning**: Soft dependency warning, not an error

**Impact**: Plan stage will use `spec.md` directly instead of `prd.md`

**Action**: Acceptable if spec.md contains sufficient detail, otherwise add specify stage.

#### Warning: "Skipping plan disables 2 quality gate checkpoints"

**Meaning**: Quality gates bypassed due to skipped stages

**Impact**: Reduced quality validation (no clarify, no checklist)

**Action**: Review if quality trade-off is acceptable for this workflow.

#### Warning: "Partial pipeline: $0.56 vs $2.46 full (saving $1.90)"

**Meaning**: Cost savings info, not a warning

**Impact**: None (informational only)

**Action**: None required.

### Debugging Configuration Issues

#### Check Current Configuration

```bash
# View loaded configuration (includes all 3 tiers merged)
# Run pipeline with dry-run mode (if supported) or check logs

# Check per-SPEC config
cat docs/SPEC-948/pipeline.toml

# Check global config
cat ~/.code/config.toml
```

#### Test Configuration Precedence

```bash
# Test 1: No config (defaults)
/speckit.auto SPEC-TEST
# Expected: All 8 stages

# Test 2: Per-SPEC config only
echo 'spec_id = "SPEC-TEST"
enabled_stages = ["new", "plan", "tasks"]' > docs/SPEC-TEST/pipeline.toml
/speckit.auto SPEC-TEST
# Expected: 3 stages (new, plan, tasks)

# Test 3: CLI override
/speckit.auto SPEC-TEST --skip-plan
# Expected: 2 stages (new, tasks) - plan skipped by CLI
```

#### Validate TOML Syntax

```bash
# Use Rust to validate TOML
cargo run --bin toml-validator docs/SPEC-948/pipeline.toml

# Or use online validator: https://www.toml-lint.com/
```

### Performance Issues

#### Slow Configuration Loading

**Symptom**: Pipeline takes >1s to start

**Cause**: Global config parsing or file I/O

**Solution**:
- Remove global config if unused (`~/.code/config.toml`)
- Use CLI flags instead of per-SPEC TOML for one-off runs
- Check file system performance (network drives can be slow)

#### Unexpected Stage Execution

**Symptom**: Stages run that should be skipped

**Cause**: Configuration precedence or CLI flag parsing

**Debug**:
```bash
# Check which config is being loaded
# Look for log output showing config source:
# "Loading global config: ~/.code/config.toml"
# "Loading per-SPEC config: docs/SPEC-948/pipeline.toml"
# "Applying CLI overrides: --skip-validate"

# Verify CLI flag syntax
/speckit.auto SPEC-948 --skip-validate  # Correct
/speckit.auto SPEC-948 -skip-validate   # Incorrect (single dash)
```

### Getting Help

#### Documentation Resources

- **This Guide**: `/docs/spec-kit/PIPELINE_CONFIGURATION_GUIDE.md`
- **Command Reference**: `CLAUDE.md` (project root)
- **Workflow Examples**: `/docs/spec-kit/workflow-examples/*.toml`
- **Implementation**: `/codex-rs/tui/src/chatwidget/spec_kit/pipeline_config.rs`

#### Error Reporting

When reporting configuration issues, include:

1. **Configuration files** (redact sensitive info):
   ```bash
   cat docs/SPEC-XXX/pipeline.toml
   cat ~/.code/config.toml
   ```

2. **Command used**:
   ```bash
   /speckit.auto SPEC-948 --skip-validate
   ```

3. **Error message** (full output):
   ```
   Error: implement requires tasks to be enabled
   Configuration has 1 error(s)
   ```

4. **Expected vs Actual behavior**:
   - Expected: Run stages new, specify, plan, tasks, implement
   - Actual: Validation error

---

## Appendix: Quick Reference

### All CLI Flags

```bash
--skip-{stage}        # Skip single stage: --skip-validate
--only-{stage}        # Run only stage(s): --only-plan --only-tasks
--stages={list}       # Comma-separated list: --stages=plan,tasks,implement
```

### Stage Names

```
new, specify, plan, tasks, implement, validate, audit, unlock
```

### Dependency Summary

**Hard Dependencies** (errors):
- `tasks` requires `plan`
- `implement` requires `tasks`

**Soft Dependencies** (warnings):
- All others can use existing artifacts

### Cost & Time Estimates (GPT-5 Baseline)

| Stage | Cost | Time |
|-------|------|------|
| new | $0.00 | <1 min |
| specify | $0.08 | 4 min |
| plan | $0.30 | 11 min |
| tasks | $0.08 | 4 min |
| implement | $0.10 | 10 min |
| validate | $0.30 | 11 min |
| audit | $0.80 | 11 min |
| unlock | $0.80 | 11 min |
| **Full Pipeline** | **$2.46** | **~63 min** |

### Workflow Examples (Quick Copy-Paste)

```bash
# Rapid Prototyping ($0.56, ~30 min)
/speckit.auto SPEC-XXX --skip-validate --skip-audit --skip-unlock

# Docs-Only ($1.18, ~26 min)
/speckit.auto SPEC-XXX --stages=new,specify,plan,unlock

# Code Refactoring ($1.86, ~36 min)
/speckit.auto SPEC-XXX --stages=new,tasks,implement,validate,unlock

# Debug Single Stage ($0.30, ~11 min)
/speckit.auto SPEC-XXX --stages=plan
```

---

**End of Pipeline Configuration Guide**
