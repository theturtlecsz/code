# Testing Strategy

Comprehensive testing approach for the codebase.

---

## Overview

**Testing Philosophy**: Balance coverage, confidence, and development velocity

**Current Metrics** (as of 2025-11-17):
- **Total Tests**: 604 tests across all modules
- **Pass Rate**: 100% (all tests passing)
- **Coverage**: 42-48% (estimated, varies by module)
- **Target**: 40%+ coverage minimum

**Test Distribution**:
- **Unit Tests**: ~380 tests (63%)
- **Integration Tests**: ~200 tests (33%)
- **E2E Tests**: ~24 tests (4%)

**Location**: Tests located alongside source in `tests/` directories per module

---

## Coverage Goals

### Overall Target: 40%+

**Rationale**:
- Industry standard for Rust projects: 60-80%
- Our target: 40%+ given complexity and time constraints
- Current achievement: 42-48% ✅ **Target Met**

**Coverage by Priority**:
- **Critical paths**: 70-80% (Spec-Kit automation, MCP client)
- **Core functionality**: 50-60% (TUI, database, config)
- **Supporting code**: 30-40% (utilities, helpers)
- **Legacy code**: 20-30% (minimal coverage acceptable)

---

### Module-Specific Targets

| Module | Priority | Target Coverage | Current Est. | Status |
|--------|----------|----------------|--------------|--------|
| **codex-tui/spec_kit** | Critical | 70% | ~75% | ✅ Exceeded |
| **codex-mcp-client** | Critical | 70% | ~65% | 🔄 Near target |
| **codex-tui** | High | 50% | ~45% | 🔄 Near target |
| **codex-core** | High | 50% | ~50% | ✅ Met |
| **codex-db** | High | 50% | ~60% | ✅ Exceeded |
| **config-loader** | Medium | 40% | ~55% | ✅ Exceeded |
| **file-search** | Medium | 40% | ~40% | ✅ Met |
| **utilities** | Low | 30% | ~35% | ✅ Met |

**Overall Status**: ✅ **42-48% coverage achieved** (exceeds 40% target)

---

## Testing Pyramid

### Level 1: Unit Tests (~63%)

**Purpose**: Test individual functions/components in isolation

**Characteristics**:
- Fast execution (<1s for all unit tests)
- No external dependencies (mocked)
- High volume (~380 tests)

**What to Unit Test**:
- ✅ Pure functions (input → output, no side effects)
- ✅ Business logic (validation, parsing, calculations)
- ✅ Data structures (serialization, deserialization)
- ✅ Error handling (edge cases, invalid inputs)

**What NOT to Unit Test**:
- ❌ Integration points (use integration tests)
- ❌ UI rendering (hard to test, low ROI)
- ❌ External APIs (mock in integration tests)

**Example Coverage**:
```
spec_kit/clarify_native.rs: 85% (pattern matching logic)
spec_kit/checklist_native.rs: 90% (scoring algorithms)
mcp-client/protocol.rs: 75% (JSON-RPC parsing)
```

---

### Level 2: Integration Tests (~33%)

**Purpose**: Test multiple modules working together

**Characteristics**:
- Moderate execution time (1-10s per test)
- Real module interactions (no mocks between modules)
- Medium volume (~200 tests)

**What to Integration Test**:
- ✅ Workflow orchestration (plan → tasks → implement)
- ✅ Cross-module communication (TUI ↔ MCP client)
- ✅ State persistence (database writes/reads)
- ✅ Error propagation across modules

**Example Coverage**:
```
spec_kit/workflow_integration_tests.rs: 60 tests
mcp_client/integration_tests.rs: 45 tests
database/integration_tests.rs: 40 tests
```

---

### Level 3: E2E Tests (~4%)

**Purpose**: Test complete user workflows end-to-end

**Characteristics**:
- Slow execution (10-60s per test)
- Full stack (TUI + backend + database + MCP)
- Low volume (~24 tests, high value)

**What to E2E Test**:
- ✅ Critical user journeys (`/speckit.auto` full pipeline)
- ✅ Error recovery (retry logic, degradation)
- ✅ Tmux session management
- ✅ Configuration hot-reload

**Example Coverage**:
```
spec_kit/e2e_tests.rs: 12 tests (full automation)
tmux/e2e_tests.rs: 8 tests (session lifecycle)
config/e2e_tests.rs: 4 tests (hot-reload)
```

---

## Test Organization

### Per-Module Tests

**Structure**:
```
codex-rs/
├── tui/
│   ├── src/
│   │   └── chatwidget/
│   │       └── spec_kit/
│   │           ├── clarify_native.rs
│   │           └── mod.rs
│   └── tests/
│       └── spec_kit/
│           ├── clarify_native_tests.rs      (unit)
│           ├── workflow_integration_tests.rs (integration)
│           └── e2e_tests.rs                  (E2E)
```

**Naming Conventions**:
- Unit tests: `{module}_tests.rs` or `#[cfg(test)] mod tests` in source
- Integration tests: `{feature}_integration_tests.rs`
- E2E tests: `e2e_tests.rs` or `{workflow}_e2e.rs`

---

### Workspace-Level Tests

**Location**: `codex-rs/tests/` (workspace root)

**Purpose**: Cross-crate integration tests

**Example**:
```
codex-rs/tests/
├── tui_mcp_integration.rs     # TUI ↔ MCP client integration
├── full_pipeline_e2e.rs       # Complete /speckit.auto workflow
└── hot_reload_integration.rs  # Config changes across crates
```

---

## Coverage Measurement

### Tools

**Primary**: `cargo-tarpaulin`

**Installation**:
```bash
cargo install cargo-tarpaulin
```

**Usage**:
```bash
# All modules
cargo tarpaulin --workspace --all-features --timeout 300

# Specific module
cargo tarpaulin -p codex-tui --all-features

# HTML report
cargo tarpaulin --workspace --all-features --out Html
```

**Configuration** (`.tarpaulin.toml`):
```toml
[tarpaulin]
timeout = "300s"
exclude-files = [
    "target/*",
    "*/tests/*",
    "*/benches/*"
]
```

---

### Alternative: `cargo-llvm-cov`

**Installation**:
```bash
cargo install cargo-llvm-cov
```

**Usage**:
```bash
# Generate coverage
cargo llvm-cov --workspace --all-features --html

# Open report
open target/llvm-cov/html/index.html
```

**Advantage**: More accurate than tarpaulin, faster execution

---

## Critical Path Coverage

### Priority 1: Spec-Kit Automation (70%+ target)

**Critical Flows**:
1. `/speckit.new` → SPEC creation
2. `/speckit.auto` → Full 6-stage pipeline
3. Quality gates → Checkpoint validation
4. Consensus → Multi-agent synthesis

**Current Coverage**: ~75% ✅

**Key Test Files**:
```
tui/tests/spec_kit/
├── new_native_tests.rs                (95 tests)
├── pipeline_coordinator_tests.rs      (85 tests)
├── quality_gate_handler_tests.rs      (75 tests)
├── consensus_coordinator_tests.rs     (45 tests)
└── workflow_integration_tests.rs      (60 tests)
```

---

### Priority 2: MCP Client (70%+ target)

**Critical Flows**:
1. JSON-RPC protocol → Serialization/deserialization
2. Connection lifecycle → Connect, request, disconnect
3. Tool invocation → MCP tool calls
4. Error handling → Retry logic, timeouts

**Current Coverage**: ~65% 🔄

**Key Test Files**:
```
mcp-client/tests/
├── protocol_tests.rs           (40 tests)
├── connection_tests.rs         (30 tests)
├── tool_invocation_tests.rs    (25 tests)
└── integration_tests.rs        (45 tests)
```

---

### Priority 3: Database Layer (50%+ target)

**Critical Flows**:
1. Schema migrations → Up/down migrations
2. CRUD operations → Insert, query, update, delete
3. Connection pooling → R2D2 integration
4. Transaction handling → Rollback on error

**Current Coverage**: ~60% ✅

**Key Test Files**:
```
db/tests/
├── schema_tests.rs         (20 tests)
├── crud_tests.rs           (35 tests)
├── pool_tests.rs           (15 tests)
└── transaction_tests.rs    (10 tests)
```

---

## Test Execution Strategy

### Local Development

**Run all tests**:
```bash
cd codex-rs
cargo test --workspace --all-features
```

**Run specific module**:
```bash
cargo test -p codex-tui --all-features
```

**Run specific test**:
```bash
cargo test -p codex-tui spec_kit::clarify_native::tests::detect_vague_language
```

**Run with output**:
```bash
cargo test -- --nocapture
```

---

### Pre-Commit Hook

**Location**: `.githooks/pre-commit`

**What it runs**:
```bash
# Format check
cargo fmt --all -- --check

# Linting
cargo clippy --workspace --all-targets --all-features -- -D warnings

# Quick test (compilation only, no execution)
cargo test --workspace --no-run
```

**Time**: ~30 seconds (fast feedback)

**Skip** (if needed):
```bash
PRECOMMIT_FAST_TEST=0 git commit -m "..."
```

---

### Pre-Push Hook

**Location**: `.githooks/pre-push`

**What it runs**:
```bash
# Format check
cargo fmt --all -- --check

# Linting
cargo clippy --workspace --all-targets --all-features -- -D warnings

# Build
cargo build --workspace --all-features

# Optional: Full test suite (slow)
# cargo test --workspace --all-features
```

**Time**: ~2-5 minutes

**Skip** (if needed):
```bash
PREPUSH_FAST=0 git push
```

---

### CI/CD Pipeline

**Location**: `.github/workflows/rust.yml`

**Triggers**:
- Push to `main`
- Pull requests
- Manual workflow dispatch

**Jobs**:
1. **Test** (parallel matrix):
   - OS: Ubuntu, macOS, Windows
   - Rust: stable, beta
   - Features: all, default

2. **Coverage** (Ubuntu only):
   - Run `cargo-tarpaulin`
   - Upload to Codecov
   - Comment PR with coverage delta

3. **Lint**:
   - `cargo fmt --check`
   - `cargo clippy -- -D warnings`

**Time**: ~10-15 minutes total

---

## Coverage Gaps

### Known Gaps (Acceptable)

**UI Rendering** (~10% coverage):
- **Reason**: Ratatui rendering hard to test
- **Mitigation**: Manual testing, visual inspection

**Error Handling Paths** (~30% coverage):
- **Reason**: Hard to trigger rare errors
- **Mitigation**: Property-based testing (proptest)

**Legacy Code** (~20% coverage):
- **Reason**: Technical debt, low ROI
- **Mitigation**: Refactor on touch, add tests incrementally

---

### Prioritized Improvements

**Phase 1 (Completed)**: 40%+ coverage
- ✅ Spec-Kit core functionality (360 tests added)
- ✅ MCP client protocol (140 tests added)
- ✅ Database layer (80 tests added)

**Phase 2 (Optional)**: 50%+ coverage
- 🔄 Error recovery scenarios
- 🔄 Concurrent operation tests
- 🔄 Edge case property testing

**Phase 3 (Future)**: 60%+ coverage
- ⏳ UI interaction tests
- ⏳ Performance regression tests
- ⏳ Chaos engineering tests

---

## Testing Best Practices

### DO

**✅ Test behavior, not implementation**:
```rust
// Good: Test behavior
#[test]
fn clarify_detects_vague_language() {
    let result = clarify("System should be fast");
    assert!(result.has_ambiguities());
    assert_eq!(result.ambiguities[0].pattern, "vague_language");
}

// Bad: Test implementation details
#[test]
fn clarify_calls_regex_find() {
    // Don't test internal regex usage
}
```

**✅ Use descriptive test names**:
```rust
#[test]
fn checklist_fails_when_score_below_80() { }

#[test]
fn consensus_degraded_when_only_2_of_3_agents() { }
```

**✅ Arrange-Act-Assert pattern**:
```rust
#[test]
fn test_feature() {
    // Arrange: Setup
    let input = "test input";

    // Act: Execute
    let result = function_under_test(input);

    // Assert: Verify
    assert_eq!(result, expected);
}
```

---

### DON'T

**❌ Test framework internals**:
```rust
// Don't test that Tokio works
#[test]
fn tokio_runtime_spawns_tasks() { }
```

**❌ Rely on test execution order**:
```rust
// Tests should be independent
#[test]
fn test_a() { /* modifies global state */ }

#[test]
fn test_b() { /* depends on test_a */ } // ❌ Bad
```

**❌ Use magic numbers**:
```rust
// Bad
assert_eq!(result.len(), 42);

// Good
const EXPECTED_ITEM_COUNT: usize = 42;
assert_eq!(result.len(), EXPECTED_ITEM_COUNT);
```

---

## Summary

**Testing Strategy Highlights**:

1. **Coverage Target**: 40%+ (achieved: 42-48%)
2. **Test Pyramid**: 63% unit, 33% integration, 4% E2E
3. **Critical Path Focus**: Spec-Kit (75%), MCP (65%), DB (60%)
4. **Tools**: cargo-tarpaulin, cargo-llvm-cov
5. **CI/CD**: GitHub Actions, pre-commit/pre-push hooks
6. **604 Tests Total**: 100% pass rate

**Next Steps**:
- [Test Infrastructure](test-infrastructure.md) - MockMcpManager, fixtures
- [Unit Testing Guide](unit-testing-guide.md) - Patterns and examples
- [Integration Testing](integration-testing-guide.md) - Cross-module tests

---

**References**:
- Rust testing guide: https://doc.rust-lang.org/book/ch11-00-testing.html
- Tarpaulin docs: https://github.com/xd009642/tarpaulin
- Test organization: `codex-rs/*/tests/` directories
