//! Spec-Kit command entry points
//!
//! This module contains the top-level command handlers that serve as entry points
//! for spec-kit slash commands. These handlers parse arguments, validate input,
//! and delegate to specialized modules for actual implementation.
//!
//! **Command Handlers:**
//! - `/speckit.status` → handle_spec_status (native dashboard)
//! - `/spec-consensus` → handle_spec_consensus (inspect consensus artifacts)
//! - `/guardrail.*` → handle_guardrail (guardrail validation)
//! - Pipeline errors → halt_spec_auto_with_error (error handling)

use super::super::ChatWidget;
use super::context::SpecKitContext;
use super::state::ValidateCompletionReason;
use crate::app_event::BackgroundPlacement;
use crate::history_cell::HistoryCellType;
use crate::slash_command::{HalMode, SlashCommand};
use crate::spec_status::{SpecStatusArgs, collect_report, degraded_warning, render_dashboard};

/// Handle /speckit.status command (native dashboard)
///
/// Displays spec-kit status dashboard with:
/// - Active specs and their stages
/// - Evidence health (conflicts, oversized, stale, missing docs)
/// - HAL validation status
/// - Degradation warnings
pub fn handle_spec_status(widget: &mut ChatWidget, raw_args: String) {
    let trimmed = raw_args.trim();
    let args = match SpecStatusArgs::from_input(trimmed) {
        Ok(args) => args,
        Err(err) => {
            widget.history_push(crate::history_cell::new_error_event(err.to_string()));
            widget.request_redraw();
            return;
        }
    };

    match collect_report(&widget.config.cwd, args) {
        Ok(report) => {
            let mut lines = render_dashboard(&report);
            if let Some(warning) = degraded_warning(&report) {
                lines.insert(1, warning);
            }
            let message = lines.join("\n");
            widget.insert_background_event_with_placement(message, BackgroundPlacement::Tail);
            widget.request_redraw();
        }
        Err(err) => {
            widget.history_push(crate::history_cell::new_error_event(format!(
                "spec-status failed: {err}"
            )));
            widget.request_redraw();
        }
    }
}

/// Halt /speckit.auto pipeline with error message
///
/// FORK-SPECIFIC (just-every/code): FR3 cancellation cleanup for SPEC-KIT-069
///
/// Displays error message with resume hint and cleans up:
/// 1. Active validate lifecycle state (if present)
/// 2. spec_auto_state
/// 3. Shows resume command hint
///
/// Note: This uses SpecKitContext trait for testability. Full cleanup with
/// telemetry emission requires calling cleanup_spec_auto_with_cancel directly
/// with ChatWidget (which has MCP manager access).
pub fn halt_spec_auto_with_error(widget: &mut impl SpecKitContext, reason: String) {
    // Clean up active validate lifecycle state if present
    if let Some(state) = widget.spec_auto_state().as_ref() {
        if state.validate_lifecycle.active().is_some() {
            // Clean up the validate lifecycle state (mark as cancelled)
            let _ = state
                .validate_lifecycle
                .reset_active(ValidateCompletionReason::Cancelled);
            // Note: Telemetry emission is handled separately by cleanup_spec_auto_with_cancel
            // when called directly with ChatWidget. When called through trait, telemetry
            // is skipped since trait doesn't expose MCP manager access.
        }
    }

    let resume_hint = widget
        .spec_auto_state()
        .as_ref()
        .and_then(|state| {
            state.current_stage().map(|stage| {
                format!(
                    "/speckit.auto {} --from {}",
                    state.spec_id,
                    stage.command_name()
                )
            })
        })
        .unwrap_or_default();

    widget.history_push(crate::history_cell::PlainHistoryCell::new(
        vec![
            ratatui::text::Line::from("⚠ /speckit.auto halted"),
            ratatui::text::Line::from(reason),
            ratatui::text::Line::from(""),
            ratatui::text::Line::from("Resume with:"),
            ratatui::text::Line::from(resume_hint),
        ],
        HistoryCellType::Error,
    ));

    *widget.spec_auto_state_mut() = None;
}

/// Handle /spec-consensus command (inspect consensus artifacts)
///
/// Thin wrapper that delegates to consensus_coordinator for implementation.
/// Kept separate for potential future middleware/hooks.
pub fn handle_spec_consensus(widget: &mut ChatWidget, raw_args: String) {
    super::consensus_coordinator::handle_spec_consensus_impl(widget, raw_args);
}

/// Handle /guardrail.* and /spec-ops-* commands (guardrail validation)
///
/// Delegates to guardrail module for actual implementation.
/// This handler just provides the entry point routing.
pub fn handle_guardrail(
    widget: &mut ChatWidget,
    command: crate::slash_command::SlashCommand,
    raw_args: String,
    hal_override: Option<crate::slash_command::HalMode>,
) {
    // Delegate to guardrail module implementation
    super::guardrail::handle_guardrail_impl(widget, command, raw_args, hal_override);
}
