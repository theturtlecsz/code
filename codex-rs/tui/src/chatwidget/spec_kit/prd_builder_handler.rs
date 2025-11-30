//! PRD builder event handlers (SPEC-KIT-970)
//!
//! Handles completion and cancellation events from the PRD builder modal.

use std::collections::HashMap;

use ratatui::text::Line;

use crate::chatwidget::ChatWidget;
use crate::history_cell::{new_error_event, HistoryCellType, PlainHistoryCell};

/// Called when user completes the PRD builder modal with all answers
pub fn on_prd_builder_submitted(
    widget: &mut ChatWidget,
    description: String,
    answers: HashMap<String, String>,
) {
    // Extract answers
    let problem = answers.get("Problem").cloned().unwrap_or_default();
    let target = answers.get("Target").cloned().unwrap_or_default();
    let success = answers.get("Success").cloned().unwrap_or_default();

    // Build enhanced description incorporating answers
    let enhanced_description = format!(
        "{}\n\n## Problem\n{}\n\n## Target User\n{}\n\n## Success Criteria\n{}",
        description, problem, target, success
    );

    // Create SPEC with enhanced description
    match super::new_native::create_spec_with_context(
        &description,
        &enhanced_description,
        &widget.config.cwd,
    ) {
        Ok(result) => {
            widget.history_push(PlainHistoryCell::new(
                vec![
                    Line::from(format!(
                        "✅ Created {}: {}",
                        result.spec_id, result.feature_name
                    )),
                    Line::from(""),
                    Line::from(format!(
                        "   Directory: docs/{}/",
                        result.directory.file_name().unwrap().to_string_lossy()
                    )),
                    Line::from(format!(
                        "   Files created: {}",
                        result.files_created.join(", ")
                    )),
                    Line::from("   Updated: SPEC.md tracker".to_string()),
                    Line::from(""),
                    Line::from("PRD Context:"),
                    Line::from(format!("   • Problem: {}", problem)),
                    Line::from(format!("   • Target: {}", target)),
                    Line::from(format!("   • Success: {}", success)),
                    Line::from(""),
                    Line::from("Next steps:"),
                    Line::from(format!(
                        "   • Run /speckit.clarify {} to resolve ambiguities",
                        result.spec_id
                    )),
                    Line::from(format!(
                        "   • Run /speckit.analyze {} to check consistency",
                        result.spec_id
                    )),
                    Line::from(format!(
                        "   • Run /speckit.auto {} to generate full implementation",
                        result.spec_id
                    )),
                    Line::from(""),
                    Line::from("Cost savings: $0.15 → $0 (100% reduction, zero agents used)"),
                ],
                HistoryCellType::Notice,
            ));
        }
        Err(err) => {
            widget.history_push(new_error_event(format!("Failed to create SPEC: {}", err)));
        }
    }
    widget.request_redraw();
}

/// Called when user cancels the PRD builder modal
pub fn on_prd_builder_cancelled(widget: &mut ChatWidget, description: String) {
    widget.history_push(PlainHistoryCell::new(
        vec![
            Line::from("❌ SPEC creation cancelled"),
            Line::from(format!("   Description: {}", description)),
            Line::from(""),
            Line::from("To try again: /speckit.new <description>"),
        ],
        HistoryCellType::Notice,
    ));
    widget.request_redraw();
}
