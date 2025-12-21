//! Spec-Kit Executor — Shared Application Core
//!
//! SPEC-KIT-921: CLI Adapter + Shared SpeckitExecutor Core
//!
//! This module provides the "thin waist" between UI surfaces (TUI, CLI) and
//! spec-kit business logic. Both adapters call `SpeckitExecutor::execute()`,
//! ensuring command parity and preventing logic duplication.
//!
//! ## Design Principles
//!
//! - **No UI types in core**: Executor returns domain types, never ratatui/CLI types
//! - **Single entrypoint**: All commands flow through `execute()`
//! - **Adapters own rendering**: TUI/CLI render domain results into their format
//!
//! ## Canonical Packet Contract (SPEC-KIT-921 P7)
//!
//! Each stage produces a specific artifact that becomes input to the next stage.
//! This is the artifact dependency DAG - the single source of truth for prereqs.
//!
//! | Stage     | Input Required       | Output Created     |
//! |-----------|---------------------|-------------------|
//! | Specify   | (none)              | PRD.md            |
//! | Plan      | PRD.md              | plan.md           |
//! | Tasks     | plan.md             | tasks.md          |
//! | Implement | tasks.md            | implement.md      |
//! | Validate  | implement.md        | validate.md       |
//! | Audit     | validate.md         | audit.md          |
//! | Unlock    | audit.md            | (approval)        |
//!
//! ## Prerequisite Matrix
//!
//! With `--strict-prereqs`, missing required prereqs become blocking errors (exit 2).
//! Without it, missing prereqs generate advisory warnings but don't block.
//!
//! | Stage     | Required (blocks if missing)         | Recommended (warns) |
//! |-----------|--------------------------------------|---------------------|
//! | Specify   | (none - first stage)                 | -                   |
//! | Plan      | PRD.md exists                        | -                   |
//! | Tasks     | plan.md exists                       | -                   |
//! | Implement | tasks.md exists                      | plan.md exists      |
//! | Validate  | implement.md exists                  | tasks.md exists     |
//! | Audit     | validate.md exists                   | implement.md exists |
//! | Unlock    | audit.md exists                      | validate.md exists  |
//!
//! ## Phase B Scope
//!
//! - Status command (read-only, pure)
//! - Review command (after status proves the pattern)

mod command;
pub mod review;
pub mod status;

pub use command::SpeckitCommand;
pub use review::{
    ArtifactSource, CheckpointArtifactRequirements, DisplayVerdict, EvidenceRefs, PolicySnapshot,
    ReviewOptions, ReviewRequest, ReviewResolution, ReviewSignal, SignalOrigin, SkipReason,
    StageReviewResult, TelemetryMode, checkpoint_artifact_requirements, is_canonical_review_point,
    is_diagnostic_review, resolve_review_request,
};
pub use status::{
    AgentCoverage, AgentOutcome, AgentStatus, EvidenceEntry, EvidenceMetrics, EvidenceThreshold,
    GuardrailRecord, PacketStatus, ScenarioStatus, SpecStatusArgs, SpecStatusReport,
    StageConsensus, StageCue, StageKind, StageSnapshot, TrackerRow,
};

use std::path::{Path, PathBuf};

// =============================================================================
// Spec ID Validation & Directory Resolution (SPEC-KIT-921 P7)
// =============================================================================

/// Error returned when spec ID validation fails
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SpecIdError {
    /// Spec ID is empty
    Empty,
    /// Spec ID contains path traversal characters
    PathTraversal,
    /// Spec ID doesn't match naming convention
    InvalidFormat(String),
}

impl std::fmt::Display for SpecIdError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SpecIdError::Empty => write!(f, "SPEC-ID is required"),
            SpecIdError::PathTraversal => {
                write!(f, "SPEC-ID contains invalid path characters (/, \\, or ..)")
            }
            SpecIdError::InvalidFormat(id) => {
                write!(f, "SPEC-ID '{}' doesn't match expected format SPEC-XXX-nnn", id)
            }
        }
    }
}

impl std::error::Error for SpecIdError {}

/// Validate a spec ID for safety and format
///
/// Rejects:
/// - Empty IDs
/// - Path traversal attempts (/, \, ..)
/// - IDs that don't match SPEC-* naming convention
///
/// Returns Ok(()) if valid, Err(SpecIdError) otherwise.
pub fn validate_spec_id(spec_id: &str) -> Result<(), SpecIdError> {
    if spec_id.is_empty() {
        return Err(SpecIdError::Empty);
    }

    // Path traversal protection
    if spec_id.contains('/') || spec_id.contains('\\') || spec_id.contains("..") {
        return Err(SpecIdError::PathTraversal);
    }

    // Naming convention: must start with SPEC- (case-insensitive)
    // Allow: SPEC-KIT-921, SPEC-001, SPEC-TEST-001, etc.
    let upper = spec_id.to_ascii_uppercase();
    if !upper.starts_with("SPEC-") {
        return Err(SpecIdError::InvalidFormat(spec_id.to_string()));
    }

    Ok(())
}

/// Result of resolving a spec directory
#[derive(Debug, Clone)]
pub struct ResolvedSpecDir {
    /// The resolved directory path
    pub path: PathBuf,
    /// Whether this was an exact match or prefix match
    pub exact_match: bool,
    /// The actual directory name (may differ from spec_id if suffix exists)
    pub dir_name: String,
}

/// Resolve spec directory with deterministic matching
///
/// Resolution order:
/// 1. Exact match: `docs/<SPEC-ID>`
/// 2. Prefix match: `docs/<SPEC-ID>-*` (sorted lexicographically, first match wins)
///
/// Returns None if no matching directory exists.
///
/// This function is the canonical resolver - all commands should use it.
pub fn resolve_spec_dir(repo_root: &Path, spec_id: &str) -> Option<ResolvedSpecDir> {
    let docs_dir = repo_root.join("docs");
    if !docs_dir.exists() {
        return None;
    }

    // 1. Try exact match first
    let exact_path = docs_dir.join(spec_id);
    if exact_path.is_dir() {
        return Some(ResolvedSpecDir {
            path: exact_path,
            exact_match: true,
            dir_name: spec_id.to_string(),
        });
    }

    // 2. Try prefix match (SPEC-ID-*)
    let prefix = format!("{}-", spec_id.to_ascii_uppercase());
    let mut matches: Vec<_> = std::fs::read_dir(&docs_dir)
        .ok()?
        .filter_map(Result::ok)
        .filter(|e| {
            e.file_type().map(|ft| ft.is_dir()).unwrap_or(false)
                && e.file_name()
                    .to_string_lossy()
                    .to_ascii_uppercase()
                    .starts_with(&prefix)
        })
        .collect();

    // Sort lexicographically for determinism
    matches.sort_by(|a, b| a.file_name().cmp(&b.file_name()));

    matches.first().map(|entry| ResolvedSpecDir {
        path: entry.path(),
        exact_match: false,
        dir_name: entry.file_name().to_string_lossy().to_string(),
    })
}

/// Get the default path for creating a new spec directory
///
/// Always returns the exact path `docs/<SPEC-ID>` without suffix.
/// Used by `speckit specify` when creating new directories.
pub fn default_spec_dir_for_creation(repo_root: &Path, spec_id: &str) -> PathBuf {
    repo_root.join("docs").join(spec_id)
}

// =============================================================================

/// Resolution of a stage validation
///
/// SPEC-KIT-921 P4-B: Generic stage resolution for all stages.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StageResolution {
    /// Validation passed, ready to proceed with agent execution
    Ready,
    /// Validation failed, needs intervention before proceeding
    Blocked,
    /// Stage not applicable (e.g., specify has no validation)
    Skipped,
}

impl StageResolution {
    /// Returns true if this is a blocking resolution
    pub fn is_blocked(&self) -> bool {
        matches!(self, StageResolution::Blocked)
    }

    /// Returns true if this is ready to proceed
    pub fn is_ready(&self) -> bool {
        matches!(self, StageResolution::Ready)
    }
}

/// Outcome from a stage validation command (plan, tasks, implement, etc.)
///
/// SPEC-KIT-921 P4-B: Replaces PlanReady/PlanBlocked with generic envelope.
/// TUI adapter should spawn agents when resolution is Ready.
/// CLI with --dry-run reports outcome and exits.
#[derive(Debug)]
pub struct StageOutcome {
    /// The validated SPEC identifier
    pub spec_id: String,
    /// Stage that was validated
    pub stage: crate::Stage,
    /// The resolution: Ready, Blocked, or Skipped
    pub resolution: StageResolution,
    /// Blocking reasons (errors that prevent proceeding)
    pub blocking_reasons: Vec<String>,
    /// Advisory signals (warnings that don't prevent proceeding)
    pub advisory_signals: Vec<String>,
    /// Optional evidence references (for stages that generate evidence)
    pub evidence_refs: Option<EvidenceRefs>,
    /// Whether this was a dry-run (validation only)
    pub dry_run: bool,
}

impl StageOutcome {
    /// Create a Ready outcome
    pub fn ready(spec_id: String, stage: crate::Stage, dry_run: bool) -> Self {
        Self {
            spec_id,
            stage,
            resolution: StageResolution::Ready,
            blocking_reasons: Vec::new(),
            advisory_signals: Vec::new(),
            evidence_refs: None,
            dry_run,
        }
    }

    /// Create a Ready outcome with warnings
    pub fn ready_with_warnings(
        spec_id: String,
        stage: crate::Stage,
        warnings: Vec<String>,
        dry_run: bool,
    ) -> Self {
        Self {
            spec_id,
            stage,
            resolution: StageResolution::Ready,
            blocking_reasons: Vec::new(),
            advisory_signals: warnings,
            evidence_refs: None,
            dry_run,
        }
    }

    /// Create a Blocked outcome
    ///
    /// SPEC-KIT-921 P4: Preserves dry_run for metadata consistency
    pub fn blocked(
        spec_id: String,
        stage: crate::Stage,
        errors: Vec<String>,
        dry_run: bool,
    ) -> Self {
        Self {
            spec_id,
            stage,
            resolution: StageResolution::Blocked,
            blocking_reasons: errors,
            advisory_signals: Vec::new(),
            evidence_refs: None,
            dry_run,
        }
    }

    /// Create a Skipped outcome
    ///
    /// SPEC-KIT-921 P4: Preserves dry_run for metadata consistency
    pub fn skipped(spec_id: String, stage: crate::Stage, reason: &str, dry_run: bool) -> Self {
        Self {
            spec_id,
            stage,
            resolution: StageResolution::Skipped,
            blocking_reasons: Vec::new(),
            advisory_signals: vec![reason.to_string()],
            evidence_refs: None,
            dry_run,
        }
    }

    /// Exit code for CLI: 0 for Ready/Skipped, 2 for Blocked
    pub fn exit_code(&self) -> i32 {
        match self.resolution {
            StageResolution::Ready | StageResolution::Skipped => 0,
            StageResolution::Blocked => 2,
        }
    }
}

/// Execution outcome from the executor
#[derive(Debug)]
#[allow(clippy::large_enum_variant)] // Acceptable: Status/Review are both large, boxing adds complexity
pub enum Outcome {
    /// Status command completed successfully
    Status(SpecStatusReport),

    /// Review command completed successfully
    Review(StageReviewResult),

    /// Review skipped (special case, not an error)
    ReviewSkipped {
        stage: crate::Stage,
        reason: SkipReason,
        suggestion: Option<&'static str>,
    },

    /// Stage validation outcome (plan, tasks, implement, etc.)
    ///
    /// SPEC-KIT-921 P4-B: Generic envelope for all stage validation commands.
    /// Replaces PlanReady/PlanBlocked.
    Stage(StageOutcome),

    /// Specify command outcome
    ///
    /// SPEC-KIT-921 P6-A: Specify creates SPEC directory structure.
    Specify(SpecifyOutcome),

    /// Command failed with error
    Error(String),
}

/// Outcome from the specify command (create SPEC directory)
///
/// SPEC-KIT-921 P6-A: Specify is a creation command, not a validation.
#[derive(Debug)]
pub struct SpecifyOutcome {
    /// The created SPEC identifier
    pub spec_id: String,
    /// Whether this was a dry-run (validation only)
    pub dry_run: bool,
    /// Path to the created SPEC directory (relative to repo root)
    pub spec_dir: String,
    /// Whether the directory already existed
    pub already_existed: bool,
    /// Created files (if any)
    pub created_files: Vec<String>,
}

/// Execution context provided by the adapter
///
/// Adapters are responsible for resolving all env/config values before
/// constructing this context. The executor should not perform any I/O.
#[derive(Debug, Clone)]
pub struct ExecutionContext {
    /// Repository root path
    pub repo_root: std::path::PathBuf,

    /// Policy snapshot (resolved by adapter from env/config)
    ///
    /// If None, defaults are used (all features disabled).
    /// Adapters should use `PolicyToggles::from_env_and_config()` to resolve.
    pub policy_snapshot: Option<PolicySnapshot>,
}

/// Spec-Kit executor — the single entrypoint for all commands
///
/// Both TUI and CLI adapters call this executor. The executor returns
/// domain types; adapters handle rendering.
pub struct SpeckitExecutor {
    context: ExecutionContext,
}

impl SpeckitExecutor {
    /// Create a new executor with the given context
    pub fn new(context: ExecutionContext) -> Self {
        Self { context }
    }

    /// Execute a command and return the outcome
    ///
    /// This is the single entrypoint for all spec-kit commands.
    /// Adapters (TUI/CLI) call this method and render the result.
    pub fn execute(&self, command: SpeckitCommand) -> Outcome {
        match command {
            SpeckitCommand::Status {
                spec_id,
                stale_hours,
            } => self.execute_status(&spec_id, stale_hours),
            SpeckitCommand::Review {
                spec_id,
                stage,
                strict_artifacts,
                strict_warnings,
                strict_schema,
                evidence_root,
            } => self.execute_review(
                &spec_id,
                stage,
                strict_artifacts,
                strict_warnings,
                strict_schema,
                evidence_root,
            ),
            SpeckitCommand::ValidateStage {
                spec_id,
                stage,
                dry_run,
                strict_prereqs,
            } => self.execute_validate_stage(&spec_id, stage, dry_run, strict_prereqs),
            SpeckitCommand::Specify { spec_id, dry_run } => {
                self.execute_specify(&spec_id, dry_run)
            }
        }
    }

    /// Execute status command
    fn execute_status(&self, spec_id: &str, stale_hours: i64) -> Outcome {
        let args = SpecStatusArgs {
            spec_id: spec_id.to_string(),
            stale_hours,
        };

        match status::collect_report(&self.context.repo_root, args) {
            Ok(report) => Outcome::Status(report),
            Err(e) => Outcome::Error(e.to_string()),
        }
    }

    /// Execute review command
    ///
    /// This maps the stage to a checkpoint using `resolve_review_request()`,
    /// then evaluates the gate artifacts and returns a `StageReviewResult`.
    fn execute_review(
        &self,
        spec_id: &str,
        stage: crate::Stage,
        strict_artifacts: bool,
        strict_warnings: bool,
        strict_schema: bool,
        evidence_root: Option<std::path::PathBuf>,
    ) -> Outcome {
        // Resolve stage → checkpoint using canonical mapping
        let resolution = review::resolve_review_request(stage);

        match resolution {
            ReviewResolution::NotApplicable { reason, suggestion } => Outcome::ReviewSkipped {
                stage,
                reason,
                suggestion,
            },
            ReviewResolution::Alias {
                actual_checkpoint,
                message: _,
            }
            | ReviewResolution::Review {
                checkpoint: actual_checkpoint,
            } => {
                // Use policy snapshot from context (adapter-resolved) or defaults
                let policy_snapshot = self.context.policy_snapshot.clone().unwrap_or_default();

                let options = ReviewOptions {
                    telemetry_mode: policy_snapshot.telemetry_mode,
                    include_diagnostic: review::is_diagnostic_review(stage),
                    strict_artifacts,
                    strict_warnings,
                    strict_schema,
                    evidence_root,
                    policy_snapshot,
                };

                let request = ReviewRequest {
                    repo_root: self.context.repo_root.clone(),
                    spec_id: spec_id.to_string(),
                    stage,
                    options,
                };

                match review::evaluate_stage_review(request, actual_checkpoint) {
                    Ok(result) => Outcome::Review(result),
                    Err(e) => Outcome::Error(e),
                }
            }
        }
    }

    /// Execute stage validation command (validate prerequisites and guardrails)
    ///
    /// SPEC-KIT-921 P4: Stage-neutral validation for any stage.
    /// SPEC-KIT-921 P6-C: Added strict_prereqs parameter.
    ///
    /// The adapter (TUI) handles agent spawning when resolution is Ready.
    /// CLI with --dry-run reports outcome and exits.
    fn execute_validate_stage(
        &self,
        spec_id: &str,
        stage: crate::Stage,
        dry_run: bool,
        strict_prereqs: bool,
    ) -> Outcome {
        let mut warnings: Vec<String> = Vec::new();
        let mut errors: Vec<String> = Vec::new();

        // 1. Validate SPEC-ID format and safety
        if let Err(e) = validate_spec_id(spec_id) {
            errors.push(e.to_string());
            return Outcome::Stage(StageOutcome::blocked(
                spec_id.to_string(),
                stage,
                errors,
                dry_run,
            ));
        }

        // 2. Resolve SPEC directory using canonical resolver
        // Supports both exact match (SPEC-ID) and prefix match (SPEC-ID-suffix)
        let resolved = resolve_spec_dir(&self.context.repo_root, spec_id);
        let spec_dir = match &resolved {
            Some(r) => {
                if !r.exact_match {
                    // Inform user that we matched a suffixed directory
                    warnings.push(format!(
                        "Resolved {} to directory: {}",
                        spec_id, r.dir_name
                    ));
                }
                r.path.clone()
            }
            None => {
                // Directory doesn't exist - use default path for prereq checking
                // (prereq check will fail if directory is required)
                default_spec_dir_for_creation(&self.context.repo_root, spec_id)
            }
        };

        // 3. Check prerequisites using centralized matrix
        // See module-level docs for the canonical packet contract
        let (required_missing, recommended_missing) =
            check_stage_prereqs(&spec_dir, spec_id, stage);

        // P6-C: With --strict-prereqs, missing REQUIRED prereqs become blocking errors
        if strict_prereqs && !required_missing.is_empty() {
            errors.extend(
                required_missing
                    .iter()
                    .map(|w| format!("[strict-prereqs] {w}")),
            );
            return Outcome::Stage(StageOutcome::blocked(
                spec_id.to_string(),
                stage,
                errors,
                dry_run,
            ));
        }

        // Required prereqs generate warnings (advisory by default)
        warnings.extend(required_missing);

        // Recommended prereqs generate info-level warnings (never block, even with --strict-prereqs)
        for rec in recommended_missing {
            warnings.push(format!("[recommended] {rec}"));
        }

        // If there are errors, return Blocked
        if !errors.is_empty() {
            return Outcome::Stage(StageOutcome::blocked(
                spec_id.to_string(),
                stage,
                errors,
                dry_run,
            ));
        }

        // Validation passed
        if warnings.is_empty() {
            Outcome::Stage(StageOutcome::ready(spec_id.to_string(), stage, dry_run))
        } else {
            Outcome::Stage(StageOutcome::ready_with_warnings(
                spec_id.to_string(),
                stage,
                warnings,
                dry_run,
            ))
        }
    }

    /// Execute specify command (create SPEC directory structure)
    ///
    /// SPEC-KIT-921 P6-A: Minimal specify implementation.
    /// SPEC-KIT-921 P7: Uses canonical spec ID validation and directory resolution.
    ///
    /// Creates docs/<SPEC-ID>/ directory with PRD.md (the canonical input artifact).
    /// Idempotent: never overwrites existing PRD.md content.
    fn execute_specify(&self, spec_id: &str, dry_run: bool) -> Outcome {
        // Validate SPEC-ID format and safety
        if let Err(e) = validate_spec_id(spec_id) {
            return Outcome::Error(e.to_string());
        }

        // Check if a matching directory already exists (exact or suffixed)
        let existing = resolve_spec_dir(&self.context.repo_root, spec_id);

        // Determine spec directory path (use existing if found, otherwise create exact)
        let (spec_dir, spec_dir_relative) = match &existing {
            Some(resolved) => (resolved.path.clone(), format!("docs/{}", resolved.dir_name)),
            None => {
                let path = default_spec_dir_for_creation(&self.context.repo_root, spec_id);
                (path, format!("docs/{spec_id}"))
            }
        };
        let already_existed = spec_dir.exists();

        let mut created_files = Vec::new();

        // In dry-run mode, just report what would happen
        if dry_run {
            return Outcome::Specify(SpecifyOutcome {
                spec_id: spec_id.to_string(),
                dry_run: true,
                spec_dir: spec_dir_relative,
                already_existed,
                created_files,
            });
        }

        // Create directory if it doesn't exist
        if !already_existed {
            if let Err(e) = std::fs::create_dir_all(&spec_dir) {
                return Outcome::Error(format!("Failed to create SPEC directory: {e}"));
            }
        }

        // Create minimal PRD.md if it doesn't exist
        let prd_path = spec_dir.join("PRD.md");
        if !prd_path.exists() {
            let prd_content = format!(
                "# {spec_id}\n\n\
                 ## Overview\n\n\
                 <!-- Brief description of what this SPEC aims to accomplish -->\n\n\
                 ## Requirements\n\n\
                 <!-- Key requirements and acceptance criteria -->\n\n\
                 ## Non-Goals\n\n\
                 <!-- What is explicitly out of scope -->\n"
            );

            if let Err(e) = std::fs::write(&prd_path, prd_content) {
                return Outcome::Error(format!("Failed to create PRD.md: {e}"));
            }
            created_files.push("PRD.md".to_string());
        }

        Outcome::Specify(SpecifyOutcome {
            spec_id: spec_id.to_string(),
            dry_run: false,
            spec_dir: spec_dir_relative,
            already_existed,
            created_files,
        })
    }
}

/// Render a status report as text lines (for TUI/CLI display)
///
/// This is a pure formatting function — no side effects.
pub fn render_status_dashboard(report: &SpecStatusReport) -> Vec<String> {
    status::render_dashboard(report)
}

/// Get degraded warning message if any issues detected
pub fn status_degraded_warning(report: &SpecStatusReport) -> Option<String> {
    status::degraded_warning(report)
}

/// Render a review result as text lines (for TUI/CLI display)
///
/// This is a pure formatting function — no side effects.
pub fn render_review_dashboard(result: &StageReviewResult) -> Vec<String> {
    review::render_review(result)
}

/// Get review warning message if escalation needed
pub fn review_warning(result: &StageReviewResult) -> Option<String> {
    if !result.is_auto_apply() {
        Some(format!(
            "⚠ Stage {:?} requires human review: {}",
            result.stage,
            match &result.resolution {
                crate::Verdict::Escalate { reason, .. } => reason.as_str(),
                _ => "Unknown reason",
            }
        ))
    } else if !result.advisory_signals.is_empty() {
        Some(format!(
            "⚠ Stage {:?} passed with {} advisory warning(s)",
            result.stage,
            result.advisory_signals.len()
        ))
    } else {
        None
    }
}

/// Check stage prerequisites against the canonical packet contract
///
/// Returns (required_missing, recommended_missing) where:
/// - required_missing: prereqs that block with --strict-prereqs
/// - recommended_missing: prereqs that warn but never block
///
/// SPEC-KIT-921 P7: Aligned with artifact dependency DAG.
/// See module-level docs for the canonical packet contract.
fn check_stage_prereqs(
    spec_dir: &std::path::Path,
    spec_id: &str,
    stage: crate::Stage,
) -> (Vec<String>, Vec<String>) {
    let mut required_missing = Vec::new();
    let mut recommended_missing = Vec::new();

    match stage {
        crate::Stage::Specify => {
            // First stage - no prerequisites
        }
        crate::Stage::Plan => {
            // Required: PRD.md exists (output of Specify)
            if !spec_dir.join("PRD.md").exists() {
                required_missing.push(format!(
                    "PRD.md not found for {spec_id} - run speckit specify first"
                ));
            }
        }
        crate::Stage::Tasks => {
            // Required: plan.md exists (output of Plan)
            if !spec_dir.join("plan.md").exists() {
                required_missing.push(format!(
                    "plan.md not found for {spec_id} - run /speckit.plan first"
                ));
            }
        }
        crate::Stage::Implement => {
            // Required: tasks.md exists (output of Tasks)
            if !spec_dir.join("tasks.md").exists() {
                required_missing.push(format!(
                    "tasks.md not found for {spec_id} - run /speckit.tasks first"
                ));
            }
            // Recommended: plan.md exists (for context)
            if spec_dir.exists() && !spec_dir.join("plan.md").exists() {
                recommended_missing.push(format!(
                    "plan.md not found for {spec_id} - consider running /speckit.plan first"
                ));
            }
        }
        crate::Stage::Validate => {
            // Required: implement.md exists (output of Implement)
            if !spec_dir.join("implement.md").exists() {
                required_missing.push(format!(
                    "implement.md not found for {spec_id} - run /speckit.implement first"
                ));
            }
            // Recommended: tasks.md exists (for test mapping)
            if spec_dir.exists() && !spec_dir.join("tasks.md").exists() {
                recommended_missing.push(format!(
                    "tasks.md not found for {spec_id} - consider running /speckit.tasks first"
                ));
            }
        }
        crate::Stage::Audit => {
            // Required: validate.md exists (output of Validate)
            if !spec_dir.join("validate.md").exists() {
                required_missing.push(format!(
                    "validate.md not found for {spec_id} - run /speckit.validate first"
                ));
            }
            // Recommended: implement.md exists (for audit context)
            if spec_dir.exists() && !spec_dir.join("implement.md").exists() {
                recommended_missing.push(format!(
                    "implement.md not found for {spec_id} - consider running /speckit.implement first"
                ));
            }
        }
        crate::Stage::Unlock => {
            // Required: audit.md exists (output of Audit)
            if !spec_dir.join("audit.md").exists() {
                required_missing.push(format!(
                    "audit.md not found for {spec_id} - run /speckit.audit first"
                ));
            }
            // Recommended: validate.md exists (for final check)
            if spec_dir.exists() && !spec_dir.join("validate.md").exists() {
                recommended_missing.push(format!(
                    "validate.md not found for {spec_id} - consider running /speckit.validate first"
                ));
            }
        }
    }

    (required_missing, recommended_missing)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Parity test: Slash command and CLI should parse to same SpeckitCommand
    #[test]
    fn test_command_parity_status() {
        // Simulate slash command parsing: "/speckit.status SPEC-123"
        let slash_args = "SPEC-123";
        let slash_cmd = SpeckitCommand::Status {
            spec_id: slash_args.to_string(),
            stale_hours: 24, // default
        };

        // Simulate CLI parsing: "code speckit status --spec SPEC-123"
        let cli_cmd = SpeckitCommand::Status {
            spec_id: "SPEC-123".to_string(),
            stale_hours: 24,
        };

        // Both should produce equivalent commands
        assert_eq!(slash_cmd, cli_cmd);
    }

    #[test]
    fn test_command_parity_status_with_stale_hours() {
        // Slash: "/speckit.status SPEC-456 --stale-hours 48"
        let slash_cmd = SpeckitCommand::Status {
            spec_id: "SPEC-456".to_string(),
            stale_hours: 48,
        };

        // CLI: "code speckit status --spec SPEC-456 --stale-hours 48"
        let cli_cmd = SpeckitCommand::Status {
            spec_id: "SPEC-456".to_string(),
            stale_hours: 48,
        };

        assert_eq!(slash_cmd, cli_cmd);
    }

    // === Review command parity tests ===

    #[test]
    fn test_command_parity_review_basic() {
        // Slash: "/review plan" (with spec_id from context)
        let slash_cmd = SpeckitCommand::parse_review("SPEC-123", "plan").unwrap();

        // CLI: "code speckit review --spec SPEC-123 --stage plan"
        let cli_cmd = SpeckitCommand::Review {
            spec_id: "SPEC-123".to_string(),
            stage: crate::Stage::Plan,
            strict_artifacts: false,
            strict_warnings: false,
            strict_schema: false,
            evidence_root: None,
        };

        assert_eq!(slash_cmd, cli_cmd);
    }

    #[test]
    fn test_command_parity_review_with_options() {
        // Slash: "/review audit --strict-artifacts --strict-warnings"
        let slash_cmd =
            SpeckitCommand::parse_review("SPEC-456", "audit --strict-artifacts --strict-warnings")
                .unwrap();

        // CLI: "code speckit review --spec SPEC-456 --stage audit --strict-artifacts --strict-warnings"
        let cli_cmd = SpeckitCommand::Review {
            spec_id: "SPEC-456".to_string(),
            stage: crate::Stage::Audit,
            strict_artifacts: true,
            strict_warnings: true,
            strict_schema: false,
            evidence_root: None,
        };

        assert_eq!(slash_cmd, cli_cmd);
    }

    #[test]
    fn test_executor_review_dispatch_skipped() {
        // Test that Specify stage returns ReviewSkipped
        let executor = SpeckitExecutor::new(ExecutionContext {
            repo_root: std::path::PathBuf::from("/tmp/nonexistent"),
            policy_snapshot: None, // Use defaults
        });

        let cmd = SpeckitCommand::Review {
            spec_id: "TEST".to_string(),
            stage: crate::Stage::Specify,
            strict_artifacts: false,
            strict_warnings: false,
            strict_schema: false,
            evidence_root: None,
        };

        let outcome = executor.execute(cmd);
        assert!(matches!(
            outcome,
            Outcome::ReviewSkipped {
                reason: SkipReason::NoArtifactsFound,
                ..
            }
        ));
    }

    #[test]
    fn test_executor_review_dispatch_alias() {
        // Test that Unlock aliases to BeforeUnlock and doesn't skip
        let executor = SpeckitExecutor::new(ExecutionContext {
            repo_root: std::path::PathBuf::from("/tmp/nonexistent"),
            policy_snapshot: None, // Use defaults
        });

        let cmd = SpeckitCommand::Review {
            spec_id: "TEST".to_string(),
            stage: crate::Stage::Unlock,
            strict_artifacts: false,
            strict_warnings: false,
            strict_schema: false,
            evidence_root: None,
        };

        let outcome = executor.execute(cmd);
        // Should NOT be skipped — Unlock is an alias to BeforeUnlock
        assert!(
            matches!(outcome, Outcome::Review(_)),
            "Unlock should alias to BeforeUnlock, got: {outcome:?}"
        );
    }
}
