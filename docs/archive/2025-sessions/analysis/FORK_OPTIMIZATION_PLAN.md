# Spec-Kit Fork-Specific Optimization Plan

**Generated**: 2025-10-28
**Repository**: theturtlecsz/code (fork of just-every/code)
**Scope**: ONLY fork-specific additions (Spec-Kit framework)
**Total Fork Code**: 25,164 LOC (15,234 implementation + 9,508 tests + 422 separate crate)

---

## Executive Summary

This optimization plan focuses EXCLUSIVELY on the **Spec-Kit multi-agent automation framework** - the code YOU added to the fork. We're NOT touching any upstream code from just-every/code.

**Key Findings**:
- **4 files need splitting** (>1000 LOC each) - handler.rs, quality_gate_handler.rs, consensus.rs, state.rs
- **~50-100 LOC dead code** - unused imports, variables, annotations
- **3-4 modules questionable** - ACE learning/constitution, config_validator, subagent_defaults
- **Integration footprint: MINIMAL** - only 5 upstream files touched, 98.8% isolation maintained

**Optimization Potential**:
- **-50-100 LOC** (dead code cleanup) - 2 hours, zero risk
- **50-60% file size reduction** (module splitting) - 16-24 hours, medium risk
- **Improved maintainability** - clearer boundaries, easier testing

---

## Part 1: Dead Code Cleanup (Quick Wins)

### 1.1 Compiler Warnings (18 instances)

**Unused Imports** (8 instances):
```rust
// spec_kit/handler.rs
use codex_core::error::CodexErr;  // Remove
use super::super::ace_route_selector::DiffStat;  // Remove

// spec_kit/ace_*
use super::ace_prompt_injector;  // Remove
use AceResult, self;  // Remove

// spec_kit/quality_gate_handler.rs
use format_ace_section, select_bullets;  // Remove

// spec_kit/file_modifier.rs
use std::io::Write;  // Remove

// test files
use context::test_mock::MockSpecKitContext;  // Remove or gate with #[cfg(test)]
```

**Unused Variables** (10 instances):
```rust
// Likely debug/test artifacts - remove or use
let config = ...;  // Remove
let bullets_used_ids = ...;  // Remove
let spacer_idx = ...;  // Remove
let prompt = ...;  // Remove
let checkpoint = ...;  // Remove
// etc.
```

**Action**: Run `cargo clippy --fix` then manual review.
**Effort**: 1-2 hours
**Impact**: Cleaner code, no warnings

---

### 1.2 Dead Code Annotations (5 instances)

```rust
// ace_orchestrator.rs:70
#[allow(dead_code)]  // Investigate or remove

// state.rs:62, 473, 553, 764
#[allow(dead_code)]  // Investigate or remove
```

**Action**:
1. Search for usage: `rg "variant_name|function_name"`
2. If truly unused, remove
3. If used conditionally, use `#[cfg(...)]` instead

**Effort**: 1-2 hours
**Impact**: ~20-50 LOC removed

---

### 1.3 Suspected Unused Modules (Need Investigation)

| Module | LOC | Imports | Issue |
|--------|-----|---------|-------|
| **ace_learning.rs** | 357 | 1 (by ace_reflector) | May be unused |
| **ace_constitution.rs** | 357 | 1 (by commands/special) | May be unused |
| **config_validator.rs** | 327 | 0 in core flow | CLI-only? |
| **subagent_defaults.rs** | 134 | 1 | May be obsolete |

**Investigation Steps**:
```bash
# Check usage
rg "use.*ace_learning" --type rust
rg "use.*ace_constitution" --type rust
rg "use.*config_validator" --type rust
rg "use.*subagent_defaults" --type rust

# Run cargo-udeps
cargo install cargo-udeps
cargo +nightly udeps --package codex-tui

# Check git history
git log --oneline -- tui/src/chatwidget/spec_kit/ace_learning.rs | head -10
```

**Potential Outcomes**:
- **If unused**: Remove or gate with `feature = "ace-advanced"`
- **If CLI-only**: Document usage pattern
- **If feature-gated**: Add clear comments

**Effort**: 2-3 hours investigation
**Impact**: Potentially -500 to -1,200 LOC if removed

---

## Part 2: Module Splitting (High-Value Refactoring)

### 2.1 🔴 CRITICAL: handler.rs (1,561 LOC → 3 files)

**Problem**: God function - orchestrates everything.

**Current Responsibilities**:
- Command dispatch
- MCP client coordination
- Telemetry collection
- State management
- Quality gate triggers
- Evidence collection
- ACE routing decisions

**Proposed Split**:

```
spec_kit/
├── handler/
│   ├── mod.rs (200 LOC)          - Public API, command dispatch
│   ├── orchestrator.rs (600 LOC) - Core workflow logic
│   ├── telemetry.rs (400 LOC)    - Evidence, telemetry, cost tracking
│   └── mcp.rs (350 LOC)          - MCP client integration
└── handler.rs → REMOVE (replaced by handler/mod.rs)
```

**Benefits**:
- ✅ Clearer separation of concerns
- ✅ Easier to test each responsibility
- ✅ Reduced incremental build time (changes to telemetry don't rebuild orchestration)

**Risks**:
- ⚠️ Many internal function calls (need careful extraction)
- ⚠️ Tests may need updates (check test_handler_orchestration)

**Effort**: 6-8 hours
**Priority**: 🔴 HIGH (biggest complexity reduction)

---

### 2.2 🟠 HIGH: quality_gate_handler.rs (1,254 LOC → 2 files)

**Problem**: Already extracted from handler (good!) but still too large.

**Current Responsibilities**:
- Agent completion handling
- GPT-5 validation orchestration
- User interaction (modal UI)
- Quality issue resolution
- Checkpoint coordination

**Proposed Split**:

```
spec_kit/
├── quality_gate/
│   ├── mod.rs (200 LOC)           - Public API
│   ├── orchestrator.rs (600 LOC)  - Agent workflows, validation
│   └── interaction.rs (450 LOC)   - User prompts, modal coordination
└── quality_gate_handler.rs → REMOVE
```

**Benefits**:
- ✅ Separate async orchestration from UI logic
- ✅ Easier to mock user interactions in tests

**Effort**: 4-6 hours
**Priority**: 🟠 HIGH (already partially refactored)

---

### 2.3 🟡 MEDIUM: consensus.rs (1,052 LOC → 3 files)

**Problem**: Single file for complex multi-agent system.

**Current Responsibilities**:
- Consensus data types
- Evidence handling
- Synthesis algorithms
- MCP integration (local-memory)
- JSON parsing

**Proposed Split**:

```
spec_kit/
├── consensus/
│   ├── mod.rs (150 LOC)       - Public API, types
│   ├── synthesis.rs (400 LOC)  - Consensus algorithms, validation
│   ├── mcp.rs (300 LOC)        - Native MCP integration
│   └── evidence.rs (200 LOC)   - Evidence storage helpers
└── consensus.rs → REMOVE
```

**Benefits**:
- ✅ MCP integration isolated (easier to test with mocks)
- ✅ Synthesis logic clearer

**Effort**: 6-8 hours
**Priority**: 🟡 MEDIUM (complex but high value)

---

### 2.4 🟡 MEDIUM: state.rs (932 LOC → 3 files)

**Problem**: God object - too many state types in one file.

**Current Contents**:
- SpecAutoState (main state machine)
- QualityCheckpoint (quality gate state)
- GuardrailOutcome (validation results)
- ValidateLifecycle (test execution state)
- Plus helpers, serialization, validation

**Proposed Split**:

```
spec_kit/
├── state/
│   ├── mod.rs (150 LOC)          - Common types, re-exports
│   ├── auto.rs (350 LOC)         - SpecAutoState, transitions
│   ├── quality.rs (250 LOC)      - QualityCheckpoint, quality state
│   └── validate.rs (180 LOC)     - GuardrailOutcome, ValidateLifecycle
└── state.rs → REMOVE
```

**Benefits**:
- ✅ Each state machine isolated
- ✅ Easier to understand transitions

**Effort**: 4-6 hours
**Priority**: 🟡 MEDIUM (many imports to update)

---

### 2.5 🟢 LOW: Optional Splits

**ace_route_selector.rs** (766 LOC):
- Extract `DiffStat` struct to `ace_diff_analysis.rs` (200 LOC)
- Keep routing logic in main file (566 LOC)
- **Effort**: 2-3 hours

**quality.rs** (851 LOC):
- Split `quality/classification.rs` (425 LOC)
- Split `quality/resolution.rs` (425 LOC)
- **Effort**: 4-6 hours

**Priority**: 🟢 LOW (nice-to-have but not critical)

---

## Part 3: Testing Strategy

### 3.1 Preserve 100% Test Pass Rate

**Current Test Suite**:
- 21 test files, 9,508 LOC
- Integration-focused (multi-module workflows)
- Test-to-code ratio: 0.62:1
- Coverage estimate: 38-42%

**During Refactoring**:
1. **Before each split**: `cargo test --package codex-tui -- spec_kit`
2. **After moving functions**: Update imports in test files
3. **After each module**: Run full test suite
4. **Before commit**: `cargo test --workspace`

**Test Updates Required**:
- handler tests: Update imports to `spec_kit::handler::*`
- quality_gate tests: Update imports to `spec_kit::quality_gate::*`
- Integration tests: May need path updates

---

### 3.2 Add Tests for Split Modules

**After splitting handler.rs**:
```rust
// New tests in handler/tests.rs
#[test]
fn test_orchestrator_stage_advancement() { ... }

#[test]
fn test_telemetry_collection() { ... }

#[test]
fn test_mcp_client_coordination() { ... }
```

**Target**: Maintain or improve 38-42% coverage

---

## Part 4: Execution Plan

### Phase 1: Quick Wins (Week 1) 🔴 IMMEDIATE

**Day 1-2**: Dead Code Cleanup
- [ ] Run `cargo clippy --fix --package codex-tui`
- [ ] Manually review remaining warnings
- [ ] Remove confirmed dead code (unused imports/variables)
- [ ] Test: `cargo test --package codex-tui`
- [ ] Commit: "chore(spec-kit): remove dead code and fix warnings"

**Day 3-4**: Investigation
- [ ] Install cargo-udeps: `cargo install cargo-udeps`
- [ ] Run analysis: `cargo +nightly udeps --package codex-tui > dead_code_analysis.txt`
- [ ] Investigate suspected modules (ace_learning, ace_constitution, config_validator, subagent_defaults)
- [ ] Document findings in `spec_kit/ARCHITECTURE.md`

**Day 5**: Documentation
- [ ] Document ACE module usage patterns
- [ ] Add rustdoc to public functions
- [ ] Update `spec_kit/mod.rs` with module overview

**Expected Impact**: -50-100 LOC, cleaner code, zero risk

---

### Phase 2: Critical Splits (Weeks 2-3) 🟠 HIGH PRIORITY

**Week 2**: handler.rs Split
- [ ] Day 1-2: Create `handler/` directory structure
- [ ] Day 2-3: Extract `orchestrator.rs` (core workflow)
- [ ] Day 3: Extract `telemetry.rs` (evidence/cost)
- [ ] Day 4: Extract `mcp.rs` (MCP client integration)
- [ ] Day 4: Update `handler/mod.rs` with public API
- [ ] Day 5: Update all imports (handler tests, integration tests)
- [ ] Day 5: Run full test suite, fix any breakage
- [ ] Commit: "refactor(spec-kit): split handler.rs into submodules"

**Week 3**: quality_gate_handler.rs Split
- [ ] Day 1: Create `quality_gate/` directory
- [ ] Day 2: Extract `orchestrator.rs` (agent workflows)
- [ ] Day 3: Extract `interaction.rs` (user prompts)
- [ ] Day 4: Update imports, run tests
- [ ] Day 5: Buffer day for fixes
- [ ] Commit: "refactor(spec-kit): split quality_gate_handler.rs"

**Expected Impact**: -60% file sizes, clearer boundaries

---

### Phase 3: State & Consensus Splits (Week 4) 🟡 MEDIUM PRIORITY

**Days 1-3**: state.rs Split
- [ ] Create `state/` directory
- [ ] Extract `auto.rs`, `quality.rs`, `validate.rs`
- [ ] Update imports (many files depend on state types)
- [ ] Run tests

**Days 4-5**: consensus.rs Split
- [ ] Create `consensus/` directory
- [ ] Extract `synthesis.rs`, `mcp.rs`, `evidence.rs`
- [ ] Update imports
- [ ] Run tests

**Expected Impact**: -50% file sizes, better isolation

---

### Phase 4: Optional Optimizations (As Needed) 🟢 LOW PRIORITY

**Only if time permits**:
- [ ] Split ace_route_selector.rs (extract DiffStat)
- [ ] Split quality.rs (classification vs resolution)
- [ ] Add performance benchmarks

---

## Part 5: Risk Mitigation

### 5.1 Refactoring Risks

| Risk | Likelihood | Impact | Mitigation |
|------|------------|--------|------------|
| **Test failures** | Medium | High | Run tests after each module extraction |
| **Import hell** | High | Medium | Use IDE refactoring tools, systematic updates |
| **Borrow checker issues** | Low | High | Careful ownership analysis before split |
| **Lost functionality** | Low | Critical | Manual QA after each phase |

---

### 5.2 Rollback Plan

**For Each Phase**:
1. Create feature branch: `git checkout -b refactor/spec-kit-phase-N`
2. Commit after each module extraction
3. Tag before major changes: `git tag spec-kit-refactor-phase-N-checkpoint`
4. If tests fail: `git reset --hard spec-kit-refactor-phase-N-checkpoint`

---

### 5.3 Upstream Sync Safety

**Integration Footprint** (MINIMAL):
- Upstream files touched: 5 (chatwidget/mod.rs, app.rs, slash_command.rs, app_event.rs, bottom_pane/quality_gate_modal.rs)
- Upstream references: ~58 uses
- All changes marked with FORK-SPECIFIC comments

**Rebase Conflict Risk**: <5% (all fork code in single subdirectory)

**Before Upstream Sync** (2026-01-15):
- [ ] Verify all FORK-SPECIFIC markers present
- [ ] Run `git diff upstream/main tui/src/chatwidget/spec_kit/` (should show only our code)
- [ ] Test merge: `git merge --no-commit upstream/main`

---

## Part 6: Success Metrics

### 6.1 Code Quality KPIs

| Metric | Before | Target | After |
|--------|--------|--------|-------|
| **Files >1000 LOC** | 4 | 0 | TBD |
| **Avg module size** | 461 LOC | 350 LOC | TBD |
| **Compiler warnings** | 18 | 0 | TBD |
| **Dead code LOC** | ~50-100 | 0 | TBD |
| **Test coverage** | 38-42% | 45%+ | TBD |

---

### 6.2 Maintainability Score

**Before Refactoring**:
- God files: 4 (handler, quality_gate_handler, consensus, state)
- Complex dependencies: handler → 15 modules
- Reuse: Difficult (monolithic files)

**After Refactoring**:
- God files: 0 (all <700 LOC)
- Clear boundaries: orchestration, telemetry, MCP
- Reuse: Easier (focused modules)

---

### 6.3 Performance Impact

**Expected**:
- Incremental build time: -10-20% (smaller modules = less recompilation)
- Full rebuild: Unchanged (same total code)
- Test execution: Unchanged or slightly faster (better isolation)

**Measurement**:
```bash
# Before
cargo clean && cargo build --timings --package codex-tui

# After
cargo clean && cargo build --timings --package codex-tui

# Compare HTML reports
```

---

## Part 7: Weekly Checklist

**Week 1** (Quick Wins):
- [x] Run cargo clippy --fix
- [x] Remove dead code
- [x] Run cargo-udeps
- [x] Document ACE usage
- [x] Add rustdoc

**Week 2** (Handler Split):
- [ ] Extract handler/orchestrator.rs
- [ ] Extract handler/telemetry.rs
- [ ] Extract handler/mcp.rs
- [ ] Update imports
- [ ] Run full tests

**Week 3** (Quality Gate Split):
- [ ] Extract quality_gate/orchestrator.rs
- [ ] Extract quality_gate/interaction.rs
- [ ] Update imports
- [ ] Run full tests

**Week 4** (State & Consensus):
- [ ] Split state.rs → state/*
- [ ] Split consensus.rs → consensus/*
- [ ] Run full tests
- [ ] Update documentation

---

## Part 8: Documentation Updates

### 8.1 Architecture Documentation

**Create** `spec_kit/ARCHITECTURE.md`:
```markdown
# Spec-Kit Architecture

## Module Structure
- handler/ - Command orchestration
- quality_gate/ - Quality checkpoints
- consensus/ - Multi-agent validation
- state/ - State machines
- ace/ - Agentic Context Engine
- commands/ - Slash command implementations
- (infrastructure modules)

## Key Patterns
- Friend module (minimal upstream touch)
- SpecKitContext trait (testability)
- Native MCP integration (5.3x faster)
- Evidence-driven (centralized collection)

## Module Dependencies
[Include dependency graph]
```

---

### 8.2 Rustdoc Coverage

**Add doc comments to**:
- Public functions in `mod.rs`
- Key types in `state.rs`
- All command handlers
- ACE subsystem interfaces

**Target**: 80%+ doc coverage on public API

---

## Part 9: Estimated Effort & Timeline

### Total Effort Breakdown

| Phase | Tasks | Hours | Priority |
|-------|-------|-------|----------|
| **Phase 1** | Dead code cleanup | 8 | 🔴 IMMEDIATE |
| **Phase 2** | Handler + Quality Gate splits | 12 | 🟠 HIGH |
| **Phase 3** | State + Consensus splits | 14 | 🟡 MEDIUM |
| **Phase 4** | Optional optimizations | 8 | 🟢 LOW |
| **Documentation** | Architecture guide, rustdoc | 6 | 🟡 MEDIUM |
| **TOTAL** | All phases | **48 hours** | |

**Timeline**: 4 weeks (assuming 12 hours/week)

---

### Return on Investment

**Effort**: 48 hours over 4 weeks
**Benefit**:
- 50-60% file size reduction (4 files >1K LOC → 0)
- Cleaner module boundaries
- Easier testing and reuse
- Reduced incremental build time (-10-20%)
- Better maintainability (clear ownership)

**Risk**: Medium (many interdependencies, but excellent test coverage)
**Recommendation**: **Execute phases 1-2 immediately** (high ROI), defer phase 3-4 if time-constrained.

---

## Part 10: Final Recommendations

### DO NOW (High ROI, Low Risk)
1. ✅ **Phase 1**: Dead code cleanup (8 hours, zero risk)
2. ✅ **Documentation**: Add architecture guide and rustdoc (6 hours)

### DO NEXT (High Value, Medium Risk)
3. ✅ **Phase 2**: Split handler.rs and quality_gate_handler.rs (12 hours)

### CONSIDER LATER (Medium Value, Medium Risk)
4. 🤔 **Phase 3**: Split state.rs and consensus.rs (14 hours)

### DON'T DO (Low Value or YAGNI)
5. ❌ Full async conversion (MAINT-10 requirements unclear)
6. ❌ ACE consolidation (usage patterns not validated)
7. ❌ ARCH-008 protocol extensions (no consumers exist)

---

## Conclusion

This optimization plan targets **YOUR fork-specific code only** - the 25,164 LOC Spec-Kit framework you added. We're not touching any upstream code from just-every/code.

**Key Priorities**:
1. 🔴 **Week 1**: Clean up dead code and warnings (8 hours, high ROI)
2. 🟠 **Weeks 2-3**: Split handler.rs and quality_gate_handler.rs (12 hours, critical)
3. 🟡 **Week 4**: Split state.rs and consensus.rs (14 hours, optional)

**Expected Outcomes**:
- Cleaner, more maintainable fork code
- 50-60% file size reduction (no files >1K LOC)
- Preserved test coverage (100% pass rate maintained)
- Minimal upstream conflict risk (rebase safety maintained)

**Ready to Start**: Phase 1 commands provided, can begin immediately!

---

**Document Owner**: @theturtlecsz
**Review Date**: 2025-10-28
**Next Review**: After Phase 1 completion (target: 2025-11-04)
