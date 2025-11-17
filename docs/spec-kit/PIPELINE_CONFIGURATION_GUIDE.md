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
6. [Common Workflows](#6-common-workflows)
7. [Troubleshooting](#7-troubleshooting)

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

## 6. Common Workflows

This section documents the 4 primary workflow patterns supported by modular pipeline configuration, with complete use cases, cost/time analysis, and selection guidance.

### Overview: When to Use Each Workflow

| Workflow | Best For | Stages | Cost | Time | Savings |
|----------|----------|--------|------|------|---------|
| **Full Pipeline** | Production features, critical changes | All 8 | $2.46 | ~50 min | Baseline |
| **Rapid Prototyping** | POC, experimental features, quick iteration | 5/8 | $0.66 | ~20 min | 73% cost, 60% time |
| **Docs-Only** | Documentation updates, planning exercises | 3/8 | $1.15 | ~15 min | 53% cost, 70% time |
| **Code Refactoring** | Bug fixes, optimization, technical debt | 3/8 | $1.06 | ~25 min | 57% cost, 50% time |
| **Debug Single Stage** | Stage debugging, testing changes | 1/8 | $0.35 | ~11 min | 86% cost, 78% time |

---

### Workflow Pattern 1: Rapid Prototyping

**Use Case**: Proof-of-concept development, exploratory features, throwaway code, low-risk experimentation

**Stages Enabled**: `new` → `specify` → `plan` → `tasks` → `implement`

**Stages Skipped**: `validate`, `audit`, `unlock` (all validation stages)

**Cost Analysis**:
- Full pipeline: $2.46 (~50 min)
- This workflow: $0.66 (~20 min)
- **Savings**: $2.05 (73% cost reduction), ~30 min (60% time savings)

**Quality Gates**: ✅ All 3 checkpoints active (Pre-Planning, Post-Plan, Post-Tasks)

**Trade-offs**:
- ✅ Fast iteration cycles (implement → manual test → iterate)
- ✅ Low cost for experimental features
- ✅ Full planning quality gates still active
- ❌ No automated test coverage (validate skipped)
- ❌ No compliance checks (audit skipped)
- ❌ No production readiness validation (unlock skipped)

**When to Use**:
- ✅ Exploring new features or architectures
- ✅ Quick proof-of-concept demonstrations
- ✅ Throwaway prototype code
- ✅ Low-risk experimental features
- ✅ Learning or training exercises
- ❌ Production-ready features (use full pipeline)
- ❌ Security-critical changes (need audit)
- ❌ Public-facing features (need unlock validation)

**Configuration**:

```toml
# docs/SPEC-XXX/pipeline.toml
spec_id = "SPEC-XXX"
enabled_stages = ["new", "specify", "plan", "tasks", "implement"]

[skip_reasons]
validate = "Prototype: no automated tests needed yet"
audit = "Low-risk experimental feature"
unlock = "Not ready for production deployment"
```

**CLI Usage**:
```bash
# Method 1: Copy example config
cp docs/spec-kit/workflow-examples/rapid-prototyping.toml docs/SPEC-XXX/pipeline.toml
/speckit.auto SPEC-XXX

# Method 2: CLI flags (one-time)
/speckit.auto SPEC-XXX --skip-validate --skip-audit --skip-unlock
```

**Example File**: `docs/spec-kit/workflow-examples/rapid-prototyping.toml`

---

### Workflow Pattern 2: Documentation-Only

**Use Case**: README updates, architecture docs, planning refinement, spec improvements, pure documentation work

**Stages Enabled**: `specify` → `plan` → `unlock`

**Stages Skipped**: `new`, `tasks`, `implement`, `validate`, `audit` (all code stages)

**Cost Analysis**:
- Full pipeline: $2.46 (~50 min)
- This workflow: $1.15 (~15 min)
- **Savings**: $1.56 (53% cost reduction), ~35 min (70% time savings)

**Quality Gates**: ✅ 2/3 checkpoints active (Pre-Planning, Post-Plan; Post-Tasks skipped)

**Trade-offs**:
- ✅ Fast documentation updates without code generation overhead
- ✅ Strategic planning without implementation commitment
- ✅ Architecture exploration and design iteration
- ✅ Unlock stage validates final documentation quality
- ❌ No task decomposition (tasks skipped)
- ❌ No code generation (implement skipped)
- ❌ No automated testing (validate skipped)

**When to Use**:
- ✅ Architecture documentation updates
- ✅ README or user guide improvements
- ✅ Planning exercises without code commitment
- ✅ Design exploration and iteration
- ✅ Specification refinement and clarification
- ❌ Features requiring code changes
- ❌ Bug fixes or performance optimization
- ❌ Test coverage improvements

**Configuration**:

```toml
# docs/SPEC-XXX/pipeline.toml
spec_id = "SPEC-XXX"
enabled_stages = ["specify", "plan", "unlock"]

[skip_reasons]
new = "Using existing SPEC directory"
tasks = "Documentation-only: no task breakdown needed"
implement = "No code changes required"
validate = "Documentation-only: no tests to validate"
audit = "Documentation updates: skip compliance checks"
```

**CLI Usage**:
```bash
# Method 1: Copy example config
cp docs/spec-kit/workflow-examples/docs-only.toml docs/SPEC-XXX/pipeline.toml
/speckit.auto SPEC-XXX

# Method 2: CLI flags (one-time)
/speckit.auto SPEC-XXX --stages=specify,plan,unlock
```

**Example File**: `docs/spec-kit/workflow-examples/docs-only.toml`

---

### Workflow Pattern 3: Code Refactoring

**Use Case**: Bug fixes, performance optimization, technical debt reduction, code cleanup

**Stages Enabled**: `implement` → `validate` → `unlock`

**Stages Skipped**: `new`, `specify`, `plan`, `tasks`, `audit` (planning stages + audit)

**Prerequisites**:
- **REQUIRED**: Pre-existing `docs/SPEC-XXX/tasks.md` (from previous run or manual creation)
- Clear understanding of changes needed (no planning required)

**Cost Analysis**:
- Full pipeline: $2.46 (~50 min)
- This workflow: $1.06 (~25 min)
- **Savings**: $1.65 (57% cost reduction), ~25 min (50% time savings)

**Quality Gates**: ❌ 0/3 checkpoints (all planning stages skipped, no checkpoints available)

**Trade-offs**:
- ✅ Fast iteration on known changes
- ✅ Skip redundant planning when scope is clear
- ✅ Automated testing validation still enforced
- ✅ Production readiness check via unlock stage
- ❌ No AI-powered planning (relies on existing tasks.md)
- ❌ No PRD refinement (assumes clear requirements)
- ❌ No quality gate checkpoints (plan/tasks gates skipped)

**When to Use**:
- ✅ Bug fixes with clear reproduction steps
- ✅ Performance optimization with known bottlenecks
- ✅ Technical debt reduction with defined scope
- ✅ Refactoring with existing task breakdown
- ✅ Changes where requirements are already documented
- ❌ New features (requires planning)
- ❌ Exploratory work (needs PRD + plan)
- ❌ When tasks.md doesn't exist (hard dependency violation)

**Configuration**:

```toml
# docs/SPEC-XXX/pipeline.toml
spec_id = "SPEC-XXX"
enabled_stages = ["implement", "validate", "unlock"]

[quality_gates]
enabled = false  # No quality gates (plan/tasks skipped)

[skip_reasons]
new = "Using existing SPEC directory"
specify = "Requirements already clear"
plan = "Using pre-existing plan.md"
tasks = "Using pre-existing tasks.md"
audit = "Low-risk refactoring"
```

**CLI Usage**:
```bash
# Method 1: Copy example config (after verifying tasks.md exists)
ls docs/SPEC-XXX/tasks.md  # Verify prerequisite
cp docs/spec-kit/workflow-examples/code-refactoring.toml docs/SPEC-XXX/pipeline.toml
/speckit.auto SPEC-XXX

# Method 2: CLI flags (one-time)
/speckit.auto SPEC-XXX --stages=implement,validate,unlock
```

**Example File**: `docs/spec-kit/workflow-examples/code-refactoring.toml`

---

### Workflow Pattern 4: Debug Single Stage

**Use Case**: Test individual stage quality, debug consensus issues, verify agent output, test prompt changes

**Stages Enabled**: Any single stage (e.g., `plan` only)

**Stages Skipped**: All others (7/8 skipped)

**Cost Analysis** (Example: plan stage):
- Full pipeline: $2.46 (~50 min)
- Single stage (plan): $0.35 (~11 min)
- **Savings**: $2.36 (86% cost reduction), ~39 min (78% time savings)

**Note**: Cost/time varies by stage (see cost table below)

**Quality Gates**: Usually disabled (unless testing quality gates specifically)

**Trade-offs**:
- ✅ Extremely fast iteration (<15 min per test)
- ✅ Minimal cost for debugging ($0.10-$0.80 per run)
- ✅ Isolate stage behavior for troubleshooting
- ✅ Test agent prompt changes without full pipeline
- ❌ No artifact carryover (stage runs in isolation)
- ❌ May require manual setup of prerequisite artifacts
- ❌ No end-to-end validation

**When to Use**:
- ✅ Debugging consensus failures (agents disagree)
- ✅ Testing agent prompt modifications
- ✅ Verifying stage quality before full pipeline
- ✅ Reproducing stage-specific issues
- ✅ Performance benchmarking individual stages
- ✅ Agent model comparison testing
- ❌ Production workflows (use full or partial pipelines)
- ❌ When artifact carryover is needed

**Per-Stage Costs**:

| Stage | Cost | Time | Common Debug Scenarios |
|-------|------|------|------------------------|
| new | $0.00 | <1 min | SPEC-ID generation testing |
| specify | $0.10 | ~4 min | PRD refinement quality |
| plan | $0.35 | ~11 min | **Planning consensus debugging** |
| tasks | $0.10 | ~4 min | Task decomposition quality |
| implement | $0.11 | ~10 min | Code generation validation |
| validate | $0.35 | ~11 min | Test strategy debugging |
| audit | $0.80 | ~11 min | Compliance check testing |
| unlock | $0.80 | ~11 min | Ship decision validation |

**Configuration**:

```toml
# docs/SPEC-XXX/pipeline.toml
spec_id = "SPEC-XXX"
enabled_stages = ["plan"]  # Change to target stage

[quality_gates]
enabled = false  # Usually disabled for debugging
```

**CLI Usage** (Fastest Method):
```bash
# Debug plan stage
/speckit.auto SPEC-XXX --stages=plan

# Debug implement stage
/speckit.auto SPEC-XXX --stages=implement

# Debug validate stage
/speckit.auto SPEC-XXX --stages=validate

# Test multiple stages (e.g., plan → tasks)
/speckit.auto SPEC-XXX --stages=plan,tasks
```

**Example File**: `docs/spec-kit/workflow-examples/debug-single-stage.toml`

---

### Decision Matrix: Choosing the Right Workflow

Use this decision tree to select the appropriate workflow pattern:

```
START
  │
  ├─ Is this a production-ready feature or critical change?
  │  └─ YES → Use FULL PIPELINE
  │
  ├─ Does this involve code changes?
  │  │
  │  ├─ NO → Is it purely documentation?
  │  │  └─ YES → Use DOCS-ONLY WORKFLOW
  │  │
  │  └─ YES → Do you have existing tasks.md?
  │     │
  │     ├─ YES → Are requirements clear (skip planning)?
  │     │  └─ YES → Use CODE REFACTORING WORKFLOW
  │     │
  │     └─ NO → Is this a prototype/POC?
  │        │
  │        ├─ YES → Use RAPID PROTOTYPING WORKFLOW
  │        │
  │        └─ NO → Use FULL PIPELINE
  │
  └─ Are you debugging/testing a specific stage?
     └─ YES → Use DEBUG SINGLE STAGE WORKFLOW
```

**Scenario-Based Recommendations**:

| Your Goal | Recommended Workflow | Why |
|-----------|---------------------|-----|
| **Build MVP for user testing** | Rapid Prototyping | Fast iteration, no production overhead |
| **Update architecture docs** | Docs-Only | No code needed, planning validation only |
| **Fix critical bug** | Code Refactoring | Skip redundant planning, keep validation |
| **Production launch** | Full Pipeline | Complete quality assurance |
| **Test plan consensus quality** | Debug Single Stage | Isolate planning, run 3-5 times |
| **Exploratory feature spike** | Rapid Prototyping | Low cost, fast feedback |
| **Security audit fix** | Full Pipeline | Critical: need full compliance |
| **README improvement** | Docs-Only | Documentation-specific workflow |
| **Performance optimization** | Code Refactoring | Known scope, focus on implementation |
| **New complex feature** | Full Pipeline | Comprehensive validation required |

---

### Cost Comparison Table

**Full Cost Breakdown** (Post-SPEC-949 GPT-5 Baseline):

| Workflow | new | specify | plan | tasks | implement | validate | audit | unlock | **Total** | **Savings** |
|----------|-----|---------|------|-------|-----------|----------|-------|--------|-----------|-------------|
| **Full Pipeline** | $0.00 | $0.10 | $0.35 | $0.10 | $0.11 | $0.35 | $0.80 | $0.80 | **$2.46** | - |
| **Rapid Prototyping** | $0.00 | $0.10 | $0.35 | $0.10 | $0.11 | - | - | - | **$0.66** | **$2.05 (73%)** |
| **Docs-Only** | - | $0.10 | $0.35 | - | - | - | - | $0.80 | **$1.15** | **$1.56 (53%)** |
| **Code Refactoring** | - | - | - | - | $0.11 | $0.35 | - | $0.80 | **$1.06** | **$1.65 (57%)** |
| **Debug (plan)** | - | - | $0.35 | - | - | - | - | - | **$0.35** | **$2.36 (86%)** |

**Time Comparison**:

| Workflow | Time | Savings vs Full |
|----------|------|-----------------|
| Full Pipeline | ~50 min | Baseline |
| Rapid Prototyping | ~20 min | **30 min (60%)** |
| Docs-Only | ~15 min | **35 min (70%)** |
| Code Refactoring | ~25 min | **25 min (50%)** |
| Debug (plan) | ~11 min | **39 min (78%)** |

**Monthly Cost Projections** (Assuming 20 SPECs/month):

| Workflow Mix | Monthly Cost | Annual Cost | Savings vs Full |
|--------------|--------------|-------------|-----------------|
| 100% Full Pipeline | $49.20 | $590 | Baseline |
| 50% Full, 50% Rapid Prototyping | $31.20 | $374 | **$216/year (37%)** |
| 25% Full, 50% Rapid, 25% Docs | $28.50 | $342 | **$248/year (42%)** |
| 100% Rapid Prototyping | $13.20 | $158 | **$432/year (73%)** |

**Key Insight**: Strategic workflow selection based on feature requirements can reduce annual costs by 37-73% while maintaining appropriate quality levels for each use case.

---

## 7. Troubleshooting

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
