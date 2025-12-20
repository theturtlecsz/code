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

use std::path::PathBuf;

use serde::{Deserialize, Serialize};

use crate::gate_policy::{
    Checkpoint, ConfidenceLevel, CounterSignalKind, EscalationTarget, Role, SignalSeverity, Stage,
    ToolTruthKind, Verdict,
};

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

    /// Number of artifacts collected
    pub artifacts_collected: usize,

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
        // Skip conditions first
        if self.artifacts_collected == 0 {
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
}

impl Default for ReviewOptions {
    fn default() -> Self {
        Self {
            telemetry_mode: TelemetryMode::Disabled,
            include_diagnostic: false,
            strict_artifacts: false,
            strict_warnings: false,
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
// Core Evaluation Function
// ============================================================================

/// Evaluate stage review artifacts and produce a result
///
/// This is the core pure function that evaluates gate artifacts.
/// No ratatui types — adapters render the result.
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
    let evidence_base = request
        .repo_root
        .join(crate::DEFAULT_EVIDENCE_BASE)
        .join(&request.spec_id);

    // Collect artifacts from evidence directories
    let (artifact_sources, artifacts_collected) = collect_artifacts(&evidence_base, checkpoint);

    // Collect signals (blocking and advisory)
    let (blocking_signals, advisory_signals) = collect_signals(&evidence_base, checkpoint);

    // Determine resolution based on blocking signals
    let resolution = if blocking_signals.is_empty() {
        // No blockers → AutoApply
        Verdict::AutoApply {
            effective_confidence: 0.9,
            confidence_level: ConfidenceLevel::High,
            reason: format!(
                "Stage {:?} artifacts reviewed, no blocking issues",
                request.stage
            ),
        }
    } else {
        // Blockers present → Escalate
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

    // Build evidence refs (paths to persisted evidence)
    let evidence = build_evidence_refs(&evidence_base);

    // Build policy snapshot
    let policy_snapshot = PolicySnapshot {
        sidecar_critic_enabled: std::env::var("SPEC_KIT_SIDECAR_CRITIC").is_ok(),
        telemetry_mode: request.options.telemetry_mode,
        legacy_voting_env_detected: std::env::var("SPEC_KIT_VOTING").is_ok(),
    };

    Ok(StageReviewResult {
        spec_id: request.spec_id,
        stage: request.stage,
        checkpoint,
        resolution,
        blocking_signals,
        advisory_signals,
        artifact_sources,
        artifacts_collected,
        evidence,
        policy_snapshot,
    })
}

/// Collect artifacts from evidence directory
fn collect_artifacts(
    evidence_base: &std::path::Path,
    checkpoint: Checkpoint,
) -> (Vec<ArtifactSource>, usize) {
    let requirements = checkpoint_artifact_requirements(checkpoint);
    let mut sources = Vec::new();
    let mut count = 0;

    // Check SQLite consensus DB
    let consensus_db = evidence_base.join("consensus.sqlite");
    if consensus_db.exists() {
        sources.push(ArtifactSource::SQLite);
        count += 1;
    }

    // Check stage-specific artifact files
    let stage_dir = evidence_base.join(checkpoint_stage_dir(checkpoint));
    for required in &requirements.required {
        if stage_dir.join(required).exists() {
            count += 1;
        }
    }
    for optional in &requirements.optional {
        if stage_dir.join(optional).exists() {
            count += 1;
        }
    }

    // Check for filesystem fallback artifacts
    if stage_dir.exists() && stage_dir.is_dir() && sources.is_empty() {
        sources.push(ArtifactSource::FilesystemFallback);
    }

    if sources.is_empty() {
        sources.push(ArtifactSource::None);
    }

    (sources, count)
}

/// Get stage directory name for a checkpoint
fn checkpoint_stage_dir(checkpoint: Checkpoint) -> &'static str {
    match checkpoint {
        Checkpoint::BeforePlan => "specify",
        Checkpoint::AfterPlan => "plan",
        Checkpoint::AfterTasks => "tasks",
        Checkpoint::AfterImplement => "implement",
        Checkpoint::AfterValidate => "validate",
        Checkpoint::BeforeUnlock => "audit",
    }
}

/// Collect signals from evidence directory
///
/// In this initial implementation, we check for signal files in the evidence.
/// Future: integrate with SQLite consensus and real gate evaluation.
fn collect_signals(
    evidence_base: &std::path::Path,
    checkpoint: Checkpoint,
) -> (Vec<ReviewSignal>, Vec<ReviewSignal>) {
    let mut blocking = Vec::new();
    let mut advisory = Vec::new();

    // Check for blocking signal file
    let stage_dir = evidence_base.join(checkpoint_stage_dir(checkpoint));
    let blocking_file = stage_dir.join("blocking_signals.json");
    let advisory_file = stage_dir.join("advisory_signals.json");

    // Parse blocking signals
    if let Some(signals) = std::fs::read_to_string(&blocking_file)
        .ok()
        .and_then(|content| serde_json::from_str::<Vec<ReviewSignal>>(&content).ok())
    {
        blocking.extend(signals);
    }

    // Parse advisory signals
    if let Some(signals) = std::fs::read_to_string(&advisory_file)
        .ok()
        .and_then(|content| serde_json::from_str::<Vec<ReviewSignal>>(&content).ok())
    {
        advisory.extend(signals);
    }

    (blocking, advisory)
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

/// Build evidence refs from evidence directory
fn build_evidence_refs(evidence_base: &std::path::Path) -> EvidenceRefs {
    let mut refs = EvidenceRefs::default();

    // Check for verdict JSON
    let verdict_path = evidence_base.join("verdict.json");
    if verdict_path.exists() {
        refs.verdict_json = Some(verdict_path.to_string_lossy().into_owned());
    }

    // Check for telemetry bundle
    let telemetry_path = evidence_base.join("telemetry.json");
    if telemetry_path.exists() {
        refs.telemetry_bundle = Some(telemetry_path.to_string_lossy().into_owned());
    }

    // Check for synthesis
    let synthesis_path = evidence_base.join("synthesis.md");
    if synthesis_path.exists() {
        refs.synthesis_path = Some(synthesis_path.to_string_lossy().into_owned());
    }

    // Set evidence directory
    if evidence_base.exists() {
        refs.evidence_dir = Some(evidence_base.to_string_lossy().into_owned());
    }

    refs
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
        let mut no_artifacts = base_result;
        no_artifacts.artifacts_collected = 0;
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
        let mut skipped = passed;
        skipped.artifacts_collected = 0;
        assert_eq!(skipped.exit_code(&default_options), 0);
        assert_eq!(skipped.exit_code(&strict_options), 2);
    }
}
