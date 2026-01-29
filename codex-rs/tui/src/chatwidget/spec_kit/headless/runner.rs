//! Headless pipeline runner for CLI automation (SPEC-KIT-900)
//!
//! Executes the `/speckit.auto` pipeline without a terminal/TUI.
//!
//! ## Exit Codes (D133)
//!
//! - 0: SUCCESS
//! - 3: INFRA_ERROR
//! - 10: NEEDS_INPUT (missing maieutic)
//! - 11: NEEDS_APPROVAL (checkpoint requires human)
//! - 13: PROMPT_ATTEMPTED (any prompt attempt in headless mode)

use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::mpsc;
use std::time::{Duration, Instant};

use codex_spec_kit::Stage;
use serde::{Deserialize, Serialize};

use super::event_pump::AgentResult;
use super::output::{HeadlessOutput, Stage0Info};
use crate::chatwidget::spec_kit::maieutic::MaieuticSpec;
use crate::chatwidget::spec_kit::stage0_integration::{
    Stage0ExecutionConfig, Stage0ExecutionResult, Stage0Progress, spawn_stage0_async,
};

/// Exit codes for headless execution (D133)
pub mod exit_codes {
    pub const SUCCESS: i32 = 0;
    pub const INFRA_ERROR: i32 = 3;
    pub const NEEDS_INPUT: i32 = 10;
    pub const NEEDS_APPROVAL: i32 = 11;
    pub const PROMPT_ATTEMPTED: i32 = 13;

    pub fn reason(code: i32) -> &'static str {
        match code {
            SUCCESS => "success",
            INFRA_ERROR => "infra_error",
            NEEDS_INPUT => "needs_input",
            NEEDS_APPROVAL => "needs_approval",
            PROMPT_ATTEMPTED => "prompt_attempted",
            _ => "unknown",
        }
    }
}

/// Headless execution errors
#[derive(Debug, Clone)]
pub enum HeadlessError {
    /// Missing required maieutic input
    NeedsInput(String),
    /// Tier-2/3 checkpoint requires approval
    NeedsApproval { checkpoint: String },
    /// Prompt was attempted in headless mode (invariant violation)
    PromptAttempted,
    /// Timeout waiting for agents
    Timeout {
        expected: usize,
        completed: usize,
        elapsed_ms: u64,
    },
    /// Infrastructure/system error
    InfraError(String),
    /// Stage validation failed
    ValidationFailed { stage: String, reason: String },
}

impl std::fmt::Display for HeadlessError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::NeedsInput(msg) => write!(f, "Missing maieutic input: {}", msg),
            Self::NeedsApproval { checkpoint } => {
                write!(f, "Checkpoint requires approval: {}", checkpoint)
            }
            Self::PromptAttempted => write!(f, "Prompt attempted in headless mode"),
            Self::Timeout {
                expected,
                completed,
                elapsed_ms,
            } => {
                write!(
                    f,
                    "Timeout: {}/{} agents completed after {}ms",
                    completed, expected, elapsed_ms
                )
            }
            Self::InfraError(msg) => write!(f, "Infrastructure error: {}", msg),
            Self::ValidationFailed { stage, reason } => {
                write!(f, "Validation failed for {}: {}", stage, reason)
            }
        }
    }
}

impl HeadlessError {
    /// Convert error to exit code
    pub fn exit_code(&self) -> i32 {
        match self {
            Self::NeedsInput(_) => exit_codes::NEEDS_INPUT,
            Self::NeedsApproval { .. } => exit_codes::NEEDS_APPROVAL,
            Self::PromptAttempted => exit_codes::PROMPT_ATTEMPTED,
            Self::Timeout { .. } => exit_codes::INFRA_ERROR,
            Self::InfraError(_) => exit_codes::INFRA_ERROR,
            Self::ValidationFailed { .. } => exit_codes::INFRA_ERROR,
        }
    }
}

/// Result of headless pipeline execution
#[derive(Debug, Clone)]
pub struct HeadlessResult {
    /// Exit code (0 = success)
    pub exit_code: i32,
    /// Stages that completed successfully
    pub stages_completed: Vec<String>,
    /// Error if any
    pub error: Option<HeadlessError>,
    /// Stage0 info if executed
    pub stage0_info: Option<Stage0Info>,
}

impl HeadlessResult {
    /// Create a successful result
    pub fn success(stages_completed: Vec<String>, stage0_info: Option<Stage0Info>) -> Self {
        Self {
            exit_code: exit_codes::SUCCESS,
            stages_completed,
            error: None,
            stage0_info,
        }
    }

    /// Create an error result
    pub fn error(error: HeadlessError, stages_completed: Vec<String>) -> Self {
        Self {
            exit_code: error.exit_code(),
            stages_completed,
            error: Some(error),
            stage0_info: None,
        }
    }

    /// Create result with specific exit code
    pub fn exit(exit_code: i32, stages_completed: Vec<String>) -> Self {
        Self {
            exit_code,
            stages_completed,
            error: None,
            stage0_info: None,
        }
    }

    /// Convert to output format for JSON serialization
    pub fn to_output(&self, spec_id: &str, from_stage: &str, to_stage: &str) -> HeadlessOutput {
        if let Some(ref err) = self.error {
            HeadlessOutput::error(
                spec_id.to_string(),
                from_stage.to_string(),
                to_stage.to_string(),
                self.exit_code,
                exit_codes::reason(self.exit_code),
                err.to_string(),
                self.stages_completed.clone(),
            )
        } else {
            HeadlessOutput::success(
                spec_id.to_string(),
                from_stage.to_string(),
                to_stage.to_string(),
                self.stages_completed.clone(),
                self.stage0_info.clone(),
            )
        }
    }
}

/// Configuration for headless pipeline execution
#[derive(Debug, Clone)]
pub struct HeadlessConfig {
    /// Stage0 execution timeout
    pub stage0_timeout: Duration,
    /// Agent execution timeout per stage
    pub agent_timeout: Duration,
    /// Enable quality gates
    pub quality_gates_enabled: bool,
    /// Output as JSON
    pub json_output: bool,
}

impl Default for HeadlessConfig {
    fn default() -> Self {
        Self {
            stage0_timeout: Duration::from_secs(120),
            agent_timeout: Duration::from_secs(300),
            quality_gates_enabled: false,
            json_output: true,
        }
    }
}

/// Headless pipeline runner
///
/// Executes `/speckit.auto` without requiring a TUI.
pub struct HeadlessPipelineRunner {
    /// SPEC identifier
    pub spec_id: String,
    /// Starting stage
    pub from_stage: Stage,
    /// Ending stage
    pub to_stage: Stage,
    /// Maieutic spec with pre-supplied answers
    pub maieutic_spec: MaieuticSpec,
    /// Configuration
    pub config: HeadlessConfig,
    /// Core config for LLM access
    pub planner_config: codex_core::config::Config,
    /// Working directory
    pub cwd: PathBuf,

    // Internal state
    stages_completed: Vec<String>,
    stage0_result: Option<Stage0ExecutionResult>,
}

impl HeadlessPipelineRunner {
    /// Create a new headless runner
    pub fn new(
        spec_id: String,
        from_stage: Stage,
        to_stage: Stage,
        maieutic_spec: MaieuticSpec,
        config: HeadlessConfig,
        planner_config: codex_core::config::Config,
        cwd: PathBuf,
    ) -> Self {
        Self {
            spec_id,
            from_stage,
            to_stage,
            maieutic_spec,
            config,
            planner_config,
            cwd,
            stages_completed: Vec::new(),
            stage0_result: None,
        }
    }

    /// Run the pipeline (main entry point)
    pub fn run(&mut self) -> HeadlessResult {
        tracing::info!(
            spec_id = %self.spec_id,
            from = %self.from_stage.as_str(),
            to = %self.to_stage.as_str(),
            "Starting headless pipeline execution"
        );

        // Validate maieutic spec
        if let Err(e) = self.maieutic_spec.validate() {
            return HeadlessResult::error(
                HeadlessError::NeedsInput(e.to_string()),
                self.stages_completed.clone(),
            );
        }

        // Step 1: Run Stage0 context injection
        match self.run_stage0() {
            Ok(result) => {
                self.stage0_result = Some(result);
            }
            Err(e) => {
                return HeadlessResult::error(e, self.stages_completed.clone());
            }
        }

        // Step 2: Execute each stage in the range
        let stages = match Stage::range(self.from_stage, self.to_stage) {
            Some(s) => s,
            None => {
                return HeadlessResult::error(
                    HeadlessError::InfraError(format!(
                        "Invalid stage range: {} > {}",
                        self.from_stage.as_str(),
                        self.to_stage.as_str()
                    )),
                    self.stages_completed.clone(),
                );
            }
        };

        for stage in stages {
            tracing::info!(stage = %stage.as_str(), "Executing stage");

            // Step 2a: Guardrail check
            if let Err(e) = self.check_guardrails(&stage) {
                return HeadlessResult::error(e, self.stages_completed.clone());
            }

            // Step 2b: Execute stage (spawn agents, wait for completion)
            if let Err(e) = self.execute_stage(&stage) {
                return HeadlessResult::error(e, self.stages_completed.clone());
            }

            // Step 2c: Quality gate (if enabled)
            if self.config.quality_gates_enabled {
                if let Err(e) = self.run_quality_gate(&stage) {
                    return HeadlessResult::error(e, self.stages_completed.clone());
                }
            }

            // Mark stage as completed
            self.stages_completed.push(stage.as_str().to_string());
            tracing::info!(stage = %stage.as_str(), "Stage completed");
        }

        // Build Stage0 info for output
        let stage0_info = self.stage0_result.as_ref().map(|r| Stage0Info {
            completed: r.result.is_some(),
            duration_ms: r.duration_ms,
            tier2_used: r.tier2_used,
        });

        HeadlessResult::success(self.stages_completed.clone(), stage0_info)
    }

    /// Run Stage0 context injection
    fn run_stage0(&self) -> Result<Stage0ExecutionResult, HeadlessError> {
        tracing::info!("Running Stage0 context injection");

        // Read spec content
        let spec_content = self.read_spec_content()?;

        // Spawn Stage0 async
        let stage0_config = Stage0ExecutionConfig::default();
        let pending = spawn_stage0_async(
            self.planner_config.clone(),
            self.spec_id.clone(),
            spec_content,
            self.cwd.clone(),
            stage0_config,
        );

        // Block on result (headless mode can wait synchronously)
        let start = Instant::now();
        loop {
            // Check timeout
            if start.elapsed() > self.config.stage0_timeout {
                return Err(HeadlessError::InfraError(format!(
                    "Stage0 timeout after {}ms",
                    start.elapsed().as_millis()
                )));
            }

            // Drain progress (for logging)
            while let Ok(progress) = pending.progress_rx.try_recv() {
                tracing::debug!(?progress, "Stage0 progress");
            }

            // Check for result
            match pending.result_rx.try_recv() {
                Ok(result) => {
                    tracing::info!(
                        tier2_used = result.tier2_used,
                        duration_ms = result.duration_ms,
                        "Stage0 completed"
                    );
                    return Ok(result);
                }
                Err(mpsc::TryRecvError::Empty) => {
                    std::thread::sleep(Duration::from_millis(100));
                }
                Err(mpsc::TryRecvError::Disconnected) => {
                    return Err(HeadlessError::InfraError(
                        "Stage0 channel disconnected".to_string(),
                    ));
                }
            }
        }
    }

    /// Check guardrails for a stage
    fn check_guardrails(&self, stage: &Stage) -> Result<(), HeadlessError> {
        // TODO: Implement actual guardrail checking
        // For now, always pass (stub)
        tracing::debug!(stage = %stage.as_str(), "Guardrail check passed (stub)");
        Ok(())
    }

    /// Execute a stage (spawn agents, wait for completion)
    fn execute_stage(&self, stage: &Stage) -> Result<(), HeadlessError> {
        // ESCALATED: Agent spawning requires widget-independent infrastructure
        // Return error to prevent false-green tests and status claims
        // Tracked in SPEC-KIT-930 (ARB Pass 2)
        tracing::warn!(
            stage = %stage.as_str(),
            "Stage execution not implemented (escalated to architect)"
        );
        Err(HeadlessError::InfraError(
            "Agent spawning not implemented - requires architectural decision (SPEC-KIT-930)"
                .to_string(),
        ))
    }

    /// Run quality gate for a stage
    fn run_quality_gate(&self, stage: &Stage) -> Result<(), HeadlessError> {
        // Quality gates may require approval
        // In headless mode, we exit with NEEDS_APPROVAL if approval is required
        // For now, this is a stub that succeeds
        tracing::debug!(stage = %stage.as_str(), "Quality gate passed (stub)");
        Ok(())
    }

    /// Read spec content from filesystem
    fn read_spec_content(&self) -> Result<String, HeadlessError> {
        // Try PRD.md first, then spec.md
        let spec_dir = self.cwd.join("docs").join(&self.spec_id);

        let prd_path = spec_dir.join("PRD.md");
        if prd_path.exists() {
            return std::fs::read_to_string(&prd_path).map_err(|e| {
                HeadlessError::InfraError(format!("Failed to read {}: {}", prd_path.display(), e))
            });
        }

        let spec_path = spec_dir.join("spec.md");
        if spec_path.exists() {
            return std::fs::read_to_string(&spec_path).map_err(|e| {
                HeadlessError::InfraError(format!("Failed to read {}: {}", spec_path.display(), e))
            });
        }

        Err(HeadlessError::InfraError(format!(
            "No spec file found for {} in {}",
            self.spec_id,
            spec_dir.display()
        )))
    }

    /// Guard against prompts in headless mode
    ///
    /// Any code that would prompt the user should call this first.
    /// Returns Err(HeadlessError::PromptAttempted) to enforce D133.
    #[allow(dead_code)]
    fn guard_prompt<T>(&self) -> Result<T, HeadlessError> {
        Err(HeadlessError::PromptAttempted)
    }

    /// Get pre-supplied maieutic answer
    ///
    /// Returns the answer if it exists, or Err(NeedsInput) if missing.
    #[allow(dead_code)]
    fn require_answer(&self, key: &str) -> Result<String, HeadlessError> {
        // MaieuticSpec fields are direct properties, not a HashMap
        match key {
            "goal" => Ok(self.maieutic_spec.goal.clone()),
            _ => Err(HeadlessError::NeedsInput(format!(
                "No pre-supplied answer for: {}",
                key
            ))),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::chatwidget::spec_kit::maieutic::{DelegationBounds, ElicitationMode};
    use chrono::Utc;

    fn test_maieutic() -> MaieuticSpec {
        MaieuticSpec::new(
            "TEST-001".to_string(),
            "test-run".to_string(),
            "Test goal".to_string(),
            vec![],
            vec!["Tests pass".to_string()],
            vec![],
            DelegationBounds::default(),
            ElicitationMode::Headless,
            0,
        )
    }

    #[test]
    fn test_headless_error_exit_codes() {
        assert_eq!(
            HeadlessError::NeedsInput("test".to_string()).exit_code(),
            10
        );
        assert_eq!(
            HeadlessError::NeedsApproval {
                checkpoint: "test".to_string()
            }
            .exit_code(),
            11
        );
        assert_eq!(HeadlessError::PromptAttempted.exit_code(), 13);
        assert_eq!(HeadlessError::InfraError("test".to_string()).exit_code(), 3);
    }

    #[test]
    fn test_headless_result_success() {
        let result = HeadlessResult::success(vec!["plan".to_string()], None);
        assert_eq!(result.exit_code, 0);
        assert!(result.error.is_none());
    }

    #[test]
    fn test_headless_result_error() {
        let result = HeadlessResult::error(
            HeadlessError::NeedsInput("missing goal".to_string()),
            vec![],
        );
        assert_eq!(result.exit_code, 10);
        assert!(result.error.is_some());
    }
}
