//! /speckit.configure command implementation
//!
//! SPEC-947 Phase 4: Pipeline UI Configurator - Command Integration
//!
//! Launches interactive TUI modal for pipeline stage selection.
//! Provides visual configuration vs manual TOML editing.

use super::super::super::ChatWidget;
use super::super::command_registry::SpecKitCommand;
use super::super::pipeline_config::PipelineConfig;
use crate::history_cell;

/// Command: /speckit.configure
/// Interactive TUI configurator for pipeline stage selection
pub struct SpecKitConfigureCommand;

impl SpecKitCommand for SpecKitConfigureCommand {
    fn name(&self) -> &'static str {
        "speckit.configure"
    }

    fn aliases(&self) -> &[&'static str] {
        &[]
    }

    fn description(&self) -> &'static str {
        "configure pipeline stages (interactive TUI)"
    }

    fn execute(&self, widget: &mut ChatWidget, args: String) {
        let spec_id = args.trim().split_whitespace().next().unwrap_or("");
        if spec_id.is_empty() {
            widget.history_push(history_cell::new_error_event(
                "Usage: /speckit.configure SPEC-ID".to_string(),
            ));
            return;
        }

        // Load existing configuration (per-SPEC > global > defaults)
        match PipelineConfig::load(spec_id, None) {
            Ok(config) => {
                // Launch interactive modal
                widget.show_pipeline_configurator(spec_id.to_string(), config);
            }
            Err(err) => {
                widget.history_push(history_cell::new_error_event(format!(
                    "Failed to load configuration for {}: {}",
                    spec_id, err
                )));
            }
        }
    }

    fn requires_args(&self) -> bool {
        true
    }

    fn is_prompt_expanding(&self) -> bool {
        false
    }
}
