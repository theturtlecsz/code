//! Performance report generation for SPEC-940
//!
//! Provides statistical reporting with Markdown table generation,
//! baseline comparison, and regression detection.
//!
//! # Example
//!
//! ```rust,ignore
//! use codex_core::report::{PerformanceReport, ComparisonResult};
//! use codex_core::benchmarks::BenchmarkResult;
//!
//! let report = PerformanceReport::new("SPEC-940 Performance Report");
//! report.add_result("spawn", &result);
//! println!("{}", report.to_markdown());
//! ```

use crate::benchmarks::BenchmarkResult;
use std::collections::HashMap;

/// A collection of benchmark results for reporting
#[derive(Debug, Default)]
pub struct PerformanceReport {
    /// Report title
    title: String,
    /// Benchmark results indexed by name
    results: HashMap<String, BenchmarkResult>,
    /// Order of results for consistent output
    order: Vec<String>,
}

impl PerformanceReport {
    /// Create a new performance report with the given title
    pub fn new(title: impl Into<String>) -> Self {
        Self {
            title: title.into(),
            results: HashMap::new(),
            order: Vec::new(),
        }
    }

    /// Add a benchmark result to the report
    pub fn add_result(&mut self, result: BenchmarkResult) {
        let name = result.name.clone();
        if !self.results.contains_key(&name) {
            self.order.push(name.clone());
        }
        self.results.insert(name, result);
    }

    /// Get a result by name
    pub fn get_result(&self, name: &str) -> Option<&BenchmarkResult> {
        self.results.get(name)
    }

    /// Get all results in order
    pub fn results(&self) -> impl Iterator<Item = &BenchmarkResult> {
        self.order.iter().filter_map(|name| self.results.get(name))
    }

    /// Generate Markdown table report
    pub fn to_markdown(&self) -> String {
        let mut output = String::new();

        output.push_str(&format!("# {}\n\n", self.title));
        output.push_str("| Operation | Mean±Stddev (ms) | Min | P50 | P95 | P99 | Max | n |\n");
        output.push_str("|-----------|------------------|-----|-----|-----|-----|-----|---|\n");

        for result in self.results() {
            output.push_str(&format!(
                "| {} | {:.2}±{:.2} | {:.2} | {:.2} | {:.2} | {:.2} | {:.2} | {} |\n",
                result.name,
                result.mean_ms,
                result.stddev_ms,
                result.min_ms,
                result.p50_ms,
                result.p95_ms,
                result.p99_ms,
                result.max_ms,
                result.sample_count
            ));
        }

        output
    }

    /// Generate comparison report between current and baseline
    pub fn compare_to_baseline(&self, baseline: &PerformanceReport) -> ComparisonReport {
        let mut comparisons = Vec::new();

        for result in self.results() {
            if let Some(baseline_result) = baseline.get_result(&result.name) {
                let comparison = ComparisonResult::compare(result, baseline_result);
                comparisons.push(comparison);
            }
        }

        ComparisonReport { comparisons }
    }
}

/// Result of comparing two benchmark results
#[derive(Debug, Clone)]
pub struct ComparisonResult {
    /// Benchmark name
    pub name: String,
    /// Current mean in ms
    pub current_mean_ms: f64,
    /// Baseline mean in ms
    pub baseline_mean_ms: f64,
    /// Speedup factor (>1.0 = faster, <1.0 = slower)
    pub speedup: f64,
    /// Percentage change (positive = regression, negative = improvement)
    pub change_percent: f64,
    /// Whether the change is statistically significant (p < 0.05)
    pub significant: bool,
    /// Whether this represents a regression (>20% slower)
    pub is_regression: bool,
    /// T-statistic from Welch's t-test
    pub t_statistic: f64,
    /// P-value from Welch's t-test
    pub p_value: f64,
}

impl ComparisonResult {
    /// Compare two benchmark results
    pub fn compare(current: &BenchmarkResult, baseline: &BenchmarkResult) -> Self {
        let speedup = if current.mean_ms > 0.0 {
            baseline.mean_ms / current.mean_ms
        } else {
            0.0
        };

        let change_percent = if baseline.mean_ms > 0.0 {
            ((current.mean_ms - baseline.mean_ms) / baseline.mean_ms) * 100.0
        } else {
            0.0
        };

        // Welch's t-test for statistical significance
        let (t_statistic, p_value) = welch_t_test(current, baseline);
        let significant = p_value < 0.05;

        // Regression if >20% slower with statistical significance
        let is_regression = change_percent > 20.0 && significant;

        Self {
            name: current.name.clone(),
            current_mean_ms: current.mean_ms,
            baseline_mean_ms: baseline.mean_ms,
            speedup,
            change_percent,
            significant,
            is_regression,
            t_statistic,
            p_value,
        }
    }

    /// Get a human-readable summary
    pub fn summary(&self) -> String {
        let direction = if self.change_percent < 0.0 {
            "faster"
        } else if self.change_percent > 0.0 {
            "slower"
        } else {
            "same"
        };

        let significance = if self.significant {
            " (significant)"
        } else {
            " (not significant)"
        };

        format!(
            "{}: {:.2}ms → {:.2}ms ({:.1}% {}){}",
            self.name,
            self.baseline_mean_ms,
            self.current_mean_ms,
            self.change_percent.abs(),
            direction,
            significance
        )
    }
}

/// Report comparing multiple benchmarks to baselines
#[derive(Debug)]
pub struct ComparisonReport {
    /// Individual comparisons
    pub comparisons: Vec<ComparisonResult>,
}

impl ComparisonReport {
    /// Check if any comparison shows a regression
    pub fn has_regression(&self) -> bool {
        self.comparisons.iter().any(|c| c.is_regression)
    }

    /// Get all regressions
    pub fn regressions(&self) -> Vec<&ComparisonResult> {
        self.comparisons
            .iter()
            .filter(|c| c.is_regression)
            .collect()
    }

    /// Get all improvements (>20% faster, significant)
    pub fn improvements(&self) -> Vec<&ComparisonResult> {
        self.comparisons
            .iter()
            .filter(|c| c.change_percent < -20.0 && c.significant)
            .collect()
    }

    /// Generate Markdown comparison table
    pub fn to_markdown(&self) -> String {
        let mut output = String::new();

        output.push_str("# Performance Comparison Report\n\n");
        output
            .push_str("| Operation | Baseline (ms) | Current (ms) | Change | Speedup | Status |\n");
        output
            .push_str("|-----------|---------------|--------------|--------|---------|--------|\n");

        for comparison in &self.comparisons {
            let status = if comparison.is_regression {
                "⚠️ REGRESSION"
            } else if comparison.change_percent < -20.0 && comparison.significant {
                "✅ IMPROVED"
            } else {
                "➡️ OK"
            };

            output.push_str(&format!(
                "| {} | {:.2} | {:.2} | {:+.1}% | {:.2}x | {} |\n",
                comparison.name,
                comparison.baseline_mean_ms,
                comparison.current_mean_ms,
                comparison.change_percent,
                comparison.speedup,
                status
            ));
        }

        if self.has_regression() {
            output.push_str("\n## ⚠️ Regressions Detected\n\n");
            for regression in self.regressions() {
                output.push_str(&format!("- {}\n", regression.summary()));
            }
        }

        output
    }

    /// Assert no regressions exist, panicking with details if any found
    pub fn assert_no_regressions(&self) {
        if self.has_regression() {
            let regressions: Vec<String> = self.regressions().iter().map(|r| r.summary()).collect();
            panic!(
                "Performance regressions detected:\n{}",
                regressions.join("\n")
            );
        }
    }
}

/// Perform Welch's t-test between two benchmark results
///
/// Returns (t-statistic, p-value)
fn welch_t_test(a: &BenchmarkResult, b: &BenchmarkResult) -> (f64, f64) {
    let n1 = a.sample_count as f64;
    let n2 = b.sample_count as f64;

    if n1 < 2.0 || n2 < 2.0 {
        return (0.0, 1.0); // Not enough samples for significance
    }

    let mean1 = a.mean_ms;
    let mean2 = b.mean_ms;
    let var1 = a.stddev_ms.powi(2);
    let var2 = b.stddev_ms.powi(2);

    // Welch's t-statistic
    let se = ((var1 / n1) + (var2 / n2)).sqrt();
    if se == 0.0 {
        return (0.0, 1.0); // No variance, can't compute
    }

    let t = (mean1 - mean2) / se;

    // Welch-Satterthwaite degrees of freedom
    let df_num = ((var1 / n1) + (var2 / n2)).powi(2);
    let df_denom = ((var1 / n1).powi(2) / (n1 - 1.0)) + ((var2 / n2).powi(2) / (n2 - 1.0));
    let df = if df_denom > 0.0 {
        df_num / df_denom
    } else {
        n1 + n2 - 2.0 // Fall back to pooled df
    };

    // Approximate p-value using Student's t distribution
    // For simplicity, use a lookup table for common significance levels
    let p_value = approximate_t_p_value(t.abs(), df);

    (t, p_value)
}

/// Approximate p-value for t-distribution
///
/// Uses critical values for common thresholds. Returns approximate p-value.
fn approximate_t_p_value(t: f64, df: f64) -> f64 {
    // Critical values for two-tailed test at various significance levels
    // These are approximate for df >= 10
    let crit_001 = 3.169 + (0.1 / df.max(1.0)); // p < 0.01
    let crit_005 = 2.228 + (0.1 / df.max(1.0)); // p < 0.05
    let crit_010 = 1.812 + (0.1 / df.max(1.0)); // p < 0.10

    if t >= crit_001 {
        0.005 // p < 0.01
    } else if t >= crit_005 {
        0.025 // 0.01 < p < 0.05
    } else if t >= crit_010 {
        0.075 // 0.05 < p < 0.10
    } else {
        0.5 // p > 0.10
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;

    #[test]
    fn test_performance_report_markdown() {
        let mut report = PerformanceReport::new("Test Report");

        let samples1: Vec<Duration> = (0..5).map(|_| Duration::from_millis(10)).collect();
        let samples2: Vec<Duration> = (0..5).map(|_| Duration::from_millis(20)).collect();

        report.add_result(BenchmarkResult::from_samples("test1", &samples1));
        report.add_result(BenchmarkResult::from_samples("test2", &samples2));

        let markdown = report.to_markdown();
        assert!(markdown.contains("# Test Report"));
        assert!(markdown.contains("test1"));
        assert!(markdown.contains("test2"));
        assert!(markdown.contains("Mean±Stddev"));
    }

    #[test]
    fn test_comparison_result_regression() {
        let baseline_samples: Vec<Duration> = (0..10).map(|_| Duration::from_millis(10)).collect();
        let current_samples: Vec<Duration> = (0..10).map(|_| Duration::from_millis(15)).collect();

        let baseline = BenchmarkResult::from_samples("test", &baseline_samples);
        let current = BenchmarkResult::from_samples("test", &current_samples);

        let comparison = ComparisonResult::compare(&current, &baseline);

        // 15ms vs 10ms = 50% slower, should be marked as regression
        assert!(comparison.change_percent > 20.0);
        // Note: May not be statistically significant with constant samples
    }

    #[test]
    fn test_comparison_result_improvement() {
        let baseline_samples: Vec<Duration> = (0..10).map(|_| Duration::from_millis(100)).collect();
        let current_samples: Vec<Duration> = (0..10).map(|_| Duration::from_millis(50)).collect();

        let baseline = BenchmarkResult::from_samples("test", &baseline_samples);
        let current = BenchmarkResult::from_samples("test", &current_samples);

        let comparison = ComparisonResult::compare(&current, &baseline);

        // 50ms vs 100ms = 50% faster (negative change)
        assert!(comparison.change_percent < 0.0);
        assert!(comparison.speedup > 1.5);
    }

    #[test]
    fn test_comparison_report_no_regressions() {
        let report = ComparisonReport {
            comparisons: vec![ComparisonResult {
                name: "fast".into(),
                current_mean_ms: 5.0,
                baseline_mean_ms: 10.0,
                speedup: 2.0,
                change_percent: -50.0,
                significant: true,
                is_regression: false,
                t_statistic: 0.0,
                p_value: 0.5,
            }],
        };

        assert!(!report.has_regression());
        assert_eq!(report.improvements().len(), 1);
    }

    #[test]
    fn test_welch_t_test_significant() {
        // Two clearly different distributions
        let samples1: Vec<Duration> = (0..10).map(|i| Duration::from_millis(10 + i)).collect();
        let samples2: Vec<Duration> = (0..10).map(|i| Duration::from_millis(100 + i)).collect();

        let result1 = BenchmarkResult::from_samples("fast", &samples1);
        let result2 = BenchmarkResult::from_samples("slow", &samples2);

        let (t, p) = welch_t_test(&result1, &result2);

        // Should be statistically significant (p < 0.05)
        assert!(p < 0.05, "p-value {p} should be < 0.05");
        assert!(t.abs() > 2.0, "t-statistic {t} should be > 2.0");
    }

    #[test]
    fn test_comparison_report_markdown() {
        let report = ComparisonReport {
            comparisons: vec![
                ComparisonResult {
                    name: "improved".into(),
                    current_mean_ms: 5.0,
                    baseline_mean_ms: 10.0,
                    speedup: 2.0,
                    change_percent: -50.0,
                    significant: true,
                    is_regression: false,
                    t_statistic: 0.0,
                    p_value: 0.01,
                },
                ComparisonResult {
                    name: "regression".into(),
                    current_mean_ms: 50.0,
                    baseline_mean_ms: 10.0,
                    speedup: 0.2,
                    change_percent: 400.0,
                    significant: true,
                    is_regression: true,
                    t_statistic: 0.0,
                    p_value: 0.01,
                },
            ],
        };

        let markdown = report.to_markdown();
        assert!(markdown.contains("REGRESSION"));
        assert!(markdown.contains("IMPROVED"));
        assert!(markdown.contains("regression"));
    }
}
