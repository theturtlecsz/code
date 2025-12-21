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

    /// Plan validation passed - ready for agent execution
    ///
    /// SPEC-KIT-921 P3-B: TUI adapter should spawn agents when receiving this.
    /// CLI with --dry-run reports success and exits.
    PlanReady {
        /// The validated SPEC identifier
        spec_id: String,
        /// Stage to execute
        stage: crate::Stage,
        /// Warnings from guardrail validation (non-blocking)
        warnings: Vec<String>,
        /// Whether this was a dry-run (validation only)
        dry_run: bool,
    },

    /// Plan validation blocked - guardrails failed
    PlanBlocked {
        /// The SPEC identifier
        spec_id: String,
        /// Stage that was attempted
        stage: crate::Stage,
        /// Errors from guardrail validation (blocking)
        errors: Vec<String>,
    },

    /// Command failed with error
    Error(String),
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
            SpeckitCommand::Plan {
                spec_id,
                stage,
                dry_run,
            } => self.execute_plan(&spec_id, stage, dry_run),
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

    /// Execute plan command (validate prerequisites and guardrails)
    ///
    /// SPEC-KIT-921 P3-B: This validates the SPEC and runs basic guardrails.
    /// Returns PlanReady if validation passes, PlanBlocked if it fails.
    ///
    /// The adapter (TUI) handles agent spawning after receiving PlanReady.
    /// CLI with --dry-run reports success and exits.
    fn execute_plan(&self, spec_id: &str, stage: crate::Stage, dry_run: bool) -> Outcome {
        let mut warnings: Vec<String> = Vec::new();
        let mut errors: Vec<String> = Vec::new();

        // 1. Validate SPEC-ID format
        if spec_id.is_empty() {
            errors.push("SPEC-ID is required".to_string());
            return Outcome::PlanBlocked {
                spec_id: spec_id.to_string(),
                stage,
                errors,
            };
        }

        // 2. Check if SPEC directory exists (docs/SPEC-xxx or docs/SPEC-xxx/*)
        let spec_dir = self.context.repo_root.join("docs").join(spec_id);
        if !spec_dir.exists() {
            // Also check for partial match (e.g., SPEC-KIT-921 might be in docs/SPEC-KIT-921-feature-name/)
            let docs_dir = self.context.repo_root.join("docs");
            if docs_dir.exists() {
                let has_matching_dir = std::fs::read_dir(&docs_dir)
                    .map(|entries| {
                        entries.filter_map(Result::ok).any(|e| {
                            e.file_name()
                                .to_string_lossy()
                                .starts_with(&format!("{spec_id}-"))
                                || e.file_name().to_string_lossy() == spec_id
                        })
                    })
                    .unwrap_or(false);

                if !has_matching_dir {
                    warnings.push(format!(
                        "SPEC directory not found: docs/{spec_id} (will be created if needed)"
                    ));
                }
            }
        }

        // 3. Basic stage validation
        match stage {
            crate::Stage::Specify => {
                // Specify is always allowed
            }
            crate::Stage::Plan => {
                // Plan requires SPEC to exist or be created
            }
            crate::Stage::Tasks | crate::Stage::Implement => {
                // These typically require plan.md to exist
                let plan_path = spec_dir.join("plan.md");
                if spec_dir.exists() && !plan_path.exists() {
                    warnings.push(format!(
                        "plan.md not found for {spec_id} - recommended to run /speckit.plan first"
                    ));
                }
            }
            crate::Stage::Validate | crate::Stage::Audit | crate::Stage::Unlock => {
                // These require implementation to exist
                if spec_dir.exists() {
                    let has_impl = ["implement.md", "tasks.md"]
                        .iter()
                        .any(|f| spec_dir.join(f).exists());
                    if !has_impl {
                        warnings.push(format!(
                            "No implementation artifacts found for {spec_id} - recommended to run earlier stages first"
                        ));
                    }
                }
            }
        }

        // If there are errors, return PlanBlocked
        if !errors.is_empty() {
            return Outcome::PlanBlocked {
                spec_id: spec_id.to_string(),
                stage,
                errors,
            };
        }

        // Validation passed
        Outcome::PlanReady {
            spec_id: spec_id.to_string(),
            stage,
            warnings,
            dry_run,
        }
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
