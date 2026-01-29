//! Vision builder event handlers (P93/SPEC-KIT-105)
//!
//! Handles completion and cancellation events from the vision builder modal.
//! Maps vision answers to constitution memories with appropriate types and priorities.
//!
//! Core persistence logic is in vision_core.rs for CLI reuse.

use std::collections::HashMap;

use ratatui::text::Line;

use crate::chatwidget::ChatWidget;
use crate::history_cell::{HistoryCellType, PlainHistoryCell, new_error_event};

use super::vision_core::persist_vision_to_overlay;

/// Called when user completes the vision builder modal with all answers
pub fn on_vision_builder_submitted(widget: &mut ChatWidget, answers: HashMap<String, String>) {
    // Delegate to shared persistence logic (CLI-reusable)
    match persist_vision_to_overlay(&widget.config.cwd, &answers) {
        Ok(result) => {
            // Build result message
            let mut lines = vec![Line::from("Project Vision captured!"), Line::from("")];

            lines.push(Line::from(format!(
                "   Constitution version: {} | Hash: {}",
                result.constitution_version,
                &result.content_hash[..8.min(result.content_hash.len())]
            )));
            lines.push(Line::from(format!(
                "   Stored: {} goals, {} non-goals, {} principles, {} guardrails",
                result.goals_count,
                result.non_goals_count,
                result.principles_count,
                result.guardrails_count
            )));

            if result.cache_invalidated > 0 {
                lines.push(Line::from(format!(
                    "   Cache: {} Tier 2 entries invalidated (P92)",
                    result.cache_invalidated
                )));
            }

            lines.push(Line::from(""));

            if let Some(ref nl_vision_path) = result.projections.nl_vision_path {
                lines.push(Line::from(format!(
                    "   NL_VISION.md: {}",
                    nl_vision_path.display()
                )));
            }

            // Check for pending projectnew flow
            if let Some(ref mut pending) = widget.pending_projectnew {
                if pending.phase == super::commands::projectnew::ProjectNewPhase::VisionPending {
                    // Advance to project intake phase
                    pending.phase =
                        super::commands::projectnew::ProjectNewPhase::ProjectIntakePending;
                    let project_id = pending.project_id.clone();
                    let deep = pending.deep;

                    // Show success message for vision capture
                    lines.push(Line::from(""));
                    lines.push(Line::from("Continuing project setup..."));
                    widget.history_push(PlainHistoryCell::new(lines, HistoryCellType::Notice));

                    // Show project intake modal
                    widget.show_project_intake_modal(project_id, deep);
                    widget.request_redraw();
                    return; // Skip normal "Next steps" message
                }
            }

            lines.push(Line::from(""));
            lines.push(Line::from("Next steps:"));
            lines.push(Line::from(
                "   /speckit.constitution view - Review stored constitution",
            ));
            lines.push(Line::from(
                "   /speckit.constitution sync - Regenerate constitution.md",
            ));
            lines.push(Line::from(
                "   /speckit.new <description> - Create a new spec (gate-ready)",
            ));

            widget.history_push(PlainHistoryCell::new(lines, HistoryCellType::Notice));
            widget.request_redraw();
        }
        Err(e) => {
            widget.history_push(new_error_event(format!("Vision persistence failed: {}", e)));
            widget.request_redraw();

            // If in projectnew flow, abort it
            if widget.pending_projectnew.take().is_some() {
                widget.history_push(PlainHistoryCell::new(
                    vec![
                        Line::from("Project setup aborted due to vision persistence failure."),
                        Line::from(""),
                        Line::from("Project scaffold remains in current directory."),
                        Line::from("To retry: /speckit.vision"),
                    ],
                    HistoryCellType::Notice,
                ));
                widget.request_redraw();
            }
        }
    }
}

/// Called when user cancels the vision builder modal
pub fn on_vision_builder_cancelled(widget: &mut ChatWidget) {
    // Check for pending projectnew - abort flow but keep scaffold
    if widget.pending_projectnew.take().is_some() {
        widget.history_push(PlainHistoryCell::new(
            vec![
                Line::from("Project setup cancelled"),
                Line::from(""),
                Line::from("Project scaffold remains in current directory."),
                Line::from("Vision was not captured."),
                Line::from(""),
                Line::from("To resume setup:"),
                Line::from("   /speckit.vision   - Capture project vision"),
                Line::from("   /speckit.new <desc> - Create a spec"),
            ],
            HistoryCellType::Notice,
        ));
        widget.request_redraw();
        return;
    }

    widget.history_push(PlainHistoryCell::new(
        vec![
            Line::from("Vision capture cancelled"),
            Line::from(""),
            Line::from("To try again: /speckit.vision"),
        ],
        HistoryCellType::Notice,
    ));
    widget.request_redraw();
}
