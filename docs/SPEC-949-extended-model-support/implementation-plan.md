# SPEC-949-IMPL: Extended Model Support Implementation Plan

**Research SPEC**: SPEC-949 (complete)
**Implementation Sequence**: 1/3 (First - enables SPEC-948 testing)
**Estimated Duration**: 16-24 hours (1-1.5 weeks)
**Dependencies**: ProviderRegistry infrastructure (exists: async_agent_executor.rs:434), Config infrastructure (exists: config_types.rs)
**Created**: 2025-11-16
**Priority**: P1 - High (Strategic Model Diversification)

---

## Executive Summary

This implementation plan delivers GPT-5/5.1 family integration (5 models) into the spec-kit automation framework, with stub infrastructure for future Deepseek V3/R1 and Kimi K2 providers. Expected cost reduction: $2.71 → $2.36 per `/speckit.auto` run (-13%) with performance improvements from adaptive reasoning and extended caching.

**Cost Baseline**: $2.71 represents current GPT-4 era cost. This SPEC targets $2.36 (GPT-5 era). All cost comparisons use $2.71 → $2.36 migration context.

**Strategic Impact**:
- **Performance**: 2-3× faster on simple tasks (adaptive reasoning)
- **Caching**: 24-hour prompt cache vs 5-minute (massive follow-up savings)
- **Specialization**: Codex variants optimized for agentic software engineering
- **Future-Proof**: OpenAI-compatible provider stubs for vendor diversity

---

## Implementation Phases

### Phase 1: Model Registration (Week 1, Days 1-2, 4-6 hours)

**Objective**: Register all 5 GPT-5 family models in model provider infrastructure

**Tasks**:

**Task 1.1**: Add GPT-5 models to `model_provider_info.rs`
- **File**: `codex-rs/core/src/model_provider_info.rs`
- **Action**: Extend `default_model_provider_info()` HashMap
- **Changes**:
  - Add 5 entries after existing GPT-4 models (lines ~200-250)
  - Structure: `map.insert("gpt-5".to_string(), ModelProviderInfo { ... })`
  - Fields: provider="openai", supports_responses_api=true, timeout=1800000ms
- **LOC**: ~60 lines new code (5 models × 12 lines each)
- **Rationale**: Central model registry enables provider lookup, timeout config, wire API selection
- **Dependencies**: None (extends existing HashMap)

```rust
// Example addition (lines 200-211):
map.insert("gpt-5".to_string(), ModelProviderInfo {
    provider: "openai".to_string(),
    model_id: "gpt-5".to_string(),
    supports_responses_api: true,
    heartbeat_interval_ms: Some(30000),
    agent_total_timeout_ms: Some(1800000),  // 30 min
});

map.insert("gpt-5.1".to_string(), ModelProviderInfo {
    provider: "openai".to_string(),
    model_id: "gpt-5.1".to_string(),
    supports_responses_api: true,
    heartbeat_interval_ms: Some(30000),
    agent_total_timeout_ms: Some(1800000),
});

// ... (repeat for gpt-5-codex, gpt-5.1-codex, gpt-5.1-codex-mini)
```

**Task 1.2**: Update agent validation logic
- **File**: `codex-rs/core/src/model_provider_info.rs` (existing validation functions)
- **Action**: Ensure GPT-5 models recognized as valid OpenAI models
- **Changes**: No code changes needed (existing provider="openai" validation covers this)
- **LOC**: 0 lines (validation already extensible)
- **Rationale**: Validation uses provider field, which is "openai" for all GPT-5 models

**Deliverables**:
- `model_provider_info.rs` modified (~+60 LOC)
- All 5 models registered with proper timeout/API settings
- Tests: 5-7 unit tests (model registration, provider lookup, timeout validation)

**Validation**:
```bash
# Compilation check
cd codex-rs && cargo build -p codex-core

# Unit tests
cargo test -p codex-core model_provider_info::tests::test_gpt5_models
```

**Success Criteria**:
- All 5 GPT-5 models registered in default HashMap
- Model lookup by name returns correct ModelProviderInfo
- Timeout values set to 30 minutes (1800000ms)
- No compilation warnings
- 5-7 unit tests passing

**Milestone 1**: Model registry updated, ready for agent config integration

---

### Phase 2: Config Integration (Week 1, Days 3-5, 6-8 hours)

**Objective**: Add agent configurations for GPT-5 models to config template and subagent commands

**Tasks**:

**Task 2.1**: Add GPT-5 agent configs to `config.toml` template
- **File**: `codex-rs/core/config_template.toml` (or wherever default config is defined)
- **Action**: Add 5 agent definitions to `[agents]` section
- **Changes**:
  - Add `[agents.gpt5]`, `[agents.gpt5_1]`, `[agents.gpt5_codex]`, etc.
  - Each with: command="chatgpt", model="{name}", temperature, args
  - Place after existing GPT-4 agents, before Gemini section
- **LOC**: ~60 lines (5 agents × ~12 lines each)
- **Rationale**: Makes GPT-5 models available to subagent command selection
- **Dependencies**: Task 1.1 (model registration)

```toml
# Example (add to config template lines ~80-140):

# ============================================================================
# GPT-5 FAMILY AGENTS (SPEC-949)
# ============================================================================

# GPT-5: Flagship reasoning model
[agents.gpt5]
command = "chatgpt"
model = "gpt-5"
temperature = 0.7
args = ["--model", "gpt-5"]
env = { OPENAI_API_KEY = "${OPENAI_API_KEY}" }

# GPT-5.1: Adaptive reasoning with extended caching
[agents.gpt5_1]
command = "chatgpt"
model = "gpt-5.1"
temperature = 0.7
args = ["--model", "gpt-5.1"]

# GPT-5-Codex: Agentic software engineering
[agents.gpt5_codex]
command = "chatgpt"
model = "gpt-5-codex"
temperature = 0.3  # Lower for code generation
args = ["--model", "gpt-5-codex"]

# GPT-5.1-Codex: Enhanced agentic with tool use
[agents.gpt5_1_codex]
command = "chatgpt"
model = "gpt-5.1-codex"
temperature = 0.3
args = ["--model", "gpt-5.1-codex"]

# GPT-5.1-Codex-Mini: Cost-optimized for high-volume
[agents.gpt5_1_mini]
command = "chatgpt"
model = "gpt-5.1-codex-mini"
temperature = 0.5
args = ["--model", "gpt-5.1-codex-mini"]
args_read_only = ["--model", "gpt-5.1-codex-mini", "-s", "read-only"]
args_write = ["--model", "gpt-5.1-codex-mini", "-s", "workspace-write"]
```

**Task 2.2**: Update subagent command model assignments
- **File**: `codex-rs/tui/src/chatwidget/spec_kit/subagent_defaults.rs` (agent arrays at lines 42, 52, 59, 66, 73)
- **Action**: Update agent arrays for each stage to use GPT-5 models
- **Changes**:
  - `speckit-specify`: ["gpt5_1_mini"] (was gpt5-low)
  - `speckit-plan`: ["gemini-flash", "claude-haiku", "gpt5_1"] (was gpt5-medium)
  - `speckit-tasks`: ["gpt5_1_mini"] (was gpt5-low)
  - `speckit-implement`: ["gpt5_1_codex", "claude-haiku"] (was gpt_codex)
  - `speckit-validate`: ["gemini-flash", "claude-haiku", "gpt5_1"]
  - `speckit-audit`: ["gpt5_codex", "claude-sonnet", "gemini-pro"] (was gpt5-high)
  - `speckit-unlock`: ["gpt5_codex", "claude-sonnet", "gemini-pro"]
- **LOC**: ~+20/-10 lines (update agent names in existing command definitions)
- **Rationale**: Route stages to optimal GPT-5 variants (codex for code, mini for simple, adaptive for multi-agent)
- **Dependencies**: Task 2.1 (agent configs exist)

**Task 2.3**: Validate agent availability at runtime
- **File**: `codex-rs/tui/src/chatwidget/spec_kit/handler.rs` (or config validator)
- **Action**: Add warning if GPT-5 agents requested but not in user's config.toml
- **Changes**: Extend existing agent validation to check for new agent names
- **LOC**: ~+10 lines (extend validation loop)
- **Rationale**: Graceful degradation if user hasn't updated config
- **Dependencies**: Task 2.2 (subagent commands use new names)

```rust
// Example validation addition:
fn validate_agent_availability(config: &Config, required_agents: &[String]) -> Result<(), String> {
    for agent_name in required_agents {
        if !config.agents.contains_key(agent_name) {
            eprintln!("Warning: Agent '{}' not found in config, falling back to default", agent_name);
            // Fallback logic here (use gemini-flash or claude-haiku)
        }
    }
    Ok(())
}
```

**Deliverables**:
- Config template updated (~+60 LOC)
- Subagent commands updated (~+20/-10 LOC)
- Agent validation enhanced (~+10 LOC)
- Tests: 6-8 integration tests (command parsing, agent selection, fallback logic)

**Validation**:
```bash
# Integration tests
cargo test -p codex-tui spec_kit::tests::test_gpt5_agent_selection

# Manual validation with sample SPEC
# (in TUI):
/speckit.plan SPEC-900  # Should use gpt5_1 in multi-agent consensus
```

**Success Criteria**:
- All 5 GPT-5 agents defined in config template
- Subagent commands reference new agent names
- Warning shown if agent not in user config
- Fallback to existing models works
- 6-8 integration tests passing

**Milestone 2**: GPT-5 agents integrated into spec-kit pipeline, ready for testing

---

### Phase 3: Provider Stubs (Week 1-2, Days 6-7, 4-6 hours)

**Objective**: Create provider stubs for future Deepseek and Kimi integration (dead code, documented)

**Tasks**:

**Task 3.1**: Implement Deepseek provider stub
- **File**: `codex-rs/core/src/async_agent_executor.rs`
- **Action**: Add `DeepseekProvider` struct implementing `ProviderConfig` trait
- **Changes**:
  - Add after `OpenAIProvider` impl (lines ~410-480)
  - Mark with `#[allow(dead_code)]` (not yet registered)
  - Document: "STUB - SPEC-949: Not yet integrated, awaiting DEEPSEEK_API_KEY"
  - Implement all trait methods (name, required_env_vars, detect_oauth2_error, format_*_args)
- **LOC**: ~60 lines (struct + trait impl)
- **Rationale**: Pre-build infrastructure for future integration, validate OpenAI-compatible approach
- **Dependencies**: None (uses existing ProviderConfig trait from SPEC-936)

```rust
// Example stub (add lines ~480-540):

/// Deepseek provider configuration (STUB - SPEC-949)
///
/// Status: Not yet integrated (no DEEPSEEK_API_KEY available)
/// API Compatibility: OpenAI-compatible endpoints
/// Base URL: https://api.deepseek.com/v1
/// Models: deepseek-chat (V3), deepseek-v3.1, deepseek-reasoner (R1)
///
/// Integration: Uncomment registration in ProviderRegistry::with_defaults()
/// when DEEPSEEK_API_KEY is obtained.
#[allow(dead_code)]
pub struct DeepseekProvider;

impl ProviderConfig for DeepseekProvider {
    fn name(&self) -> &str {
        "deepseek"
    }

    fn required_env_vars(&self) -> Vec<&str> {
        vec!["DEEPSEEK_API_KEY"]
    }

    fn detect_oauth2_error(&self, stderr: &str) -> bool {
        stderr.contains("invalid_api_key")
            || stderr.contains("authentication_failed")
            || stderr.contains("API key")
    }

    fn format_small_prompt_args(&self, prompt: &str) -> Vec<String> {
        vec![
            "--base-url".to_string(),
            "https://api.deepseek.com/v1".to_string(),
            "--model".to_string(),
            "deepseek-chat".to_string(),
            "-p".to_string(),
            prompt.to_string(),
        ]
    }

    fn format_large_prompt_args(&self) -> Vec<String> {
        vec![
            "--base-url".to_string(),
            "https://api.deepseek.com/v1".to_string(),
            "--model".to_string(),
            "deepseek-chat".to_string(),
            // Large prompts sent via stdin
        ]
    }
}
```

**Task 3.2**: Implement Kimi provider stub
- **File**: `codex-rs/core/src/async_agent_executor.rs`
- **Action**: Add `KimiProvider` struct implementing `ProviderConfig` trait
- **Changes**: Similar to Task 3.1 but for Kimi/Moonshot AI
  - Base URL: https://platform.moonshot.ai/v1
  - Env var: MOONSHOT_API_KEY
  - Models: kimi-k2, kimi-k2-thinking
- **LOC**: ~60 lines (struct + trait impl)
- **Rationale**: Pre-build for future 256K context, fast inference model
- **Dependencies**: None (parallel to Task 3.1)

**Task 3.3**: Document future activation process
- **File**: `codex-rs/core/src/async_agent_executor.rs` (inline comments)
- **Action**: Add commented-out registration in `ProviderRegistry::with_defaults()`
- **Changes**:
  - Find `impl ProviderRegistry` block (lines ~434-449)
  - Add commented lines after OpenAI registration
- **LOC**: ~4 lines (commented registration calls)
- **Rationale**: Clear activation path when API keys obtained
- **Dependencies**: Task 3.1, 3.2 (provider structs exist)

```rust
// Example commented registration (lines ~446-448):

impl ProviderRegistry {
    pub fn with_defaults() -> Self {
        let mut registry = Self::new();

        // Existing providers
        registry.register(Box::new(AnthropicProvider));
        registry.register(Box::new(GoogleProvider));
        registry.register(Box::new(OpenAIProvider));

        // Future providers (SPEC-949 - uncomment when API keys available)
        // registry.register(Box::new(DeepseekProvider));
        // registry.register(Box::new(KimiProvider));

        registry
    }
}
```

**Deliverables**:
- `async_agent_executor.rs` modified (~+124 LOC for 2 stubs + comments)
- Deepseek provider stub (dead code, fully documented)
- Kimi provider stub (dead code, fully documented)
- Commented registration in ProviderRegistry
- Tests: No new tests (stubs not active)

**Validation**:
```bash
# Compilation check (stubs should not cause errors)
cd codex-rs && cargo build -p codex-core

# Verify dead_code attribute prevents warnings
cargo clippy -p codex-core --all-features
```

**Success Criteria**:
- Both provider stubs compile without errors
- `#[allow(dead_code)]` prevents unused code warnings
- Commented registration shows clear activation path
- Documentation explains stub status and requirements

**Milestone 3**: Future provider infrastructure ready for activation

---

### Phase 4: Migration & Validation (Week 2, Days 8-10, 2-4 hours)

**Objective**: Document migration, validate GPT-5 models work correctly, measure cost/performance improvements

**Tasks**:

**Task 4.1**: Create GPT-5 migration guide
- **File**: `docs/spec-kit/GPT5_MIGRATION_GUIDE.md` (NEW)
- **Action**: Write migration documentation (200-300 lines)
- **Content**:
  - Overview of GPT-5 family models (5 models, use cases)
  - Migration steps (update config.toml, test with /speckit.plan, measure cost)
  - Model mapping (GPT-4 → GPT-5 equivalents)
  - Troubleshooting (model not found, API key issues, rate limits)
  - Performance expectations (2-3× faster, 24h caching)
- **LOC**: ~200-300 lines markdown
- **Rationale**: Users need clear upgrade path
- **Dependencies**: Phases 1-3 complete

**Task 4.2**: Document provider setup process
- **File**: `docs/spec-kit/PROVIDER_SETUP_GUIDE.md` (NEW)
- **Action**: Write provider activation guide (300-400 lines)
- **Content**:
  - Section 1: GPT-5 setup (already active by default)
  - Section 2: Deepseek activation (uncomment stub, set DEEPSEEK_API_KEY, test)
  - Section 3: Kimi activation (similar to Deepseek)
  - Section 4: Troubleshooting OpenAI-compatible APIs
  - Section 5: Future providers (how to add custom providers)
- **LOC**: ~300-400 lines markdown
- **Rationale**: Enable future provider diversity
- **Dependencies**: Phase 3 (stubs exist)

**Task 4.3**: Validate GPT-5 models with test SPEC
- **File**: Evidence files in `docs/SPEC-949-extended-model-support/evidence/`
- **Action**: Execute test SPEC (e.g., SPEC-900 or small new SPEC) using GPT-5 models
- **Test Cases**:
  1. Single-agent stage (specify): gpt5_1_mini
  2. Multi-agent stage (plan): gpt5_1 in consensus
  3. Code generation (implement): gpt5_1_codex
  4. Full pipeline with GPT-5 agents
- **Metrics to Capture**:
  - Cost per stage (compare to GPT-4 baseline)
  - Duration per stage (measure adaptive reasoning speedup)
  - Output quality (subjective assessment)
  - Caching behavior (follow-up queries)
- **LOC**: N/A (test execution, evidence capture)
- **Rationale**: Validate cost reduction claim ($2.71 → $2.36)
- **Dependencies**: Phases 1-2 (models registered, agents configured)

**Task 4.4**: Measure and document cost reduction
- **File**: `docs/SPEC-949-extended-model-support/evidence/cost_validation.md` (NEW)
- **Action**: Compare costs before/after GPT-5 migration
- **Baseline** (GPT-4 era, from SPEC-070):
  - Specify: $0.10 (gpt5-low)
  - Plan: $0.35 (gemini-flash + claude-haiku + gpt5-medium)
  - Tasks: $0.10 (gpt5-low)
  - Implement: $0.11 (gpt_codex + claude-haiku)
  - Validate: $0.35 (3 agents)
  - Audit: $0.80 (3 premium)
  - Unlock: $0.80 (3 premium)
  - **Total**: $2.71
- **Expected** (GPT-5 era):
  - Specify: $0.08 (gpt5_1_mini, -20%)
  - Plan: $0.30 (gpt5_1, -14%)
  - Tasks: $0.08 (gpt5_1_mini, -20%)
  - Implement: $0.10 (gpt5_1_codex, -9%)
  - Validate: $0.30 (gpt5_1, -14%)
  - Audit: $0.80 (gpt5_codex, same)
  - Unlock: $0.80 (gpt5_codex, same)
  - **Total**: $2.36 (-13%)
- **Actual** (measured from Task 4.3 validation run)
- **LOC**: ~100-150 lines markdown (comparison table, analysis)
- **Rationale**: Validate research SPEC cost estimates
- **Dependencies**: Task 4.3 (validation run complete)

**Deliverables**:
- `GPT5_MIGRATION_GUIDE.md` (~200-300 lines)
- `PROVIDER_SETUP_GUIDE.md` (~300-400 lines)
- Validation evidence (SPEC execution logs, consensus artifacts, cost tracking)
- `cost_validation.md` (~100-150 lines with actual measurements)

**Validation**:
```bash
# Run test SPEC with GPT-5 models
/speckit.auto SPEC-900  # Or create small test SPEC

# Measure cost (check telemetry/evidence files)
cat docs/SPEC-949-extended-model-support/evidence/cost_validation.md

# Verify cost reduction achieved
grep "Total cost" evidence/*.json
```

**Success Criteria**:
- Migration guide complete and reviewed
- Provider setup guide covers all 3 future providers
- At least 1 test SPEC executed successfully with GPT-5 models
- Cost reduction measured: target -13%, acceptable range -10% to -15%
- Performance improvement observed (2-3× faster on simple tasks)

**Milestone 4**: GPT-5 integration validated, ready for production use in SPEC-948 testing

---

## Complete File Manifest

### New Files (SPEC-949-IMPL)

| File Path | Purpose | LOC | Tests | Phase |
|-----------|---------|-----|-------|-------|
| `docs/spec-kit/GPT5_MIGRATION_GUIDE.md` | User migration documentation | 200-300 | N/A | Phase 4 |
| `docs/spec-kit/PROVIDER_SETUP_GUIDE.md` | Provider activation guide | 300-400 | N/A | Phase 4 |
| `docs/SPEC-949-.../evidence/cost_validation.md` | Cost comparison analysis | 100-150 | N/A | Phase 4 |

**Total New**: 3 files, ~600-850 LOC (documentation)

### Modified Files (SPEC-949-IMPL)

| File Path | Changes | LOC Change | Rationale | Risk | Phase |
|-----------|---------|------------|-----------|------|-------|
| `codex-rs/core/src/model_provider_info.rs` | Add 5 GPT-5 models to HashMap | +60/-0 | Model registration | Low | Phase 1 |
| `codex-rs/core/config_template.toml` | Add 5 agent configs | +60/-0 | Agent availability | Low | Phase 2 |
| `codex-rs/tui/src/chatwidget/spec_kit/handler.rs` | Update agent names in commands | +20/-10 | Model routing | Low | Phase 2 |
| `codex-rs/core/src/async_agent_executor.rs` | Add provider stubs (dead code) | +124/-0 | Future providers | None | Phase 3 |

**Total Modified**: 4 files, ~+264/-10 LOC net

---

## Test Coverage Plan

### Unit Test Matrix

| Module | Coverage Target | Test Count | Key Scenarios |
|--------|-----------------|------------|---------------|
| model_provider_info | 80%+ | 5-7 | Model lookup by name, timeout values, provider validation |
| agent_config_parsing | 70%+ | 6-8 | Config parsing, agent selection, fallback logic |

**Total Unit Tests**: 11-15 tests (~120-180 lines)

### Integration Test Scenarios

1. **Model Registration Validation**:
   - Given: Fresh model_provider_info HashMap
   - When: Query for "gpt-5.1-codex"
   - Then: Returns ModelProviderInfo with correct provider="openai", timeout=1800000ms
   - Validates: Phase 1 Task 1.1

2. **Agent Selection for Stage**:
   - Given: Config with all GPT-5 agents defined
   - When: Execute /speckit.plan with test SPEC
   - Then: Multi-agent consensus uses gpt5_1, gemini-flash, claude-haiku
   - Validates: Phase 2 Task 2.2

3. **Fallback When Agent Missing**:
   - Given: Config WITHOUT gpt5_1_mini defined
   - When: Execute /speckit.specify
   - Then: Warning logged, falls back to gemini-flash or claude-haiku
   - Validates: Phase 2 Task 2.3

4. **Provider Stub Compilation**:
   - Given: DeepseekProvider and KimiProvider structs exist
   - When: Compile with cargo build
   - Then: No errors, no unused code warnings (dead_code attribute)
   - Validates: Phase 3 Tasks 3.1, 3.2

5. **End-to-End Cost Validation**:
   - Given: Test SPEC with all stages
   - When: Execute /speckit.auto with GPT-5 agents
   - Then: Total cost $2.30-$2.42 (target $2.36 ± 2.5%)
   - Validates: Phase 4 Task 4.4

6. **Performance Validation**:
   - Given: Simple specify prompt (500 tokens)
   - When: Execute with gpt5_1_mini
   - Then: Response time <2s (vs ~3-4s with gpt-4-turbo)
   - Validates: Adaptive reasoning 2-3× speedup claim

**Total Integration Tests**: 6 tests (~200-250 lines)

### Performance Validation Tests

**Metric 1: Cost Reduction**
- **Description**: Total cost per /speckit.auto run
- **Baseline**: $2.71 (GPT-4 era, measured from SPEC-070 implementation)
- **Target**: $2.36 (-13%)
- **Measurement Method**:
  ```bash
  # Extract cost from telemetry
  grep '"cost"' docs/SPEC-949-.../evidence/*.json | jq '.cost' | awk '{s+=$1} END {print s}'
  ```
- **Validation**: Run n≥3 test SPECs, calculate mean cost
- **Success Threshold**: $2.30-$2.42 (target ± 2.5%)

**Metric 2: Adaptive Reasoning Speedup**
- **Description**: Time for simple single-agent stages (specify, tasks)
- **Baseline**: 3-4 minutes (GPT-4-turbo)
- **Target**: 1.5-2 minutes (2-3× faster)
- **Measurement Method**: Use SPEC-940 timing infrastructure (`measure_time!` macro)
- **Validation**: Run n≥10 iterations, mean±stddev, p<0.01
- **Success Threshold**: <2.5 minutes (50% faster minimum)

**Metric 3: Caching Effectiveness**
- **Description**: Cost reduction on follow-up queries (24h cache vs 5min)
- **Baseline**: 100% cost on every query (5min cache expires)
- **Target**: 50-90% cost reduction on follow-ups (24h cache active)
- **Measurement Method**:
  1. Run /speckit.plan SPEC-900 (first run, no cache)
  2. Wait 1 minute, run again (24h cache hit expected)
  3. Compare costs
- **Validation**: Follow-up cost <20% of initial cost
- **Success Threshold**: ≥70% cost reduction on cached queries

---

## Migration & Rollback Plan

### Incremental Deployment

**Phase 1 Complete → Deploy**:
- Merge: Phase 1 changes (model_provider_info.rs)
- Validate: Unit tests pass, model lookup works
- Checkpoint: If tests fail, rollback Phase 1 commit

**Phase 2 Complete → Deploy**:
- Merge: Phase 2 changes (config template, handler.rs)
- Validate: Integration tests pass, agent selection works
- Checkpoint: If agent selection fails, rollback Phase 2 commit
- User Testing: Run /speckit.plan with test SPEC, verify gpt5_1 used

**Phase 3 Complete → Deploy**:
- Merge: Phase 3 changes (provider stubs)
- Validate: Compilation succeeds, no warnings
- Checkpoint: Stubs remain inactive (commented out registration)

**Phase 4 Complete → Production**:
- Merge: Phase 4 changes (documentation, validation evidence)
- Validate: Full pipeline test (SPEC-900 or similar)
- Checkpoint: Cost reduction measured, matches expectations
- Production Deployment: Update default config.toml in ~/.code/
- Monitoring: Track cost/performance metrics for 1 week

### Backward Compatibility

**Preserved**:
- All existing /speckit.* commands work unchanged (default behavior)
- GPT-4 agents still available (not removed, just deprioritized)
- Config.toml without GPT-5 agents still works (fallback logic)
- No breaking changes to existing SPECs (execution paths identical)

**Optional**:
- GPT-5 agents opt-in via config.toml update
- Users can choose to keep GPT-4 agents (override subagent_commands)
- Default behavior: Use GPT-5 if available, fall back to GPT-4

### Rollback Strategy

**Rollback Trigger: Cost increase detected**
- **Condition**: Cost per pipeline >$2.71 (worse than GPT-4 baseline)
- **Action**:
  1. Revert handler.rs agent name changes (restore gpt5-low → gpt4-turbo)
  2. Keep model registry intact (no harm in having GPT-5 models registered)
  3. Document cost anomaly in SPEC evidence
- **Recovery Time**: <1 hour (simple git revert + cargo build)

**Rollback Trigger: Performance degradation**
- **Condition**: Stage duration >10% slower than GPT-4 baseline
- **Action**:
  1. Revert to GPT-4 agents for affected stages only
  2. Keep GPT-5 for other stages (partial rollback)
  3. Investigate performance issue (model API changes? rate limits?)
- **Recovery Time**: <2 hours (selective rollback + testing)

**Rollback Trigger: Model availability issues**
- **Condition**: GPT-5 models return 404 or "model not found" errors
- **Action**:
  1. Full rollback to GPT-4 agents
  2. Check OpenAI API status, model naming changes
  3. Update model names if OpenAI renamed models (e.g., gpt-5 → gpt-5-0324)
- **Recovery Time**: <1 hour (immediate fallback) + investigation time

**Rollback Procedure**:
```bash
# 1. Revert Phase 2 changes (agent selection)
git revert <phase-2-commit-hash>

# 2. Rebuild
cd codex-rs && cargo build --workspace

# 3. Test with existing SPEC
/speckit.auto SPEC-900  # Should use GPT-4 agents now

# 4. Document rollback in evidence
echo "Rollback: $(date) - Reason: <issue>" >> docs/SPEC-949-.../evidence/rollback.log
```

---

## Timeline & Milestones

### Week 1 (Days 1-7, 14-20 hours total)

**Days 1-2 (Phase 1: Model Registration, 4-6h)**:
- Mon AM: Add 5 GPT-5 models to model_provider_info.rs
- Mon PM: Write 5-7 unit tests for model registration
- Tue AM: Run tests, fix any issues
- Tue PM: Code review, commit Phase 1
- **Deliverable**: All 5 models registered, tests passing
- **Validation**: `cargo test -p codex-core model_provider_info`

**Days 3-5 (Phase 2: Config Integration, 6-8h)**:
- Wed AM: Add 5 agent configs to config.toml template
- Wed PM: Update subagent command agent names in handler.rs
- Thu AM: Add agent validation warnings
- Thu PM: Write 6-8 integration tests
- Fri AM: Run tests, validate with /speckit.plan SPEC-900
- Fri PM: Code review, commit Phase 2
- **Deliverable**: GPT-5 agents integrated, subagent commands updated
- **Validation**: `/speckit.plan SPEC-900` uses gpt5_1 in consensus

**Days 6-7 (Phase 3: Provider Stubs, 4-6h)**:
- Sat AM: Implement DeepseekProvider stub
- Sat PM: Implement KimiProvider stub
- Sun AM: Add commented registration, documentation
- Sun PM: Compilation test, commit Phase 3
- **Deliverable**: Future provider stubs ready
- **Validation**: `cargo clippy -p codex-core` passes

**Milestone 1 (End of Week 1)**: GPT-5 models operational, provider stubs ready

---

### Week 2 (Days 8-10, 2-4 hours)

**Days 8-9 (Phase 4: Migration & Documentation, 2-3h)**:
- Mon AM: Write GPT5_MIGRATION_GUIDE.md (200-300 lines)
- Mon PM: Write PROVIDER_SETUP_GUIDE.md (300-400 lines)
- Tue AM: Review documentation, commit Phase 4
- **Deliverable**: User documentation complete
- **Validation**: Peer review of migration guide

**Days 9-10 (Phase 4: Validation, 1-2h)**:
- Tue PM: Run test SPEC (SPEC-900) with GPT-5 agents
- Wed AM: Measure cost, performance metrics
- Wed PM: Write cost_validation.md, commit evidence
- **Deliverable**: Validation evidence captured, cost reduction measured
- **Validation**: Cost $2.30-$2.42 (target $2.36 ± 2.5%)

**Milestone 2 (End of Week 2)**: SPEC-949 implementation complete, validated, ready for SPEC-948 integration

---

## Risk Assessment & Mitigation

### Technical Risks

**Risk 1: GPT-5 Model Names Change**
- **Severity**: Medium
- **Probability**: Medium (OpenAI has history of versioned names like gpt-4-0314)
- **Impact**: Model lookup fails, execution falls back to GPT-4
- **Mitigation**:
  - Use model aliases in config (allows remapping)
  - Subscribe to OpenAI release notes
  - Add model name validation with helpful error messages
- **Contingency**: Quick fix PR to update model names when notified

**Risk 2: ChatGPT API Key Lacks GPT-5 Access**
- **Severity**: High
- **Probability**: Low (GPT-5 public preview confirmed in GitHub Copilot)
- **Impact**: All GPT-5 model calls fail with 403/404 errors
- **Mitigation**:
  - Validate access with test call before full deployment (Phase 4)
  - Graceful fallback to GPT-4 agents
  - Clear error message: "GPT-5 not available, using GPT-4"
- **Contingency**: Keep GPT-4 agents active until GPT-5 access confirmed

**Risk 3: Cost Reduction Not Achieved**
- **Severity**: Medium
- **Probability**: Low (GPT-5.1-mini explicitly cost-optimized)
- **Impact**: Budget impact neutral or negative
- **Mitigation**:
  - Measure actual costs in Phase 4 validation
  - Compare multiple test SPECs (n≥3)
  - If cost higher, revert to GPT-4 for high-volume stages
- **Contingency**: Partial rollback (keep GPT-5 for premium stages only)

### Integration Risks

**Risk 1: Subagent Command Routing Breaks**
- **Severity**: High
- **Probability**: Low (simple name changes)
- **Impact**: Stage execution fails, pipeline halts
- **Mitigation**:
  - Comprehensive integration tests (Phase 2)
  - Manual testing with /speckit.plan before deployment
  - Graceful fallback to default agents if routing fails
- **Contingency**: Immediate rollback of handler.rs changes

**Risk 2: SPEC-948 Integration Issues**
- **Severity**: Medium
- **Probability**: Low (SPEC-948 uses existing agent system)
- **Impact**: New pipeline logic can't use GPT-5 models
- **Mitigation**:
  - SPEC-949 deploys first, validates standalone
  - SPEC-948 testing includes GPT-5 agents from day 1
  - Integration test: Run SPEC-948 pipeline with GPT-5 agents
- **Contingency**: SPEC-948 can proceed with GPT-4 agents if needed

---

## Success Criteria

### Phase-Level Criteria

**Phase 1 Success**:
1. All 5 GPT-5 models registered in model_provider_info.rs
2. Model lookup by name returns correct provider, timeout values
3. 5-7 unit tests passing (100% pass rate maintained)
4. No compilation warnings

**Phase 2 Success**:
1. 5 agent configs defined in config.toml template
2. Subagent commands use new agent names (7 commands updated)
3. Agent validation warnings work correctly
4. 6-8 integration tests passing
5. Manual test: /speckit.plan uses gpt5_1 in multi-agent consensus

**Phase 3 Success**:
1. DeepseekProvider stub compiles without errors
2. KimiProvider stub compiles without errors
3. Stubs marked with `#[allow(dead_code)]`
4. Commented registration shows clear activation path

**Phase 4 Success**:
1. Migration guide complete (200-300 lines, peer-reviewed)
2. Provider setup guide complete (300-400 lines)
3. At least 1 test SPEC executed with GPT-5 models
4. Cost reduction measured: -10% to -15% (target -13%)
5. Performance improvement observed: 1.5-2× faster on simple stages

### Overall SPEC Criteria

1. **Phases Complete**: All 4 phases 100% complete
2. **Tests Passing**: 100% pass rate maintained (604+ existing tests + 17-21 new tests)
3. **Cost Target Met**: $2.30-$2.42 per /speckit.auto run (target $2.36)
4. **Performance Validated**: 50-100% faster on single-agent stages
5. **Documentation Complete**: 2 guides written, 1 validation report
6. **No Regressions**: Existing functionality unchanged (backward compatible)
7. **Evidence Captured**: Validation telemetry, cost tracking, performance benchmarks

---

## Documentation Requirements

### User-Facing Documentation

1. **GPT-5 Migration Guide** (`docs/spec-kit/GPT5_MIGRATION_GUIDE.md`):
   - Model overview (5 models, use cases, pricing)
   - Migration steps (config update, testing, rollback)
   - Model mapping (GPT-4 → GPT-5 equivalents)
   - Performance expectations (2-3× faster, 24h caching)
   - Troubleshooting (common issues, error messages)

2. **Provider Setup Guide** (`docs/spec-kit/PROVIDER_SETUP_GUIDE.md`):
   - GPT-5 setup (already active, validation steps)
   - Deepseek activation (uncomment stub, API key, testing)
   - Kimi activation (similar to Deepseek)
   - Custom provider addition (how to extend ProviderRegistry)
   - Troubleshooting OpenAI-compatible APIs

3. **Cost Validation Report** (`docs/SPEC-949-.../evidence/cost_validation.md`):
   - Before/after cost comparison
   - Per-stage cost breakdown
   - Total pipeline cost reduction
   - Performance metrics (duration, adaptive reasoning speedup)
   - Caching effectiveness measurement

### Developer Documentation

1. **Inline Code Comments**:
   - Provider stub documentation (status, activation steps)
   - Model registration rationale (why these timeout values)
   - Agent selection logic (why gpt5_1_mini for Tier 1)

2. **README Update** (`codex-rs/core/README.md` or project README):
   - Add GPT-5 family to supported models section
   - Update provider list (mention Deepseek/Kimi stubs)
   - Link to PROVIDER_SETUP_GUIDE.md

3. **CHANGELOG Entry**:
   - SPEC-949: Extended Model Support
   - Added: GPT-5/5.1 family (5 models)
   - Added: Deepseek/Kimi provider stubs
   - Changed: Default agents use GPT-5.1 variants
   - Performance: 2-3× faster simple stages, 24h caching
   - Cost: -13% per /speckit.auto run ($2.71 → $2.36)

---

**SPEC-949-IMPL Status**: Ready for implementation
**Estimated Total Effort**: 16-24 hours (1-1.5 weeks)
**Next SPEC**: SPEC-948-IMPL (Modular Pipeline Logic)
