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
                display_config_info(widget, spec_id, &config);
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

/// Display configuration information
///
/// Shows current pipeline configuration with stage list, costs, and next steps.
/// TODO: Replace with interactive modal (Phase 4 Task 4.1 full implementation)
fn display_config_info(widget: &mut ChatWidget, spec_id: &str, config: &PipelineConfig) {
    let enabled_count = config.enabled_stages.len();
    let total_stages = 8;

    // Calculate total cost and duration
    let total_cost: f64 = config.enabled_stages.iter().map(|s| s.cost_estimate()).sum();
    let total_duration: u32 = config
        .enabled_stages
        .iter()
        .map(|s| s.duration_estimate())
        .sum();

    // Build stage list
    let stage_list: Vec<String> = config
        .enabled_stages
        .iter()
        .map(|s| format!("  • {} (${:.2}, ~{} min)", s, s.cost_estimate(), s.duration_estimate()))
        .collect();

    let message = format!(
        "📊 Pipeline Configuration: {}\n\n\
         Enabled Stages: {}/{}\n\
         {}\n\n\
         Total Cost: ~${:.2}\n\
         Total Duration: ~{} min\n\n\
         ℹ️  Interactive TUI configurator coming soon!\n\
         For now, edit manually: docs/{}/pipeline.toml\n\n\
         To execute: /speckit.auto {}",
        spec_id,
        enabled_count,
        total_stages,
        stage_list.join("\n"),
        total_cost,
        total_duration,
        spec_id,
        spec_id
    );

    widget.history_push(history_cell::new_background_event(message));
}
