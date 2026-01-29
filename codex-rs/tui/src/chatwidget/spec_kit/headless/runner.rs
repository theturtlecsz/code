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

use std::path::PathBuf;
use std::sync::mpsc;
use std::time::{Duration, Instant};

use codex_spec_kit::Stage;

use super::backend::{AgentBackend, DefaultAgentBackend};
use super::output::{HeadlessOutput, Stage0Info};
use super::prompt_builder::{build_headless_prompt, get_agents_for_stage};
use crate::chatwidget::spec_kit::maieutic::MaieuticSpec;
use crate::chatwidget::spec_kit::stage0_integration::{
    Stage0ExecutionConfig, Stage0ExecutionResult, spawn_stage0_async,
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
    /// Agent execution backend (mockable for tests)
    backend: Box<dyn AgentBackend>,

    // Internal state
    stages_completed: Vec<String>,
    stage0_result: Option<Stage0ExecutionResult>,
}

impl HeadlessPipelineRunner {
    /// Create a new headless runner with default backend
    ///
    /// Uses `DefaultAgentBackend` which currently returns an error for real
    /// agent spawning (pending SPEC-KIT-930). For tests, use `new_with_backend()`
    /// with a `MockAgentBackend`.
    pub fn new(
        spec_id: String,
        from_stage: Stage,
        to_stage: Stage,
        maieutic_spec: MaieuticSpec,
        config: HeadlessConfig,
        planner_config: codex_core::config::Config,
        cwd: PathBuf,
    ) -> Self {
        let backend = Box::new(DefaultAgentBackend::new(Vec::new()));
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
        let stage_name = stage.as_str();

        // Get agents for this stage from prompts.json
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
        assert!(
            plan_content.contains("gemini"),
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
}
