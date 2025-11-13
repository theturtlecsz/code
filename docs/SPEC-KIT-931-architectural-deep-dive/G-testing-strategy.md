# SPEC-931G: Testing Strategy & Quality Assurance (ULTRATHINK Analysis)

**Date**: 2025-11-13
**Status**: ✅ COMPLETE (Testing Analysis & Strategy)
**Analyst**: Claude Code (Ultrathink Mode - Rigorous Deep Dive)
**Parent**: SPEC-931 (Architectural Deep Dive)
**Related**: SPEC-931F (Event Sourcing NO-GO, ACID Alternative)

---

## Executive Summary

**FINDINGS**: Strong test infrastructure with critical gaps for ACID compliance

**Test Inventory**:
- **584 total tests** (252 unit + ~332 integration)
- **94% pass rate** (233/252 unit tests passing, 15 failures from global state pollution)
- **9,600 lines** of test code across 22 integration test files
- **Well-categorized**: Handler, workflow, quality gates, errors, state, consensus, evidence

**CRITICAL GAPS Identified**:
1. ❌ **NO database transaction tests** (no BEGIN/COMMIT/ROLLBACK validation)
2. ❌ **NO real concurrency tests** (stubs only, no actual race conditions)
3. ❌ **NO crash recovery at DB level** (SQLite corruption, WAL recovery)
4. ❌ **NO CI/CD automation** (no GitHub Actions, no pre-commit hooks)
5. ❌ **LIMITED performance benchmarks** (MCP only, no orchestration metrics)

**RECOMMENDATIONS**:
1. **P0**: Add transaction atomicity tests for ACID compliance (8 hours)
2. **P0**: Add real concurrency tests for dual-write validation (6 hours)
3. **P0**: Set up GitHub Actions CI for automated testing (4 hours)
4. **P1**: Add DB-level crash recovery tests (6 hours)
5. **P1**: Expand performance benchmarks (orchestration, DB, spawn) (4 hours)
6. **P2**: Fix global state pollution in unit tests (2 hours)
7. **P2**: Update out-of-sync tests (core, mcp-server, e2e) (3 hours)

**Total Effort**: 33 hours to achieve production-grade testing for ACID approach

---

## Table of Contents

1. [Test Inventory & Coverage](#1-test-inventory--coverage)
2. [Test Infrastructure Analysis](#2-test-infrastructure-analysis)
3. [Testing Gaps for ACID Compliance](#3-testing-gaps-for-acid-compliance)
4. [Mock & Fixture Architecture](#4-mock--fixture-architecture)
5. [CI/CD Integration](#5-cicd-integration)
6. [Performance Testing Baseline](#6-performance-testing-baseline)
7. [Testing Strategy Recommendations](#7-testing-strategy-recommendations)
8. [Appendix: Test File Reference](#appendix-test-file-reference)

---

## 1. Test Inventory & Coverage

### 1.1 Test Count Breakdown

**Unit Tests** (codex-tui/lib): **252 tests**
- Status: 233 passing, 15 failing, 3 ignored
- Failure Rate: 6% (non-critical, global state pollution)
- Failures pass when run individually → not blocking

**Integration Tests** (codex-tui/tests/): **~332 tests**
```
Handler & Orchestration:     55 tests  (handler_orchestration_tests.rs: 24KB)
Workflow Integration:        15 tests  (workflow_integration_tests.rs: 36KB)
Quality Gates:               43 tests  (quality_resolution, quality_flow, quality_gates_integration)
Error Handling:              41 tests  (error_recovery_integration, error_tests)
State Management:            37 tests  (state_persistence, state_tests)
Consensus Logic:             26 tests  (consensus_logic_tests.rs: 19KB)
Evidence & Guardrails:       49 tests  (evidence, guardrail, schemas tests)
Edge Cases:                  25 tests  (edge_case_tests.rs: 18KB)
Concurrent Operations:       10 tests  (concurrent_operations: 5KB) ⚠️ STUBS ONLY
Property-Based:           ~10-15 tests (property_based_tests.rs: 10KB)
MCP Integration:           ~10 tests  (mcp_consensus_integration, mock_mcp_tests)
Benchmarks:                   2 tests  (mcp_consensus_benchmark.rs: 8KB, --ignored)
```

**E2E Tests** (broken):
- spec_auto_e2e.rs: 47 compilation errors (validate_retries field removed)
- Impact: No end-to-end testing for /speckit.auto command

**Total**: **584+ tests** (excellent coverage breadth)

### 1.2 Test Compilation Status

✅ **COMPILES**: codex-tui (lib + most integration tests)
❌ **FAILS**:
- codex-core: 49 test compilation errors (API changes not reflected)
- mcp-server: 4 test compilation errors
- spec_auto_e2e: 47 errors (field removed from SpecAutoState)

**Root Cause**: Tests out of sync with code changes
- Missing field: `agent_total_timeout_ms` in ModelProviderInfo
- Unresolved imports: codex_core::protocol (CompactedItem, ResponseItem, SessionMeta)
- Private module access: environment_context, rollout

**Impact**: Can't run codex-core/mcp-server tests, but TUI tests (where agent orchestration is) work fine.

### 1.3 Coverage Analysis

**Line Coverage**: ❌ **UNKNOWN** - cargo tarpaulin can't run due to core test failures

**Manual Analysis** (test files vs source):
- Test code: 9,600 lines (tui/tests/)
- Source code estimate: ~20,000 lines (tui/src/)
- **Estimated coverage**: ~40-50% (rough guess, needs tarpaulin validation)

**Coverage by Component**:
- ✅ Quality gates: GOOD (55 handler tests, 43 quality gate tests)
- ✅ Workflow: GOOD (15 workflow tests, 37 state tests)
- ✅ Error handling: GOOD (41 error tests, 25 edge case tests)
- ✅ Evidence/guardrails: GOOD (49 tests)
- ⚠️ Database: PARTIAL (state persistence tests, NO transaction tests)
- ⚠️ Concurrency: POOR (10 stub tests, no real concurrency)
- ❌ Performance: MINIMAL (2 MCP benchmarks only)

---

## 2. Test Infrastructure Analysis

### 2.1 Test Organization

**Well-Structured Categorization**:
```
tui/tests/
├── common/                          # Shared test infrastructure
│   ├── integration_harness.rs       # IntegrationTestContext, StateBuilder
│   └── mock_mcp.rs                  # MockMcpManager (fixture-based mocking)
│
├── *_integration_tests.rs           # Cross-module integration tests
├── *_logic_tests.rs                 # Logic-focused unit tests
├── *_tests.rs                       # Component-specific tests
├── spec_auto_e2e.rs                 # E2E tests (BROKEN)
└── mcp_consensus_benchmark.rs       # Performance benchmarks (--ignored)
```

**Pattern**: Tests use descriptive names (W01-W15 for workflow, E01-E15 for error recovery, etc.)

### 2.2 IntegrationTestContext

**Purpose**: Isolated test environment with temp directories

**Features**:
```rust
pub struct IntegrationTestContext {
    temp_dir: TempDir,              // Isolated filesystem per test
    spec_id: String,                 // SPEC-XXX-001 identifier
    evidence_dir: PathBuf,           // Evidence directory structure
}
```

**Helpers**:
- `create_spec_dirs()`: Create SPEC directory structure
- `write_prd()`, `write_spec()`: Write test fixtures
- `assert_consensus_exists()`: Verify consensus artifacts
- `assert_guardrail_telemetry_exists()`: Verify guardrail output
- `count_consensus_files()`: Count evidence files

**Strengths**:
- ✅ Excellent isolation (each test gets TempDir)
- ✅ Evidence verification helpers
- ✅ Realistic directory structure

**Gaps**:
- ❌ No SQLite database mocking
- ❌ No transaction coordination testing

### 2.3 MockMcpManager

**Purpose**: Fixture-based MCP mocking for testing without local-memory server

**Features**:
```rust
let mut mock = MockMcpManager::new();
mock.add_fixture("local-memory", "search", Some("SPEC-065"), fixture_json);
let result = mock.call_tool("local-memory", "search", args, None).await?;
```

**Capabilities**:
- ✅ Pattern matching (query_pattern filters)
- ✅ Multiple fixtures per server/tool
- ✅ Load from JSON files
- ✅ Call log for assertions

**Strengths**:
- ✅ Clean fixture API
- ✅ Realistic MCP behavior simulation
- ✅ Good for integration tests

**Gaps**:
- ❌ Can't mock SQLite transactions
- ❌ Can't simulate DB errors/crashes

---

## 3. Testing Gaps for ACID Compliance

### 3.1 CRITICAL: No Database Transaction Tests

**Problem**: SPEC-931F recommends ACID transactions to solve dual-write (HashMap + SQLite), but NO tests validate transaction behavior.

**Missing Tests**:
1. **Transaction Atomicity**:
   ```rust
   // Example missing test
   #[test]
   fn transaction_rollback_on_sqlite_error() {
       // Spawn agent → HashMap insert + SQLite BEGIN
       // Simulate SQLite error (disk full, constraint violation)
       // Verify: Transaction rolled back, HashMap NOT updated
       // Currently: NO TEST for this
   }
   ```

2. **Dual-Write Coordination**:
   ```rust
   #[test]
   fn hashmap_sqlite_atomic_update() {
       // Update agent status
       // Verify: BOTH HashMap and SQLite updated OR neither
       // Currently: NO TEST
   }
   ```

3. **Crash Recovery**:
   ```rust
   #[test]
   fn crash_during_transaction() {
       // Start transaction
       // Simulate crash (panic, kill process)
       // Restart → verify: SQLite journal replayed, state consistent
       // Currently: NO TEST
   }
   ```

**Impact**: Can't validate that ACID approach actually works!

**Recommendation**: Add 15-20 transaction tests (8 hours effort)

### 3.2 CRITICAL: No Real Concurrency Tests

**Problem**: concurrent_operations_integration_tests.rs has 10 tests, but they're STUBS:

**What Tests Do Now**:
```rust
#[test]
fn c01_parallel_agent_spawns() {
    // Just write 3 JSON files
    for agent in &["gemini", "claude", "gpt_pro"] {
        std::fs::write(file, json!({"agent": agent})).unwrap();
    }
    assert_eq!(count_files(), 3);  // That's it!
}
```

**What's Missing**:
- ❌ NO tokio::spawn (no actual parallelism)
- ❌ NO race condition simulation
- ❌ NO concurrent HashMap + SQLite writes
- ❌ NO transaction conflict testing

**Real Concurrency Test Should Look Like**:
```rust
#[tokio::test]
async fn concurrent_agent_spawn_race_condition() {
    let db = setup_test_db();
    let agent_manager = Arc::new(Mutex::new(HashMap::new()));

    // Spawn 10 agents concurrently
    let handles: Vec<_> = (0..10).map(|i| {
        let db = db.clone();
        let manager = agent_manager.clone();
        tokio::spawn(async move {
            spawn_agent_with_transaction(i, manager, db).await
        })
    }).collect();

    // Wait for all
    for h in handles { h.await.unwrap(); }

    // Verify: All 10 agents in HashMap AND SQLite, no duplicates
    assert_eq!(agent_manager.lock().unwrap().len(), 10);
    assert_eq!(db.count_agents(), 10);
}
```

**Impact**: Zero confidence that dual-write is safe under concurrent load

**Recommendation**: Rewrite concurrent tests with actual parallelism (6 hours)

### 3.3 HIGH: No DB-Level Crash Recovery

**Current Tests**: w14_state_recovery_after_crash, e13_mcp_server_crash_reconnect_replay

**What They Actually Test**:
- File-based state recovery (read SpecAutoState from disk)
- MCP reconnection logic

**What's Missing**:
- ❌ SQLite database file corruption
- ❌ WAL (Write-Ahead Logging) file recovery
- ❌ Transaction journal replay
- ❌ Partial writes (row half-written)

**Real Crash Recovery Test**:
```rust
#[test]
fn sqlite_wal_recovery_after_crash() {
    let db = setup_test_db_with_wal();

    // Write some transactions
    db.begin_transaction();
    db.insert_agent("agent-1");
    // Simulate crash BEFORE commit (don't call commit())
    drop(db);  // Force close without commit

    // Reopen database
    let db2 = reopen_db();

    // Verify: WAL recovered, agent-1 NOT in database (rolled back)
    assert_eq!(db2.count_agents(), 0);
}
```

**Recommendation**: Add 10 crash recovery tests (6 hours)

### 3.4 MEDIUM: No Performance Regression Tests

**Current Benchmarks**: mcp_consensus_benchmark.rs (2 tests, --ignored)

**Coverage**:
- ✅ MCP connection initialization (~avg time)
- ✅ MCP search calls

**Missing Benchmarks**:
- ❌ Agent spawn latency (current: ~200ms, target: <100ms)
- ❌ Quality gate end-to-end time (current: ~7s, breakdown unknown)
- ❌ Database query performance (SELECT, INSERT, UPDATE)
- ❌ Consensus collection overhead
- ❌ Tmux session creation (if keeping tmux)

**Recommendation**: Add orchestration benchmarks (4 hours)

---

## 4. Mock & Fixture Architecture

### 4.1 Current Mock Strategy

**Two-Tier Approach**:
1. **File-based**: IntegrationTestContext with TempDir
2. **MCP-based**: MockMcpManager with fixtures

**Strengths**:
- ✅ Clean separation (filesystem vs protocol)
- ✅ Realistic directory structure
- ✅ Reusable fixtures (can load from JSON files)
- ✅ Call log for assertions

**Weaknesses**:
- ❌ No SQLite mocking (can't inject errors, test transactions)
- ❌ No AGENT_MANAGER mocking (uses real HashMap)
- ❌ No network mocking (for provider APIs)

### 4.2 Recommended Mock Architecture for ACID Tests

**Add Third Tier**: MockSqlite for transaction testing

**Design**:
```rust
pub struct MockSqlite {
    mode: MockMode,
    call_log: Vec<CallLogEntry>,
}

enum MockMode {
    /// Normal operation
    Normal,
    /// Fail on next commit
    FailCommit(String),
    /// Simulate crash (panic on next operation)
    Crash,
    /// Simulate constraint violation
    ConstraintViolation(&'static str),
}

impl MockSqlite {
    pub fn begin_transaction(&mut self) -> Result<()> { /* ... */ }
    pub fn commit(&mut self) -> Result<()> {
        match self.mode {
            MockMode::FailCommit(ref err) => Err(anyhow!(err.clone())),
            MockMode::Crash => panic!("Simulated crash!"),
            _ => Ok(())
        }
    }
    pub fn rollback(&mut self) -> Result<()> { /* ... */ }
}
```

**Usage**:
```rust
#[test]
fn test_transaction_rollback_on_commit_failure() {
    let mut mock_db = MockSqlite::new();
    mock_db.set_mode(MockMode::FailCommit("Disk full".to_string()));

    let mut agent_manager = HashMap::new();

    // Attempt dual-write
    let result = spawn_agent_with_transaction("agent-1", &mut agent_manager, &mut mock_db);

    // Verify: Transaction failed, HashMap NOT updated
    assert!(result.is_err());
    assert_eq!(agent_manager.len(), 0);
    assert_eq!(mock_db.call_log.last().unwrap().operation, "rollback");
}
```

**Effort**: 4-6 hours to implement MockSqlite trait

---

## 5. CI/CD Integration

### 5.1 Current State

**Automation**: ❌ **NONE**
- No `.github/workflows/` directory
- No `.githooks/` directory
- No pre-commit hooks installed
- No automated testing on PR

**Manual Testing Only**:
- Developers run `cargo test` locally
- No enforcement, no visibility

**Evidence**:
```bash
$ find .github -name "*.yml"
# (no output - no workflows)

$ ls -la .githooks/
# No .githooks directory

$ cat .git/hooks/pre-commit
# (no output - no hook installed)
```

**Impact**: Regressions can slip into main branch

### 5.2 Recommended CI/CD Setup

**Priority 1: GitHub Actions (4 hours)**

**Workflow**: `.github/workflows/test.yml`
```yaml
name: Tests

on:
  push:
    branches: [main]
  pull_request:
    branches: [main]

jobs:
  test:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v3

      - name: Install Rust
        uses: actions-rust-lang/setup-rust-toolchain@v1

      - name: Run tests
        run: |
          cargo test --workspace --lib
          cargo test --package codex-tui --tests

      - name: Run clippy
        run: cargo clippy --workspace --all-targets -- -D warnings

      - name: Check formatting
        run: cargo fmt --all -- --check
```

**Priority 2: Pre-commit Hooks (2 hours)**

**Setup**: `scripts/setup-hooks.sh`
```bash
#!/bin/bash
# Install git hooks
ln -sf ../../.githooks/pre-commit .git/hooks/pre-commit
chmod +x .githooks/pre-commit
```

**Hook**: `.githooks/pre-commit`
```bash
#!/bin/bash
set -e

echo "Running pre-commit checks..."

# Fast checks only (no full test suite)
cargo fmt --all -- --check
cargo clippy --workspace --lib -- -D warnings
cargo test --workspace --lib --no-run  # Compile tests

echo "✅ Pre-commit checks passed"
```

**Priority 3: Coverage Reporting (2 hours)**

**Add to GitHub Actions**:
```yaml
- name: Install tarpaulin
  run: cargo install cargo-tarpaulin

- name: Generate coverage
  run: cargo tarpaulin --workspace --out Xml

- name: Upload to Codecov
  uses: codecov/codecov-action@v3
```

**Total Effort**: 8 hours for full CI/CD setup

---

## 6. Performance Testing Baseline

### 6.1 Current Benchmarks

**File**: `mcp_consensus_benchmark.rs` (8KB, 2 tests, run with `--ignored`)

**Test 1**: MCP Connection Initialization
```rust
#[tokio::test]
#[ignore]
async fn bench_mcp_initialization() {
    const ITERATIONS: usize = 10;

    // Measure: McpConnectionManager::new() time
    // Reports: avg, min, max, total
}
```

**Test 2**: MCP Search Calls
```rust
#[tokio::test]
#[ignore]
async fn bench_mcp_search_calls() {
    // Measure: call_tool("local-memory", "search") time
}
```

**Strengths**:
- ✅ Uses `Instant::now()` (accurate)
- ✅ Reports statistics (avg, min, max)
- ✅ Multiple iterations (n=10)

**Weaknesses**:
- ❌ Only covers MCP operations
- ❌ Not part of regular test suite (--ignored)
- ❌ No regression tracking (no historical data)

### 6.2 Missing Performance Baselines

**Agent Orchestration**:
- Agent spawn latency (target: <100ms)
- Quality gate end-to-end time (current: ~7s, breakdown unknown)
- Multi-agent consensus collection

**Database**:
- INSERT agent_executions (target: <10ms)
- UPDATE agent status (target: <5ms)
- SELECT by spec_id (target: <5ms)
- Transaction overhead (BEGIN + COMMIT: target <2ms)

**Tmux** (if keeping):
- Session creation time
- Pane creation time
- Output capture time

**Recommended Benchmark Structure**:
```rust
#[tokio::test]
#[ignore]
async fn bench_agent_spawn_latency() {
    const ITERATIONS: usize = 100;
    let mut timings = Vec::new();

    for _ in 0..ITERATIONS {
        let start = Instant::now();
        spawn_agent("test-agent").await?;
        timings.push(start.elapsed());
    }

    // Report: avg, p50, p95, p99
    let avg = timings.iter().sum::<Duration>() / timings.len() as u32;
    timings.sort();
    let p50 = timings[timings.len() / 2];
    let p95 = timings[timings.len() * 95 / 100];
    let p99 = timings[timings.len() * 99 / 100];

    println!("Spawn latency: avg={:?} p50={:?} p95={:?} p99={:?}", avg, p50, p95, p99);

    // Assertion: p95 < 100ms
    assert!(p95 < Duration::from_millis(100), "p95 latency too high");
}
```

**Effort**: 4 hours to add comprehensive benchmarks

---

## 7. Testing Strategy Recommendations

### 7.1 Immediate Priorities (P0)

**1. Add Transaction Atomicity Tests** (8 hours)
- Test: Dual-write atomicity (HashMap + SQLite together or neither)
- Test: Transaction rollback on SQLite error
- Test: Crash during transaction (journal replay)
- Test: Constraint violations
- Test: Concurrent transactions (conflict detection)

**Example**:
```rust
#[tokio::test]
async fn test_dual_write_atomicity() {
    let db = setup_test_db();
    let manager = Arc::new(Mutex::new(HashMap::new()));

    // Force SQLite error
    db.set_mode(MockMode::ConstraintViolation("UNIQUE constraint"));

    // Attempt spawn
    let result = spawn_agent_with_transaction("agent-1", manager.clone(), db).await;

    // Verify: BOTH failed (atomicity)
    assert!(result.is_err());
    assert_eq!(manager.lock().unwrap().len(), 0);  // HashMap NOT updated
    assert_eq!(db.count_agents(), 0);              // SQLite NOT updated
}
```

**2. Add Real Concurrency Tests** (6 hours)
- Rewrite concurrent_operations_integration_tests.rs with tokio::spawn
- Test: 10 agents spawning concurrently
- Test: Race condition in dual-write
- Test: Transaction conflicts
- Test: Concurrent read during write

**3. Setup GitHub Actions CI** (4 hours)
- Add `.github/workflows/test.yml`
- Run on every PR and push to main
- Include: tests, clippy, fmt
- Optional: coverage reporting

**Total P0 Effort**: 18 hours

### 7.2 Short-Term Goals (P1)

**4. Add DB-Level Crash Recovery Tests** (6 hours)
- Test: WAL file recovery
- Test: Transaction journal replay
- Test: Database corruption detection
- Test: Partial write recovery

**5. Expand Performance Benchmarks** (4 hours)
- Benchmark: Agent spawn latency
- Benchmark: Quality gate end-to-end
- Benchmark: Database operations
- Benchmark: Transaction overhead

**6. Fix Broken Tests** (3 hours)
- Fix: spec_auto_e2e.rs (47 errors - validate_retries field)
- Fix: codex-core tests (49 errors - API changes)
- Fix: mcp-server tests (4 errors)

**Total P1 Effort**: 13 hours

### 7.3 Medium-Term Goals (P2)

**7. Fix Global State Pollution** (2 hours)
- Investigate: GLOBAL_REGISTRY initialization in tests
- Fix: 15 failing unit tests (command_registry, json_extractor, routing, subagent_defaults)
- Pattern: Reset global state between tests or use serial execution

**8. Add Coverage Reporting** (2 hours)
- Install: cargo-tarpaulin
- Configure: Codecov integration
- Goal: Achieve 60% line coverage (from current ~40-50%)

**9. Document Testing Patterns** (2 hours)
- Write: TESTING.md guide
- Document: IntegrationTestContext usage
- Document: MockMcpManager patterns
- Document: Benchmark running

**Total P2 Effort**: 6 hours

**GRAND TOTAL**: 18 + 13 + 6 = **37 hours** for comprehensive testing strategy

---

## Appendix: Test File Reference

### Integration Test Files (22 files, 9,600 lines)

**Workflow & Orchestration**:
- `workflow_integration_tests.rs` (36KB, 15 tests): W01-W15 workflow scenarios
- `handler_orchestration_tests.rs` (24KB, 55 tests): Handler coordination tests
- `quality_flow_integration_tests.rs` (6KB, 10 tests): Quality gate flow

**Error Handling**:
- `error_recovery_integration_tests.rs` (30KB, 15 tests): E01-E15 recovery scenarios
- `error_tests.rs` (8KB, 26 tests): Error path validation
- `edge_case_tests.rs` (18KB, 25 tests): Edge case handling

**State & Persistence**:
- `state_persistence_integration_tests.rs` (8KB, 10 tests): S01-S10 state tests
- `state_tests.rs` (11KB, 27 tests): State logic tests

**Quality Gates**:
- `quality_resolution_tests.rs` (11KB, 33 tests): Consensus resolution
- `quality_gates_integration.rs` (16KB, ~20 tests): Quality gate integration

**Consensus & Evidence**:
- `consensus_logic_tests.rs` (19KB, 26 tests): Consensus algorithms
- `evidence_tests.rs` (8KB, 24 tests): Evidence collection
- `guardrail_tests.rs` (9KB, 25 tests): Guardrail validation
- `schemas_tests.rs` (7KB, 21 tests): Schema validation

**Concurrency** (⚠️ STUBS):
- `concurrent_operations_integration_tests.rs` (5KB, 10 tests): C01-C10 (NO real concurrency)

**MCP & Mocking**:
- `mcp_consensus_integration.rs` (8KB, ~10 tests): MCP integration
- `mock_mcp_tests.rs` (2KB, ~5 tests): Mock testing

**Property-Based**:
- `property_based_tests.rs` (10KB, ~15 tests): Property testing

**Benchmarks** (--ignored):
- `mcp_consensus_benchmark.rs` (8KB, 2 tests): MCP performance

**E2E** (❌ BROKEN):
- `spec_auto_e2e.rs` (23KB, BROKEN: 47 errors): /speckit.auto end-to-end

**Utility**:
- `spec_id_generator_integration.rs` (2KB, ~5 tests): SPEC-ID generation
- `spec_status.rs` (4KB, ~5 tests): Status display

---

## Questions Added to MASTER-QUESTIONS.md

**13 new questions added** (Q159-Q171):

**CRITICAL**:
- Q159: ✅ Test compilation failures (API changes, not blocker)
- Q160: ⏳ Test coverage percentage (need tarpaulin)
- Q162: ❌ spec_auto_e2e broken (validate_retries removed)
- Q163: ✅ 15 unit test failures (global state pollution)
- Q164: ❌ No transaction atomicity tests
- Q165: ❌ No DB-level crash recovery tests
- Q166: ✅ No real concurrency tests (stubs only)

**HIGH**:
- Q167: ✅ Mock/fixture strategy (good, missing DB mocking)

**MEDIUM**:
- Q168: ❌ Performance baselines (MCP only, missing orchestration)
- Q169: ❌ CI/CD integration (none exists)
- Q170: ❌ Test independence from tmux
- Q171: ❌ Test execution time

---

## Conclusion

**Strengths**:
- ✅ 584 tests with good categorization
- ✅ Well-designed mock infrastructure (MockMcpManager, IntegrationTestContext)
- ✅ Comprehensive coverage of workflows, errors, state management
- ✅ Some performance benchmarks (MCP)

**Critical Gaps**:
- ❌ NO transaction tests (blocker for ACID compliance validation)
- ❌ NO real concurrency tests (can't validate dual-write safety)
- ❌ NO CI/CD automation (regressions possible)

**Recommendation**: **37 hours** of focused effort to achieve production-grade testing for ACID transaction approach (SPEC-931F). Prioritize P0 items (18 hours) to unblock ACID validation.
