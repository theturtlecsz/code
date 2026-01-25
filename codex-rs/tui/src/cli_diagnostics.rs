//! SPEC-KIT-979: Headless CLI Diagnostics
//!
//! Provides headless execution for:
//! - A/B evaluation harness (`--eval-ab`)
//! - Capsule diagnostics (`--capsule-doctor`)
//!
//! These commands exit immediately after completion without starting TUI.
//!
//! ## Exit Codes
//! - 0: Success (all checks pass, gates pass)
//! - 1: Error/failure (gate failures, test failures)
//! - 2: Configuration error (missing prerequisites, invalid config)
//!
//! ## Decision IDs
//! - D39: Dual-backend runtime flag for A/B comparison
//! - D40: Parity gates for migration

use crate::cli::Cli;
use crate::memvid_adapter::{
    CapsuleHandle, DiagnosticResult, default_capsule_path, run_ab_harness_synthetic,
};
use std::path::Path;

/// Exit codes for headless diagnostic commands.
pub mod exit_codes {
    /// Success - all checks/gates pass
    pub const SUCCESS: i32 = 0;
    /// Error/failure - gate failures, test failures
    pub const ERROR: i32 = 1;
    /// Configuration error - missing prerequisites, invalid config
    pub const CONFIG_ERROR: i32 = 2;
}

// =============================================================================
// Capsule Doctor
// =============================================================================

/// Run capsule diagnostics in headless mode.
///
/// Checks:
/// - Capsule existence
/// - Lock status
/// - Header integrity
/// - Version compatibility
///
/// ## Output
/// - JSON mode: Structured diagnostic report
/// - Human mode: Colored status lines
///
/// ## Exit Codes
/// - 0: All checks pass (or warnings only)
/// - 1: At least one error detected
#[allow(clippy::print_stdout, clippy::print_stderr)]
pub fn run_capsule_doctor(cli: &Cli, cwd: &Path) -> i32 {
    let capsule_path = default_capsule_path(cwd);
    let results = CapsuleHandle::doctor(&capsule_path);

    let has_errors = results
        .iter()
        .any(|r| matches!(r, DiagnosticResult::Error(_, _)));
    let has_warnings = results
        .iter()
        .any(|r| matches!(r, DiagnosticResult::Warning(_, _)));

    if cli.json_output {
        // JSON output mode
        let json_results: Vec<serde_json::Value> = results
            .iter()
            .map(|r| match r {
                DiagnosticResult::Ok(msg) => serde_json::json!({
                    "status": "ok",
                    "message": msg,
                }),
                DiagnosticResult::Warning(msg, fix) => serde_json::json!({
                    "status": "warning",
                    "message": msg,
                    "fix": fix,
                }),
                DiagnosticResult::Error(msg, fix) => serde_json::json!({
                    "status": "error",
                    "message": msg,
                    "fix": fix,
                }),
            })
            .collect();

        let overall = if has_errors {
            "fail"
        } else if has_warnings {
            "warn"
        } else {
            "pass"
        };

        let output = serde_json::json!({
            "capsule_path": capsule_path.display().to_string(),
            "checks": json_results,
            "overall": overall,
            "exit_code": if has_errors { exit_codes::ERROR } else { exit_codes::SUCCESS },
        });

        println!("{}", serde_json::to_string_pretty(&output).unwrap());
    } else {
        // Human-readable output mode
        println!();
        println!("Capsule Diagnostics");
        println!("===================");
        println!("Path: {}", capsule_path.display());
        println!();

        for result in &results {
            match result {
                DiagnosticResult::Ok(msg) => {
                    println!("\x1b[32m[OK]\x1b[0m   {}", msg);
                }
                DiagnosticResult::Warning(msg, fix) => {
                    println!("\x1b[33m[WARN]\x1b[0m {}", msg);
                    println!("       Fix: {}", fix);
                }
                DiagnosticResult::Error(msg, fix) => {
                    println!("\x1b[31m[FAIL]\x1b[0m {}", msg);
                    println!("       Fix: {}", fix);
                }
            }
        }

        println!();
        if has_errors {
            println!("Result: \x1b[31mFAIL\x1b[0m (errors detected)");
        } else if has_warnings {
            println!("Result: \x1b[33mWARN\x1b[0m (warnings detected)");
        } else {
            println!("Result: \x1b[32mPASS\x1b[0m");
        }
    }

    if has_errors {
        exit_codes::ERROR
    } else {
        exit_codes::SUCCESS
    }
}

// =============================================================================
// A/B Evaluation Harness
// =============================================================================

/// Run A/B evaluation harness in headless mode.
///
/// Compares local-memory vs memvid backends on golden queries and
/// produces evaluation reports.
///
/// ## Current Behavior (Phase 0)
/// Uses synthetic test data since setting up dual backends requires
/// additional infrastructure. Real A/B evaluation will be enabled
/// when backend connectivity is established.
///
/// ## Output
/// - JSON mode: Full ABReport as JSON
/// - Human mode: Markdown summary + file paths
///
/// ## Exit Codes
/// - 0: Parity gates pass (B meets baseline, latency acceptable)
/// - 1: Parity gates fail
/// - 2: Configuration error (cannot create backends)
#[allow(clippy::print_stdout, clippy::print_stderr)]
pub async fn run_eval_ab(cli: &Cli, cwd: &Path) -> i32 {
    // Determine output directory
    let output_dir = cli
        .output_dir
        .clone()
        .unwrap_or_else(|| cwd.join(".speckit").join("eval"));

    eprintln!("info: Running A/B evaluation with synthetic data (Phase 0 mode).");
    eprintln!("      Output directory: {}", output_dir.display());
    eprintln!();

    // Run synthetic harness
    // Note: Real A/B evaluation with dual backends will be enabled in Phase 1
    // when backend connectivity infrastructure is established.
    match run_ab_harness_synthetic(&output_dir, 10).await {
        Ok(result) => {
            if cli.json_output {
                // Output full JSON report
                match serde_json::to_string_pretty(&result.report) {
                    Ok(json) => println!("{}", json),
                    Err(e) => {
                        eprintln!("Error serializing report: {}", e);
                        return exit_codes::ERROR;
                    }
                }
            } else {
                // Human-readable output
                println!("{}", result.report.to_markdown());
                println!();
                println!("Reports saved to:");
                println!("  JSON:     {}", result.json_path.display());
                println!("  Markdown: {}", result.md_path.display());
                println!();

                // Verdict
                if result.meets_baseline && result.latency_acceptable {
                    println!("\x1b[32mVERDICT: PASS\x1b[0m");
                    println!("  - Retrieval quality: meets baseline");
                    println!(
                        "  - P95 latency: {}ms (< 250ms threshold)",
                        result.report.p95_latency_b().as_millis()
                    );
                } else {
                    println!("\x1b[31mVERDICT: FAIL\x1b[0m");
                    if !result.meets_baseline {
                        println!("  - Retrieval quality: below baseline");
                    }
                    if !result.latency_acceptable {
                        println!(
                            "  - P95 latency: {}ms (>= 250ms threshold)",
                            result.report.p95_latency_b().as_millis()
                        );
                    }
                }
            }

            if result.meets_baseline && result.latency_acceptable {
                exit_codes::SUCCESS
            } else {
                exit_codes::ERROR
            }
        }
        Err(e) => {
            let error_msg = format!("A/B evaluation failed: {}", e);
            if cli.json_output {
                println!(
                    "{}",
                    serde_json::json!({
                        "success": false,
                        "error": error_msg,
                        "exit_code": exit_codes::CONFIG_ERROR,
                    })
                );
            } else {
                eprintln!("Error: {}", error_msg);
            }
            exit_codes::CONFIG_ERROR
        }
    }
}

// =============================================================================
// Tests
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_capsule_doctor_missing_capsule() {
        let temp_dir = TempDir::new().unwrap();
        let cli = Cli {
            json_output: true,
            ..Default::default()
        };

        let exit_code = run_capsule_doctor(&cli, temp_dir.path());

        // Missing capsule should return error
        assert_eq!(exit_code, exit_codes::ERROR);
    }

    #[tokio::test]
    async fn test_eval_ab_synthetic_runs() {
        let temp_dir = TempDir::new().unwrap();
        let cli = Cli {
            eval_ab: true,
            json_output: true,
            output_dir: Some(temp_dir.path().join("eval")),
            ..Default::default()
        };

        let exit_code = run_eval_ab(&cli, temp_dir.path()).await;

        // Synthetic A/B should pass (self-comparison)
        assert_eq!(exit_code, exit_codes::SUCCESS);
    }
}
