# SPEC-KIT FORK-SPECIFIC CODE ANALYSIS
# Comprehensive inventory of fork additions to codex-rs

## 1. SPEC-KIT MODULE INVENTORY (33 modules, 15,234 LOC)

### Root Modules (27 files)
handler.rs                    1,561 LOC  - Main command orchestration
quality_gate_handler.rs       1,254 LOC  - Quality gate workflow (extracted from handler)
consensus.rs                  1,052 LOC  - Multi-agent consensus validation
state.rs                        932 LOC  - State management and types
quality.rs                      851 LOC  - Quality issue classification
ace_route_selector.rs          766 LOC  - ACE routing decisions (complexity-based)
evidence.rs                     691 LOC  - Evidence collection/storage
guardrail.rs                    672 LOC  - Shell script integration
file_modifier.rs                554 LOC  - File operations (SPEC.md updates)
cost_tracker.rs                 537 LOC  - Budget tracking (SPEC-KIT-070)
command_registry.rs             537 LOC  - Command routing registry
ace_client.rs                   460 LOC  - ACE MCP client
quality_gate_broker.rs          433 LOC  - Quality gate coordination
ace_prompt_injector.rs          417 LOC  - ACE bullet injection
ace_learning.rs                 357 LOC  - Learning from execution
ace_constitution.rs             357 LOC  - Constitution pinning
context.rs                      349 LOC  - SpecKitContext trait (testability)
ace_curator.rs                  333 LOC  - Strategic playbook management
config_validator.rs             327 LOC  - Config validation
ace_orchestrator.rs             318 LOC  - Full ACE reflection cycle
ace_reflector.rs                317 LOC  - Deep outcome analysis
error.rs                        279 LOC  - Error types
routing.rs                      205 LOC  - Command dispatch
schemas.rs                      197 LOC  - JSON schema validation
spec_id_generator.rs            189 LOC  - Native SPEC-ID generation (cost opt)
subagent_defaults.rs            134 LOC  - Default subagent configs
mod.rs                          102 LOC  - Public API surface

### Command Submodules (6 files)
commands/special.rs             393 LOC  - /speckit.constitution, /speckit.ace-status
commands/guardrail.rs           308 LOC  - /guardrail.* commands
commands/plan.rs                193 LOC  - /speckit.plan
commands/quality.rs             103 LOC  - /speckit.clarify, analyze, checklist
commands/status.rs               37 LOC  - /speckit.status
commands/mod.rs                  19 LOC  - Command module exports

TOTAL: 15,234 LOC across 33 modules

## 2. INTERNAL DEPENDENCY ANALYSIS

### Core Dependencies (Most Referenced)
- error.rs (20 imports) - Foundation error types
- state.rs (6 imports) - State management types
- ace_client.rs (5 imports) - ACE MCP integration

### ACE Subsystem (8 modules, ~3,700 LOC)
ace_orchestrator.rs → ace_reflector, ace_curator, ace_learning, ace_client
ace_route_selector.rs → (standalone - routing logic)
ace_prompt_injector.rs → ace_client
ace_reflector.rs → ace_learning
ace_curator.rs → ace_client, ace_reflector
ace_constitution.rs → ace_client
ace_learning.rs → (standalone - data types)
ace_client.rs → (leaf - MCP interface)

Purpose: Agentic Context Engine for complex task routing and playbook learning

### Consensus Subsystem (2 modules, ~1,900 LOC)
consensus.rs → error, state
quality.rs → state, consensus (quality issue classification)

Purpose: Multi-agent consensus validation and synthesis

### Quality Gate Subsystem (2 modules, ~1,700 LOC)
quality_gate_handler.rs → state, quality_gate_broker
quality_gate_broker.rs → state

Purpose: Interactive quality checkpoint workflow

### Command Routing (3 modules, ~1,100 LOC)
handler.rs → ALL modules (orchestrator)
routing.rs → command_registry
command_registry.rs → commands/*

Purpose: Slash command dispatch and execution

### Supporting Infrastructure (6 modules, ~2,800 LOC)
evidence.rs → error
guardrail.rs → error, state
file_modifier.rs → error
cost_tracker.rs → (standalone)
spec_id_generator.rs → (standalone)
config_validator.rs → (standalone)
context.rs → error, state (testability trait)
schemas.rs → state (JSON validation)

Purpose: Evidence storage, shell integration, file ops, validation

## 3. INTEGRATION SURFACE AREA

### Upstream Touchpoints (Minimal by design)
tui/src/chatwidget/mod.rs:
  - Line 3: `pub mod spec_kit;` (single integration point)
  - Line 31-42: Import spec_kit types (consensus, state)
  - Line 102-106: FORK-SPECIFIC mcp_manager (shared instance)
  
tui/src/app.rs:
  - Line 3: `use crate::chatwidget::spec_kit;`
  - Line 102-106: FORK-SPECIFIC mcp_manager (shared with ChatWidget)
  
tui/src/slash_command.rs:
  - Lines 118-204: FORK-SPECIFIC enum variants (26 spec-kit commands)
  - Lines 240-296: FORK-SPECIFIC descriptions
  - Lines 326-352: is_prompt_expanding, requires_arguments logic

tui/src/app_event.rs:
  - Import spec_kit types for event handling

tui/src/bottom_pane/quality_gate_modal.rs:
  - Use spec_kit types for quality gate UI

Total upstream references: ~58 uses across 5 files
Strategy: Friend module pattern - spec_kit is submodule of chatwidget

## 4. SEPARATE SPEC-KIT CRATE (4 files, 422 LOC)

spec-kit/src/lib.rs          34 LOC  - Crate exports
spec-kit/src/types.rs       166 LOC  - SpecStage, SpecAgent enums
spec-kit/src/api.rs         134 LOC  - Future async API (MAINT-10)
spec-kit/src/error.rs        88 LOC  - Error types

Purpose: Foundation for CLI/API extraction (MAINT-10)
Status: Types only, no async implementation yet
Dependencies: mcp-types, codex-core (for McpConnectionManager)

## 5. FORK-SPECIFIC TESTS (21 files, 9,508 LOC)

### Integration Tests (14 files, ~7,500 LOC)
workflow_integration_tests.rs          1,083 LOC - Multi-stage workflows
error_recovery_integration_tests.rs      935 LOC - Error handling
spec_auto_e2e.rs                         800 LOC - Full pipeline E2E
consensus_logic_tests.rs                 642 LOC - Consensus validation
quality_gates_integration.rs             476 LOC - Quality checkpoint flows
guardrail_tests.rs                       350 LOC - Shell script integration
quality_resolution_tests.rs              345 LOC - Issue resolution
evidence_tests.rs                        295 LOC - Evidence collection
state_persistence_integration_tests.rs   271 LOC - State management
mcp_consensus_benchmark.rs               270 LOC - Performance benchmarks
mcp_consensus_integration.rs             247 LOC - MCP integration
quality_flow_integration_tests.rs        220 LOC - Quality flows
concurrent_operations_integration.rs     198 LOC - Concurrency safety
spec_status.rs                           138 LOC - Status dashboard

### Unit Tests (3 files, ~800 LOC)
property_based_tests.rs                  [count in integration total]
mock_mcp_tests.rs                        [count in integration total]
handler_orchestration_tests.rs           [count in integration total]

### Test Infrastructure (3 files, ~500 LOC)
common/integration_harness.rs            253 LOC - Test fixtures
common/mock_mcp.rs                       [bundled]
common/mod.rs                            [bundled]

### Smaller Tests (2 files, ~200 LOC)
spec_id_generator_integration.rs          69 LOC - ID generation
edge_case_tests.rs                       [bundled]

Coverage: Estimated 38-42% based on test count vs implementation LOC
Quality: Integration-focused with multi-module workflows

## 6. DEAD CODE CANDIDATES

### Current #[allow(dead_code)] Annotations (5 instances)
ace_orchestrator.rs:70   - Likely unused orchestration variant
state.rs:62              - Possibly unused state enum variant
state.rs:473             - Internal helper
state.rs:553             - Internal helper
state.rs:764             - Possibly unused state type

### Compiler Warnings (Unused Imports/Variables - 18 instances)
UNUSED IMPORTS:
- codex_core::error::CodexErr
- super::super::ace_route_selector::DiffStat
- AceResult, self (ace modules)
- super::ace_prompt_injector
- context::test_mock::MockSpecKitContext
- format_ace_section, select_bullets
- std::io::Write

UNUSED VARIABLES (likely test/debug artifacts):
- config, bullets_used_ids, bullet_texts
- spacer_idx, prompt, scope
- repo_root, branch, cwd
- checkpoint, spec_id, stage

Cleanup Impact: ~50-100 LOC removable, minor complexity reduction

### Potential Redundancies (Needs Investigation)
1. ACE modules - 8 modules but only 3 external imports
   - ace_learning.rs (357 LOC) - only imported by ace_reflector
   - ace_constitution.rs (357 LOC) - only imported by commands/special
   - May be feature-gated or future-use
   
2. subagent_defaults.rs (134 LOC) - only 1 import
   - May be unused after routing refactor
   
3. config_validator.rs (327 LOC) - not imported in core flow
   - Possibly CLI-only or validation harness

Recommendation: Run cargo-udeps and dead-code analysis to confirm

## 7. OPTIMIZATION OPPORTUNITIES

### High-Value Targets

1. **handler.rs (1,561 LOC) - Too Many Responsibilities**
   - Orchestrates all commands
   - MCP integration
   - Telemetry
   - State management
   - Quality gates
   - Suggested: Split into handler_core.rs + handler_telemetry.rs + handler_mcp.rs
   - Estimated impact: 500-600 LOC per file, clearer boundaries

2. **quality_gate_handler.rs (1,254 LOC) - Already Extracted but Still Large**
   - Was extracted from handler.rs (good!)
   - Still handles: agents complete, broker result, validation, cancellation
   - Suggested: Extract broker coordination vs user interaction
   - Estimated impact: 600 LOC each

3. **state.rs (932 LOC) - God Object Syndrome**
   - Contains: SpecAutoState, QualityCheckpoint, GuardrailOutcome, ValidateLifecycle
   - Plus helpers, validation, serialization
   - Suggested: Split into state/auto.rs, state/quality.rs, state/validate.rs
   - Estimated impact: 300-400 LOC per file

4. **consensus.rs (1,052 LOC) - Single File for Complex System**
   - Types, evidence handling, synthesis, MCP integration
   - Suggested: consensus/types.rs, consensus/synthesis.rs, consensus/mcp.rs
   - Estimated impact: 350 LOC each

### Medium-Value Targets

5. **ace_route_selector.rs (766 LOC) - Complex Logic**
   - Route selection, diff analysis, decision tree
   - Suggested: Extract DiffStat to separate module
   - Estimated impact: 500 + 266 LOC

6. **quality.rs (851 LOC) - Issue Classification**
   - Multiple classification algorithms
   - Suggested: quality/classification.rs + quality/resolution.rs
   - Estimated impact: 425 LOC each

### Cost-Benefit Analysis

High-impact splits (handler, quality_gate_handler, state, consensus):
- Effort: 16-24 hours (careful extraction to preserve tests)
- Benefit: 60% file size reduction, clearer module boundaries
- Risk: Medium (many internal dependencies)

Medium-impact splits (ace_route_selector, quality):
- Effort: 8-12 hours
- Benefit: 40% file size reduction
- Risk: Low (more isolated)

## 8. ARCHITECTURE PATTERNS

### Strengths
1. **Isolation Strategy**: Friend module pattern minimizes upstream conflicts
2. **Testability**: SpecKitContext trait enables unit testing (MAINT-3)
3. **Evidence Collection**: Centralized through evidence.rs
4. **Error Handling**: Consistent SpecKitError type
5. **MCP Integration**: Native integration (5.3x faster than subprocess)

### Anti-Patterns
1. **God Functions**: handler.rs orchestrates too much
2. **Mega Files**: 4 files >1000 LOC each
3. **Deep Nesting**: Some modules 3-4 levels deep (commands/special.rs → ace_constitution → ace_client)
4. **Implicit Dependencies**: Handler depends on nearly all modules

### Rebase Safety
EXCELLENT - Fork code is isolated in:
- Single subdirectory: `tui/src/chatwidget/spec_kit/`
- Marked integration points: FORK-SPECIFIC comments
- Minimal upstream changes: ~58 references across 5 files
- No upstream function modifications (only additions)

Rebase conflict surface: <5% of codebase changes

## 9. SUMMARY METRICS

### Code Volume
Total Fork-Specific Code: 25,164 LOC
- spec_kit TUI modules: 15,234 LOC (60.5%)
- Fork-specific tests: 9,508 LOC (37.8%)
- Separate spec-kit crate: 422 LOC (1.7%)

### Module Breakdown
- ACE subsystem: 3,700 LOC (24%)
- Handler/orchestration: 2,800 LOC (18%)
- Consensus/quality: 2,800 LOC (18%)
- State management: 932 LOC (6%)
- Supporting infrastructure: 2,800 LOC (18%)
- Commands: 1,053 LOC (7%)
- Core types/errors: 650 LOC (4%)
- Public API: 500 LOC (3%)

### Integration Footprint
Upstream files touched: 5
Upstream references: ~58
Integration pattern: Friend submodule
Rebase risk: LOW

### Test Coverage
Test files: 21
Test LOC: 9,508
Implementation LOC: 15,234
Test-to-code ratio: 0.62:1
Coverage estimate: 38-42%
Test focus: Integration > Unit

### Optimization Potential
Files >1000 LOC: 4 (handler, quality_gate_handler, consensus, state)
Dead code candidates: ~50-100 LOC
Unused warnings: 18
Splitting opportunity: 16-36 hours effort for 50% size reduction

### Future Work
MAINT-10: Extract CLI/API crate (spec-kit foundation ready)
SPEC-KIT-072: Consensus DB separation from local-memory
Cost optimization: Native tools > AI consensus (spec_id_generator pattern)

## 10. RECOMMENDATIONS

### Immediate Actions (High Value, Low Risk)
1. Fix compiler warnings (18 instances) - 1-2 hours
2. Remove confirmed dead code - 1-2 hours
3. Document ACE module usage (clarify if unused or feature-gated) - 2 hours

### Short-term Refactors (High Value, Medium Risk)
1. Split handler.rs into 3 files - 6-8 hours
2. Split state.rs into 3 files - 4-6 hours
3. Run cargo-udeps to confirm unused dependencies - 1 hour

### Long-term Architecture (Medium Value, High Risk)
1. Split consensus.rs into submodules - 6-8 hours
2. Split quality_gate_handler.rs - 4-6 hours
3. Extract DiffStat from ace_route_selector - 2-3 hours

### DO NOT DO (Low Value or Premature)
1. ARCH-008 protocol extensions (YAGNI - no consumers)
2. Full async conversion before MAINT-10 requirements clear
3. ACE module consolidation without understanding usage patterns

Total estimated effort for high-value optimizations: 16-24 hours
Expected benefit: 50-60% file size reduction, clearer boundaries, easier maintenance
Risk level: Medium (preserve test coverage during splits)
