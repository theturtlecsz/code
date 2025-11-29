//! /speckit.plan command implementation (and other stage commands)
//!
//! FORK-SPECIFIC (just-every/code): Spec-kit multi-agent automation framework
//!
//! SPEC-KIT-902: These commands now use DIRECT agent spawning instead of
//! the orchestrator pattern. They call auto_submit_spec_stage_prompt() directly.

use super::super::super::ChatWidget;
use super::super::command_registry::SpecKitCommand;
use super::super::native_guardrail::run_native_guardrail;
use crate::spec_prompts::SpecStage;

/// Command: /speckit.plan
/// Creates work breakdown with multi-agent consensus
pub struct SpecKitPlanCommand;

impl SpecKitCommand for SpecKitPlanCommand {
    fn name(&self) -> &'static str {
        "speckit.plan"
    }

    fn aliases(&self) -> &[&'static str] {
        &[] // SPEC-KIT-902: Legacy aliases removed
    }

    fn description(&self) -> &'static str {
        "create work breakdown with multi-agent consensus"
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
/// Code generation with multi-agent consensus
pub struct SpecKitImplementCommand;

impl SpecKitCommand for SpecKitImplementCommand {
    fn name(&self) -> &'static str {
        "speckit.implement"
    }

    fn aliases(&self) -> &[&'static str] {
        &[] // SPEC-KIT-902: Legacy aliases removed
    }

    fn description(&self) -> &'static str {
        "write code with multi-agent consensus"
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
/// Test strategy with multi-agent validation
pub struct SpecKitValidateCommand;

impl SpecKitCommand for SpecKitValidateCommand {
    fn name(&self) -> &'static str {
        "speckit.validate"
    }

    fn aliases(&self) -> &[&'static str] {
        &[] // SPEC-KIT-902: Legacy aliases removed
    }

    fn description(&self) -> &'static str {
        "run test strategy with validation"
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
/// Compliance review with multi-agent
pub struct SpecKitAuditCommand;

impl SpecKitCommand for SpecKitAuditCommand {
    fn name(&self) -> &'static str {
        "speckit.audit"
    }

    fn aliases(&self) -> &[&'static str] {
        &[] // SPEC-KIT-902: Legacy aliases removed
    }

    fn description(&self) -> &'static str {
        "compliance review with multi-agent"
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

/// Execute a stage command with direct agent spawning (SPEC-KIT-902)
///
/// 1. Validates SPEC-ID argument
/// 2. Runs native guardrail validation
/// 3. Spawns agents directly via auto_submit_spec_stage_prompt()
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

    // Run native guardrail validation
    let result = run_native_guardrail(&widget.config.cwd, spec_id, stage, false);

    if !result.success {
        // Report guardrail failures
        for error in &result.errors {
            widget.history_push(crate::history_cell::new_error_event(error.clone()));
        }
        return;
    }

    // Report any warnings
    for warning in &result.warnings {
        widget.history_push(crate::history_cell::new_warning_event(warning.clone()));
    }

    // Spawn agents directly (SPEC-KIT-902: eliminates orchestrator)
    super::super::agent_orchestrator::auto_submit_spec_stage_prompt(widget, stage, spec_id);
}
