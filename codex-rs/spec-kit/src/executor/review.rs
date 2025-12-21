//! Stage Review Evaluation — Domain Types and Core Logic
//!
//! SPEC-KIT-921: Shared review evaluation for CLI/TUI parity.
//!
//! This module provides the pure domain types and evaluation logic for
//! stage review. No ratatui types. Adapters render the result.
//!
//! ## Architecture
//!
//! ```text
//! User: /review plan
//!        ↓
//! Adapter: resolve_review_request(Stage::Plan)
//!        ↓
//! Core: evaluate_stage_review(AfterPlan checkpoint)
//!        ↓
//! Result: StageReviewResult { resolution: Verdict, ... }
//!        ↓
//! Adapter: render_stage_review_tui(&result) → Vec<Line>
//! ```
//!
//! ## Evidence Topology (REVIEW-CONTRACT.md)
//!
//! - Spec packet: `docs/<SPEC-ID>/{spec.md, plan.md, tasks.md}`
//! - Review evidence: `evidence/consensus/<SPEC-ID>/spec-<stage>_*.json`
//! - Telemetry: `evidence/commands/<SPEC-ID>/*_telemetry_*.json`

use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};
use walkdir::WalkDir;

use crate::gate_policy::{
    Checkpoint, ConfidenceLevel, CounterSignalKind, EscalationTarget, Role, SignalSeverity, Stage,
    ToolTruthKind, Verdict,
};

// ============================================================================
// Path Constants (aligned with REVIEW-CONTRACT.md)
// ============================================================================

/// Root for spec packet docs
const SPEC_PACKET_ROOT: &str = "docs";

/// Root for evidence (consensus + commands)
const EVIDENCE_ROOT: &str = "docs/SPEC-OPS-004-integrated-coder-hooks/evidence";

/// Consensus evidence subdirectory (legacy name retained)
const CONSENSUS_DIR: &str = "consensus";

/// Commands telemetry subdirectory
#[allow(dead_code)]
const COMMANDS_DIR: &str = "commands";

// ============================================================================
// Stage → Checkpoint Mapping
// ============================================================================

/// Review request resolution
///
/// Maps user-facing stage to internal checkpoint or special case.
#[derive(Debug, Clone)]
pub enum ReviewResolution {
    /// Normal review at checkpoint
    Review { checkpoint: Checkpoint },

    /// Alias to another checkpoint (e.g., unlock → BeforeUnlock)
    Alias {
        actual_checkpoint: Checkpoint,
        message: &'static str,
    },

    /// Not applicable for this stage
    NotApplicable {
        reason: SkipReason,
        suggestion: Option<&'static str>,
    },
}

/// Resolve a review request for a stage
///
/// This is the canonical mapping from user-facing stage to checkpoint.
/// All adapters should use this function.
pub fn resolve_review_request(stage: Stage) -> ReviewResolution {
    match stage {
        // Canonical review points (run by default in /speckit.auto)
        Stage::Plan => ReviewResolution::Review {
            checkpoint: Checkpoint::AfterPlan,
        },
        Stage::Tasks => ReviewResolution::Review {
            checkpoint: Checkpoint::AfterTasks,
        },
        Stage::Audit => ReviewResolution::Review {
            checkpoint: Checkpoint::BeforeUnlock,
        },

        // Diagnostic reviews (opt-in, not default pipeline gates)
        Stage::Implement => ReviewResolution::Review {
            checkpoint: Checkpoint::AfterImplement,
        },
        Stage::Validate => ReviewResolution::Review {
            checkpoint: Checkpoint::AfterValidate,
        },

        // Special cases
        Stage::Specify => ReviewResolution::NotApplicable {
            reason: SkipReason::NoArtifactsFound,
            suggestion: Some(
                "Run `/speckit.status` to check spec packet, or review Plan after planning completes",
            ),
        },
        Stage::Unlock => ReviewResolution::Alias {
            actual_checkpoint: Checkpoint::BeforeUnlock,
            message: "Reviewing Audit output (final gate readiness)",
        },
    }
}

/// Check if a stage is a canonical review point (runs by default)
pub fn is_canonical_review_point(stage: Stage) -> bool {
    matches!(stage, Stage::Plan | Stage::Tasks | Stage::Audit)
}

/// Check if a stage is a diagnostic review (opt-in only)
pub fn is_diagnostic_review(stage: Stage) -> bool {
    matches!(stage, Stage::Implement | Stage::Validate)
}

// ============================================================================
// Review Result Types
// ============================================================================

/// Domain result from stage review evaluation
///
/// This is the core output type. No ratatui types. Adapters render this.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StageReviewResult {
    // === Identity ===
    /// SPEC identifier
    pub spec_id: String,

    /// Stage that was reviewed
    pub stage: Stage,

    /// Checkpoint where review ran
    pub checkpoint: Checkpoint,

    // === Gate Decision (source of truth for CLI/CI) ===
    /// The gate verdict: AutoApply or Escalate
    pub resolution: Verdict,

    // === Signals ===
    /// Blocking signals (forced escalation)
    /// Invariant: non-empty ⟹ resolution == Escalate
    pub blocking_signals: Vec<ReviewSignal>,

    /// Advisory signals (warnings, did not block)
    pub advisory_signals: Vec<ReviewSignal>,

    // === Artifact provenance ===
    /// Where artifacts came from
    pub artifact_sources: Vec<ArtifactSource>,

    /// Number of artifacts collected (spec docs + review evidence)
    pub artifacts_collected: usize,

    /// Number of review evidence files found (consensus files only)
    ///
    /// P0-3: This is what determines skip logic, not artifacts_collected.
    /// Review can only proceed if review evidence exists.
    pub review_evidence_count: usize,

    // === Evidence references (repo-relative paths) ===
    /// Paths to persisted evidence
    pub evidence: EvidenceRefs,

    // === Policy context snapshot ===
    /// Policy state at evaluation time
    pub policy_snapshot: PolicySnapshot,
}

impl StageReviewResult {
    /// Compute display verdict from resolution + signals
    ///
    /// NOT stored — derived on demand to prevent drift.
    pub fn display_verdict(&self) -> DisplayVerdict {
        // Skip conditions first - P0-3: use review_evidence_count, not artifacts_collected
        // Review requires review evidence (consensus files), not just spec docs
        if self.review_evidence_count == 0 {
            return DisplayVerdict::Skipped {
                reason: SkipReason::NoArtifactsFound,
            };
        }

        // Resolution-based
        match &self.resolution {
            Verdict::AutoApply { .. } => {
                if self.advisory_signals.is_empty() && self.blocking_signals.is_empty() {
                    DisplayVerdict::Passed
                } else {
                    DisplayVerdict::PassedWithWarnings
                }
            }
            Verdict::Escalate { .. } => DisplayVerdict::Failed,
        }
    }

    /// Map to CI exit code
    pub fn exit_code(&self, options: &ReviewOptions) -> i32 {
        match self.display_verdict() {
            DisplayVerdict::Passed => 0,
            DisplayVerdict::PassedWithWarnings => {
                if options.strict_warnings {
                    1
                } else {
                    0
                }
            }
            DisplayVerdict::Failed => 2,
            DisplayVerdict::Skipped { reason } => match reason {
                SkipReason::NoArtifactsFound if options.strict_artifacts => 2,
                _ => 0, // warn on stderr
            },
        }
    }

    /// Convenience: does this allow automatic progression?
    pub fn is_auto_apply(&self) -> bool {
        self.resolution.is_auto_apply()
    }
}

/// Display-oriented verdict (for UI rendering, not gate decisions)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DisplayVerdict {
    /// All checks passed, no signals
    Passed,

    /// Passed but with advisory signals
    PassedWithWarnings,

    /// Escalation required (maps to CONFLICT in UI)
    Failed,

    /// Review skipped (maps to DEGRADED in UI)
    Skipped { reason: SkipReason },
}

impl DisplayVerdict {
    /// User-facing label for the verdict
    pub fn label(&self) -> &'static str {
        match self {
            DisplayVerdict::Passed => "REVIEW OK",
            DisplayVerdict::PassedWithWarnings => "REVIEW DEGRADED",
            DisplayVerdict::Failed => "REVIEW CONFLICT",
            DisplayVerdict::Skipped { .. } => "REVIEW DEGRADED",
        }
    }
}

/// Reason why review was skipped
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SkipReason {
    /// No artifacts found for this stage
    NoArtifactsFound,

    /// Stage is not applicable for review
    NotApplicableToStage,

    /// Review explicitly disabled via policy
    PolicyDisabled,
}

// ============================================================================
// Signal Types
// ============================================================================

/// Individual signal from review
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReviewSignal {
    /// Signal classification
    pub kind: CounterSignalKind,

    /// Severity level
    pub severity: SignalSeverity,

    /// Where the signal originated
    pub origin: SignalOrigin,

    /// Which worker instance (if applicable)
    pub worker_id: Option<String>,

    /// Human-readable message
    pub message: String,

    /// Path to supporting evidence (repo-relative)
    pub evidence_path: Option<String>,
}

/// Signal source (structured, not free-form string)
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SignalOrigin {
    /// From a role (Architect, Implementer, etc.)
    Role(Role),

    /// From tool execution
    Tool(ToolTruthKind),

    /// Synthesized/merged signal
    Synthesis,

    /// System/infrastructure (timeout, connection, etc.)
    System,
}

impl SignalOrigin {
    /// Display name for the origin
    pub fn display_name(&self) -> String {
        match self {
            SignalOrigin::Role(role) => format!("{role:?}"),
            SignalOrigin::Tool(kind) => format!("Tool:{kind:?}"),
            SignalOrigin::Synthesis => "Synthesis".to_string(),
            SignalOrigin::System => "System".to_string(),
        }
    }
}

// ============================================================================
// Artifact and Evidence Types
// ============================================================================

/// Where artifacts came from
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ArtifactSource {
    /// SQLite consensus database
    SQLite,

    /// MCP local-memory
    Mcp,

    /// Filesystem fallback
    FilesystemFallback,

    /// No artifacts found
    None,
}

/// References to persisted evidence (repo-relative paths)
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct EvidenceRefs {
    /// Path to verdict JSON
    pub verdict_json: Option<String>,

    /// Path to telemetry bundle
    pub telemetry_bundle: Option<String>,

    /// Path to synthesis file
    pub synthesis_path: Option<String>,

    /// Evidence directory (for tooling)
    pub evidence_dir: Option<String>,
}

// ============================================================================
// Policy and Options
// ============================================================================

/// Policy state at evaluation time (no consensus vocabulary)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PolicySnapshot {
    /// Sidecar critic enabled
    pub sidecar_critic_enabled: bool,

    /// Telemetry mode
    pub telemetry_mode: TelemetryMode,

    /// Legacy voting env var detected (ignored, but recorded for audit)
    pub legacy_voting_env_detected: bool,
}

impl Default for PolicySnapshot {
    fn default() -> Self {
        Self {
            sidecar_critic_enabled: false,
            telemetry_mode: TelemetryMode::Disabled,
            legacy_voting_env_detected: false,
        }
    }
}

/// Telemetry mode
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum TelemetryMode {
    /// Telemetry enabled
    Enabled,

    /// Telemetry disabled
    #[default]
    Disabled,

    /// Dry run (compute but don't persist)
    DryRun,
}

/// Options for review evaluation
#[derive(Debug, Clone)]
pub struct ReviewOptions {
    /// Telemetry mode (resolved from config)
    pub telemetry_mode: TelemetryMode,

    /// Include diagnostic reviews (Implement/Validate)
    pub include_diagnostic: bool,

    /// Strict artifact mode: fail if expected artifacts missing
    pub strict_artifacts: bool,

    /// Strict warnings mode: exit 1 on PassedWithWarnings
    pub strict_warnings: bool,

    /// Strict schema mode: parse/schema failures → exit 3
    /// Prevents CI passing on corrupted evidence
    pub strict_schema: bool,

    /// P1-D: Override evidence root path (relative to repo_root)
    /// If None, uses default: docs/SPEC-OPS-004-integrated-coder-hooks/evidence
    pub evidence_root: Option<PathBuf>,

    /// P0-B: Policy snapshot (resolved by adapter, not read from env in core)
    pub policy_snapshot: PolicySnapshot,
}

impl Default for ReviewOptions {
    fn default() -> Self {
        Self {
            telemetry_mode: TelemetryMode::Disabled,
            include_diagnostic: false,
            strict_artifacts: false,
            strict_warnings: false,
            strict_schema: false,
            evidence_root: None,
            policy_snapshot: PolicySnapshot::default(),
        }
    }
}

/// Request for stage review evaluation
#[derive(Debug, Clone)]
pub struct ReviewRequest {
    /// Repository root
    pub repo_root: PathBuf,

    /// SPEC identifier
    pub spec_id: String,

    /// Stage to review
    pub stage: Stage,

    /// Review options
    pub options: ReviewOptions,
}

// ============================================================================
// Required Artifacts per Checkpoint
// ============================================================================

/// Artifact requirements for a checkpoint (for strict mode)
#[derive(Debug, Clone)]
pub struct CheckpointArtifactRequirements {
    /// Required artifact files
    pub required: Vec<&'static str>,

    /// Optional artifact files
    pub optional: Vec<&'static str>,
}

/// Get artifact requirements for a checkpoint
pub fn checkpoint_artifact_requirements(checkpoint: Checkpoint) -> CheckpointArtifactRequirements {
    match checkpoint {
        Checkpoint::AfterPlan => CheckpointArtifactRequirements {
            required: vec!["plan.md"],
            optional: vec!["analysis.json"],
        },
        Checkpoint::AfterTasks => CheckpointArtifactRequirements {
            required: vec!["tasks.md"],
            optional: vec!["decomposition.json"],
        },
        Checkpoint::AfterImplement => CheckpointArtifactRequirements {
            required: vec![], // Code changes tracked via git
            optional: vec!["implementation.json"],
        },
        Checkpoint::AfterValidate => CheckpointArtifactRequirements {
            required: vec![], // Test results from tool-truth
            optional: vec!["validation.json"],
        },
        Checkpoint::BeforeUnlock => CheckpointArtifactRequirements {
            required: vec!["audit.md"],
            optional: vec!["compliance.json"],
        },
        Checkpoint::BeforePlan => CheckpointArtifactRequirements {
            required: vec!["spec.md"],
            optional: vec!["context.json"],
        },
    }
}

// ============================================================================
// ConsensusJson Parsing Types (aligned with status.rs)
// ============================================================================

/// JSON structure for consensus evidence files
///
/// Matches: `evidence/consensus/<SPEC-ID>/spec-<stage>_*.json`
#[derive(Debug, Deserialize)]
struct ConsensusJson {
    /// Agent name (e.g., "claude", "architect")
    pub agent: Option<String>,
    /// Model identifier (retained for future enrichment)
    #[allow(dead_code)]
    pub model: Option<String>,
    /// Error message if agent failed
    pub error: Option<String>,
    /// Consensus details (conflicts, synthesis status)
    pub consensus: Option<ConsensusDetailJson>,
}

/// Consensus detail within a ConsensusJson file
#[derive(Debug, Deserialize)]
struct ConsensusDetailJson {
    /// List of conflict descriptions (blocking if non-empty)
    pub conflicts: Option<Vec<String>>,
    /// Synthesis status string
    #[allow(dead_code)]
    pub synthesis_status: Option<String>,
}

/// Infer agent role from filename or agent field
fn infer_agent_role(agent: Option<&str>, filename: &str) -> Role {
    let agent_str = agent.unwrap_or(filename);
    let lower = agent_str.to_lowercase();

    if lower.contains("architect") {
        Role::Architect
    } else if lower.contains("implement") {
        Role::Implementer
    } else if lower.contains("valid") {
        Role::Validator
    } else if lower.contains("judge") || lower.contains("audit") {
        Role::Judge
    } else if lower.contains("sidecar") || lower.contains("critic") {
        Role::SidecarCritic
    } else {
        // Default to architect for unknown agents
        Role::Architect
    }
}

// ============================================================================
// Core Evaluation Function
// ============================================================================

/// Evaluate stage review artifacts and produce a result
///
/// This is the core pure function that evaluates gate artifacts.
/// No ratatui types — adapters render the result.
///
/// **P0-A**: Reads from correct evidence topology (REVIEW-CONTRACT.md)
/// **P0-B**: No env reads — policy comes from request
/// **P0-C**: Evidence refs are repo-relative
/// **P0-D**: Parses ConsensusJson and derives signals
///
/// # Arguments
/// * `request` - The review request with repo root, spec id, stage, and options
/// * `checkpoint` - The checkpoint to evaluate (resolved from stage)
///
/// # Returns
/// * `Ok(StageReviewResult)` - Evaluation completed (may be pass or fail)
/// * `Err(String)` - Infrastructure error (e.g., file I/O failure)
pub fn evaluate_stage_review(
    request: ReviewRequest,
    checkpoint: Checkpoint,
) -> Result<StageReviewResult, String> {
    let repo_root = &request.repo_root;

    // P0-A: Build correct paths per REVIEW-CONTRACT.md
    // P1-D: Use evidence_root override if provided, else default
    let spec_packet_dir = repo_root.join(SPEC_PACKET_ROOT).join(&request.spec_id);
    let evidence_root = request
        .options
        .evidence_root
        .as_ref()
        .map(|p| repo_root.join(p))
        .unwrap_or_else(|| repo_root.join(EVIDENCE_ROOT));
    let consensus_dir = evidence_root.join(CONSENSUS_DIR).join(&request.spec_id);

    // Collect artifacts from spec packet + consensus evidence
    let artifacts = collect_artifacts_v2(repo_root, &spec_packet_dir, &consensus_dir, checkpoint);

    // P0-D: Parse ConsensusJson files and derive signals
    // P0-2: Pass repo_root for repo-relative evidence paths
    // P1-C: Track parse errors separately for --strict-schema
    let signal_result =
        collect_signals_from_consensus(&artifacts.consensus_files, checkpoint, repo_root);

    // P1-C: In strict_schema mode, parse errors are infrastructure failures → exit 3
    if request.options.strict_schema && !signal_result.parse_errors.is_empty() {
        return Err(format!(
            "Parse/schema errors in evidence files (--strict-schema enabled): {}",
            signal_result.parse_errors.join("; ")
        ));
    }

    let blocking_signals = signal_result.blocking;
    let advisory_signals = signal_result.advisory;

    // Determine resolution based on blocking signals
    // Invariant: resolution == AutoApply ⟹ blocking_signals.is_empty()
    let resolution = if blocking_signals.is_empty() {
        Verdict::AutoApply {
            effective_confidence: 0.9,
            confidence_level: ConfidenceLevel::High,
            reason: format!(
                "Stage {:?} artifacts reviewed, no blocking issues",
                request.stage
            ),
        }
    } else {
        Verdict::Escalate {
            target: EscalationTarget::Human,
            effective_confidence: compute_effective_confidence(&blocking_signals),
            confidence_level: ConfidenceLevel::Low,
            reason: format!(
                "{} blocking signal(s) require human review",
                blocking_signals.len()
            ),
        }
    };

    // P0-C: Build repo-relative evidence refs
    let evidence = build_evidence_refs_relative(repo_root, &consensus_dir);

    // P0-B: Policy snapshot from request (no env reads in core)
    let policy_snapshot = request.options.policy_snapshot.clone();

    Ok(StageReviewResult {
        spec_id: request.spec_id,
        stage: request.stage,
        checkpoint,
        resolution,
        blocking_signals,
        advisory_signals,
        artifact_sources: artifacts.sources,
        artifacts_collected: artifacts.total_count,
        review_evidence_count: artifacts.review_evidence_count,
        evidence,
        policy_snapshot,
    })
}

/// Collect artifacts from spec packet and consensus evidence (P0-A fix)
/// Artifact collection result
///
/// P0-3: Separate counts for spec docs vs review evidence.
struct ArtifactCollection {
    sources: Vec<ArtifactSource>,
    /// Total artifacts (spec docs + review evidence)
    total_count: usize,
    /// Review evidence files only (consensus files)
    review_evidence_count: usize,
    /// Paths to consensus files for signal parsing
    consensus_files: Vec<PathBuf>,
}

fn collect_artifacts_v2(
    repo_root: &Path,
    spec_packet_dir: &Path,
    consensus_dir: &Path,
    checkpoint: Checkpoint,
) -> ArtifactCollection {
    let mut sources = Vec::new();
    let mut spec_docs_count = 0;
    let mut consensus_files = Vec::new();

    let stage_slug = checkpoint_stage_slug(checkpoint);
    let requirements = checkpoint_artifact_requirements(checkpoint);

    // Check spec packet docs (e.g., docs/<SPEC-ID>/plan.md)
    for required in &requirements.required {
        if spec_packet_dir.join(required).exists() {
            spec_docs_count += 1;
        }
    }
    for optional in &requirements.optional {
        if spec_packet_dir.join(optional).exists() {
            spec_docs_count += 1;
        }
    }

    // Check consensus evidence files: spec-<stage>_*.json
    if consensus_dir.exists() && consensus_dir.is_dir() {
        let pattern_prefix = format!("spec-{stage_slug}_");

        for entry in WalkDir::new(consensus_dir)
            .max_depth(1)
            .into_iter()
            .flatten()
        {
            let path = entry.path();
            if path.extension().and_then(|s| s.to_str()) != Some("json") {
                continue;
            }
            let filename = entry.file_name().to_string_lossy();
            if filename.starts_with(&pattern_prefix) {
                consensus_files.push(path.to_path_buf());
            }
        }

        if !consensus_files.is_empty() {
            sources.push(ArtifactSource::FilesystemFallback);
        }
    }

    // Check for SQLite consensus DB (future)
    let consensus_db = repo_root.join(EVIDENCE_ROOT).join("consensus.sqlite");
    if consensus_db.exists() {
        sources.push(ArtifactSource::SQLite);
    }

    if sources.is_empty() {
        sources.push(ArtifactSource::None);
    }

    // Sort consensus files for deterministic "latest" selection (lexicographic max)
    consensus_files.sort();

    let review_evidence_count = consensus_files.len();

    ArtifactCollection {
        sources,
        total_count: spec_docs_count + review_evidence_count,
        review_evidence_count,
        consensus_files,
    }
}

/// Get stage slug for file matching
fn checkpoint_stage_slug(checkpoint: Checkpoint) -> &'static str {
    match checkpoint {
        Checkpoint::BeforePlan => "specify",
        Checkpoint::AfterPlan => "plan",
        Checkpoint::AfterTasks => "tasks",
        Checkpoint::AfterImplement => "implement",
        Checkpoint::AfterValidate => "validate",
        Checkpoint::BeforeUnlock => "audit",
    }
}

/// Result of collecting signals from consensus files
struct SignalCollectionResult {
    blocking: Vec<ReviewSignal>,
    advisory: Vec<ReviewSignal>,
    /// Parse/read errors (tracked separately for --strict-schema)
    parse_errors: Vec<String>,
}

/// Collect signals by parsing ConsensusJson files (P0-D)
///
/// - `consensus.conflicts` non-empty → blocking signals
/// - `error` field present → advisory signals
/// - Parse failures → advisory System signal (or error if strict_schema)
///
/// P0-2: All evidence paths are made repo-relative for stable CI output.
/// P1-C: Parse errors are tracked separately for --strict-schema handling.
fn collect_signals_from_consensus(
    consensus_files: &[PathBuf],
    _checkpoint: Checkpoint,
    repo_root: &Path,
) -> SignalCollectionResult {
    let mut blocking = Vec::new();
    let mut advisory = Vec::new();
    let mut parse_errors = Vec::new();

    // Helper to make path repo-relative (P0-2)
    let make_relative = |p: &Path| -> String {
        p.strip_prefix(repo_root)
            .map(|rel| rel.to_string_lossy().to_string())
            .unwrap_or_else(|_| p.to_string_lossy().to_string())
    };

    for path in consensus_files {
        let filename = path
            .file_name()
            .map(|s| s.to_string_lossy().to_string())
            .unwrap_or_default();

        // Try to parse the consensus file
        let content = match std::fs::read_to_string(path) {
            Ok(c) => c,
            Err(e) => {
                let error_msg = format!("Failed to read consensus file {filename}: {e}");
                parse_errors.push(error_msg.clone());
                // Also add as advisory for default (non-strict) mode
                advisory.push(ReviewSignal {
                    kind: CounterSignalKind::Other,
                    severity: SignalSeverity::Advisory,
                    origin: SignalOrigin::System,
                    worker_id: None,
                    message: error_msg,
                    evidence_path: Some(make_relative(path)),
                });
                continue;
            }
        };

        let data: ConsensusJson = match serde_json::from_str(&content) {
            Ok(d) => d,
            Err(e) => {
                let error_msg = format!("Failed to parse consensus file {filename}: {e}");
                parse_errors.push(error_msg.clone());
                // Also add as advisory for default (non-strict) mode
                advisory.push(ReviewSignal {
                    kind: CounterSignalKind::Other,
                    severity: SignalSeverity::Advisory,
                    origin: SignalOrigin::System,
                    worker_id: None,
                    message: error_msg,
                    evidence_path: Some(make_relative(path)),
                });
                continue;
            }
        };

        let role = infer_agent_role(data.agent.as_deref(), &filename);

        // Check for conflicts → blocking signals
        if let Some(consensus) = &data.consensus
            && let Some(conflicts) = &consensus.conflicts
        {
            for conflict in conflicts {
                if !conflict.is_empty() {
                    blocking.push(ReviewSignal {
                        kind: CounterSignalKind::Contradiction,
                        severity: SignalSeverity::Block,
                        origin: SignalOrigin::Role(role),
                        worker_id: data.agent.clone(),
                        message: conflict.clone(),
                        evidence_path: Some(make_relative(path)),
                    });
                }
            }
        }

        // Check for errors → advisory signals
        // P0-4: Per REVIEW-CONTRACT.md, error field → System origin
        // (errors are infrastructure/execution issues, not role-specific feedback)
        if let Some(error) = &data.error {
            advisory.push(ReviewSignal {
                kind: CounterSignalKind::Other,
                severity: SignalSeverity::Advisory,
                origin: SignalOrigin::System,
                worker_id: data.agent.clone(),
                message: error.clone(),
                evidence_path: Some(make_relative(path)),
            });
        }
    }

    SignalCollectionResult {
        blocking,
        advisory,
        parse_errors,
    }
}

/// Compute effective confidence from blocking signals
fn compute_effective_confidence(signals: &[ReviewSignal]) -> f32 {
    if signals.is_empty() {
        return 0.9;
    }

    // More signals = lower confidence
    // Each blocking signal reduces confidence
    let reduction = signals.len() as f32 * 0.15;
    (0.9 - reduction).max(0.1)
}

/// Build repo-relative evidence refs (P0-C)
fn build_evidence_refs_relative(repo_root: &Path, consensus_dir: &Path) -> EvidenceRefs {
    let mut refs = EvidenceRefs::default();

    // Helper to make path repo-relative
    let make_relative = |p: &Path| -> Option<String> {
        p.strip_prefix(repo_root)
            .ok()
            .map(|rel| rel.to_string_lossy().to_string())
    };

    // Check for verdict JSON
    let verdict_path = consensus_dir.join("verdict.json");
    if verdict_path.exists() {
        refs.verdict_json = make_relative(&verdict_path);
    }

    // Check for telemetry bundle
    let telemetry_path = consensus_dir.join("telemetry.json");
    if telemetry_path.exists() {
        refs.telemetry_bundle = make_relative(&telemetry_path);
    }

    // Check for synthesis
    let synthesis_path = consensus_dir.join("synthesis.md");
    if synthesis_path.exists() {
        refs.synthesis_path = make_relative(&synthesis_path);
    }

    // Set evidence directory (repo-relative)
    if consensus_dir.exists() {
        refs.evidence_dir = make_relative(consensus_dir);
    }

    refs
}

// ============================================================================
// Rendering (pure text, no ratatui)
// ============================================================================

/// Render a stage review result as text lines
///
/// Pure formatting function — adapters use this for CLI/TUI display.
pub fn render_review(result: &StageReviewResult) -> Vec<String> {
    let mut lines = Vec::new();

    // Header with verdict
    let verdict = result.display_verdict();
    lines.push(format!(
        "┌─ {} ─ {:?} @ {:?}",
        verdict.label(),
        result.stage,
        result.checkpoint
    ));
    lines.push(format!("│ SPEC: {}", result.spec_id));

    // Resolution details
    match &result.resolution {
        Verdict::AutoApply {
            effective_confidence,
            reason,
            ..
        } => {
            let conf_pct = effective_confidence * 100.0;
            lines.push(format!(
                "│ Resolution: AutoApply (confidence: {conf_pct:.0}%)"
            ));
            lines.push(format!("│ Reason: {reason}"));
        }
        Verdict::Escalate {
            target,
            effective_confidence,
            reason,
            ..
        } => {
            let conf_pct = effective_confidence * 100.0;
            lines.push(format!(
                "│ Resolution: Escalate to {target:?} (confidence: {conf_pct:.0}%)"
            ));
            lines.push(format!("│ Reason: {reason}"));
        }
    }

    // Artifacts
    lines.push(format!(
        "│ Artifacts: {} from {:?}",
        result.artifacts_collected, result.artifact_sources
    ));

    // Blocking signals
    if !result.blocking_signals.is_empty() {
        lines.push("│".to_string());
        lines.push(format!(
            "│ ⛔ Blocking signals ({})",
            result.blocking_signals.len()
        ));
        for signal in &result.blocking_signals {
            lines.push(format!(
                "│   • [{:?}] {} (from {})",
                signal.kind,
                signal.message,
                signal.origin.display_name()
            ));
        }
    }

    // Advisory signals
    if !result.advisory_signals.is_empty() {
        lines.push("│".to_string());
        lines.push(format!(
            "│ ⚠ Advisory signals ({})",
            result.advisory_signals.len()
        ));
        for signal in &result.advisory_signals {
            lines.push(format!(
                "│   • [{:?}] {} (from {})",
                signal.kind,
                signal.message,
                signal.origin.display_name()
            ));
        }
    }

    // Evidence paths
    if result.evidence.verdict_json.is_some()
        || result.evidence.telemetry_bundle.is_some()
        || result.evidence.synthesis_path.is_some()
    {
        lines.push("│".to_string());
        lines.push("│ Evidence:".to_string());
        if let Some(p) = &result.evidence.verdict_json {
            lines.push(format!("│   verdict: {p}"));
        }
        if let Some(p) = &result.evidence.telemetry_bundle {
            lines.push(format!("│   telemetry: {p}"));
        }
        if let Some(p) = &result.evidence.synthesis_path {
            lines.push(format!("│   synthesis: {p}"));
        }
    }

    // Policy snapshot
    lines.push("│".to_string());
    lines.push(format!(
        "│ Policy: sidecar_critic={}, telemetry={:?}",
        result.policy_snapshot.sidecar_critic_enabled, result.policy_snapshot.telemetry_mode
    ));

    // Footer
    lines.push("└─────────────────────────".to_string());

    lines
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_stage_checkpoint_mapping() {
        // Canonical review points
        assert!(matches!(
            resolve_review_request(Stage::Plan),
            ReviewResolution::Review {
                checkpoint: Checkpoint::AfterPlan
            }
        ));
        assert!(matches!(
            resolve_review_request(Stage::Tasks),
            ReviewResolution::Review {
                checkpoint: Checkpoint::AfterTasks
            }
        ));
        assert!(matches!(
            resolve_review_request(Stage::Audit),
            ReviewResolution::Review {
                checkpoint: Checkpoint::BeforeUnlock
            }
        ));

        // Diagnostic reviews
        assert!(matches!(
            resolve_review_request(Stage::Implement),
            ReviewResolution::Review {
                checkpoint: Checkpoint::AfterImplement
            }
        ));
        assert!(matches!(
            resolve_review_request(Stage::Validate),
            ReviewResolution::Review {
                checkpoint: Checkpoint::AfterValidate
            }
        ));
    }

    #[test]
    fn test_special_cases() {
        // Unlock aliases to BeforeUnlock
        assert!(matches!(
            resolve_review_request(Stage::Unlock),
            ReviewResolution::Alias {
                actual_checkpoint: Checkpoint::BeforeUnlock,
                ..
            }
        ));

        // Specify is not applicable
        assert!(matches!(
            resolve_review_request(Stage::Specify),
            ReviewResolution::NotApplicable { .. }
        ));
    }

    #[test]
    fn test_canonical_vs_diagnostic() {
        assert!(is_canonical_review_point(Stage::Plan));
        assert!(is_canonical_review_point(Stage::Tasks));
        assert!(is_canonical_review_point(Stage::Audit));

        assert!(!is_canonical_review_point(Stage::Implement));
        assert!(!is_canonical_review_point(Stage::Validate));

        assert!(is_diagnostic_review(Stage::Implement));
        assert!(is_diagnostic_review(Stage::Validate));

        assert!(!is_diagnostic_review(Stage::Plan));
    }

    #[test]
    fn test_display_verdict_derivation() {
        let base_result = StageReviewResult {
            spec_id: "TEST-001".to_string(),
            stage: Stage::Plan,
            checkpoint: Checkpoint::AfterPlan,
            resolution: Verdict::AutoApply {
                effective_confidence: 0.9,
                confidence_level: ConfidenceLevel::High,
                reason: "All checks passed".to_string(),
            },
            blocking_signals: vec![],
            advisory_signals: vec![],
            artifact_sources: vec![ArtifactSource::SQLite],
            artifacts_collected: 3,
            review_evidence_count: 3, // P0-3: Separate review evidence count
            evidence: EvidenceRefs::default(),
            policy_snapshot: PolicySnapshot::default(),
        };

        // AutoApply + no signals → Passed
        assert_eq!(base_result.display_verdict(), DisplayVerdict::Passed);

        // AutoApply + advisory signals → PassedWithWarnings
        let mut with_warnings = base_result.clone();
        with_warnings.advisory_signals.push(ReviewSignal {
            kind: CounterSignalKind::Ambiguity,
            severity: SignalSeverity::Advisory,
            origin: SignalOrigin::Role(Role::SidecarCritic),
            worker_id: None,
            message: "Minor concern".to_string(),
            evidence_path: None,
        });
        assert_eq!(
            with_warnings.display_verdict(),
            DisplayVerdict::PassedWithWarnings
        );

        // Escalate → Failed
        let mut escalated = base_result.clone();
        escalated.resolution = Verdict::Escalate {
            target: EscalationTarget::Human,
            effective_confidence: 0.4,
            confidence_level: ConfidenceLevel::Low,
            reason: "Blocking issue".to_string(),
        };
        assert_eq!(escalated.display_verdict(), DisplayVerdict::Failed);

        // No artifacts → Skipped
        // P0-3: Now uses review_evidence_count for skip logic
        let mut no_artifacts = base_result;
        no_artifacts.review_evidence_count = 0;
        assert!(matches!(
            no_artifacts.display_verdict(),
            DisplayVerdict::Skipped {
                reason: SkipReason::NoArtifactsFound
            }
        ));
    }

    #[test]
    fn test_exit_codes() {
        let default_options = ReviewOptions::default();
        let strict_options = ReviewOptions {
            strict_artifacts: true,
            strict_warnings: true,
            ..Default::default()
        };

        let passed = StageReviewResult {
            spec_id: "TEST".to_string(),
            stage: Stage::Plan,
            checkpoint: Checkpoint::AfterPlan,
            resolution: Verdict::AutoApply {
                effective_confidence: 0.9,
                confidence_level: ConfidenceLevel::High,
                reason: "ok".to_string(),
            },
            blocking_signals: vec![],
            advisory_signals: vec![],
            artifact_sources: vec![ArtifactSource::SQLite],
            artifacts_collected: 1,
            review_evidence_count: 1, // P0-3: Separate review evidence count
            evidence: EvidenceRefs::default(),
            policy_snapshot: PolicySnapshot::default(),
        };

        assert_eq!(passed.exit_code(&default_options), 0);
        assert_eq!(passed.exit_code(&strict_options), 0);

        // Failed always exit 2
        let mut failed = passed.clone();
        failed.resolution = Verdict::Escalate {
            target: EscalationTarget::Human,
            effective_confidence: 0.3,
            confidence_level: ConfidenceLevel::Low,
            reason: "blocked".to_string(),
        };
        assert_eq!(failed.exit_code(&default_options), 2);
        assert_eq!(failed.exit_code(&strict_options), 2);

        // No artifacts: exit 0 normally, exit 2 in strict mode
        // P0-3: Now uses review_evidence_count for skip logic
        let mut skipped = passed;
        skipped.review_evidence_count = 0;
        assert_eq!(skipped.exit_code(&default_options), 0);
        assert_eq!(skipped.exit_code(&strict_options), 2);
    }

    // ========================================================================
    // Fixture-based tests (P0-A through P0-D validation)
    // ========================================================================

    /// Helper: Create fixture directory structure for review tests
    fn create_fixture_dir() -> tempfile::TempDir {
        tempfile::tempdir().expect("Failed to create temp dir")
    }

    /// Helper: Create consensus JSON content with conflicts
    fn consensus_with_conflicts(conflicts: &[&str]) -> String {
        let conflicts_json: Vec<String> = conflicts.iter().map(|s| format!("\"{s}\"")).collect();
        format!(
            r#"{{
            "agent": "architect",
            "model": "claude",
            "consensus": {{
                "conflicts": [{}],
                "synthesis_status": "complete"
            }}
        }}"#,
            conflicts_json.join(", ")
        )
    }

    /// Helper: Create clean consensus JSON (no conflicts)
    fn consensus_clean() -> String {
        r#"{
            "agent": "implementer",
            "model": "claude",
            "consensus": {
                "conflicts": [],
                "synthesis_status": "complete"
            }
        }"#
        .to_string()
    }

    /// Helper: Create consensus JSON with error field
    fn consensus_with_error(error: &str) -> String {
        format!(
            r#"{{
            "agent": "validator",
            "model": "claude",
            "error": "{error}",
            "consensus": {{
                "conflicts": [],
                "synthesis_status": "failed"
            }}
        }}"#
        )
    }

    #[test]
    fn test_fixture_conflicts_produce_escalate() {
        // Setup: Create fixture with consensus file containing conflicts
        let temp = create_fixture_dir();
        let repo_root = temp.path();
        let spec_id = "FIXTURE-001";

        // Create consensus directory structure
        let consensus_dir = repo_root
            .join(EVIDENCE_ROOT)
            .join(CONSENSUS_DIR)
            .join(spec_id);
        std::fs::create_dir_all(&consensus_dir).unwrap();

        // Create spec packet directory with plan.md
        let spec_packet_dir = repo_root.join(SPEC_PACKET_ROOT).join(spec_id);
        std::fs::create_dir_all(&spec_packet_dir).unwrap();
        std::fs::write(spec_packet_dir.join("plan.md"), "# Plan\nTest plan").unwrap();

        // Create consensus file with conflict
        let consensus_file = consensus_dir.join("spec-plan_architect_20251220.json");
        std::fs::write(
            &consensus_file,
            consensus_with_conflicts(&["Requirement A contradicts requirement B"]),
        )
        .unwrap();

        // Execute review
        let request = ReviewRequest {
            repo_root: repo_root.to_path_buf(),
            spec_id: spec_id.to_string(),
            stage: Stage::Plan,
            options: ReviewOptions::default(),
        };

        let result = evaluate_stage_review(request, Checkpoint::AfterPlan).unwrap();

        // Verify: Should escalate due to conflict
        assert!(
            matches!(result.resolution, Verdict::Escalate { .. }),
            "Expected Escalate due to conflict, got: {:?}",
            result.resolution
        );
        assert!(
            !result.blocking_signals.is_empty(),
            "Expected blocking signals"
        );
        assert!(result.blocking_signals[0].message.contains("contradicts"));
    }

    #[test]
    fn test_fixture_clean_produces_autoapply() {
        // Setup: Create fixture with clean consensus (no conflicts)
        let temp = create_fixture_dir();
        let repo_root = temp.path();
        let spec_id = "FIXTURE-002";

        // Create consensus directory structure
        let consensus_dir = repo_root
            .join(EVIDENCE_ROOT)
            .join(CONSENSUS_DIR)
            .join(spec_id);
        std::fs::create_dir_all(&consensus_dir).unwrap();

        // Create spec packet directory with plan.md
        let spec_packet_dir = repo_root.join(SPEC_PACKET_ROOT).join(spec_id);
        std::fs::create_dir_all(&spec_packet_dir).unwrap();
        std::fs::write(spec_packet_dir.join("plan.md"), "# Plan\nTest plan").unwrap();

        // Create clean consensus file
        let consensus_file = consensus_dir.join("spec-plan_implementer_20251220.json");
        std::fs::write(&consensus_file, consensus_clean()).unwrap();

        // Execute review
        let request = ReviewRequest {
            repo_root: repo_root.to_path_buf(),
            spec_id: spec_id.to_string(),
            stage: Stage::Plan,
            options: ReviewOptions::default(),
        };

        let result = evaluate_stage_review(request, Checkpoint::AfterPlan).unwrap();

        // Verify: Should auto-apply (no conflicts)
        assert!(
            matches!(result.resolution, Verdict::AutoApply { .. }),
            "Expected AutoApply with clean consensus, got: {:?}",
            result.resolution
        );
        assert!(
            result.blocking_signals.is_empty(),
            "Expected no blocking signals"
        );
    }

    #[test]
    fn test_fixture_error_produces_advisory() {
        // Setup: Create fixture with consensus containing error field
        let temp = create_fixture_dir();
        let repo_root = temp.path();
        let spec_id = "FIXTURE-003";

        // Create consensus directory structure
        let consensus_dir = repo_root
            .join(EVIDENCE_ROOT)
            .join(CONSENSUS_DIR)
            .join(spec_id);
        std::fs::create_dir_all(&consensus_dir).unwrap();

        // Create spec packet directory with plan.md
        let spec_packet_dir = repo_root.join(SPEC_PACKET_ROOT).join(spec_id);
        std::fs::create_dir_all(&spec_packet_dir).unwrap();
        std::fs::write(spec_packet_dir.join("plan.md"), "# Plan\nTest plan").unwrap();

        // Create consensus file with error (but no conflicts)
        let consensus_file = consensus_dir.join("spec-plan_validator_20251220.json");
        std::fs::write(
            &consensus_file,
            consensus_with_error("Agent timeout exceeded"),
        )
        .unwrap();

        // Execute review
        let request = ReviewRequest {
            repo_root: repo_root.to_path_buf(),
            spec_id: spec_id.to_string(),
            stage: Stage::Plan,
            options: ReviewOptions::default(),
        };

        let result = evaluate_stage_review(request, Checkpoint::AfterPlan).unwrap();

        // Verify: Should auto-apply (error is advisory, not blocking)
        assert!(
            matches!(result.resolution, Verdict::AutoApply { .. }),
            "Expected AutoApply (error is advisory), got: {:?}",
            result.resolution
        );
        assert!(result.blocking_signals.is_empty(), "Error should not block");
        assert!(
            !result.advisory_signals.is_empty(),
            "Expected advisory signal for error"
        );
        assert!(result.advisory_signals[0].message.contains("timeout"));
    }

    #[test]
    fn test_fixture_parse_error_produces_advisory() {
        // Setup: Create fixture with malformed JSON
        let temp = create_fixture_dir();
        let repo_root = temp.path();
        let spec_id = "FIXTURE-004";

        // Create consensus directory structure
        let consensus_dir = repo_root
            .join(EVIDENCE_ROOT)
            .join(CONSENSUS_DIR)
            .join(spec_id);
        std::fs::create_dir_all(&consensus_dir).unwrap();

        // Create spec packet directory with plan.md
        let spec_packet_dir = repo_root.join(SPEC_PACKET_ROOT).join(spec_id);
        std::fs::create_dir_all(&spec_packet_dir).unwrap();
        std::fs::write(spec_packet_dir.join("plan.md"), "# Plan\nTest plan").unwrap();

        // Create malformed consensus file
        let consensus_file = consensus_dir.join("spec-plan_broken_20251220.json");
        std::fs::write(&consensus_file, "{ this is not valid json }").unwrap();

        // Execute review
        let request = ReviewRequest {
            repo_root: repo_root.to_path_buf(),
            spec_id: spec_id.to_string(),
            stage: Stage::Plan,
            options: ReviewOptions::default(),
        };

        let result = evaluate_stage_review(request, Checkpoint::AfterPlan).unwrap();

        // Verify: Should auto-apply (parse error is advisory, not blocking)
        assert!(
            matches!(result.resolution, Verdict::AutoApply { .. }),
            "Expected AutoApply (parse error is advisory), got: {:?}",
            result.resolution
        );
        assert!(
            result.blocking_signals.is_empty(),
            "Parse error should not block"
        );
        assert!(
            !result.advisory_signals.is_empty(),
            "Expected advisory signal for parse error"
        );
        assert!(
            result.advisory_signals[0]
                .message
                .contains("Failed to parse")
        );
    }

    #[test]
    fn test_fixture_no_artifacts_with_strict() {
        // Setup: Create fixture with no consensus files
        let temp = create_fixture_dir();
        let repo_root = temp.path();
        let spec_id = "FIXTURE-005";

        // Create consensus directory (empty - no files)
        let consensus_dir = repo_root
            .join(EVIDENCE_ROOT)
            .join(CONSENSUS_DIR)
            .join(spec_id);
        std::fs::create_dir_all(&consensus_dir).unwrap();

        // Execute review with strict artifacts mode
        let request = ReviewRequest {
            repo_root: repo_root.to_path_buf(),
            spec_id: spec_id.to_string(),
            stage: Stage::Plan,
            options: ReviewOptions {
                strict_artifacts: true,
                ..Default::default()
            },
        };

        let result = evaluate_stage_review(request, Checkpoint::AfterPlan).unwrap();

        // Verify: Should show skipped with no artifacts
        assert_eq!(result.artifacts_collected, 0, "Expected no artifacts");
        assert!(
            matches!(
                result.display_verdict(),
                DisplayVerdict::Skipped {
                    reason: SkipReason::NoArtifactsFound
                }
            ),
            "Expected Skipped verdict"
        );

        // In strict mode, exit code should be 2
        let strict_options = ReviewOptions {
            strict_artifacts: true,
            ..Default::default()
        };
        assert_eq!(
            result.exit_code(&strict_options),
            2,
            "Strict mode: missing artifacts → exit 2"
        );
    }

    #[test]
    fn test_fixture_evidence_refs_are_repo_relative() {
        // Setup: Create fixture with verdict.json
        let temp = create_fixture_dir();
        let repo_root = temp.path();
        let spec_id = "FIXTURE-006";

        // Create consensus directory structure
        let consensus_dir = repo_root
            .join(EVIDENCE_ROOT)
            .join(CONSENSUS_DIR)
            .join(spec_id);
        std::fs::create_dir_all(&consensus_dir).unwrap();

        // Create spec packet directory with plan.md
        let spec_packet_dir = repo_root.join(SPEC_PACKET_ROOT).join(spec_id);
        std::fs::create_dir_all(&spec_packet_dir).unwrap();
        std::fs::write(spec_packet_dir.join("plan.md"), "# Plan\nTest plan").unwrap();

        // Create consensus and verdict files
        let consensus_file = consensus_dir.join("spec-plan_claude_20251220.json");
        std::fs::write(&consensus_file, consensus_clean()).unwrap();

        let verdict_file = consensus_dir.join("verdict.json");
        std::fs::write(&verdict_file, r#"{"verdict": "pass"}"#).unwrap();

        // Execute review
        let request = ReviewRequest {
            repo_root: repo_root.to_path_buf(),
            spec_id: spec_id.to_string(),
            stage: Stage::Plan,
            options: ReviewOptions::default(),
        };

        let result = evaluate_stage_review(request, Checkpoint::AfterPlan).unwrap();

        // Verify: Evidence refs should be repo-relative (P0-C)
        if let Some(verdict_path) = &result.evidence.verdict_json {
            assert!(
                !verdict_path.starts_with('/'),
                "Evidence path should be repo-relative, not absolute: {verdict_path}"
            );
            assert!(
                verdict_path.starts_with("docs/"),
                "Evidence path should start with docs/: {verdict_path}"
            );
        }

        if let Some(evidence_dir) = &result.evidence.evidence_dir {
            assert!(
                !evidence_dir.starts_with('/'),
                "Evidence dir should be repo-relative: {evidence_dir}"
            );
        }
    }

    #[test]
    fn test_infer_agent_role() {
        // Test role inference from agent field
        assert_eq!(
            infer_agent_role(Some("architect"), "file.json"),
            Role::Architect
        );
        assert_eq!(
            infer_agent_role(Some("implementer"), "file.json"),
            Role::Implementer
        );
        assert_eq!(
            infer_agent_role(Some("validator"), "file.json"),
            Role::Validator
        );
        assert_eq!(infer_agent_role(Some("judge"), "file.json"), Role::Judge);
        assert_eq!(
            infer_agent_role(Some("sidecar-critic"), "file.json"),
            Role::SidecarCritic
        );

        // Test fallback to filename
        assert_eq!(
            infer_agent_role(None, "spec-plan_architect_20251220.json"),
            Role::Architect
        );
        assert_eq!(
            infer_agent_role(None, "implementer_response.json"),
            Role::Implementer
        );

        // Test default
        assert_eq!(infer_agent_role(None, "unknown.json"), Role::Architect);
        assert_eq!(
            infer_agent_role(Some("claude"), "file.json"),
            Role::Architect
        );
    }

    #[test]
    fn test_fixture_two_files_determinism() {
        // Risk mitigation: Verify deterministic processing of multiple consensus files
        // Files are sorted lexicographically; signals from all files are aggregated
        let temp = create_fixture_dir();
        let repo_root = temp.path();
        let spec_id = "FIXTURE-DETERM";

        // Create consensus directory structure
        let consensus_dir = repo_root
            .join(EVIDENCE_ROOT)
            .join(CONSENSUS_DIR)
            .join(spec_id);
        std::fs::create_dir_all(&consensus_dir).unwrap();

        // Create spec packet directory with plan.md
        let spec_packet_dir = repo_root.join(SPEC_PACKET_ROOT).join(spec_id);
        std::fs::create_dir_all(&spec_packet_dir).unwrap();
        std::fs::write(spec_packet_dir.join("plan.md"), "# Plan\nTest plan").unwrap();

        // Create TWO consensus files for the same stage with different timestamps
        // File 1: older timestamp, has conflict A
        let file_older = consensus_dir.join("spec-plan_architect_20251219.json");
        std::fs::write(
            &file_older,
            consensus_with_conflicts(&["Conflict from older file"]),
        )
        .unwrap();

        // File 2: newer timestamp, has conflict B
        let file_newer = consensus_dir.join("spec-plan_implementer_20251220.json");
        std::fs::write(
            &file_newer,
            consensus_with_conflicts(&["Conflict from newer file"]),
        )
        .unwrap();

        // Execute review multiple times to verify determinism
        let mut results = Vec::new();
        for _ in 0..3 {
            let request = ReviewRequest {
                repo_root: repo_root.to_path_buf(),
                spec_id: spec_id.to_string(),
                stage: Stage::Plan,
                options: ReviewOptions::default(),
            };
            results.push(evaluate_stage_review(request, Checkpoint::AfterPlan).unwrap());
        }

        // Verify: All runs produce identical results
        for result in &results {
            assert!(
                matches!(result.resolution, Verdict::Escalate { .. }),
                "Expected Escalate due to conflicts"
            );
            // Both files' conflicts should be aggregated
            assert_eq!(
                result.blocking_signals.len(),
                2,
                "Expected 2 blocking signals (one from each file)"
            );
        }

        // Verify deterministic ordering: older file processed first (lexicographic)
        let first_result = &results[0];
        assert!(
            first_result.blocking_signals[0]
                .message
                .contains("older file"),
            "First signal should be from older file (lexicographic order)"
        );
        assert!(
            first_result.blocking_signals[1]
                .message
                .contains("newer file"),
            "Second signal should be from newer file (lexicographic order)"
        );

        // All runs identical
        for i in 1..results.len() {
            assert_eq!(
                results[0].blocking_signals.len(),
                results[i].blocking_signals.len(),
                "All runs should have same signal count"
            );
            for j in 0..results[0].blocking_signals.len() {
                assert_eq!(
                    results[0].blocking_signals[j].message, results[i].blocking_signals[j].message,
                    "Signal order must be deterministic across runs"
                );
            }
        }
    }

    #[test]
    fn test_fixture_multiple_agents_aggregation() {
        // Verify that signals from multiple agents are correctly aggregated
        // One file with conflict, one clean, one with error
        let temp = create_fixture_dir();
        let repo_root = temp.path();
        let spec_id = "FIXTURE-MULTI";

        let consensus_dir = repo_root
            .join(EVIDENCE_ROOT)
            .join(CONSENSUS_DIR)
            .join(spec_id);
        std::fs::create_dir_all(&consensus_dir).unwrap();

        let spec_packet_dir = repo_root.join(SPEC_PACKET_ROOT).join(spec_id);
        std::fs::create_dir_all(&spec_packet_dir).unwrap();
        std::fs::write(spec_packet_dir.join("plan.md"), "# Plan").unwrap();

        // Agent 1: Architect with conflict
        std::fs::write(
            consensus_dir.join("spec-plan_architect_20251220.json"),
            consensus_with_conflicts(&["Scope creep detected"]),
        )
        .unwrap();

        // Agent 2: Implementer clean (no conflicts)
        std::fs::write(
            consensus_dir.join("spec-plan_implementer_20251220.json"),
            consensus_clean(),
        )
        .unwrap();

        // Agent 3: Validator with error (advisory)
        std::fs::write(
            consensus_dir.join("spec-plan_validator_20251220.json"),
            consensus_with_error("Timeout during validation"),
        )
        .unwrap();

        let request = ReviewRequest {
            repo_root: repo_root.to_path_buf(),
            spec_id: spec_id.to_string(),
            stage: Stage::Plan,
            options: ReviewOptions::default(),
        };

        let result = evaluate_stage_review(request, Checkpoint::AfterPlan).unwrap();

        // Verify aggregation
        assert!(
            matches!(result.resolution, Verdict::Escalate { .. }),
            "Should escalate due to architect's conflict"
        );
        assert_eq!(
            result.blocking_signals.len(),
            1,
            "One blocking signal from architect"
        );
        assert!(result.blocking_signals[0].message.contains("Scope creep"));
        assert_eq!(
            result.advisory_signals.len(),
            1,
            "One advisory signal from validator error"
        );
        assert!(result.advisory_signals[0].message.contains("Timeout"));
    }

    // ========================================================================
    // P1-C: --strict-schema tests
    // ========================================================================

    #[test]
    fn test_fixture_strict_schema_parse_error_returns_err() {
        // P1-C: With --strict-schema, parse errors should return Err (exit 3)
        let temp = create_fixture_dir();
        let repo_root = temp.path();
        let spec_id = "FIXTURE-STRICT";

        // Create consensus directory structure
        let consensus_dir = repo_root
            .join(EVIDENCE_ROOT)
            .join(CONSENSUS_DIR)
            .join(spec_id);
        std::fs::create_dir_all(&consensus_dir).unwrap();

        // Create spec packet directory with plan.md
        let spec_packet_dir = repo_root.join(SPEC_PACKET_ROOT).join(spec_id);
        std::fs::create_dir_all(&spec_packet_dir).unwrap();
        std::fs::write(spec_packet_dir.join("plan.md"), "# Plan").unwrap();

        // Create malformed consensus file
        let consensus_file = consensus_dir.join("spec-plan_broken_20251220.json");
        std::fs::write(&consensus_file, "{ this is not valid json }").unwrap();

        // Execute review WITH strict_schema enabled
        let request = ReviewRequest {
            repo_root: repo_root.to_path_buf(),
            spec_id: spec_id.to_string(),
            stage: Stage::Plan,
            options: ReviewOptions {
                strict_schema: true,
                ..Default::default()
            },
        };

        let result = evaluate_stage_review(request, Checkpoint::AfterPlan);

        // Verify: Should return Err due to parse error with strict_schema
        assert!(
            result.is_err(),
            "Expected Err with --strict-schema and parse error, got: {result:?}"
        );
        let error_msg = result.unwrap_err();
        assert!(
            error_msg.contains("Parse/schema errors"),
            "Error should mention parse/schema errors: {error_msg}"
        );
        assert!(
            error_msg.contains("--strict-schema"),
            "Error should mention --strict-schema flag: {error_msg}"
        );
    }

    #[test]
    fn test_fixture_strict_schema_clean_succeeds() {
        // P1-C: With --strict-schema but no parse errors, should succeed
        let temp = create_fixture_dir();
        let repo_root = temp.path();
        let spec_id = "FIXTURE-STRICT-CLEAN";

        // Create consensus directory structure
        let consensus_dir = repo_root
            .join(EVIDENCE_ROOT)
            .join(CONSENSUS_DIR)
            .join(spec_id);
        std::fs::create_dir_all(&consensus_dir).unwrap();

        // Create spec packet directory with plan.md
        let spec_packet_dir = repo_root.join(SPEC_PACKET_ROOT).join(spec_id);
        std::fs::create_dir_all(&spec_packet_dir).unwrap();
        std::fs::write(spec_packet_dir.join("plan.md"), "# Plan").unwrap();

        // Create clean consensus file (valid JSON)
        let consensus_file = consensus_dir.join("spec-plan_clean_20251220.json");
        std::fs::write(&consensus_file, consensus_clean()).unwrap();

        // Execute review WITH strict_schema enabled
        let request = ReviewRequest {
            repo_root: repo_root.to_path_buf(),
            spec_id: spec_id.to_string(),
            stage: Stage::Plan,
            options: ReviewOptions {
                strict_schema: true,
                ..Default::default()
            },
        };

        let result = evaluate_stage_review(request, Checkpoint::AfterPlan);

        // Verify: Should succeed (no parse errors)
        assert!(
            result.is_ok(),
            "Expected Ok with --strict-schema and valid files, got: {result:?}"
        );
    }

    // ========================================================================
    // P1-B: SPEC-CI-001 Smoke Packet Contract Tests
    // ========================================================================

    /// Get path to SPEC-CI-001 smoke packet fixture
    fn get_smoke_packet_path() -> std::path::PathBuf {
        // Fixture is at: spec-kit/tests/fixtures/SPEC-CI-001/
        let manifest_dir = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        manifest_dir.join("tests/fixtures/SPEC-CI-001")
    }

    #[test]
    fn smoke_packet_clean_exits_0() {
        // P1-B: SPEC-CI-001-clean should exit 0 (AutoApply, no conflicts)
        let repo_root = get_smoke_packet_path();
        if !repo_root.exists() {
            // Skip if fixture not found (graceful degradation)
            return;
        }

        let request = ReviewRequest {
            repo_root,
            spec_id: "SPEC-CI-001-clean".to_string(),
            stage: Stage::Plan,
            options: ReviewOptions::default(),
        };

        let result = evaluate_stage_review(request, Checkpoint::AfterPlan).unwrap();

        // Contract: clean case → AutoApply, exit 0
        assert!(
            matches!(result.resolution, Verdict::AutoApply { .. }),
            "SPEC-CI-001-clean should AutoApply, got: {:?}",
            result.resolution
        );
        assert!(
            result.blocking_signals.is_empty(),
            "SPEC-CI-001-clean should have no blocking signals"
        );
        assert_eq!(
            result.exit_code(&ReviewOptions::default()),
            0,
            "SPEC-CI-001-clean should exit 0"
        );
    }

    #[test]
    fn smoke_packet_conflict_exits_2() {
        // P1-B: SPEC-CI-001-conflict should exit 2 (Escalate, has conflicts)
        let repo_root = get_smoke_packet_path();
        if !repo_root.exists() {
            // Skip if fixture not found (graceful degradation)
            return;
        }

        let request = ReviewRequest {
            repo_root,
            spec_id: "SPEC-CI-001-conflict".to_string(),
            stage: Stage::Plan,
            options: ReviewOptions::default(),
        };

        let result = evaluate_stage_review(request, Checkpoint::AfterPlan).unwrap();

        // Contract: conflict case → Escalate, exit 2
        assert!(
            matches!(result.resolution, Verdict::Escalate { .. }),
            "SPEC-CI-001-conflict should Escalate, got: {:?}",
            result.resolution
        );
        assert!(
            !result.blocking_signals.is_empty(),
            "SPEC-CI-001-conflict should have blocking signals"
        );
        assert_eq!(
            result.exit_code(&ReviewOptions::default()),
            2,
            "SPEC-CI-001-conflict should exit 2"
        );

        // Verify expected conflicts are present
        let conflict_messages: Vec<&str> = result
            .blocking_signals
            .iter()
            .map(|s| s.message.as_str())
            .collect();
        assert!(
            conflict_messages.iter().any(|m| m.contains("contradicts")),
            "Expected 'contradicts' conflict, got: {conflict_messages:?}"
        );
    }

    #[test]
    fn smoke_packet_malformed_advisory_without_strict() {
        // P1-B: SPEC-CI-001-malformed should exit 0 without --strict-schema
        let repo_root = get_smoke_packet_path();
        if !repo_root.exists() {
            // Skip if fixture not found (graceful degradation)
            return;
        }

        let request = ReviewRequest {
            repo_root,
            spec_id: "SPEC-CI-001-malformed".to_string(),
            stage: Stage::Plan,
            options: ReviewOptions::default(), // strict_schema: false
        };

        let result = evaluate_stage_review(request, Checkpoint::AfterPlan);

        // Contract: malformed without strict → Ok with advisory
        assert!(
            result.is_ok(),
            "SPEC-CI-001-malformed should succeed without strict, got: {result:?}"
        );
        let result = result.unwrap();
        assert!(
            !result.advisory_signals.is_empty(),
            "SPEC-CI-001-malformed should have advisory signal for parse error"
        );
    }

    #[test]
    fn smoke_packet_malformed_error_with_strict() {
        // P1-B: SPEC-CI-001-malformed should exit 3 with --strict-schema
        let repo_root = get_smoke_packet_path();
        if !repo_root.exists() {
            // Skip if fixture not found (graceful degradation)
            return;
        }

        let request = ReviewRequest {
            repo_root,
            spec_id: "SPEC-CI-001-malformed".to_string(),
            stage: Stage::Plan,
            options: ReviewOptions {
                strict_schema: true,
                ..Default::default()
            },
        };

        let result = evaluate_stage_review(request, Checkpoint::AfterPlan);

        // Contract: malformed with strict → Err
        assert!(
            result.is_err(),
            "SPEC-CI-001-malformed should fail with --strict-schema, got: {result:?}"
        );
    }

    // ========================================================================
    // P1-D: evidence_root override tests
    // ========================================================================

    #[test]
    fn test_evidence_root_override() {
        // P1-D: --evidence-root should override the default path
        let temp = create_fixture_dir();
        let repo_root = temp.path();
        let spec_id = "OVERRIDE-TEST";

        // Create custom evidence root (not the default SPEC-OPS-004 path)
        let custom_evidence = repo_root.join("custom").join("evidence");
        let consensus_dir = custom_evidence.join(CONSENSUS_DIR).join(spec_id);
        std::fs::create_dir_all(&consensus_dir).unwrap();

        // Create spec packet
        let spec_packet_dir = repo_root.join(SPEC_PACKET_ROOT).join(spec_id);
        std::fs::create_dir_all(&spec_packet_dir).unwrap();
        std::fs::write(spec_packet_dir.join("plan.md"), "# Plan").unwrap();

        // Create consensus file in custom location
        let consensus_file = consensus_dir.join("spec-plan_test_20251220.json");
        std::fs::write(&consensus_file, consensus_clean()).unwrap();

        // Request with evidence_root override
        let request = ReviewRequest {
            repo_root: repo_root.to_path_buf(),
            spec_id: spec_id.to_string(),
            stage: Stage::Plan,
            options: ReviewOptions {
                evidence_root: Some(PathBuf::from("custom/evidence")),
                ..Default::default()
            },
        };

        let result = evaluate_stage_review(request, Checkpoint::AfterPlan);

        // Should find evidence at custom path
        assert!(
            result.is_ok(),
            "Should find evidence at custom path, got: {result:?}"
        );
        let result = result.unwrap();
        assert!(
            result.review_evidence_count > 0,
            "Should have review evidence from custom path"
        );
        assert!(
            matches!(result.resolution, Verdict::AutoApply { .. }),
            "Should AutoApply with clean consensus"
        );
    }

    #[test]
    fn test_evidence_root_override_not_found() {
        // P1-D: If evidence_root points to missing path, should skip (no evidence)
        let temp = create_fixture_dir();
        let repo_root = temp.path();
        let spec_id = "OVERRIDE-MISSING";

        // Create spec packet only (no evidence at custom path)
        let spec_packet_dir = repo_root.join(SPEC_PACKET_ROOT).join(spec_id);
        std::fs::create_dir_all(&spec_packet_dir).unwrap();
        std::fs::write(spec_packet_dir.join("plan.md"), "# Plan").unwrap();

        // Request with non-existent evidence_root
        let request = ReviewRequest {
            repo_root: repo_root.to_path_buf(),
            spec_id: spec_id.to_string(),
            stage: Stage::Plan,
            options: ReviewOptions {
                evidence_root: Some(PathBuf::from("nonexistent/evidence")),
                ..Default::default()
            },
        };

        let result = evaluate_stage_review(request, Checkpoint::AfterPlan).unwrap();

        // Should produce Skipped (no review evidence found)
        assert_eq!(
            result.review_evidence_count, 0,
            "Should have no review evidence from missing path"
        );
        assert!(
            matches!(
                result.display_verdict(),
                DisplayVerdict::Skipped {
                    reason: SkipReason::NoArtifactsFound
                }
            ),
            "Should be Skipped with no artifacts"
        );
    }
}
