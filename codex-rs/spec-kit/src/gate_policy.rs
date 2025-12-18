//! Gate Policy Domain Types
//!
//! Canonical vocabulary for Spec-Kit gate evaluation.
//!
//! **Design Principle**: Gate evaluation is deterministic. Sidecars emit signals,
//! not competing answers. There is no voting, no committee synthesis, no debate loops.
//!
//! See `docs/spec-kit/GATE_POLICY.md` for full specification.

use serde::{Deserialize, Serialize};
use std::path::PathBuf;

// ============================================================================
// Core Domain Enums
// ============================================================================

/// Pipeline stages where artifacts are produced.
///
/// Each stage has exactly one owner (a Role implemented by a Worker).
/// This is the canonical stage enum for gate policy; it extends `SpecStage`
/// with `Specify` (pre-plan context gathering).
#[derive(Clone, Copy, Debug, Eq, PartialEq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Stage {
    /// Pre-plan: gather context, define scope
    Specify,
    /// Plan: architecture decisions, approach
    Plan,
    /// Tasks: decomposition into actionable items
    Tasks,
    /// Implement: code generation
    Implement,
    /// Validate: test strategy and execution
    Validate,
    /// Audit: compliance and quality review
    Audit,
    /// Unlock: final approval gate
    Unlock,
}

impl Stage {
    /// All stages in pipeline order
    pub fn all() -> [Self; 7] {
        [
            Self::Specify,
            Self::Plan,
            Self::Tasks,
            Self::Implement,
            Self::Validate,
            Self::Audit,
            Self::Unlock,
        ]
    }

    /// Convert to command name (lowercase)
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Specify => "specify",
            Self::Plan => "plan",
            Self::Tasks => "tasks",
            Self::Implement => "implement",
            Self::Validate => "validate",
            Self::Audit => "audit",
            Self::Unlock => "unlock",
        }
    }

    /// Human-readable display name
    pub fn display_name(&self) -> &'static str {
        match self {
            Self::Specify => "Specify",
            Self::Plan => "Plan",
            Self::Tasks => "Tasks",
            Self::Implement => "Implement",
            Self::Validate => "Validate",
            Self::Audit => "Audit",
            Self::Unlock => "Unlock",
        }
    }
}

impl From<crate::types::SpecStage> for Stage {
    fn from(stage: crate::types::SpecStage) -> Self {
        match stage {
            crate::types::SpecStage::Plan => Self::Plan,
            crate::types::SpecStage::Tasks => Self::Tasks,
            crate::types::SpecStage::Implement => Self::Implement,
            crate::types::SpecStage::Validate => Self::Validate,
            crate::types::SpecStage::Audit => Self::Audit,
            crate::types::SpecStage::Unlock => Self::Unlock,
        }
    }
}

/// Checkpoints where gates run (stage boundaries).
///
/// Gates evaluate signals at these boundaries to determine whether
/// to auto-apply or escalate.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Checkpoint {
    /// Before planning begins (after Specify)
    BeforePlan,
    /// After plan artifact is produced
    AfterPlan,
    /// After tasks decomposition
    AfterTasks,
    /// After implementation (pre-validation)
    AfterImplement,
    /// After validation (pre-audit)
    AfterValidate,
    /// Before final unlock
    BeforeUnlock,
}

impl Checkpoint {
    /// Which stage this checkpoint follows (if any)
    pub fn after_stage(&self) -> Option<Stage> {
        match self {
            Self::BeforePlan => Some(Stage::Specify),
            Self::AfterPlan => Some(Stage::Plan),
            Self::AfterTasks => Some(Stage::Tasks),
            Self::AfterImplement => Some(Stage::Implement),
            Self::AfterValidate => Some(Stage::Validate),
            Self::BeforeUnlock => Some(Stage::Audit),
        }
    }
}

/// Responsibilities in the workflow.
///
/// Roles are abstract; the Router maps them to concrete Workers.
/// This avoids hardcoding model/provider names in gate policy.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Role {
    /// Designs architecture, makes high-level decisions
    Architect,
    /// Writes code, executes implementation
    Implementer,
    /// Runs tests, validates correctness
    Validator,
    /// Final reviewer, enforces policy compliance
    Judge,

    // --- Sidecars (non-authoritative, emit signals only) ---
    /// Non-blocking critic sidecar
    SidecarCritic,
    /// Security-focused reviewer
    SecurityReviewer,
    /// Performance-focused reviewer
    PerformanceReviewer,
    /// Long-context reconciliation, evidence sweeps
    Librarian,
}

impl Role {
    /// Whether this role is a sidecar (non-authoritative)
    pub fn is_sidecar(&self) -> bool {
        matches!(
            self,
            Self::SidecarCritic
                | Self::SecurityReviewer
                | Self::PerformanceReviewer
                | Self::Librarian
        )
    }

    /// Whether this role can own a stage (produce authoritative artifacts)
    pub fn can_own_stage(&self) -> bool {
        !self.is_sidecar()
    }
}

// ============================================================================
// Signal Types
// ============================================================================

/// Severity of a counter-signal.
///
/// - `Advisory`: Improves quality but never blocks on its own
/// - `Block`: Forces escalation OR additional gate
#[derive(Clone, Copy, Debug, Eq, PartialEq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SignalSeverity {
    Advisory,
    Block,
}

/// Classification of counter-signal issues.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CounterSignalKind {
    PolicyViolation,
    Contradiction,
    MissingAcceptanceCriteria,
    Ambiguity,
    HighRiskChange,
    SafetyRisk,
    SecurityRisk,
    PerformanceRisk,
    Other,
}

/// A counter-signal emitted by a sidecar or validator.
///
/// Counter-signals are **not** competing answers. They are typed observations
/// that may affect confidence or force escalation.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct CounterSignal {
    pub severity: SignalSeverity,
    pub kind: CounterSignalKind,
    pub source_role: Role,
    pub message: String,
    /// Optional reference to evidence artifact (file path, JSON key, etc.)
    pub evidence_ref: Option<String>,
}

/// Tool-truth signal kinds (deterministic, non-LLM).
#[derive(Clone, Copy, Debug, Eq, PartialEq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ToolTruthKind {
    Compile,
    UnitTests,
    IntegrationTests,
    Lint,
    Format,
    SchemaValidation,
    TypeCheck,
}

/// Tool-truth signal: deterministic result from compiler/tests/linters.
///
/// These signals are authoritative because they come from tooling, not LLMs.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ToolTruth {
    pub kind: ToolTruthKind,
    pub passed: bool,
    pub details: Option<String>,
    /// Exit code if applicable
    pub exit_code: Option<i32>,
}

impl ToolTruth {
    /// Create a passing tool-truth signal
    pub fn pass(kind: ToolTruthKind) -> Self {
        Self {
            kind,
            passed: true,
            details: None,
            exit_code: Some(0),
        }
    }

    /// Create a failing tool-truth signal
    pub fn fail(kind: ToolTruthKind, details: impl Into<String>) -> Self {
        Self {
            kind,
            passed: false,
            details: Some(details.into()),
            exit_code: None,
        }
    }
}

/// Risk level classification for a stage/artifact.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RiskLevel {
    Low,
    Medium,
    High,
    Critical,
}

/// All signal types that can be inputs to a gate.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum Signal {
    /// Owner's self-reported confidence
    OwnerConfidence {
        value: f32,
        rationale: Option<String>,
    },
    /// Tool-truth (compiler, tests, linters)
    ToolTruth(ToolTruth),
    /// Counter-signal from sidecar/validator
    CounterSignal(CounterSignal),
    /// Risk classification
    Risk(RiskLevel),
}

// ============================================================================
// Decision Rule and Verdict
// ============================================================================

/// Configuration for gate decision rules.
///
/// The field `min_confidence_for_auto_apply` replaces the legacy
/// `consensus_threshold` (handled via serde alias in config layer).
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct DecisionRule {
    /// Minimum effective confidence to auto-apply (replaces legacy consensus_threshold)
    #[serde(default = "default_min_confidence")]
    pub min_confidence_for_auto_apply: f32,

    /// Whether to allow auto-apply when only advisory signals are present
    #[serde(default = "default_allow_advisory")]
    pub allow_advisory_auto_apply: bool,

    /// Force escalation if any tool-truth fails
    #[serde(default = "default_tool_truth_escalation")]
    pub escalate_on_tool_failure: bool,
}

/// Default minimum confidence for auto-apply.
///
/// Set slightly above the Medium threshold (0.65) to provide margin.
/// This is the canonical default; override via config if needed.
fn default_min_confidence() -> f32 {
    0.65
}
fn default_allow_advisory() -> bool {
    true
}
fn default_tool_truth_escalation() -> bool {
    true
}

impl Default for DecisionRule {
    fn default() -> Self {
        Self {
            min_confidence_for_auto_apply: default_min_confidence(),
            allow_advisory_auto_apply: default_allow_advisory(),
            escalate_on_tool_failure: default_tool_truth_escalation(),
        }
    }
}

/// Where to escalate when auto-apply is not possible.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EscalationTarget {
    Human,
    JudgeRole,
    ImplementerFallback,
    LibrarianSweep,
}

/// Computed confidence level (categorical).
#[derive(Clone, Copy, Debug, Eq, PartialEq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ConfidenceLevel {
    High,   // >= 0.80
    Medium, // >= 0.65
    Low,    // < 0.65
}

impl ConfidenceLevel {
    /// Compute confidence level from numeric value
    pub fn from_value(value: f32) -> Self {
        if value >= 0.80 {
            Self::High
        } else if value >= 0.65 {
            Self::Medium
        } else {
            Self::Low
        }
    }
}

/// Gate verdict: the output of gate evaluation.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(tag = "resolution", rename_all = "snake_case")]
pub enum Verdict {
    AutoApply {
        effective_confidence: f32,
        confidence_level: ConfidenceLevel,
        reason: String,
    },
    Escalate {
        target: EscalationTarget,
        effective_confidence: f32,
        confidence_level: ConfidenceLevel,
        reason: String,
    },
}

impl Verdict {
    /// Whether this verdict allows automatic progression
    pub fn is_auto_apply(&self) -> bool {
        matches!(self, Self::AutoApply { .. })
    }

    /// Get the effective confidence value
    pub fn effective_confidence(&self) -> f32 {
        match self {
            Self::AutoApply {
                effective_confidence,
                ..
            } => *effective_confidence,
            Self::Escalate {
                effective_confidence,
                ..
            } => *effective_confidence,
        }
    }
}

// ============================================================================
// Gate Context and Full Verdict
// ============================================================================

/// Context for gate evaluation.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct GateContext {
    pub spec_id: String,
    pub stage: Stage,
    pub checkpoint: Checkpoint,
    pub artifact_paths: Vec<PathBuf>,
    pub is_high_risk: bool,
    pub retry_count: u32,
}

/// Complete gate verdict with all context and signals.
///
/// This is the "unit of evidence" that should be persisted for auditing.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct GateVerdict {
    pub context: GateContext,
    pub decision_rule: DecisionRule,
    pub verdict: Verdict,
    pub signals: Vec<Signal>,
    pub counter_signals: Vec<CounterSignal>,
    pub timestamp: chrono::DateTime<chrono::Utc>,
}

impl GateVerdict {
    /// Quick check: did the gate pass (auto-apply)?
    pub fn passed(&self) -> bool {
        self.verdict.is_auto_apply()
    }
}

// ============================================================================
// Role Assignment (Gate Policy → Orchestrator interface)
// ============================================================================

/// Role assignment for a stage.
///
/// Returned by `roles_for_stage()` to tell the orchestrator who owns
/// the stage and what sidecars should run.
#[derive(Clone, Debug)]
pub struct RoleAssignment {
    /// The authoritative owner of this stage
    pub owner: Role,
    /// Optional sidecars that emit signals (non-authoritative)
    pub sidecars: Vec<Role>,
}

/// Get the role assignment for a given stage.
///
/// This is the Gate Policy's interface to the orchestrator.
/// It returns who should own the stage and what sidecars to run.
///
/// **Important**: This function is deterministic and does NOT read env vars.
/// The orchestration layer is responsible for populating `ctx.policy` from
/// environment variables and configuration.
///
/// **Important**: This function does NOT return model/provider names.
/// The Router handles that mapping separately.
pub fn roles_for_stage(stage: Stage, ctx: &StageContext) -> RoleAssignment {
    let owner = match stage {
        Stage::Specify | Stage::Plan => Role::Architect,
        Stage::Tasks => Role::Architect,
        Stage::Implement => Role::Implementer,
        Stage::Validate => Role::Validator,
        Stage::Audit | Stage::Unlock => Role::Judge,
    };

    // Determine sidecars based on context (deterministic, no env reads)
    let mut sidecars = Vec::new();

    // Critic sidecar if enabled via policy toggles
    if ctx.policy.sidecar_critic_enabled {
        sidecars.push(Role::SidecarCritic);
    }

    // High-risk stages get security review if enabled
    let security_review_stages = matches!(stage, Stage::Implement | Stage::Validate);
    if ctx.is_high_risk && security_review_stages && ctx.policy.security_reviewer_enabled {
        sidecars.push(Role::SecurityReviewer);
    }

    RoleAssignment { owner, sidecars }
}

/// Get checkpoints for a stage transition.
pub fn checkpoints_for_stage_transition(from: Stage, to: Stage) -> Vec<Checkpoint> {
    match (from, to) {
        (Stage::Specify, Stage::Plan) => vec![Checkpoint::BeforePlan],
        (Stage::Plan, Stage::Tasks) => vec![Checkpoint::AfterPlan],
        (Stage::Tasks, Stage::Implement) => vec![Checkpoint::AfterTasks],
        (Stage::Implement, Stage::Validate) => vec![Checkpoint::AfterImplement],
        (Stage::Validate, Stage::Audit) => vec![Checkpoint::AfterValidate],
        (Stage::Audit, Stage::Unlock) => vec![Checkpoint::BeforeUnlock],
        _ => vec![],
    }
}

// ============================================================================
// Stage Context (shared with Router)
// ============================================================================

/// Policy toggles that control gate behavior.
///
/// These are determined by the orchestration layer (reading env vars, config, etc.)
/// and passed into gate policy functions. Gate policy itself does NOT read env vars.
#[derive(Clone, Debug, Default)]
pub struct PolicyToggles {
    /// Enable non-blocking critic sidecar (env: SPEC_KIT_SIDECAR_CRITIC)
    pub sidecar_critic_enabled: bool,
    /// Enable security reviewer for high-risk stages
    pub security_reviewer_enabled: bool,
}

/// Context passed to both Gate Policy and Router.
///
/// **Design**: This struct carries policy-level context, not filesystem-level details.
/// Artifact paths are identifiers, not absolute filesystem paths.
#[derive(Clone, Debug, Default)]
pub struct StageContext {
    pub spec_id: String,
    pub stage: Option<Stage>,
    pub local_only: bool,
    pub is_high_risk: bool,
    pub retry_count: u32,
    pub artifact_paths: Vec<PathBuf>,
    /// Policy toggles (sidecars, etc.) - set by orchestration layer
    pub policy: PolicyToggles,
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_stage_conversion_from_spec_stage() {
        use crate::types::SpecStage;

        assert_eq!(Stage::from(SpecStage::Plan), Stage::Plan);
        assert_eq!(Stage::from(SpecStage::Implement), Stage::Implement);
        assert_eq!(Stage::from(SpecStage::Unlock), Stage::Unlock);
    }

    #[test]
    fn test_role_is_sidecar() {
        assert!(!Role::Architect.is_sidecar());
        assert!(!Role::Implementer.is_sidecar());
        assert!(Role::SidecarCritic.is_sidecar());
        assert!(Role::SecurityReviewer.is_sidecar());
    }

    #[test]
    fn test_confidence_level_from_value() {
        assert_eq!(ConfidenceLevel::from_value(0.95), ConfidenceLevel::High);
        assert_eq!(ConfidenceLevel::from_value(0.80), ConfidenceLevel::High);
        assert_eq!(ConfidenceLevel::from_value(0.79), ConfidenceLevel::Medium);
        assert_eq!(ConfidenceLevel::from_value(0.65), ConfidenceLevel::Medium);
        assert_eq!(ConfidenceLevel::from_value(0.64), ConfidenceLevel::Low);
        assert_eq!(ConfidenceLevel::from_value(0.0), ConfidenceLevel::Low);
    }

    #[test]
    fn test_verdict_is_auto_apply() {
        let auto = Verdict::AutoApply {
            effective_confidence: 0.85,
            confidence_level: ConfidenceLevel::High,
            reason: "All signals pass".into(),
        };
        assert!(auto.is_auto_apply());

        let escalate = Verdict::Escalate {
            target: EscalationTarget::Human,
            effective_confidence: 0.45,
            confidence_level: ConfidenceLevel::Low,
            reason: "Block signal present".into(),
        };
        assert!(!escalate.is_auto_apply());
    }

    #[test]
    fn test_roles_for_stage_defaults() {
        let ctx = StageContext::default();

        let plan = roles_for_stage(Stage::Plan, &ctx);
        assert_eq!(plan.owner, Role::Architect);
        assert!(plan.sidecars.is_empty()); // No policy toggles enabled

        let impl_stage = roles_for_stage(Stage::Implement, &ctx);
        assert_eq!(impl_stage.owner, Role::Implementer);
    }

    #[test]
    fn test_roles_for_stage_with_sidecars() {
        let ctx = StageContext {
            policy: PolicyToggles {
                sidecar_critic_enabled: true,
                security_reviewer_enabled: true,
            },
            is_high_risk: true,
            ..Default::default()
        };

        let plan = roles_for_stage(Stage::Plan, &ctx);
        assert_eq!(plan.owner, Role::Architect);
        assert_eq!(plan.sidecars, vec![Role::SidecarCritic]); // Critic only (Plan not security-reviewed)

        let impl_stage = roles_for_stage(Stage::Implement, &ctx);
        assert_eq!(impl_stage.owner, Role::Implementer);
        assert!(impl_stage.sidecars.contains(&Role::SidecarCritic));
        assert!(impl_stage.sidecars.contains(&Role::SecurityReviewer)); // High-risk + Implement
    }

    #[test]
    fn test_checkpoints_for_transition() {
        let checkpoints = checkpoints_for_stage_transition(Stage::Plan, Stage::Tasks);
        assert_eq!(checkpoints, vec![Checkpoint::AfterPlan]);

        let checkpoints = checkpoints_for_stage_transition(Stage::Audit, Stage::Unlock);
        assert_eq!(checkpoints, vec![Checkpoint::BeforeUnlock]);
    }

    #[test]
    fn test_tool_truth_constructors() {
        let pass = ToolTruth::pass(ToolTruthKind::Compile);
        assert!(pass.passed);
        assert_eq!(pass.exit_code, Some(0));

        let fail = ToolTruth::fail(ToolTruthKind::UnitTests, "3 tests failed");
        assert!(!fail.passed);
        assert_eq!(fail.details, Some("3 tests failed".into()));
    }

    #[test]
    fn test_decision_rule_defaults() {
        let rule = DecisionRule::default();
        assert!((rule.min_confidence_for_auto_apply - 0.65).abs() < 0.001);
        assert!(rule.allow_advisory_auto_apply);
        assert!(rule.escalate_on_tool_failure);
    }
}
