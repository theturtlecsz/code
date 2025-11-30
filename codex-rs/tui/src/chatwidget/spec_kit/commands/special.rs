//! Special command implementations (auto, new, specify, consensus, constitution)
//!
//! FORK-SPECIFIC (just-every/code): Spec-kit multi-agent automation framework

use super::super::super::ChatWidget;
use super::super::ace_constitution;
use super::super::command_registry::SpecKitCommand;
use super::super::handler;
use super::super::routing::{get_current_branch, get_repo_root};

/// Command: /speckit.auto
/// Full 6-stage pipeline with auto-advancement
/// Note: Legacy /spec-auto alias removed to prevent confusion with subagent routing
pub struct SpecKitAutoCommand;

impl SpecKitCommand for SpecKitAutoCommand {
    fn name(&self) -> &'static str {
        "speckit.auto"
    }

    fn aliases(&self) -> &[&'static str] {
        &[] // No aliases - use /speckit.auto explicitly
    }

    fn description(&self) -> &'static str {
        "full 6-stage pipeline with auto-advancement (supports --configure for interactive modal)"
    }

    fn execute(&self, widget: &mut ChatWidget, args: String) {
        // Parse spec-auto args and delegate to handler
        match crate::slash_command::parse_spec_auto_args(&args) {
            Ok(invocation) => {
                widget.handle_spec_auto_command(invocation);
            }
            Err(err) => {
                let error_msg = match err {
                    crate::slash_command::SpecAutoParseError::MissingSpecId => {
                        "Missing SPEC ID. Usage: /speckit.auto SPEC-KIT-### [--configure] [--from stage]"
                            .to_string()
                    }
                    crate::slash_command::SpecAutoParseError::MissingFromStage => {
                        "`--from` flag requires a stage name".to_string()
                    }
                    crate::slash_command::SpecAutoParseError::UnknownStage(stage) => {
                        format!(
                            "Unknown stage '{}'. Valid stages: plan, tasks, implement, validate, audit, unlock",
                            stage
                        )
                    }
                    crate::slash_command::SpecAutoParseError::UnknownHalMode(mode) => {
                        format!("Unknown HAL mode '{}'. Expected 'mock' or 'live'", mode)
                    }
                };
                widget.history_push(crate::history_cell::new_error_event(error_msg));
                widget.request_redraw();
            }
        }
    }

    fn requires_args(&self) -> bool {
        true
    }
}

/// Command: /speckit.new (and /new-spec)
/// Create new SPEC from description with interactive Q&A - FULLY NATIVE (zero agents, $0)
/// SPEC-KIT-970: Now shows modal with 3 required questions before generating PRD
/// SPEC-KIT-971: Questions customized based on detected project type
pub struct SpecKitNewCommand;

impl SpecKitCommand for SpecKitNewCommand {
    fn name(&self) -> &'static str {
        "speckit.new"
    }

    fn aliases(&self) -> &[&'static str] {
        &["new-spec"]
    }

    fn description(&self) -> &'static str {
        "create new SPEC with project-aware Q&A (INSTANT, zero agents, $0)"
    }

    fn execute(&self, widget: &mut ChatWidget, args: String) {
        use super::super::project_detector::{detect_project_type, get_project_questions};

        // SPEC-KIT-971: Detect project type and customize questions
        let project_type = detect_project_type(&widget.config.cwd);
        let project_questions = get_project_questions(project_type);

        // Convert project_detector questions to prd_builder_modal format
        let modal_questions: Vec<crate::bottom_pane::prd_builder_modal::PrdQuestion> =
            project_questions
                .into_iter()
                .map(|q| crate::bottom_pane::prd_builder_modal::PrdQuestion {
                    category: q.category,
                    question: q.question,
                    options: q
                        .options
                        .into_iter()
                        .map(|o| crate::bottom_pane::prd_builder_modal::PrdOption {
                            label: o.label,
                            text: o.text,
                            is_custom: o.is_custom,
                        })
                        .collect(),
                })
                .collect();

        let project_display = format!("{} {}", project_type.icon(), project_type.display_name());

        // SPEC-KIT-970: Show interactive PRD builder modal with project-aware questions
        widget.show_prd_builder_with_context(
            args.trim().to_string(),
            project_display,
            modal_questions,
        );
    }

    fn requires_args(&self) -> bool {
        true
    }
}

/// Command: /speckit.specify
/// Generate PRD with single-agent refinement (SPEC-KIT-957: Tier 1)
pub struct SpecKitSpecifyCommand;

impl SpecKitCommand for SpecKitSpecifyCommand {
    fn name(&self) -> &'static str {
        "speckit.specify"
    }

    fn aliases(&self) -> &[&'static str] {
        &[]
    }

    fn description(&self) -> &'static str {
        "refine PRD with single-agent (Tier 1, ~$0.10)"
    }

    fn execute(&self, widget: &mut ChatWidget, args: String) {
        // SPEC-KIT-957: Direct execution, no longer uses orchestrator pattern
        super::plan::execute_stage_command(
            widget,
            args,
            crate::spec_prompts::SpecStage::Specify,
            "speckit.specify",
        );
    }

    fn expand_prompt(&self, _args: &str) -> Option<String> {
        None // SPEC-KIT-957: No longer uses orchestrator pattern
    }

    fn requires_args(&self) -> bool {
        true
    }
}

/// Command: /spec-consensus
/// Check multi-agent consensus via local-memory
pub struct SpecConsensusCommand;

impl SpecKitCommand for SpecConsensusCommand {
    fn name(&self) -> &'static str {
        "spec-consensus"
    }

    fn aliases(&self) -> &[&'static str] {
        &[]
    }

    fn description(&self) -> &'static str {
        "check multi-agent consensus via local-memory (requires SPEC ID & stage)"
    }

    fn execute(&self, widget: &mut ChatWidget, args: String) {
        handler::handle_spec_consensus(widget, args);
    }

    fn requires_args(&self) -> bool {
        true
    }
}

/// Command: /speckit.constitution
/// Extract and pin constitution bullets to ACE
pub struct SpecKitConstitutionCommand;

impl SpecKitCommand for SpecKitConstitutionCommand {
    fn name(&self) -> &'static str {
        "speckit.constitution"
    }

    fn aliases(&self) -> &[&'static str] {
        &[]
    }

    fn description(&self) -> &'static str {
        "extract and pin constitution bullets to ACE playbook"
    }

    fn execute(&self, widget: &mut ChatWidget, _args: String) {
        tracing::info!("SpecKitConstitution: execute() called");

        // Find constitution.md in the repository
        let constitution_path = widget.config.cwd.join("memory").join("constitution.md");

        tracing::info!(
            "SpecKitConstitution: Looking for constitution at: {:?}",
            constitution_path
        );

        if !constitution_path.exists() {
            widget.history_push(crate::history_cell::new_error_event(
                "Constitution not found at memory/constitution.md".to_string(),
            ));
            widget.request_redraw();
            return;
        }

        // Read constitution
        let markdown = match std::fs::read_to_string(&constitution_path) {
            Ok(content) => content,
            Err(e) => {
                widget.history_push(crate::history_cell::new_error_event(format!(
                    "Failed to read constitution: {}",
                    e
                )));
                widget.request_redraw();
                return;
            }
        };

        // Extract bullets
        let bullets = ace_constitution::extract_bullets(&markdown);

        if bullets.is_empty() {
            widget.history_push(crate::history_cell::new_error_event(
                "No valid bullets extracted from constitution".to_string(),
            ));
            widget.request_redraw();
            return;
        }

        // Show detailed extraction info
        let scope_counts: std::collections::HashMap<String, usize> = bullets
            .iter()
            .flat_map(|b| b.scopes.iter())
            .fold(std::collections::HashMap::new(), |mut acc, scope| {
                *acc.entry(scope.clone()).or_insert(0) += 1;
                acc
            });

        let scope_summary = scope_counts
            .iter()
            .map(|(scope, count)| format!("{}: {}", scope, count))
            .collect::<Vec<_>>()
            .join(", ");

        widget.history_push(crate::history_cell::PlainHistoryCell::new(
            vec![
                ratatui::text::Line::from(format!(
                    "📋 Extracted {} bullets from constitution",
                    bullets.len()
                )),
                ratatui::text::Line::from(format!("   Scopes: {}", scope_summary)),
                ratatui::text::Line::from("   Pinning to ACE playbook..."),
            ],
            crate::history_cell::HistoryCellType::Notice,
        ));

        // Get git context
        let repo_root = get_repo_root(&widget.config.cwd).unwrap_or_else(|| ".".to_string());
        let branch = get_current_branch(&widget.config.cwd).unwrap_or_else(|| "main".to_string());

        // Pin to ACE
        match ace_constitution::pin_constitution_to_ace_sync(
            &widget.config.ace,
            repo_root,
            branch,
            bullets,
        ) {
            Ok(pinned_count) => {
                widget.history_push(crate::history_cell::PlainHistoryCell::new(
                    vec![
                        ratatui::text::Line::from(format!(
                            "✅ Successfully pinned {} bullets to ACE playbook",
                            pinned_count
                        )),
                        ratatui::text::Line::from(
                            "   Database: ~/.code/ace/playbooks_normalized.sqlite3",
                        ),
                        ratatui::text::Line::from("   Use /speckit.ace-status to view playbook"),
                    ],
                    crate::history_cell::HistoryCellType::Notice,
                ));
            }
            Err(e) => {
                widget.history_push(crate::history_cell::new_error_event(format!(
                    "❌ Failed to pin bullets to ACE: {}",
                    e
                )));
            }
        }

        widget.request_redraw();
    }

    fn requires_args(&self) -> bool {
        false
    }
}

/// Command: /speckit.ace-status
/// Show ACE playbook status and statistics
pub struct SpecKitAceStatusCommand;

impl SpecKitCommand for SpecKitAceStatusCommand {
    fn name(&self) -> &'static str {
        "speckit.ace-status"
    }

    fn aliases(&self) -> &[&'static str] {
        &["ace-status"]
    }

    fn description(&self) -> &'static str {
        "show ACE playbook status and bullet statistics"
    }

    fn execute(&self, widget: &mut ChatWidget, _args: String) {
        use std::process::Command;

        widget.history_push(crate::history_cell::PlainHistoryCell::new(
            vec![ratatui::text::Line::from("📊 ACE Playbook Status")],
            crate::history_cell::HistoryCellType::Notice,
        ));

        let home = std::env::var("HOME").unwrap_or_else(|_| ".".to_string());
        let db_path = std::path::PathBuf::from(home).join(".code/ace/playbooks_normalized.sqlite3");

        // Check if database exists
        if !db_path.exists() {
            widget.history_push(crate::history_cell::new_error_event(
                "ACE database not found. Run /speckit.constitution to initialize.".to_string(),
            ));
            widget.request_redraw();
            return;
        }

        // Get statistics
        let query = "SELECT scope, COUNT(*), SUM(pinned), AVG(score), MAX(score) FROM playbook_bullet GROUP BY scope ORDER BY scope;";

        match Command::new("sqlite3").arg(&db_path).arg(query).output() {
            Ok(result) if result.status.success() => {
                let stats = String::from_utf8_lossy(&result.stdout);

                let mut lines = vec![
                    ratatui::text::Line::from(""),
                    ratatui::text::Line::from(
                        "Scope      | Total | Pinned | Avg Score | Max Score",
                    ),
                    ratatui::text::Line::from(
                        "-----------|-------|--------|-----------|----------",
                    ),
                ];

                for line in stats.lines() {
                    let parts: Vec<&str> = line.split('|').collect();
                    if parts.len() >= 5 {
                        lines.push(ratatui::text::Line::from(format!(
                            "{:<10} | {:<5} | {:<6} | {:<9.2} | {:.2}",
                            parts[0],
                            parts[1],
                            parts[2],
                            parts[3].parse::<f64>().unwrap_or(0.0),
                            parts[4].parse::<f64>().unwrap_or(0.0)
                        )));
                    }
                }

                lines.push(ratatui::text::Line::from(""));
                lines.push(ratatui::text::Line::from(format!(
                    "Database: {}",
                    db_path.display()
                )));

                widget.history_push(crate::history_cell::PlainHistoryCell::new(
                    lines,
                    crate::history_cell::HistoryCellType::Notice,
                ));
            }
            _ => {
                widget.history_push(crate::history_cell::new_error_event(
                    "Failed to query ACE database. Is sqlite3 installed?".to_string(),
                ));
            }
        }

        widget.request_redraw();
    }

    fn requires_args(&self) -> bool {
        false
    }
}
