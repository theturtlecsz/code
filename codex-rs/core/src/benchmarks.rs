//! Performance benchmark infrastructure for SPEC-940
//!
//! Provides statistical benchmarking with configurable iterations, warmup,
//! and comprehensive statistics (mean, stddev, percentiles).
//!
//! # Example
//!
//! ```rust,ignore
//! use codex_core::benchmarks::{BenchmarkHarness, BenchmarkConfig};
//! use std::time::Duration;
//!
//! #[tokio::test]
//! async fn benchmark_spawn() {
//!     let harness = BenchmarkHarness::new(BenchmarkConfig {
//!         name: "spawn_agent".into(),
//!         iterations: 10,
//!         warmup_iterations: 2,
//!     });
//!
//!     let result = harness.run_async(|| async {
//!         spawn_agent("test").await
//!     }).await;
//!
//!     assert!(result.mean_ms < 50.0, "Spawn took too long: {:.1}ms", result.mean_ms);
//! }
//! ```

use std::future::Future;
use std::time::{Duration, Instant};

/// Configuration for a benchmark run
#[derive(Debug, Clone)]
pub struct BenchmarkConfig {
    /// Name of the benchmark (for logging)
    pub name: String,
    /// Number of iterations to run (default: 10)
    pub iterations: usize,
    /// Number of warmup iterations to discard (default: 2)
    pub warmup_iterations: usize,
}

impl Default for BenchmarkConfig {
    fn default() -> Self {
        Self {
            name: "benchmark".into(),
            iterations: 10,
            warmup_iterations: 2,
        }
    }
}

impl BenchmarkConfig {
    /// Create a new benchmark config with the given name
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            ..Default::default()
        }
    }

    /// Set the number of iterations
    pub fn iterations(mut self, n: usize) -> Self {
        self.iterations = n;
        self
    }

    /// Set the number of warmup iterations
    pub fn warmup(mut self, n: usize) -> Self {
        self.warmup_iterations = n;
        self
    }
}

/// Benchmark execution harness
#[derive(Debug)]
pub struct BenchmarkHarness {
    config: BenchmarkConfig,
}

impl BenchmarkHarness {
    /// Create a new benchmark harness with the given configuration
    pub fn new(config: BenchmarkConfig) -> Self {
        Self { config }
    }

    /// Create a simple harness with just a name
    pub fn named(name: impl Into<String>) -> Self {
        Self::new(BenchmarkConfig::new(name))
    }

    /// Run a synchronous benchmark operation
    pub fn run_sync<F, T>(&self, mut operation: F) -> BenchmarkResult
    where
        F: FnMut() -> T,
    {
        // Warmup iterations (discard results)
        for _ in 0..self.config.warmup_iterations {
            let _ = operation();
        }

        // Collect samples
        let mut samples = Vec::with_capacity(self.config.iterations);
        for _ in 0..self.config.iterations {
            let start = Instant::now();
            let _ = operation();
            let elapsed = start.elapsed();
            samples.push(elapsed);
        }

        BenchmarkResult::from_samples(&self.config.name, &samples)
    }

    /// Run an async benchmark operation
    pub async fn run_async<F, Fut, T>(&self, mut operation: F) -> BenchmarkResult
    where
        F: FnMut() -> Fut,
        Fut: Future<Output = T>,
    {
        // Warmup iterations (discard results)
        for _ in 0..self.config.warmup_iterations {
            let _ = operation().await;
        }

        // Collect samples
        let mut samples = Vec::with_capacity(self.config.iterations);
        for _ in 0..self.config.iterations {
            let start = Instant::now();
            let _ = operation().await;
            let elapsed = start.elapsed();
            samples.push(elapsed);
        }

        BenchmarkResult::from_samples(&self.config.name, &samples)
    }

    /// Run an async benchmark operation that returns Result, logging failures
    pub async fn run_async_fallible<F, Fut, T, E>(&self, mut operation: F) -> BenchmarkResult
    where
        F: FnMut() -> Fut,
        Fut: Future<Output = Result<T, E>>,
        E: std::fmt::Debug,
    {
        // Warmup iterations (discard results)
        for _ in 0..self.config.warmup_iterations {
            let _ = operation().await;
        }

        // Collect samples
        let mut samples = Vec::with_capacity(self.config.iterations);
        let mut failures = 0usize;

        for i in 0..self.config.iterations {
            let start = Instant::now();
            let result = operation().await;
            let elapsed = start.elapsed();

            match result {
                Ok(_) => samples.push(elapsed),
                Err(e) => {
                    failures += 1;
                    tracing::warn!(
                        benchmark = %self.config.name,
                        iteration = i,
                        error = ?e,
                        "Benchmark iteration failed, excluding from stats"
                    );
                }
            }
        }

        if failures > 0 {
            tracing::info!(
                benchmark = %self.config.name,
                total = self.config.iterations,
                failures = failures,
                "Benchmark completed with failures"
            );
        }

        BenchmarkResult::from_samples(&self.config.name, &samples)
    }
}

/// Result of a benchmark run with statistical analysis
#[derive(Debug, Clone)]
pub struct BenchmarkResult {
    /// Benchmark name
    pub name: String,
    /// Mean execution time in milliseconds
    pub mean_ms: f64,
    /// Standard deviation in milliseconds
    pub stddev_ms: f64,
    /// Minimum execution time in milliseconds
    pub min_ms: f64,
    /// Maximum execution time in milliseconds
    pub max_ms: f64,
    /// 50th percentile (median) in milliseconds
    pub p50_ms: f64,
    /// 95th percentile in milliseconds
    pub p95_ms: f64,
    /// 99th percentile in milliseconds
    pub p99_ms: f64,
    /// Number of successful samples collected
    pub sample_count: usize,
    /// Raw samples in microseconds (for advanced analysis)
    samples_us: Vec<u64>,
}

impl BenchmarkResult {
    /// Create a result from duration samples
    pub fn from_samples(name: &str, samples: &[Duration]) -> Self {
        if samples.is_empty() {
            return Self::empty(name);
        }

        // Convert to microseconds for precision
        let samples_us: Vec<u64> = samples.iter().map(|d| d.as_micros() as u64).collect();

        // Calculate statistics
        let sum: u64 = samples_us.iter().sum();
        let mean_us = sum as f64 / samples_us.len() as f64;

        let variance: f64 = samples_us
            .iter()
            .map(|&x| (x as f64 - mean_us).powi(2))
            .sum::<f64>()
            / samples_us.len() as f64;
        let stddev_us = variance.sqrt();

        let mut sorted = samples_us.clone();
        sorted.sort_unstable();

        // Use pattern matching to satisfy clippy (we know it's non-empty from check above)
        let (min_us, max_us) = match (sorted.first(), sorted.last()) {
            (Some(&min), Some(&max)) => (min, max),
            _ => return Self::empty(name), // Unreachable given is_empty check, but keeps clippy happy
        };

        // Percentile calculations
        let p50_us = percentile(&sorted, 50);
        let p95_us = percentile(&sorted, 95);
        let p99_us = percentile(&sorted, 99);

        Self {
            name: name.to_string(),
            mean_ms: mean_us / 1000.0,
            stddev_ms: stddev_us / 1000.0,
            min_ms: min_us as f64 / 1000.0,
            max_ms: max_us as f64 / 1000.0,
            p50_ms: p50_us as f64 / 1000.0,
            p95_ms: p95_us as f64 / 1000.0,
            p99_ms: p99_us as f64 / 1000.0,
            sample_count: samples_us.len(),
            samples_us,
        }
    }

    /// Create an empty result (for failed benchmarks)
    fn empty(name: &str) -> Self {
        Self {
            name: name.to_string(),
            mean_ms: 0.0,
            stddev_ms: 0.0,
            min_ms: 0.0,
            max_ms: 0.0,
            p50_ms: 0.0,
            p95_ms: 0.0,
            p99_ms: 0.0,
            sample_count: 0,
            samples_us: Vec::new(),
        }
    }

    /// Check if the benchmark has any samples
    pub fn is_empty(&self) -> bool {
        self.sample_count == 0
    }

    /// Get the raw samples in microseconds
    pub fn samples_us(&self) -> &[u64] {
        &self.samples_us
    }

    /// Format as a single-line summary
    pub fn summary(&self) -> String {
        format!(
            "{}: {:.2}±{:.2}ms (min: {:.2}, p50: {:.2}, p95: {:.2}, max: {:.2}, n={})",
            self.name,
            self.mean_ms,
            self.stddev_ms,
            self.min_ms,
            self.p50_ms,
            self.p95_ms,
            self.max_ms,
            self.sample_count
        )
    }

    /// Log the result via tracing
    pub fn log(&self) {
        tracing::info!(
            benchmark = %self.name,
            mean_ms = self.mean_ms,
            stddev_ms = self.stddev_ms,
            min_ms = self.min_ms,
            max_ms = self.max_ms,
            p50_ms = self.p50_ms,
            p95_ms = self.p95_ms,
            p99_ms = self.p99_ms,
            sample_count = self.sample_count,
            "Benchmark completed"
        );
    }

    /// Assert mean is under threshold, panicking with details if not
    pub fn assert_mean_under(&self, max_ms: f64) {
        assert!(
            self.mean_ms < max_ms,
            "Benchmark '{}' FAILED: mean {:.2}ms exceeds {:.2}ms threshold\n{}",
            self.name,
            self.mean_ms,
            max_ms,
            self.summary()
        );
    }

    /// Assert max is under threshold (catches outliers)
    pub fn assert_max_under(&self, max_ms: f64) {
        assert!(
            self.max_ms < max_ms,
            "Benchmark '{}' FAILED: max {:.2}ms exceeds {:.2}ms threshold (outlier)\n{}",
            self.name,
            self.max_ms,
            max_ms,
            self.summary()
        );
    }

    /// Assert stddev is under threshold (consistency check)
    pub fn assert_stddev_under(&self, max_ms: f64) {
        assert!(
            self.stddev_ms < max_ms,
            "Benchmark '{}' FAILED: stddev {:.2}ms exceeds {:.2}ms (inconsistent)\n{}",
            self.name,
            self.stddev_ms,
            max_ms,
            self.summary()
        );
    }

    /// Calculate speedup factor compared to another result
    pub fn speedup_vs(&self, baseline: &BenchmarkResult) -> f64 {
        if self.mean_ms == 0.0 {
            return 0.0;
        }
        baseline.mean_ms / self.mean_ms
    }
}

/// Calculate percentile from sorted samples
fn percentile(sorted: &[u64], p: usize) -> u64 {
    if sorted.is_empty() {
        return 0;
    }
    let idx = (sorted.len() * p / 100).saturating_sub(1).min(sorted.len() - 1);
    sorted[idx]
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;

    #[test]
    fn test_benchmark_result_from_samples() {
        let samples = vec![
            Duration::from_millis(10),
            Duration::from_millis(12),
            Duration::from_millis(11),
            Duration::from_millis(15),
            Duration::from_millis(9),
        ];

        let result = BenchmarkResult::from_samples("test", &samples);

        assert_eq!(result.sample_count, 5);
        assert!((result.mean_ms - 11.4).abs() < 0.01, "mean: {}", result.mean_ms);
        assert!((result.min_ms - 9.0).abs() < 0.01, "min: {}", result.min_ms);
        assert!((result.max_ms - 15.0).abs() < 0.01, "max: {}", result.max_ms);
    }

    #[test]
    fn test_benchmark_result_empty() {
        let result = BenchmarkResult::from_samples("empty", &[]);
        assert!(result.is_empty());
        assert_eq!(result.sample_count, 0);
    }

    #[test]
    fn test_benchmark_result_single_sample() {
        let samples = vec![Duration::from_millis(100)];
        let result = BenchmarkResult::from_samples("single", &samples);

        assert_eq!(result.sample_count, 1);
        assert!((result.mean_ms - 100.0).abs() < 0.01);
        assert!((result.stddev_ms).abs() < 0.01, "single sample has no variance");
    }

    #[test]
    fn test_benchmark_result_percentiles() {
        // 10 samples: 1, 2, 3, ..., 10
        let samples: Vec<Duration> = (1..=10).map(|i| Duration::from_millis(i)).collect();
        let result = BenchmarkResult::from_samples("percentiles", &samples);

        assert_eq!(result.sample_count, 10);
        // p50 (median) should be around 5
        assert!(result.p50_ms >= 4.0 && result.p50_ms <= 6.0, "p50: {}", result.p50_ms);
        // p95 should be around 9-10
        assert!(result.p95_ms >= 8.0 && result.p95_ms <= 10.0, "p95: {}", result.p95_ms);
    }

    #[test]
    fn test_benchmark_harness_sync() {
        let harness = BenchmarkHarness::new(BenchmarkConfig {
            name: "sync_test".into(),
            iterations: 5,
            warmup_iterations: 1,
        });

        let mut counter = 0;
        let result = harness.run_sync(|| {
            counter += 1;
            std::thread::sleep(Duration::from_millis(1));
        });

        // 1 warmup + 5 iterations = 6 total calls
        assert_eq!(counter, 6);
        assert_eq!(result.sample_count, 5);
        assert!(result.mean_ms >= 1.0, "mean: {}", result.mean_ms);
    }

    #[tokio::test]
    async fn test_benchmark_harness_async() {
        let harness = BenchmarkHarness::new(BenchmarkConfig {
            name: "async_test".into(),
            iterations: 5,
            warmup_iterations: 1,
        });

        let result = harness
            .run_async(|| async {
                tokio::time::sleep(Duration::from_millis(1)).await;
            })
            .await;

        assert_eq!(result.sample_count, 5);
        assert!(result.mean_ms >= 1.0, "mean: {}", result.mean_ms);
    }

    #[tokio::test]
    async fn test_benchmark_harness_fallible() {
        let harness = BenchmarkHarness::new(BenchmarkConfig {
            name: "fallible_test".into(),
            iterations: 10,
            warmup_iterations: 1,
        });

        let mut call_count = 0;
        let result = harness
            .run_async_fallible(|| {
                call_count += 1;
                async move {
                    if call_count % 3 == 0 {
                        Err("simulated failure")
                    } else {
                        Ok(42)
                    }
                }
            })
            .await;

        // Some iterations should have failed
        assert!(result.sample_count < 11, "expected some failures");
        assert!(result.sample_count >= 6, "too many failures: {}", result.sample_count);
    }

    #[test]
    fn test_benchmark_config_builder() {
        let config = BenchmarkConfig::new("builder_test")
            .iterations(20)
            .warmup(3);

        assert_eq!(config.name, "builder_test");
        assert_eq!(config.iterations, 20);
        assert_eq!(config.warmup_iterations, 3);
    }

    #[test]
    fn test_benchmark_result_summary() {
        let samples: Vec<Duration> = (1..=5).map(|i| Duration::from_millis(i * 10)).collect();
        let result = BenchmarkResult::from_samples("summary_test", &samples);

        let summary = result.summary();
        assert!(summary.contains("summary_test"));
        assert!(summary.contains("n=5"));
    }

    #[test]
    fn test_speedup_calculation() {
        let baseline_samples: Vec<Duration> = (0..10).map(|_| Duration::from_millis(100)).collect();
        let improved_samples: Vec<Duration> = (0..10).map(|_| Duration::from_millis(10)).collect();

        let baseline = BenchmarkResult::from_samples("baseline", &baseline_samples);
        let improved = BenchmarkResult::from_samples("improved", &improved_samples);

        let speedup = improved.speedup_vs(&baseline);
        assert!((speedup - 10.0).abs() < 0.1, "speedup: {}", speedup);
    }

    #[test]
    fn test_assert_mean_under_passes() {
        let samples: Vec<Duration> = (0..5).map(|_| Duration::from_millis(10)).collect();
        let result = BenchmarkResult::from_samples("assert_test", &samples);
        result.assert_mean_under(50.0); // Should not panic
    }

    #[test]
    #[should_panic(expected = "FAILED")]
    fn test_assert_mean_under_fails() {
        let samples: Vec<Duration> = (0..5).map(|_| Duration::from_millis(100)).collect();
        let result = BenchmarkResult::from_samples("assert_fail_test", &samples);
        result.assert_mean_under(50.0); // Should panic
    }
}
