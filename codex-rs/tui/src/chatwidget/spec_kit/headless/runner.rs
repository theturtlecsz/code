//! Headless pipeline runner for CLI automation (SPEC-KIT-900)
//!
//! Executes the `/speckit.auto` pipeline without a terminal/TUI.
//!
//! ## Exit Codes (D133)
//!
//! - 0: SUCCESS
//! - 2: HARD_FAIL (validation failed, agent failed, stage blocked)
//! - 3: INFRA_ERROR (infrastructure/system errors)
//! - 10: NEEDS_INPUT (missing maieutic)
//! - 11: NEEDS_APPROVAL (checkpoint requires human)
//! - 13: PROMPT_ATTEMPTED (any prompt attempt in headless mode)

use std::path::PathBuf;
use std::sync::mpsc;
use std::time::{Duration, Instant};

use codex_spec_kit::Stage;

use super::backend::{AgentBackend, DefaultAgentBackend};
use super::output::{HeadlessOutput, Stage0Info};
use super::prompt_builder::{build_headless_prompt, get_agents_for_stage};
use crate::chatwidget::spec_kit::maieutic::MaieuticSpec;
use crate::chatwidget::spec_kit::native_guardrail::run_native_guardrail;
use crate::chatwidget::spec_kit::stage0_integration::{
    Stage0ExecutionConfig, Stage0ExecutionResult, spawn_stage0_async,
    write_divine_truth_to_evidence, write_task_brief_to_evidence,
};
use crate::spec_prompts::SpecStage;

/// Exit codes for headless execution (D133)
pub mod exit_codes {
    pub const SUCCESS: i32 = 0;
    /// Hard fail: validation failed, agent failed, or stage blocked
    pub const HARD_FAIL: i32 = 2;
    pub const INFRA_ERROR: i32 = 3;
    pub const NEEDS_INPUT: i32 = 10;
    pub const NEEDS_APPROVAL: i32 = 11;
    pub const PROMPT_ATTEMPTED: i32 = 13;

    pub fn reason(code: i32) -> &'static str {
        match code {
            SUCCESS => "success",
            HARD_FAIL => "hard_fail",
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
    /// Infrastructure/system error (config issues, runtime creation, etc.)
    InfraError(String),
    /// Stage validation failed (exit code 2: hard fail)
    ValidationFailed { stage: String, reason: String },
    /// Agent execution failed (exit code 2: hard fail)
    AgentFailed { agent: String, reason: String },
    /// Stage is blocked and cannot proceed (exit code 2: hard fail)
    StageBlocked { stage: String, reason: String },
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
            Self::AgentFailed { agent, reason } => {
                write!(f, "Agent {} failed: {}", agent, reason)
            }
            Self::StageBlocked { stage, reason } => {
                write!(f, "Stage {} blocked: {}", stage, reason)
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
            // Hard failures (exit code 2): validation, agent, and blocked errors
            Self::ValidationFailed { .. } => exit_codes::HARD_FAIL,
            Self::AgentFailed { .. } => exit_codes::HARD_FAIL,
            Self::StageBlocked { .. } => exit_codes::HARD_FAIL,
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
    /// Agent execution backend (mockable for tests)
    backend: Box<dyn AgentBackend>,

    // Internal state
    stages_completed: Vec<String>,
    stage0_result: Option<Stage0ExecutionResult>,
}

impl HeadlessPipelineRunner {
    /// Create a new headless runner with default backend
    ///
    /// Uses `DefaultAgentBackend` with real agent spawning via AGENT_MANAGER.
    /// For tests, use `new_with_backend()` with a `MockAgentBackend`.
    pub fn new(
        spec_id: String,
        from_stage: Stage,
        to_stage: Stage,
        maieutic_spec: MaieuticSpec,
        config: HeadlessConfig,
        planner_config: codex_core::config::Config,
        cwd: PathBuf,
    ) -> Self {
        // Pass agent configs to enable real agent spawning (SPEC-KIT-900)
        let backend = Box::new(DefaultAgentBackend::new(planner_config.agents.clone()));
        Self::new_with_backend(
            spec_id,
            from_stage,
            to_stage,
            maieutic_spec,
            config,
            planner_config,
            cwd,
            backend,
        )
    }

    /// Create a new headless runner with custom backend
    ///
    /// Use this constructor for testing with a mock backend.
    pub fn new_with_backend(
        spec_id: String,
        from_stage: Stage,
        to_stage: Stage,
        maieutic_spec: MaieuticSpec,
        config: HeadlessConfig,
        planner_config: codex_core::config::Config,
        cwd: PathBuf,
        backend: Box<dyn AgentBackend>,
    ) -> Self {
        Self {
            spec_id,
            from_stage,
            to_stage,
            maieutic_spec,
            config,
            planner_config,
            cwd,
            backend,
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
                // SPEC-KIT-900: Persist Stage0 evidence files if result is available
                if let Some(ref stage0_res) = result.result {
                    // Write TASK_BRIEF.md
                    if let Err(e) = write_task_brief_to_evidence(
                        &self.spec_id,
                        &self.cwd,
                        &stage0_res.task_brief_md,
                    ) {
                        tracing::warn!(
                            error = %e,
                            "Failed to write TASK_BRIEF.md to evidence (non-fatal)"
                        );
                    } else {
                        tracing::info!("Wrote TASK_BRIEF.md to evidence directory");
                    }

                    // Write DIVINE_TRUTH.md
                    if let Err(e) = write_divine_truth_to_evidence(
                        &self.spec_id,
                        &self.cwd,
                        &stage0_res.divine_truth.raw_markdown,
                    ) {
                        tracing::warn!(
                            error = %e,
                            "Failed to write DIVINE_TRUTH.md to evidence (non-fatal)"
                        );
                    } else {
                        tracing::info!("Wrote DIVINE_TRUTH.md to evidence directory");
                    }
                }
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
        // Specify stage has no guardrails (no SpecStage equivalent)
        let Some(spec_stage) = stage_to_spec_stage(stage) else {
            tracing::debug!(stage = %stage.as_str(), "No guardrails for stage");
            return Ok(());
        };

        let result = run_native_guardrail(
            &self.cwd,
            &self.spec_id,
            spec_stage,
            false, // allow_dirty=false - enforces clean tree in headless mode
        );

        if result.success {
            tracing::debug!(
                stage = %stage.as_str(),
                checks = result.checks_run.len(),
                "Guardrail checks passed"
            );
            Ok(())
        } else {
            // Build detailed reason from failed checks
            let failed_checks: Vec<String> = result
                .checks_run
                .iter()
                .filter(|c| {
                    c.status == crate::chatwidget::spec_kit::native_guardrail::CheckStatus::Failed
                })
                .map(|c| {
                    format!(
                        "{}: {}",
                        c.name,
                        c.message.as_deref().unwrap_or("no details")
                    )
                })
                .collect();

            let reason = if !result.errors.is_empty() {
                result.errors.join("; ")
            } else if !failed_checks.is_empty() {
                failed_checks.join("; ")
            } else {
                "Guardrail validation failed (unknown reason)".to_string()
            };

            tracing::warn!(
                stage = %stage.as_str(),
                errors = ?result.errors,
                warnings = ?result.warnings,
                failed_checks = ?failed_checks,
                "Guardrail checks failed"
            );
            Err(HeadlessError::ValidationFailed {
                stage: stage.as_str().to_string(),
                reason,
            })
        }
    }

    /// Execute a stage (spawn agents, wait for completion)
    fn execute_stage(&self, stage: &Stage) -> Result<(), HeadlessError> {
        let stage_name = stage.as_str();

        // D113/D133: Get preferred agent for this stage (GR-001 single-agent routing)
        let agents = get_agents_for_stage(&self.cwd, stage_name)?;

        tracing::info!(
            stage = %stage_name,
            agents = ?agents,
            "Executing stage with {} agents",
            agents.len()
        );

        // Get Stage 0 context if available (combined Divine Truth + Task Brief)
        let stage0_context_owned: Option<String> = self
            .stage0_result
            .as_ref()
            .and_then(|r| r.result.as_ref())
            .map(|s0| s0.combined_context_md());
        let stage0_context = stage0_context_owned.as_deref();

        // Execute each agent and collect outputs
        let mut outputs: Vec<(String, String)> = Vec::new();
        for agent_name in &agents {
            // Build prompt for this agent
            let prompt = build_headless_prompt(
                &self.spec_id,
                stage_name,
                agent_name,
                &self.cwd,
                stage0_context,
            )?;

            tracing::info!(
                agent = %agent_name,
                prompt_len = prompt.len(),
                "Running agent"
            );

            // Run agent via backend
            let output = self.backend.run_stage_agent(
                agent_name,
                prompt,
                &self.spec_id,
                stage_name,
                self.config.agent_timeout,
            )?;

            tracing::info!(
                agent = %agent_name,
                output_len = output.len(),
                "Agent completed"
            );

            outputs.push((agent_name.clone(), output));
        }

        // Write stage output file
        self.write_stage_output(stage, &outputs)?;

        Ok(())
    }

    /// Write stage outputs to the spec directory
    fn write_stage_output(
        &self,
        stage: &Stage,
        outputs: &[(String, String)],
    ) -> Result<(), HeadlessError> {
        let stage_name = stage.as_str();

        // Determine output filename
        let filename = match stage {
            Stage::Plan => "plan.md",
            Stage::Tasks => "tasks.md",
            Stage::Implement => "implement.md",
            Stage::Validate => "validate.md",
            Stage::Audit => "audit.md",
            Stage::Unlock => "unlock.md",
            _ => {
                tracing::warn!(stage = %stage_name, "Unknown stage, skipping output write");
                return Ok(());
            }
        };

        // Find spec directory
        let spec_dir = self.find_spec_dir()?;

        // Format output as markdown
        let content = format!(
            "# {} Stage Output\n\n_Generated by headless pipeline execution_\n\n{}\n",
            stage_name,
            outputs
                .iter()
                .map(|(agent, output)| {
                    format!("## Agent: {}\n\n```json\n{}\n```\n", agent, output)
                })
                .collect::<Vec<_>>()
                .join("\n")
        );

        // Write output file
        let output_path = spec_dir.join(filename);
        std::fs::write(&output_path, &content).map_err(|e| {
            HeadlessError::InfraError(format!("Failed to write {}: {}", output_path.display(), e))
        })?;

        tracing::info!(
            path = %output_path.display(),
            size = content.len(),
            "Wrote stage output"
        );

        Ok(())
    }

    /// Find the spec directory
    fn find_spec_dir(&self) -> Result<PathBuf, HeadlessError> {
        // Try common patterns
        let candidates = [
            self.cwd.join("docs").join(&self.spec_id),
            self.cwd.join(&self.spec_id),
        ];

        for candidate in &candidates {
            if candidate.exists() && candidate.is_dir() {
                return Ok(candidate.clone());
            }
        }

        // Try fuzzy matching
        if let Ok(entries) = std::fs::read_dir(self.cwd.join("docs")) {
            for entry in entries.flatten() {
                let name = entry.file_name().to_string_lossy().to_string();
                if name.starts_with(&self.spec_id) {
                    return Ok(entry.path());
                }
            }
        }

        Err(HeadlessError::InfraError(format!(
            "Could not find spec directory for {}",
            self.spec_id
        )))
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

/// Convert codex_spec_kit::Stage to SpecStage for guardrail validation
///
/// Returns None for stages without a SpecStage equivalent (e.g., Specify).
fn stage_to_spec_stage(stage: &Stage) -> Option<SpecStage> {
    match stage {
        Stage::Plan => Some(SpecStage::Plan),
        Stage::Tasks => Some(SpecStage::Tasks),
        Stage::Implement => Some(SpecStage::Implement),
        Stage::Validate => Some(SpecStage::Validate),
        Stage::Audit => Some(SpecStage::Audit),
        Stage::Unlock => Some(SpecStage::Unlock),
        Stage::Specify => None, // No guardrails for Specify stage
    }
}

#[cfg(test)]
mod tests {
    use super::super::backend::MockAgentBackend;
    use super::*;
    use crate::chatwidget::spec_kit::maieutic::{DelegationBounds, ElicitationMode};
    use std::collections::HashMap;
    use tempfile::TempDir;

    fn test_maieutic() -> MaieuticSpec {
        MaieuticSpec::new(
            "TEST-001".to_string(),
            "test-run".to_string(),
            "Test goal".to_string(),
            vec![],
            vec!["Tests pass".to_string()],
            vec![],
            DelegationBounds::default(),
            ElicitationMode::PreSupplied,
            0,
        )
    }

    /// Setup test directory structure with spec and prompts.json
    fn setup_test_spec(temp: &TempDir, spec_id: &str) {
        // Create spec directory
        let spec_dir = temp.path().join("docs").join(spec_id);
        std::fs::create_dir_all(&spec_dir).unwrap();

        // Create spec.md
        std::fs::write(
            spec_dir.join("spec.md"),
            format!(
                "# {}\n\n## Overview\n\nTest spec for headless execution.\n",
                spec_id
            ),
        )
        .unwrap();

        // Create prompts.json
        let prompts_dir = temp.path().join("docs/spec-kit");
        std::fs::create_dir_all(&prompts_dir).unwrap();
        std::fs::write(
            prompts_dir.join("prompts.json"),
            r#"{
                "spec-plan": {
                    "version": "test",
                    "gemini": {
                        "role": "Researcher",
                        "prompt": "Research ${SPEC_ID}.\n\n${CONTEXT}"
                    }
                },
                "spec-tasks": {
                    "version": "test",
                    "gemini": {
                        "role": "Researcher",
                        "prompt": "Research tasks for ${SPEC_ID}.\n\n${CONTEXT}"
                    }
                }
            }"#,
        )
        .unwrap();
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

    #[test]
    fn test_validation_failed_exit_code_is_hard_fail() {
        let error = HeadlessError::ValidationFailed {
            stage: "plan".to_string(),
            reason: "test".to_string(),
        };
        assert_eq!(error.exit_code(), exit_codes::HARD_FAIL);
        assert_eq!(error.exit_code(), 2);
    }

    #[test]
    fn test_agent_failed_exit_code_is_hard_fail() {
        let error = HeadlessError::AgentFailed {
            agent: "gemini".to_string(),
            reason: "test".to_string(),
        };
        assert_eq!(error.exit_code(), exit_codes::HARD_FAIL);
        assert_eq!(error.exit_code(), 2);
    }

    #[test]
    fn test_stage_blocked_exit_code_is_hard_fail() {
        let error = HeadlessError::StageBlocked {
            stage: "implement".to_string(),
            reason: "missing dependency".to_string(),
        };
        assert_eq!(error.exit_code(), exit_codes::HARD_FAIL);
        assert_eq!(error.exit_code(), 2);
    }

    #[test]
    fn test_infra_error_exit_code_is_infra() {
        let error = HeadlessError::InfraError("test".to_string());
        assert_eq!(error.exit_code(), exit_codes::INFRA_ERROR);
        assert_eq!(error.exit_code(), 3);
    }

    #[test]
    fn test_timeout_exit_code_is_infra() {
        let error = HeadlessError::Timeout {
            expected: 2,
            completed: 1,
            elapsed_ms: 30000,
        };
        assert_eq!(error.exit_code(), exit_codes::INFRA_ERROR);
        assert_eq!(error.exit_code(), 3);
    }

    /// Create a minimal Config for testing purposes
    fn create_test_config(codex_home: &std::path::Path) -> codex_core::config::Config {
        use codex_core::config::{Config, ConfigOverrides};

        // Create minimal config.toml
        let config_path = codex_home.join("config.toml");
        std::fs::write(&config_path, "").unwrap();

        // Set CODEX_HOME env var temporarily and load config
        // SAFETY: This is only used in tests and we're not running tests in parallel
        // that would race on this env var
        unsafe {
            std::env::set_var("CODEX_HOME", codex_home);
        }
        Config::load_with_cli_overrides(vec![], ConfigOverrides::default())
            .expect("Failed to load test config")
    }

    #[test]
    fn test_execute_stage_with_mock_backend() {
        // Setup test directory
        let temp = TempDir::new().unwrap();
        setup_test_spec(&temp, "TEST-001");

        // Create a temp CODEX_HOME for config
        let codex_home = TempDir::new().unwrap();
        let planner_config = create_test_config(codex_home.path());

        // Create mock backend with plan response
        let mut responses = HashMap::new();
        responses.insert(
            "plan".to_string(),
            r#"{"stage":"spec-plan","agent":"mock","research_summary":[]}"#.to_string(),
        );
        let backend = MockAgentBackend::with_responses(responses);

        // Create runner with mock backend
        let mut runner = HeadlessPipelineRunner::new_with_backend(
            "TEST-001".to_string(),
            Stage::Plan,
            Stage::Plan,
            test_maieutic(),
            HeadlessConfig {
                stage0_timeout: Duration::from_secs(1),
                agent_timeout: Duration::from_secs(30),
                quality_gates_enabled: false,
                json_output: true,
            },
            planner_config,
            temp.path().to_path_buf(),
            Box::new(backend),
        );

        // Execute just the stage (skip Stage0 by manually calling execute_stage)
        let stage = Stage::Plan;
        let result = runner.execute_stage(&stage);

        // Verify stage execution succeeded
        assert!(
            result.is_ok(),
            "execute_stage failed: {:?}",
            result.unwrap_err()
        );

        // Verify plan.md was created
        let plan_md = temp.path().join("docs/TEST-001/plan.md");
        assert!(
            plan_md.exists(),
            "plan.md should exist at {}",
            plan_md.display()
        );

        // Verify plan.md contains agent output
        let plan_content = std::fs::read_to_string(&plan_md).unwrap();
        assert!(
            plan_content.contains("spec-plan"),
            "plan.md should contain agent output"
        );
        // SPEC-KIT-981: Default agent changed from gemini to gpt_pro
        assert!(
            plan_content.contains("gpt_pro"),
            "plan.md should contain agent name"
        );
    }

    #[test]
    fn test_execute_stage_returns_error_when_no_mock_response() {
        // Setup test directory
        let temp = TempDir::new().unwrap();
        setup_test_spec(&temp, "TEST-002");

        // Create a temp CODEX_HOME for config
        let codex_home = TempDir::new().unwrap();
        let planner_config = create_test_config(codex_home.path());

        // Create mock backend WITHOUT plan response
        let backend = MockAgentBackend::with_responses(HashMap::new());

        let mut runner = HeadlessPipelineRunner::new_with_backend(
            "TEST-002".to_string(),
            Stage::Plan,
            Stage::Plan,
            test_maieutic(),
            HeadlessConfig::default(),
            planner_config,
            temp.path().to_path_buf(),
            Box::new(backend),
        );

        // Execute stage (should fail because no mock response)
        let result = runner.execute_stage(&Stage::Plan);

        // Verify it failed with InfraError
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(
            matches!(err, HeadlessError::InfraError(_)),
            "Expected InfraError, got {:?}",
            err
        );
    }

    #[test]
    fn test_check_guardrails_fails_without_spec_file() {
        // Setup temp directory WITHOUT spec files
        let temp = TempDir::new().unwrap();
        let spec_id = "SPEC-TEST-GUARDRAIL";

        // Don't create spec directory/files - guardrail should fail

        let codex_home = TempDir::new().unwrap();
        let planner_config = create_test_config(codex_home.path());

        let backend = MockAgentBackend::with_default_responses();
        let runner = HeadlessPipelineRunner::new_with_backend(
            spec_id.to_string(),
            Stage::Plan,
            Stage::Tasks,
            test_maieutic(),
            HeadlessConfig::default(),
            planner_config,
            temp.path().to_path_buf(),
            Box::new(backend),
        );

        // Guardrail should fail - no spec.md exists
        let result = runner.check_guardrails(&Stage::Plan);
        assert!(
            result.is_err(),
            "Expected guardrail to fail without spec.md"
        );

        let err = result.unwrap_err();
        assert_eq!(
            err.exit_code(),
            exit_codes::HARD_FAIL,
            "Expected exit code 2 (HARD_FAIL)"
        );

        // Verify error is ValidationFailed
        match err {
            HeadlessError::ValidationFailed { stage, reason: _ } => {
                assert_eq!(stage, "plan");
            }
            _ => panic!("Expected ValidationFailed, got {:?}", err),
        }
    }

    #[test]
    fn test_check_guardrails_passes_with_valid_spec() {
        // Setup temp directory WITH spec files
        let temp = TempDir::new().unwrap();
        let spec_id = "SPEC-TEST-GUARDRAIL-PASS";
        setup_test_spec(&temp, spec_id);

        let codex_home = TempDir::new().unwrap();
        let planner_config = create_test_config(codex_home.path());

        let backend = MockAgentBackend::with_default_responses();
        let runner = HeadlessPipelineRunner::new_with_backend(
            spec_id.to_string(),
            Stage::Plan,
            Stage::Tasks,
            test_maieutic(),
            HeadlessConfig::default(),
            planner_config,
            temp.path().to_path_buf(),
            Box::new(backend),
        );

        // Guardrail should pass with valid spec.md
        let result = runner.check_guardrails(&Stage::Plan);
        assert!(
            result.is_ok(),
            "Expected guardrail to pass with valid spec: {:?}",
            result.unwrap_err()
        );
    }

    #[test]
    fn test_stage_to_spec_stage_mapping() {
        use super::stage_to_spec_stage;

        // Regular stages should map
        assert!(stage_to_spec_stage(&Stage::Plan).is_some());
        assert!(stage_to_spec_stage(&Stage::Tasks).is_some());
        assert!(stage_to_spec_stage(&Stage::Implement).is_some());
        assert!(stage_to_spec_stage(&Stage::Validate).is_some());
        assert!(stage_to_spec_stage(&Stage::Audit).is_some());
        assert!(stage_to_spec_stage(&Stage::Unlock).is_some());

        // Specify stage should return None (no guardrails)
        assert!(stage_to_spec_stage(&Stage::Specify).is_none());
    }

    /// Test that Stage0 evidence files can be written correctly (SPEC-KIT-900)
    ///
    /// Verifies:
    /// - TASK_BRIEF.md is written to evidence directory
    /// - DIVINE_TRUTH.md is written to evidence directory
    /// - Files have expected minimum length
    /// - Guardrail clean-tree passes with evidence/ directory present
    #[test]
    fn test_stage0_evidence_file_persistence() {
        use super::{write_divine_truth_to_evidence, write_task_brief_to_evidence};
        use std::process::Command;

        // Setup temp directory as a git repo (for guardrail clean-tree check)
        let temp = TempDir::new().unwrap();
        let spec_id = "SPEC-EVIDENCE-TEST";
        setup_test_spec(&temp, spec_id);

        // Initialize git repo
        Command::new("git")
            .args(["init"])
            .current_dir(temp.path())
            .output()
            .expect("Failed to init git repo");

        // Add and commit initial files
        Command::new("git")
            .args(["add", "."])
            .current_dir(temp.path())
            .output()
            .expect("Failed to git add");

        Command::new("git")
            .args(["commit", "-m", "Initial commit"])
            .current_dir(temp.path())
            .output()
            .expect("Failed to git commit");

        // Create test content exceeding 500 bytes
        let task_brief_content = format!(
            "# Task Brief: {}\n\n\
            ## Section 0: Constitution Context\n\n\
            The project constitution defines core principles and guardrails.\n\n\
            ## Section 1: Spec Overview\n\n\
            This spec implements a test feature for validation purposes.\n\n\
            ## Section 2: Historical Context\n\n\
            Previous work on similar features established key patterns.\n\n\
            ## Section 3: Code Context\n\n\
            Relevant code sections include the main module and test suite.\n\n\
            ## Section 4: Local Memory Insights\n\n\
            Historical decisions and patterns from local-memory are included here.\n\n\
            Additional padding to ensure we exceed 500 bytes for validation.\n",
            spec_id
        );

        let divine_truth_content = format!(
            "# Divine Truth Brief: {}\n\n\
            ## 1. Executive Summary\n\n\
            This feature adds important functionality to the system.\n\n\
            ## 2. Constitution Alignment\n\n\
            **Aligned with:** Core architectural principles\n\
            **Potential conflicts:** None identified\n\n\
            ## 3. Architectural Guardrails\n\n\
            - Maintain backward compatibility\n\
            - Follow existing patterns\n\
            - Add comprehensive tests\n\n\
            ## 4. Historical Context & Lessons\n\n\
            Previous implementations have established clear patterns.\n\n\
            ## 5. Risks & Open Questions\n\n\
            - Risk: Integration complexity\n\
            - Open: Performance implications\n\n\
            ## 6. Suggested Causal Links\n\n\
            ```json\n[]\n```\n",
            spec_id
        );

        // Write TASK_BRIEF.md
        let task_brief_path =
            write_task_brief_to_evidence(spec_id, temp.path(), &task_brief_content)
                .expect("Failed to write TASK_BRIEF.md");

        // Verify TASK_BRIEF.md exists and has correct length
        assert!(task_brief_path.exists(), "TASK_BRIEF.md should exist");
        let task_brief_len = std::fs::read_to_string(&task_brief_path)
            .expect("Failed to read TASK_BRIEF.md")
            .len();
        assert!(
            task_brief_len > 500,
            "TASK_BRIEF.md should be > 500 bytes, got {}",
            task_brief_len
        );

        // Write DIVINE_TRUTH.md
        let divine_truth_path =
            write_divine_truth_to_evidence(spec_id, temp.path(), &divine_truth_content)
                .expect("Failed to write DIVINE_TRUTH.md");

        // Verify DIVINE_TRUTH.md exists and has correct length
        assert!(divine_truth_path.exists(), "DIVINE_TRUTH.md should exist");
        let divine_truth_len = std::fs::read_to_string(&divine_truth_path)
            .expect("Failed to read DIVINE_TRUTH.md")
            .len();
        assert!(
            divine_truth_len > 500,
            "DIVINE_TRUTH.md should be > 500 bytes, got {}",
            divine_truth_len
        );

        // Verify evidence directory structure
        let evidence_dir = temp.path().join("docs").join(spec_id).join("evidence");
        assert!(evidence_dir.exists(), "evidence/ directory should exist");
        assert!(
            evidence_dir.join("TASK_BRIEF.md").exists(),
            "TASK_BRIEF.md should be in evidence/"
        );
        assert!(
            evidence_dir.join("DIVINE_TRUTH.md").exists(),
            "DIVINE_TRUTH.md should be in evidence/"
        );

        // Verify guardrail clean-tree check passes with evidence/ present
        // The evidence directory should be excluded from dirty-tree checks
        let codex_home = TempDir::new().unwrap();
        let planner_config = create_test_config(codex_home.path());

        let backend = MockAgentBackend::with_default_responses();
        let runner = HeadlessPipelineRunner::new_with_backend(
            spec_id.to_string(),
            Stage::Plan,
            Stage::Tasks,
            test_maieutic(),
            HeadlessConfig::default(),
            planner_config,
            temp.path().to_path_buf(),
            Box::new(backend),
        );

        // Guardrail should pass - evidence files are in evidence/ which is excluded
        let result = runner.check_guardrails(&Stage::Plan);
        assert!(
            result.is_ok(),
            "Guardrail should pass with evidence/ directory: {:?}",
            result.unwrap_err()
        );
    }
}
