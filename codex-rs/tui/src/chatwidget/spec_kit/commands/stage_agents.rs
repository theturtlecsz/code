//! /speckit.stage-agents command implementation
//!
//! SPEC-KIT-983: Stage→agent defaults UI
//!
//! Opens TUI modal to configure which agent handles each pipeline stage.
//! Changes persist to root config.toml under [speckit.stage_agents],
//! ensuring Tier-1 parity between TUI and CLI/headless modes.

use super::super::super::ChatWidget;
use super::super::command_registry::SpecKitCommand;

/// Command: /speckit.stage-agents
/// Interactive TUI modal for stage→agent configuration
pub struct SpecKitStageAgentsCommand;

impl SpecKitCommand for SpecKitStageAgentsCommand {
    fn name(&self) -> &'static str {
        "speckit.stage-agents"
    }

    fn aliases(&self) -> &[&'static str] {
        &[]
    }

    fn description(&self) -> &'static str {
        "configure stage→agent defaults (TUI modal)"
    }

    fn execute(&self, widget: &mut ChatWidget, _args: String) {
        widget.show_spec_kit_stage_agents_modal();
    }

    fn requires_args(&self) -> bool {
        false
    }

    fn is_prompt_expanding(&self) -> bool {
        false
    }
}
