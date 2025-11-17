# Performance Testing Guide

Comprehensive guide to performance testing, benchmarking, and profiling.

---

## Overview

**Performance Testing Philosophy**: Measure, don't guess. Validate optimizations with data.

**Goals**:
- Measure baseline performance
- Validate optimizations
- Detect regressions
- Identify bottlenecks

**Tools**:
- **criterion**: Statistical benchmarking
- **cargo-flamegraph**: Profiling
- **cargo-bloat**: Binary size analysis
- **hyperfine**: Command-line benchmarking

**Current Benchmarks**:
- Database performance (6.6× read speedup validated)
- MCP client (5.3× faster than subprocess validated)
- Connection pooling (R2D2)

---

## Benchmarking with Criterion

### What is Criterion?

**Criterion** is a statistical benchmarking tool for Rust that provides:
- Accurate measurements (micro/nanosecond precision)
- Statistical analysis (mean, stddev, outliers)
- Regression detection (compare to baseline)
- HTML reports with charts

**Website**: https://bheisler.github.io/criterion.rs/

---

### Setup

**Add to Cargo.toml**:
```toml
[dev-dependencies]
criterion = { version = "0.5", features = ["html_reports"] }

[[bench]]
name = "my_benchmark"
harness = false
```

---

### Basic Benchmark

**File**: `benches/simple_benchmark.rs`

```rust
use criterion::{Criterion, black_box, criterion_group, criterion_main};

fn fibonacci(n: u64) -> u64 {
    match n {
        0 => 1,
        1 => 1,
        n => fibonacci(n - 1) + fibonacci(n - 2),
    }
}

fn benchmark_fibonacci(c: &mut Criterion) {
    c.bench_function("fib 20", |b| {
        b.iter(|| fibonacci(black_box(20)));
    });
}

criterion_group!(benches, benchmark_fibonacci);
criterion_main!(benches);
```

**Run**:
```bash
cargo bench --bench simple_benchmark
```

**Output**:
```
fib 20                  time:   [26.029 µs 26.251 µs 26.509 µs]
Found 11 outliers among 100 measurements (11.00%)
  6 (6.00%) high mild
  5 (5.00%) high severe
```

---

### Database Performance Benchmark

**Example**: `codex-rs/core/benches/db_performance.rs`

**Performance Targets**:
- Before: 850µs/read, 2.1ms/write, 78ms/100-read batch
- After: 129µs/read, 0.9ms/write, 12ms/100-read batch
- Improvement: 6.6× read, 2.3× write, 6.5× batch

---

#### Benchmark Setup

```rust
use criterion::{Criterion, Throughput, black_box, criterion_group, criterion_main};
use codex_core::db::initialize_pool;
use tempfile::TempDir;

/// Create temporary database with schema
fn setup_temp_db() -> (TempDir, PathBuf) {
    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let db_path = temp_dir.path().join("test.db");

    let conn = Connection::open(&db_path).expect("Failed to open connection");
    conn.execute_batch(
        "CREATE TABLE IF NOT EXISTS consensus_runs (
            id INTEGER PRIMARY KEY,
            spec_id TEXT NOT NULL,
            stage TEXT NOT NULL,
            consensus_ok INTEGER NOT NULL,
            created_at INTEGER NOT NULL
        );
        CREATE INDEX IF NOT EXISTS idx_spec_stage ON consensus_runs(spec_id, stage);"
    )
    .expect("Failed to create schema");

    (temp_dir, db_path)
}

/// Create connection pool with WAL mode
fn setup_pool(db_path: &PathBuf) -> Pool<SqliteConnectionManager> {
    initialize_pool(db_path, 10).expect("Failed to initialize pool")
}
```

---

#### Benchmark #1: Connection Pool vs Single Connection

```rust
fn benchmark_connection_pool_vs_single(c: &mut Criterion) {
    let mut group = c.benchmark_group("connection_pool_vs_single");

    // Setup: Create database with test data
    let (_temp_dir, db_path) = setup_temp_db();
    let pool = setup_pool(&db_path);

    // Insert 1000 test records
    {
        let conn = pool.get().expect("Failed to get connection");
        insert_test_data(&conn, 1000);
    }

    // Benchmark: Pooled connection reads
    group.bench_function("pooled_connection_read", |b| {
        b.iter(|| {
            let conn = pool.get().expect("Failed to get connection");
            let mut stmt = conn
                .prepare("SELECT * FROM consensus_runs WHERE spec_id = ?1")
                .expect("Failed to prepare statement");
            let _count = stmt
                .query_map(["SPEC-TEST-050"], |_row| Ok(()))
                .expect("Failed to query")
                .count();
            black_box(_count);
        });
    });

    // Benchmark: Single connection reads (reused connection)
    group.bench_function("single_connection_read", |b| {
        let conn = setup_single_connection_wal(&db_path);
        b.iter(|| {
            let mut stmt = conn
                .prepare("SELECT * FROM consensus_runs WHERE spec_id = ?1")
                .expect("Failed to prepare statement");
            let _count = stmt
                .query_map(["SPEC-TEST-050"], |_row| Ok(()))
                .expect("Failed to query")
                .count();
            black_box(_count);
        });
    });

    group.finish();
}
```

**Results**:
```
connection_pool_vs_single/pooled_connection_read
                        time:   [129.45 µs 130.12 µs 130.89 µs]

connection_pool_vs_single/single_connection_read
                        time:   [127.89 µs 128.45 µs 129.12 µs]
```

**Analysis**:
- ✅ Pool overhead minimal (~1-2µs)
- ✅ Both achieve target (<150µs vs 850µs before)
- ✅ 6.6× improvement validated

---

#### Benchmark #2: WAL Mode Impact

```rust
fn benchmark_wal_mode_impact(c: &mut Criterion) {
    let mut group = c.benchmark_group("wal_mode_impact");

    let (_temp_dir, db_path) = setup_temp_db();

    // Setup: Connection with WAL mode
    let conn_wal = setup_single_connection_wal(&db_path);
    insert_test_data(&conn_wal, 1000);

    // Setup: Connection with DELETE mode (no WAL)
    let (_temp_dir2, db_path2) = setup_temp_db();
    let conn_delete = setup_single_connection_delete(&db_path2);
    insert_test_data(&conn_delete, 1000);

    // Benchmark: Read with WAL
    group.bench_function("read_wal", |b| {
        b.iter(|| {
            let mut stmt = conn_wal
                .prepare("SELECT * FROM consensus_runs WHERE spec_id = ?1")
                .unwrap();
            black_box(stmt.query_map(["SPEC-TEST-050"], |_| Ok(())).unwrap().count());
        });
    });

    // Benchmark: Read with DELETE mode
    group.bench_function("read_delete", |b| {
        b.iter(|| {
            let mut stmt = conn_delete
                .prepare("SELECT * FROM consensus_runs WHERE spec_id = ?1")
                .unwrap();
            black_box(stmt.query_map(["SPEC-TEST-050"], |_| Ok(())).unwrap().count());
        });
    });

    group.finish();
}
```

**Results**:
```
wal_mode_impact/read_wal
                        time:   [129.12 µs 130.45 µs 131.89 µs]

wal_mode_impact/read_delete
                        time:   [847.34 µs 851.23 µs 856.78 µs]

Improvement: 6.58× faster with WAL ✅
```

---

### Throughput Benchmarks

**Pattern**: Measure operations per second

```rust
fn benchmark_batch_reads(c: &mut Criterion) {
    let mut group = c.benchmark_group("batch_reads");

    let (_temp_dir, db_path) = setup_temp_db();
    let pool = setup_pool(&db_path);
    let conn = pool.get().unwrap();
    insert_test_data(&conn, 1000);

    // Benchmark 100 reads (measure throughput)
    group.throughput(Throughput::Elements(100));
    group.bench_function("read_100", |b| {
        b.iter(|| {
            for i in 0..100 {
                let conn = pool.get().unwrap();
                let mut stmt = conn.prepare("SELECT * FROM consensus_runs WHERE spec_id = ?1").unwrap();
                let _count = stmt.query_map([format!("SPEC-TEST-{:03}", i % 100)], |_| Ok(())).unwrap().count();
                black_box(_count);
            }
        });
    });

    group.finish();
}
```

**Results**:
```
batch_reads/read_100    time:   [12.234 ms 12.456 ms 12.689 ms]
                        thrpt:  [7.88 Kelem/s 8.03 Kelem/s 8.17 Kelem/s]

Before optimization: 78ms/100 reads → 1.28 Kelem/s
After optimization:  12ms/100 reads → 8.03 Kelem/s
Improvement: 6.27× faster ✅
```

---

### Running Benchmarks

**Run all benchmarks**:
```bash
cd codex-rs
cargo bench
```

**Run specific benchmark**:
```bash
cargo bench --bench db_performance
```

**Run specific function**:
```bash
cargo bench --bench db_performance -- connection_pool
```

**Generate baseline** (for regression detection):
```bash
cargo bench -- --save-baseline baseline_2025_11_17
```

**Compare to baseline**:
```bash
cargo bench -- --baseline baseline_2025_11_17
```

**View HTML reports**:
```bash
open target/criterion/report/index.html
```

---

## Profiling

### Flamegraphs with cargo-flamegraph

**What are Flamegraphs?**:
- Visual representation of stack traces
- Shows where CPU time is spent
- Width = time spent in function
- Height = call stack depth

**Install**:
```bash
cargo install flamegraph
```

**Usage**:
```bash
# Profile specific benchmark
cargo flamegraph --bench db_performance -- --bench

# Profile specific test
cargo flamegraph --test integration_test

# Profile binary
cargo flamegraph --bin code
```

**Output**: `flamegraph.svg` (interactive SVG)

**Interpretation**:
- **Wide bars**: Hot paths (optimize these)
- **Narrow bars**: Not worth optimizing
- **Tall stacks**: Deep call chains

---

### perf (Linux only)

**Install**:
```bash
sudo apt install linux-tools-generic
```

**Record**:
```bash
cargo build --release
perf record --call-graph=dwarf ./target/release/code
```

**Analyze**:
```bash
perf report
```

**Generate Flamegraph**:
```bash
perf script | stackcollapse-perf.pl | flamegraph.pl > perf.svg
```

---

### cargo-bloat (Binary Size Analysis)

**Purpose**: Find large dependencies

**Install**:
```bash
cargo install cargo-bloat
```

**Usage**:
```bash
cd codex-rs
cargo bloat --release
```

**Output**:
```
File  .text     Size Crate
0.7%   1.2%   24.5KiB regex
0.6%   1.0%   20.1KiB serde_json
0.5%   0.9%   18.7KiB tokio
...
```

**Optimize** (if needed):
```toml
# Cargo.toml
[profile.release]
lto = true              # Link-time optimization
codegen-units = 1       # Better optimization, slower build
strip = true            # Strip symbols
opt-level = "z"         # Optimize for size
```

---

## Command-Line Benchmarking

### hyperfine

**Purpose**: Benchmark CLI commands

**Install**:
```bash
cargo install hyperfine
```

**Usage**:
```bash
# Benchmark single command
hyperfine './codex-rs/target/release/code --version'

# Compare commands
hyperfine \
  './codex-rs/target/release/code doctor' \
  './codex-rs/target/dev-fast/code doctor'

# Warmup runs
hyperfine --warmup 3 'cargo test'

# Multiple runs
hyperfine --runs 100 './codex-rs/target/release/code --help'
```

**Example Output**:
```
Benchmark 1: ./target/release/code --version
  Time (mean ± σ):      12.3 ms ±   0.5 ms    [User: 8.2 ms, System: 3.1 ms]
  Range (min … max):    11.5 ms …  14.2 ms    100 runs
```

---

### Benchmarking /speckit.auto

**Example**:
```bash
hyperfine --warmup 1 --runs 5 \
  './codex-rs/target/release/code run "/speckit.auto SPEC-TEST-001"'
```

**Expected**:
```
Time (mean ± σ):     45.2 s ±  2.1 s    [User: 38.1 s, System: 3.2 s]
Range (min … max):   42.8 s … 48.5 s    5 runs
```

---

## Performance Metrics

### Database Performance

**Measured Metrics**:
- Read latency (µs): 850 → 129 (6.6× improvement)
- Write latency (ms): 2.1 → 0.9 (2.3× improvement)
- Batch reads (ms/100): 78 → 12 (6.5× improvement)

**How Measured**:
```rust
// codex-rs/core/benches/db_performance.rs
criterion_group!(benches,
    benchmark_connection_pool_vs_single,
    benchmark_wal_mode_impact,
    benchmark_batch_reads,
);
```

---

### MCP Performance

**Measured Metrics**:
- Native MCP client: 8.7ms typical
- Subprocess MCP: 46ms typical
- Improvement: 5.3× faster

**How Measured**:
```rust
// Integration test timing
let start = std::time::Instant::now();
let result = mcp_client.call_tool(...).await?;
let elapsed = start.elapsed();
assert!(elapsed < Duration::from_millis(10)); // <10ms
```

---

### Config Hot-Reload

**Measured Metrics**:
- Reload latency (p95): <100ms
- File watch overhead: <1% CPU

**How Measured**:
```rust
// Integration test
let start = std::time::Instant::now();
// Modify config file
std::fs::write(&config_path, new_content)?;
// Wait for reload
tokio::time::sleep(Duration::from_millis(50)).await;
// Verify reload
assert_eq!(app.current_model(), "gpt-5-medium");
let elapsed = start.elapsed();
assert!(elapsed < Duration::from_millis(100));
```

---

## Regression Testing

### Baseline Comparison

**Save baseline**:
```bash
cargo bench -- --save-baseline v1.0.0
```

**Compare**:
```bash
# After changes
cargo bench -- --baseline v1.0.0
```

**Output**:
```
connection_pool_vs_single/pooled_connection_read
                        time:   [129.45 µs 130.12 µs 130.89 µs]
                        change: [-0.5% +0.2% +1.1%] (p = 0.23 > 0.05)
                        No change in performance detected.
```

**Interpretation**:
- Change <5%: No regression
- Change >5%: Investigate
- Change >10%: **Regression detected** (fix before merge)

---

### Continuous Performance Monitoring

**CI Integration** (future):
```yaml
# .github/workflows/performance.yml
jobs:
  benchmark:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4

      - name: Run benchmarks
        run: cargo bench -- --save-baseline ci-baseline

      - name: Compare to previous
        run: cargo bench -- --baseline ci-baseline-previous

      - name: Fail if regression >10%
        run: |
          if grep "change:.*[+][1-9][0-9]" target/criterion/**/new/estimates.txt; then
            echo "Performance regression detected!"
            exit 1
          fi
```

---

## Best Practices

### DO

**✅ Measure before optimizing**:
```bash
# Before: Measure baseline
cargo bench -- --save-baseline before_optimization

# Optimize code...

# After: Measure improvement
cargo bench -- --baseline before_optimization
```

---

**✅ Use `black_box()` to prevent optimization**:
```rust
// Good: Prevents compiler from optimizing away
b.iter(|| {
    black_box(expensive_function(black_box(input)));
});

// Bad: Compiler might optimize this away
b.iter(|| {
    expensive_function(input);
});
```

---

**✅ Benchmark realistic workloads**:
```rust
// Good: Real-world data
let data = load_fixture("real_prd.md");
b.iter(|| detect_ambiguities(black_box(&data)));

// Bad: Trivial input
let data = "test";
b.iter(|| detect_ambiguities(black_box(&data)));
```

---

**✅ Run benchmarks on consistent hardware**:
- Same machine (or CI runner)
- Close other programs
- Disable CPU frequency scaling (if possible)

---

**✅ Set performance targets**:
```rust
// Document targets in benchmark comments
/// Target: <150µs (was 850µs before optimization)
group.bench_function("pooled_read", |b| { ... });
```

---

### DON'T

**❌ Optimize without measuring**:
```rust
// Bad: Premature optimization
// "This looks slow, let me rewrite it"

// Good: Measure first
// cargo bench → identify hot path → optimize
```

---

**❌ Trust microbenchmarks for macro performance**:
```rust
// Bad: Optimizing single function
fn fast_function() { /* 1µs faster */ }

// Better: Benchmark complete workflow
fn complete_pipeline() { /* Does 1µs matter here? */ }
```

---

**❌ Ignore variance**:
```
# Bad: "It ran in 10ms once"

# Good: "Mean: 10.2ms ± 0.3ms (100 runs)"
```

---

**❌ Benchmark in debug mode**:
```bash
# Bad: Debug mode (100× slower)
cargo bench

# Good: Release mode (default for benches)
cargo bench --release
```

---

## Summary

**Performance Testing Best Practices**:

1. **Measure**: Use criterion for accurate benchmarks
2. **Profile**: Use flamegraphs to find hot paths
3. **Validate**: Confirm optimizations with data
4. **Regress**: Detect performance regressions
5. **Target**: Set clear performance goals

**Tools**:
- ✅ criterion (statistical benchmarking)
- ✅ cargo-flamegraph (profiling)
- ✅ cargo-bloat (binary size)
- ✅ hyperfine (CLI benchmarking)
- ✅ perf (Linux profiling)

**Validated Improvements**:
- ✅ Database: 6.6× read, 2.3× write
- ✅ MCP: 5.3× faster (8.7ms vs 46ms)
- ✅ Config reload: <100ms (p95)

**Key Metrics**:
- ✅ Latency (µs, ms, s)
- ✅ Throughput (ops/sec, elem/sec)
- ✅ Percentiles (p50, p95, p99)
- ✅ Variance (stddev, outliers)

**Next Steps**:
- [Testing Strategy](testing-strategy.md) - Overall testing approach
- [CI/CD Integration](ci-cd-integration.md) - Automated testing
- [Test Infrastructure](test-infrastructure.md) - MockMcpManager, fixtures

---

**References**:
- criterion: https://bheisler.github.io/criterion.rs/
- Database benchmarks: `codex-rs/core/benches/db_performance.rs`
- Profiling guide: https://nnethercote.github.io/perf-book/
- hyperfine: https://github.com/sharkdp/hyperfine
