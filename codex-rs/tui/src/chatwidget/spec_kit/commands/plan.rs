//! /speckit.plan command implementation (and other stage commands)
//!
//! FORK-SPECIFIC (just-every/code): Spec-kit multi-agent automation framework
//!
//! SPEC-KIT-921 P4-D: Stage commands now use SpeckitExecutor for CLI/TUI parity.
//! The executor validates prerequisites; TUI handles agent spawning.

use super::super::super::ChatWidget;
use super::super::command_registry::SpecKitCommand;
use crate::spec_prompts::SpecStage;

// SPEC-KIT-921: Use shared executor for stage validation (CLI/TUI parity)
use codex_spec_kit::Stage;
use codex_spec_kit::config::policy_toggles::PolicyToggles;
use codex_spec_kit::executor::{
    ExecutionContext, Outcome, PolicySnapshot, SpeckitCommand as ExecutorCommand, SpeckitExecutor,
    StageResolution, TelemetryMode,
};

/// Command: /speckit.plan
/// Creates work breakdown with gate review
pub struct SpecKitPlanCommand;

impl SpecKitCommand for SpecKitPlanCommand {
    fn name(&self) -> &'static str {
        "speckit.plan"
    }

    fn aliases(&self) -> &[&'static str] {
        &[] // SPEC-KIT-902: Legacy aliases removed
    }

    fn description(&self) -> &'static str {
        "create work breakdown with gate review"
    }

    fn execute(&self, widget: &mut ChatWidget, args: String) {
        execute_stage_command(widget, args, SpecStage::Plan, "speckit.plan");
    }

    fn expand_prompt(&self, _args: &str) -> Option<String> {
        None // SPEC-KIT-902: No longer uses orchestrator pattern
    }

    fn requires_args(&self) -> bool {
        true
    }
}

/// Command: /speckit.tasks
/// Generates task list with validation mapping
pub struct SpecKitTasksCommand;

impl SpecKitCommand for SpecKitTasksCommand {
    fn name(&self) -> &'static str {
        "speckit.tasks"
    }

    fn aliases(&self) -> &[&'static str] {
        &[] // SPEC-KIT-902: Legacy aliases removed
    }

    fn description(&self) -> &'static str {
        "generate task list with validation mapping"
    }

    fn execute(&self, widget: &mut ChatWidget, args: String) {
        execute_stage_command(widget, args, SpecStage::Tasks, "speckit.tasks");
    }

    fn expand_prompt(&self, _args: &str) -> Option<String> {
        None // SPEC-KIT-902: No longer uses orchestrator pattern
    }

    fn requires_args(&self) -> bool {
        true
    }
}

/// Command: /speckit.implement
/// Code generation with gate review
pub struct SpecKitImplementCommand;

impl SpecKitCommand for SpecKitImplementCommand {
    fn name(&self) -> &'static str {
        "speckit.implement"
    }

    fn aliases(&self) -> &[&'static str] {
        &[] // SPEC-KIT-902: Legacy aliases removed
    }

    fn description(&self) -> &'static str {
        "write code with gate review"
    }

    fn execute(&self, widget: &mut ChatWidget, args: String) {
        execute_stage_command(widget, args, SpecStage::Implement, "speckit.implement");
    }

    fn expand_prompt(&self, _args: &str) -> Option<String> {
        None // SPEC-KIT-902: No longer uses orchestrator pattern
    }

    fn requires_args(&self) -> bool {
        true
    }
}

/// Command: /speckit.validate
/// Test strategy with gate review
pub struct SpecKitValidateCommand;

impl SpecKitCommand for SpecKitValidateCommand {
    fn name(&self) -> &'static str {
        "speckit.validate"
    }

    fn aliases(&self) -> &[&'static str] {
        &[] // SPEC-KIT-902: Legacy aliases removed
    }

    fn description(&self) -> &'static str {
        "run test strategy with gate review"
    }

    fn execute(&self, widget: &mut ChatWidget, args: String) {
        execute_stage_command(widget, args, SpecStage::Validate, "speckit.validate");
    }

    fn expand_prompt(&self, _args: &str) -> Option<String> {
        None // SPEC-KIT-902: No longer uses orchestrator pattern
    }

    fn requires_args(&self) -> bool {
        true
    }
}

/// Command: /speckit.audit
/// Compliance review with gate signals
pub struct SpecKitAuditCommand;

impl SpecKitCommand for SpecKitAuditCommand {
    fn name(&self) -> &'static str {
        "speckit.audit"
    }

    fn aliases(&self) -> &[&'static str] {
        &[] // SPEC-KIT-902: Legacy aliases removed
    }

    fn description(&self) -> &'static str {
        "compliance review with gate signals"
    }

    fn execute(&self, widget: &mut ChatWidget, args: String) {
        execute_stage_command(widget, args, SpecStage::Audit, "speckit.audit");
    }

    fn expand_prompt(&self, _args: &str) -> Option<String> {
        None // SPEC-KIT-902: No longer uses orchestrator pattern
    }

    fn requires_args(&self) -> bool {
        true
    }
}

/// Command: /speckit.unlock
/// Final approval for merge
pub struct SpecKitUnlockCommand;

impl SpecKitCommand for SpecKitUnlockCommand {
    fn name(&self) -> &'static str {
        "speckit.unlock"
    }

    fn aliases(&self) -> &[&'static str] {
        &[] // SPEC-KIT-902: Legacy aliases removed
    }

    fn description(&self) -> &'static str {
        "final approval for merge"
    }

    fn execute(&self, widget: &mut ChatWidget, args: String) {
        execute_stage_command(widget, args, SpecStage::Unlock, "speckit.unlock");
    }

    fn expand_prompt(&self, _args: &str) -> Option<String> {
        None // SPEC-KIT-902: No longer uses orchestrator pattern
    }

    fn requires_args(&self) -> bool {
        true
    }
}

// === Shared Implementation ===

/// Convert TUI SpecStage to spec-kit Stage
///
/// SPEC-KIT-921 P4: Enable executor integration
///
/// Returns None for non-stage items (quality commands) to prevent silent bugs.
/// Callers must handle the None case explicitly.
fn spec_stage_to_stage(stage: SpecStage) -> Option<Stage> {
    match stage {
        SpecStage::Specify => Some(Stage::Specify),
        SpecStage::Plan => Some(Stage::Plan),
        SpecStage::Tasks => Some(Stage::Tasks),
        SpecStage::Implement => Some(Stage::Implement),
        SpecStage::Validate => Some(Stage::Validate),
        SpecStage::Audit => Some(Stage::Audit),
        SpecStage::Unlock => Some(Stage::Unlock),
        // Quality commands don't map to executor stages
        SpecStage::Clarify | SpecStage::Analyze | SpecStage::Checklist => None,
    }
}

/// Execute a stage command with executor validation (SPEC-KIT-921 P4-D)
///
/// 1. Validates SPEC-ID argument
/// 2. Uses SpeckitExecutor for CLI/TUI parity validation
/// 3. If Ready: spawns agents directly via auto_submit_spec_stage_prompt()
/// 4. If Blocked: displays errors in chat
///
/// SPEC-KIT-957: Made public for use by SpecKitSpecifyCommand
pub fn execute_stage_command(
    widget: &mut ChatWidget,
    args: String,
    stage: SpecStage,
    command_name: &str,
) {
    let spec_id = args.split_whitespace().next().unwrap_or("");
    if spec_id.is_empty() {
        widget.history_push(crate::history_cell::new_error_event(format!(
            "Usage: /{} SPEC-ID",
            command_name
        )));
        return;
    }

    // SPEC-KIT-921 P4: Use executor for validation (CLI/TUI parity)
    let executor_stage = match spec_stage_to_stage(stage) {
        Some(s) => s,
        None => {
            // Quality commands (Clarify/Analyze/Checklist) don't use executor
            // Fall back to direct agent spawning without validation
            super::super::agent_orchestrator::auto_submit_spec_stage_prompt(widget, stage, spec_id);
            return;
        }
    };

    // Resolve policy from env/config at adapter boundary (not in executor)
    let toggles = PolicyToggles::from_env_and_config();
    let policy_snapshot = PolicySnapshot {
        sidecar_critic_enabled: toggles.sidecar_critic_enabled,
        telemetry_mode: TelemetryMode::Disabled,
        legacy_voting_env_detected: toggles.legacy_voting_enabled,
    };

    // Create executor with current working directory and resolved policy
    let executor = SpeckitExecutor::new(ExecutionContext {
        repo_root: widget.config.cwd.clone(),
        policy_snapshot: Some(policy_snapshot),
    });

    // Execute validation via shared executor (same path as CLI)
    let command = ExecutorCommand::ValidateStage {
        spec_id: spec_id.to_string(),
        stage: executor_stage,
        dry_run: false, // TUI actually spawns agents
    };

    match executor.execute(command) {
        Outcome::Stage(outcome) => {
            match outcome.resolution {
                StageResolution::Ready => {
                    // Report any warnings
                    for warning in &outcome.advisory_signals {
                        widget
                            .history_push(crate::history_cell::new_warning_event(warning.clone()));
                    }

                    // Spawn agents directly (SPEC-KIT-902: eliminates orchestrator)
                    super::super::agent_orchestrator::auto_submit_spec_stage_prompt(
                        widget, stage, spec_id,
                    );
                }
                StageResolution::Blocked => {
                    // Report blocking errors
                    for error in &outcome.blocking_reasons {
                        widget.history_push(crate::history_cell::new_error_event(error.clone()));
                    }
                }
                StageResolution::Skipped => {
                    // Report skip reason as warning
                    for signal in &outcome.advisory_signals {
                        widget.history_push(crate::history_cell::new_warning_event(signal.clone()));
                    }
                }
            }
        }
        Outcome::Error(err) => {
            widget.history_push(crate::history_cell::new_error_event(format!(
                "{command_name} failed: {err}"
            )));
        }
        // Stage commands never return Status/Review variants
        Outcome::Status(_) | Outcome::Review(_) | Outcome::ReviewSkipped { .. } => {
            unreachable!("Stage command should never return Status/Review outcome")
        }
    }
}
