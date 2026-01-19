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
    CapsuleConfig, CapsuleError, CapsuleHandle, CheckpointId, DiagnosticResult, EventType,
    // SPEC-KIT-976: Memory Card and Logic Edge types
    ObjectType, CardType, EdgeType, MemoryCardV1, LogicEdgeV1, CardFact, FactValueType, LogicalUri,
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

    /// Reflex (local inference) management
    ///
    /// SPEC-KIT-978: Commands for reflex mode configuration and bakeoff analysis.
    /// Compare local inference performance against cloud inference.
    Reflex(ReflexArgs),

    /// Policy snapshot management
    ///
    /// SPEC-KIT-977: Commands for viewing and validating policy snapshots.
    /// List, show, and validate policy configurations.
    Policy(PolicyArgs),

    /// Replay run events from capsule
    ///
    /// SPEC-KIT-975: Commands for replaying and verifying run events.
    /// Display timeline of events, verify determinism, check URI resolution.
    Replay(ReplayArgs),

    /// Graph operations (Logic Mesh)
    ///
    /// SPEC-KIT-976: Commands for managing memory cards and logic mesh edges.
    /// Add cards, edges, and query the knowledge graph.
    Graph(GraphArgs),
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

    /// Force creation even if label already exists on branch
    #[arg(long = "force", short = 'f')]
    pub force: bool,

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
/// SPEC-KIT-975: Adds branch, since-checkpoint, and audit-only filters.
#[derive(Debug, Parser)]
pub struct CapsuleEventsArgs {
    /// Filter by stage (e.g., "plan", "implement")
    #[arg(long = "stage", short = 's', value_name = "STAGE")]
    pub stage: Option<String>,

    /// Filter by event type (e.g., "StageTransition", "ToolCall", "GateDecision")
    ///
    /// Valid types: StageTransition, PolicySnapshotRef, RoutingDecision, BranchMerged,
    /// DebugTrace, RetrievalRequest, RetrievalResponse, ToolCall, ToolResult,
    /// PatchApply, GateDecision, ErrorEvent, ModelCallEnvelope, CapsuleExported, CapsuleImported
    #[arg(long = "type", short = 't', value_name = "TYPE")]
    pub event_type: Option<String>,

    /// Filter by spec ID
    #[arg(long = "spec", value_name = "SPEC-ID")]
    pub spec_id: Option<String>,

    /// Filter by run ID
    #[arg(long = "run", value_name = "RUN-ID")]
    pub run_id: Option<String>,

    /// Filter by branch ID (e.g., "main", "run/SPEC-KIT-975_20260117_abc12345")
    #[arg(long = "branch", short = 'b', value_name = "BRANCH")]
    pub branch: Option<String>,

    /// Only show events since this checkpoint
    #[arg(long = "since-checkpoint", value_name = "CHECKPOINT-ID")]
    pub since_checkpoint: Option<String>,

    /// Only show audit-critical events (SPEC-KIT-975)
    #[arg(long = "audit-only")]
    pub audit_only: bool,

    /// Only show curated-eligible events (excludes debug/sensitive)
    #[arg(long = "curated-only")]
    pub curated_only: bool,

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

// =============================================================================
// SPEC-KIT-978: Reflex (Local Inference) Commands
// =============================================================================

/// Arguments for `speckit reflex` subcommand
///
/// SPEC-KIT-978: Reflex mode management and bakeoff analysis.
#[derive(Debug, Parser)]
pub struct ReflexArgs {
    #[command(subcommand)]
    pub command: ReflexSubcommand,
}

#[derive(Debug, Subcommand)]
pub enum ReflexSubcommand {
    /// Show bakeoff statistics (reflex vs cloud)
    ///
    /// Compares performance metrics between local reflex inference
    /// and cloud inference for data-driven routing decisions.
    Bakeoff(ReflexBakeoffArgs),

    /// Check if reflex meets bakeoff thresholds
    ///
    /// Validates P95 latency, success rate, and JSON compliance
    /// against configured thresholds.
    Check(ReflexCheckArgs),

    /// Run bakeoff trials against reflex endpoint
    ///
    /// Executes N trials through the local reflex endpoint, records metrics,
    /// and generates report files. Use this to collect fresh data before
    /// running `check`.
    RunBakeoff(ReflexRunBakeoffArgs),
}

/// Arguments for `speckit reflex bakeoff`
#[derive(Debug, Parser)]
pub struct ReflexBakeoffArgs {
    /// Duration to analyze (default: 24h, format: 1h, 7d, 30d)
    #[arg(long = "since", short = 's', default_value = "24h")]
    pub since: String,

    /// Output as JSON instead of text
    #[arg(long = "json", short = 'j')]
    pub json: bool,
}

/// Arguments for `speckit reflex check`
#[derive(Debug, Parser)]
pub struct ReflexCheckArgs {
    /// Duration to analyze (default: 24h)
    #[arg(long = "since", short = 's', default_value = "24h")]
    pub since: String,

    /// Minimum samples required (default: 10)
    #[arg(long = "min-samples", default_value = "10")]
    pub min_samples: u64,

    /// Output as JSON instead of text
    #[arg(long = "json", short = 'j')]
    pub json: bool,

    /// CI gate mode: fail with exit code 1 if reflex is enabled but thresholds not met
    ///
    /// Use this in CI pipelines. When enabled, the command will:
    /// - Exit 0 if reflex is disabled in policy (no check needed)
    /// - Exit 0 if reflex is enabled AND thresholds are met
    /// - Exit 1 if reflex is enabled AND thresholds are NOT met
    #[arg(long = "ci-gate")]
    pub ci_gate: bool,
}

/// Arguments for `speckit reflex run-bakeoff`
#[derive(Debug, Parser)]
pub struct ReflexRunBakeoffArgs {
    /// Number of trials to run (default: 10)
    #[arg(long = "trials", short = 'n', default_value = "10")]
    pub trials: u32,

    /// Output as JSON instead of text
    #[arg(long = "json", short = 'j')]
    pub json: bool,

    /// Fail with exit code 1 if thresholds are not met
    #[arg(long = "fail-on-threshold")]
    pub fail_on_threshold: bool,
}

// =============================================================================
// SPEC-KIT-977: Policy Commands
// =============================================================================

/// Arguments for `speckit policy` subcommand
///
/// SPEC-KIT-977: Policy snapshot management and validation.
#[derive(Debug, Parser)]
pub struct PolicyArgs {
    #[command(subcommand)]
    pub command: PolicySubcommand,
}

#[derive(Debug, Subcommand)]
pub enum PolicySubcommand {
    /// List all policy snapshots
    ///
    /// Shows policy ID, creation time, and hash for each snapshot.
    List(PolicyListArgs),

    /// Show details of a specific policy snapshot
    ///
    /// Displays full policy configuration including model_config,
    /// weights, and governance settings.
    Show(PolicyShowArgs),

    /// Show the current (latest) policy snapshot
    ///
    /// Equivalent to `policy show <latest-id>`.
    Current(PolicyCurrentArgs),

    /// Validate model_policy.toml
    ///
    /// Checks that the policy file exists and has valid structure.
    Validate(PolicyValidateArgs),

    /// Compare two policy snapshots
    ///
    /// Shows differences in governance, model_config, weights, and other fields.
    /// Output is deterministic with stable ordering for reproducibility.
    Diff(PolicyDiffArgs),
}

/// Arguments for `speckit policy diff`
#[derive(Debug, Parser)]
pub struct PolicyDiffArgs {
    /// First policy ID (older)
    #[arg(value_name = "POLICY-ID-A")]
    pub policy_id_a: String,

    /// Second policy ID (newer)
    #[arg(value_name = "POLICY-ID-B")]
    pub policy_id_b: String,

    /// Output as JSON instead of text
    #[arg(long = "json", short = 'j')]
    pub json: bool,
}

/// Arguments for `speckit policy list`
#[derive(Debug, Parser)]
pub struct PolicyListArgs {
    /// Maximum number of policies to show
    #[arg(long = "limit", short = 'n', default_value = "20")]
    pub limit: usize,

    /// Output as JSON instead of text
    #[arg(long = "json", short = 'j')]
    pub json: bool,
}

/// Arguments for `speckit policy show`
#[derive(Debug, Parser)]
pub struct PolicyShowArgs {
    /// Policy ID to show
    #[arg(value_name = "POLICY-ID")]
    pub policy_id: String,

    /// Output as JSON instead of text
    #[arg(long = "json", short = 'j')]
    pub json: bool,
}

/// Arguments for `speckit policy current`
#[derive(Debug, Parser)]
pub struct PolicyCurrentArgs {
    /// Output as JSON instead of text
    #[arg(long = "json", short = 'j')]
    pub json: bool,
}

/// Arguments for `speckit policy validate`
#[derive(Debug, Parser)]
pub struct PolicyValidateArgs {
    /// Path to model_policy.toml (default: auto-detect)
    #[arg(long = "path", short = 'p', value_name = "PATH")]
    pub path: Option<PathBuf>,

    /// Output as JSON instead of text
    #[arg(long = "json", short = 'j')]
    pub json: bool,
}

// =============================================================================
// SPEC-KIT-975: Replay Commands
// =============================================================================

/// Arguments for `speckit replay`
#[derive(Debug, Parser)]
pub struct ReplayArgs {
    #[command(subcommand)]
    pub command: ReplaySubcommand,
}

#[derive(Debug, Subcommand)]
pub enum ReplaySubcommand {
    /// Display run timeline
    ///
    /// Shows all events for a run in chronological order.
    /// Use --json for machine-readable output.
    Run(ReplayRunArgs),

    /// Verify run determinism
    ///
    /// Checks that retrieval responses resolve in capsule.
    /// Validates event sequence and checkpoint integrity.
    Verify(ReplayVerifyArgs),
}

/// Arguments for `speckit replay run`
#[derive(Debug, Parser)]
pub struct ReplayRunArgs {
    /// Run ID to replay
    #[arg(long = "run", short = 'r', value_name = "RUN_ID")]
    pub run_id: String,

    /// Optional branch filter (default: run/<RUN_ID>)
    #[arg(long = "branch", short = 'b')]
    pub branch: Option<String>,

    /// Output as JSON instead of text
    #[arg(long = "json", short = 'j')]
    pub json: bool,

    /// Filter to specific event types (comma-separated)
    #[arg(long = "types", value_name = "TYPES")]
    pub event_types: Option<String>,

    /// Capsule path override
    #[arg(long = "capsule", short = 'C', value_name = "PATH")]
    pub capsule_path: Option<PathBuf>,
}

/// Arguments for `speckit replay verify`
#[derive(Debug, Parser)]
pub struct ReplayVerifyArgs {
    /// Run ID to verify
    #[arg(long = "run", short = 'r', value_name = "RUN_ID")]
    pub run_id: String,

    /// Check retrieval response URIs resolve
    #[arg(long = "check-retrievals")]
    pub check_retrievals: bool,

    /// Check event sequence is monotonic
    #[arg(long = "check-sequence")]
    pub check_sequence: bool,

    /// Output as JSON instead of text
    #[arg(long = "json", short = 'j')]
    pub json: bool,

    /// Capsule path override
    #[arg(long = "capsule", short = 'C', value_name = "PATH")]
    pub capsule_path: Option<PathBuf>,
}

// =============================================================================
// SPEC-KIT-976: Graph (Logic Mesh) Commands
// =============================================================================

/// Arguments for `speckit graph` command
#[derive(Debug, Parser)]
pub struct GraphArgs {
    /// Capsule path override
    #[arg(long = "capsule", short = 'C', value_name = "PATH")]
    pub capsule_path: Option<PathBuf>,

    #[command(subcommand)]
    pub command: GraphSubcommand,
}

#[derive(Debug, Subcommand)]
pub enum GraphSubcommand {
    /// Add a memory card to the graph
    ///
    /// Creates a new card with the specified type and title.
    /// Use --fact KEY=VALUE to add facts.
    AddCard(GraphAddCardArgs),

    /// Add a logic edge to the graph
    ///
    /// Creates a relationship between two entities using mv2:// URIs.
    AddEdge(GraphAddEdgeArgs),

    /// Query the graph
    ///
    /// Lookup by URI, list by type, or traverse adjacencies.
    Query(GraphQueryArgs),
}

/// Arguments for `graph add-card`
#[derive(Debug, Parser)]
pub struct GraphAddCardArgs {
    /// Card type (spec, decision, task, risk, component, person, artifact, run)
    #[arg(long = "type", short = 't', value_name = "TYPE")]
    pub card_type: String,

    /// Card title (human-readable label)
    #[arg(long = "title", value_name = "TITLE")]
    pub title: String,

    /// Card ID (auto-generated UUID if not provided)
    #[arg(long = "id", value_name = "CARD-ID")]
    pub card_id: Option<String>,

    /// Add a fact: KEY=VALUE (can be repeated)
    #[arg(long = "fact", short = 'f', value_name = "KEY=VALUE")]
    pub facts: Vec<String>,

    /// SPEC ID for provenance
    #[arg(long = "spec", short = 's', value_name = "SPEC-ID")]
    pub spec_id: Option<String>,

    /// Run ID for provenance
    #[arg(long = "run", short = 'r', value_name = "RUN-ID")]
    pub run_id: Option<String>,

    /// Output as JSON
    #[arg(long = "json", short = 'j')]
    pub json: bool,
}

/// Arguments for `graph add-edge`
#[derive(Debug, Parser)]
pub struct GraphAddEdgeArgs {
    /// Edge type (depends_on, blocks, implements, references, owns, risks, related_to)
    #[arg(long = "type", short = 't', value_name = "TYPE")]
    pub edge_type: String,

    /// Source URI (mv2://...)
    #[arg(long = "from", value_name = "URI")]
    pub from_uri: String,

    /// Target URI (mv2://...)
    #[arg(long = "to", value_name = "URI")]
    pub to_uri: String,

    /// Edge ID (auto-generated UUID if not provided)
    #[arg(long = "id", value_name = "EDGE-ID")]
    pub edge_id: Option<String>,

    /// Optional weight/confidence (0.0-1.0)
    #[arg(long = "weight", short = 'w', value_name = "N")]
    pub weight: Option<f64>,

    /// SPEC ID for provenance
    #[arg(long = "spec", short = 's', value_name = "SPEC-ID")]
    pub spec_id: Option<String>,

    /// Run ID for provenance
    #[arg(long = "run", short = 'r', value_name = "RUN-ID")]
    pub run_id: Option<String>,

    /// Output as JSON
    #[arg(long = "json", short = 'j')]
    pub json: bool,
}

/// Arguments for `graph query`
#[derive(Debug, Parser)]
pub struct GraphQueryArgs {
    /// Lookup by specific URI (mv2://...)
    #[arg(long = "uri", value_name = "URI")]
    pub uri: Option<String>,

    /// List by object type (card, edge)
    #[arg(long = "type", short = 't', value_name = "TYPE")]
    pub object_type: Option<String>,

    /// Filter by card type (only with --type card)
    #[arg(long = "card-type", value_name = "CARD-TYPE")]
    pub card_type: Option<String>,

    /// Filter by edge type (only with --type edge)
    #[arg(long = "edge-type", value_name = "EDGE-TYPE")]
    pub edge_type: Option<String>,

    /// Adjacency query: find edges connected to this URI
    #[arg(long = "adjacency", short = 'a', value_name = "URI")]
    pub adjacency: Option<String>,

    /// Traversal depth for adjacency query (default: 1)
    #[arg(long = "depth", short = 'd', value_name = "N", default_value = "1")]
    pub depth: u32,

    /// Limit number of results
    #[arg(long = "limit", short = 'n', value_name = "N")]
    pub limit: Option<usize>,

    /// Output as JSON
    #[arg(long = "json", short = 'j')]
    pub json: bool,

    /// Capsule path override
    #[arg(long = "capsule", short = 'C', value_name = "PATH")]
    pub capsule_path: Option<PathBuf>,
}

impl SpeckitCli {
    /// Run the speckit CLI command
    pub async fn run(self) -> anyhow::Result<()> {
        let cwd = self
            .cwd
            .unwrap_or_else(|| std::env::current_dir().unwrap_or_else(|_| PathBuf::from(".")));

        // Handle capsule, reflex, and policy commands separately (don't need executor)
        if let SpeckitSubcommand::Capsule(args) = self.command {
            return run_capsule(cwd, args);
        }
        if let SpeckitSubcommand::Reflex(args) = self.command {
            return run_reflex(args);
        }
        if let SpeckitSubcommand::Policy(args) = self.command {
            return run_policy(cwd, args);
        }
        if let SpeckitSubcommand::Replay(args) = self.command {
            return run_replay(cwd, args);
        }
        if let SpeckitSubcommand::Graph(args) = self.command {
            return run_graph(cwd, args);
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
            SpeckitSubcommand::Reflex(_) => unreachable!("Reflex handled above"),
            SpeckitSubcommand::Policy(_) => unreachable!("Policy handled above"),
            SpeckitSubcommand::Replay(_) => unreachable!("Replay handled above"),
            SpeckitSubcommand::Graph(_) => unreachable!("Graph handled above"),
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
    pub const USER_ERROR: i32 = 1;      // Bad args, invalid URI
    pub const SYSTEM_ERROR: i32 = 2;    // Corrupt capsule, locked, IO error
    pub const VALIDATION_ERROR: i32 = 3; // Invalid event type, invalid checkpoint ID
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

    // Use canonical workspace_id for URI consistency (SPEC-KIT-971/977 alignment)
    let config = CapsuleConfig {
        capsule_path: capsule_path.clone(),
        workspace_id: "default".to_string(),
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

    // SPEC-KIT-971: Use commit_manual_with_options for force flag support
    let checkpoint_id = match handle.commit_manual_with_options(&args.label, args.force) {
        Ok(id) => id,
        Err(CapsuleError::DuplicateLabel { label, branch }) => {
            // DuplicateLabel is a user-correctable error (use --force)
            if args.json {
                let json = serde_json::json!({
                    "schema_version": SCHEMA_VERSION,
                    "tool_version": tool_version(),
                    "error": format!("Label '{}' already exists on branch '{}'", label, branch),
                    "error_code": "DUPLICATE_LABEL",
                    "hint": "Use --force to create duplicate label",
                    "capsule_path": capsule_path.display().to_string(),
                    "label": label,
                    "branch": branch,
                });
                println!("{}", serde_json::to_string_pretty(&json)?);
            } else {
                eprintln!("Error: Label '{}' already exists on branch '{}'", label, branch);
                eprintln!("Hint: Use --force to create duplicate label");
            }
            std::process::exit(capsule_exit::USER_ERROR);
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
/// SPEC-KIT-975: Adds branch, since-checkpoint, audit-only, and curated-only filters.
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

    // Validate event type if provided
    let event_type_filter: Option<EventType> = if let Some(ref type_str) = args.event_type {
        match EventType::from_str(type_str) {
            Some(et) => Some(et),
            None => {
                let valid_types = EventType::all_variants().join(", ");
                if args.json {
                    let json = serde_json::json!({
                        "schema_version": SCHEMA_VERSION,
                        "tool_version": tool_version(),
                        "error": format!("Invalid event type: '{}'. Valid types: {}", type_str, valid_types),
                        "valid_types": EventType::all_variants(),
                    });
                    println!("{}", serde_json::to_string_pretty(&json)?);
                } else {
                    eprintln!("Error: Invalid event type: '{}'", type_str);
                    eprintln!("Valid types: {}", valid_types);
                }
                std::process::exit(capsule_exit::VALIDATION_ERROR);
            }
        }
    } else {
        None
    };

    // Get since-checkpoint timestamp if provided
    let since_timestamp = if let Some(ref checkpoint_id) = args.since_checkpoint {
        let checkpoints = handle.list_checkpoints();
        checkpoints
            .iter()
            .find(|cp| cp.checkpoint_id.as_str() == checkpoint_id)
            .map(|cp| cp.timestamp)
    } else {
        None
    };

    let mut events = handle.list_events();

    // Apply filters
    if let Some(ref stage) = args.stage {
        events.retain(|e| e.stage.as_deref() == Some(stage.as_str()));
    }
    if let Some(et) = event_type_filter {
        events.retain(|e| e.event_type == et);
    }
    if let Some(ref spec_id) = args.spec_id {
        events.retain(|e| &e.spec_id == spec_id);
    }
    if let Some(ref run_id) = args.run_id {
        events.retain(|e| &e.run_id == run_id);
    }
    // SPEC-KIT-975: Branch filter
    if let Some(ref branch) = args.branch {
        events.retain(|e| e.branch_id.as_deref() == Some(branch.as_str()));
    }
    // SPEC-KIT-975: Since checkpoint filter
    if let Some(since) = since_timestamp {
        events.retain(|e| e.timestamp > since);
    }
    // SPEC-KIT-975: Audit-only filter
    if args.audit_only {
        events.retain(|e| e.event_type.is_audit_critical());
    }
    // SPEC-KIT-975: Curated-only filter
    if args.curated_only {
        events.retain(|e| e.event_type.is_curated_eligible());
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
                    "event_type": e.event_type.as_str(),
                    "timestamp": e.timestamp.to_rfc3339(),
                    "spec_id": e.spec_id,
                    "run_id": e.run_id,
                    "stage": e.stage,
                    "branch_id": e.branch_id,
                    "is_curated_eligible": e.event_type.is_curated_eligible(),
                    "is_audit_critical": e.event_type.is_audit_critical(),
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
                "branch": args.branch,
                "since_checkpoint": args.since_checkpoint,
                "audit_only": args.audit_only,
                "curated_only": args.curated_only,
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

// =============================================================================
// SPEC-KIT-978: Reflex Commands Implementation
// =============================================================================

/// Run reflex subcommand
fn run_reflex(args: ReflexArgs) -> anyhow::Result<()> {
    match args.command {
        ReflexSubcommand::Bakeoff(args) => run_reflex_bakeoff(args),
        ReflexSubcommand::Check(args) => run_reflex_check(args),
        ReflexSubcommand::RunBakeoff(args) => run_reflex_run_bakeoff(args),
    }
}

/// Parse duration string (e.g., "1h", "24h", "7d") to Duration
fn parse_duration_str(s: &str) -> anyhow::Result<std::time::Duration> {
    let s = s.trim().to_lowercase();

    if s.ends_with('h') {
        let hours: u64 = s[..s.len() - 1]
            .parse()
            .map_err(|_| anyhow::anyhow!("Invalid hours: {}", s))?;
        Ok(std::time::Duration::from_secs(hours * 3600))
    } else if s.ends_with('d') {
        let days: u64 = s[..s.len() - 1]
            .parse()
            .map_err(|_| anyhow::anyhow!("Invalid days: {}", s))?;
        Ok(std::time::Duration::from_secs(days * 86400))
    } else if s.ends_with('m') {
        let mins: u64 = s[..s.len() - 1]
            .parse()
            .map_err(|_| anyhow::anyhow!("Invalid minutes: {}", s))?;
        Ok(std::time::Duration::from_secs(mins * 60))
    } else {
        // Try parsing as seconds
        let secs: u64 = s
            .parse()
            .map_err(|_| anyhow::anyhow!("Invalid duration format: {}", s))?;
        Ok(std::time::Duration::from_secs(secs))
    }
}

/// Run `speckit reflex bakeoff` command
///
/// Shows P95 latency, success rate, and JSON compliance comparing reflex vs cloud.
fn run_reflex_bakeoff(args: ReflexBakeoffArgs) -> anyhow::Result<()> {
    use codex_tui::reflex_metrics::ReflexMetricsDb;

    let db = ReflexMetricsDb::init_default()?;
    let since = parse_duration_str(&args.since)?;
    let stats = db.compute_bakeoff_stats(since)?;

    if args.json {
        let json = serde_json::json!({
            "schema_version": SCHEMA_VERSION,
            "tool_version": tool_version(),
            "period": {
                "start": stats.period_start,
                "end": stats.period_end,
                "duration": args.since,
            },
            "total_attempts": stats.total_attempts,
            "reflex": stats.reflex.as_ref().map(|r| serde_json::json!({
                "total_attempts": r.total_attempts,
                "success_count": r.success_count,
                "success_rate": r.success_rate,
                "json_compliant_count": r.json_compliant_count,
                "json_compliance_rate": r.json_compliance_rate,
                "latency_ms": {
                    "avg": r.avg_latency_ms,
                    "p50": r.p50_latency_ms,
                    "p95": r.p95_latency_ms,
                    "p99": r.p99_latency_ms,
                    "min": r.min_latency_ms,
                    "max": r.max_latency_ms,
                },
            })),
            "cloud": stats.cloud.as_ref().map(|c| serde_json::json!({
                "total_attempts": c.total_attempts,
                "success_count": c.success_count,
                "success_rate": c.success_rate,
                "json_compliant_count": c.json_compliant_count,
                "json_compliance_rate": c.json_compliance_rate,
                "latency_ms": {
                    "avg": c.avg_latency_ms,
                    "p50": c.p50_latency_ms,
                    "p95": c.p95_latency_ms,
                    "p99": c.p99_latency_ms,
                    "min": c.min_latency_ms,
                    "max": c.max_latency_ms,
                },
            })),
        });
        println!("{}", serde_json::to_string_pretty(&json)?);
    } else {
        println!("Reflex Bakeoff Statistics (since {})", args.since);
        println!("  Period: {} - {}", stats.period_start, stats.period_end);
        println!("  Total Attempts: {}", stats.total_attempts);
        println!();

        // Reflex stats
        if let Some(ref r) = stats.reflex {
            println!("  REFLEX (local inference):");
            println!("    Attempts:        {}", r.total_attempts);
            println!("    Success Rate:    {:.1}% ({}/{})", r.success_rate, r.success_count, r.total_attempts);
            println!("    JSON Compliance: {:.1}% ({}/{})", r.json_compliance_rate, r.json_compliant_count, r.total_attempts);
            println!("    Latency (ms):");
            println!("      P50:  {}ms", r.p50_latency_ms);
            println!("      P95:  {}ms", r.p95_latency_ms);
            println!("      P99:  {}ms", r.p99_latency_ms);
            println!("      Avg:  {:.1}ms", r.avg_latency_ms);
            println!("      Min:  {}ms, Max: {}ms", r.min_latency_ms, r.max_latency_ms);
        } else {
            println!("  REFLEX: No data");
        }

        println!();

        // Cloud stats
        if let Some(ref c) = stats.cloud {
            println!("  CLOUD (remote inference):");
            println!("    Attempts:        {}", c.total_attempts);
            println!("    Success Rate:    {:.1}% ({}/{})", c.success_rate, c.success_count, c.total_attempts);
            println!("    JSON Compliance: {:.1}% ({}/{})", c.json_compliance_rate, c.json_compliant_count, c.total_attempts);
            println!("    Latency (ms):");
            println!("      P50:  {}ms", c.p50_latency_ms);
            println!("      P95:  {}ms", c.p95_latency_ms);
            println!("      P99:  {}ms", c.p99_latency_ms);
            println!("      Avg:  {:.1}ms", c.avg_latency_ms);
            println!("      Min:  {}ms, Max: {}ms", c.min_latency_ms, c.max_latency_ms);
        } else {
            println!("  CLOUD: No data");
        }

        // Comparison summary
        if stats.reflex.is_some() && stats.cloud.is_some() {
            let reflex = stats.reflex.as_ref().unwrap();
            let cloud = stats.cloud.as_ref().unwrap();
            println!();
            println!("  COMPARISON:");
            let latency_ratio = if cloud.p95_latency_ms > 0 {
                cloud.p95_latency_ms as f64 / reflex.p95_latency_ms.max(1) as f64
            } else {
                0.0
            };
            println!("    P95 Latency Ratio: {:.1}x faster (reflex)", latency_ratio);
            println!("    Success Parity:    {:.1}% (reflex vs cloud)",
                     reflex.success_rate / cloud.success_rate.max(0.01) * 100.0);
        }
    }

    Ok(())
}

/// Run `speckit reflex check` command
///
/// Validates if reflex meets bakeoff thresholds.
fn run_reflex_check(args: ReflexCheckArgs) -> anyhow::Result<()> {
    use codex_tui::reflex_metrics::ReflexMetricsDb;
    use codex_stage0::reflex_config::load_reflex_config;
    use codex_stage0::GovernancePolicy;

    // CI gate mode: check if reflex is enabled in policy first
    if args.ci_gate {
        // Load governance policy to check if reflex is enabled
        let reflex_enabled = match GovernancePolicy::load(None) {
            Ok(policy) => policy.routing.reflex.enabled,
            Err(_) => {
                // No policy file = reflex not enabled
                false
            }
        };

        if !reflex_enabled {
            if args.json {
                let json = serde_json::json!({
                    "schema_version": SCHEMA_VERSION,
                    "tool_version": tool_version(),
                    "ci_gate": true,
                    "reflex_enabled": false,
                    "passes": true,
                    "reason": "Reflex is disabled in policy - no check needed",
                });
                println!("{}", serde_json::to_string_pretty(&json)?);
            } else {
                println!("CI GATE PASS: Reflex is disabled in policy - no check needed");
            }
            return Ok(());
        }
    }

    let db = ReflexMetricsDb::init_default()?;
    let since = parse_duration_str(&args.since)?;

    // Load thresholds from config
    let config = load_reflex_config(None).unwrap_or_default();
    let thresholds = &config.thresholds;

    let (passes, reason) = db.check_thresholds(
        since,
        args.min_samples,
        thresholds.p95_latency_ms,
        thresholds.success_parity_percent,
        thresholds.json_schema_compliance_percent,
    )?;

    if args.json {
        let json = serde_json::json!({
            "schema_version": SCHEMA_VERSION,
            "tool_version": tool_version(),
            "ci_gate": args.ci_gate,
            "reflex_enabled": true,
            "passes": passes,
            "reason": reason,
            "thresholds": {
                "p95_latency_ms": thresholds.p95_latency_ms,
                "success_parity_percent": thresholds.success_parity_percent,
                "json_schema_compliance_percent": thresholds.json_schema_compliance_percent,
                "min_samples": args.min_samples,
            },
            "period": args.since,
        });
        println!("{}", serde_json::to_string_pretty(&json)?);
    } else {
        if passes {
            println!("{}: {}", if args.ci_gate { "CI GATE PASS" } else { "PASS" }, reason);
            println!();
            println!("Thresholds met:");
            println!("  P95 Latency:      < {}ms", thresholds.p95_latency_ms);
            println!("  Success Parity:   >= {}%", thresholds.success_parity_percent);
            println!("  JSON Compliance:  >= {}%", thresholds.json_schema_compliance_percent);
            println!("  Min Samples:      >= {}", args.min_samples);
        } else {
            println!("{}: {}", if args.ci_gate { "CI GATE FAIL" } else { "FAIL" }, reason);
            std::process::exit(1);
        }
    }

    Ok(())
}

/// Run `speckit reflex run-bakeoff` command
///
/// Executes bakeoff trials and writes report files.
fn run_reflex_run_bakeoff(args: ReflexRunBakeoffArgs) -> anyhow::Result<()> {
    use codex_stage0::reflex_config::load_reflex_config;
    use codex_tui::bakeoff_runner::{run_bakeoff, BakeoffConfig};

    // Load reflex config
    let reflex_config = load_reflex_config(None).map_err(|e| anyhow::anyhow!("{}", e))?;

    if !reflex_config.enabled {
        if args.json {
            let json = serde_json::json!({
                "schema_version": SCHEMA_VERSION,
                "tool_version": tool_version(),
                "error": "Reflex is not enabled in configuration",
                "reflex_enabled": false,
            });
            println!("{}", serde_json::to_string_pretty(&json)?);
        } else {
            eprintln!("Error: Reflex is not enabled in configuration.");
            eprintln!();
            eprintln!("Enable reflex in model_policy.toml:");
            eprintln!("  [routing.reflex]");
            eprintln!("  enabled = true");
            eprintln!("  endpoint = \"http://127.0.0.1:3009/v1\"");
        }
        std::process::exit(1);
    }

    // Configure bakeoff
    let bakeoff_config = BakeoffConfig {
        trial_count: args.trials,
        p95_latency_threshold_ms: reflex_config.thresholds.p95_latency_ms,
        success_rate_threshold_pct: reflex_config.thresholds.success_parity_percent,
        json_compliance_threshold_pct: reflex_config.thresholds.json_schema_compliance_percent,
        min_samples: 5,
    };

    if !args.json {
        println!("Running reflex bakeoff...");
        println!("  Trials:    {}", bakeoff_config.trial_count);
        println!("  Endpoint:  {}", reflex_config.endpoint);
        println!("  Model:     {}", reflex_config.model);
        println!();
    }

    // Run bakeoff (blocking async call)
    let cwd = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
    let report = tokio::runtime::Runtime::new()?.block_on(async {
        run_bakeoff(&cwd, &bakeoff_config, &reflex_config).await
    })?;

    // Write report files
    let (json_path, md_path) = report.write_files(&cwd)?;

    if args.json {
        let json = serde_json::json!({
            "schema_version": SCHEMA_VERSION,
            "tool_version": tool_version(),
            "report_id": report.report_id,
            "passes_thresholds": report.evaluation.passes_thresholds,
            "trials_run": report.trials.len(),
            "stats": {
                "reflex": report.stats.reflex.as_ref().map(|r| serde_json::json!({
                    "p95_latency_ms": r.p95_latency_ms,
                    "success_rate": r.success_rate,
                    "json_compliance_rate": r.json_compliance_rate,
                })),
            },
            "evaluation": {
                "p95_check": {
                    "passes": report.evaluation.p95_check.passes,
                    "actual": report.evaluation.p95_check.actual,
                    "threshold": report.evaluation.p95_check.threshold,
                },
                "success_rate_check": {
                    "passes": report.evaluation.success_rate_check.passes,
                    "actual": report.evaluation.success_rate_check.actual,
                    "threshold": report.evaluation.success_rate_check.threshold,
                },
                "json_compliance_check": {
                    "passes": report.evaluation.json_compliance_check.passes,
                    "actual": report.evaluation.json_compliance_check.actual,
                    "threshold": report.evaluation.json_compliance_check.threshold,
                },
            },
            "recommendation": report.evaluation.recommendation,
            "output_files": {
                "json": json_path,
                "markdown": md_path,
            },
        });
        println!("{}", serde_json::to_string_pretty(&json)?);
    } else {
        let status_emoji = if report.evaluation.passes_thresholds { "✅" } else { "❌" };
        println!("{} {}", status_emoji, report.evaluation.recommendation);
        println!();

        if let Some(ref reflex) = report.stats.reflex {
            println!("Results ({} trials):", report.trials.len());
            println!("  P95 Latency:      {}ms (threshold: {}ms)",
                reflex.p95_latency_ms, bakeoff_config.p95_latency_threshold_ms);
            println!("  Success Rate:     {:.1}% (threshold: {}%)",
                reflex.success_rate, bakeoff_config.success_rate_threshold_pct);
            println!("  JSON Compliance:  {:.1}% (threshold: {}%)",
                reflex.json_compliance_rate, bakeoff_config.json_compliance_threshold_pct);
        }

        println!();
        println!("Report files:");
        println!("  JSON: {}", json_path);
        println!("  Markdown: {}", md_path);
    }

    // Exit with error if thresholds not met and --fail-on-threshold is set
    if args.fail_on_threshold && !report.evaluation.passes_thresholds {
        std::process::exit(1);
    }

    Ok(())
}

// =============================================================================
// SPEC-KIT-977: Policy Commands Implementation
// =============================================================================

/// Run policy subcommand
fn run_policy(cwd: PathBuf, args: PolicyArgs) -> anyhow::Result<()> {
    match args.command {
        PolicySubcommand::List(args) => run_policy_list(cwd, args),
        PolicySubcommand::Show(args) => run_policy_show(cwd, args),
        PolicySubcommand::Current(args) => run_policy_current(cwd, args),
        PolicySubcommand::Validate(args) => run_policy_validate(cwd, args),
        PolicySubcommand::Diff(args) => run_policy_diff(cwd, args),
    }
}

/// Run `speckit policy list` command
fn run_policy_list(_cwd: PathBuf, args: PolicyListArgs) -> anyhow::Result<()> {
    use codex_stage0::PolicyStore;

    let store = PolicyStore::new();
    let policies = store.list().unwrap_or_default();

    if policies.is_empty() {
        if args.json {
            let json = serde_json::json!({
                "schema_version": SCHEMA_VERSION,
                "tool_version": tool_version(),
                "policies": [],
                "count": 0,
            });
            println!("{}", serde_json::to_string_pretty(&json)?);
        } else {
            println!("No policy snapshots found.");
            println!();
            println!("Policy snapshots are created when:");
            println!("  - A /speckit.auto run starts");
            println!("  - Policy drift is detected at stage boundaries");
            println!();
            println!("Location: .speckit/policies/");
        }
        return Ok(());
    }

    let policies: Vec<_> = policies.into_iter().take(args.limit).collect();

    if args.json {
        let policy_entries: Vec<_> = policies
            .iter()
            .map(|p| {
                serde_json::json!({
                    "policy_id": p.policy_id,
                    "created_at": p.created_at.to_rfc3339(),
                    "hash_short": p.hash_short,
                    "source_count": p.source_count,
                })
            })
            .collect();

        let json = serde_json::json!({
            "schema_version": SCHEMA_VERSION,
            "tool_version": tool_version(),
            "policies": policy_entries,
            "count": policies.len(),
        });
        println!("{}", serde_json::to_string_pretty(&json)?);
    } else {
        println!("Policy Snapshots ({} total)", policies.len());
        println!("{}", "=".repeat(70));
        println!();
        println!(
            "{:<40} {:<20} {:<10}",
            "POLICY ID", "CREATED", "HASH"
        );
        println!("{}", "-".repeat(70));

        for policy in &policies {
            let created = policy.created_at.format("%Y-%m-%d %H:%M:%S").to_string();
            println!(
                "{:<40} {:<20} {:<10}",
                policy.policy_id, created, policy.hash_short
            );
        }

        println!();
        println!("Use `code speckit policy show <POLICY-ID>` for details");
    }

    Ok(())
}

/// Run `speckit policy show` command
fn run_policy_show(_cwd: PathBuf, args: PolicyShowArgs) -> anyhow::Result<()> {
    use codex_stage0::PolicyStore;

    let store = PolicyStore::new();

    match store.load(&args.policy_id) {
        Ok(snapshot) => {
            if args.json {
                // Output the full snapshot as JSON
                let json = serde_json::json!({
                    "schema_version": SCHEMA_VERSION,
                    "tool_version": tool_version(),
                    "policy": snapshot,
                });
                println!("{}", serde_json::to_string_pretty(&json)?);
            } else {
                println!("Policy Snapshot: {}", snapshot.policy_id);
                println!("{}", "=".repeat(60));
                println!();
                println!("Created:        {}", snapshot.created_at.format("%Y-%m-%d %H:%M:%S UTC"));
                println!("Schema Version: {}", snapshot.schema_version);
                println!("Hash:           {}", snapshot.hash);
                println!();
                println!("Source Files:");
                for file in &snapshot.source_files {
                    println!("  - {}", file);
                }
                println!();
                println!("Model Config:");
                println!("  max_tokens:        {}", snapshot.model_config.max_tokens);
                println!("  top_k:             {}", snapshot.model_config.top_k);
                println!("  hybrid_enabled:    {}", snapshot.model_config.hybrid_enabled);
                println!("  tier2_enabled:     {}", snapshot.model_config.tier2_enabled);
                println!();
                println!("Scoring Weights:");
                println!("  usage:             {:.2}", snapshot.weights.usage);
                println!("  recency:           {:.2}", snapshot.weights.recency);
                println!("  priority:          {:.2}", snapshot.weights.priority);
                println!("  decay:             {:.2}", snapshot.weights.decay);

                if let Some(gov) = &snapshot.governance {
                    println!();
                    println!("Governance (from model_policy.toml):");
                    println!("  SOR primary:       {}", gov.system_of_record.primary);
                    println!("  Capture mode:      {}", gov.capture.mode);
                    println!("  Reflex enabled:    {}", gov.routing.reflex.enabled);
                }

                println!();
                println!("Hash verified:  {}", if snapshot.verify_hash() { "✓" } else { "✗" });
            }
        }
        Err(e) => {
            if args.json {
                let json = serde_json::json!({
                    "schema_version": SCHEMA_VERSION,
                    "tool_version": tool_version(),
                    "error": format!("Policy not found: {}", e),
                    "policy_id": args.policy_id,
                });
                println!("{}", serde_json::to_string_pretty(&json)?);
            } else {
                eprintln!("Error: Policy '{}' not found: {}", args.policy_id, e);
            }
            std::process::exit(1);
        }
    }

    Ok(())
}

/// Run `speckit policy current` command
fn run_policy_current(_cwd: PathBuf, args: PolicyCurrentArgs) -> anyhow::Result<()> {
    use codex_stage0::PolicyStore;

    let store = PolicyStore::new();

    match store.latest() {
        Ok(Some(snapshot)) => {
            if args.json {
                let json = serde_json::json!({
                    "schema_version": SCHEMA_VERSION,
                    "tool_version": tool_version(),
                    "policy": snapshot,
                });
                println!("{}", serde_json::to_string_pretty(&json)?);
            } else {
                println!("Current Policy Snapshot: {}", snapshot.policy_id);
                println!("{}", "=".repeat(60));
                println!();
                println!("Created:        {}", snapshot.created_at.format("%Y-%m-%d %H:%M:%S UTC"));
                println!("Hash:           {}", snapshot.hash);
                println!();

                if let Some(gov) = &snapshot.governance {
                    println!("Governance Summary:");
                    println!("  SOR primary:       {}", gov.system_of_record.primary);
                    println!("  Capture mode:      {}", gov.capture.mode);
                    println!("  Reflex enabled:    {}", gov.routing.reflex.enabled);
                    if gov.routing.reflex.enabled {
                        println!("  Reflex endpoint:   {}", gov.routing.reflex.endpoint);
                        println!("  Reflex model:      {}", gov.routing.reflex.model);
                    }
                }

                println!();
                println!("Use `code speckit policy show {}` for full details", snapshot.policy_id);
            }
        }
        Ok(None) => {
            if args.json {
                let json = serde_json::json!({
                    "schema_version": SCHEMA_VERSION,
                    "tool_version": tool_version(),
                    "policy": null,
                    "message": "No policy snapshots found",
                });
                println!("{}", serde_json::to_string_pretty(&json)?);
            } else {
                println!("No policy snapshots found.");
                println!();
                println!("Run a /speckit.auto pipeline to create a policy snapshot.");
            }
        }
        Err(e) => {
            if args.json {
                let json = serde_json::json!({
                    "schema_version": SCHEMA_VERSION,
                    "tool_version": tool_version(),
                    "error": format!("{}", e),
                });
                println!("{}", serde_json::to_string_pretty(&json)?);
            } else {
                eprintln!("Error loading policy: {}", e);
            }
            std::process::exit(1);
        }
    }

    Ok(())
}

/// Run `speckit policy validate` command
fn run_policy_validate(cwd: PathBuf, args: PolicyValidateArgs) -> anyhow::Result<()> {
    use codex_stage0::GovernancePolicy;

    // Find model_policy.toml
    let policy_path = if let Some(path) = args.path {
        path
    } else {
        // Auto-detect in cwd or parent
        let local = cwd.join("model_policy.toml");
        if local.exists() {
            local
        } else {
            let parent = cwd.parent().map(|p| p.join("model_policy.toml"));
            if let Some(p) = parent {
                if p.exists() {
                    p
                } else {
                    local // Will fail with "not found"
                }
            } else {
                local
            }
        }
    };

    if !policy_path.exists() {
        if args.json {
            let json = serde_json::json!({
                "schema_version": SCHEMA_VERSION,
                "tool_version": tool_version(),
                "valid": false,
                "error": "model_policy.toml not found",
                "path": policy_path.display().to_string(),
            });
            println!("{}", serde_json::to_string_pretty(&json)?);
        } else {
            eprintln!("Error: model_policy.toml not found at {}", policy_path.display());
            eprintln!();
            eprintln!("Expected locations:");
            eprintln!("  - ./model_policy.toml");
            eprintln!("  - ../model_policy.toml");
        }
        std::process::exit(1);
    }

    // Try to load and validate
    match GovernancePolicy::load(Some(&policy_path)) {
        Ok(policy) => {
            // Validate required sections
            let mut issues = Vec::new();

            if policy.meta.schema_version.is_empty() {
                issues.push("meta.schema_version is empty");
            }
            if policy.system_of_record.primary.is_empty() {
                issues.push("system_of_record.primary is empty");
            }
            if policy.capture.mode.is_empty() {
                issues.push("capture.mode is empty");
            }

            let valid = issues.is_empty();

            if args.json {
                let json = serde_json::json!({
                    "schema_version": SCHEMA_VERSION,
                    "tool_version": tool_version(),
                    "valid": valid,
                    "path": policy_path.display().to_string(),
                    "issues": issues,
                    "summary": {
                        "meta": {
                            "schema_version": policy.meta.schema_version,
                            "effective_date": policy.meta.effective_date,
                        },
                        "system_of_record": policy.system_of_record.primary,
                        "capture_mode": policy.capture.mode,
                        "reflex_enabled": policy.routing.reflex.enabled,
                    },
                });
                println!("{}", serde_json::to_string_pretty(&json)?);
            } else {
                if valid {
                    println!("✓ model_policy.toml is valid");
                    println!();
                    println!("Path:            {}", policy_path.display());
                    println!("Schema Version:  {}", policy.meta.schema_version);
                    println!("Effective Date:  {}", policy.meta.effective_date);
                    println!("SOR Primary:     {}", policy.system_of_record.primary);
                    println!("Capture Mode:    {}", policy.capture.mode);
                    println!("Reflex Enabled:  {}", policy.routing.reflex.enabled);
                } else {
                    println!("✗ model_policy.toml has issues:");
                    for issue in &issues {
                        println!("  - {}", issue);
                    }
                    std::process::exit(1);
                }
            }
        }
        Err(e) => {
            if args.json {
                let json = serde_json::json!({
                    "schema_version": SCHEMA_VERSION,
                    "tool_version": tool_version(),
                    "valid": false,
                    "error": format!("{}", e),
                    "path": policy_path.display().to_string(),
                });
                println!("{}", serde_json::to_string_pretty(&json)?);
            } else {
                eprintln!("✗ Failed to parse model_policy.toml: {}", e);
            }
            std::process::exit(1);
        }
    }

    Ok(())
}

/// Run `speckit policy diff` command
///
/// Compares two policy snapshots and shows differences in:
/// - Governance (routing, capture mode, SOR)
/// - Model configuration
/// - Scoring weights
/// - Source files
///
/// Output is deterministic with stable ordering.
fn run_policy_diff(_cwd: PathBuf, args: PolicyDiffArgs) -> anyhow::Result<()> {
    use codex_stage0::{PolicyDiff, PolicyStore};

    let store = PolicyStore::new();

    // Load both snapshots
    let snapshot_a = match store.load(&args.policy_id_a) {
        Ok(s) => s,
        Err(e) => {
            if args.json {
                let json = serde_json::json!({
                    "schema_version": SCHEMA_VERSION,
                    "tool_version": tool_version(),
                    "error": format!("Policy A not found: {}", e),
                    "policy_id_a": args.policy_id_a,
                });
                println!("{}", serde_json::to_string_pretty(&json)?);
            } else {
                eprintln!("Error: Policy '{}' not found: {}", args.policy_id_a, e);
            }
            std::process::exit(1);
        }
    };

    let snapshot_b = match store.load(&args.policy_id_b) {
        Ok(s) => s,
        Err(e) => {
            if args.json {
                let json = serde_json::json!({
                    "schema_version": SCHEMA_VERSION,
                    "tool_version": tool_version(),
                    "error": format!("Policy B not found: {}", e),
                    "policy_id_b": args.policy_id_b,
                });
                println!("{}", serde_json::to_string_pretty(&json)?);
            } else {
                eprintln!("Error: Policy '{}' not found: {}", args.policy_id_b, e);
            }
            std::process::exit(1);
        }
    };

    // Compute diff
    let diff = PolicyDiff::compute(&snapshot_a, &snapshot_b);

    if args.json {
        // JSON output with stable format
        let json = serde_json::json!({
            "schema_version": SCHEMA_VERSION,
            "tool_version": tool_version(),
            "policy_id_a": diff.policy_id_a,
            "policy_id_b": diff.policy_id_b,
            "hash_a": diff.hash_a,
            "hash_b": diff.hash_b,
            "identical": diff.identical,
            "change_count": diff.changes.len(),
            "changes": diff.changes.iter().map(|c| serde_json::json!({
                "path": c.path,
                "old_value": c.old_value,
                "new_value": c.new_value,
                "category": c.category.as_str(),
            })).collect::<Vec<_>>(),
            "changed_keys": diff.changed_keys(),
        });
        println!("{}", serde_json::to_string_pretty(&json)?);
    } else {
        // Human-readable text output
        println!("{}", diff.to_text());
    }

    Ok(())
}

// =============================================================================
// SPEC-KIT-975: Replay Command Handlers
// =============================================================================

/// Run the replay command (SPEC-KIT-975)
fn run_replay(cwd: PathBuf, args: ReplayArgs) -> anyhow::Result<()> {
    use codex_tui::memvid_adapter::{
        BranchId, default_capsule_config, LogicalUri,
    };

    match args.command {
        ReplaySubcommand::Run(run_args) => run_replay_run(cwd, run_args),
        ReplaySubcommand::Verify(verify_args) => run_replay_verify(cwd, verify_args),
    }
}

/// Run the `speckit replay run` command
fn run_replay_run(cwd: PathBuf, args: ReplayRunArgs) -> anyhow::Result<()> {
    use codex_tui::memvid_adapter::{BranchId, default_capsule_config};
    use std::collections::HashSet;

    // Get capsule config
    let config = if let Some(path) = &args.capsule_path {
        CapsuleConfig {
            capsule_path: path.clone(),
            workspace_id: "cli".to_string(),
            ..Default::default()
        }
    } else {
        default_capsule_config(&cwd)
    };

    // Open capsule
    let handle = match CapsuleHandle::open(config) {
        Ok(h) => h,
        Err(e) => {
            if args.json {
                let json = serde_json::json!({
                    "schema_version": SCHEMA_VERSION,
                    "tool_version": tool_version(),
                    "error": format!("Failed to open capsule: {}", e),
                });
                println!("{}", serde_json::to_string_pretty(&json)?);
            } else {
                eprintln!("Error: Failed to open capsule: {}", e);
            }
            std::process::exit(1);
        }
    };

    // Build branch filter
    let branch = args.branch
        .map(|b| BranchId::from_str(&b))
        .unwrap_or_else(|| BranchId::for_run(&args.run_id));

    // Get events for this run
    let mut events = handle.list_events_filtered(Some(&branch));
    events.retain(|e| e.run_id == args.run_id);

    // Filter by event types if specified
    if let Some(ref types_str) = args.event_types {
        let type_set: HashSet<&str> = types_str.split(',').map(|s| s.trim()).collect();
        events.retain(|e| type_set.contains(e.event_type.as_str()));
    }

    // Sort by timestamp
    events.sort_by_key(|e| e.timestamp);

    // Get checkpoints for this run
    let checkpoints = handle.list_checkpoints();
    let run_checkpoints: Vec<_> = checkpoints
        .iter()
        .filter(|c| c.run_id.as_deref() == Some(&args.run_id))
        .collect();

    if args.json {
        let timeline: Vec<_> = events.iter().map(|e| {
            serde_json::json!({
                "seq": extract_seq_from_uri(&e.uri),
                "timestamp": e.timestamp.to_rfc3339(),
                "event_type": e.event_type.as_str(),
                "stage": e.stage,
                "payload": e.payload,
                "uri": e.uri.as_str(),
            })
        }).collect();

        let json = serde_json::json!({
            "schema_version": SCHEMA_VERSION,
            "tool_version": tool_version(),
            "run_id": args.run_id,
            "branch": branch.as_str(),
            "event_count": events.len(),
            "checkpoint_count": run_checkpoints.len(),
            "timeline": timeline,
            "checkpoints": run_checkpoints.iter().map(|c| {
                serde_json::json!({
                    "id": c.checkpoint_id.as_str(),
                    "stage": c.stage,
                    "timestamp": c.timestamp.to_rfc3339(),
                })
            }).collect::<Vec<_>>(),
        });
        println!("{}", serde_json::to_string_pretty(&json)?);
    } else {
        println!("Run Timeline: {}", args.run_id);
        println!("Branch: {}", branch.as_str());
        println!();

        let mut current_stage: Option<String> = None;

        for event in &events {
            // Print stage header if changed
            if event.stage != current_stage {
                if let Some(ref stage) = event.stage {
                    println!("\n=== Stage: {} ===\n", stage);
                }
                current_stage = event.stage.clone();
            }

            // Print event
            let type_icon = match event.event_type {
                EventType::ToolCall => "[TOOL]",
                EventType::ToolResult => "[RESULT]",
                EventType::RetrievalRequest => "[QUERY]",
                EventType::RetrievalResponse => "[HITS]",
                EventType::PatchApply => "[PATCH]",
                EventType::ModelCallEnvelope => "[MODEL]",
                EventType::StageTransition => "[STAGE]",
                EventType::GateDecision => "[GATE]",
                _ => "[EVENT]",
            };

            println!("{} {} {}",
                event.timestamp.format("%H:%M:%S%.3f"),
                type_icon,
                format_event_summary(event),
            );
        }

        println!("\nTotal: {} events, {} checkpoints", events.len(), run_checkpoints.len());
    }

    Ok(())
}

/// Run the `speckit replay verify` command
fn run_replay_verify(cwd: PathBuf, args: ReplayVerifyArgs) -> anyhow::Result<()> {
    use codex_tui::memvid_adapter::{BranchId, default_capsule_config, LogicalUri};

    // Get capsule config
    let config = if let Some(path) = &args.capsule_path {
        CapsuleConfig {
            capsule_path: path.clone(),
            workspace_id: "cli".to_string(),
            ..Default::default()
        }
    } else {
        default_capsule_config(&cwd)
    };

    // Open capsule
    let handle = match CapsuleHandle::open(config) {
        Ok(h) => h,
        Err(e) => {
            if args.json {
                let json = serde_json::json!({
                    "schema_version": SCHEMA_VERSION,
                    "tool_version": tool_version(),
                    "error": format!("Failed to open capsule: {}", e),
                });
                println!("{}", serde_json::to_string_pretty(&json)?);
            } else {
                eprintln!("Error: Failed to open capsule: {}", e);
            }
            std::process::exit(1);
        }
    };

    let branch = BranchId::for_run(&args.run_id);
    let events = handle.list_events_filtered(Some(&branch));
    let run_events: Vec<_> = events.iter()
        .filter(|e| e.run_id == args.run_id)
        .collect();

    #[derive(serde::Serialize)]
    struct VerificationIssue {
        check: String,
        severity: String,
        message: String,
        event_uri: Option<String>,
    }

    let mut issues: Vec<VerificationIssue> = Vec::new();

    // Check 1: Retrieval response URIs resolve
    if args.check_retrievals {
        for event in run_events.iter().filter(|e| e.event_type == EventType::RetrievalResponse) {
            if let Some(uris) = event.payload.get("hit_uris").and_then(|v| v.as_array()) {
                for uri_val in uris {
                    if let Some(uri_str) = uri_val.as_str() {
                        if let Ok(uri) = uri_str.parse::<LogicalUri>() {
                            if handle.resolve_uri(&uri, None, None).is_err() {
                                issues.push(VerificationIssue {
                                    check: "retrieval_uri_resolve".to_string(),
                                    severity: "error".to_string(),
                                    message: format!("URI not resolvable: {}", uri_str),
                                    event_uri: Some(event.uri.as_str().to_string()),
                                });
                            }
                        }
                    }
                }
            }
        }
    }

    // Check 2: Event sequence is monotonic
    if args.check_sequence {
        let mut last_seq = 0u64;
        for event in &run_events {
            let seq = extract_seq_from_uri(&event.uri);
            if seq <= last_seq && seq != 0 {
                issues.push(VerificationIssue {
                    check: "sequence_monotonic".to_string(),
                    severity: "warning".to_string(),
                    message: format!("Non-monotonic sequence: {} after {}", seq, last_seq),
                    event_uri: Some(event.uri.as_str().to_string()),
                });
            }
            last_seq = seq;
        }
    }

    let passed = issues.iter().all(|i| i.severity != "error");

    if args.json {
        let json = serde_json::json!({
            "schema_version": SCHEMA_VERSION,
            "tool_version": tool_version(),
            "run_id": args.run_id,
            "passed": passed,
            "issue_count": issues.len(),
            "issues": issues,
        });
        println!("{}", serde_json::to_string_pretty(&json)?);
    } else {
        if passed {
            println!("Verification PASSED for run: {}", args.run_id);
        } else {
            println!("Verification FAILED for run: {}", args.run_id);
        }
        for issue in &issues {
            println!("  [{}/{}] {}", issue.check, issue.severity, issue.message);
        }
    }

    std::process::exit(if passed { 0 } else { 1 });
}

/// Extract sequence number from event URI
fn extract_seq_from_uri(uri: &codex_tui::memvid_adapter::LogicalUri) -> u64 {
    // URIs are like mv2://workspace/SPEC-ID/RUN-ID/event/SEQ
    uri.as_str()
        .rsplit('/')
        .next()
        .and_then(|s| s.parse().ok())
        .unwrap_or(0)
}

/// Format event summary for human-readable output
fn format_event_summary(event: &codex_tui::memvid_adapter::RunEventEnvelope) -> String {
    match event.event_type {
        EventType::ToolCall => {
            let tool = event.payload.get("tool_name")
                .and_then(|v| v.as_str())
                .unwrap_or("unknown");
            format!("Tool: {}", tool)
        }
        EventType::RetrievalResponse => {
            let count = event.payload.get("hit_uris")
                .and_then(|v| v.as_array())
                .map(|a| a.len())
                .unwrap_or(0);
            format!("{} hits", count)
        }
        EventType::ModelCallEnvelope => {
            let model = event.payload.get("model")
                .and_then(|v| v.as_str())
                .unwrap_or("unknown");
            let mode = event.payload.get("capture_mode")
                .and_then(|v| v.as_str())
                .unwrap_or("?");
            format!("{} (capture: {})", model, mode)
        }
        EventType::PatchApply => {
            let path = event.payload.get("file_path")
                .and_then(|v| v.as_str())
                .unwrap_or("?");
            let ptype = event.payload.get("patch_type")
                .and_then(|v| v.as_str())
                .unwrap_or("?");
            format!("{}: {}", ptype, path)
        }
        _ => event.event_type.as_str().to_string(),
    }
}

// =============================================================================
// SPEC-KIT-976: Graph (Logic Mesh) Command Handlers
// =============================================================================

/// Run the graph command
fn run_graph(cwd: PathBuf, args: GraphArgs) -> anyhow::Result<()> {
    let capsule_path = args
        .capsule_path
        .unwrap_or_else(|| cwd.join(DEFAULT_CAPSULE_PATH));

    match args.command {
        GraphSubcommand::AddCard(cmd_args) => run_graph_add_card(&capsule_path, cmd_args),
        GraphSubcommand::AddEdge(cmd_args) => run_graph_add_edge(&capsule_path, cmd_args),
        GraphSubcommand::Query(cmd_args) => run_graph_query(&cwd, &capsule_path, cmd_args),
    }
}

/// Run `graph add-card` command
fn run_graph_add_card(capsule_path: &PathBuf, args: GraphAddCardArgs) -> anyhow::Result<()> {
    // Validate card type
    let card_type = match CardType::from_str(&args.card_type) {
        Some(ct) => ct,
        None => {
            let valid_types = CardType::all_variants().join(", ");
            if args.json {
                let json = serde_json::json!({
                    "schema_version": SCHEMA_VERSION,
                    "tool_version": tool_version(),
                    "error": format!("Invalid card type: '{}'. Valid types: {}", args.card_type, valid_types),
                    "valid_types": CardType::all_variants(),
                });
                println!("{}", serde_json::to_string_pretty(&json)?);
            } else {
                eprintln!("Error: Invalid card type: '{}'", args.card_type);
                eprintln!("Valid types: {}", valid_types);
            }
            std::process::exit(2);
        }
    };

    // Generate card_id if not provided
    let card_id = args
        .card_id
        .unwrap_or_else(|| uuid::Uuid::new_v4().to_string());

    // Parse facts from KEY=VALUE format
    let facts: Vec<CardFact> = args
        .facts
        .iter()
        .filter_map(|f| {
            let parts: Vec<&str> = f.splitn(2, '=').collect();
            if parts.len() == 2 {
                Some(CardFact {
                    key: parts[0].to_string(),
                    value: serde_json::Value::String(parts[1].to_string()),
                    value_type: FactValueType::String,
                    confidence: None,
                    source_uris: Vec::new(),
                })
            } else {
                None
            }
        })
        .collect();

    // Create card
    let mut card = MemoryCardV1::new(&card_id, card_type, &args.title, "cli");
    card.facts = facts;
    if let Some(spec_id) = &args.spec_id {
        card = card.with_spec_id(spec_id);
    }
    if let Some(run_id) = &args.run_id {
        card = card.with_run_id(run_id);
    }

    // Open capsule and store
    let config = CapsuleConfig {
        capsule_path: capsule_path.clone(),
        workspace_id: "default".to_string(),
        ..Default::default()
    };

    let handle = match CapsuleHandle::open(config) {
        Ok(h) => h,
        Err(e) => {
            if args.json {
                let json = serde_json::json!({
                    "schema_version": SCHEMA_VERSION,
                    "tool_version": tool_version(),
                    "error": format!("Failed to open capsule: {}", e),
                });
                println!("{}", serde_json::to_string_pretty(&json)?);
            } else {
                eprintln!("Error: Failed to open capsule: {}", e);
            }
            std::process::exit(3);
        }
    };

    // Store card using put()
    let spec_id = args.spec_id.as_deref().unwrap_or("_global");
    let run_id = args.run_id.as_deref().unwrap_or("_manual");
    let data = card.to_bytes()?;
    let metadata = serde_json::json!({
        "card_type": card_type.as_str(),
        "title": args.title,
    });

    let uri = handle.put(spec_id, run_id, ObjectType::Card, &card_id, data, metadata)?;

    // Commit immediately
    handle.commit_manual(&format!("card:{}", card_id))?;

    // Output result
    if args.json {
        let json = serde_json::json!({
            "schema_version": SCHEMA_VERSION,
            "tool_version": tool_version(),
            "card_id": card_id,
            "uri": uri.as_str(),
            "card_type": card_type.as_str(),
            "title": args.title,
            "created": true,
        });
        println!("{}", serde_json::to_string_pretty(&json)?);
    } else {
        println!("Created card: {}", card_id);
        println!("  URI: {}", uri.as_str());
        println!("  Type: {}", card_type.as_str());
        println!("  Title: {}", args.title);
    }

    Ok(())
}

/// Run `graph add-edge` command
fn run_graph_add_edge(capsule_path: &PathBuf, args: GraphAddEdgeArgs) -> anyhow::Result<()> {
    // Validate edge type
    let edge_type = match EdgeType::from_str(&args.edge_type) {
        Some(et) => et,
        None => {
            let valid_types = EdgeType::all_variants().join(", ");
            if args.json {
                let json = serde_json::json!({
                    "schema_version": SCHEMA_VERSION,
                    "tool_version": tool_version(),
                    "error": format!("Invalid edge type: '{}'. Valid types: {}", args.edge_type, valid_types),
                    "valid_types": EdgeType::all_variants(),
                });
                println!("{}", serde_json::to_string_pretty(&json)?);
            } else {
                eprintln!("Error: Invalid edge type: '{}'", args.edge_type);
                eprintln!("Valid types: {}", valid_types);
            }
            std::process::exit(2);
        }
    };

    // Validate URIs are mv2://
    let from_uri: LogicalUri = match args.from_uri.parse() {
        Ok(u) => u,
        Err(_) => {
            if args.json {
                let json = serde_json::json!({
                    "schema_version": SCHEMA_VERSION,
                    "tool_version": tool_version(),
                    "error": "from_uri must be a valid mv2:// URI",
                    "from_uri": args.from_uri,
                });
                println!("{}", serde_json::to_string_pretty(&json)?);
            } else {
                eprintln!("Error: from_uri must be a valid mv2:// URI");
                eprintln!("  Got: {}", args.from_uri);
            }
            std::process::exit(2);
        }
    };

    let to_uri: LogicalUri = match args.to_uri.parse() {
        Ok(u) => u,
        Err(_) => {
            if args.json {
                let json = serde_json::json!({
                    "schema_version": SCHEMA_VERSION,
                    "tool_version": tool_version(),
                    "error": "to_uri must be a valid mv2:// URI",
                    "to_uri": args.to_uri,
                });
                println!("{}", serde_json::to_string_pretty(&json)?);
            } else {
                eprintln!("Error: to_uri must be a valid mv2:// URI");
                eprintln!("  Got: {}", args.to_uri);
            }
            std::process::exit(2);
        }
    };

    // Generate edge_id if not provided
    let edge_id = args
        .edge_id
        .unwrap_or_else(|| uuid::Uuid::new_v4().to_string());

    // Create edge
    let mut edge = LogicEdgeV1::new(&edge_id, edge_type, from_uri.clone(), to_uri.clone(), "cli");
    if let Some(w) = args.weight {
        edge = edge.with_weight(w);
    }
    if let Some(spec_id) = &args.spec_id {
        edge = edge.with_spec_id(spec_id);
    }
    if let Some(run_id) = &args.run_id {
        edge = edge.with_run_id(run_id);
    }

    // Open capsule and store
    let config = CapsuleConfig {
        capsule_path: capsule_path.clone(),
        workspace_id: "default".to_string(),
        ..Default::default()
    };

    let handle = match CapsuleHandle::open(config) {
        Ok(h) => h,
        Err(e) => {
            if args.json {
                let json = serde_json::json!({
                    "schema_version": SCHEMA_VERSION,
                    "tool_version": tool_version(),
                    "error": format!("Failed to open capsule: {}", e),
                });
                println!("{}", serde_json::to_string_pretty(&json)?);
            } else {
                eprintln!("Error: Failed to open capsule: {}", e);
            }
            std::process::exit(3);
        }
    };

    // Store edge using put()
    let spec_id = args.spec_id.as_deref().unwrap_or("_global");
    let run_id = args.run_id.as_deref().unwrap_or("_manual");
    let data = edge.to_bytes()?;
    let metadata = serde_json::json!({
        "edge_type": edge_type.as_str(),
        "from_uri": from_uri.as_str(),
        "to_uri": to_uri.as_str(),
    });

    let edge_uri = handle.put(spec_id, run_id, ObjectType::Edge, &edge_id, data, metadata)?;

    // Commit immediately
    handle.commit_manual(&format!("edge:{}", edge_id))?;

    // Output result
    if args.json {
        let json = serde_json::json!({
            "schema_version": SCHEMA_VERSION,
            "tool_version": tool_version(),
            "edge_id": edge_id,
            "uri": edge_uri.as_str(),
            "edge_type": edge_type.as_str(),
            "from_uri": from_uri.as_str(),
            "to_uri": to_uri.as_str(),
            "weight": args.weight,
            "created": true,
        });
        println!("{}", serde_json::to_string_pretty(&json)?);
    } else {
        println!("Created edge: {}", edge_id);
        println!("  URI: {}", edge_uri.as_str());
        println!("  Type: {}", edge_type.as_str());
        println!("  From: {}", from_uri.as_str());
        println!("  To: {}", to_uri.as_str());
        if let Some(w) = args.weight {
            println!("  Weight: {}", w);
        }
    }

    Ok(())
}

/// Run `graph query` command
fn run_graph_query(cwd: &PathBuf, capsule_path: &PathBuf, args: GraphQueryArgs) -> anyhow::Result<()> {
    // Use capsule_path from args if provided, otherwise use the default
    let actual_capsule_path = args
        .capsule_path
        .as_ref()
        .unwrap_or(capsule_path);

    // Open capsule read-only
    let config = CapsuleConfig {
        capsule_path: actual_capsule_path.clone(),
        workspace_id: "default".to_string(),
        ..Default::default()
    };

    let handle = match CapsuleHandle::open_read_only(config) {
        Ok(h) => h,
        Err(e) => {
            if args.json {
                let json = serde_json::json!({
                    "schema_version": SCHEMA_VERSION,
                    "tool_version": tool_version(),
                    "error": format!("Failed to open capsule: {}", e),
                });
                println!("{}", serde_json::to_string_pretty(&json)?);
            } else {
                eprintln!("Error: Failed to open capsule: {}", e);
            }
            std::process::exit(3);
        }
    };

    // Handle different query modes
    if let Some(uri_str) = &args.uri {
        // Lookup by URI
        let uri: LogicalUri = match uri_str.parse() {
            Ok(u) => u,
            Err(_) => {
                if args.json {
                    let json = serde_json::json!({
                        "schema_version": SCHEMA_VERSION,
                        "tool_version": tool_version(),
                        "error": "Invalid URI format",
                        "uri": uri_str,
                    });
                    println!("{}", serde_json::to_string_pretty(&json)?);
                } else {
                    eprintln!("Error: Invalid URI format: {}", uri_str);
                }
                std::process::exit(2);
            }
        };

        match handle.get_bytes(&uri, None, None) {
            Ok(bytes) => {
                let obj_type = uri.object_type();
                if args.json {
                    // Try to parse as JSON for structured output
                    let payload: serde_json::Value = serde_json::from_slice(&bytes)
                        .unwrap_or_else(|_| serde_json::Value::String(String::from_utf8_lossy(&bytes).to_string()));
                    let json = serde_json::json!({
                        "schema_version": SCHEMA_VERSION,
                        "tool_version": tool_version(),
                        "uri": uri.as_str(),
                        "object_type": obj_type.map(|t| t.as_str()),
                        "size_bytes": bytes.len(),
                        "payload": payload,
                    });
                    println!("{}", serde_json::to_string_pretty(&json)?);
                } else {
                    println!("URI: {}", uri.as_str());
                    println!("Type: {:?}", obj_type);
                    println!("Size: {} bytes", bytes.len());
                    println!("---");
                    // Try to print as JSON, otherwise as text
                    if let Ok(parsed) = serde_json::from_slice::<serde_json::Value>(&bytes) {
                        println!("{}", serde_json::to_string_pretty(&parsed)?);
                    } else {
                        println!("{}", String::from_utf8_lossy(&bytes));
                    }
                }
            }
            Err(e) => {
                if args.json {
                    let json = serde_json::json!({
                        "schema_version": SCHEMA_VERSION,
                        "tool_version": tool_version(),
                        "error": format!("URI not found: {}", e),
                        "uri": uri.as_str(),
                    });
                    println!("{}", serde_json::to_string_pretty(&json)?);
                } else {
                    eprintln!("Error: URI not found: {}", uri.as_str());
                }
                std::process::exit(1);
            }
        }
    } else if args.adjacency.is_some() {
        // Adjacency query - TODO: implement when we have edge index
        if args.json {
            let json = serde_json::json!({
                "schema_version": SCHEMA_VERSION,
                "tool_version": tool_version(),
                "error": "Adjacency query not yet implemented",
            });
            println!("{}", serde_json::to_string_pretty(&json)?);
        } else {
            println!("Adjacency query not yet implemented");
            println!("Use --uri to lookup specific URIs, or --type to list by type.");
        }
    } else if args.object_type.is_some() || args.card_type.is_some() || args.edge_type.is_some() {
        // Type-based listing - TODO: implement when we have type index
        if args.json {
            let json = serde_json::json!({
                "schema_version": SCHEMA_VERSION,
                "tool_version": tool_version(),
                "error": "Type-based listing not yet implemented",
            });
            println!("{}", serde_json::to_string_pretty(&json)?);
        } else {
            println!("Type-based listing not yet implemented");
            println!("Use --uri to lookup specific URIs.");
        }
    } else {
        // No query specified
        if args.json {
            let json = serde_json::json!({
                "schema_version": SCHEMA_VERSION,
                "tool_version": tool_version(),
                "error": "No query specified. Use --uri, --type, or --adjacency.",
            });
            println!("{}", serde_json::to_string_pretty(&json)?);
        } else {
            println!("No query specified.");
            println!("Usage:");
            println!("  --uri <URI>        Lookup by specific mv2:// URI");
            println!("  --type card|edge   List by object type");
            println!("  --adjacency <URI>  Find edges connected to URI");
        }
    }

    Ok(())
}
