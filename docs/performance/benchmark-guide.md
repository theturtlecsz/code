# Performance Benchmark Guide

SPEC-940 performance instrumentation for codex-rs.

## Quick Start

```bash
# Run all benchmarks
cd codex-rs
cargo test -p codex-core --test spec940_benchmarks -- --nocapture

# Run specific benchmark
cargo test -p codex-core --test spec940_benchmarks benchmark_spawn -- --nocapture

# Run library benchmarks
cargo test -p codex-core --lib -- benchmarks --nocapture
```

## Using BenchmarkHarness

```rust
use codex_core::benchmarks::{BenchmarkConfig, BenchmarkHarness};

#[tokio::test]
async fn my_benchmark() {
    let harness = BenchmarkHarness::new(
        BenchmarkConfig::new("operation_name")
            .iterations(10)
            .warmup(2)
    );

    let result = harness.run_async(|| async {
        // Your operation here
    }).await;

    // Print results
    eprintln!("{}", result.summary());

    // Assert thresholds
    result.assert_mean_under(50.0);  // Fail if mean > 50ms
    result.assert_max_under(100.0);  // Fail if max > 100ms
}
```

## BenchmarkResult Fields

| Field | Description |
|-------|-------------|
| `mean_ms` | Average execution time |
| `stddev_ms` | Standard deviation |
| `min_ms` | Minimum execution time |
| `max_ms` | Maximum execution time |
| `p50_ms` | 50th percentile (median) |
| `p95_ms` | 95th percentile |
| `p99_ms` | 99th percentile |
| `sample_count` | Number of successful samples |

## Comparing Results

```rust
use codex_core::report::{PerformanceReport, ComparisonResult};

// Compare two runs
let comparison = ComparisonResult::compare(&current, &baseline);

if comparison.is_regression {
    panic!("Performance regression: {}", comparison.summary());
}

// Generate comparison report
let mut current_report = PerformanceReport::new("Current");
current_report.add_result(current);

let mut baseline_report = PerformanceReport::new("Baseline");
baseline_report.add_result(baseline);

let comparison_report = current_report.compare_to_baseline(&baseline_report);
comparison_report.assert_no_regressions();
```

## Regression Detection

The framework automatically detects regressions when:
- Performance is >20% slower than baseline
- Change is statistically significant (p < 0.05)

Use in CI:
```rust
comparison_report.assert_no_regressions();  // Panics if regression detected
```

## Current Baselines (2025-11-28)

| Operation | Mean (ms) | Target (ms) | Margin |
|-----------|-----------|-------------|--------|
| spawn_echo_command | 0.92 | <50 | 54x |
| sqlite_write | 0.04 | <30 | 750x |
| config_parse | 0.05 | <10 | 200x |

See `docs/SPEC-KIT-940-performance-instrumentation/evidence/` for full baselines.

## Adding New Benchmarks

1. Create benchmark in `codex-rs/core/tests/spec940_benchmarks.rs`
2. Use `BenchmarkHarness` for measurement
3. Add assertion thresholds
4. Run and record baseline to evidence file
