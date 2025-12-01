//! Project scaffolding command
//!
//! FORK-SPECIFIC (just-every/code): SPEC-KIT-960 - /speckit.project command
//!
//! Creates new projects with spec-kit workflow infrastructure.
//! Tier 0: Native Rust, $0, <1s.

use super::super::super::ChatWidget;
use super::super::command_registry::SpecKitCommand;
use super::super::project_native::{ProjectType, create_project};

/// Command: /speckit.project
/// Create new project with spec-kit workflow infrastructure
pub struct SpecKitProjectCommand;

impl SpecKitCommand for SpecKitProjectCommand {
    fn name(&self) -> &'static str {
        "speckit.project"
    }

    fn aliases(&self) -> &[&'static str] {
        &["project"]
    }

    fn description(&self) -> &'static str {
        "scaffold new project with spec-kit workflow support (INSTANT, zero agents, $0)"
    }

    fn execute(&self, widget: &mut ChatWidget, args: String) {
        use crate::history_cell::{HistoryCellType, PlainHistoryCell};
        use ratatui::text::Line;

        let args = args.trim();

        // Parse arguments: <type> <name>
        let parts: Vec<&str> = args.split_whitespace().collect();

        if parts.len() < 2 {
            // Show usage help
            widget.history_push(PlainHistoryCell::new(
                vec![
                    Line::from("Usage: /speckit.project <type> <name>"),
                    Line::from(""),
                    Line::from("Types:"),
                    Line::from("  rust       - Cargo workspace with src/lib.rs"),
                    Line::from("  python     - pyproject.toml with src/<name>/"),
                    Line::from("  typescript - package.json with src/index.ts"),
                    Line::from("  generic    - Spec-kit files only (no language setup)"),
                    Line::from(""),
                    Line::from("Examples:"),
                    Line::from("  /speckit.project rust my-rust-lib"),
                    Line::from("  /speckit.project python my-py-app"),
                    Line::from("  /speckit.project ts my-ts-lib"),
                    Line::from("  /speckit.project generic minimal-spec"),
                    Line::from(""),
                    Line::from("Created structure:"),
                    Line::from("  <name>/"),
                    Line::from("  +-- CLAUDE.md              # Project instructions"),
                    Line::from("  +-- SPEC.md                # Task tracker"),
                    Line::from("  +-- docs/                  # SPEC directories"),
                    Line::from("  +-- memory/constitution.md # Project charter"),
                    Line::from("  +-- [type-specific files]"),
                ],
                HistoryCellType::Notice,
            ));
            widget.request_redraw();
            return;
        }

        let type_str = parts[0];
        let name = parts[1];

        // Parse project type
        let project_type = match ProjectType::from_str(type_str) {
            Some(t) => t,
            None => {
                widget.history_push(crate::history_cell::new_error_event(format!(
                    "Unknown project type '{}'. Valid types: {}",
                    type_str,
                    ProjectType::valid_types()
                )));
                widget.request_redraw();
                return;
            }
        };

        // Create project
        match create_project(project_type, name, &widget.config.cwd) {
            Ok(result) => {
                let project_dir = result.directory.clone();

                let mut lines = vec![
                    Line::from(format!(
                        "✓ Created {} project: {}",
                        result.project_type.display_name(),
                        result.project_name
                    )),
                    Line::from(""),
                    Line::from(format!("   Directory: {}", result.directory.display())),
                    Line::from(format!("   Files created: {}", result.files_created.len())),
                ];

                // List files
                for file in &result.files_created {
                    lines.push(Line::from(format!("      {}", file)));
                }

                lines.extend(vec![
                    Line::from(""),
                    Line::from("Switching to project directory..."),
                    Line::from(""),
                    Line::from("Next: /speckit.new <feature description>"),
                    Line::from(""),
                    Line::from("Cost: $0 (zero agents, instant)"),
                ]);

                widget.history_push(PlainHistoryCell::new(lines, HistoryCellType::Notice));

                // Auto-switch to the new project directory
                widget
                    .app_event_tx
                    .send(crate::app_event::AppEvent::SwitchCwd(project_dir, None));
            }
            Err(err) => {
                widget.history_push(crate::history_cell::new_error_event(format!(
                    "Failed to create project: {}",
                    err
                )));
            }
        }
        widget.request_redraw();
    }

    fn requires_args(&self) -> bool {
        false // Show help when no args
    }
}
