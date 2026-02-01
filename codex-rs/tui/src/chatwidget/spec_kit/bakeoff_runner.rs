//! SPEC-KIT-978: Bakeoff Runner for Reflex vs Cloud Comparison
//!
//! Executes N trials comparing local reflex inference vs cloud inference.
//! Records latency, success rate, and JSON compliance for data-driven routing.
//!
//! ## Usage
//! ```rust,ignore
//! let config = BakeoffConfig::default();
//! let report = run_bakeoff(&cwd, &config).await?;
//! report.write_files(&cwd)?;
//! ```
//!
//! ## Output Files
//! - `.speckit/eval/reflex-bakeoff-<timestamp>.json` - Raw metrics
//! - `.speckit/eval/reflex-bakeoff-<timestamp>.md` - Human-readable report

use chrono::{DateTime, Utc};
use codex_stage0::ReflexConfig;
use serde::{Deserialize, Serialize};
use std::path::Path;
use std::time::Instant;

use super::reflex_client::{ChatMessage, ReflexClient, ReflexError};
use super::reflex_metrics::{BakeoffStats, ModeStats, get_metrics_db};

/// Configuration for bakeoff execution
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BakeoffConfig {
    /// Number of trials per mode (reflex and cloud each run N times)
    pub trial_count: u32,
    /// P95 latency threshold in milliseconds (reflex must be below this)
    pub p95_latency_threshold_ms: u64,
    /// Minimum success rate percentage (0-100)
    pub success_rate_threshold_pct: u8,
    /// Minimum JSON compliance percentage (0-100)
    pub json_compliance_threshold_pct: u8,
    /// Minimum samples required for threshold evaluation
    pub min_samples: u64,
}

impl Default for BakeoffConfig {
    fn default() -> Self {
        Self {
            trial_count: 10,
            p95_latency_threshold_ms: 2000,
            success_rate_threshold_pct: 85,
            json_compliance_threshold_pct: 90,
            min_samples: 5,
        }
    }
}

/// Single trial result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TrialResult {
    pub trial_id: u32,
    pub mode: String,
    pub latency_ms: u64,
    pub success: bool,
    pub json_compliant: bool,
    pub content_length: usize,
    pub error: Option<String>,
    pub timestamp: DateTime<Utc>,
}

/// Bakeoff evaluation result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BakeoffEvaluation {
    pub passes_thresholds: bool,
    pub p95_check: ThresholdCheck,
    pub success_rate_check: ThresholdCheck,
    pub json_compliance_check: ThresholdCheck,
    pub recommendation: String,
}

/// Individual threshold check result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ThresholdCheck {
    pub name: String,
    pub passes: bool,
    pub actual: f64,
    pub threshold: f64,
    pub message: String,
}

/// Complete bakeoff report
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BakeoffReport {
    pub timestamp: DateTime<Utc>,
    pub config: BakeoffConfig,
    pub trials: Vec<TrialResult>,
    pub stats: BakeoffStats,
    pub evaluation: BakeoffEvaluation,
    pub report_id: String,
}

impl BakeoffReport {
    /// Write report files to .speckit/eval/
    pub fn write_files(&self, cwd: &Path) -> Result<(String, String), std::io::Error> {
        let eval_dir = cwd.join(".speckit/eval");
        std::fs::create_dir_all(&eval_dir)?;

        let timestamp_str = self.timestamp.format("%Y%m%d-%H%M%S").to_string();
        let json_path = eval_dir.join(format!("reflex-bakeoff-{}.json", timestamp_str));
        let md_path = eval_dir.join(format!("reflex-bakeoff-{}.md", timestamp_str));

        // Write JSON report
        let json_content = serde_json::to_string_pretty(self).map_err(std::io::Error::other)?;
        std::fs::write(&json_path, &json_content)?;

        // Write Markdown report
        let md_content = self.generate_markdown();
        std::fs::write(&md_path, &md_content)?;

        Ok((
            json_path.to_string_lossy().to_string(),
            md_path.to_string_lossy().to_string(),
        ))
    }

    /// Generate human-readable Markdown report
    fn generate_markdown(&self) -> String {
        let mut md = String::new();

        // Header
        md.push_str("# Reflex Bakeoff Report\n\n");
        md.push_str(&format!(
            "**Generated:** {}\n",
            self.timestamp.format("%Y-%m-%d %H:%M:%S UTC")
        ));
        md.push_str(&format!("**Report ID:** {}\n\n", self.report_id));

        // Overall result
        let status_emoji = if self.evaluation.passes_thresholds {
            "✅"
        } else {
            "❌"
        };
        md.push_str(&format!(
            "## Result: {} {}\n\n",
            status_emoji, self.evaluation.recommendation
        ));

        // Configuration
        md.push_str("## Configuration\n\n");
        md.push_str("| Parameter | Value |\n");
        md.push_str("|-----------|-------|\n");
        md.push_str(&format!("| Trial Count | {} |\n", self.config.trial_count));
        md.push_str(&format!(
            "| P95 Latency Threshold | {}ms |\n",
            self.config.p95_latency_threshold_ms
        ));
        md.push_str(&format!(
            "| Success Rate Threshold | {}% |\n",
            self.config.success_rate_threshold_pct
        ));
        md.push_str(&format!(
            "| JSON Compliance Threshold | {}% |\n",
            self.config.json_compliance_threshold_pct
        ));
        md.push('\n');

        // Threshold Checks
        md.push_str("## Threshold Checks\n\n");
        md.push_str("| Check | Status | Actual | Threshold |\n");
        md.push_str("|-------|--------|--------|----------|\n");

        for check in [
            &self.evaluation.p95_check,
            &self.evaluation.success_rate_check,
            &self.evaluation.json_compliance_check,
        ] {
            let status = if check.passes { "✅ PASS" } else { "❌ FAIL" };
            md.push_str(&format!(
                "| {} | {} | {:.1} | {:.1} |\n",
                check.name, status, check.actual, check.threshold
            ));
        }
        md.push('\n');

        // Statistics by mode
        md.push_str("## Statistics\n\n");

        if let Some(ref reflex) = self.stats.reflex {
            md.push_str("### Reflex (Local Inference)\n\n");
            md.push_str(&format_mode_stats(reflex));
        }

        if let Some(ref cloud) = self.stats.cloud {
            md.push_str("### Cloud Inference\n\n");
            md.push_str(&format_mode_stats(cloud));
        }

        // Comparison
        if let (Some(reflex), Some(cloud)) = (&self.stats.reflex, &self.stats.cloud) {
            md.push_str("### Comparison\n\n");
            let latency_ratio = if reflex.p95_latency_ms > 0 {
                cloud.p95_latency_ms as f64 / reflex.p95_latency_ms as f64
            } else {
                0.0
            };
            md.push_str(&format!(
                "- **Latency Improvement:** Reflex is {:.1}x faster (P95)\n",
                latency_ratio
            ));
            md.push_str(&format!(
                "- **Success Rate Delta:** {:.1}%\n",
                reflex.success_rate - cloud.success_rate
            ));
            md.push_str(&format!(
                "- **JSON Compliance Delta:** {:.1}%\n",
                reflex.json_compliance_rate - cloud.json_compliance_rate
            ));
            md.push('\n');
        }

        // Trial details
        md.push_str("## Trial Details\n\n");
        md.push_str("| Trial | Mode | Latency (ms) | Success | JSON | Error |\n");
        md.push_str("|-------|------|--------------|---------|------|-------|\n");

        for trial in &self.trials {
            let success = if trial.success { "✅" } else { "❌" };
            let json = if trial.json_compliant { "✅" } else { "❌" };
            let error = trial
                .error
                .as_ref()
                .map(|e| e.chars().take(30).collect::<String>())
                .unwrap_or_default();
            md.push_str(&format!(
                "| {} | {} | {} | {} | {} | {} |\n",
                trial.trial_id, trial.mode, trial.latency_ms, success, json, error
            ));
        }

        md
    }
}

/// Format mode statistics as Markdown
fn format_mode_stats(stats: &ModeStats) -> String {
    let mut s = String::new();
    s.push_str("| Metric | Value |\n");
    s.push_str("|--------|-------|\n");
    s.push_str(&format!("| Total Attempts | {} |\n", stats.total_attempts));
    s.push_str(&format!("| Success Rate | {:.1}% |\n", stats.success_rate));
    s.push_str(&format!(
        "| JSON Compliance | {:.1}% |\n",
        stats.json_compliance_rate
    ));
    s.push_str(&format!(
        "| Avg Latency | {:.0}ms |\n",
        stats.avg_latency_ms
    ));
    s.push_str(&format!("| P50 Latency | {}ms |\n", stats.p50_latency_ms));
    s.push_str(&format!("| P95 Latency | {}ms |\n", stats.p95_latency_ms));
    s.push_str(&format!("| P99 Latency | {}ms |\n", stats.p99_latency_ms));
    s.push_str(&format!("| Min Latency | {}ms |\n", stats.min_latency_ms));
    s.push_str(&format!("| Max Latency | {}ms |\n", stats.max_latency_ms));
    s.push('\n');
    s
}

/// Standard test prompts for bakeoff trials
fn bakeoff_test_prompts() -> Vec<(String, String)> {
    vec![
        (
            "implement_simple".to_string(),
            r#"You are an Implement agent for SPEC-BAKEOFF. Analyze the following task and provide your implementation plan.

Task: Add a logging utility function that supports log levels (debug, info, warn, error).

Return your response as JSON with:
- stage: "implement"
- confidence: your confidence (0.0-1.0)
- decision: your implementation approach
- reasoning: why this approach"#.to_string()
        ),
        (
            "validate_simple".to_string(),
            r#"You are a Validate agent for SPEC-BAKEOFF. Review the following implementation and provide your assessment.

Implementation:
```rust
fn log(level: &str, message: &str) {
    println!("[{}] {}", level, message);
}
```

Return your response as JSON with:
- stage: "validate"
- confidence: your confidence (0.0-1.0)
- decision: "approve" or "reject"
- reasoning: your assessment"#.to_string()
        ),
    ]
}

/// Run a single trial with the given prompt
async fn run_single_trial(
    client: &ReflexClient,
    trial_id: u32,
    _prompt_name: &str,
    prompt_content: &str,
    schema: &serde_json::Value,
) -> TrialResult {
    let messages = vec![
        ChatMessage {
            role: "system".to_string(),
            content: "You are an expert agent. Return valid JSON as instructed.".to_string(),
        },
        ChatMessage {
            role: "user".to_string(),
            content: prompt_content.to_string(),
        },
    ];

    let start = Instant::now();
    let result = client.chat_completion_json(&messages, schema).await;
    let latency_ms = start.elapsed().as_millis() as u64;

    match result {
        Ok(response) => TrialResult {
            trial_id,
            mode: "reflex".to_string(),
            latency_ms,
            success: true,
            json_compliant: response.json_compliant,
            content_length: response.content.len(),
            error: None,
            timestamp: Utc::now(),
        },
        Err(e) => TrialResult {
            trial_id,
            mode: "reflex".to_string(),
            latency_ms,
            success: false,
            json_compliant: false,
            content_length: 0,
            error: Some(format!("{}", e)),
            timestamp: Utc::now(),
        },
    }
}

/// Agent output schema for bakeoff trials
fn bakeoff_schema() -> serde_json::Value {
    serde_json::json!({
        "name": "agent_output",
        "strict": false,
        "schema": {
            "type": "object",
            "properties": {
                "stage": {
                    "type": "string",
                    "description": "The stage this agent output belongs to"
                },
                "confidence": {
                    "type": "number",
                    "description": "Agent's confidence (0.0 to 1.0)"
                },
                "decision": {
                    "type": "string",
                    "description": "Agent's decision"
                },
                "reasoning": {
                    "type": "string",
                    "description": "Explanation of reasoning"
                }
            },
            "required": ["stage"],
            "additionalProperties": true
        }
    })
}

/// Evaluate bakeoff results against thresholds
fn evaluate_bakeoff(stats: &BakeoffStats, config: &BakeoffConfig) -> BakeoffEvaluation {
    let reflex = stats.reflex.as_ref();

    // P95 latency check
    let p95_check = if let Some(r) = reflex {
        ThresholdCheck {
            name: "P95 Latency".to_string(),
            passes: r.p95_latency_ms <= config.p95_latency_threshold_ms,
            actual: r.p95_latency_ms as f64,
            threshold: config.p95_latency_threshold_ms as f64,
            message: if r.p95_latency_ms <= config.p95_latency_threshold_ms {
                "P95 latency within threshold".to_string()
            } else {
                format!(
                    "P95 latency {}ms exceeds {}ms threshold",
                    r.p95_latency_ms, config.p95_latency_threshold_ms
                )
            },
        }
    } else {
        ThresholdCheck {
            name: "P95 Latency".to_string(),
            passes: false,
            actual: 0.0,
            threshold: config.p95_latency_threshold_ms as f64,
            message: "No reflex data available".to_string(),
        }
    };

    // Success rate check
    let success_rate_check = if let Some(r) = reflex {
        ThresholdCheck {
            name: "Success Rate".to_string(),
            passes: r.success_rate >= config.success_rate_threshold_pct as f64,
            actual: r.success_rate,
            threshold: config.success_rate_threshold_pct as f64,
            message: if r.success_rate >= config.success_rate_threshold_pct as f64 {
                "Success rate meets threshold".to_string()
            } else {
                format!(
                    "Success rate {:.1}% below {}% threshold",
                    r.success_rate, config.success_rate_threshold_pct
                )
            },
        }
    } else {
        ThresholdCheck {
            name: "Success Rate".to_string(),
            passes: false,
            actual: 0.0,
            threshold: config.success_rate_threshold_pct as f64,
            message: "No reflex data available".to_string(),
        }
    };

    // JSON compliance check
    let json_compliance_check = if let Some(r) = reflex {
        ThresholdCheck {
            name: "JSON Compliance".to_string(),
            passes: r.json_compliance_rate >= config.json_compliance_threshold_pct as f64,
            actual: r.json_compliance_rate,
            threshold: config.json_compliance_threshold_pct as f64,
            message: if r.json_compliance_rate >= config.json_compliance_threshold_pct as f64 {
                "JSON compliance meets threshold".to_string()
            } else {
                format!(
                    "JSON compliance {:.1}% below {}% threshold",
                    r.json_compliance_rate, config.json_compliance_threshold_pct
                )
            },
        }
    } else {
        ThresholdCheck {
            name: "JSON Compliance".to_string(),
            passes: false,
            actual: 0.0,
            threshold: config.json_compliance_threshold_pct as f64,
            message: "No reflex data available".to_string(),
        }
    };

    let passes_all = p95_check.passes && success_rate_check.passes && json_compliance_check.passes;

    let recommendation = if passes_all {
        "Reflex is ready for production routing".to_string()
    } else {
        let mut issues = Vec::new();
        if !p95_check.passes {
            issues.push("latency");
        }
        if !success_rate_check.passes {
            issues.push("success rate");
        }
        if !json_compliance_check.passes {
            issues.push("JSON compliance");
        }
        format!(
            "Reflex needs improvement: {} below threshold",
            issues.join(", ")
        )
    };

    BakeoffEvaluation {
        passes_thresholds: passes_all,
        p95_check,
        success_rate_check,
        json_compliance_check,
        recommendation,
    }
}

/// Run the full bakeoff evaluation
///
/// Executes N trials through reflex inference, records metrics, and generates report.
pub async fn run_bakeoff(
    _cwd: &Path,
    config: &BakeoffConfig,
    reflex_config: &ReflexConfig,
) -> Result<BakeoffReport, BakeoffRunnerError> {
    let report_id = uuid::Uuid::new_v4().to_string()[..8].to_string();
    let timestamp = Utc::now();

    tracing::info!(
        "SPEC-KIT-978: Starting bakeoff run {} with {} trials",
        report_id,
        config.trial_count
    );

    // Create reflex client
    let client = ReflexClient::new(reflex_config)?;
    let schema = bakeoff_schema();
    let prompts = bakeoff_test_prompts();

    // Get metrics database
    let db = get_metrics_db()?;

    let mut trials = Vec::new();
    let mut trial_id = 0u32;

    // Run trials for each prompt
    for (prompt_name, prompt_content) in &prompts {
        for _ in 0..config.trial_count {
            trial_id += 1;

            let trial =
                run_single_trial(&client, trial_id, prompt_name, prompt_content, &schema).await;

            // Record to metrics database
            let _ = db.record_reflex_attempt(
                "SPEC-BAKEOFF",
                &report_id,
                trial.latency_ms,
                trial.success,
                trial.json_compliant,
            );

            tracing::debug!(
                "SPEC-KIT-978: Trial {}/{} - {}ms, success={}, json={}",
                trial_id,
                config.trial_count * prompts.len() as u32,
                trial.latency_ms,
                trial.success,
                trial.json_compliant
            );

            trials.push(trial);
        }
    }

    // Compute statistics from trials
    let stats = compute_stats_from_trials(&trials);

    // Evaluate against thresholds
    let evaluation = evaluate_bakeoff(&stats, config);

    tracing::info!(
        "SPEC-KIT-978: Bakeoff complete - {} trials, passes={}",
        trials.len(),
        evaluation.passes_thresholds
    );

    Ok(BakeoffReport {
        timestamp,
        config: config.clone(),
        trials,
        stats,
        evaluation,
        report_id,
    })
}

/// Compute statistics from trial results
fn compute_stats_from_trials(trials: &[TrialResult]) -> BakeoffStats {
    let reflex_trials: Vec<_> = trials.iter().filter(|t| t.mode == "reflex").collect();

    let reflex = if !reflex_trials.is_empty() {
        let total = reflex_trials.len() as u64;
        let success_count = reflex_trials.iter().filter(|t| t.success).count() as u64;
        let json_count = reflex_trials.iter().filter(|t| t.json_compliant).count() as u64;

        let mut latencies: Vec<u64> = reflex_trials.iter().map(|t| t.latency_ms).collect();
        latencies.sort();

        let p50_idx = (latencies.len() as f64 * 0.50).ceil() as usize - 1;
        let p95_idx = (latencies.len() as f64 * 0.95).ceil() as usize - 1;
        let p99_idx = (latencies.len() as f64 * 0.99).ceil() as usize - 1;

        Some(super::reflex_metrics::ModeStats {
            mode: "reflex".to_string(),
            total_attempts: total,
            success_count,
            success_rate: (success_count as f64 / total as f64) * 100.0,
            json_compliant_count: json_count,
            json_compliance_rate: (json_count as f64 / total as f64) * 100.0,
            avg_latency_ms: latencies.iter().sum::<u64>() as f64 / latencies.len() as f64,
            p50_latency_ms: latencies
                .get(p50_idx.min(latencies.len() - 1))
                .copied()
                .unwrap_or(0),
            p95_latency_ms: latencies
                .get(p95_idx.min(latencies.len() - 1))
                .copied()
                .unwrap_or(0),
            p99_latency_ms: latencies
                .get(p99_idx.min(latencies.len() - 1))
                .copied()
                .unwrap_or(0),
            min_latency_ms: latencies.first().copied().unwrap_or(0),
            max_latency_ms: latencies.last().copied().unwrap_or(0),
        })
    } else {
        None
    };

    BakeoffStats {
        reflex,
        cloud: None, // Cloud trials not implemented in this version
        period_start: Utc::now().format("%Y-%m-%d %H:%M:%S").to_string(),
        period_end: Utc::now().format("%Y-%m-%d %H:%M:%S").to_string(),
        total_attempts: trials.len() as u64,
    }
}

/// Bakeoff runner errors
#[derive(Debug, thiserror::Error)]
pub enum BakeoffRunnerError {
    #[error("Reflex client error: {0}")]
    ReflexError(#[from] ReflexError),

    #[error("Metrics database error: {0}")]
    MetricsError(String),

    #[error("IO error: {0}")]
    IoError(#[from] std::io::Error),
}

impl From<rusqlite::Error> for BakeoffRunnerError {
    fn from(e: rusqlite::Error) -> Self {
        BakeoffRunnerError::MetricsError(e.to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_bakeoff_config_defaults() {
        let config = BakeoffConfig::default();
        assert_eq!(config.trial_count, 10);
        assert_eq!(config.p95_latency_threshold_ms, 2000);
        assert_eq!(config.success_rate_threshold_pct, 85);
        assert_eq!(config.json_compliance_threshold_pct, 90);
    }

    #[test]
    fn test_bakeoff_evaluation_passes() {
        let stats = BakeoffStats {
            reflex: Some(super::super::reflex_metrics::ModeStats {
                mode: "reflex".to_string(),
                total_attempts: 20,
                success_count: 19,
                success_rate: 95.0,
                json_compliant_count: 18,
                json_compliance_rate: 90.0,
                avg_latency_ms: 150.0,
                p50_latency_ms: 140,
                p95_latency_ms: 200,
                p99_latency_ms: 250,
                min_latency_ms: 100,
                max_latency_ms: 300,
            }),
            cloud: None,
            period_start: "2024-01-01 00:00:00".to_string(),
            period_end: "2024-01-01 01:00:00".to_string(),
            total_attempts: 20,
        };

        let config = BakeoffConfig::default();
        let evaluation = evaluate_bakeoff(&stats, &config);

        assert!(evaluation.passes_thresholds);
        assert!(evaluation.p95_check.passes);
        assert!(evaluation.success_rate_check.passes);
        assert!(evaluation.json_compliance_check.passes);
    }

    #[test]
    fn test_bakeoff_evaluation_fails_latency() {
        let stats = BakeoffStats {
            reflex: Some(super::super::reflex_metrics::ModeStats {
                mode: "reflex".to_string(),
                total_attempts: 20,
                success_count: 19,
                success_rate: 95.0,
                json_compliant_count: 18,
                json_compliance_rate: 90.0,
                avg_latency_ms: 3000.0,
                p50_latency_ms: 2500,
                p95_latency_ms: 3500, // Exceeds 2000ms threshold
                p99_latency_ms: 4000,
                min_latency_ms: 2000,
                max_latency_ms: 5000,
            }),
            cloud: None,
            period_start: "2024-01-01 00:00:00".to_string(),
            period_end: "2024-01-01 01:00:00".to_string(),
            total_attempts: 20,
        };

        let config = BakeoffConfig::default();
        let evaluation = evaluate_bakeoff(&stats, &config);

        assert!(!evaluation.passes_thresholds);
        assert!(!evaluation.p95_check.passes);
        assert!(evaluation.success_rate_check.passes);
        assert!(evaluation.json_compliance_check.passes);
    }

    #[test]
    fn test_compute_stats_from_trials() {
        let trials = vec![
            TrialResult {
                trial_id: 1,
                mode: "reflex".to_string(),
                latency_ms: 100,
                success: true,
                json_compliant: true,
                content_length: 500,
                error: None,
                timestamp: Utc::now(),
            },
            TrialResult {
                trial_id: 2,
                mode: "reflex".to_string(),
                latency_ms: 200,
                success: true,
                json_compliant: true,
                content_length: 500,
                error: None,
                timestamp: Utc::now(),
            },
            TrialResult {
                trial_id: 3,
                mode: "reflex".to_string(),
                latency_ms: 150,
                success: true,
                json_compliant: false,
                content_length: 500,
                error: None,
                timestamp: Utc::now(),
            },
        ];

        let stats = compute_stats_from_trials(&trials);

        let reflex = stats.reflex.unwrap();
        assert_eq!(reflex.total_attempts, 3);
        assert_eq!(reflex.success_count, 3);
        assert!((reflex.success_rate - 100.0).abs() < 0.01);
        assert_eq!(reflex.json_compliant_count, 2);
        assert!((reflex.json_compliance_rate - 66.67).abs() < 0.1);
    }

    #[test]
    fn test_bakeoff_report_markdown_generation() {
        let config = BakeoffConfig::default();
        let stats = BakeoffStats {
            reflex: Some(super::super::reflex_metrics::ModeStats {
                mode: "reflex".to_string(),
                total_attempts: 10,
                success_count: 10,
                success_rate: 100.0,
                json_compliant_count: 10,
                json_compliance_rate: 100.0,
                avg_latency_ms: 150.0,
                p50_latency_ms: 140,
                p95_latency_ms: 200,
                p99_latency_ms: 250,
                min_latency_ms: 100,
                max_latency_ms: 300,
            }),
            cloud: None,
            period_start: "2024-01-01 00:00:00".to_string(),
            period_end: "2024-01-01 01:00:00".to_string(),
            total_attempts: 10,
        };

        let evaluation = evaluate_bakeoff(&stats, &config);

        let report = BakeoffReport {
            timestamp: Utc::now(),
            config,
            trials: vec![],
            stats,
            evaluation,
            report_id: "test123".to_string(),
        };

        let md = report.generate_markdown();

        assert!(md.contains("# Reflex Bakeoff Report"));
        assert!(md.contains("test123"));
        assert!(md.contains("Threshold Checks"));
        assert!(md.contains("P95 Latency"));
    }
}
