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
//!
//! ## JSON Schema Versioning
//!
//! All JSON outputs include:
//! - `schema_version`: Integer, bumped only on breaking changes
//! - `tool_version`: Cargo version + git sha for debugging

use clap::{Parser, Subcommand};
use codex_spec_kit::Stage;
use codex_spec_kit::config::policy_toggles::PolicyToggles;
use codex_spec_kit::executor::{
    ExecutionContext, MigrateStatus, Outcome, PolicySnapshot, ReviewOptions, ReviewSignal,
    RunOverallStatus, SpeckitCommand, SpeckitExecutor, StageResolution, TelemetryMode,
    render_review_dashboard, render_status_dashboard, review_warning, status_degraded_warning,
};
use codex_tui::memvid_adapter::{
    CapsuleConfig, CapsuleHandle, CheckpointId, DiagnosticResult,
};
use std::io::Write;
use std::path::PathBuf;

/// Schema version for JSON outputs.
/// Bump ONLY on breaking changes (removed/renamed fields, semantic changes).
/// Additive changes (new fields) do NOT require a version bump.
const SCHEMA_VERSION: u32 = 1;

/// Get tool version string with git sha for debugging.
/// Format: "{cargo_version}+{git_sha}" or just "{cargo_version}" if no git info.
///
/// SPEC-KIT-921 P4: Build-time only SHA injection (no runtime git).
/// Set SPECKIT_GIT_SHA or GIT_SHA environment variable at build time for SHA suffix.
fn tool_version() -> String {
    let base_version = codex_version::version();
    // Build-time only: no runtime git fallback for determinism
    let git_sha = option_env!("SPECKIT_GIT_SHA")
        .or(option_env!("GIT_SHA"))
        .unwrap_or("");

    if git_sha.is_empty() {
        base_version.to_string()
    } else {
        format!("{base_version}+{git_sha}")
    }
}

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

    /// Create a new SPEC directory structure
    ///
    /// SPEC-KIT-921 P6-A: Specify command creates SPEC directory with PRD.md.
    /// This is the first stage of the SPEC lifecycle.
    Specify(SpecifyArgs),

    /// Validate SPEC prerequisites and execute plan stage (dry-run by default)
    ///
    /// SPEC-KIT-921 P3-B: CLI plan command for CI validation.
    /// Validates SPEC exists, checks prerequisites, runs guardrails.
    /// Use --no-dry-run to actually trigger agent execution (TUI only).
    Plan(PlanArgs),

    /// Validate SPEC prerequisites and execute tasks stage (dry-run by default)
    ///
    /// SPEC-KIT-921 P4-C: CLI tasks command for CI validation.
    /// Validates SPEC exists, checks that plan.md exists, runs guardrails.
    /// Use --no-dry-run to actually trigger agent execution (TUI only).
    Tasks(TasksArgs),

    /// Validate SPEC prerequisites and execute implement stage (dry-run by default)
    ///
    /// SPEC-KIT-921 P5-A: CLI implement command for CI validation.
    /// Validates SPEC exists, checks that tasks.md exists, runs guardrails.
    /// Use --no-dry-run to actually trigger agent execution (TUI only).
    Implement(ImplementArgs),

    /// Validate SPEC prerequisites and execute validate stage (dry-run by default)
    ///
    /// SPEC-KIT-921 P5-B: CLI validate command for CI validation.
    /// Validates SPEC exists, checks implementation artifacts, runs guardrails.
    /// Use --no-dry-run to actually trigger agent execution (TUI only).
    Validate(ValidateStageArgs),

    /// Validate SPEC prerequisites and execute audit stage (dry-run by default)
    ///
    /// SPEC-KIT-921 P5-B: CLI audit command for CI validation.
    /// Validates SPEC exists, checks validation artifacts, runs guardrails.
    /// Use --no-dry-run to actually trigger agent execution (TUI only).
    Audit(AuditArgs),

    /// Validate SPEC prerequisites and execute unlock stage (dry-run by default)
    ///
    /// SPEC-KIT-921 P5-B: CLI unlock command for CI validation.
    /// Validates SPEC exists, checks audit artifacts, runs guardrails.
    /// Use --no-dry-run to actually trigger agent execution (TUI only).
    Unlock(UnlockArgs),

    /// Validate multiple stages in a batch (dry-run only)
    ///
    /// SPEC-KIT-921 P7-A: Run command validates stages from --from to --to.
    /// This is validation-only (no agent spawning) - a readiness check for CI.
    /// Reports aggregated outcome with per-stage results.
    Run(RunArgs),

    /// Migrate legacy spec.md to PRD.md
    ///
    /// SPEC-KIT-921 P7-B: Migration command for legacy spec.md files.
    /// Creates PRD.md with migration header, leaves spec.md intact.
    Migrate(MigrateArgs),

    /// Capsule operations (MV2 persistent storage)
    ///
    /// SPEC-KIT-971: Headless CLI for capsule management.
    /// Provides doctor, stats, checkpoints, commit, and resolve-uri commands.
    Capsule(CapsuleArgs),
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

    /// Show human-readable explanation of exit code decision
    /// Explains why the review passed or failed
    #[arg(long = "explain")]
    pub explain: bool,

    /// Output as JSON instead of text
    #[arg(long = "json", short = 'j')]
    pub json: bool,
}

/// Arguments for `speckit specify` command
///
/// SPEC-KIT-921 P6-A: Specify command creates SPEC directory structure.
#[derive(Debug, Parser)]
pub struct SpecifyArgs {
    /// SPEC identifier to create (e.g., SPEC-KIT-999)
    #[arg(long = "spec", short = 's', value_name = "SPEC-ID")]
    pub spec_id: String,

    /// Actually create files (default is dry-run)
    /// Use --execute to actually create the SPEC directory
    #[arg(long = "execute")]
    pub execute: bool,

    /// Output as JSON instead of text
    #[arg(long = "json", short = 'j')]
    pub json: bool,
}

/// Arguments for `speckit plan` command
///
/// SPEC-KIT-921 P4-A: Plan command is locked to plan stage.
/// Use `speckit tasks` for tasks stage, `speckit implement` for implement, etc.
#[derive(Debug, Parser)]
pub struct PlanArgs {
    /// SPEC identifier (e.g., SPEC-KIT-921)
    #[arg(long = "spec", short = 's', value_name = "SPEC-ID")]
    pub spec_id: String,

    /// Dry-run mode: validate only, don't trigger agent execution
    /// This is the default for CLI (model-free CI)
    #[arg(long = "dry-run", default_value = "true")]
    pub dry_run: bool,

    /// Strict prerequisite mode: treat missing prerequisites as blocking errors
    /// Default: false (advisory warnings only)
    /// With --strict-prereqs: missing prereqs → Blocked (exit 2)
    #[arg(long = "strict-prereqs")]
    pub strict_prereqs: bool,

    /// Output as JSON instead of text
    #[arg(long = "json", short = 'j')]
    pub json: bool,
}

/// Arguments for `speckit tasks` command
///
/// SPEC-KIT-921 P4-C: Tasks command is locked to tasks stage.
#[derive(Debug, Parser)]
pub struct TasksArgs {
    /// SPEC identifier (e.g., SPEC-KIT-921)
    #[arg(long = "spec", short = 's', value_name = "SPEC-ID")]
    pub spec_id: String,

    /// Dry-run mode: validate only, don't trigger agent execution
    /// This is the default for CLI (model-free CI)
    #[arg(long = "dry-run", default_value = "true")]
    pub dry_run: bool,

    /// Strict prerequisite mode: treat missing prerequisites as blocking errors
    #[arg(long = "strict-prereqs")]
    pub strict_prereqs: bool,

    /// Output as JSON instead of text
    #[arg(long = "json", short = 'j')]
    pub json: bool,
}

/// Arguments for `speckit implement` command
///
/// SPEC-KIT-921 P5-A: Implement command is locked to implement stage.
#[derive(Debug, Parser)]
pub struct ImplementArgs {
    /// SPEC identifier (e.g., SPEC-KIT-921)
    #[arg(long = "spec", short = 's', value_name = "SPEC-ID")]
    pub spec_id: String,

    /// Dry-run mode: validate only, don't trigger agent execution
    /// This is the default for CLI (model-free CI)
    #[arg(long = "dry-run", default_value = "true")]
    pub dry_run: bool,

    /// Strict prerequisite mode: treat missing prerequisites as blocking errors
    #[arg(long = "strict-prereqs")]
    pub strict_prereqs: bool,

    /// Output as JSON instead of text
    #[arg(long = "json", short = 'j')]
    pub json: bool,
}

/// Arguments for `speckit validate` command
///
/// SPEC-KIT-921 P5-B: Validate command is locked to validate stage.
/// Note: Named ValidateStageArgs to avoid conflict with other validation concepts.
#[derive(Debug, Parser)]
pub struct ValidateStageArgs {
    /// SPEC identifier (e.g., SPEC-KIT-921)
    #[arg(long = "spec", short = 's', value_name = "SPEC-ID")]
    pub spec_id: String,

    /// Dry-run mode: validate only, don't trigger agent execution
    /// This is the default for CLI (model-free CI)
    #[arg(long = "dry-run", default_value = "true")]
    pub dry_run: bool,

    /// Strict prerequisite mode: treat missing prerequisites as blocking errors
    #[arg(long = "strict-prereqs")]
    pub strict_prereqs: bool,

    /// Output as JSON instead of text
    #[arg(long = "json", short = 'j')]
    pub json: bool,
}

/// Arguments for `speckit audit` command
///
/// SPEC-KIT-921 P5-B: Audit command is locked to audit stage.
#[derive(Debug, Parser)]
pub struct AuditArgs {
    /// SPEC identifier (e.g., SPEC-KIT-921)
    #[arg(long = "spec", short = 's', value_name = "SPEC-ID")]
    pub spec_id: String,

    /// Dry-run mode: validate only, don't trigger agent execution
    /// This is the default for CLI (model-free CI)
    #[arg(long = "dry-run", default_value = "true")]
    pub dry_run: bool,

    /// Strict prerequisite mode: treat missing prerequisites as blocking errors
    #[arg(long = "strict-prereqs")]
    pub strict_prereqs: bool,

    /// Output as JSON instead of text
    #[arg(long = "json", short = 'j')]
    pub json: bool,
}

/// Arguments for `speckit unlock` command
///
/// SPEC-KIT-921 P5-B: Unlock command is locked to unlock stage.
#[derive(Debug, Parser)]
pub struct UnlockArgs {
    /// SPEC identifier (e.g., SPEC-KIT-921)
    #[arg(long = "spec", short = 's', value_name = "SPEC-ID")]
    pub spec_id: String,

    /// Dry-run mode: validate only, don't trigger agent execution
    /// This is the default for CLI (model-free CI)
    #[arg(long = "dry-run", default_value = "true")]
    pub dry_run: bool,

    /// Strict prerequisite mode: treat missing prerequisites as blocking errors
    #[arg(long = "strict-prereqs")]
    pub strict_prereqs: bool,

    /// Output as JSON instead of text
    #[arg(long = "json", short = 'j')]
    pub json: bool,
}

/// Arguments for `speckit run` command
///
/// SPEC-KIT-921 P7-A: Batch stage validation for CI readiness checks.
#[derive(Debug, Parser)]
pub struct RunArgs {
    /// SPEC identifier (e.g., SPEC-KIT-921)
    #[arg(long = "spec", short = 's', value_name = "SPEC-ID")]
    pub spec_id: String,

    /// Starting stage (inclusive)
    /// Valid stages: specify, plan, tasks, implement, validate, audit, unlock
    #[arg(long = "from", value_name = "STAGE")]
    pub from_stage: String,

    /// Ending stage (inclusive)
    /// Valid stages: specify, plan, tasks, implement, validate, audit, unlock
    #[arg(long = "to", value_name = "STAGE")]
    pub to_stage: String,

    /// Output as JSON instead of text
    #[arg(long = "json", short = 'j')]
    pub json: bool,
}

/// Arguments for `speckit migrate` command
///
/// SPEC-KIT-921 P7-B: Migrate legacy spec.md to PRD.md
#[derive(Debug, Parser)]
pub struct MigrateArgs {
    /// SPEC identifier (e.g., SPEC-KIT-921)
    #[arg(long = "spec", short = 's', value_name = "SPEC-ID")]
    pub spec_id: String,

    /// Check what would be migrated without making changes
    #[arg(long = "dry-run")]
    pub dry_run: bool,

    /// Output as JSON instead of text
    #[arg(long = "json", short = 'j')]
    pub json: bool,
}

// =============================================================================
// Capsule CLI Commands (SPEC-KIT-971)
// =============================================================================

/// Arguments for `speckit capsule` command
///
/// SPEC-KIT-971: Capsule management commands for MV2 persistent storage.
#[derive(Debug, Parser)]
pub struct CapsuleArgs {
    /// Path to capsule file (default: .speckit/memvid/workspace.mv2)
    #[arg(long = "capsule", value_name = "PATH")]
    pub capsule_path: Option<PathBuf>,

    #[command(subcommand)]
    pub command: CapsuleSubcommand,
}

#[derive(Debug, Subcommand)]
pub enum CapsuleSubcommand {
    /// Initialize a new capsule (workspace.mv2)
    ///
    /// SPEC-KIT-971: Creates a new MV2 capsule at the default path.
    /// Safe to run multiple times - will not overwrite existing capsule.
    Init(CapsuleInitArgs),

    /// Run capsule diagnostics
    ///
    /// Checks capsule existence, lock status, header validity, and version.
    /// Returns actionable recovery steps for any issues found.
    Doctor(CapsuleDoctorArgs),

    /// Show capsule statistics
    ///
    /// Displays size, frame counts, index status, and dedup ratio.
    Stats(CapsuleStatsArgs),

    /// List checkpoints
    ///
    /// Shows all checkpoints with timestamps, labels, and stages.
    Checkpoints(CapsuleCheckpointsArgs),

    /// List events with optional filtering
    ///
    /// SPEC-KIT-971: Shows events from the capsule with stage/type filters.
    Events(CapsuleEventsArgs),

    /// Create a manual checkpoint
    ///
    /// Creates a labeled checkpoint at the current state.
    Commit(CapsuleCommitArgs),

    /// Resolve a logical URI to its payload
    ///
    /// Looks up a mv2:// URI and optionally writes payload to a file.
    ResolveUri(CapsuleResolveUriArgs),

    /// Export capsule to per-run archive
    ///
    /// SPEC-KIT-971: Exports events and artifacts for a specific run.
    Export(CapsuleExportArgs),
}

/// Arguments for `capsule init`
#[derive(Debug, Parser)]
pub struct CapsuleInitArgs {
    /// Force creation even if capsule exists (will backup existing)
    #[arg(long = "force")]
    pub force: bool,

    /// Output as JSON instead of text
    #[arg(long = "json", short = 'j')]
    pub json: bool,
}

/// Arguments for `capsule doctor`
#[derive(Debug, Parser)]
pub struct CapsuleDoctorArgs {
    /// Output as JSON instead of text
    #[arg(long = "json", short = 'j')]
    pub json: bool,
}

/// Arguments for `capsule stats`
#[derive(Debug, Parser)]
pub struct CapsuleStatsArgs {
    /// Output as JSON instead of text
    #[arg(long = "json", short = 'j')]
    pub json: bool,
}

/// Arguments for `capsule checkpoints`
#[derive(Debug, Parser)]
pub struct CapsuleCheckpointsArgs {
    /// Output as JSON instead of text
    #[arg(long = "json", short = 'j')]
    pub json: bool,
}

/// Arguments for `capsule commit`
#[derive(Debug, Parser)]
pub struct CapsuleCommitArgs {
    /// Label for the checkpoint (required)
    #[arg(long = "label", short = 'l', value_name = "LABEL")]
    pub label: String,

    /// Output as JSON instead of text
    #[arg(long = "json", short = 'j')]
    pub json: bool,
}

/// Arguments for `capsule resolve-uri`
#[derive(Debug, Parser)]
pub struct CapsuleResolveUriArgs {
    /// The logical URI to resolve (mv2://...)
    #[arg(value_name = "URI")]
    pub uri: String,

    /// Resolve as of a specific checkpoint
    #[arg(long = "as-of", value_name = "CHECKPOINT")]
    pub as_of: Option<String>,

    /// Write payload to file instead of stdout
    #[arg(long = "out", short = 'o', value_name = "PATH")]
    pub out: Option<PathBuf>,

    /// Output as JSON instead of payload bytes
    #[arg(long = "json", short = 'j')]
    pub json: bool,
}

/// Arguments for `capsule events`
///
/// SPEC-KIT-971: List events with optional filtering.
#[derive(Debug, Parser)]
pub struct CapsuleEventsArgs {
    /// Filter by stage (e.g., "plan", "implement")
    #[arg(long = "stage", short = 's', value_name = "STAGE")]
    pub stage: Option<String>,

    /// Filter by event type (e.g., "StageTransition", "PolicySnapshotRef")
    #[arg(long = "type", short = 't', value_name = "TYPE")]
    pub event_type: Option<String>,

    /// Filter by spec ID
    #[arg(long = "spec", value_name = "SPEC-ID")]
    pub spec_id: Option<String>,

    /// Filter by run ID
    #[arg(long = "run", value_name = "RUN-ID")]
    pub run_id: Option<String>,

    /// Limit number of results
    #[arg(long = "limit", short = 'n', value_name = "N")]
    pub limit: Option<usize>,

    /// Output as JSON instead of text
    #[arg(long = "json", short = 'j')]
    pub json: bool,
}

/// Arguments for `capsule export`
///
/// SPEC-KIT-971: Export capsule to per-run archive.
#[derive(Debug, Parser)]
pub struct CapsuleExportArgs {
    /// Spec ID to export
    #[arg(long = "spec", short = 's', value_name = "SPEC-ID")]
    pub spec_id: String,

    /// Run ID to export
    #[arg(long = "run", short = 'r', value_name = "RUN-ID")]
    pub run_id: String,

    /// Output directory for the export (default: .speckit/exports/)
    #[arg(long = "out", short = 'o', value_name = "PATH")]
    pub out: Option<PathBuf>,

    /// Output as JSON instead of text
    #[arg(long = "json", short = 'j')]
    pub json: bool,
}

impl SpeckitCli {
    /// Run the speckit CLI command
    pub async fn run(self) -> anyhow::Result<()> {
        let cwd = self
            .cwd
            .unwrap_or_else(|| std::env::current_dir().unwrap_or_else(|_| PathBuf::from(".")));

        // Handle capsule commands separately (don't need executor)
        if let SpeckitSubcommand::Capsule(args) = self.command {
            return run_capsule(cwd, args);
        }

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
            SpeckitSubcommand::Specify(args) => run_specify(executor, args),
            SpeckitSubcommand::Plan(args) => run_plan(executor, args),
            SpeckitSubcommand::Tasks(args) => run_tasks(executor, args),
            SpeckitSubcommand::Implement(args) => run_implement(executor, args),
            SpeckitSubcommand::Validate(args) => run_validate(executor, args),
            SpeckitSubcommand::Audit(args) => run_audit(executor, args),
            SpeckitSubcommand::Unlock(args) => run_unlock(executor, args),
            SpeckitSubcommand::Run(args) => run_run(executor, args),
            SpeckitSubcommand::Migrate(args) => run_migrate(executor, args),
            SpeckitSubcommand::Capsule(_) => unreachable!("Capsule handled above"),
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
                // JSON output for CI parsing - comprehensive structure
                let stages: Vec<_> = report
                    .stage_snapshots
                    .iter()
                    .map(|s| {
                        serde_json::json!({
                            "stage": s.stage.display(),
                            "cue": format!("{:?}", s.cue),
                            "is_stale": s.is_stale,
                            "has_guardrail": s.guardrail.is_some(),
                            "agent_count": s.consensus.agents.len(),
                            "disagreement": s.consensus.disagreement,
                            "notes": s.notes,
                        })
                    })
                    .collect();

                let top_entries: Vec<_> = report
                    .evidence
                    .top_entries
                    .iter()
                    .map(|e| {
                        serde_json::json!({
                            "path": e.path,
                            "bytes": e.bytes,
                        })
                    })
                    .collect();

                let json = serde_json::json!({
                    "schema_version": SCHEMA_VERSION,
                    "tool_version": tool_version(),
                    "spec_id": report.spec_id,
                    "generated_at": report.generated_at.to_rfc3339(),
                    "stale_hours": report.stale_cutoff.num_hours(),
                    "packet": {
                        "directory": report.packet.directory.as_ref().map(|p| p.display().to_string()),
                        "docs": report.packet.docs.iter().map(|(k, v)| (*k, *v)).collect::<std::collections::HashMap<_, _>>(),
                    },
                    "tracker": report.tracker_row.as_ref().map(|row| {
                        serde_json::json!({
                            "status": row.status,
                            "branch": row.branch,
                            "last_validation": row.last_validation,
                        })
                    }),
                    "stages": stages,
                    "evidence": {
                        "commands_bytes": report.evidence.commands_bytes,
                        "consensus_bytes": report.evidence.consensus_bytes,
                        "combined_bytes": report.evidence.combined_bytes,
                        "threshold": report.evidence.threshold.map(|t| format!("{t:?}")),
                        "latest_artifact": report.evidence.latest_artifact.map(|dt| dt.to_rfc3339()),
                        "top_entries": top_entries,
                    },
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
        Outcome::Stage(_) => {
            unreachable!("Status command should never return Stage outcome")
        }
        Outcome::Specify(_) => {
            unreachable!("Status command should never return Specify outcome")
        }
        Outcome::Run(_) => {
            unreachable!("Status command should never return Run outcome")
        }
        Outcome::Migrate(_) => {
            unreachable!("Status command should never return Migrate outcome")
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
                let mut json = serde_json::json!({
                    "schema_version": SCHEMA_VERSION,
                    "tool_version": tool_version(),
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

                // Add explanation if requested
                if args.explain {
                    let explanation = explain_review_exit_code(
                        exit_code,
                        &result.blocking_signals,
                        &result.advisory_signals,
                        &options,
                    );
                    if let Some(obj) = json.as_object_mut() {
                        obj.insert(
                            "explanation".to_string(),
                            serde_json::json!({
                                "summary": explanation.summary,
                                "reasons": explanation.reasons,
                                "flags_active": explanation.flags_active,
                            }),
                        );
                    }
                }

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

                // Add explanation if requested
                if args.explain {
                    println!();
                    let explanation = explain_review_exit_code(
                        exit_code,
                        &result.blocking_signals,
                        &result.advisory_signals,
                        &options,
                    );
                    println!("## Exit Code Explanation");
                    println!("Exit code: {exit_code}");
                    println!("Summary: {}", explanation.summary);
                    if !explanation.flags_active.is_empty() {
                        println!("Flags: {}", explanation.flags_active.join(", "));
                    }
                    if !explanation.reasons.is_empty() {
                        println!("Reasons:");
                        for reason in &explanation.reasons {
                            println!("  - {reason}");
                        }
                    }
                }
            }

            std::process::exit(exit_code);
        }
        Outcome::ReviewSkipped {
            stage,
            reason,
            suggestion,
        } => {
            let exit_code = if args.strict_artifacts { 2 } else { 0 };

            if args.json {
                let mut json = serde_json::json!({
                    "schema_version": SCHEMA_VERSION,
                    "tool_version": tool_version(),
                    "stage": format!("{:?}", stage),
                    "verdict": "Skipped",
                    "reason": format!("{:?}", reason),
                    "suggestion": suggestion,
                    "exit_code": exit_code,
                });

                if args.explain
                    && let Some(obj) = json.as_object_mut()
                {
                    obj.insert(
                        "explanation".to_string(),
                        serde_json::json!({
                            "summary": if args.strict_artifacts {
                                "Review skipped with --strict-artifacts: missing artifacts treated as failure"
                            } else {
                                "Review skipped: no artifacts to evaluate (exit 0 in default mode)"
                            },
                            "reasons": [format!("{reason:?}")],
                            "flags_active": if args.strict_artifacts {
                                vec!["--strict-artifacts"]
                            } else {
                                vec![]
                            },
                        }),
                    );
                }

                println!("{}", serde_json::to_string_pretty(&json)?);
            } else {
                eprintln!("⚠ Review skipped for {stage:?}: {reason:?}");
                if let Some(hint) = &suggestion {
                    eprintln!("  Suggestion: {hint}");
                }

                if args.explain {
                    println!();
                    println!("## Exit Code Explanation");
                    println!("Exit code: {exit_code}");
                    if args.strict_artifacts {
                        println!(
                            "Summary: --strict-artifacts enabled; missing artifacts treated as failure"
                        );
                    } else {
                        println!("Summary: No artifacts found; skipped (exit 0 in default mode)");
                    }
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
                    "schema_version": SCHEMA_VERSION,
                    "tool_version": tool_version(),
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
        Outcome::Stage(_) => {
            unreachable!("Review command should never return Stage outcome")
        }
        Outcome::Specify(_) => {
            unreachable!("Review command should never return Specify outcome")
        }
        Outcome::Run(_) => {
            unreachable!("Review command should never return Run outcome")
        }
        Outcome::Migrate(_) => {
            unreachable!("Review command should never return Migrate outcome")
        }
    }
}

/// Run the specify command
///
/// SPEC-KIT-921 P6-A: Specify command creates SPEC directory structure.
/// Creates docs/<SPEC-ID>/ directory with minimal PRD.md template.
/// Returns exit 0 on success, exit 1 on error.
fn run_specify(executor: SpeckitExecutor, args: SpecifyArgs) -> anyhow::Result<()> {
    // Default is dry-run, --execute enables actual creation
    let dry_run = !args.execute;

    let command = SpeckitCommand::Specify {
        spec_id: args.spec_id.clone(),
        dry_run,
    };

    match executor.execute(command) {
        Outcome::Specify(outcome) => {
            if args.json {
                let json = serde_json::json!({
                    "schema_version": SCHEMA_VERSION,
                    "tool_version": tool_version(),
                    "spec_id": outcome.spec_id,
                    "spec_dir": outcome.spec_dir,
                    "dry_run": outcome.dry_run,
                    "already_existed": outcome.already_existed,
                    "created_files": outcome.created_files,
                });
                println!("{}", serde_json::to_string_pretty(&json)?);
            } else if outcome.dry_run {
                if outcome.already_existed {
                    println!(
                        "[dry-run] SPEC directory {} already exists",
                        outcome.spec_dir
                    );
                } else {
                    println!("[dry-run] Would create SPEC directory {}", outcome.spec_dir);
                }
            } else if outcome.already_existed {
                println!(
                    "SPEC {} already exists at {}",
                    outcome.spec_id, outcome.spec_dir
                );
            } else {
                println!("Created SPEC {} at {}", outcome.spec_id, outcome.spec_dir);
                if !outcome.created_files.is_empty() {
                    println!("  Created files: {}", outcome.created_files.join(", "));
                }
            }
            Ok(())
        }
        Outcome::Error(msg) => {
            anyhow::bail!("Specify command failed: {msg}")
        }
        _ => {
            unreachable!("Specify command should return Specify or Error outcome")
        }
    }
}

/// Run the plan command
///
/// SPEC-KIT-921 P4-A: Plan command is locked to Stage::Plan.
/// Validates SPEC prerequisites and guardrails.
/// Returns exit 0 on success, exit 2 on validation failure.
fn run_plan(executor: SpeckitExecutor, args: PlanArgs) -> anyhow::Result<()> {
    // P4-A: Plan command always uses Stage::Plan
    let stage = Stage::Plan;

    let command = SpeckitCommand::ValidateStage {
        spec_id: args.spec_id.clone(),
        stage,
        dry_run: args.dry_run,
        strict_prereqs: args.strict_prereqs,
    };

    match executor.execute(command) {
        Outcome::Stage(outcome) => {
            let exit_code = outcome.exit_code();

            if args.json {
                let status = match outcome.resolution {
                    StageResolution::Ready => "ready",
                    StageResolution::Blocked => "blocked",
                    StageResolution::Skipped => "skipped",
                };
                let json = serde_json::json!({
                    "schema_version": SCHEMA_VERSION,
                    "tool_version": tool_version(),
                    "spec_id": outcome.spec_id,
                    "stage": format!("{:?}", outcome.stage),
                    "status": status,
                    "resolution": format!("{:?}", outcome.resolution),
                    "dry_run": outcome.dry_run,
                    "warnings": outcome.advisory_signals,
                    "errors": outcome.blocking_reasons,
                    "exit_code": exit_code,
                });
                println!("{}", serde_json::to_string_pretty(&json)?);
            } else {
                match outcome.resolution {
                    StageResolution::Ready => {
                        println!(
                            "✓ SPEC {} validated for stage {:?}",
                            outcome.spec_id, outcome.stage
                        );
                        if outcome.dry_run {
                            println!("  (dry-run mode: validation only, no agents spawned)");
                        }
                        for warning in &outcome.advisory_signals {
                            println!("  ⚠ {warning}");
                        }
                    }
                    StageResolution::Blocked => {
                        eprintln!(
                            "✗ SPEC {} blocked for stage {:?}",
                            outcome.spec_id, outcome.stage
                        );
                        for error in &outcome.blocking_reasons {
                            eprintln!("  ✗ {error}");
                        }
                    }
                    StageResolution::Skipped => {
                        println!(
                            "⊘ SPEC {} skipped for stage {:?}",
                            outcome.spec_id, outcome.stage
                        );
                        for signal in &outcome.advisory_signals {
                            println!("  {signal}");
                        }
                    }
                }
            }

            if exit_code != 0 {
                std::process::exit(exit_code);
            }
            Ok(())
        }
        Outcome::Error(err) => {
            if args.json {
                let json = serde_json::json!({
                    "schema_version": SCHEMA_VERSION,
                    "tool_version": tool_version(),
                    "error": err,
                    "exit_code": 3,
                });
                println!("{}", serde_json::to_string_pretty(&json)?);
            } else {
                eprintln!("Error: {err}");
            }
            std::process::exit(3);
        }
        Outcome::Status(_) | Outcome::Review(_) | Outcome::ReviewSkipped { .. } => {
            unreachable!("Plan command should never return Status/Review outcome")
        }
        Outcome::Specify(_) => {
            unreachable!("Plan command should never return Specify outcome")
        }
        Outcome::Run(_) => {
            unreachable!("Plan command should never return Run outcome")
        }
        Outcome::Migrate(_) => {
            unreachable!("Plan command should never return Migrate outcome")
        }
    }
}

/// Run the tasks command
///
/// SPEC-KIT-921 P4-C: Tasks command is locked to Stage::Tasks.
/// Validates SPEC prerequisites and guardrails for tasks stage.
/// Returns exit 0 on success, exit 2 on validation failure.
fn run_tasks(executor: SpeckitExecutor, args: TasksArgs) -> anyhow::Result<()> {
    // P4-C: Tasks command always uses Stage::Tasks
    let stage = Stage::Tasks;

    let command = SpeckitCommand::ValidateStage {
        spec_id: args.spec_id.clone(),
        stage,
        dry_run: args.dry_run,
        strict_prereqs: args.strict_prereqs,
    };

    match executor.execute(command) {
        Outcome::Stage(outcome) => {
            let exit_code = outcome.exit_code();

            if args.json {
                let status = match outcome.resolution {
                    StageResolution::Ready => "ready",
                    StageResolution::Blocked => "blocked",
                    StageResolution::Skipped => "skipped",
                };
                let json = serde_json::json!({
                    "schema_version": SCHEMA_VERSION,
                    "tool_version": tool_version(),
                    "spec_id": outcome.spec_id,
                    "stage": format!("{:?}", outcome.stage),
                    "status": status,
                    "resolution": format!("{:?}", outcome.resolution),
                    "dry_run": outcome.dry_run,
                    "warnings": outcome.advisory_signals,
                    "errors": outcome.blocking_reasons,
                    "exit_code": exit_code,
                });
                println!("{}", serde_json::to_string_pretty(&json)?);
            } else {
                match outcome.resolution {
                    StageResolution::Ready => {
                        println!(
                            "✓ SPEC {} validated for stage {:?}",
                            outcome.spec_id, outcome.stage
                        );
                        if outcome.dry_run {
                            println!("  (dry-run mode: validation only, no agents spawned)");
                        }
                        for warning in &outcome.advisory_signals {
                            println!("  ⚠ {warning}");
                        }
                    }
                    StageResolution::Blocked => {
                        eprintln!(
                            "✗ SPEC {} blocked for stage {:?}",
                            outcome.spec_id, outcome.stage
                        );
                        for error in &outcome.blocking_reasons {
                            eprintln!("  ✗ {error}");
                        }
                    }
                    StageResolution::Skipped => {
                        println!(
                            "⊘ SPEC {} skipped for stage {:?}",
                            outcome.spec_id, outcome.stage
                        );
                        for signal in &outcome.advisory_signals {
                            println!("  {signal}");
                        }
                    }
                }
            }

            if exit_code != 0 {
                std::process::exit(exit_code);
            }
            Ok(())
        }
        Outcome::Error(err) => {
            if args.json {
                let json = serde_json::json!({
                    "schema_version": SCHEMA_VERSION,
                    "tool_version": tool_version(),
                    "error": err,
                    "exit_code": 3,
                });
                println!("{}", serde_json::to_string_pretty(&json)?);
            } else {
                eprintln!("Error: {err}");
            }
            std::process::exit(3);
        }
        Outcome::Status(_) | Outcome::Review(_) | Outcome::ReviewSkipped { .. } => {
            unreachable!("Tasks command should never return Status/Review outcome")
        }
        Outcome::Specify(_) => {
            unreachable!("Tasks command should never return Specify outcome")
        }
        Outcome::Run(_) => {
            unreachable!("Tasks command should never return Run outcome")
        }
        Outcome::Migrate(_) => {
            unreachable!("Tasks command should never return Migrate outcome")
        }
    }
}

/// Run the implement command
///
/// SPEC-KIT-921 P5-A: Implement command is locked to Stage::Implement.
/// Validates SPEC prerequisites and guardrails for implement stage.
/// Returns exit 0 on success, exit 2 on validation failure.
fn run_implement(executor: SpeckitExecutor, args: ImplementArgs) -> anyhow::Result<()> {
    // P5-A: Implement command always uses Stage::Implement
    let stage = Stage::Implement;

    let command = SpeckitCommand::ValidateStage {
        spec_id: args.spec_id.clone(),
        stage,
        dry_run: args.dry_run,
        strict_prereqs: args.strict_prereqs,
    };

    match executor.execute(command) {
        Outcome::Stage(outcome) => {
            let exit_code = outcome.exit_code();

            if args.json {
                let status = match outcome.resolution {
                    StageResolution::Ready => "ready",
                    StageResolution::Blocked => "blocked",
                    StageResolution::Skipped => "skipped",
                };
                let json = serde_json::json!({
                    "schema_version": SCHEMA_VERSION,
                    "tool_version": tool_version(),
                    "spec_id": outcome.spec_id,
                    "stage": format!("{:?}", outcome.stage),
                    "status": status,
                    "resolution": format!("{:?}", outcome.resolution),
                    "dry_run": outcome.dry_run,
                    "warnings": outcome.advisory_signals,
                    "errors": outcome.blocking_reasons,
                    "exit_code": exit_code,
                });
                println!("{}", serde_json::to_string_pretty(&json)?);
            } else {
                match outcome.resolution {
                    StageResolution::Ready => {
                        println!(
                            "✓ SPEC {} validated for stage {:?}",
                            outcome.spec_id, outcome.stage
                        );
                        if outcome.dry_run {
                            println!("  (dry-run mode: validation only, no agents spawned)");
                        }
                        for warning in &outcome.advisory_signals {
                            println!("  ⚠ {warning}");
                        }
                    }
                    StageResolution::Blocked => {
                        eprintln!(
                            "✗ SPEC {} blocked for stage {:?}",
                            outcome.spec_id, outcome.stage
                        );
                        for error in &outcome.blocking_reasons {
                            eprintln!("  ✗ {error}");
                        }
                    }
                    StageResolution::Skipped => {
                        println!(
                            "⊘ SPEC {} skipped for stage {:?}",
                            outcome.spec_id, outcome.stage
                        );
                        for signal in &outcome.advisory_signals {
                            println!("  {signal}");
                        }
                    }
                }
            }

            if exit_code != 0 {
                std::process::exit(exit_code);
            }
            Ok(())
        }
        Outcome::Error(err) => {
            if args.json {
                let json = serde_json::json!({
                    "schema_version": SCHEMA_VERSION,
                    "tool_version": tool_version(),
                    "error": err,
                    "exit_code": 3,
                });
                println!("{}", serde_json::to_string_pretty(&json)?);
            } else {
                eprintln!("Error: {err}");
            }
            std::process::exit(3);
        }
        Outcome::Status(_) | Outcome::Review(_) | Outcome::ReviewSkipped { .. } => {
            unreachable!("Implement command should never return Status/Review outcome")
        }
        Outcome::Specify(_) => {
            unreachable!("Implement command should never return Specify outcome")
        }
        Outcome::Run(_) => {
            unreachable!("Implement command should never return Run outcome")
        }
        Outcome::Migrate(_) => {
            unreachable!("Implement command should never return Migrate outcome")
        }
    }
}

/// Run the validate command
///
/// SPEC-KIT-921 P5-B: Validate command is locked to Stage::Validate.
/// Validates SPEC prerequisites and guardrails for validate stage.
/// Returns exit 0 on success, exit 2 on validation failure.
fn run_validate(executor: SpeckitExecutor, args: ValidateStageArgs) -> anyhow::Result<()> {
    // P5-B: Validate command always uses Stage::Validate
    let stage = Stage::Validate;

    let command = SpeckitCommand::ValidateStage {
        spec_id: args.spec_id.clone(),
        stage,
        dry_run: args.dry_run,
        strict_prereqs: args.strict_prereqs,
    };

    match executor.execute(command) {
        Outcome::Stage(outcome) => {
            let exit_code = outcome.exit_code();

            if args.json {
                let status = match outcome.resolution {
                    StageResolution::Ready => "ready",
                    StageResolution::Blocked => "blocked",
                    StageResolution::Skipped => "skipped",
                };
                let json = serde_json::json!({
                    "schema_version": SCHEMA_VERSION,
                    "tool_version": tool_version(),
                    "spec_id": outcome.spec_id,
                    "stage": format!("{:?}", outcome.stage),
                    "status": status,
                    "resolution": format!("{:?}", outcome.resolution),
                    "dry_run": outcome.dry_run,
                    "warnings": outcome.advisory_signals,
                    "errors": outcome.blocking_reasons,
                    "exit_code": exit_code,
                });
                println!("{}", serde_json::to_string_pretty(&json)?);
            } else {
                match outcome.resolution {
                    StageResolution::Ready => {
                        println!(
                            "✓ SPEC {} validated for stage {:?}",
                            outcome.spec_id, outcome.stage
                        );
                        if outcome.dry_run {
                            println!("  (dry-run mode: validation only, no agents spawned)");
                        }
                        for warning in &outcome.advisory_signals {
                            println!("  ⚠ {warning}");
                        }
                    }
                    StageResolution::Blocked => {
                        eprintln!(
                            "✗ SPEC {} blocked for stage {:?}",
                            outcome.spec_id, outcome.stage
                        );
                        for error in &outcome.blocking_reasons {
                            eprintln!("  ✗ {error}");
                        }
                    }
                    StageResolution::Skipped => {
                        println!(
                            "⊘ SPEC {} skipped for stage {:?}",
                            outcome.spec_id, outcome.stage
                        );
                        for signal in &outcome.advisory_signals {
                            println!("  {signal}");
                        }
                    }
                }
            }

            if exit_code != 0 {
                std::process::exit(exit_code);
            }
            Ok(())
        }
        Outcome::Error(err) => {
            if args.json {
                let json = serde_json::json!({
                    "schema_version": SCHEMA_VERSION,
                    "tool_version": tool_version(),
                    "error": err,
                    "exit_code": 3,
                });
                println!("{}", serde_json::to_string_pretty(&json)?);
            } else {
                eprintln!("Error: {err}");
            }
            std::process::exit(3);
        }
        Outcome::Status(_) | Outcome::Review(_) | Outcome::ReviewSkipped { .. } => {
            unreachable!("Validate command should never return Status/Review outcome")
        }
        Outcome::Specify(_) => {
            unreachable!("Validate command should never return Specify outcome")
        }
        Outcome::Run(_) => {
            unreachable!("Validate command should never return Run outcome")
        }
        Outcome::Migrate(_) => {
            unreachable!("Validate command should never return Migrate outcome")
        }
    }
}

/// Run the audit command
///
/// SPEC-KIT-921 P5-B: Audit command is locked to Stage::Audit.
/// Validates SPEC prerequisites and guardrails for audit stage.
/// Returns exit 0 on success, exit 2 on validation failure.
fn run_audit(executor: SpeckitExecutor, args: AuditArgs) -> anyhow::Result<()> {
    // P5-B: Audit command always uses Stage::Audit
    let stage = Stage::Audit;

    let command = SpeckitCommand::ValidateStage {
        spec_id: args.spec_id.clone(),
        stage,
        dry_run: args.dry_run,
        strict_prereqs: args.strict_prereqs,
    };

    match executor.execute(command) {
        Outcome::Stage(outcome) => {
            let exit_code = outcome.exit_code();

            if args.json {
                let status = match outcome.resolution {
                    StageResolution::Ready => "ready",
                    StageResolution::Blocked => "blocked",
                    StageResolution::Skipped => "skipped",
                };
                let json = serde_json::json!({
                    "schema_version": SCHEMA_VERSION,
                    "tool_version": tool_version(),
                    "spec_id": outcome.spec_id,
                    "stage": format!("{:?}", outcome.stage),
                    "status": status,
                    "resolution": format!("{:?}", outcome.resolution),
                    "dry_run": outcome.dry_run,
                    "warnings": outcome.advisory_signals,
                    "errors": outcome.blocking_reasons,
                    "exit_code": exit_code,
                });
                println!("{}", serde_json::to_string_pretty(&json)?);
            } else {
                match outcome.resolution {
                    StageResolution::Ready => {
                        println!(
                            "✓ SPEC {} validated for stage {:?}",
                            outcome.spec_id, outcome.stage
                        );
                        if outcome.dry_run {
                            println!("  (dry-run mode: validation only, no agents spawned)");
                        }
                        for warning in &outcome.advisory_signals {
                            println!("  ⚠ {warning}");
                        }
                    }
                    StageResolution::Blocked => {
                        eprintln!(
                            "✗ SPEC {} blocked for stage {:?}",
                            outcome.spec_id, outcome.stage
                        );
                        for error in &outcome.blocking_reasons {
                            eprintln!("  ✗ {error}");
                        }
                    }
                    StageResolution::Skipped => {
                        println!(
                            "⊘ SPEC {} skipped for stage {:?}",
                            outcome.spec_id, outcome.stage
                        );
                        for signal in &outcome.advisory_signals {
                            println!("  {signal}");
                        }
                    }
                }
            }

            if exit_code != 0 {
                std::process::exit(exit_code);
            }
            Ok(())
        }
        Outcome::Error(err) => {
            if args.json {
                let json = serde_json::json!({
                    "schema_version": SCHEMA_VERSION,
                    "tool_version": tool_version(),
                    "error": err,
                    "exit_code": 3,
                });
                println!("{}", serde_json::to_string_pretty(&json)?);
            } else {
                eprintln!("Error: {err}");
            }
            std::process::exit(3);
        }
        Outcome::Status(_) | Outcome::Review(_) | Outcome::ReviewSkipped { .. } => {
            unreachable!("Audit command should never return Status/Review outcome")
        }
        Outcome::Specify(_) => {
            unreachable!("Audit command should never return Specify outcome")
        }
        Outcome::Run(_) => {
            unreachable!("Audit command should never return Run outcome")
        }
        Outcome::Migrate(_) => {
            unreachable!("Audit command should never return Migrate outcome")
        }
    }
}

/// Run the unlock command
///
/// SPEC-KIT-921 P5-B: Unlock command is locked to Stage::Unlock.
/// Validates SPEC prerequisites and guardrails for unlock stage.
/// Returns exit 0 on success, exit 2 on validation failure.
fn run_unlock(executor: SpeckitExecutor, args: UnlockArgs) -> anyhow::Result<()> {
    // P5-B: Unlock command always uses Stage::Unlock
    let stage = Stage::Unlock;

    let command = SpeckitCommand::ValidateStage {
        spec_id: args.spec_id.clone(),
        stage,
        dry_run: args.dry_run,
        strict_prereqs: args.strict_prereqs,
    };

    match executor.execute(command) {
        Outcome::Stage(outcome) => {
            let exit_code = outcome.exit_code();

            if args.json {
                let status = match outcome.resolution {
                    StageResolution::Ready => "ready",
                    StageResolution::Blocked => "blocked",
                    StageResolution::Skipped => "skipped",
                };
                let json = serde_json::json!({
                    "schema_version": SCHEMA_VERSION,
                    "tool_version": tool_version(),
                    "spec_id": outcome.spec_id,
                    "stage": format!("{:?}", outcome.stage),
                    "status": status,
                    "resolution": format!("{:?}", outcome.resolution),
                    "dry_run": outcome.dry_run,
                    "warnings": outcome.advisory_signals,
                    "errors": outcome.blocking_reasons,
                    "exit_code": exit_code,
                });
                println!("{}", serde_json::to_string_pretty(&json)?);
            } else {
                match outcome.resolution {
                    StageResolution::Ready => {
                        println!(
                            "✓ SPEC {} validated for stage {:?}",
                            outcome.spec_id, outcome.stage
                        );
                        if outcome.dry_run {
                            println!("  (dry-run mode: validation only, no agents spawned)");
                        }
                        for warning in &outcome.advisory_signals {
                            println!("  ⚠ {warning}");
                        }
                    }
                    StageResolution::Blocked => {
                        eprintln!(
                            "✗ SPEC {} blocked for stage {:?}",
                            outcome.spec_id, outcome.stage
                        );
                        for error in &outcome.blocking_reasons {
                            eprintln!("  ✗ {error}");
                        }
                    }
                    StageResolution::Skipped => {
                        println!(
                            "⊘ SPEC {} skipped for stage {:?}",
                            outcome.spec_id, outcome.stage
                        );
                        for signal in &outcome.advisory_signals {
                            println!("  {signal}");
                        }
                    }
                }
            }

            if exit_code != 0 {
                std::process::exit(exit_code);
            }
            Ok(())
        }
        Outcome::Error(err) => {
            if args.json {
                let json = serde_json::json!({
                    "schema_version": SCHEMA_VERSION,
                    "tool_version": tool_version(),
                    "error": err,
                    "exit_code": 3,
                });
                println!("{}", serde_json::to_string_pretty(&json)?);
            } else {
                eprintln!("Error: {err}");
            }
            std::process::exit(3);
        }
        Outcome::Status(_) | Outcome::Review(_) | Outcome::ReviewSkipped { .. } => {
            unreachable!("Unlock command should never return Status/Review outcome")
        }
        Outcome::Specify(_) => {
            unreachable!("Unlock command should never return Specify outcome")
        }
        Outcome::Run(_) => {
            unreachable!("Unlock command should never return Run outcome")
        }
        Outcome::Migrate(_) => {
            unreachable!("Unlock command should never return Migrate outcome")
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

/// Explanation of exit code decision
struct ExitCodeExplanation {
    summary: String,
    reasons: Vec<String>,
    flags_active: Vec<&'static str>,
}

/// Generate human-readable explanation for review exit code
fn explain_review_exit_code(
    exit_code: i32,
    blocking_signals: &[ReviewSignal],
    advisory_signals: &[ReviewSignal],
    options: &ReviewOptions,
) -> ExitCodeExplanation {
    let mut flags_active = Vec::new();
    if options.strict_artifacts {
        flags_active.push("--strict-artifacts");
    }
    if options.strict_warnings {
        flags_active.push("--strict-warnings");
    }
    if options.strict_schema {
        flags_active.push("--strict-schema");
    }

    let mut reasons = Vec::new();

    match exit_code {
        0 => ExitCodeExplanation {
            summary: "Review passed with no blocking signals".to_string(),
            reasons: if blocking_signals.is_empty() && advisory_signals.is_empty() {
                vec!["No conflicts detected in consensus evidence".to_string()]
            } else if blocking_signals.is_empty() {
                vec![format!(
                    "{} advisory signal(s) detected (not blocking without --strict-warnings)",
                    advisory_signals.len()
                )]
            } else {
                vec![]
            },
            flags_active,
        },
        1 => {
            for signal in advisory_signals {
                reasons.push(format!("[Advisory] {}", signal.message));
            }
            ExitCodeExplanation {
                summary: "Review passed with warnings (exit 1 due to --strict-warnings)"
                    .to_string(),
                reasons,
                flags_active,
            }
        }
        2 => {
            for signal in blocking_signals {
                reasons.push(format!(
                    "[{:?}] {} (from {})",
                    signal.kind,
                    signal.message,
                    signal.origin.display_name()
                ));
            }
            if reasons.is_empty() {
                reasons.push("Missing required artifacts with --strict-artifacts".to_string());
            }
            ExitCodeExplanation {
                summary: "Review failed - blocking signals or escalation required".to_string(),
                reasons,
                flags_active,
            }
        }
        3 => ExitCodeExplanation {
            summary: "Infrastructure error - parse/schema errors with --strict-schema".to_string(),
            reasons: advisory_signals
                .iter()
                .filter(|s| s.message.contains("parse") || s.message.contains("Parse"))
                .map(|s| format!("[ParseError] {}", s.message))
                .collect(),
            flags_active,
        },
        _ => ExitCodeExplanation {
            summary: format!("Unknown exit code {exit_code}"),
            reasons: vec![],
            flags_active,
        },
    }
}

/// Run the run command (batch stage validation)
///
/// SPEC-KIT-921 P7-A: Validate stages from --from to --to.
/// Returns exit 0 if all stages ready, exit 2 if any blocked, exit 3 for infrastructure errors.
fn run_run(executor: SpeckitExecutor, args: RunArgs) -> anyhow::Result<()> {
    // Parse stage names
    let from_stage = parse_stage(&args.from_stage)?;
    let to_stage = parse_stage(&args.to_stage)?;

    let command = SpeckitCommand::Run {
        spec_id: args.spec_id.clone(),
        from_stage,
        to_stage,
    };

    match executor.execute(command) {
        Outcome::Run(outcome) => {
            if args.json {
                // JSON output per HANDOFF.md schema
                let stages: Vec<_> = outcome
                    .stages
                    .iter()
                    .map(|s| {
                        serde_json::json!({
                            "stage": s.stage.as_str(),
                            "status": s.status,
                            "warnings": s.warnings,
                            "errors": s.errors,
                        })
                    })
                    .collect();

                let mut json = serde_json::json!({
                    "schema_version": SCHEMA_VERSION,
                    "tool_version": tool_version(),
                    "spec_id": outcome.spec_id,
                    "from_stage": outcome.from_stage.as_str(),
                    "to_stage": outcome.to_stage.as_str(),
                    "overall_status": outcome.overall_status.as_str(),
                    "stages": stages,
                    "exit_code": outcome.exit_code,
                });

                // Add legacy detection info if present (blocked until migrated)
                if outcome.legacy_detected
                    && let Some(obj) = json.as_object_mut()
                {
                    obj.insert(
                        "packet_source".to_string(),
                        serde_json::json!("spec_md_legacy"),
                    );
                    if let Some(ref warning) = outcome.legacy_warning {
                        obj.insert("legacy_warning".to_string(), serde_json::json!(warning));
                    }
                }

                println!("{}", serde_json::to_string_pretty(&json)?);
            } else {
                // Text output
                match outcome.overall_status {
                    RunOverallStatus::Ready => {
                        println!(
                            "✓ SPEC {} ready for stages {} to {}",
                            outcome.spec_id,
                            outcome.from_stage.display_name(),
                            outcome.to_stage.display_name()
                        );
                    }
                    RunOverallStatus::Blocked => {
                        eprintln!(
                            "✗ SPEC {} blocked for stages {} to {}",
                            outcome.spec_id,
                            outcome.from_stage.display_name(),
                            outcome.to_stage.display_name()
                        );
                    }
                    RunOverallStatus::Partial => {
                        println!(
                            "⚠ SPEC {} partially ready for stages {} to {}",
                            outcome.spec_id,
                            outcome.from_stage.display_name(),
                            outcome.to_stage.display_name()
                        );
                    }
                }

                // Print per-stage details
                for stage_outcome in &outcome.stages {
                    let icon = if stage_outcome.status == "ready" {
                        "✓"
                    } else {
                        "✗"
                    };
                    println!(
                        "  {} {}: {}",
                        icon,
                        stage_outcome.stage.display_name(),
                        stage_outcome.status
                    );

                    for warning in &stage_outcome.warnings {
                        println!("    ⚠ {warning}");
                    }
                    for error in &stage_outcome.errors {
                        println!("    ✗ {error}");
                    }
                }

                // Print legacy warning
                if let Some(ref warning) = outcome.legacy_warning {
                    eprintln!("\n⚠ {warning}");
                }
            }

            std::process::exit(outcome.exit_code);
        }
        Outcome::Error(err) => {
            if args.json {
                let json = serde_json::json!({
                    "schema_version": SCHEMA_VERSION,
                    "tool_version": tool_version(),
                    "error": err,
                    "exit_code": 3,
                });
                println!("{}", serde_json::to_string_pretty(&json)?);
            } else {
                eprintln!("Error: {err}");
            }
            std::process::exit(3);
        }
        _ => {
            unreachable!("Run command should return Run or Error outcome")
        }
    }
}

/// Run the migrate command
///
/// SPEC-KIT-921 P7-B: Migrate legacy spec.md to PRD.md
fn run_migrate(executor: SpeckitExecutor, args: MigrateArgs) -> anyhow::Result<()> {
    let command = SpeckitCommand::Migrate {
        spec_id: args.spec_id.clone(),
        dry_run: args.dry_run,
    };

    match executor.execute(command) {
        Outcome::Migrate(outcome) => {
            if args.json {
                let json = serde_json::json!({
                    "schema_version": SCHEMA_VERSION,
                    "tool_version": tool_version(),
                    "spec_id": outcome.spec_id,
                    "dry_run": outcome.dry_run,
                    "status": outcome.status.as_str(),
                    "spec_dir": outcome.spec_dir,
                    "source_file": outcome.source_file,
                    "dest_file": outcome.dest_file,
                    "exit_code": outcome.exit_code,
                    "warnings": outcome.warnings,
                });
                println!("{}", serde_json::to_string_pretty(&json)?);
            } else {
                // Text output
                match outcome.status {
                    MigrateStatus::Migrated => {
                        println!("✓ Migrated spec.md → PRD.md for {}", outcome.spec_id);
                        println!("  Source: {}/spec.md", outcome.spec_dir);
                        println!("  Created: {}/PRD.md", outcome.spec_dir);
                    }
                    MigrateStatus::WouldMigrate => {
                        println!("Would migrate spec.md → PRD.md for {}", outcome.spec_id);
                        println!("  Source: {}/spec.md", outcome.spec_dir);
                        println!("  Would create: {}/PRD.md", outcome.spec_dir);
                    }
                    MigrateStatus::AlreadyMigrated => {
                        println!(
                            "✓ {} already has PRD.md, no migration needed",
                            outcome.spec_id
                        );
                    }
                    MigrateStatus::NoSourceFile => {
                        for warning in &outcome.warnings {
                            println!("⚠ {warning}");
                        }
                    }
                }
            }

            std::process::exit(outcome.exit_code);
        }
        Outcome::Error(err) => {
            if args.json {
                let json = serde_json::json!({
                    "schema_version": SCHEMA_VERSION,
                    "tool_version": tool_version(),
                    "spec_id": args.spec_id,
                    "error": err,
                    "exit_code": 1,
                });
                println!("{}", serde_json::to_string_pretty(&json)?);
            } else {
                eprintln!("Error: {err}");
            }
            std::process::exit(1);
        }
        _ => {
            unreachable!("Migrate command should return Migrate or Error outcome")
        }
    }
}

// =============================================================================
// Capsule CLI Handlers (SPEC-KIT-971)
// =============================================================================

/// Default capsule path relative to repo root
const DEFAULT_CAPSULE_PATH: &str = ".speckit/memvid/workspace.mv2";

/// Exit codes for capsule commands (per SPEC-KIT-971)
mod capsule_exit {
    pub const SUCCESS: i32 = 0;
    pub const USER_ERROR: i32 = 1;   // Bad args, invalid URI
    pub const SYSTEM_ERROR: i32 = 2; // Corrupt capsule, locked, IO error
}

/// Run the capsule command
fn run_capsule(cwd: PathBuf, args: CapsuleArgs) -> anyhow::Result<()> {
    let capsule_path = args
        .capsule_path
        .unwrap_or_else(|| cwd.join(DEFAULT_CAPSULE_PATH));

    match args.command {
        CapsuleSubcommand::Init(cmd_args) => run_capsule_init(&capsule_path, cmd_args),
        CapsuleSubcommand::Doctor(cmd_args) => run_capsule_doctor(&capsule_path, cmd_args),
        CapsuleSubcommand::Stats(cmd_args) => run_capsule_stats(&capsule_path, cmd_args),
        CapsuleSubcommand::Checkpoints(cmd_args) => run_capsule_checkpoints(&capsule_path, cmd_args),
        CapsuleSubcommand::Events(cmd_args) => run_capsule_events(&capsule_path, cmd_args),
        CapsuleSubcommand::Commit(cmd_args) => run_capsule_commit(&capsule_path, cmd_args),
        CapsuleSubcommand::ResolveUri(cmd_args) => run_capsule_resolve_uri(&capsule_path, cmd_args),
        CapsuleSubcommand::Export(cmd_args) => run_capsule_export(&capsule_path, cmd_args),
    }
}

/// Run `capsule doctor` command
fn run_capsule_doctor(capsule_path: &PathBuf, args: CapsuleDoctorArgs) -> anyhow::Result<()> {
    let diagnostics = CapsuleHandle::doctor(capsule_path);

    // Determine overall status
    let has_errors = diagnostics.iter().any(|d| matches!(d, DiagnosticResult::Error(_, _)));
    let has_warnings = diagnostics.iter().any(|d| matches!(d, DiagnosticResult::Warning(_, _)));
    let status = if has_errors {
        "error"
    } else if has_warnings {
        "warning"
    } else {
        "ok"
    };

    if args.json {
        let diag_json: Vec<_> = diagnostics
            .iter()
            .map(|d| match d {
                DiagnosticResult::Ok(msg) => serde_json::json!({
                    "level": "ok",
                    "message": msg,
                }),
                DiagnosticResult::Warning(msg, hint) => serde_json::json!({
                    "level": "warning",
                    "message": msg,
                    "hint": hint,
                }),
                DiagnosticResult::Error(msg, hint) => serde_json::json!({
                    "level": "error",
                    "message": msg,
                    "hint": hint,
                }),
            })
            .collect();

        let json = serde_json::json!({
            "schema_version": SCHEMA_VERSION,
            "tool_version": tool_version(),
            "status": status,
            "capsule_path": capsule_path.display().to_string(),
            "diagnostics": diag_json,
        });
        println!("{}", serde_json::to_string_pretty(&json)?);
    } else {
        println!("Capsule Doctor: {}", capsule_path.display());
        println!("Status: {}", status.to_uppercase());
        println!();
        for diag in &diagnostics {
            match diag {
                DiagnosticResult::Ok(msg) => println!("  ✓ {msg}"),
                DiagnosticResult::Warning(msg, hint) => {
                    println!("  ⚠ {msg}");
                    println!("    → {hint}");
                }
                DiagnosticResult::Error(msg, hint) => {
                    println!("  ✗ {msg}");
                    println!("    → {hint}");
                }
            }
        }
    }

    if has_errors {
        std::process::exit(capsule_exit::SYSTEM_ERROR);
    }
    Ok(())
}

/// Run `capsule stats` command
fn run_capsule_stats(capsule_path: &PathBuf, args: CapsuleStatsArgs) -> anyhow::Result<()> {
    // Try to open capsule read-only for stats
    let config = CapsuleConfig {
        capsule_path: capsule_path.clone(),
        workspace_id: "cli".to_string(),
        ..Default::default()
    };

    let handle = match CapsuleHandle::open_read_only(config) {
        Ok(h) => h,
        Err(e) => {
            if args.json {
                let json = serde_json::json!({
                    "schema_version": SCHEMA_VERSION,
                    "tool_version": tool_version(),
                    "error": format!("{e}"),
                    "capsule_path": capsule_path.display().to_string(),
                });
                println!("{}", serde_json::to_string_pretty(&json)?);
            } else {
                eprintln!("Error: Failed to open capsule: {e}");
            }
            std::process::exit(capsule_exit::SYSTEM_ERROR);
        }
    };

    let stats = handle.stats();

    let index_status_str = format!("{:?}", stats.index_status);

    if args.json {
        let json = serde_json::json!({
            "schema_version": SCHEMA_VERSION,
            "tool_version": tool_version(),
            "capsule_path": capsule_path.display().to_string(),
            "size_bytes": stats.size_bytes,
            "frame_count": stats.frame_count,
            "uri_count": stats.uri_count,
            "checkpoint_count": stats.checkpoint_count,
            "event_count": stats.event_count,
            "dedup_ratio": stats.dedup_ratio,
            "index_status": index_status_str,
        });
        println!("{}", serde_json::to_string_pretty(&json)?);
    } else {
        println!("Capsule Stats: {}", capsule_path.display());
        println!();
        println!("  Size:        {} bytes", stats.size_bytes);
        println!("  Frames:      {}", stats.frame_count);
        println!("  URIs:        {}", stats.uri_count);
        println!("  Checkpoints: {}", stats.checkpoint_count);
        println!("  Events:      {}", stats.event_count);
        println!("  Dedup ratio: {:.2}", stats.dedup_ratio);
        println!("  Index:       {}", index_status_str);
    }

    Ok(())
}

/// Run `capsule checkpoints` command
fn run_capsule_checkpoints(capsule_path: &PathBuf, args: CapsuleCheckpointsArgs) -> anyhow::Result<()> {
    let config = CapsuleConfig {
        capsule_path: capsule_path.clone(),
        workspace_id: "cli".to_string(),
        ..Default::default()
    };

    let handle = match CapsuleHandle::open_read_only(config) {
        Ok(h) => h,
        Err(e) => {
            if args.json {
                let json = serde_json::json!({
                    "schema_version": SCHEMA_VERSION,
                    "tool_version": tool_version(),
                    "error": format!("{e}"),
                    "capsule_path": capsule_path.display().to_string(),
                });
                println!("{}", serde_json::to_string_pretty(&json)?);
            } else {
                eprintln!("Error: Failed to open capsule: {e}");
            }
            std::process::exit(capsule_exit::SYSTEM_ERROR);
        }
    };

    let checkpoints = handle.list_checkpoints();

    if args.json {
        let cp_json: Vec<_> = checkpoints
            .iter()
            .map(|cp| {
                serde_json::json!({
                    "checkpoint_id": cp.checkpoint_id.as_str(),
                    "label": cp.label,
                    "stage": cp.stage,
                    "spec_id": cp.spec_id,
                    "run_id": cp.run_id,
                    "commit_hash": cp.commit_hash,
                    "timestamp": cp.timestamp.to_rfc3339(),
                    "is_manual": cp.is_manual,
                })
            })
            .collect();

        let json = serde_json::json!({
            "schema_version": SCHEMA_VERSION,
            "tool_version": tool_version(),
            "capsule_path": capsule_path.display().to_string(),
            "checkpoints": cp_json,
            "count": checkpoints.len(),
        });
        println!("{}", serde_json::to_string_pretty(&json)?);
    } else {
        println!("Capsule Checkpoints: {}", capsule_path.display());
        println!();
        if checkpoints.is_empty() {
            println!("  (no checkpoints)");
        } else {
            for cp in &checkpoints {
                let label = cp.label.as_deref().unwrap_or("-");
                let stage = cp.stage.as_deref().unwrap_or("-");
                println!(
                    "  {} | {} | {} | {}",
                    cp.checkpoint_id.as_str(),
                    label,
                    stage,
                    cp.timestamp.format("%Y-%m-%d %H:%M:%S")
                );
            }
        }
        println!();
        println!("Total: {} checkpoint(s)", checkpoints.len());
    }

    Ok(())
}

/// Run `capsule commit` command
fn run_capsule_commit(capsule_path: &PathBuf, args: CapsuleCommitArgs) -> anyhow::Result<()> {
    // Validate label
    if args.label.is_empty() {
        if args.json {
            let json = serde_json::json!({
                "schema_version": SCHEMA_VERSION,
                "tool_version": tool_version(),
                "error": "Label cannot be empty",
                "capsule_path": capsule_path.display().to_string(),
            });
            println!("{}", serde_json::to_string_pretty(&json)?);
        } else {
            eprintln!("Error: Label cannot be empty");
        }
        std::process::exit(capsule_exit::USER_ERROR);
    }

    let config = CapsuleConfig {
        capsule_path: capsule_path.clone(),
        workspace_id: "cli".to_string(),
        ..Default::default()
    };

    // Open with write lock for commit
    let handle = match CapsuleHandle::open(config) {
        Ok(h) => h,
        Err(e) => {
            if args.json {
                let json = serde_json::json!({
                    "schema_version": SCHEMA_VERSION,
                    "tool_version": tool_version(),
                    "error": format!("{e}"),
                    "capsule_path": capsule_path.display().to_string(),
                });
                println!("{}", serde_json::to_string_pretty(&json)?);
            } else {
                eprintln!("Error: Failed to open capsule: {e}");
            }
            std::process::exit(capsule_exit::SYSTEM_ERROR);
        }
    };

    let checkpoint_id = match handle.commit_manual(&args.label) {
        Ok(id) => id,
        Err(e) => {
            if args.json {
                let json = serde_json::json!({
                    "schema_version": SCHEMA_VERSION,
                    "tool_version": tool_version(),
                    "error": format!("{e}"),
                    "capsule_path": capsule_path.display().to_string(),
                });
                println!("{}", serde_json::to_string_pretty(&json)?);
            } else {
                eprintln!("Error: Failed to create checkpoint: {e}");
            }
            std::process::exit(capsule_exit::SYSTEM_ERROR);
        }
    };

    if args.json {
        let json = serde_json::json!({
            "schema_version": SCHEMA_VERSION,
            "tool_version": tool_version(),
            "capsule_path": capsule_path.display().to_string(),
            "checkpoint_id": checkpoint_id.as_str(),
            "label": args.label,
            "created": true,
        });
        println!("{}", serde_json::to_string_pretty(&json)?);
    } else {
        println!("Created checkpoint: {}", checkpoint_id.as_str());
        println!("Label: {}", args.label);
    }

    Ok(())
}

/// Run `capsule resolve-uri` command
fn run_capsule_resolve_uri(capsule_path: &PathBuf, args: CapsuleResolveUriArgs) -> anyhow::Result<()> {
    // Validate URI format
    if !args.uri.starts_with("mv2://") {
        if args.json {
            let json = serde_json::json!({
                "schema_version": SCHEMA_VERSION,
                "tool_version": tool_version(),
                "error": "URI must start with mv2://",
                "uri": args.uri,
                "capsule_path": capsule_path.display().to_string(),
            });
            println!("{}", serde_json::to_string_pretty(&json)?);
        } else {
            eprintln!("Error: URI must start with mv2://");
        }
        std::process::exit(capsule_exit::USER_ERROR);
    }

    let config = CapsuleConfig {
        capsule_path: capsule_path.clone(),
        workspace_id: "cli".to_string(),
        ..Default::default()
    };

    let handle = match CapsuleHandle::open_read_only(config) {
        Ok(h) => h,
        Err(e) => {
            if args.json {
                let json = serde_json::json!({
                    "schema_version": SCHEMA_VERSION,
                    "tool_version": tool_version(),
                    "error": format!("{e}"),
                    "uri": args.uri,
                    "capsule_path": capsule_path.display().to_string(),
                });
                println!("{}", serde_json::to_string_pretty(&json)?);
            } else {
                eprintln!("Error: Failed to open capsule: {e}");
            }
            std::process::exit(capsule_exit::SYSTEM_ERROR);
        }
    };

    // Resolve as_of checkpoint if provided
    let as_of = args.as_of.as_ref().map(|s| CheckpointId::new(s.clone()));

    // Get the bytes
    let bytes = match handle.get_bytes_str(&args.uri, None, as_of.as_ref()) {
        Ok(b) => b,
        Err(e) => {
            if args.json {
                let json = serde_json::json!({
                    "schema_version": SCHEMA_VERSION,
                    "tool_version": tool_version(),
                    "error": format!("{e}"),
                    "uri": args.uri,
                    "as_of": args.as_of,
                    "capsule_path": capsule_path.display().to_string(),
                });
                println!("{}", serde_json::to_string_pretty(&json)?);
            } else {
                eprintln!("Error: Failed to resolve URI: {e}");
            }
            std::process::exit(capsule_exit::USER_ERROR);
        }
    };

    if args.json {
        // In JSON mode, return metadata about the resolution
        let json = serde_json::json!({
            "schema_version": SCHEMA_VERSION,
            "tool_version": tool_version(),
            "capsule_path": capsule_path.display().to_string(),
            "uri": args.uri,
            "as_of": args.as_of,
            "size_bytes": bytes.len(),
            "content_preview": String::from_utf8_lossy(&bytes[..bytes.len().min(200)]),
            "out_path": args.out.as_ref().map(|p| p.display().to_string()),
        });
        println!("{}", serde_json::to_string_pretty(&json)?);

        // Still write to file if --out was specified
        if let Some(out_path) = &args.out {
            std::fs::write(out_path, &bytes)?;
        }
    } else if let Some(out_path) = &args.out {
        // Write to file
        std::fs::write(out_path, &bytes)?;
        println!("Wrote {} bytes to {}", bytes.len(), out_path.display());
    } else {
        // Write to stdout (binary)
        std::io::stdout().write_all(&bytes)?;
    }

    Ok(())
}

/// Run `capsule init` command
///
/// SPEC-KIT-971: Initialize a new workspace capsule.
fn run_capsule_init(capsule_path: &PathBuf, args: CapsuleInitArgs) -> anyhow::Result<()> {
    // Check if capsule already exists
    if capsule_path.exists() {
        if args.force {
            // Backup existing capsule
            let backup_path = capsule_path.with_extension("mv2.bak");
            if let Err(e) = std::fs::rename(capsule_path, &backup_path) {
                if args.json {
                    let json = serde_json::json!({
                        "schema_version": SCHEMA_VERSION,
                        "tool_version": tool_version(),
                        "error": format!("Failed to backup existing capsule: {e}"),
                        "capsule_path": capsule_path.display().to_string(),
                    });
                    println!("{}", serde_json::to_string_pretty(&json)?);
                } else {
                    eprintln!("Error: Failed to backup existing capsule: {e}");
                }
                std::process::exit(capsule_exit::SYSTEM_ERROR);
            }

            if !args.json {
                println!("Backed up existing capsule to {}", backup_path.display());
            }
        } else {
            if args.json {
                let json = serde_json::json!({
                    "schema_version": SCHEMA_VERSION,
                    "tool_version": tool_version(),
                    "exists": true,
                    "created": false,
                    "capsule_path": capsule_path.display().to_string(),
                    "message": "Capsule already exists. Use --force to replace.",
                });
                println!("{}", serde_json::to_string_pretty(&json)?);
            } else {
                println!("Capsule already exists at {}", capsule_path.display());
                println!("Use --force to backup and replace.");
            }
            return Ok(());
        }
    }

    // Create the capsule
    let config = CapsuleConfig {
        capsule_path: capsule_path.clone(),
        workspace_id: "default".to_string(),
        ..Default::default()
    };

    match CapsuleHandle::open(config) {
        Ok(handle) => {
            // Immediately drop to release lock
            drop(handle);

            if args.json {
                let json = serde_json::json!({
                    "schema_version": SCHEMA_VERSION,
                    "tool_version": tool_version(),
                    "created": true,
                    "capsule_path": capsule_path.display().to_string(),
                });
                println!("{}", serde_json::to_string_pretty(&json)?);
            } else {
                println!("Created capsule at {}", capsule_path.display());
            }
            Ok(())
        }
        Err(e) => {
            if args.json {
                let json = serde_json::json!({
                    "schema_version": SCHEMA_VERSION,
                    "tool_version": tool_version(),
                    "error": format!("{e}"),
                    "capsule_path": capsule_path.display().to_string(),
                });
                println!("{}", serde_json::to_string_pretty(&json)?);
            } else {
                eprintln!("Error: Failed to create capsule: {e}");
            }
            std::process::exit(capsule_exit::SYSTEM_ERROR);
        }
    }
}

/// Run `capsule events` command
///
/// SPEC-KIT-971: List events with optional filtering.
fn run_capsule_events(capsule_path: &PathBuf, args: CapsuleEventsArgs) -> anyhow::Result<()> {
    let config = CapsuleConfig {
        capsule_path: capsule_path.clone(),
        workspace_id: "cli".to_string(),
        ..Default::default()
    };

    let handle = match CapsuleHandle::open_read_only(config) {
        Ok(h) => h,
        Err(e) => {
            if args.json {
                let json = serde_json::json!({
                    "schema_version": SCHEMA_VERSION,
                    "tool_version": tool_version(),
                    "error": format!("{e}"),
                    "capsule_path": capsule_path.display().to_string(),
                });
                println!("{}", serde_json::to_string_pretty(&json)?);
            } else {
                eprintln!("Error: Failed to open capsule: {e}");
            }
            std::process::exit(capsule_exit::SYSTEM_ERROR);
        }
    };

    let mut events = handle.list_events();

    // Apply filters
    if let Some(ref stage) = args.stage {
        events.retain(|e| e.stage.as_deref() == Some(stage.as_str()));
    }
    if let Some(ref event_type) = args.event_type {
        events.retain(|e| format!("{:?}", e.event_type).to_lowercase() == event_type.to_lowercase());
    }
    if let Some(ref spec_id) = args.spec_id {
        events.retain(|e| &e.spec_id == spec_id);
    }
    if let Some(ref run_id) = args.run_id {
        events.retain(|e| &e.run_id == run_id);
    }

    // Apply limit
    if let Some(limit) = args.limit {
        events.truncate(limit);
    }

    if args.json {
        let events_json: Vec<_> = events
            .iter()
            .map(|e| {
                serde_json::json!({
                    "uri": e.uri.as_str(),
                    "event_type": format!("{:?}", e.event_type),
                    "timestamp": e.timestamp.to_rfc3339(),
                    "spec_id": e.spec_id,
                    "run_id": e.run_id,
                    "stage": e.stage,
                    "payload": e.payload,
                })
            })
            .collect();

        let json = serde_json::json!({
            "schema_version": SCHEMA_VERSION,
            "tool_version": tool_version(),
            "capsule_path": capsule_path.display().to_string(),
            "events": events_json,
            "count": events.len(),
            "filters": {
                "stage": args.stage,
                "event_type": args.event_type,
                "spec_id": args.spec_id,
                "run_id": args.run_id,
                "limit": args.limit,
            },
        });
        println!("{}", serde_json::to_string_pretty(&json)?);
    } else {
        println!("Capsule Events: {}", capsule_path.display());
        println!();
        if events.is_empty() {
            println!("  (no events matching filters)");
        } else {
            for event in &events {
                let stage_str = event.stage.as_deref().unwrap_or("-");
                println!(
                    "  {} | {:?} | {} | {}",
                    event.timestamp.format("%Y-%m-%d %H:%M:%S"),
                    event.event_type,
                    stage_str,
                    event.spec_id,
                );
            }
        }
        println!();
        println!("Total: {} event(s)", events.len());
    }

    Ok(())
}

/// Run `capsule export` command
///
/// SPEC-KIT-971: Export capsule to per-run archive.
fn run_capsule_export(capsule_path: &PathBuf, args: CapsuleExportArgs) -> anyhow::Result<()> {
    let config = CapsuleConfig {
        capsule_path: capsule_path.clone(),
        workspace_id: "cli".to_string(),
        ..Default::default()
    };

    let handle = match CapsuleHandle::open_read_only(config) {
        Ok(h) => h,
        Err(e) => {
            if args.json {
                let json = serde_json::json!({
                    "schema_version": SCHEMA_VERSION,
                    "tool_version": tool_version(),
                    "error": format!("{e}"),
                    "capsule_path": capsule_path.display().to_string(),
                });
                println!("{}", serde_json::to_string_pretty(&json)?);
            } else {
                eprintln!("Error: Failed to open capsule: {e}");
            }
            std::process::exit(capsule_exit::SYSTEM_ERROR);
        }
    };

    // Get events for this spec/run
    let events = handle.list_events();
    let run_events: Vec<_> = events
        .iter()
        .filter(|e| e.spec_id == args.spec_id && e.run_id == args.run_id)
        .collect();

    // Get checkpoints for this spec/run
    let checkpoints = handle.list_checkpoints();
    let run_checkpoints: Vec<_> = checkpoints
        .iter()
        .filter(|c| {
            c.spec_id.as_deref() == Some(args.spec_id.as_str())
                && c.run_id.as_deref() == Some(args.run_id.as_str())
        })
        .collect();

    // Determine output path
    let out_dir = args.out.unwrap_or_else(|| {
        capsule_path
            .parent()
            .unwrap_or(std::path::Path::new("."))
            .join("exports")
    });

    // Create export directory
    let export_name = format!("{}_{}", args.spec_id, args.run_id);
    let export_path = out_dir.join(&export_name);
    if let Err(e) = std::fs::create_dir_all(&export_path) {
        if args.json {
            let json = serde_json::json!({
                "schema_version": SCHEMA_VERSION,
                "tool_version": tool_version(),
                "error": format!("Failed to create export directory: {e}"),
            });
            println!("{}", serde_json::to_string_pretty(&json)?);
        } else {
            eprintln!("Error: Failed to create export directory: {e}");
        }
        std::process::exit(capsule_exit::SYSTEM_ERROR);
    }

    // Write events.json
    let events_json: Vec<_> = run_events
        .iter()
        .map(|e| {
            serde_json::json!({
                "uri": e.uri.as_str(),
                "event_type": format!("{:?}", e.event_type),
                "timestamp": e.timestamp.to_rfc3339(),
                "stage": e.stage,
                "payload": e.payload,
            })
        })
        .collect();
    let events_file = export_path.join("events.json");
    std::fs::write(&events_file, serde_json::to_string_pretty(&events_json)?)?;

    // Write checkpoints.json
    let cp_json: Vec<_> = run_checkpoints
        .iter()
        .map(|c| {
            serde_json::json!({
                "checkpoint_id": c.checkpoint_id.as_str(),
                "label": c.label,
                "stage": c.stage,
                "commit_hash": c.commit_hash,
                "timestamp": c.timestamp.to_rfc3339(),
            })
        })
        .collect();
    let checkpoints_file = export_path.join("checkpoints.json");
    std::fs::write(&checkpoints_file, serde_json::to_string_pretty(&cp_json)?)?;

    // Write manifest.json
    let manifest = serde_json::json!({
        "schema_version": SCHEMA_VERSION,
        "tool_version": tool_version(),
        "spec_id": args.spec_id,
        "run_id": args.run_id,
        "exported_at": chrono::Utc::now().to_rfc3339(),
        "source_capsule": capsule_path.display().to_string(),
        "event_count": run_events.len(),
        "checkpoint_count": run_checkpoints.len(),
    });
    let manifest_file = export_path.join("manifest.json");
    std::fs::write(&manifest_file, serde_json::to_string_pretty(&manifest)?)?;

    if args.json {
        let json = serde_json::json!({
            "schema_version": SCHEMA_VERSION,
            "tool_version": tool_version(),
            "exported": true,
            "export_path": export_path.display().to_string(),
            "spec_id": args.spec_id,
            "run_id": args.run_id,
            "event_count": run_events.len(),
            "checkpoint_count": run_checkpoints.len(),
            "files": [
                events_file.display().to_string(),
                checkpoints_file.display().to_string(),
                manifest_file.display().to_string(),
            ],
        });
        println!("{}", serde_json::to_string_pretty(&json)?);
    } else {
        println!("Exported run {} / {} to {}", args.spec_id, args.run_id, export_path.display());
        println!("  Events: {} → {}", run_events.len(), events_file.display());
        println!("  Checkpoints: {} → {}", run_checkpoints.len(), checkpoints_file.display());
        println!("  Manifest: {}", manifest_file.display());
    }

    Ok(())
}
