# PRD: SPEC-KIT-902 - Convert Shell Guardrails to Native Rust

**Priority**: P1 (Medium Priority)
**Status**: Draft
**Created**: 2025-10-30
**Template Version**: 1.0

---

## Problem Statement

The current spec-kit guardrail system relies on 6 shell scripts (`scripts/spec_ops_004/*.sh`) that perform validation for each pipeline stage. While functional, this approach has several drawbacks:

1. **Performance Overhead**: Shell spawn adds 100-200ms per stage (600-1200ms cumulative for full pipeline)
2. **Portability Issues**: Shell scripts less maintainable on Windows, require bash dependencies
3. **DRY Violations**: ~60% code duplication across 6 scripts (env setup, HAL handling, telemetry emission)
4. **Testing Complexity**: Integration tests only, no unit tests for individual validation logic
5. **Maintenance Burden**: Bash parsing errors, harder to refactor than type-safe Rust

The `/speckit.status` command proved native Rust implementation is faster and more maintainable - same benefits should apply to guardrails.

---

## Goals

### Primary Goal
Convert all 6 shell guardrail scripts to pure Rust implementations, improving performance by 100-200ms per stage while maintaining identical telemetry schema and validation behavior.

### Secondary Goals
- Achieve 100% cross-platform compatibility (Linux, macOS, Windows)
- Enable unit testing for individual guardrail validation logic
- Eliminate code duplication through shared guardrail infrastructure
- Simplify debugging with type-safe error handling

---

## Requirements

### Functional Requirements

1. **Guardrail Trait System**
   - Define `Guardrail` trait with common interface
   - Implement trait for each stage: Plan, Tasks, Implement, Validate, Audit, Unlock
   - Maintain identical validation logic as current shell scripts
   - Preserve telemetry schema v1 compatibility

2. **Core Validations** (per stage):
   - **Plan**: Baseline check (spec.md exists), policy validation (constitution.md), HAL optional
   - **Tasks**: Task breakdown validation, SPEC.md consistency
   - **Implement**: Code quality checks, build verification, test compilation
   - **Validate**: Test harness execution, coverage analysis
   - **Audit**: Compliance scanning, security review
   - **Unlock**: Final validation, artifact completeness

3. **Telemetry Emission**
   - JSON schema v1 output (identical to current shell scripts)
   - Write to same evidence paths: `docs/SPEC-OPS-004-integrated-coder-hooks/evidence/commands/<SPEC-ID>/`
   - Include all fields: command, specId, sessionId, timestamp, schemaVersion, stage-specific data

4. **Integration Points**
   - Called from `pipeline_coordinator.rs` instead of shell execution
   - Return `GuardrailOutcome` struct with success/failure details
   - HAL integration preserved (optional, controlled by `SPEC_OPS_HAL_SKIP`)

5. **Shared Infrastructure**
   - Common environment setup (SPEC_OPS_CARGO_MANIFEST, etc.)
   - Shared telemetry writer
   - Unified error handling and logging

### Non-Functional Requirements

1. **Performance Targets**
   - 100-200ms faster per guardrail vs shell (600-1200ms cumulative savings)
   - Total pipeline improvement: 75min → 72-74min (2-4% faster)

2. **Compatibility Requirements**
   - Maintain exact same validation behavior (no functional changes)
   - Preserve telemetry schema v1 (backward compatible)
   - Support all platforms: Linux, macOS, Windows

3. **Code Quality**
   - Unit tests for each validation function
   - Integration tests for full guardrail runs
   - Type-safe error handling (no panics)
   - Clear documentation for each validation step

---

## Technical Approach

### Module Structure

```rust
// spec_kit/guardrails/mod.rs
pub mod plan;
pub mod tasks;
pub mod implement;
pub mod validate;
pub mod audit;
pub mod unlock;
pub mod common;  // Shared infrastructure

pub trait Guardrail {
    fn validate(&self, spec_id: &str, cwd: &Path) -> Result<GuardrailOutcome>;
    fn stage_name(&self) -> &'static str;
}

pub struct GuardrailOutcome {
    pub success: bool,
    pub failures: Vec<String>,
    pub telemetry_path: Option<PathBuf>,
    pub hal_summary: Option<HalSummary>,  // Optional HAL validation
}
```

### Example Implementation (Plan Guardrail)

```rust
// spec_kit/guardrails/plan.rs
pub struct PlanGuardrail;

impl Guardrail for PlanGuardrail {
    fn validate(&self, spec_id: &str, cwd: &Path) -> Result<GuardrailOutcome> {
        let mut failures = Vec::new();

        // 1. Baseline check (spec.md exists)
        let spec_path = format!("docs/{}/spec.md", spec_id);
        if !cwd.join(&spec_path).exists() {
            failures.push(format!("Baseline artifact missing: {}", spec_path));
        }

        // 2. Policy validation (constitution.md compliance)
        let constitution_path = cwd.join("memory/constitution.md");
        if !constitution_path.exists() {
            failures.push("Constitution missing: memory/constitution.md".to_string());
        }

        // 3. HAL validation (optional)
        let hal_summary = if !env::var("SPEC_OPS_HAL_SKIP").is_ok() {
            Some(self.run_hal_validation(spec_id, cwd)?)
        } else {
            None
        };

        // 4. Emit telemetry JSON
        let telemetry_path = self.emit_telemetry(spec_id, &failures, &hal_summary)?;

        Ok(GuardrailOutcome {
            success: failures.is_empty(),
            failures,
            telemetry_path: Some(telemetry_path),
            hal_summary,
        })
    }

    fn stage_name(&self) -> &'static str {
        "plan"
    }
}

impl PlanGuardrail {
    fn emit_telemetry(
        &self,
        spec_id: &str,
        failures: &[String],
        hal_summary: &Option<HalSummary>,
    ) -> Result<PathBuf> {
        let telemetry = json!({
            "command": "spec-plan",
            "specId": spec_id,
            "sessionId": Uuid::new_v4().to_string(),
            "timestamp": Utc::now().to_rfc3339(),
            "schemaVersion": "1",
            "baseline": {
                "mode": "check",
                "artifact": format!("docs/{}/spec.md", spec_id),
                "status": if failures.is_empty() { "passed" } else { "failed" }
            },
            "hal": hal_summary,
        });

        let evidence_dir = format!("docs/SPEC-OPS-004-integrated-coder-hooks/evidence/commands/{}", spec_id);
        fs::create_dir_all(&evidence_dir)?;

        let path = PathBuf::from(format!("{}/plan_telemetry_{}.json", evidence_dir, Utc::now().timestamp()));
        fs::write(&path, serde_json::to_string_pretty(&telemetry)?)?;

        Ok(path)
    }
}
```

### Integration with Pipeline

```rust
// pipeline_coordinator.rs (updated)
async fn run_guardrail_for_stage(
    widget: &mut ChatWidget,
    spec_id: &str,
    stage: SpecStage,
) -> Result<GuardrailOutcome> {
    let guardrail: Box<dyn Guardrail> = match stage {
        SpecStage::Plan => Box::new(PlanGuardrail),
        SpecStage::Tasks => Box::new(TasksGuardrail),
        SpecStage::Implement => Box::new(ImplementGuardrail),
        SpecStage::Validate => Box::new(ValidateGuardrail),
        SpecStage::Audit => Box::new(AuditGuardrail),
        SpecStage::Unlock => Box::new(UnlockGuardrail),
    };

    // Native Rust call (no shell spawn)
    guardrail.validate(spec_id, &widget.config.cwd)
}
```

---

## Acceptance Criteria

- [ ] `spec_kit/guardrails/` module created with 6 implementations + common infrastructure
- [ ] `Guardrail` trait defined with `validate()` and `stage_name()` methods
- [ ] All 6 guardrails ported: Plan, Tasks, Implement, Validate, Audit, Unlock
- [ ] Telemetry schema v1 preserved (JSON output identical to shell scripts)
- [ ] Evidence paths unchanged (`docs/SPEC-OPS-004-integrated-coder-hooks/evidence/`)
- [ ] Performance benchmarks show 100-200ms improvement per stage
- [ ] Unit tests for each validation function (>80% coverage)
- [ ] Integration tests verify full guardrail runs match shell behavior
- [ ] HAL integration preserved (optional, controlled by env var)
- [ ] Shell scripts deprecated (marked in comments, optional to remove)
- [ ] `pipeline_coordinator.rs` updated to call native guardrails
- [ ] Documentation updated (`CLAUDE.md`, guardrail docs)

---

## Out of Scope

- **Validation logic changes**: This SPEC only ports existing behavior, doesn't enhance it
- **Telemetry schema updates**: Maintains schema v1 compatibility, no new fields
- **HAL enhancement**: Preserves existing HAL integration, doesn't improve it
- **Shell script removal**: Scripts can remain as reference, removal optional

---

## Success Metrics

1. **Performance**: 100-200ms faster per guardrail (measured via benchmarks)
2. **Test Coverage**: >80% unit test coverage for guardrail modules
3. **Cross-Platform**: All guardrails pass on Linux, macOS, Windows
4. **Behavioral Parity**: Integration tests confirm identical behavior to shell scripts

---

## Dependencies

### Prerequisites
- Evidence cleanup (ARCH-013 recommended but not required)
- Stable pipeline coordinator (current state sufficient)

### Downstream Dependencies
- Future guardrail enhancements will benefit from native implementation
- Performance improvements enable faster CI/CD pipelines

---

## Estimated Effort

**1 week** (as per architecture review)

**Breakdown**:
- Trait design + common infrastructure: 1 day
- Port 6 guardrails (1 per day): 6 days
  - Plan: 1 day
  - Tasks: 1 day
  - Implement: 1 day
  - Validate: 1 day
  - Audit: 1 day
  - Unlock: 1 day

---

## Priority

**P1 (Medium Priority)** - Performance improvement + maintainability win, fits within 60-day action window. Recommended before upstream sync to simplify merge conflicts.

---

## Related Documents

- Architecture Review: Section "60-Day Actions, Task 5"
- `scripts/spec_ops_004/*.sh` - Current shell guardrails (reference for porting)
- `pipeline_coordinator.rs` - Integration point
- SPEC-KIT-069: Native `/speckit.status` command (proof of concept for native approach)
