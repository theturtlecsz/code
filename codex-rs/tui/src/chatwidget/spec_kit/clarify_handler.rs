//! Clarify event handlers (SPEC-KIT-971)
//!
//! Handles completion and cancellation events from the clarify modal.

use ratatui::text::Line;

use crate::bottom_pane::clarify_modal::ClarifyQuestion;
use crate::chatwidget::ChatWidget;
use crate::history_cell::{HistoryCellType, PlainHistoryCell, new_error_event};

/// Called when user completes the clarify modal with all answers
pub fn on_clarify_submitted(
    widget: &mut ChatWidget,
    spec_id: String,
    resolutions: Vec<(ClarifyQuestion, String)>,
) {
    // Convert ClarifyQuestion to ClarificationMarker for the native resolver
    let native_resolutions: Vec<(super::clarify_native::ClarificationMarker, String)> = resolutions
        .iter()
        .map(|(q, answer)| {
            let marker = super::clarify_native::ClarificationMarker {
                id: q.id.clone(),
                question: q.question.clone(),
                file_path: q.file_path.clone(),
                line_number: q.line_number,
                original_text: q.original_text.clone(),
            };
            (marker, answer.clone())
        })
        .collect();

    // Count how many were actually resolved (not skipped)
    let resolved_count = resolutions
        .iter()
        .filter(|(q, a)| a != &q.original_text)
        .count();
    let skipped_count = resolutions.len() - resolved_count;

    // Apply resolutions
    match super::clarify_native::resolve_markers(&native_resolutions) {
        Ok(()) => {
            let mut lines = vec![
                Line::from(format!(
                    "Resolved {} clarification{} in {}",
                    resolved_count,
                    if resolved_count == 1 { "" } else { "s" },
                    spec_id
                )),
                Line::from(""),
            ];

            // Show resolved items
            for (q, answer) in &resolutions {
                if answer != &q.original_text {
                    let filename = q
                        .file_path
                        .file_name()
                        .map(|s| s.to_string_lossy().to_string())
                        .unwrap_or_else(|| "unknown".to_string());
                    lines.push(Line::from(format!(
                        "  {} ({}:{})",
                        q.id, filename, q.line_number
                    )));
                    lines.push(Line::from(format!("    Q: {}", truncate(&q.question, 50))));
                    lines.push(Line::from(format!("    A: {}", truncate(answer, 50))));
                    lines.push(Line::from(""));
                }
            }

            if skipped_count > 0 {
                lines.push(Line::from(format!(
                    "  ({} skipped - markers unchanged)",
                    skipped_count
                )));
                lines.push(Line::from(""));
            }

            lines.push(Line::from(
                "Files updated. Run /speckit.clarify again to check for remaining markers.",
            ));

            widget.history_push(PlainHistoryCell::new(lines, HistoryCellType::Notice));
        }
        Err(err) => {
            widget.history_push(new_error_event(format!(
                "Failed to resolve clarifications: {}",
                err
            )));
        }
    }
    widget.request_redraw();
}

/// Called when user cancels the clarify modal
pub fn on_clarify_cancelled(widget: &mut ChatWidget, spec_id: String) {
    widget.history_push(PlainHistoryCell::new(
        vec![
            Line::from("Clarification cancelled"),
            Line::from(format!("   SPEC: {}", spec_id)),
            Line::from(""),
            Line::from("Markers unchanged. Run /speckit.clarify again when ready."),
        ],
        HistoryCellType::Notice,
    ));
    widget.request_redraw();
}

/// Truncate text to max length with ellipsis
fn truncate(s: &str, max: usize) -> String {
    if s.len() <= max {
        s.to_string()
    } else {
        format!("{}...", &s[..max.saturating_sub(3)])
    }
}
