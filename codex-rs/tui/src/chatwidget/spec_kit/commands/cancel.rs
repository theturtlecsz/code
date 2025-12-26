//! /speckit.cancel command implementation
//!
//! FORK-SPECIFIC (just-every/code): Spec-kit multi-agent automation framework
//!
//! SPEC-DOGFOOD-001: Clears stale pipeline state when overlays prevent Esc cancellation.

use super::super::super::ChatWidget;
use super::super::command_registry::SpecKitCommand;
use crate::history_cell::{HistoryCellType, PlainHistoryCell};

/// Command: /speckit.cancel
/// Clears stale pipeline state (spec_auto_state and spec_auto_metrics)
///
/// Use when:
/// - Pipeline is stuck in a broken state
/// - Esc key doesn't clear state (overlays intercept)
/// - Need to restart a failed /speckit.auto run
pub struct SpecKitCancelCommand;

impl SpecKitCommand for SpecKitCancelCommand {
    fn name(&self) -> &'static str {
        "speckit.cancel"
    }

    fn aliases(&self) -> &[&'static str] {
        &["spec-cancel"]
    }

    fn description(&self) -> &'static str {
        "clear stale pipeline state (use when Esc doesn't work)"
    }

    fn execute(&self, widget: &mut ChatWidget, _args: String) {
        handle_speckit_cancel(widget);
    }

    fn requires_args(&self) -> bool {
        false
    }

    fn is_prompt_expanding(&self) -> bool {
        false
    }
}

/// Clear stale pipeline state
///
/// Clears:
/// - spec_auto_state: Active pipeline state
/// - spec_auto_metrics: Status bar token metrics
///
/// Pushes a notice to history confirming the cancellation.
pub fn handle_speckit_cancel(widget: &mut ChatWidget) {
    let had_state = widget.spec_auto_state.is_some();

    // Clear pipeline state
    widget.spec_auto_state = None;

    // Clear status bar metrics
    widget.bottom_pane.set_spec_auto_metrics(None);

    // Push confirmation to history
    let message = if had_state {
        "✓ Pipeline state cleared. Ready for new /speckit.auto run."
    } else {
        "ℹ No active pipeline state to clear."
    };

    widget.history_push(PlainHistoryCell::new(
        vec![ratatui::text::Line::from(message)],
        HistoryCellType::Notice,
    ));

    widget.request_redraw();
}
