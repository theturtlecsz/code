# Integration Analysis: Modular Pipeline & Extended Models

**Analysis Date**: 2025-11-16
**SPECs Covered**: SPEC-947, SPEC-948, SPEC-949
**Total Effort**: 60-84 hours (3-4 weeks)
**Implementation Sequence**: 949 → 948 → 947

---

## Executive Summary

This integration analysis provides a comprehensive implementation roadmap for three interconnected SPECs that together deliver modular pipeline configuration and extended model support to the spec-kit automation framework. The features enable:

1. **Extended Model Support** (SPEC-949): GPT-5/5.1 family integration with Deepseek/Kimi stubs (-13% cost reduction)
2. **Modular Pipeline Logic** (SPEC-948): Backend stage filtering with CLI flags and TOML configs
3. **Pipeline UI Configurator** (SPEC-947): Interactive TUI modal for visual stage selection

**Strategic Impact**:
- **Cost Optimization**: $2.71 → $1.20-2.50 depending on workflow (-56% to -8%)
- **Performance Improvement**: 2-3× faster on simple stages (GPT-5.1 adaptive reasoning)
- **User Experience**: Visual configurator vs manual TOML editing
- **Workflow Flexibility**: 4 primary patterns (prototyping, docs-only, refactoring, debugging)

---

## Implementation Sequence & Dependencies

### Dependency Graph

```
SPEC-936 ProviderRegistry (95% complete)
    ↓
SPEC-949 (Extended Model Support, Week 1-2, 16-24h)
    ├─ Enables: SPEC-948 testing with GPT-5 models
    └─ Blocks: None (standalone provider extension)

Config Infrastructure (exists: config_types.rs)
    ↓
SPEC-948 (Modular Pipeline Logic, Week 2-3, 20-28h)
    ├─ Creates: pipeline_config.rs (HARD DEPENDENCY for SPEC-947)
    ├─ Enables: SPEC-947 backend functionality
    └─ Blocks: None (optional: can use GPT-4 models)

SPEC-948 pipeline_config.rs (HARD DEPENDENCY)
    ↓
SPEC-947 (Pipeline UI Configurator, Week 3-4, 24-32h)
    ├─ Requires: SPEC-948 complete
    └─ Enables: User-facing configurator
```

**Critical Path**: SPEC-948 → SPEC-947 (SPEC-947 cannot start without SPEC-948 Phase 1)

**Parallel Opportunities**:
- SPEC-949 can execute fully in parallel with SPEC-948/947 (independent)
- SPEC-948 Phases 2-4 can overlap with SPEC-947 planning/design

---

## Shared Components & Integration Points

### Component 1: pipeline_config.rs (Created by SPEC-948, Used by SPEC-947)

**Creator**: SPEC-948 Phase 1 (Week 2, Days 1-3)
**Consumer**: SPEC-947 Phases 2-4 (Week 3-4)

**Interface Contract**:
```rust
pub struct PipelineConfig {
    pub spec_id: String,
    pub enabled_stages: Vec<StageType>,
    pub quality_gates: QualityGateConfig,
    // ...
}

impl PipelineConfig {
    pub fn load(spec_id: &str, cli_overrides: Option<PipelineOverrides>) -> Result<Self, String>;
    pub fn save(&self, path: &str) -> Result<(), String>;
    pub fn validate(&self) -> Result<ValidationResult, String>;
    pub fn is_enabled(&self, stage: StageType) -> bool;
}

pub enum StageType {
    New, Specify, Plan, Tasks, Implement, Validate, Audit, Unlock,
}

impl StageType {
    pub fn cost_estimate(&self) -> f64;
    pub fn duration_estimate(&self) -> u32;
    pub fn has_quality_gate(&self) -> bool;
}
```

**Integration Risk**: Medium (API changes in SPEC-948 affect SPEC-947)

**Mitigation**:
- SPEC-948 Phase 1 complete before SPEC-947 starts
- API review: SPEC-947 author validates SPEC-948 API meets UI needs
- Integration tests validate round-trip (load → modify → save)

---

### Component 2: ProviderRegistry (Extended by SPEC-949, Used by SPEC-948)

**Extender**: SPEC-949 Phase 1 (Week 1, Days 1-2)
**Consumer**: SPEC-948 Phase 2 (Week 2, Days 4-6) - model selection for stage execution

**Interface Contract** (from SPEC-936):
```rust
pub trait ProviderConfig {
    fn name(&self) -> &str;
    fn required_env_vars(&self) -> Vec<&str>;
    fn detect_oauth2_error(&self, stderr: &str) -> bool;
    fn format_small_prompt_args(&self, prompt: &str) -> Vec<String>;
    fn format_large_prompt_args(&self) -> Vec<String>;
}

pub struct ProviderRegistry {
    pub fn register(&mut self, provider: Box<dyn ProviderConfig>);
    pub fn get(&self, name: &str) -> Option<&dyn ProviderConfig>;
}
```

**Integration Risk**: Low (SPEC-949 extends existing, SPEC-948 uses existing agent system)

**Mitigation**:
- SPEC-949 doesn't modify ProviderConfig trait (only adds implementations)
- SPEC-948 uses agent names from config.toml (already exists)
- Model name mismatches caught by agent validation (Phase 2 Task 2.3)

---

### Component 3: Config Validation (Created by SPEC-948, Displayed by SPEC-947)

**Creator**: SPEC-948 Phase 1 Task 1.3 (dependency validation logic)
**Consumer**: SPEC-947 Phase 3 Task 3.2 (warning display in TUI)

**Interface Contract**:
```rust
pub struct ValidationResult {
    pub warnings: Vec<String>,
}

impl PipelineConfig {
    pub fn validate(&self) -> Result<ValidationResult, String> {
        // Returns Err() if hard dependencies violated (errors)
        // Returns Ok(ValidationResult) with warnings otherwise
    }
}
```

**Integration Risk**: Low (single source of truth for validation rules)

**Mitigation**:
- SPEC-947 reads ValidationResult, doesn't re-implement validation
- Warning formatting consistent (⚠ prefix, Error: prefix)
- UI and logic both use pipeline_config.rs validation

---

## Cross-SPEC Integration Testing Strategy

### Checkpoint 1: After SPEC-949 (GPT-5 Models Operational)

**Test Goals**: Validate new models work with existing /speckit commands

**Test Cases**:
1. **Single-Agent Validation**:
   - Given: /speckit.specify with gpt5_1_mini configured
   - When: Execute on test SPEC (e.g., SPEC-900)
   - Then: Specify stage completes, cost ~$0.08 (vs $0.10 GPT-4)
   - Evidence: `docs/SPEC-949-.../evidence/gpt5_validation.json`

2. **Multi-Agent Validation**:
   - Given: /speckit.plan with gpt5_1 in consensus (+ gemini-flash, claude-haiku)
   - When: Execute on test SPEC
   - Then: Plan stage completes, cost ~$0.30 (vs $0.35 GPT-4)
   - Evidence: Consensus artifacts with gpt5_1 responses

3. **Cost Reduction Measurement**:
   - Given: Full /speckit.auto pipeline
   - When: Execute with GPT-5 agents
   - Then: Total cost $2.30-$2.42 (target $2.36, -13%)
   - Evidence: cost_validation.md with actual measurements

**Success Criteria**:
- All 3 tests pass
- Cost reduction measured within ±2.5% of target
- No regressions in existing tests (604 tests remain 100% pass rate)

**Rollback Trigger**: Cost >$2.71 (worse than GPT-4 baseline)

---

### Checkpoint 2: After SPEC-948 (Backend Logic Complete)

**Test Goals**: Validate stage filtering, dependency validation, CLI flags

**Test Cases**:
1. **CLI Flag Override**:
   - Given: /speckit.auto SPEC-947 --skip-validate --skip-audit
   - When: Execute
   - Then: 6 stages execute (validate, audit skipped), skip telemetry written
   - Evidence: SKIPPED.json files in evidence directory

2. **Dependency Error Handling**:
   - Given: pipeline.toml with enabled_stages = ["implement"]
   - When: Load config
   - Then: Validation error "implement requires tasks to be enabled"
   - Evidence: Error message displayed, pipeline halts

3. **Quality Gate Bypass Warning**:
   - Given: pipeline.toml with plan, tasks disabled
   - When: Load config
   - Then: Warning "⚠ Skipping plan disables 2 quality gate checkpoints"
   - Evidence: Warning displayed before execution

4. **Workflow Pattern Validation**:
   - Given: rapid-prototyping.toml (specify, plan, tasks, implement only)
   - When: Execute /speckit.auto
   - Then: 4 stages execute, cost ~$0.66, time ~20 min
   - Evidence: Timing data, cost tracking

**Success Criteria**:
- All 4 tests pass
- Workflow cost/time within ±10% of estimates
- Validation prevents invalid configs
- CLI flags work correctly

**Rollback Trigger**: Stage filtering breaks pipeline (stages execute out of order)

---

### Checkpoint 3: After SPEC-947 (Full Feature Complete)

**Test Goals**: End-to-end validation (configure via UI, execute via logic, run with GPT-5)

**Test Cases**:
1. **Interactive Configuration**:
   - Given: /speckit.configure SPEC-947
   - When: Toggle validate off (Space key), press 'q' to save
   - Then: pipeline.toml written with validate disabled
   - Evidence: TOML file contents verified

2. **Configuration Execution**:
   - Given: pipeline.toml from Test 1 (validate disabled)
   - When: /speckit.auto SPEC-947
   - Then: Validate skipped, other stages execute
   - Evidence: Skip telemetry for validate stage

3. **End-to-End Integration**:
   - Given: Fresh SPEC, no config
   - When: /speckit.auto SPEC-XXX --configure
   - Then: Configurator launches → user configures → pipeline executes
   - Evidence: Full execution log (configurator + pipeline)

4. **GPT-5 Model Usage**:
   - Given: pipeline.toml (all stages enabled)
   - When: Execute with GPT-5 agents
   - Then: Cost $2.30-$2.42, 50-100% faster on simple stages
   - Evidence: Performance metrics, cost validation

**Success Criteria**:
- All 4 tests pass
- TUI configurator works on 80×24 and 120×40 terminals
- Configuration persists correctly (save/load round-trip)
- GPT-5 + modular pipeline integration seamless

**Rollback Trigger**: Modal breaks TUI, configuration doesn't save

---

## Timeline & Milestones

### Week 1-2: SPEC-949 (Extended Model Support)

**Week 1**:
- **Days 1-2 (Phase 1)**: Model registration (5 GPT-5 models in model_provider_info.rs)
- **Days 3-5 (Phase 2)**: Agent configs (config.toml template, subagent commands)
- **Days 6-7 (Phase 3)**: Provider stubs (Deepseek, Kimi - dead code)

**Week 2**:
- **Days 8-9 (Phase 4)**: Migration guide, validation with test SPEC
- **Days 9-10**: Cost reduction measurement (-13% target)

**Milestone 1**: GPT-5 models operational, cost reduction validated → SPEC-948 can use new models for testing

---

### Week 2-3: SPEC-948 (Modular Pipeline Logic)

**Week 2**:
- **Days 1-3 (Phase 1)**: pipeline_config.rs creation (250-300 LOC, 10-12 unit tests)

**Week 3**:
- **Days 4-6 (Phase 2)**: pipeline_coordinator.rs modifications (stage filtering, skip logic)
- **Days 1-2 (Phase 3)**: CLI flag support (--skip-*, --stages=)
- **Days 3-4 (Phase 4)**: Documentation (guide + 4 workflow examples)

**Milestone 2**: Backend logic complete, SPEC-947 unblocked → TUI can use pipeline_config.rs API

---

### Week 3-4: SPEC-947 (Pipeline UI Configurator)

**Week 3-4**:
- **Day 1 (Phase 1)**: API verification (0h - SPEC-948 already done)
- **Days 2-4 (Phase 2)**: Widget core (state machine, event handling, rendering)
- **Days 5-7 (Phase 3)**: Interactive components (stage_selector, stage_details, help bar)
- **Days 8-9 (Phase 4)**: Command integration (/speckit.configure, --configure flag)

**Milestone 3**: Full feature complete → User-facing interactive configurator ready

---

**Total Timeline**: 3-4 weeks (60-84 hours)

**Critical Path**: SPEC-948 Phase 1 → SPEC-947 (2-day buffer built in)

---

## Consolidated Risk Matrix

### High-Severity Risks

| Risk | SPEC | Probability | Impact | Mitigation | Recovery Time |
|------|------|-------------|--------|------------|---------------|
| SPEC-948 API incomplete for SPEC-947 | 948/947 | Very Low | SPEC-947 blocked | Phase 1 verification, API review | 4-8h (extend API) |
| Stage filtering breaks pipeline | 948 | Low | Pipeline halts | 8-10 integration tests, manual testing | 1-2h (rollback) |
| Modal rendering crashes TUI | 947 | Low | Feature unusable | Follow proven patterns, multi-terminal testing | 1-2h (disable command) |

### Medium-Severity Risks

| Risk | SPEC | Probability | Impact | Mitigation | Recovery Time |
|------|------|-------------|--------|------------|---------------|
| GPT-5 model names change | 949 | Medium | Model lookup fails | Model aliases, OpenAI monitoring | <1h (update names) |
| Config precedence bugs | 948 | Medium | Wrong config applied | 6+ precedence tests, validation | 2h (fix merge logic) |
| TUI state machine bugs | 947 | Low | Toggle doesn't update cost | 6-8 state tests, immutable updates | 2-4h (fix state sync) |

### Low-Severity Risks

| Risk | SPEC | Probability | Impact | Mitigation | Recovery Time |
|------|------|-------------|--------|------------|---------------|
| ChatGPT API lacks GPT-5 access | 949 | Low | Fallback to GPT-4 | Validation before deployment, fallback logic | <30min (config change) |
| Evidence footprint growth | 948 | High | Soft limit exceeded | Minimal skip telemetry (~100 bytes), archival | N/A (acceptable) |
| Terminal size variations | 947 | High | Layout broken | Percentage-based layout, resize handling | 2h (add constraints) |

---

## Performance Validation Framework

### SPEC-949 Metrics (GPT-5 Family)

**Cost Reduction**:
- **Baseline**: $2.71 per /speckit.auto run (GPT-4 era)
- **Target**: $2.36 per run (-13%)
- **Measurement**: Extract from telemetry, sum all stage costs
- **Success**: $2.30-$2.42 (±2.5%)

**Adaptive Reasoning Speedup**:
- **Baseline**: 3-4 minutes (GPT-4-turbo on specify/tasks)
- **Target**: 1.5-2 minutes (2-3× faster)
- **Measurement**: SPEC-940 timing infrastructure, measure_time! macro
- **Success**: <2.5 minutes (50% faster minimum)

**Caching Effectiveness**:
- **Target**: 50-90% cost reduction on follow-up queries (24h cache)
- **Measurement**: Run /speckit.plan twice (1 min apart), compare costs
- **Success**: Follow-up cost <20% of initial

---

### SPEC-948 Metrics (Workflow Patterns)

**Pipeline Execution Time**:
- **Full pipeline**: ~45-50 minutes (baseline)
- **Rapid prototyping**: ~20 minutes (target, 60% savings)
- **Docs-only**: ~15 minutes (target, 70% savings)
- **Code refactoring**: ~25 minutes (target, 50% savings)
- **Measurement**: SPEC-940 timing, end-to-end workflow tests
- **Success**: Within ±10% of target

**Config Load Latency**:
- **Target**: <50ms (negligible overhead)
- **Measurement**: Measure PipelineConfig::load() duration
- **Success**: <100ms (acceptable UX)

---

### SPEC-947 Metrics (TUI Responsiveness)

**Modal Render Time**:
- **Target**: <100ms (instant perceived)
- **Measurement**: Time from command to modal visible
- **Success**: <200ms (acceptable)

**Toggle Response Time**:
- **Target**: <50ms (instant feedback)
- **Measurement**: Space key press to cost update display
- **Success**: <100ms (acceptable)

---

## Validation Against Project Standards

### SPEC-Kit Automation Standards ✅

- **6-Stage Template**: ✅ All SPECs follow specify → plan → tasks → implement → validate → unlock
- **Telemetry v1**: ✅ All skip telemetry uses schemaVersion: "1.0"
- **Quality Gates**: ✅ SPEC-948 calculates active checkpoints, SPEC-947 displays bypass warnings
- **Evidence Policy**: ✅ Skip metadata minimal (~100 bytes), within 25MB soft limit

### Test Coverage Policy ✅

- **Target**: 40%+ coverage (from testing-policy.md)
- **SPEC-949**: 17-21 tests (unit + integration + performance)
- **SPEC-948**: 24-30 tests (unit + integration)
- **SPEC-947**: 17-21 tests (unit + integration + manual TUI)
- **Total**: 58-72 new tests (maintains 100% pass rate on existing 604 tests)

### Evidence Policy ✅

- **Soft Limit**: 25MB per SPEC (from evidence-policy.md)
- **SPEC-949**: ~5MB (validation telemetry, cost benchmarks, migration guide)
- **SPEC-948**: ~3MB (config tests, skip telemetry, workflow validation)
- **SPEC-947**: ~2MB (TUI screenshots, integration tests)
- **Total**: ~10MB (well within limits)

### Upstream Sync Considerations ✅

- **FORK-SPECIFIC Markers**: Used in model_provider_info.rs (agent_total_timeout_ms)
- **Isolation**: All three SPECs extend existing infrastructure, no upstream conflicts
- **Rebase Safety**: Modular changes, minimal file overlap, clear SPEC boundaries

---

## Success Criteria (Overall Integration)

### Functional Criteria

1. ✅ **SPEC-949**: All 5 GPT-5 models operational, cost -13% achieved
2. ✅ **SPEC-948**: Stage filtering works, 4 workflow patterns validated
3. ✅ **SPEC-947**: TUI configurator functional, saves/loads configs correctly
4. ✅ **Integration**: GPT-5 models + modular pipeline work together seamlessly

### Quality Criteria

1. ✅ **Test Coverage**: 58-72 new tests, 100% pass rate maintained (current 604 + new = 662-676 total)
2. ✅ **No Regressions**: All existing /speckit.* commands work unchanged
3. ✅ **Backward Compatible**: Existing SPECs execute unchanged (no pipeline.toml = defaults used)
4. ✅ **Evidence Within Limits**: All three SPECs <25MB each

### Documentation Criteria

1. ✅ **User Docs**: GPT5_MIGRATION_GUIDE, PROVIDER_SETUP_GUIDE, PIPELINE_CONFIGURATION_GUIDE
2. ✅ **Workflow Examples**: 4 example pipeline.toml files with cost/time estimates
3. ✅ **CLAUDE.md Updates**: New commands documented (/speckit.configure, CLI flags)
4. ✅ **CHANGELOG**: All three SPECs documented with changes, features, impacts

### Performance Criteria

1. ✅ **Cost Reduction**: $2.71 → $2.36 (SPEC-949), customizable $0.66-$2.71 (SPEC-948/947)
2. ✅ **Speed Improvement**: 50-100% faster on simple stages (SPEC-949 GPT-5.1)
3. ✅ **Time Savings**: 50-70% on workflow patterns (SPEC-948)
4. ✅ **Responsiveness**: <100ms config load, <100ms toggle response (SPEC-948/947)

---

## Next Steps

### Immediate (Post-Analysis)

1. ✅ **Store Research SPECs to local-memory**: All three research SPECs (947, 948, 949) with importance ≥9
2. ✅ **Update SPEC.md**: Add three implementation SPECs to backlog (SPEC-949-IMPL, SPEC-948-IMPL, SPEC-947-IMPL)
3. ⏭️ **Implementation**: Begin SPEC-949-IMPL Phase 1 (Week 1, Days 1-2)

### Week 1-2 (SPEC-949 Implementation)

- Days 1-2: Model registration
- Days 3-5: Agent configs
- Days 6-7: Provider stubs
- Days 8-10: Migration & validation
- **Checkpoint**: GPT-5 models operational, cost reduction validated

### Week 2-3 (SPEC-948 Implementation)

- Days 1-3: pipeline_config.rs data layer
- Days 4-6: pipeline_coordinator.rs modifications
- Days 1-2: CLI flag support
- Days 3-4: Documentation
- **Checkpoint**: Backend logic complete, SPEC-947 unblocked

### Week 3-4 (SPEC-947 Implementation)

- Day 1: API verification (0h)
- Days 2-4: Widget core
- Days 5-7: Interactive components
- Days 8-9: Command integration
- **Checkpoint**: Full feature complete, user-facing

### Week 4+ (Post-Implementation)

- User acceptance testing (2-3 real SPECs)
- Production deployment
- Monitor cost/performance metrics for 1 week
- Document lessons learned in local-memory

---

**Integration Analysis Complete**: 2025-11-16
**Implementation Ready**: All three SPECs have detailed, actionable plans
**Total Estimated Effort**: 60-84 hours (3-4 weeks, single developer)
