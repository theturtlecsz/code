# SPEC-940 Session 2: Performance Instrumentation Framework

**Date**: 2025-11-28
**Time Budget**: Extended (8h+)
**Primary Focus**: Complete SPEC-940 Phases 2-4
**Secondary**: Selective WIP cleanup

---

## Session Context

### Previous Session (2025-11-28)
- SPEC-959: Verified complete, marked Done
- SPEC-940 Quick Validation: Added `test_spec940_spawn_performance_validation`
  - Result: 0.1ms mean spawn overhead (500x better than <50ms target)
  - Commit: `c4954c13e`

### Current State
- **Build**: Passes (`cargo build -p codex-tui` succeeds)
- **Tests**: TUI 391 pass, Core all pass
- **WIP Files**: 49 uncommitted (from SPEC-958 migration, need selective cleanup)
- **SPEC-940 Progress**: Phase 1 complete (20%), Phases 2-4 remain (9-13h)

---

## Phase 0: WIP Cleanup (30-60 min)

Before new work, stabilize the codebase:

### Checklist
- [ ] Review uncommitted files: `git status --short`
- [ ] Identify stable vs broken changes
- [ ] Commit stable SPEC-958 test migration work
- [ ] Discard or stash broken test files (clippy errors)
- [ ] Verify clean build: `cargo build --workspace`
- [ ] Verify test suite: `cargo test -p codex-tui --lib`

### Key Files to Evaluate
```
codex-rs/core/tests/suite/*.rs - SPEC-958 test migration (mostly stable)
codex-rs/mcp-server/tests/suite/*.rs - auth/login deleted (intentional?)
codex-rs/tui/src/*.rs - streaming/chatwidget changes (stable)
CLAUDE.md - documentation updates (commit)
```

---

## Phase 1: BenchmarkHarness Implementation (3-4h)

**Goal**: Create reusable benchmark infrastructure with statistical rigor

### Deliverables
1. `codex-rs/core/src/benchmarks.rs` (~250 LOC)
   - `BenchmarkHarness` struct with configurable iterations
   - Warmup iteration support (discard first N runs)
   - Async operation support via `BoxFuture`

2. `BenchmarkResult` struct with:
   - mean, stddev calculation
   - min/max tracking
   - Percentiles: p50, p95, p99
   - Sample count

3. Unit tests (~10 tests)
   - Statistics calculation correctness
   - Edge cases (single sample, high variance)
   - Async operation timing

### Implementation Reference
```rust
// From PRD - adapt as needed
pub struct BenchmarkHarness {
    pub name: String,
    pub iterations: usize,
    pub warmup_iterations: usize,
}

impl BenchmarkHarness {
    pub async fn run<F, T>(&self, operation: F) -> BenchmarkResult
    where
        F: Fn() -> BoxFuture<'static, Result<T>>,
    { ... }
}
```

### Validation
- [ ] `cargo test -p codex-core benchmarks` passes
- [ ] Statistics match expected values for known inputs
- [ ] No clippy warnings

---

## Phase 2: Pre/Post Validation Baselines (3-4h)

**Goal**: Measure actual performance of key operations

### Target Operations

| Operation | Module | Expected | Priority |
|-----------|--------|----------|----------|
| DirectProcessExecutor spawn | async_agent_executor.rs | <1ms | P0 |
| SQLite consensus write | consensus_db.rs | <30ms | P1 |
| Config parsing | config.rs | <10ms | P1 |
| Template substitution | prompts.rs | <5ms | P2 |

### Deliverables
1. Benchmark tests in `codex-rs/core/tests/benchmarks/`
   - `spec940_executor_benchmark.rs`
   - `spec940_storage_benchmark.rs`
   - `spec940_config_benchmark.rs`

2. Baseline measurements saved to:
   - `docs/SPEC-KIT-940-performance-instrumentation/evidence/baselines-YYYY-MM-DD.md`

### Validation
- [ ] All benchmarks run successfully
- [ ] Results within expected ranges
- [ ] Evidence file created with measurements

---

## Phase 3: Statistical Reporting & CI Integration (3-5h)

**Goal**: Generate reports and detect regressions in CI

### Deliverables
1. `codex-rs/core/src/report.rs` (~150 LOC)
   - Markdown table generation
   - Comparison with baseline (speedup/regression %)
   - Statistical significance (Welch's t-test, p<0.05)

2. CI integration:
   - Add benchmark job to GitHub Actions
   - Fail on >20% regression
   - Store artifacts for historical tracking

3. Documentation:
   - `docs/performance/instrumentation-guide.md`
   - `docs/performance/benchmark-guide.md`

### Validation
- [ ] Report generation produces valid Markdown
- [ ] CI job runs successfully
- [ ] Regression detection works (test with artificial slowdown)

---

## Success Criteria

### Minimum Viable (MVP)
- [ ] BenchmarkHarness with basic statistics
- [ ] At least 3 operations benchmarked
- [ ] Baseline evidence file created

### Full Completion
- [ ] All 4 target operations benchmarked
- [ ] Statistical reporting with Markdown output
- [ ] CI integration with regression detection
- [ ] Documentation complete

---

## Checkpoints

| Time | Checkpoint | Exit Criteria |
|------|------------|---------------|
| +1h | Phase 0 complete | Clean git status, build passes |
| +4h | Phase 1 complete | BenchmarkHarness tested |
| +7h | Phase 2 complete | Baselines measured |
| +10h | Phase 3 complete | CI integration done |

---

## Reference Files

### Existing Infrastructure
- `codex-rs/core/src/timing.rs` - measure_time! macros (49 LOC)
- `codex-rs/spec-kit/src/timing.rs` - Timer struct + tests (296 LOC)
- `codex-rs/core/src/async_agent_executor.rs:1625` - SPEC-940 quick validation test

### PRD
- `docs/SPEC-KIT-940-performance-instrumentation/PRD.md` - Full specification

### Related SPECs
- SPEC-936: DirectProcessExecutor (provides spawn baseline)
- SPEC-934: SQLite consolidation (provides storage baseline)
- SPEC-933: Parallel spawning (provides concurrency baseline)

---

## Commands Quick Reference

```bash
# Build check
cd ~/code/codex-rs && cargo build --workspace

# Run specific benchmark test
cargo test -p codex-core test_spec940 -- --nocapture

# Run all benchmarks
cargo test -p codex-core benchmarks -- --nocapture

# Clippy check
cargo clippy --workspace --all-targets -- -D warnings

# Format
cargo fmt --all
```

---

## Notes for Claude

1. **Start with Phase 0** - Clean git state before new work
2. **Use timing.rs macros** - Don't reinvent, extend existing infrastructure
3. **Test incrementally** - Run tests after each component
4. **Store in local-memory** - Key decisions and patterns (importance ≥8)
5. **Update SPEC.md** - Progress after each phase

---

## Local Memory Queries

At session start, retrieve context:
```
mcp__local-memory__search(query="SPEC-940 performance", limit=5)
mcp__local-memory__search(query="BenchmarkHarness benchmark", limit=5)
mcp__local-memory__search(query="DirectProcessExecutor timing", limit=5)
```
