//! WP-A: Projections Commands
//!
//! Commands for managing filesystem projections:
//! - `/speckit.projections rebuild` - Rebuild filesystem artifacts from capsule/OverlayDb SoR
//!
//! ## Design
//! - Capsule is Source of Record for spec/project intakes
//! - OverlayDb is Source of Record for vision (constitution memories)
//! - Filesystem artifacts (docs/, memory/) are rebuildable projections

use super::super::super::ChatWidget;
use super::super::command_registry::SpecKitCommand;
use super::super::rebuild_projections::{RebuildRequest, rebuild_projections};
use crate::history_cell::{HistoryCellType, PlainHistoryCell};
use ratatui::text::Line;

// =============================================================================
// speckit.projections
// =============================================================================

/// Command: /speckit.projections [rebuild]
/// Manage filesystem projections.
pub struct ProjectionsCommand;

impl SpecKitCommand for ProjectionsCommand {
    fn name(&self) -> &'static str {
        "speckit.projections"
    }

    fn aliases(&self) -> &[&'static str] {
        &["projections"]
    }

    fn description(&self) -> &'static str {
        "manage filesystem projections (rebuild from SoR)"
    }

    fn execute(&self, widget: &mut ChatWidget, args: String) {
        let parts: Vec<&str> = args.split_whitespace().collect();
        let subcommand = parts.first().copied().unwrap_or("help");

        match subcommand {
            "rebuild" => execute_rebuild(widget, &parts[1..]),
            _ => show_help(widget),
        }
    }

    fn requires_args(&self) -> bool {
        false
    }

    fn is_prompt_expanding(&self) -> bool {
        false
    }
}

// =============================================================================
// Command implementations
// =============================================================================

fn show_help(widget: &mut ChatWidget) {
    let lines = vec![
        Line::from("📁 Projections Commands (WP-A: Filesystem Is Projection)"),
        Line::from(""),
        Line::from("/speckit.projections rebuild [options]"),
        Line::from("  Rebuild filesystem artifacts from capsule/OverlayDb SoR"),
        Line::from(""),
        Line::from("Options:"),
        Line::from("  --spec <SPEC-ID>     Rebuild only this spec"),
        Line::from("  --project <ID>       Rebuild only this project"),
        Line::from("  --no-vision          Skip vision rebuild from OverlayDb"),
        Line::from("  --dry-run            List files without writing"),
        Line::from(""),
        Line::from("Examples:"),
        Line::from("  /speckit.projections rebuild"),
        Line::from("  /speckit.projections rebuild --spec SPEC-KIT-042"),
        Line::from("  /speckit.projections rebuild --dry-run"),
        Line::from(""),
        Line::from("Sources:"),
        Line::from("  - Spec/Project projections: Capsule IntakeCompleted events"),
        Line::from("  - Vision projections: OverlayDb constitution memories"),
    ];
    widget.history_push(PlainHistoryCell::new(lines, HistoryCellType::Notice));
}

fn execute_rebuild(widget: &mut ChatWidget, args: &[&str]) {
    // Parse arguments
    let mut request = RebuildRequest::new();
    let mut i = 0;

    while i < args.len() {
        match args[i] {
            "--spec" => {
                if i + 1 < args.len() {
                    request = request.with_spec(args[i + 1].to_string());
                    i += 2;
                } else {
                    let lines = vec![Line::from("❌ --spec requires a SPEC-ID argument")];
                    widget.history_push(PlainHistoryCell::new(lines, HistoryCellType::Error));
                    return;
                }
            }
            "--project" => {
                if i + 1 < args.len() {
                    request = request.with_project(args[i + 1].to_string());
                    i += 2;
                } else {
                    let lines = vec![Line::from("❌ --project requires a project ID argument")];
                    widget.history_push(PlainHistoryCell::new(lines, HistoryCellType::Error));
                    return;
                }
            }
            "--no-vision" => {
                request = request.no_vision();
                i += 1;
            }
            "--dry-run" => {
                request = request.dry_run();
                i += 1;
            }
            _ => {
                let lines = vec![
                    Line::from(format!("❌ Unknown argument: {}", args[i])),
                    Line::from(""),
                    Line::from("Use /speckit.projections help for usage"),
                ];
                widget.history_push(PlainHistoryCell::new(lines, HistoryCellType::Error));
                return;
            }
        }
    }

    // Get working directory
    let cwd = std::env::current_dir().unwrap_or_else(|_| std::path::PathBuf::from("."));

    // Show progress
    let mut lines = vec![
        Line::from("🔄 Rebuilding projections from SoR..."),
        Line::from(""),
    ];
    widget.history_push(PlainHistoryCell::new(
        lines.clone(),
        HistoryCellType::Notice,
    ));

    // Execute rebuild
    match rebuild_projections(&cwd, request) {
        Ok(result) => {
            lines.clear();

            if result.dry_run {
                lines.push(Line::from("📋 Dry-run: Would write files:"));
            } else {
                lines.push(Line::from(format!(
                    "✅ Rebuilt {} files:",
                    result.files_written.len()
                )));
            }
            lines.push(Line::from(""));

            for file in &result.files_written {
                lines.push(Line::from(format!("  • {}", file.display())));
            }

            // Show spec intakes processed
            if !result.spec_intakes.is_empty() {
                lines.push(Line::from(""));
                lines.push(Line::from("Spec intakes:"));
                for spec in &result.spec_intakes {
                    let deep_marker = if spec.deep { " [deep]" } else { "" };
                    lines.push(Line::from(format!(
                        "  • {} (intake: {}...){}",
                        spec.spec_id,
                        &spec.intake_id[..8.min(spec.intake_id.len())],
                        deep_marker
                    )));
                }
            }

            // Show project intakes processed
            if !result.project_intakes.is_empty() {
                lines.push(Line::from(""));
                lines.push(Line::from("Project intakes:"));
                for project in &result.project_intakes {
                    let deep_marker = if project.deep { " [deep]" } else { "" };
                    lines.push(Line::from(format!(
                        "  • {} (intake: {}...){}",
                        project.project_id,
                        &project.intake_id[..8.min(project.intake_id.len())],
                        deep_marker
                    )));
                }
            }

            // Show vision details
            if result.vision_rebuilt {
                lines.push(Line::from(""));
                lines.push(Line::from("Vision (from OverlayDb):"));
                if let Some(ref details) = result.vision_details {
                    lines.push(Line::from(format!(
                        "  Goals: {}, Non-goals: {}, Principles: {}, Guardrails: {}",
                        details.goals_count,
                        details.non_goals_count,
                        details.principles_count,
                        details.guardrails_count
                    )));
                    if let Some(ref note) = details.limitation_note {
                        lines.push(Line::from(format!("  Note: {}", note)));
                    }
                }
            }

            widget.history_push(PlainHistoryCell::new(lines, HistoryCellType::Notice));
        }
        Err(e) => {
            let lines = vec![
                Line::from(format!("❌ Rebuild failed: {}", e)),
                Line::from(""),
                Line::from("Capsule SoR may be missing or corrupted."),
                Line::from("Run /speckit.capsule doctor for diagnostics."),
            ];
            widget.history_push(PlainHistoryCell::new(lines, HistoryCellType::Error));
        }
    }
}
