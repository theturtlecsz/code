//! ProjectNew command - Full project setup with vision and intake
//!
//! FORK-SPECIFIC (just-every/code): /speckit.projectnew command
//!
//! Creates new projects with mandatory vision capture, project intake,
//! and optional bootstrap spec. Orchestrates a multi-phase flow:
//!
//! 1. Scaffold project (reuse create_project())
//! 2. Switch cwd
//! 3. Vision modal (mandatory)
//! 4. Project intake modal (new)
//! 5. Optional bootstrap spec via spec intake modal

use std::path::PathBuf;

use super::super::super::ChatWidget;
use super::super::command_registry::SpecKitCommand;
use super::super::project_native::{ProjectType, create_project};

// =============================================================================
// State Machine for /speckit.projectnew multi-phase flow
// =============================================================================

/// Phases for the projectnew orchestration flow
#[derive(Debug, Clone, PartialEq, Eq)]
#[allow(dead_code)]
pub enum ProjectNewPhase {
    /// Waiting for vision modal to complete
    VisionPending,
    /// Waiting for project intake modal to complete
    ProjectIntakePending,
    /// Waiting for bootstrap spec intake modal to complete
    BootstrapSpecPending,
    /// Flow completed
    Done,
}

/// State for pending /speckit.projectnew multi-phase flow
///
/// Stored on ChatWidget to coordinate across modal completions.
#[derive(Debug, Clone)]
#[allow(dead_code)]
pub struct PendingProjectNew {
    /// Project type (rust, python, typescript, go, generic)
    pub project_type: ProjectType,
    /// Project name
    pub project_name: String,
    /// Project directory path
    pub project_dir: PathBuf,
    /// Project ID (dirname, used for capsule URIs)
    pub project_id: String,
    /// Whether deep intake questions should be asked
    pub deep: bool,
    /// Optional description for bootstrap spec
    pub bootstrap_desc: Option<String>,
    /// Skip bootstrap spec creation
    pub no_bootstrap_spec: bool,
    /// Area for bootstrap spec (required when bootstrap is enabled)
    pub bootstrap_area: Option<String>,
    /// Current phase in the orchestration flow
    pub phase: ProjectNewPhase,
}

// =============================================================================
// Command Implementation
// =============================================================================

/// Command: /speckit.projectnew
/// Create new project with vision, project intake, and optional bootstrap spec
pub struct SpecKitProjectNewCommand;

impl SpecKitCommand for SpecKitProjectNewCommand {
    fn name(&self) -> &'static str {
        "speckit.projectnew"
    }

    fn aliases(&self) -> &[&'static str] {
        &["projectnew"]
    }

    fn description(&self) -> &'static str {
        "scaffold project + vision + project intake + bootstrap spec (INSTANT scaffold, then modals)"
    }

    fn execute(&self, widget: &mut ChatWidget, args: String) {
        use crate::history_cell::{HistoryCellType, PlainHistoryCell, new_error_event};
        use ratatui::text::Line;

        // Parse arguments with shlex for proper quoted string handling
        let parsed = match shlex::split(&args) {
            Some(tokens) => tokens,
            None => {
                widget.history_push(new_error_event(
                    "Failed to parse arguments. Check for unmatched quotes.".to_string(),
                ));
                widget.request_redraw();
                return;
            }
        };

        // Check for help or empty args
        if parsed.is_empty() {
            show_usage(widget);
            return;
        }

        // Extract positional args and flags
        let mut project_type_str: Option<&str> = None;
        let mut project_name: Option<&str> = None;
        let mut deep = false;
        let mut bootstrap_desc: Option<String> = None;
        let mut bootstrap_area: Option<String> = None;
        let mut no_bootstrap_spec = false;

        let mut i = 0;
        while i < parsed.len() {
            let arg = &parsed[i];
            match arg.as_str() {
                "--deep" => {
                    deep = true;
                }
                "--no-bootstrap-spec" => {
                    no_bootstrap_spec = true;
                }
                "--bootstrap" => {
                    i += 1;
                    if i < parsed.len() {
                        bootstrap_desc = Some(parsed[i].clone());
                    } else {
                        widget.history_push(new_error_event(
                            "--bootstrap requires a description argument".to_string(),
                        ));
                        widget.request_redraw();
                        return;
                    }
                }
                "--bootstrap-area" => {
                    i += 1;
                    if i < parsed.len() {
                        let area = &parsed[i];
                        // Validate area format
                        if let Err(e) = super::super::spec_id_generator::validate_area(area) {
                            widget.history_push(new_error_event(format!(
                                "Invalid --bootstrap-area: {}\n\nAvailable areas: {}",
                                e,
                                super::super::spec_id_generator::get_available_areas(
                                    &widget.config.cwd
                                )
                                .join(", ")
                            )));
                            widget.request_redraw();
                            return;
                        }
                        bootstrap_area = Some(area.clone());
                    } else {
                        widget.history_push(new_error_event(
                            "--bootstrap-area requires an AREA argument".to_string(),
                        ));
                        widget.request_redraw();
                        return;
                    }
                }
                _ => {
                    // Positional argument
                    if arg.starts_with('-') {
                        widget.history_push(new_error_event(format!(
                            "Unknown flag: {}. Use --deep, --bootstrap \"desc\", --bootstrap-area <AREA>, or --no-bootstrap-spec",
                            arg
                        )));
                        widget.request_redraw();
                        return;
                    }
                    if project_type_str.is_none() {
                        project_type_str = Some(arg);
                    } else if project_name.is_none() {
                        project_name = Some(arg);
                    } else {
                        widget.history_push(new_error_event(format!(
                            "Unexpected argument: {}. Usage: /speckit.projectnew <type> <name> [options]",
                            arg
                        )));
                        widget.request_redraw();
                        return;
                    }
                }
            }
            i += 1;
        }

        // Validate required positional args
        let Some(type_str) = project_type_str else {
            show_usage(widget);
            return;
        };
        let Some(name) = project_name else {
            show_usage(widget);
            return;
        };

        // Parse project type
        let project_type = match ProjectType::parse(type_str) {
            Some(t) => t,
            None => {
                widget.history_push(new_error_event(format!(
                    "Unknown project type '{}'. Valid types: {}",
                    type_str,
                    ProjectType::valid_types()
                )));
                widget.request_redraw();
                return;
            }
        };

        // Create project scaffold
        let result = match create_project(project_type, name, &widget.config.cwd) {
            Ok(r) => r,
            Err(err) => {
                widget.history_push(new_error_event(format!(
                    "Failed to create project: {}",
                    err
                )));
                widget.request_redraw();
                return;
            }
        };

        let project_dir = result.directory.clone();
        let project_id = result.project_name.clone();

        // Display scaffold success
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

        for file in &result.files_created {
            lines.push(Line::from(format!("      {}", file)));
        }

        lines.extend(vec![
            Line::from(""),
            Line::from("Starting project setup flow..."),
            Line::from("   1. Vision capture (mandatory)"),
            Line::from("   2. Project intake"),
            if no_bootstrap_spec {
                Line::from("   3. (skipped) Bootstrap spec")
            } else {
                Line::from("   3. Bootstrap spec")
            },
        ]);

        widget.history_push(PlainHistoryCell::new(lines, HistoryCellType::Notice));

        // Set up pending state for multi-phase orchestration
        widget.pending_projectnew = Some(PendingProjectNew {
            project_type,
            project_name: name.to_string(),
            project_dir: project_dir.clone(),
            project_id,
            deep,
            bootstrap_desc,
            no_bootstrap_spec,
            bootstrap_area,
            phase: ProjectNewPhase::VisionPending,
        });

        // Switch cwd to new project directory
        widget.switch_cwd(project_dir.clone(), None);

        // Immediately show vision builder modal
        widget.show_vision_builder();

        widget.request_redraw();
    }

    fn requires_args(&self) -> bool {
        false // Show help when no args
    }
}

fn show_usage(widget: &mut ChatWidget) {
    use crate::history_cell::{HistoryCellType, PlainHistoryCell};
    use ratatui::text::Line;

    widget.history_push(PlainHistoryCell::new(
        vec![
            Line::from("Usage: /speckit.projectnew <type> <name> [options]"),
            Line::from(""),
            Line::from("Creates project with mandatory vision capture and project intake."),
            Line::from(""),
            Line::from("Types:"),
            Line::from("  rust       - Cargo workspace with src/lib.rs"),
            Line::from("  python     - pyproject.toml with src/<name>/"),
            Line::from("  typescript - package.json with src/index.ts"),
            Line::from("  go         - go.mod with main.go"),
            Line::from("  generic    - Spec-kit files only (no language setup)"),
            Line::from(""),
            Line::from("Options:"),
            Line::from("  --deep                     Enable deep intake questions"),
            Line::from("  --bootstrap \"desc\"         Description for bootstrap spec"),
            Line::from(
                "  --bootstrap-area <AREA>    Area for bootstrap spec (required with --bootstrap)",
            ),
            Line::from("  --no-bootstrap-spec        Skip bootstrap spec creation"),
            Line::from(""),
            Line::from("Examples:"),
            Line::from("  /speckit.projectnew rust my-lib"),
            Line::from("  /speckit.projectnew python my-app --deep"),
            Line::from(
                "  /speckit.projectnew ts my-svc --bootstrap \"add auth\" --bootstrap-area CORE",
            ),
            Line::from("  /speckit.projectnew generic docs-only --no-bootstrap-spec"),
            Line::from(""),
            Line::from("Flow:"),
            Line::from("  1. Scaffold project (instant, $0)"),
            Line::from("  2. Switch to project directory"),
            Line::from("  3. Vision modal (mandatory)"),
            Line::from("  4. Project intake modal"),
            Line::from("  5. Bootstrap spec intake (unless --no-bootstrap-spec)"),
            Line::from(""),
            Line::from("Capsule storage:"),
            Line::from("  mv2://default/project/<name>/artifact/intake/..."),
        ],
        HistoryCellType::Notice,
    ));
    widget.request_redraw();
}
