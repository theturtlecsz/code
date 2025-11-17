# SPEC-948-IMPL: Modular Pipeline Logic Implementation Plan

**Research SPEC**: SPEC-948 (complete)
**Implementation Sequence**: 2/3 (Second - provides backend for SPEC-947)
**Estimated Duration**: 20-28 hours (1-1.5 weeks)
**Dependencies**: Config infrastructure (exists: config_types.rs)
**Created**: 2025-11-16
**Priority**: P1 - High (Backend Foundation)

---

## Executive Summary

This implementation delivers the core backend logic for modular pipeline stage execution, enabling selective stage filtering, dependency validation, and workflow customization. Creates `pipeline_config.rs` data layer (250-300 LOC) and extends `pipeline_coordinator.rs` with stage filtering logic. Supports 4 primary workflow patterns: rapid prototyping, docs-only, code refactoring, and debugging.

**Cost Baseline Note**: Assumes SPEC-949 GPT-5 migration complete (baseline $2.36). Pre-SPEC-949 baseline was $2.71 (GPT-4).

**Strategic Impact**:
- **Flexibility**: Enable partial workflows (skip expensive stages, iterate faster)
- **Cost Control**: User-driven stage selection ($2.71 → $1.20-2.50 depending on workflow)
- **Quality Awareness**: Dependency validation + quality gate bypass warnings
- **Developer Experience**: CLI flags for quick overrides, TOML configs for repeatability

---

## Implementation Phases

### Phase 1: Config Data Layer (Week 2, Days 1-3, 6-8 hours)

**Objective**: Create `pipeline_config.rs` module with TOML-based configuration, precedence system, and validation logic

**Tasks**:

**Task 1.1**: Create `PipelineConfig` core data structure
- **File**: `codex-rs/tui/src/chatwidget/spec_kit/pipeline_config.rs` (NEW)
- **Action**: Define config schema matching research SPEC design
- **Changes**:
  - `PipelineConfig` struct (fields: spec_id, enabled_stages, quality_gates, stage_models, skip_conditions)
  - `StageType` enum (8 variants: New, Specify, Plan, Tasks, Implement, Validate, Audit, Unlock)
  - `SkipCondition` enum (5 variants: NoTests, LowRisk, FileCountBelow, Always, Never)
  - `QualityGateConfig` struct (fields: enabled, auto_resolve, thresholds)
  - Derive: Debug, Clone, Serialize, Deserialize
- **LOC**: ~80-100 lines (data structures only)
- **Rationale**: Type-safe config representation enables validation, precedence merging
- **Dependencies**: serde, toml crates (already in Cargo.toml)

```rust
// Example structure (lines 1-100):
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

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
            Self::New => 0.0,
            Self::Specify => 0.10,
            Self::Plan => 0.35,
            Self::Tasks => 0.10,
            Self::Implement => 0.11,
            Self::Validate => 0.35,
            Self::Audit => 0.80,
            Self::Unlock => 0.80,
        }
    }

    /// Get time estimate for this stage (minutes)
    pub fn duration_estimate(&self) -> u32 {
        match self {
            Self::New => 1,
            Self::Specify => 4,
            Self::Plan => 11,
            Self::Tasks => 4,
            Self::Implement => 10,
            Self::Validate => 11,
            Self::Audit => 11,
            Self::Unlock => 11,
        }
    }

    /// Does this stage have a quality gate checkpoint?
    pub fn has_quality_gate(&self) -> bool {
        matches!(self, Self::Plan | Self::Tasks)
    }
}

/// Conditional skip rules
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SkipCondition {
    NoTests,
    LowRisk,
    FileCountBelow { pattern: String, count: usize },
    Always,
    Never,
}
```

**Task 1.2**: Implement TOML serialization/deserialization
- **File**: `codex-rs/tui/src/chatwidget/spec_kit/pipeline_config.rs` (continuation)
- **Action**: Add TOML load/save functions
- **Changes**:
  - `load_pipeline_config(spec_id: &str) -> Result<PipelineConfig, String>` (precedence logic)
  - `save_pipeline_config(config: &PipelineConfig, path: &str) -> Result<(), String>`
  - `PipelineConfig::defaults() -> PipelineConfig` (built-in defaults)
  - `PipelineConfig::merge(&mut self, other: PipelineConfig)` (precedence merging)
- **LOC**: ~60-80 lines (I/O + precedence)
- **Rationale**: 3-tier precedence (CLI > per-SPEC > global > defaults) requires explicit merge logic
- **Dependencies**: std::fs, std::path

```rust
// Example load logic (lines 100-160):
impl PipelineConfig {
    /// Load configuration with 3-tier precedence
    pub fn load(spec_id: &str, cli_overrides: Option<PipelineOverrides>) -> Result<Self, String> {
        // Start with built-in defaults
        let mut config = Self::defaults();

        // Layer 1: Global user config (~/.code/config.toml)
        if let Ok(global) = Self::load_global_config() {
            config.merge(global);
        }

        // Layer 2: Per-SPEC config (docs/SPEC-*/pipeline.toml)
        let spec_path = format!("docs/{}/pipeline.toml", spec_id);
        if let Ok(per_spec) = Self::load_file_config(&spec_path) {
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
            quality_gates: QualityGateConfig::defaults(),
            stage_models: HashMap::new(),
            skip_conditions: HashMap::new(),
            created: None,
            modified: None,
        }
    }

    fn merge(&mut self, other: PipelineConfig) {
        // Higher precedence config overwrites lower
        if !other.enabled_stages.is_empty() {
            self.enabled_stages = other.enabled_stages;
        }
        self.stage_models.extend(other.stage_models);
        self.skip_conditions.extend(other.skip_conditions);
        // quality_gates merged field-by-field
    }
}
```

**Task 1.3**: Implement dependency validation
- **File**: `codex-rs/tui/src/chatwidget/spec_kit/pipeline_config.rs` (continuation)
- **Action**: Add validation methods
- **Changes**:
  - `PipelineConfig::validate(&self) -> Result<ValidationResult, String>`
  - `ValidationResult` struct (fields: warnings: Vec<String>)
  - Validation rules: hard dependencies (implement needs tasks), quality gate bypass warnings
- **LOC**: ~60-80 lines (validation logic)
- **Rationale**: Prevent invalid configs (e.g., implement without tasks), warn on quality bypass
- **Dependencies**: None (self-contained)

```rust
// Example validation (lines 200-280):
#[derive(Debug, Clone)]
pub struct ValidationResult {
    pub warnings: Vec<String>,
}

impl PipelineConfig {
    pub fn validate(&self) -> Result<ValidationResult, String> {
        let mut errors = Vec::new();
        let mut warnings = Vec::new();

        // Check hard dependencies
        for stage in &self.enabled_stages {
            for dep in stage.dependencies() {
                if !self.is_enabled(dep) {
                    if is_hard_dependency(*stage, dep) {
                        errors.push(format!(
                            "Error: {} requires {} to be enabled",
                            stage.display_name(),
                            dep.display_name()
                        ));
                    } else {
                        warnings.push(format!(
                            "Warning: {} without {}: will use existing artifacts",
                            stage.display_name(),
                            dep.display_name()
                        ));
                    }
                }
            }
        }

        // Check quality gate bypass
        if !self.is_enabled(StageType::Plan) {
            warnings.push("⚠ Skipping plan disables 2 quality gate checkpoints".into());
        }
        if !self.is_enabled(StageType::Tasks) {
            warnings.push("⚠ Skipping tasks disables 1 quality gate checkpoint".into());
        }

        // Check cost implications
        let total_cost: f64 = self.enabled_stages.iter()
            .map(|s| s.cost_estimate())
            .sum();
        let full_cost = 2.71;
        if total_cost < full_cost * 0.5 {
            warnings.push(format!(
                "ℹ️ Partial pipeline: ${:.2} vs ${:.2} full (saving ${:.2})",
                total_cost, full_cost, full_cost - total_cost
            ));
        }

        if errors.is_empty() {
            Ok(ValidationResult { warnings })
        } else {
            Err(format!("Configuration has {} error(s):\n{}", errors.len(), errors.join("\n")))
        }
    }

    pub fn is_enabled(&self, stage: StageType) -> bool {
        self.enabled_stages.contains(&stage)
    }
}

fn is_hard_dependency(stage: StageType, dep: StageType) -> bool {
    match (stage, dep) {
        (StageType::Tasks, StageType::Plan) => true,
        (StageType::Implement, StageType::Tasks) => true,
        _ => false,
    }
}
```

**Deliverables**:
- `pipeline_config.rs` created (~250-300 LOC total)
- Tests: 10-12 unit tests (parsing, precedence, validation, defaults)

**Validation**:
```bash
# Compilation check
cd codex-rs && cargo build -p codex-tui

# Unit tests
cargo test -p codex-tui pipeline_config::tests
```

**Success Criteria**:
- All data structures compile without errors
- TOML serialization/deserialization works (valid + invalid cases)
- Precedence merging correct (CLI > per-SPEC > global > defaults)
- Dependency validation catches invalid configs
- 10-12 unit tests passing

**Milestone 1**: Config data layer complete, ready for pipeline integration

---

### Phase 2: Pipeline Execution Logic (Week 2, Days 4-6, 8-10 hours)

**Objective**: Modify `pipeline_coordinator.rs` to load config, filter stages, skip with logging

**Tasks**:

**Task 2.1**: Extend `handle_spec_auto` to load PipelineConfig
- **File**: `codex-rs/tui/src/chatwidget/spec_kit/pipeline_coordinator.rs`
- **Action**: Add config loading before pipeline execution
- **Changes**:
  - Import `pipeline_config::*`
  - Call `PipelineConfig::load(spec_id, cli_overrides)` at function start
  - Store config in SpecAutoState or pass through execution
  - Handle validation errors (display to user, halt pipeline)
- **LOC**: ~+20 lines (config load + error handling)
- **Rationale**: Config determines which stages execute
- **Dependencies**: Phase 1 (pipeline_config.rs exists)

```rust
// Example modification (lines 27-50):
pub fn handle_spec_auto(
    widget: &mut ChatWidget,
    spec_id: String,
    goal: String,
    resume_from: SpecStage,
    hal_mode: Option<HalMode>,
    cli_overrides: Option<PipelineOverrides>,  // NEW parameter
) {
    // ... existing header display ...

    // NEW: Load pipeline configuration
    let pipeline_config = match PipelineConfig::load(&spec_id, cli_overrides) {
        Ok(config) => config,
        Err(err) => {
            widget.history_push(crate::history_cell::new_error_event(format!(
                "Pipeline configuration error: {}",
                err
            )));
            return;
        }
    };

    // Display warnings if any
    if let Ok(validation) = pipeline_config.validate() {
        for warning in &validation.warnings {
            widget.history_push(crate::history_cell::new_warning_event(warning.clone()));
        }
    }

    // ... rest of function (pass pipeline_config through) ...
}
```

**Task 2.2**: Modify stage execution loop for filtering
- **File**: `codex-rs/tui/src/chatwidget/spec_kit/pipeline_coordinator.rs`
- **Action**: Add stage skip logic in execution loop
- **Changes**:
  - Find existing stage iteration loop (likely around `advance_spec_auto` or similar)
  - Add `if pipeline_config.is_enabled(stage)` check
  - If disabled: log skip, record telemetry, skip to next stage
  - If enabled: execute normally (existing path)
- **LOC**: ~+40 lines (skip logic + telemetry)
- **Rationale**: Core filtering mechanism
- **Dependencies**: Task 2.1 (pipeline_config available)

```rust
// Example modification (hypothetical existing loop):
pub async fn execute_pipeline(
    spec_id: &str,
    pipeline_config: &PipelineConfig,
    ctx: &mut dyn SpecKitContext
) -> Result<(), String> {
    let stages = pipeline_config.enabled_stages_in_order();  // NEW: Filter stages

    for stage in stages {
        if pipeline_config.is_enabled(stage) {
            tracing::info!("Executing stage: {:?}", stage);

            // Execute stage normally (EXISTING CODE)
            execute_stage(stage, spec_id, ctx).await?;

            // Auto-commit artifacts (EXISTING CODE - SPEC-922)
            if std::env::var("SPEC_KIT_AUTO_COMMIT").unwrap_or("true".into()) == "true" {
                auto_commit_stage_artifacts(spec_id, stage).await?;
            }
        } else {
            // NEW: Log skip with reason
            let reason = pipeline_config.skip_reason(stage)
                .unwrap_or("Disabled in pipeline config");

            tracing::info!("Skipping stage {:?}: {}", stage, reason);

            // NEW: Record skip in telemetry
            record_stage_skip(spec_id, stage, reason).await?;

            // Do NOT create artifacts for skipped stages
        }
    }

    Ok(())
}
```

**Task 2.3**: Implement skip telemetry recording
- **File**: `codex-rs/tui/src/chatwidget/spec_kit/pipeline_coordinator.rs` OR new `skip_telemetry.rs`
- **Action**: Add `record_stage_skip` function
- **Changes**:
  - Create skip metadata JSON (command, specId, stage, action="skipped", reason, timestamp)
  - Write to evidence directory: `docs/SPEC-OPS-004-.../SPEC-{id}/speckit-{stage}_SKIPPED.json`
  - Schema version 1.0 (compatible with existing telemetry)
- **LOC**: ~+30 lines (telemetry function)
- **Rationale**: Evidence tracking for skipped stages (audit trail)
- **Dependencies**: Task 2.2 (skip logic calls this)

```rust
// Example skip telemetry (new function):
async fn record_stage_skip(
    spec_id: &str,
    stage: StageType,
    reason: &str
) -> Result<(), String> {
    let skip_metadata = serde_json::json!({
        "command": format!("speckit.{}", stage.display_name().to_lowercase()),
        "specId": spec_id,
        "stage": stage.display_name(),
        "action": "skipped",
        "reason": reason,
        "configSource": "pipeline.toml",  // Or detect source
        "timestamp": chrono::Utc::now().to_rfc3339(),
        "schemaVersion": "1.0"
    });

    let evidence_dir = format!("docs/SPEC-OPS-004-integrated-coder-hooks/evidence/commands/{}", spec_id);
    std::fs::create_dir_all(&evidence_dir)?;

    let skip_file = format!("{}/speckit-{}_SKIPPED.json", evidence_dir, stage.display_name().to_lowercase());
    std::fs::write(skip_file, serde_json::to_string_pretty(&skip_metadata)?)?;

    Ok(())
}
```

**Task 2.4**: Add quality gate checkpoint calculation
- **File**: `codex-rs/tui/src/chatwidget/spec_kit/quality_gate_handler.rs` (likely)
- **Action**: Calculate active quality gates based on enabled stages
- **Changes**:
  - Add `active_quality_gates(config: &PipelineConfig) -> Vec<QualityCheckpoint>`
  - Logic: Include checkpoint only if corresponding stage enabled
  - Pre-planning: if specify + plan enabled
  - Post-plan: if plan enabled
  - Post-tasks: if tasks enabled
- **LOC**: ~+20 lines (checkpoint calculation)
- **Rationale**: Quality gate bypass awareness
- **Dependencies**: Task 2.1 (pipeline_config available)

**Deliverables**:
- `pipeline_coordinator.rs` modified (~+100 LOC)
- Skip telemetry function (~+30 LOC)
- Quality gate calculation (~+20 LOC)
- Tests: 8-10 integration tests (full pipeline, partial pipeline, skip telemetry, quality gates)

**Validation**:
```bash
# Integration tests
cargo test -p codex-tui spec_kit::pipeline::tests

# Manual test: Skip validate+audit
# (in TUI, manually modify pipeline.toml or use CLI flags once Phase 3 done)
```

**Success Criteria**:
- Config loads at pipeline start (valid + invalid handled)
- Stages filter correctly (enabled execute, disabled skip)
- Skip telemetry written to evidence directory
- Quality gate checkpoints calculated based on enabled stages
- 8-10 integration tests passing

**Milestone 2**: Backend execution logic complete, ready for CLI integration

---

### Phase 3: CLI Flag Support (Week 3, Days 1-2, 4-6 hours)

**Objective**: Add CLI flag parsing for `--skip-*` and `--stages=` overrides

**Tasks**:

**Task 3.1**: Define `PipelineOverrides` struct
- **File**: `codex-rs/tui/src/chatwidget/spec_kit/pipeline_config.rs` (extend existing module)
- **Action**: Add CLI override data structure
- **Changes**:
  - `PipelineOverrides` struct (fields: skip_stages, only_stages, configure_mode)
  - `from_cli_args(args: &[String]) -> Self` (parse flags)
  - Support: `--skip-validate`, `--skip-audit`, `--stages=plan,tasks,implement`, `--configure`
- **LOC**: ~+40 lines (struct + parsing)
- **Rationale**: CLI provides quick overrides without editing TOML
- **Dependencies**: None (extends Phase 1 module)

```rust
// Example CLI override struct (add to pipeline_config.rs):
#[derive(Debug, Clone, Default)]
pub struct PipelineOverrides {
    pub skip_stages: Vec<StageType>,
    pub only_stages: Option<Vec<StageType>>,
    pub configure_mode: bool,
}

impl PipelineOverrides {
    pub fn from_cli_args(args: &[String]) -> Self {
        let mut overrides = Self::default();

        for arg in args {
            if let Some(stage_name) = arg.strip_prefix("--skip-") {
                if let Ok(stage) = StageType::from_str(stage_name) {
                    overrides.skip_stages.push(stage);
                }
            } else if let Some(stages_str) = arg.strip_prefix("--stages=") {
                let stages: Vec<StageType> = stages_str
                    .split(',')
                    .filter_map(|s| StageType::from_str(s.trim()).ok())
                    .collect();
                overrides.only_stages = Some(stages);
            } else if arg == "--configure" {
                overrides.configure_mode = true;
            }
        }

        overrides
    }
}

impl PipelineConfig {
    pub fn apply_overrides(&mut self, overrides: PipelineOverrides) {
        // --skip-* flags
        for skip_stage in &overrides.skip_stages {
            self.enabled_stages.retain(|s| s != skip_stage);
        }

        // --stages=... flag (replaces enabled_stages entirely)
        if let Some(only_stages) = overrides.only_stages {
            self.enabled_stages = only_stages;
        }

        // --configure flag handled separately (launches TUI modal)
    }
}
```

**Task 3.2**: Update `/speckit.auto` command handler
- **File**: `codex-rs/tui/src/chatwidget/spec_kit/pipeline_coordinator.rs` (handle_spec_auto function starting at line 29)
- **Action**: Parse CLI args and pass to `handle_spec_auto`
- **Changes**:
  - Parse command string for flags: `/speckit.auto SPEC-947 --skip-validate --skip-audit`
  - Create `PipelineOverrides` from parsed args
  - Pass overrides to `handle_spec_auto` (Task 2.1 added parameter)
- **LOC**: ~+30 lines (command parsing)
- **Rationale**: Wire CLI flags into execution path
- **Dependencies**: Task 3.1 (PipelineOverrides exists), Phase 2 Task 2.1 (handle_spec_auto accepts overrides)

**Task 3.3**: Update command help text
- **File**: Command documentation or help strings (wherever `/speckit.auto` help is defined)
- **Action**: Document new CLI flags
- **Changes**:
  - Add to help: `--skip-{stage}` skips individual stages
  - Add to help: `--stages=plan,tasks,implement` runs only listed stages
  - Add examples: `/speckit.auto SPEC-947 --skip-validate`
- **LOC**: ~+10 lines (documentation)
- **Rationale**: User discoverability
- **Dependencies**: None (documentation only)

**Deliverables**:
- `PipelineOverrides` struct (~+40 LOC)
- CLI parsing in handler (~+30 LOC)
- Help text updated (~+10 LOC)
- Tests: 6-8 CLI parsing tests (flag parsing, invalid flags, precedence)

**Validation**:
```bash
# CLI parsing tests
cargo test -p codex-tui spec_kit::cli_parsing::tests

# Manual test (in TUI):
/speckit.auto SPEC-947 --skip-validate --skip-audit
# Should skip validate and audit stages
```

**Success Criteria**:
- CLI flags parsed correctly (--skip-*, --stages=)
- Overrides apply with correct precedence (CLI highest)
- Invalid flags handled gracefully (warning or error)
- Help text documents new flags
- 6-8 CLI parsing tests passing

**Milestone 3**: CLI integration complete, full feature operational

---

### Phase 4: Documentation & Examples (Week 3, Days 3-4, 2-4 hours)

**Objective**: Create configuration guide, workflow examples, migration documentation

**Tasks**:

**Task 4.1**: Write pipeline configuration guide
- **File**: `docs/spec-kit/PIPELINE_CONFIGURATION_GUIDE.md` (NEW)
- **Action**: Comprehensive config documentation (300-400 lines)
- **Content**:
  - Section 1: Overview (3-tier precedence, when to use per-SPEC vs global)
  - Section 2: Config schema (pipeline.toml structure, all fields explained)
  - Section 3: CLI flags (all supported flags, examples)
  - Section 4: Dependency rules (hard vs soft, validation errors)
  - Section 5: Quality gate interaction (bypass warnings, checkpoint calculation)
  - Section 6: Troubleshooting (common issues, validation errors)
- **LOC**: ~300-400 lines markdown
- **Rationale**: Users need clear guide for feature
- **Dependencies**: Phases 1-3 complete (all features implemented)

**Task 4.2**: Create workflow pattern examples
- **File**: `docs/spec-kit/workflow-examples/` directory with 4 example configs
- **Action**: Write 4 example `pipeline.toml` files
- **Examples**:
  1. `rapid-prototyping.toml` (skip validate, audit, unlock)
  2. `docs-only.toml` (specify, plan, unlock only)
  3. `code-refactoring.toml` (implement, validate, unlock only)
  4. `debug-single-stage.toml` (plan only)
- **LOC**: ~40 lines each = ~160 lines total TOML
- **Rationale**: Concrete examples help users adopt feature
- **Dependencies**: Task 4.1 (guide references examples)

```toml
# Example: docs/spec-kit/workflow-examples/rapid-prototyping.toml
[pipeline]
spec_id = "SPEC-XXX"
enabled_stages = ["new", "specify", "plan", "tasks", "implement"]

[skip_reasons]
validate = "Prototype, no tests needed yet"
audit = "Low-risk experimental feature"
unlock = "Not ready for production"

# Cost: ~$0.66 (vs $2.71 full pipeline, 76% savings)
# Time: ~20 min (vs ~50 min, 60% savings)
```

**Task 4.3**: Document workflow patterns in guide
- **File**: `docs/spec-kit/PIPELINE_CONFIGURATION_GUIDE.md` (continuation)
- **Action**: Add "Common Workflows" section
- **Content**:
  - For each of 4 patterns: Use case, stages enabled, cost/time savings, example
  - Decision matrix: Which workflow for which scenario?
  - Cost comparison table (all patterns vs full pipeline)
- **LOC**: ~100 lines (part of Task 4.1's 300-400 total)
- **Rationale**: Help users choose appropriate workflow
- **Dependencies**: Task 4.2 (examples exist to reference)

**Task 4.4**: Update CLAUDE.md command reference
- **File**: `CLAUDE.md` (project root)
- **Action**: Document new CLI flags for `/speckit.auto`
- **Changes**:
  - Add examples: `/speckit.auto SPEC-ID --skip-validate`
  - Add note: "See docs/spec-kit/PIPELINE_CONFIGURATION_GUIDE.md for details"
  - Update command syntax documentation
- **LOC**: ~+20 lines (command examples)
- **Rationale**: Central documentation hub
- **Dependencies**: Task 4.1 (guide exists to reference)

**Deliverables**:
- `PIPELINE_CONFIGURATION_GUIDE.md` (~300-400 lines)
- 4 workflow example configs (~160 lines total TOML)
- `CLAUDE.md` updated (~+20 lines)

**Validation**:
```bash
# Review documentation
cat docs/spec-kit/PIPELINE_CONFIGURATION_GUIDE.md

# Test example configs
cp docs/spec-kit/workflow-examples/rapid-prototyping.toml docs/SPEC-XXX/pipeline.toml
/speckit.auto SPEC-XXX  # Should skip validate, audit, unlock
```

**Success Criteria**:
- Configuration guide complete and peer-reviewed
- 4 workflow examples documented with cost/time estimates
- CLAUDE.md reflects new CLI flags
- Examples validated (copy to test SPEC, execute successfully)

**Milestone 4**: Documentation complete, feature ready for user adoption

---

## Complete File Manifest

### New Files (SPEC-948-IMPL)

| File Path | Purpose | LOC | Tests | Phase |
|-----------|---------|-----|-------|-------|
| `codex-rs/tui/src/chatwidget/spec_kit/pipeline_config.rs` | Config data layer | 250-300 | 10-12 unit | Phase 1 |
| `docs/spec-kit/PIPELINE_CONFIGURATION_GUIDE.md` | User guide | 300-400 | N/A | Phase 4 |
| `docs/spec-kit/workflow-examples/rapid-prototyping.toml` | Workflow example | ~40 | N/A | Phase 4 |
| `docs/spec-kit/workflow-examples/docs-only.toml` | Workflow example | ~40 | N/A | Phase 4 |
| `docs/spec-kit/workflow-examples/code-refactoring.toml` | Workflow example | ~40 | N/A | Phase 4 |
| `docs/spec-kit/workflow-examples/debug-single-stage.toml` | Workflow example | ~40 | N/A | Phase 4 |

**Total New**: 6 files, ~670-860 LOC (260-300 Rust + 410-560 docs/examples)

### Modified Files (SPEC-948-IMPL)

| File Path | Changes | LOC Change | Rationale | Risk | Phase |
|-----------|---------|------------|-----------|------|-------|
| `codex-rs/tui/src/chatwidget/spec_kit/pipeline_coordinator.rs` | Load config, filter stages, skip logic | +100/-10 | Core execution | Medium | Phase 2 |
| `codex-rs/tui/src/chatwidget/spec_kit/handler.rs` | Parse CLI flags | +30/-5 | CLI integration | Low | Phase 3 |
| `codex-rs/tui/src/chatwidget/spec_kit/quality_gate_handler.rs` | Calculate checkpoints | +20/-0 | Quality gates | Low | Phase 2 |
| `CLAUDE.md` | Document CLI flags | +20/-0 | Documentation | None | Phase 4 |

**Total Modified**: 4 files, ~+170/-15 LOC net

---

## Test Coverage Plan

### Unit Test Matrix

| Module | Coverage Target | Test Count | Key Scenarios |
|--------|-----------------|------------|---------------|
| pipeline_config | 80%+ | 10-12 | TOML parsing, precedence, validation, defaults, merge logic |
| cli_parsing | 70%+ | 6-8 | Flag parsing, invalid flags, precedence (CLI > TOML) |

**Total Unit Tests**: 16-20 tests (~220-280 lines)

### Integration Test Scenarios

1. **Full Pipeline Execution (All Stages)**:
   - Given: Default config (all stages enabled)
   - When: Execute /speckit.auto SPEC-XXX
   - Then: All 8 stages execute sequentially
   - Validates: Phase 2 baseline (no skipping)

2. **Partial Pipeline (Skip Validate+Audit)**:
   - Given: pipeline.toml with validate, audit disabled
   - When: Execute /speckit.auto SPEC-XXX
   - Then: 6 stages execute, validate+audit skipped, skip telemetry written
   - Validates: Phase 2 Task 2.2 (stage filtering)

3. **Dependency Validation Error (Implement Without Tasks)**:
   - Given: pipeline.toml with enabled_stages = ["implement"]
   - When: Load config
   - Then: Validation error "implement requires tasks to be enabled"
   - Validates: Phase 1 Task 1.3 (hard dependency check)

4. **Quality Gate Bypass Warning**:
   - Given: pipeline.toml with plan, tasks disabled
   - When: Load config, display warnings
   - Then: Warning "⚠ Skipping plan disables 2 quality gate checkpoints"
   - Validates: Phase 2 Task 2.4 (checkpoint calculation)

5. **CLI Flag Override (Skip Validate)**:
   - Given: Default config (all stages enabled)
   - When: Execute /speckit.auto SPEC-XXX --skip-validate
   - Then: Validate skipped, CLI override recorded in telemetry
   - Validates: Phase 3 Task 3.2 (CLI precedence)

6. **Precedence Merging (CLI > Per-SPEC > Global)**:
   - Given: Global config (all stages), per-SPEC (skip audit), CLI (--skip-validate)
   - When: Load config
   - Then: enabled_stages excludes audit (per-SPEC) and validate (CLI)
   - Validates: Phase 1 Task 1.2 (precedence merge)

7. **Artifact Discovery (Skipped Plan, Existing plan.md)**:
   - Given: pipeline.toml (plan disabled), docs/SPEC-XXX/plan.md exists
   - When: Execute tasks stage
   - Then: Uses existing plan.md artifact, no error
   - Validates: Phase 2 skip logic + artifact discovery

8. **Workflow Pattern (Rapid Prototyping)**:
   - Given: rapid-prototyping.toml (specify, plan, tasks, implement only)
   - When: Execute /speckit.auto SPEC-XXX
   - Then: 4 stages execute, cost ~$0.66, time ~20 min
   - Validates: End-to-end workflow pattern

**Total Integration Tests**: 8 tests (~250-300 lines)

### Performance Validation Tests

**Metric 1: Pipeline Execution Time (Workflow Comparison)**
- **Description**: Measure total time for different workflows
- **Baseline**: Full pipeline ~45-50 minutes (all 8 stages)
- **Targets**:
  - Rapid prototyping: ~20 minutes (60% savings)
  - Docs-only: ~15 minutes (70% savings)
  - Code refactoring: ~25 minutes (50% savings)
- **Measurement Method**: SPEC-940 timing infrastructure, measure_time! macro
- **Validation**: Run each workflow n≥3 times, mean duration
- **Success Threshold**: Within ±10% of target

**Metric 2: Config Load Latency**
- **Description**: Time to load and validate pipeline config
- **Baseline**: N/A (new feature)
- **Target**: <50ms (negligible overhead)
- **Measurement Method**: Measure PipelineConfig::load() duration
- **Validation**: Average over n≥100 loads
- **Success Threshold**: <100ms (acceptable user experience)

**Metric 3: Skip Telemetry Overhead**
- **Description**: Time added by skip telemetry recording
- **Baseline**: 0ms (no skipping in current system)
- **Target**: <10ms per skipped stage
- **Measurement Method**: Measure record_stage_skip() duration
- **Validation**: Average over n≥50 skips
- **Success Threshold**: <20ms (acceptable)

---

## Migration & Rollback Plan

### Incremental Deployment

**Phase 1 Complete → Deploy**:
- Merge: pipeline_config.rs module
- Validate: Unit tests pass (10-12 tests)
- Checkpoint: Config parsing works, validation logic correct
- No user impact: Module exists but not used yet

**Phase 2 Complete → Deploy**:
- Merge: pipeline_coordinator.rs modifications
- Validate: Integration tests pass (8 tests)
- Checkpoint: Stage filtering works, skip telemetry written
- User Testing: Create test pipeline.toml, run /speckit.auto
- Rollback Trigger: If pipeline execution fails, revert Phase 2 commit

**Phase 3 Complete → Deploy**:
- Merge: CLI flag parsing
- Validate: CLI tests pass (6-8 tests)
- Checkpoint: Flags parsed correctly, override precedence works
- User Testing: Run /speckit.auto --skip-validate on test SPEC
- Rollback Trigger: If CLI parsing breaks existing commands, revert Phase 3

**Phase 4 Complete → Production**:
- Merge: Documentation and examples
- Validate: Peer review of guide, test all 4 example workflows
- Checkpoint: Examples work as documented
- Production Release: Feature complete, announce to users

### Backward Compatibility

**Preserved**:
- All existing /speckit.* commands work unchanged (default config = all stages)
- No command syntax changes (flags are optional)
- Existing SPECs continue to execute fully (no pipeline.toml = defaults used)
- Evidence directory structure unchanged (skip telemetry is additive)

**Optional**:
- Pipeline configuration opt-in via pipeline.toml creation
- CLI flags opt-in on per-execution basis
- Default behavior: Execute all stages (existing behavior)

**Breaking Changes**: None (fully backward compatible)

### Rollback Strategy

**Rollback Trigger: Config validation too strict**
- **Condition**: Users report valid configs rejected
- **Action**:
  1. Identify validation rule causing false positives
  2. Patch validation logic (e.g., soften hard dependency check)
  3. Redeploy patched version
- **Recovery Time**: <2 hours (targeted fix)

**Rollback Trigger: Stage filtering breaks pipeline**
- **Condition**: Stages execute out of order or skip incorrectly
- **Action**:
  1. Revert pipeline_coordinator.rs changes (Phase 2 rollback)
  2. Keep pipeline_config.rs module (no harm)
  3. Investigate filtering bug (unit test reproduction)
- **Recovery Time**: <1 hour (git revert) + bug fix time

**Rollback Trigger: CLI flag parsing conflicts**
- **Condition**: Flags interfere with existing command parsing
- **Action**:
  1. Revert handler.rs CLI parsing (Phase 3 rollback)
  2. Keep pipeline_config.rs + coordinator changes (TOML-only mode)
  3. Fix CLI parser conflicts
- **Recovery Time**: <1 hour (git revert) + parser fix

**Rollback Procedure**:
```bash
# 1. Identify problematic phase
git log --oneline --grep="SPEC-948" | head -10

# 2. Revert specific phase commit
git revert <phase-commit-hash>

# 3. Rebuild
cd codex-rs && cargo build --workspace

# 4. Test rollback
/speckit.auto SPEC-900  # Should work with reverted code

# 5. Document rollback
echo "Rollback: $(date) - Phase X - Reason: <issue>" >> docs/SPEC-948-.../evidence/rollback.log
```

---

## Timeline & Milestones

### Week 2 (Days 1-7, 18-24 hours total)

**Days 1-3 (Phase 1: Config Data Layer, 6-8h)**:
- Mon AM: Create PipelineConfig data structures (80-100 LOC)
- Mon PM: Implement TOML serialization, load/save logic (60-80 LOC)
- Tue AM: Add dependency validation (60-80 LOC)
- Tue PM: Write 10-12 unit tests
- Wed AM: Run tests, fix validation logic edge cases
- Wed PM: Code review, commit Phase 1
- **Deliverable**: pipeline_config.rs module (~250-300 LOC), tests passing
- **Validation**: `cargo test -p codex-tui pipeline_config`

**Days 4-6 (Phase 2: Pipeline Execution Logic, 8-10h)**:
- Thu AM: Extend handle_spec_auto for config loading (+20 LOC)
- Thu PM: Modify stage execution loop for filtering (+40 LOC)
- Fri AM: Implement skip telemetry recording (+30 LOC)
- Fri PM: Add quality gate checkpoint calculation (+20 LOC)
- Sat AM: Write 8-10 integration tests
- Sat PM: Run tests, manual validation with test SPEC
- Sun: Code review, commit Phase 2
- **Deliverable**: pipeline_coordinator.rs modified (+100 LOC), tests passing
- **Validation**: `/speckit.auto` with custom pipeline.toml

**Milestone 1 (End of Week 2)**: Backend logic complete, CLI integration next

---

### Week 3 (Days 1-4, 6-10 hours)

**Days 1-2 (Phase 3: CLI Flag Support, 4-6h)**:
- Mon AM: Define PipelineOverrides struct, CLI parsing (+40 LOC)
- Mon PM: Update /speckit.auto handler for flags (+30 LOC)
- Tue AM: Update help text, write 6-8 CLI tests
- Tue PM: Run tests, manual CLI flag testing
- **Deliverable**: CLI flags working, tests passing
- **Validation**: `/speckit.auto SPEC-XXX --skip-validate`

**Days 3-4 (Phase 4: Documentation, 2-4h)**:
- Wed AM: Write PIPELINE_CONFIGURATION_GUIDE.md (300-400 lines)
- Wed PM: Create 4 workflow example configs (~160 lines total)
- Thu AM: Update CLAUDE.md, peer review all docs
- Thu PM: Test all 4 example workflows, commit Phase 4
- **Deliverable**: Complete documentation suite
- **Validation**: Peer review + example workflow testing

**Milestone 2 (End of Week 3)**: SPEC-948 implementation complete, SPEC-947 unblocked

---

## Risk Assessment & Mitigation

### Technical Risks

**Risk 1: Config Precedence Complexity**
- **Severity**: Medium
- **Probability**: Medium (3-tier merge logic is non-trivial)
- **Impact**: Wrong config applied, stages skip incorrectly
- **Mitigation**:
  - Comprehensive unit tests for precedence (6+ scenarios)
  - Clear precedence rules in documentation
  - Validation warning if config source ambiguous
- **Contingency**: Simplify to 2-tier (per-SPEC > defaults, remove global)

**Risk 2: Dependency Validation Too Restrictive**
- **Severity**: Medium
- **Probability**: Low (hard dependencies well-defined)
- **Impact**: Users blocked from valid workflows
- **Mitigation**:
  - Soft dependencies generate warnings, not errors
  - Allow artifact discovery (existing files bypass dependency)
  - User can override with skip_conditions: Never
- **Contingency**: Patch validation to be more permissive

**Risk 3: TOML Parsing Errors**
- **Severity**: Low
- **Probability**: Medium (user-created TOML has typos)
- **Impact**: Pipeline fails to start, confusing error
- **Mitigation**:
  - Comprehensive error messages (line numbers, field names)
  - Validation before execution (catch at load time)
  - Fall back to defaults if TOML invalid (with warning)
- **Contingency**: Add TOML syntax validator tool

### Integration Risks

**Risk 1: SPEC-947 Integration Misalignment**
- **Severity**: High
- **Probability**: Low (SPEC-947 designed around this API)
- **Impact**: TUI configurator can't use pipeline_config.rs
- **Mitigation**:
  - SPEC-948 implements API contract from SPEC-947 research
  - Integration tests validate config round-trip (load → modify → save)
  - Parallel development review (SPEC-947 author reviews SPEC-948 API)
- **Contingency**: API modification if SPEC-947 needs different interface

**Risk 2: Evidence Footprint Growth**
- **Severity**: Low
- **Probability**: High (skip telemetry adds JSON files)
- **Impact**: Evidence directory grows, soft limit exceeded
- **Mitigation**:
  - Skip metadata is minimal (~100 bytes vs ~5KB for full telemetry)
  - Existing evidence policy (25MB soft limit) still applies
  - Archive old evidence with scripts/spec_ops_004/evidence_archive.sh
- **Contingency**: Disable skip telemetry if footprint issue (flag to opt out)

**Risk 3: Quality Gate Bypass Reduces Output Quality**
- **Severity**: Medium
- **Probability**: Medium (users will skip to save time)
- **Impact**: More bugs, lower quality specs
- **Mitigation**:
  - Warnings displayed prominently (⚠ emoji, multi-line)
  - Confirmation dialog in SPEC-947 UI (explicit user consent)
  - Track quality metrics (skipped gates vs defect rate) post-deployment
- **Contingency**: Make quality gates un-skippable for certain SPEC priorities

---

## Success Criteria

### Phase-Level Criteria

**Phase 1 Success**:
1. PipelineConfig data structures compile without errors
2. TOML parsing works (valid + invalid files handled)
3. Precedence merging correct (CLI > per-SPEC > global > defaults)
4. Dependency validation catches invalid configs (hard dependencies enforced)
5. 10-12 unit tests passing (100% pass rate)

**Phase 2 Success**:
1. Config loads at pipeline start without errors
2. Stages filter correctly (enabled execute, disabled skip)
3. Skip telemetry written to evidence directory (JSON schema v1.0)
4. Quality gate checkpoints calculated based on enabled stages
5. 8-10 integration tests passing
6. Manual test: Run pipeline with custom pipeline.toml successfully

**Phase 3 Success**:
1. CLI flags parsed correctly (--skip-*, --stages=)
2. Overrides apply with highest precedence
3. Invalid flags handled gracefully (warning or ignored)
4. Help text documents new flags
5. 6-8 CLI parsing tests passing

**Phase 4 Success**:
1. Configuration guide complete (300-400 lines, peer-reviewed)
2. 4 workflow examples documented with accurate cost/time estimates
3. CLAUDE.md updated with CLI flag examples
4. All example workflows tested successfully

### Overall SPEC Criteria

1. **Phases Complete**: All 4 phases 100% complete
2. **Tests Passing**: 100% pass rate maintained (604+ existing + 24-30 new = 628-634 total)
3. **Backward Compatible**: Existing SPECs execute unchanged (default config = all stages)
4. **Documentation Complete**: Guide + 4 examples + CLAUDE.md update
5. **No Regressions**: All existing /speckit.* commands work
6. **Evidence Captured**: Skip telemetry schema v1.0, integration test logs
7. **SPEC-947 Unblocked**: pipeline_config.rs API ready for TUI integration

---

## Documentation Requirements

### User-Facing Documentation

1. **Pipeline Configuration Guide** (`docs/spec-kit/PIPELINE_CONFIGURATION_GUIDE.md`):
   - 3-tier precedence explanation
   - Complete config schema reference
   - CLI flag reference (all supported flags)
   - Dependency rules documentation
   - Quality gate interaction
   - 4 workflow patterns with cost/time comparisons
   - Troubleshooting common issues

2. **Workflow Examples** (`docs/spec-kit/workflow-examples/*.toml`):
   - rapid-prototyping.toml (skip validation stages)
   - docs-only.toml (skip code stages)
   - code-refactoring.toml (skip planning stages)
   - debug-single-stage.toml (run one stage)
   - Each with inline comments explaining use case, cost savings

3. **Command Reference Update** (`CLAUDE.md`):
   - Document /speckit.auto CLI flags
   - Add examples: `--skip-validate`, `--stages=plan,tasks`
   - Link to full guide

### Developer Documentation

1. **Inline Code Comments**:
   - pipeline_config.rs: Precedence merging logic explained
   - Dependency validation rules rationale
   - Skip condition evaluation logic

2. **API Documentation** (rustdoc):
   - PipelineConfig public API (load, validate, is_enabled)
   - StageType methods (dependencies, cost_estimate, duration_estimate)
   - PipelineOverrides parsing logic

3. **CHANGELOG Entry**:
   - SPEC-948: Modular Pipeline Logic
   - Added: pipeline_config.rs module (250-300 LOC)
   - Added: CLI flags (--skip-*, --stages=)
   - Changed: pipeline_coordinator.rs supports stage filtering
   - Feature: 4 workflow patterns (prototyping, docs-only, refactoring, debugging)
   - Cost: Customizable workflows ($0.66-$2.71 depending on stages)

---

**SPEC-948-IMPL Status**: Ready for implementation
**Estimated Total Effort**: 20-28 hours (1-1.5 weeks)
**Depends On**: SPEC-949-IMPL (optional - can use existing models)
**Enables**: SPEC-947-IMPL (provides backend logic layer)
