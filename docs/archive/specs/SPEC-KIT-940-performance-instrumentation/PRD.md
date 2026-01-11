# PRD: Performance Instrumentation

**SPEC-ID**: SPEC-KIT-940
**Created**: 2025-11-13
**Status**: Draft - **MEDIUM PRIORITY**
**Priority**: **P2** (Validation + Optimization Foundation)
**Owner**: Code
**Estimated Effort**: 12-16 hours (2-3 days)
**Dependencies**: SPEC-936 (validates tmux elimination claims)
**Blocks**: None

---

## 🔥 Executive Summary

**Current State**: Performance claims are ESTIMATED not MEASURED. No Instant::now() timing instrumentation in code. Claims lack statistical rigor (single runs, no variance). Examples: "93% tmux overhead" (estimated), "5× faster SQLite" (estimated), "3× parallel spawning" (estimated). Can't identify actual bottlenecks (optimization based on guesses). Post-implementation can't prove gains.

**Proposed State**: Comprehensive timing infrastructure using tracing::info! macros with elapsed time. Benchmark harness runs operations n≥10 times, collects statistics (mean, stddev, min/max, percentiles). Statistical reporting validates all performance claims. Pre/post validation measures SPEC-933/934/936 actual impact.

**Impact**:
- ✅ Validates performance claims (SPEC-936 65× speedup, SPEC-934 5× faster storage)
- ✅ Identifies actual bottlenecks (measure, don't guess)
- ✅ Statistical rigor (mean±stddev over n≥10 runs)
- ✅ Proves value (before/after evidence for SPECs)

**Source**: SPEC-931A architectural analysis identified measurement gap (Q72-Q74, Q89, Q91). QUESTION-CONSOLIDATION-ANALYSIS.md Block 1 acknowledged gap but proceeded with SPEC-936 anyway (acceptable risk).

---

## 1. Problem Statement

### Issue #1: Performance Claims Are ESTIMATED Not MEASURED (CRITICAL)

**Evidence from SPEC-936** (tmux elimination):
```
Claim: "93% overhead (6.5s of 7s total)"
Reality: ESTIMATED, no Instant::now() instrumentation
Evidence: Session reports show 77s total, but no per-step breakdown
```

**Evidence from SPEC-934** (storage consolidation):
```
Claim: "5× faster (30ms SQLite vs 150ms MCP)"
Reality: ESTIMATED, no benchmarks
```

**Evidence from SPEC-933** (parallel spawning):
```
Claim: "3× faster (150ms sequential → 50ms parallel)"
Reality: ESTIMATED, no timing measurements
```

**Problem**: We're optimizing based on guesses, not measurements.

**Impact**:
- Can't prove SPEC value (no before/after evidence)
- Might optimize wrong things (if estimates are wrong)
- Users can't validate claims (trust us, no proof)

**Example Consequence** (if estimate is 50% wrong):
```
Estimated: 65× speedup (6.5s → 0.1s)
Actual: 10× speedup (6.5s → 0.65s)
Result: Still valuable, but oversold
```

---

### Issue #2: No Statistical Validation (MEDIUM)

**Current Approach** (single runs):
```rust
// Run once, report time
let start = Instant::now();
spawn_agents().await;
let elapsed = start.elapsed();
println!("Spawn time: {}ms", elapsed.as_millis());  // Single data point
```

**Problems**:
- High variance (network, CPU load, disk I/O)
- Outliers not detected (slow run due to background process)
- Can't compare before/after (was improvement statistically significant?)

**Proper Approach** (statistical rigor):
```rust
// Run n≥10 times, collect statistics
let mut samples = Vec::new();
for _ in 0..10 {
    let start = Instant::now();
    spawn_agents().await;
    samples.push(start.elapsed().as_millis());
}

let mean = samples.iter().sum::<u128>() / samples.len() as u128;
let stddev = calculate_stddev(&samples);
let min = samples.iter().min().unwrap();
let max = samples.iter().max().unwrap();

println!("Spawn time: {}±{}ms (min: {}, max: {}, n=10)", mean, stddev, min, max);
```

**Benefit**: Know if improvement is real or noise.

---

### Issue #3: Can't Identify Actual Bottlenecks (MEDIUM)

**Current Reality**:
```
Quality gate total time: 77s
Breakdown: ???
- Tmux overhead: ??? (estimated 6.5s)
- Agent execution: ??? (estimated 60s)
- Consensus storage: ??? (estimated 150ms)
- Database writes: ??? (unknown)
- Config parsing: ??? (unknown)
```

**Problem**: Don't know where to optimize (guessing based on intuition).

**Proposed** (instrumentation):
```
Quality gate total time: 77.2s
Breakdown (measured):
- Tmux creation: 2.1s (2.7% of total)
- Tmux pane spawn: 4.3s (5.6% of total)
- Agent execution: 68.5s (88.7% of total) ← ACTUAL BOTTLENECK
- Consensus storage: 1.8s (2.3% of total)
- Database writes: 0.4s (0.5% of total)
- Config parsing: 0.1s (0.1% of total)
```

**Discovery**: Tmux is only 8.3% (not 93%!). Agent execution is actual bottleneck.

**Benefit**: Optimize what matters (agent execution, not tmux).

---

### Issue #4: Post-Implementation Can't Prove Gains (HIGH)

**Scenario**: SPEC-936 (tmux elimination) claims 65× speedup.

**Problem**: No baseline measurement before implementation.

**After Implementation**:
```
User: "How much faster is it now?"
Us: "Uh... we didn't measure before, so we can't say. Trust us, it's faster!"
```

**Better Approach**:
```
BEFORE (SPEC-936):
- Quality gate spawn time: 6.2±0.8s (n=10)

AFTER (SPEC-936):
- Quality gate spawn time: 0.15±0.03s (n=10)

PROOF: 41× faster (6.2s → 0.15s, p<0.05)
```

**Benefit**: Evidence-based validation of SPEC value.

---

## 2. Proposed Solution

### Component 1: Timing Infrastructure (CRITICAL - 4-5h)

**Implementation**:
```rust
// timing.rs - Timing macros
#[macro_export]
macro_rules! measure_time {
    ($label:expr, $block:expr) => {{
        let start = std::time::Instant::now();
        let result = $block;
        let elapsed = start.elapsed();
        tracing::info!(
            operation = $label,
            elapsed_ms = elapsed.as_millis(),
            "Operation completed"
        );
        result
    }};
}

// Usage in orchestration
async fn spawn_agent(agent_name: &str) -> Result<AgentHandle> {
    measure_time!("spawn_agent", async {
        // Tmux session creation
        let session_id = measure_time!("tmux_create_session", {
            create_tmux_session(agent_name).await?
        });

        // Pane initialization
        let pane_id = measure_time!("tmux_create_pane", {
            create_tmux_pane(session_id, agent_name).await?
        });

        // Stability polling
        measure_time!("tmux_poll_stability", {
            poll_tmux_stability(pane_id).await?
        });

        Ok(AgentHandle { session_id, pane_id })
    }).await
}
```

**Instrumentation Points** (prioritized):

**P0 (Critical - SPEC-936 validation)**:
- Tmux session creation
- Tmux pane creation
- Tmux stability polling
- Total agent spawn time

**P0 (Critical - SPEC-933 validation)**:
- Agent spawning (sequential vs parallel)
- Database transaction time

**P1 (Important - SPEC-934 validation)**:
- MCP consensus storage
- SQLite consensus storage
- Consensus retrieval (MCP vs SQLite)

**P1 (Important - General optimization)**:
- Config parsing
- Prompt building
- Template substitution

**P2 (Nice-to-have)**:
- Network latency (API calls)
- Evidence file writes
- Log formatting

---

### Component 2: Benchmark Harness (CRITICAL - 3-4h)

**Implementation**:
```rust
// benchmarks.rs
pub struct BenchmarkHarness {
    pub name: String,
    pub iterations: usize,
    pub warmup_iterations: usize,
}

impl BenchmarkHarness {
    pub async fn run<F, T>(&self, operation: F) -> BenchmarkResult
    where
        F: Fn() -> BoxFuture<'static, Result<T>>,
    {
        // Warmup (discard results)
        for _ in 0..self.warmup_iterations {
            operation().await.ok();
        }

        // Collect samples
        let mut samples = Vec::new();
        for i in 0..self.iterations {
            let start = Instant::now();
            let result = operation().await;
            let elapsed = start.elapsed();

            if result.is_ok() {
                samples.push(elapsed.as_millis());
            } else {
                tracing::warn!(
                    benchmark = %self.name,
                    iteration = i,
                    "Benchmark iteration failed, excluding from stats"
                );
            }
        }

        // Calculate statistics
        BenchmarkResult::from_samples(&samples)
    }
}

pub struct BenchmarkResult {
    pub mean: f64,
    pub stddev: f64,
    pub min: u128,
    pub max: u128,
    pub p50: u128,  // Median
    pub p95: u128,
    pub p99: u128,
    pub sample_count: usize,
}

impl BenchmarkResult {
    pub fn from_samples(samples: &[u128]) -> Self {
        let mean = samples.iter().sum::<u128>() as f64 / samples.len() as f64;

        let variance = samples.iter()
            .map(|&x| (x as f64 - mean).powi(2))
            .sum::<f64>() / samples.len() as f64;
        let stddev = variance.sqrt();

        let mut sorted = samples.to_vec();
        sorted.sort();

        BenchmarkResult {
            mean,
            stddev,
            min: *sorted.first().unwrap(),
            max: *sorted.last().unwrap(),
            p50: sorted[sorted.len() / 2],
            p95: sorted[sorted.len() * 95 / 100],
            p99: sorted[sorted.len() * 99 / 100],
            sample_count: samples.len(),
        }
    }

    pub fn report(&self) {
        tracing::info!(
            mean_ms = self.mean,
            stddev_ms = self.stddev,
            min_ms = self.min,
            max_ms = self.max,
            p50_ms = self.p50,
            p95_ms = self.p95,
            p99_ms = self.p99,
            sample_count = self.sample_count,
            "Benchmark completed"
        );
    }
}
```

---

### Component 3: Statistical Reporting (MEDIUM - 2-3h)

**Implementation**:
```rust
// report.rs
pub fn generate_performance_report(benchmarks: &[BenchmarkResult]) -> String {
    let mut report = String::from("# Performance Benchmark Report\n\n");

    report.push_str("| Operation | Mean±Stddev (ms) | Min | P50 | P95 | P99 | Max | n |\n");
    report.push_str("|-----------|------------------|-----|-----|-----|-----|-----|---|\n");

    for (name, result) in benchmarks {
        report.push_str(&format!(
            "| {} | {:.1}±{:.1} | {} | {} | {} | {} | {} | {} |\n",
            name,
            result.mean,
            result.stddev,
            result.min,
            result.p50,
            result.p95,
            result.p99,
            result.max,
            result.sample_count
        ));
    }

    report
}

// Save to evidence
pub fn save_performance_report(spec_id: &str, report: &str) -> Result<()> {
    let evidence_path = format!("docs/{}/evidence/performance-baseline.md", spec_id);
    std::fs::write(&evidence_path, report)?;
    tracing::info!(
        spec_id = %spec_id,
        evidence_path = %evidence_path,
        "Performance report saved"
    );
    Ok(())
}
```

**Example Output**:
```markdown
# Performance Benchmark Report

| Operation | Mean±Stddev (ms) | Min | P50 | P95 | P99 | Max | n |
|-----------|------------------|-----|-----|-----|-----|-----|---|
| tmux_create_session | 2134.2±187.3 | 1891 | 2098 | 2456 | 2589 | 2643 | 10 |
| tmux_create_pane | 4287.5±312.1 | 3876 | 4234 | 4789 | 4923 | 5012 | 10 |
| tmux_poll_stability | 521.3±89.7 | 412 | 503 | 687 | 721 | 743 | 10 |
| spawn_agent_total | 6942.9±421.8 | 6234 | 6891 | 7456 | 7689 | 7823 | 10 |
| parallel_spawn_3_agents | 1234.5±156.2 | 1087 | 1198 | 1456 | 1523 | 1589 | 10 |
| mcp_consensus_storage | 152.3±23.4 | 123 | 148 | 187 | 201 | 213 | 10 |
| sqlite_consensus_storage | 28.7±5.1 | 21 | 27 | 35 | 39 | 42 | 10 |
```

---

### Component 4: Pre/Post Validation (CRITICAL - 3-4h)

**SPEC-936 Validation** (tmux elimination):
```rust
// BEFORE SPEC-936 implementation
#[tokio::test]
async fn benchmark_tmux_spawn_baseline() {
    let harness = BenchmarkHarness {
        name: "tmux_spawn_baseline".to_string(),
        iterations: 10,
        warmup_iterations: 2,
    };

    let result = harness.run(|| {
        Box::pin(async {
            spawn_agent_with_tmux("gemini").await
        })
    }).await;

    result.report();
    save_performance_report("SPEC-936", &format!("BASELINE: {:?}", result))?;
}

// AFTER SPEC-936 implementation
#[tokio::test]
async fn benchmark_direct_spawn_validation() {
    let harness = BenchmarkHarness {
        name: "direct_spawn_validation".to_string(),
        iterations: 10,
        warmup_iterations: 2,
    };

    let result = harness.run(|| {
        Box::pin(async {
            spawn_agent_direct("gemini").await
        })
    }).await;

    result.report();

    // Load baseline
    let baseline = load_baseline("SPEC-936")?;

    // Statistical comparison
    let speedup = baseline.mean / result.mean;
    let p_value = welch_t_test(&baseline.samples, &result.samples);

    tracing::info!(
        baseline_mean = baseline.mean,
        new_mean = result.mean,
        speedup = speedup,
        p_value = p_value,
        significant = p_value < 0.01,
        "SPEC-936 validation complete"
    );

    // Assert significant improvement
    assert!(speedup > 10.0, "Expected ≥10× speedup, got {:.1}×", speedup);
    assert!(p_value < 0.01, "Improvement not statistically significant");
}
```

**SPEC-934 Validation** (storage consolidation):
```rust
#[tokio::test]
async fn benchmark_storage_comparison() {
    // Benchmark MCP storage
    let mcp_result = benchmark_mcp_consensus_storage().await;

    // Benchmark SQLite storage
    let sqlite_result = benchmark_sqlite_consensus_storage().await;

    // Compare
    let speedup = mcp_result.mean / sqlite_result.mean;

    tracing::info!(
        mcp_mean = mcp_result.mean,
        sqlite_mean = sqlite_result.mean,
        speedup = speedup,
        "SPEC-934 validation: SQLite {:.1}× faster than MCP", speedup
    );

    assert!(speedup >= 3.0, "Expected ≥3× speedup, got {:.1}×", speedup);
}
```

---

## 3. Acceptance Criteria

### AC1: Timing Infrastructure ✅
- [ ] measure_time! macro implemented
- [ ] All P0 instrumentation points covered (tmux, spawning, transactions)
- [ ] All P1 instrumentation points covered (MCP, SQLite, config)
- [ ] Logs capture operation name + elapsed time

### AC2: Benchmark Harness ✅
- [ ] BenchmarkHarness runs n≥10 iterations
- [ ] Warmup iterations discard first 2 runs
- [ ] Statistics calculated (mean, stddev, min/max, percentiles)
- [ ] Failed iterations excluded from stats

### AC3: Statistical Reporting ✅
- [ ] Performance reports generated (Markdown table)
- [ ] Reports saved to evidence directory
- [ ] All benchmarks include: mean±stddev, min, P50, P95, P99, max, n

### AC4: Pre/Post Validation ✅
- [ ] SPEC-936 baseline measured (before tmux elimination)
- [ ] SPEC-936 validation measured (after tmux elimination)
- [ ] SPEC-934 baseline measured (MCP storage)
- [ ] SPEC-934 validation measured (SQLite storage)
- [ ] Statistical significance tested (Welch's t-test, p<0.05)

---

## 4. Technical Implementation

### Day 1: Timing Infrastructure (4-5h)

**Morning (3h)**:
- Create timing.rs with measure_time! macro
- Add instrumentation to orchestration code (P0 points)
- Test timing logs (verify elapsed_ms captured)

**Afternoon (1-2h)**:
- Add P1 instrumentation (MCP, SQLite, config)
- Verify all timing logs include operation context

**Files**:
- `codex-core/src/timing.rs` (~100 LOC)
- `codex-tui/src/chatwidget/spec_kit/agent_orchestrator.rs` (+50 LOC timing)
- `codex-tui/src/chatwidget/spec_kit/quality_gate_handler.rs` (+50 LOC timing)

---

### Day 2: Benchmark Harness + Reporting (5-7h)

**Morning (3-4h)**:
- Create benchmarks.rs with BenchmarkHarness
- Implement statistics calculation (mean, stddev, percentiles)
- Unit tests (verify stats correctness)

**Afternoon (2-3h)**:
- Create report.rs with Markdown generation
- Save reports to evidence directory
- Test report formatting

**Files**:
- `codex-core/src/benchmarks.rs` (~250 LOC)
- `codex-core/src/report.rs` (~150 LOC)
- `codex-core/src/tests/benchmarks_tests.rs` (~200 LOC)

---

### Day 3: Pre/Post Validation (3-4h)

**Morning (2h)**:
- Create baseline benchmarks (SPEC-936 tmux spawn, SPEC-934 MCP storage)
- Run benchmarks, save to evidence

**Afternoon (1-2h)**:
- Create validation tests (compare baseline to post-implementation)
- Statistical significance testing (Welch's t-test)
- Generate validation reports

**Files**:
- `codex-core/src/tests/spec_936_validation.rs` (~150 LOC)
- `codex-core/src/tests/spec_934_validation.rs` (~150 LOC)
- Evidence files (performance-baseline.md, validation-report.md)

---

## 5. Success Metrics

### Validation Metrics
- **SPEC-936 Speedup**: Measured 10-65× faster (validate estimate)
- **SPEC-934 Speedup**: Measured 3-5× faster (validate estimate)
- **SPEC-933 Speedup**: Measured 2-3× faster parallel spawning

### Instrumentation Coverage
- **P0 Operations**: 100% covered (tmux, spawning, transactions)
- **P1 Operations**: 100% covered (MCP, SQLite, config)
- **P2 Operations**: 50% covered (network, evidence, logs)

### Statistical Rigor
- **Sample Size**: n≥10 for all benchmarks
- **Significance**: p<0.05 for all performance claims
- **Reporting**: Mean±stddev for all measurements

---

## 6. Risk Analysis

### Risk 1: High Variance in Measurements (MEDIUM)

**Scenario**: Network latency, CPU load cause high stddev (e.g., 100±50ms).

**Mitigation**:
- Report percentiles (P95, P99) to show distribution
- Filter outliers (>3σ from mean)
- Increase sample size (n=20 if needed)

**Likelihood**: Medium (network variance inevitable)

---

### Risk 2: Baseline Lost (LOW)

**Scenario**: Implement SPEC-936 before measuring baseline (can't prove gains).

**Mitigation**:
- Measure baseline FIRST (before implementing SPEC-936)
- Save baseline to git (evidence/performance-baseline.md)
- Document baseline in SPEC-936 PRD

**Likelihood**: Low (this SPEC prevents it)

---

## 7. Open Questions

### Q1: Should benchmarks run in CI?

**Context**: Automated benchmarks in CI could detect performance regressions.

**Decision**: YES (Phase 2) - Add CI benchmarks after baseline established. Gate PRs if performance regresses >20%.

---

### Q2: What p-value threshold for significance?

**Context**: p<0.01 (99% confidence) vs p<0.05 (95% confidence).

**Decision**: p<0.05 (95% confidence) - Standard significance threshold, easier to achieve with reasonable sample sizes while maintaining statistical rigor.

---

## 8. Implementation Strategy

### Day 1: Timing Infrastructure (5h)
- **Hour 1-3**: measure_time! macro, P0 instrumentation
- **Hour 4-5**: P1 instrumentation, verify logs

### Day 2: Benchmark Harness + Reporting (7h)
- **Hour 1-4**: BenchmarkHarness, statistics calculation
- **Hour 5-7**: Report generation, evidence saving

### Day 3: Pre/Post Validation (4h)
- **Hour 1-2**: Baseline benchmarks (SPEC-936, SPEC-934)
- **Hour 3-4**: Validation tests, statistical significance

**Total**: 16h (within 12-16h estimate, upper bound)

---

## 9. Deliverables

1. **Code Changes**:
   - `codex-core/src/timing.rs` - Timing infrastructure
   - `codex-core/src/benchmarks.rs` - Benchmark harness
   - `codex-core/src/report.rs` - Statistical reporting
   - Instrumentation throughout orchestration code

2. **Benchmarks**:
   - Baseline benchmarks (SPEC-936, SPEC-934)
   - Validation tests (pre/post comparison)
   - Statistical significance tests

3. **Evidence**:
   - `docs/SPEC-KIT-936/evidence/performance-baseline.md`
   - `docs/SPEC-KIT-936/evidence/validation-report.md`
   - `docs/SPEC-KIT-934/evidence/performance-baseline.md`

4. **Documentation**:
   - `docs/performance/instrumentation-guide.md` - How to add timing
   - `docs/performance/benchmark-guide.md` - How to run benchmarks

---

## 10. Validation Plan

### Unit Tests (10 tests)
- Statistics calculation (mean, stddev, percentiles)
- measure_time! macro correctness
- Report formatting

### Benchmark Tests (8 benchmarks)
- Tmux session creation (baseline)
- Tmux pane creation (baseline)
- Direct async spawn (validation)
- MCP consensus storage (baseline)
- SQLite consensus storage (validation)
- Parallel spawning (SPEC-933 validation)
- Sequential spawning (SPEC-933 baseline)
- Config parsing (optimization baseline)

### Statistical Tests (3 tests)
- Welch's t-test implementation
- Statistical significance (p<0.05)
- Outlier filtering (>3σ)

**Total**: 21 tests

---

## 11. Conclusion

SPEC-940 adds comprehensive performance instrumentation, statistical benchmarking, and pre/post validation for SPEC-933/934/936. **Estimated effort: 12-16 hours over 3 days.**

**Key Benefits**:
- ✅ Validates all performance claims (measured, not estimated)
- ✅ Identifies actual bottlenecks (measure, don't guess)
- ✅ Statistical rigor (mean±stddev, p<0.05)
- ✅ Evidence-based value (before/after proof)

**Next Steps**:
1. Review and approve SPEC-940
2. **CRITICAL**: Measure baselines BEFORE implementing SPEC-936
3. Schedule Day 1 (timing infrastructure)
4. Coordinate with SPEC-936 (validate claims post-implementation)

---

Back to [Key Docs](../KEY_DOCS.md)
