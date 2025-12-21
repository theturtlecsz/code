//! Spec-Kit CLI Commands
//!
//! SPEC-KIT-921: CLI adapter for spec-kit executor.
//!
//! Provides headless CLI commands that use the same SpeckitExecutor as TUI,
//! ensuring CLI/TUI parity for automation and CI.
//!
//! ## Commands
//!
//! - `code speckit status --spec <ID> [--stale-hours N] [--json]`
//! - `code speckit review --spec <ID> --stage <STAGE> [--strict-*] [--json]`
//!
//! ## Exit Codes (per REVIEW-CONTRACT.md)
//!
//! - 0: Success / proceed
//! - 1: Soft fail (warnings in strict mode)
//! - 2: Hard fail (escalation / missing artifacts in strict mode)
//! - 3: Infrastructure error

use clap::{Parser, Subcommand};
use codex_spec_kit::config::policy_toggles::PolicyToggles;
use codex_spec_kit::executor::{
    ExecutionContext, Outcome, PolicySnapshot, ReviewOptions, SpeckitCommand, SpeckitExecutor,
    TelemetryMode, render_review_dashboard, render_status_dashboard, review_warning,
    status_degraded_warning,
};
use codex_spec_kit::Stage;
use std::path::PathBuf;

/// Spec-Kit CLI — headless commands for automation and CI
#[derive(Debug, Parser)]
pub struct SpeckitCli {
    /// Working directory (defaults to current directory)
    #[arg(short = 'C', long = "cwd", value_name = "DIR")]
    pub cwd: Option<PathBuf>,

    #[command(subcommand)]
    pub command: SpeckitSubcommand,
}

#[derive(Debug, Subcommand)]
pub enum SpeckitSubcommand {
    /// Show SPEC status dashboard
    Status(StatusArgs),

    /// Evaluate stage gate artifacts
    Review(ReviewArgs),
}

/// Arguments for `speckit status` command
#[derive(Debug, Parser)]
pub struct StatusArgs {
    /// SPEC identifier (e.g., SPEC-KIT-921)
    #[arg(long = "spec", short = 's', value_name = "SPEC-ID")]
    pub spec_id: String,

    /// Hours after which telemetry is considered stale
    #[arg(long = "stale-hours", default_value = "24")]
    pub stale_hours: i64,

    /// Output as JSON instead of text
    #[arg(long = "json", short = 'j')]
    pub json: bool,
}

/// Arguments for `speckit review` command
#[derive(Debug, Parser)]
pub struct ReviewArgs {
    /// SPEC identifier (e.g., SPEC-KIT-921)
    #[arg(long = "spec", short = 's', value_name = "SPEC-ID")]
    pub spec_id: String,

    /// Stage to review (plan, tasks, implement, validate, audit, unlock)
    #[arg(long = "stage", value_name = "STAGE")]
    pub stage: String,

    /// Fail if expected artifacts are missing (exit 2)
    #[arg(long = "strict-artifacts")]
    pub strict_artifacts: bool,

    /// Treat PassedWithWarnings as exit 1
    #[arg(long = "strict-warnings")]
    pub strict_warnings: bool,

    /// Fail on parse/schema errors (exit 3)
    /// Prevents CI from passing on corrupted evidence files
    #[arg(long = "strict-schema")]
    pub strict_schema: bool,

    /// Override evidence root path (relative to repo root)
    /// Default: docs/SPEC-OPS-004-integrated-coder-hooks/evidence
    #[arg(long = "evidence-root", value_name = "PATH")]
    pub evidence_root: Option<String>,

    /// Output as JSON instead of text
    #[arg(long = "json", short = 'j')]
    pub json: bool,
}

impl SpeckitCli {
    /// Run the speckit CLI command
    pub async fn run(self) -> anyhow::Result<()> {
        let cwd = self.cwd.unwrap_or_else(|| {
            std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."))
        });

        // Resolve policy from env/config at adapter boundary (not in executor)
        let toggles = PolicyToggles::from_env_and_config();
        let policy_snapshot = PolicySnapshot {
            sidecar_critic_enabled: toggles.sidecar_critic_enabled,
            telemetry_mode: TelemetryMode::Disabled,
            legacy_voting_env_detected: toggles.legacy_voting_enabled,
        };

        let executor = SpeckitExecutor::new(ExecutionContext {
            repo_root: cwd,
            policy_snapshot: Some(policy_snapshot),
        });

        match self.command {
            SpeckitSubcommand::Status(args) => run_status(executor, args),
            SpeckitSubcommand::Review(args) => run_review(executor, args),
        }
    }
}

/// Run the status command
fn run_status(executor: SpeckitExecutor, args: StatusArgs) -> anyhow::Result<()> {
    let command = SpeckitCommand::Status {
        spec_id: args.spec_id,
        stale_hours: args.stale_hours,
    };

    match executor.execute(command) {
        Outcome::Status(report) => {
            if args.json {
                // JSON output for CI parsing
                let json = serde_json::json!({
                    "spec_id": report.spec_id,
                    "generated_at": report.generated_at.to_rfc3339(),
                    "evidence": {
                        "commands_bytes": report.evidence.commands_bytes,
                        "consensus_bytes": report.evidence.consensus_bytes,
                        "combined_bytes": report.evidence.combined_bytes,
                        "threshold": report.evidence.threshold.map(|t| format!("{t:?}")),
                    },
                    "stage_count": report.stage_snapshots.len(),
                    "warnings": report.warnings,
                });
                println!("{}", serde_json::to_string_pretty(&json)?);
            } else {
                // Text output for human consumption
                let mut lines = render_status_dashboard(&report);
                if let Some(warning) = status_degraded_warning(&report) {
                    lines.insert(1, warning);
                }
                for line in lines {
                    println!("{line}");
                }
            }
            Ok(())
        }
        Outcome::Error(err) => {
            eprintln!("Error: {err}");
            std::process::exit(3);
        }
        Outcome::Review(_) | Outcome::ReviewSkipped { .. } => {
            unreachable!("Status command should never return Review outcome")
        }
    }
}

/// Run the review command
fn run_review(executor: SpeckitExecutor, args: ReviewArgs) -> anyhow::Result<()> {
    // Parse stage
    let stage = parse_stage(&args.stage)?;

    let command = SpeckitCommand::Review {
        spec_id: args.spec_id.clone(),
        stage,
        strict_artifacts: args.strict_artifacts,
        strict_warnings: args.strict_warnings,
        strict_schema: args.strict_schema,
        evidence_root: args.evidence_root.map(std::path::PathBuf::from),
    };

    match executor.execute(command) {
        Outcome::Review(result) => {
            // Calculate exit code based on result and options
            let options = ReviewOptions {
                strict_artifacts: args.strict_artifacts,
                strict_warnings: args.strict_warnings,
                strict_schema: args.strict_schema,
                ..Default::default()
            };
            let exit_code = result.exit_code(&options);

            if args.json {
                // JSON output for CI parsing
                let json = serde_json::json!({
                    "spec_id": result.spec_id,
                    "stage": format!("{:?}", result.stage),
                    "checkpoint": format!("{:?}", result.checkpoint),
                    "verdict": format!("{:?}", result.display_verdict()),
                    "exit_code": exit_code,
                    "artifacts_collected": result.artifacts_collected,
                    "blocking_signals": result.blocking_signals.iter().map(|s| {
                        serde_json::json!({
                            "kind": format!("{:?}", s.kind),
                            "message": s.message,
                            "origin": s.origin.display_name(),
                        })
                    }).collect::<Vec<_>>(),
                    "advisory_signals": result.advisory_signals.iter().map(|s| {
                        serde_json::json!({
                            "kind": format!("{:?}", s.kind),
                            "message": s.message,
                            "origin": s.origin.display_name(),
                        })
                    }).collect::<Vec<_>>(),
                    "evidence": {
                        "verdict_json": result.evidence.verdict_json,
                        "telemetry_bundle": result.evidence.telemetry_bundle,
                        "synthesis_path": result.evidence.synthesis_path,
                        "evidence_dir": result.evidence.evidence_dir,
                    },
                });
                println!("{}", serde_json::to_string_pretty(&json)?);
            } else {
                // Text output for human consumption
                let mut lines = render_review_dashboard(&result);
                if let Some(warning) = review_warning(&result) {
                    lines.insert(1, warning);
                }
                for line in lines {
                    println!("{line}");
                }
            }

            std::process::exit(exit_code);
        }
        Outcome::ReviewSkipped {
            stage,
            reason,
            suggestion,
        } => {
            if args.json {
                let json = serde_json::json!({
                    "stage": format!("{:?}", stage),
                    "verdict": "Skipped",
                    "reason": format!("{:?}", reason),
                    "suggestion": suggestion,
                    "exit_code": if args.strict_artifacts { 2 } else { 0 },
                });
                println!("{}", serde_json::to_string_pretty(&json)?);
            } else {
                eprintln!("⚠ Review skipped for {:?}: {:?}", stage, reason);
                if let Some(hint) = suggestion {
                    eprintln!("  Suggestion: {hint}");
                }
            }

            // In strict mode, skipped = exit 2
            if args.strict_artifacts {
                std::process::exit(2);
            }
            Ok(())
        }
        Outcome::Error(err) => {
            if args.json {
                let json = serde_json::json!({
                    "error": err,
                    "exit_code": 3,
                });
                println!("{}", serde_json::to_string_pretty(&json)?);
            } else {
                eprintln!("Error: {err}");
            }
            std::process::exit(3);
        }
        Outcome::Status(_) => {
            unreachable!("Review command should never return Status outcome")
        }
    }
}

/// Parse stage from string
fn parse_stage(input: &str) -> anyhow::Result<Stage> {
    match input.to_lowercase().as_str() {
        "specify" => Ok(Stage::Specify),
        "plan" => Ok(Stage::Plan),
        "tasks" => Ok(Stage::Tasks),
        "implement" => Ok(Stage::Implement),
        "validate" => Ok(Stage::Validate),
        "audit" => Ok(Stage::Audit),
        "unlock" => Ok(Stage::Unlock),
        _ => anyhow::bail!(
            "Unknown stage '{}'. Valid stages: specify, plan, tasks, implement, validate, audit, unlock",
            input
        ),
    }
}
