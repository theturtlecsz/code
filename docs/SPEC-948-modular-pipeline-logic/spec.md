# SPEC-948: Modular Pipeline Execution Logic

**Created**: 2025-11-16
**Type**: Research SPEC (Core Logic)
**Status**: Research Complete
**Priority**: P1 - High (Backend for SPEC-947 UI)
**Owner**: Code
**Estimated Research Duration**: 3-4 hours ✅ COMPLETE
**Estimated Implementation Duration**: 20-28 hours (1-1.5 weeks)

---

## Executive Summary

This research SPEC investigates the core execution logic for modular pipeline stages, enabling selective stage execution based on user configuration. This provides the backend logic that SPEC-947 (Pipeline UI) relies on for functionality.

**Strategic Value**:
- **Flexibility**: Enable partial workflows (docs-only, code-only, prototyping)
- **Efficiency**: Skip unnecessary stages (save time and cost)
- **Isolation**: Run individual stages for debugging
- **Customization**: Per-SPEC workflow overrides for special cases

**Key Capabilities**:
- Load pipeline configuration from 3 sources (CLI > per-SPEC > global)
- Filter stages based on enabled/disabled state
- Validate stage dependencies before execution
- Skip stages with logging/telemetry
- Preserve evidence for executed stages only

---

## Research Questions & Findings

### Q1: What are the stage dependencies and execution requirements?

**Finding**: Current 6-stage pipeline has clear dependency graph:

```
new (native, Tier 0)
  ↓
specify (1 agent, Tier 1) ─┐
  ↓                         │ Optional: Can skip and use raw spec.md
plan (3 agents, Tier 2) ←──┘
  ↓ [Quality Gate 1: Pre-planning clarify]
  ↓ [Quality Gate 2: Post-plan checklist]
tasks (1 agent, Tier 1)
  ↓ [Quality Gate 3: Post-tasks analyze]
implement (2 agents, Tier 2)
  ↓
validate (3 agents, Tier 2) ─┐
  ↓                           │ Independent validation stages
audit (3 agents, Tier 3) ────┤ Can be skipped individually
  ↓                           │
unlock (3 agents, Tier 3) ───┘
```

**Hard Dependencies** (cannot be violated):
- `tasks` requires `plan` OR raw `docs/SPEC-*/spec.md`
- `implement` requires `tasks` OR raw `docs/SPEC-*/tasks.md`

**Soft Dependencies** (allowed with warnings):
- `plan` can skip `specify` (uses raw spec.md, no AI refinement)
- `validate`/`audit`/`unlock` can be skipped independently (validation stages)

**Quality Gate Dependencies**:
- Skipping `plan` disables: Pre-planning clarify + Post-plan checklist
- Skipping `tasks` disables: Post-tasks analyze
- All 3 quality gates lost if plan+tasks skipped

---

### Q2: How should configuration precedence and loading work?

**Finding**: Three-tier precedence system with fail-safe defaults:

**Precedence Order** (highest to lowest):
```
1. CLI flags (--skip-validate, --only-plan)
   ↓
2. Per-SPEC config (docs/SPEC-*/pipeline.toml)
   ↓
3. Global user config (~/.code/config.toml → [pipeline.defaults])
   ↓
4. Built-in defaults (all stages enabled, all quality gates on)
```

**Loading Logic**:
```rust
/// Load pipeline configuration with precedence
pub fn load_pipeline_config(
    spec_id: &str,
    cli_overrides: Option<PipelineOverrides>
) -> Result<PipelineConfig, String> {
    // Start with built-in defaults
    let mut config = PipelineConfig::defaults();

    // Layer 1: Global user config (~/.code/config.toml)
    if let Ok(global) = load_global_pipeline_config() {
        config.merge(global);
    }

    // Layer 2: Per-SPEC config (docs/SPEC-*/pipeline.toml)
    let spec_path = format!("docs/{}/pipeline.toml", spec_id);
    if let Ok(per_spec) = load_file_config(&spec_path) {
        config.merge(per_spec);
    }

    // Layer 3: CLI overrides
    if let Some(overrides) = cli_overrides {
        config.apply_overrides(overrides);
    }

    // Validate dependencies
    config.validate()?;

    Ok(config)
}
```

**Default Configuration** (built-in):
```rust
impl PipelineConfig {
    pub fn defaults() -> Self {
        Self {
            spec_id: String::new(),
            enabled_stages: vec![
                StageType::New,
                StageType::Specify,
                StageType::Plan,
                StageType::Tasks,
                StageType::Implement,
                StageType::Validate,
                StageType::Audit,
                StageType::Unlock,
            ],
            quality_gates: QualityGateConfig {
                enabled: true,
                auto_resolve: true,
                thresholds: default_thresholds(),
            },
            stage_models: HashMap::new(),  // Use tier defaults
            skip_conditions: HashMap::new(),
        }
    }
}
```

---

### Q3: How should stage execution handle skipped stages?

**Finding**: Graceful skip with logging, telemetry, and artifact preservation:

**Execution Loop** (modified `pipeline_coordinator.rs`):
```rust
pub async fn execute_pipeline(
    spec_id: &str,
    config: &PipelineConfig,
    ctx: &mut dyn SpecKitContext
) -> Result<(), String> {
    // Get enabled stages in dependency order
    let stages = config.enabled_stages_in_order();

    for stage in stages {
        if config.is_enabled(stage) {
            tracing::info!("Executing stage: {:?}", stage);

            // Execute stage normally
            execute_stage(stage, spec_id, ctx).await?;

            // Auto-commit artifacts (SPEC-922)
            if std::env::var("SPEC_KIT_AUTO_COMMIT").unwrap_or("true".into()) == "true" {
                auto_commit_stage_artifacts(spec_id, stage).await?;
            }
        } else {
            // Log skip with reason
            let reason = config.skip_reason(stage)
                .unwrap_or("Disabled in pipeline config");

            tracing::info!("Skipping stage {:?}: {}", stage, reason);

            // Record skip in telemetry
            record_stage_skip(spec_id, stage, reason).await?;

            // Do NOT create artifacts for skipped stages
            // Evidence directory will only contain executed stages
        }
    }

    Ok(())
}
```

**Skip Telemetry** (evidence tracking):
```json
{
  "command": "speckit.auto",
  "specId": "SPEC-947",
  "stage": "validate",
  "action": "skipped",
  "reason": "Disabled in pipeline.toml",
  "timestamp": "2025-11-16T12:00:00Z",
  "configSource": "per-spec"
}
```

---

### Q4: What are the conditional skip patterns for common workflows?

**Finding**: Four primary workflow patterns identified:

**Pattern 1: Rapid Prototyping** (skip validation/audit)
```toml
# Fast iteration for experimental features
enabled_stages = ["new", "specify", "plan", "tasks", "implement"]
# Skip: validate, audit, unlock
# Time saved: ~35 min
# Cost saved: $1.50
# Use case: Proof-of-concept, exploration, throwaway code
```

**Pattern 2: Documentation-Only** (skip code stages)
```toml
# Pure documentation updates
enabled_stages = ["specify", "plan", "unlock"]
# Skip: tasks, implement, validate, audit
# Time saved: ~45 min
# Cost saved: $1.56
# Use case: README updates, architecture docs, planning refinement
```

**Pattern 3: Code Refactoring** (skip planning stages)
```toml
# Direct implementation from existing tasks
enabled_stages = ["implement", "validate", "unlock"]
# Skip: new, specify, plan, tasks
# Time saved: ~20 min
# Cost saved: $1.65
# Use case: Bug fixes, performance optimization, tech debt reduction
# Requires: Pre-existing docs/SPEC-*/tasks.md
```

**Pattern 4: Debugging Individual Stages**
```toml
# Run single stage for testing
enabled_stages = ["plan"]
# Skip: All others
# Time saved: ~48 min
# Cost saved: $2.36
# Use case: Test plan quality, debug consensus issues, verify agent output
```

**Conditional Skip Logic**:
```rust
/// Auto-skip stages based on conditions
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SkipCondition {
    /// Skip if no test files exist in docs/SPEC-*/
    NoTests,

    /// Skip if SPEC priority is "low" in spec.md
    LowRisk,

    /// Skip if file count < threshold (e.g., docs-only if no .rs files)
    FileCountBelow { pattern: String, count: usize },

    /// Always skip
    Always,

    /// Never skip (force execution)
    Never,
}

impl SkipCondition {
    pub fn evaluate(&self, spec_id: &str) -> bool {
        match self {
            Self::NoTests => {
                // Check if docs/SPEC-*/tests/ exists or any *_test.rs files
                !has_test_files(spec_id)
            }
            Self::LowRisk => {
                // Parse spec.md for priority: low
                is_low_priority_spec(spec_id)
            }
            Self::FileCountBelow { pattern, count } => {
                count_matching_files(spec_id, pattern) < *count
            }
            Self::Always => true,
            Self::Never => false,
        }
    }
}
```

---

## Technical Architecture

### Core Data Structures

```rust
/// Pipeline configuration (TOML schema)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PipelineConfig {
    /// SPEC ID this configuration applies to
    pub spec_id: String,

    /// Enabled stages (order preserved, executed sequentially)
    pub enabled_stages: Vec<StageType>,

    /// Quality gate configuration
    pub quality_gates: QualityGateConfig,

    /// Model overrides per stage (optional)
    #[serde(default)]
    pub stage_models: HashMap<StageType, Vec<String>>,

    /// Conditional skip rules (optional)
    #[serde(default)]
    pub skip_conditions: HashMap<StageType, SkipCondition>,

    /// Metadata
    pub created: Option<String>,
    pub modified: Option<String>,
}

/// Stage types in pipeline
#[derive(Debug, Clone, Copy, Serialize, Deserialize, Hash, Eq, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum StageType {
    New,
    Specify,
    Plan,
    Tasks,
    Implement,
    Validate,
    Audit,
    Unlock,
}

impl StageType {
    /// Get required dependencies for this stage
    pub fn dependencies(&self) -> Vec<StageType> {
        match self {
            Self::New => vec![],
            Self::Specify => vec![Self::New],
            Self::Plan => vec![Self::Specify],  // Soft: can use raw spec.md
            Self::Tasks => vec![Self::Plan],
            Self::Implement => vec![Self::Tasks],
            Self::Validate => vec![Self::Implement],
            Self::Audit => vec![Self::Implement],
            Self::Unlock => vec![Self::Implement],
        }
    }

    /// Get cost estimate for this stage
    pub fn cost_estimate(&self) -> f64 {
        match self {
            Self::New => 0.0,           // Native
            Self::Specify => 0.10,      // 1 agent
            Self::Plan => 0.35,         // 3 agents
            Self::Tasks => 0.10,        // 1 agent
            Self::Implement => 0.11,    // 2 agents
            Self::Validate => 0.35,     // 3 agents
            Self::Audit => 0.80,        // 3 premium
            Self::Unlock => 0.80,       // 3 premium
        }
    }

    /// Get time estimate for this stage (minutes)
    pub fn duration_estimate(&self) -> u32 {
        match self {
            Self::New => 1,             // <1s native
            Self::Specify => 4,         // 3-5 min
            Self::Plan => 11,           // 10-12 min
            Self::Tasks => 4,           // 3-5 min
            Self::Implement => 10,      // 8-12 min
            Self::Validate => 11,       // 10-12 min
            Self::Audit => 11,          // 10-12 min
            Self::Unlock => 11,         // 10-12 min
        }
    }

    /// Does this stage have a quality gate checkpoint?
    pub fn has_quality_gate(&self) -> bool {
        matches!(self, Self::Plan | Self::Tasks)
    }
}
```

---

### Configuration Validation

```rust
impl PipelineConfig {
    /// Validate configuration for errors and warnings
    pub fn validate(&self) -> Result<ValidationResult, String> {
        let mut errors = Vec::new();
        let mut warnings = Vec::new();

        // Check hard dependencies
        for stage in &self.enabled_stages {
            for dep in stage.dependencies() {
                if !self.is_enabled(dep) {
                    // Check if dependency is hard requirement
                    if is_hard_dependency(*stage, dep) {
                        errors.push(format!(
                            "Error: {} requires {} to be enabled",
                            stage, dep
                        ));
                    } else {
                        warnings.push(format!(
                            "Warning: {} without {}: will use existing artifacts",
                            stage, dep
                        ));
                    }
                }
            }
        }

        // Check quality gate bypass
        if !self.is_enabled(StageType::Plan) {
            warnings.push(
                "⚠ Skipping plan disables 2 quality gate checkpoints".into()
            );
        }
        if !self.is_enabled(StageType::Tasks) {
            warnings.push(
                "⚠ Skipping tasks disables 1 quality gate checkpoint".into()
            );
        }

        // Check cost implications
        let total_cost: f64 = self.enabled_stages.iter()
            .map(|s| s.cost_estimate())
            .sum();
        let full_cost = 2.71;  // All stages
        if total_cost < full_cost * 0.5 {
            warnings.push(format!(
                "ℹ️ Partial pipeline: ${:.2} vs ${:.2} full (saving ${:.2})",
                total_cost, full_cost, full_cost - total_cost
            ));
        }

        if errors.is_empty() {
            Ok(ValidationResult { warnings })
        } else {
            Err(format!("Configuration has {} error(s):\n{}",
                errors.len(),
                errors.join("\n")
            ))
        }
    }

    fn is_enabled(&self, stage: StageType) -> bool {
        self.enabled_stages.contains(&stage)
    }
}

/// Hard dependency: Must be enabled or have pre-existing artifact
fn is_hard_dependency(stage: StageType, dep: StageType) -> bool {
    match (stage, dep) {
        (StageType::Tasks, StageType::Plan) => true,      // Tasks needs plan
        (StageType::Implement, StageType::Tasks) => true, // Implement needs tasks
        _ => false,
    }
}
```

---

### Q2: How should CLI flag parsing work for stage control?

**Finding**: Multiple CLI flag patterns for flexibility:

**Skip Flags** (disable specific stages):
```bash
/speckit.auto SPEC-947 --skip-validate --skip-audit

# Implementation:
pub struct PipelineOverrides {
    skip_stages: Vec<StageType>,
    only_stages: Option<Vec<StageType>>,
}

impl PipelineOverrides {
    pub fn from_cli_args(args: &[String]) -> Self {
        let mut skip_stages = Vec::new();

        for arg in args {
            if let Some(stage_name) = arg.strip_prefix("--skip-") {
                if let Ok(stage) = StageType::from_str(stage_name) {
                    skip_stages.push(stage);
                }
            }
        }

        Self { skip_stages, only_stages: None }
    }
}
```

**Only Flags** (enable specific stages, disable all others):
```bash
/speckit.auto SPEC-947 --only-plan --only-tasks

# Or:
/speckit.auto SPEC-947 --stages=plan,tasks,implement
```

**Configure Flag** (launch interactive configurator):
```bash
/speckit.auto SPEC-947 --configure
```

---

### Q3: How should skipped stages affect evidence and artifacts?

**Finding**: Skip evidence, preserve artifact discovery:

**Evidence Strategy**:
- **Executed stages**: Full evidence (telemetry JSON, consensus artifacts, cost tracking)
- **Skipped stages**: Minimal skip record (timestamp, reason, config source)
- **Evidence directory**: Only contains executed + skipped metadata

**Example Evidence Structure**:
```
docs/SPEC-OPS-004-integrated-coder-hooks/evidence/commands/SPEC-947/
├── speckit-new_2025-11-16T10:00:00Z.json          (executed)
├── speckit-specify_2025-11-16T10:01:00Z.json      (executed)
├── speckit-plan_2025-11-16T10:05:00Z.json         (executed)
├── speckit-tasks_2025-11-16T10:15:00Z.json        (executed)
├── speckit-implement_2025-11-16T10:20:00Z.json    (executed)
├── speckit-validate_SKIPPED.json                   (skipped - metadata only)
├── speckit-audit_SKIPPED.json                      (skipped - metadata only)
└── speckit-unlock_2025-11-16T10:30:00Z.json       (executed)
```

**Skip Metadata** (minimal):
```json
{
  "command": "speckit.validate",
  "specId": "SPEC-947",
  "stage": "validate",
  "action": "skipped",
  "reason": "Disabled in pipeline.toml: no test files for UI research SPEC",
  "configSource": "per-spec",
  "timestamp": "2025-11-16T10:25:00Z",
  "schemaVersion": "1.0"
}
```

**Artifact Discovery** (existing artifacts used):
- If `plan` skipped: Look for `docs/SPEC-947-*/plan.md` (from previous run or manual creation)
- If `tasks` skipped: Look for `docs/SPEC-947-*/tasks.md` (required for implement)
- Error if artifact required but missing: "Error: tasks.md not found (required for implement stage)"

---

### Q4: How should quality gates interact with skipped stages?

**Finding**: Checkpoint bypass with explicit user awareness:

**Quality Gate Checkpoint Logic**:
```rust
/// Determine which quality gate checkpoints apply
pub fn active_quality_gates(config: &PipelineConfig) -> Vec<QualityCheckpoint> {
    let mut checkpoints = Vec::new();

    // Checkpoint 1: Pre-planning (clarify)
    if config.is_enabled(StageType::Specify)
        && config.is_enabled(StageType::Plan)
    {
        checkpoints.push(QualityCheckpoint::PrePlanning);
    }

    // Checkpoint 2: Post-plan (checklist)
    if config.is_enabled(StageType::Plan) {
        checkpoints.push(QualityCheckpoint::PostPlan);
    }

    // Checkpoint 3: Post-tasks (analyze)
    if config.is_enabled(StageType::Tasks) {
        checkpoints.push(QualityCheckpoint::PostTasks);
    }

    checkpoints
}

/// Show quality gate bypass warning modal
pub async fn confirm_quality_gate_bypass(
    ctx: &mut dyn SpecKitContext,
    bypassed_gates: Vec<QualityCheckpoint>
) -> bool {
    let message = format!(
        "⚠️ Configuration bypasses {} quality gate checkpoint(s):\n\n{}\n\n\
         Quality gates help catch issues early. Proceed anyway?",
        bypassed_gates.len(),
        bypassed_gates.iter()
            .map(|g| format!("  • {}", g.description()))
            .collect::<Vec<_>>()
            .join("\n")
    );

    ctx.show_confirmation_dialog("Quality Gate Bypass", &message).await
}
```

**Execution Flow** (with quality gate awareness):
1. Load `PipelineConfig`
2. Calculate active quality gates based on enabled stages
3. If <3 quality gates active: Show bypass warning modal
4. User confirms or cancels
5. Execute pipeline with active checkpoints only

---

## Implementation Recommendations

### Phase 1: Config Data Layer (6-8 hours)

**Tasks**:
- Create `pipeline_config.rs` module
- Define `PipelineConfig`, `StageType`, `SkipCondition`, `QualityGateConfig` structs
- Implement TOML serialization/deserialization
- Add loading logic with 3-tier precedence
- Add validation methods (errors, warnings, dependency checking)
- **Tests**: 10-12 unit tests

**Files**:
- `codex-rs/tui/src/chatwidget/spec_kit/pipeline_config.rs` (~250-300 LOC)
- `codex-rs/tui/tests/pipeline_config_tests.rs` (~180-220 LOC)

**Test Coverage**:
- Config parsing (valid, invalid TOML)
- Precedence (CLI > per-SPEC > global > defaults)
- Dependency validation (hard deps, soft deps)
- Skip condition evaluation

---

### Phase 2: Pipeline Execution Logic (8-10 hours)

**Tasks**:
- Modify `pipeline_coordinator.rs` to load `PipelineConfig`
- Add stage filtering based on `enabled_stages`
- Implement skip logging and telemetry
- Add quality gate checkpoint calculation
- Handle artifact discovery for skipped stages
- **Tests**: 8-10 integration tests

**Files**:
- `codex-rs/tui/src/chatwidget/spec_kit/pipeline_coordinator.rs` (~+100 LOC)
- Integration tests (~200-250 LOC)

**Test Coverage**:
- Full pipeline execution (all stages)
- Partial pipeline (skip validate+audit)
- Dependency validation (error on implement without tasks)
- Quality gate bypass (warning confirmation)
- Artifact discovery (existing tasks.md)

---

### Phase 3: CLI Flag Support (4-6 hours)

**Tasks**:
- Add CLI flag parsing to `/speckit.auto` command
- Implement `PipelineOverrides` struct
- Add `--skip-*`, `--only-*`, `--stages=` flag support
- Update command help text
- **Tests**: 6-8 CLI parsing tests

**Files**:
- `codex-rs/tui/src/chatwidget/spec_kit/commands/auto.rs` (~+50 LOC)
- CLI parsing tests (~120-150 LOC)

---

### Phase 4: Documentation & Examples (2-4 hours)

**Tasks**:
- Create pipeline configuration guide
- Document 4 workflow patterns (prototyping, docs-only, refactoring, debugging)
- Add `pipeline.toml` examples
- Update CLAUDE.md command reference
- **Deliverables**: Configuration guide, workflow examples

**Files**:
- `docs/spec-kit/PIPELINE_CONFIGURATION_GUIDE.md` (~300-400 lines)
- `docs/spec-kit/workflow-examples/` (4 example configs)

---

**Total Implementation Effort**: 20-28 hours (1-1.5 weeks)

---

## Dependencies & Risks

### Dependencies

- **Ratatui**: TUI framework (existing)
- **TOML**: Config parsing (`serde`, `toml` - existing)
- **SPEC-947**: UI configurator widget (parallel development)
- **Existing**: `pipeline_coordinator.rs`, quality gate infrastructure

### Risks

**Risk 1: Artifact Discovery Failures**
- **Issue**: User skips plan but `plan.md` doesn't exist
- **Mitigation**: Pre-execution validation checks for required artifacts
- **Error Message**: "Error: plan.md not found (required for tasks stage). Enable plan stage or create docs/SPEC-947-*/plan.md manually."

**Risk 2: Quality Gate Bypass Reduces Output Quality**
- **Issue**: Skipping plan/tasks bypasses quality checkpoints
- **Mitigation**: Explicit warnings + confirmation dialog (SPEC-947 UI)
- **Data**: Track quality metrics (skipped checkpoints vs defect rate)

**Risk 3: Config Complexity for New Users**
- **Issue**: Too many knobs and dials confuse beginners
- **Mitigation**: Sane defaults (all stages enabled), progressive disclosure
- **Documentation**: Start with simple examples, advanced configs later

---

## Success Criteria

### Research Phase ✅

1. ✅ Stage dependency graph documented (hard vs soft dependencies)
2. ✅ Configuration precedence system designed (3-tier)
3. ✅ Skip logic patterns defined (conditional, always, never)
4. ✅ Quality gate interaction rules specified
5. ✅ Evidence/artifact strategy designed
6. ✅ Four workflow patterns documented with cost/time savings
7. ✅ CLI flag patterns defined

### Implementation Phase (Deferred)

1. `PipelineConfig` loads from per-SPEC > global > defaults with correct precedence
2. `pipeline_coordinator.rs` executes only enabled stages
3. Skipped stages logged with reason to evidence directory
4. Dependency validation prevents invalid configurations
5. Quality gate bypass requires explicit confirmation
6. CLI flags (`--skip-validate`) work correctly
7. Artifact discovery works for skipped stages (error if missing)
8. Documentation includes 4 workflow pattern examples

---

## Next Steps

1. ✅ SPEC-947 (Pipeline UI) research complete
2. ✅ SPEC-948 (Pipeline Logic) research complete ← **YOU ARE HERE**
3. ⏭️ Create SPEC-949 (Extended Model Support) research
4. ⏭️ Store all three research SPECs to local-memory
5. ⏭️ Later: Create implementation SPECs based on research findings

---

## Appendix

### A. Related SPECs

- **SPEC-947**: Pipeline UI Configurator (provides user interface for this logic)
- **SPEC-922**: Auto-commit stage artifacts (integration point for evidence)
- **SPEC-068**: Quality Gates (bypass logic integration)

### B. Configuration Examples

**Example 1: Rapid Prototyping**
```toml
# docs/SPEC-950-prototype/pipeline.toml
[pipeline]
spec_id = "SPEC-950"
enabled_stages = ["new", "specify", "plan", "implement"]

[skip_reasons]
tasks = "Direct implementation from plan"
validate = "Prototype, no tests needed"
audit = "Low-risk experimental feature"
unlock = "Not ready for production"
```

**Example 2: Documentation-Only**
```toml
# docs/SPEC-951-docs-update/pipeline.toml
[pipeline]
spec_id = "SPEC-951"
enabled_stages = ["specify", "plan", "unlock"]

[skip_reasons]
tasks = "No code changes"
implement = "Documentation updates only"
validate = "No tests for docs"
audit = "No security implications"
```

**Example 3: Bug Fix (Minimal Pipeline)**
```toml
# docs/SPEC-952-bugfix/pipeline.toml
[pipeline]
spec_id = "SPEC-952"
enabled_stages = ["implement", "validate", "unlock"]

[skip_reasons]
new = "SPEC already exists"
specify = "Bug already specified in issue"
plan = "Using existing plan from original feature"
tasks = "Using existing tasks"
audit = "Low-risk bug fix"
```

---

**Research SPEC-948 Status**: ✅ **COMPLETE**
**Next**: Create SPEC-949 (Extended Model Support Research)
