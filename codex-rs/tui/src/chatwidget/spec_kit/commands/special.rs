//! Special command implementations (auto, new, specify, consensus, constitution)
//!
//! FORK-SPECIFIC (just-every/code): Spec-kit multi-agent automation framework

use super::super::super::ChatWidget;
use super::super::ace_constitution;
use super::super::command_registry::SpecKitCommand;
use super::super::handler;
use super::super::routing::{get_current_branch, get_repo_root};

/// Command: /speckit.auto (and /spec-auto)
/// Full 6-stage pipeline with auto-advancement
pub struct SpecKitAutoCommand;

impl SpecKitCommand for SpecKitAutoCommand {
    fn name(&self) -> &'static str {
        "speckit.auto"
    }

    fn aliases(&self) -> &[&'static str] {
        &["spec-auto"]
    }

    fn description(&self) -> &'static str {
        "full 6-stage pipeline with auto-advancement"
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
                        "Missing SPEC ID. Usage: /speckit.auto SPEC-KIT-### [--from stage]"
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
/// Create new SPEC from description with templates
pub struct SpecKitNewCommand;

impl SpecKitCommand for SpecKitNewCommand {
    fn name(&self) -> &'static str {
        "speckit.new"
    }

    fn aliases(&self) -> &[&'static str] {
        &["new-spec"]
    }

    fn description(&self) -> &'static str {
        "create new SPEC from description with templates (55% faster)"
    }

    fn execute(&self, widget: &mut ChatWidget, args: String) {
        // SPEC-KIT-070: Generate SPEC-ID natively to eliminate $2.40 consensus cost
        let spec_id = match super::super::spec_id_generator::generate_next_spec_id(&widget.config.cwd) {
            Ok(id) => id,
            Err(e) => {
                widget.history_push(crate::history_cell::new_error_event(format!(
                    "Failed to generate SPEC-ID: {}",
                    e
                )));
                widget.request_redraw();
                return;
            }
        };

        let slug = super::super::spec_id_generator::create_slug(&args);
        let spec_dir_name = format!("{}-{}", spec_id, slug);

        // Inject SPEC-ID into prompt for orchestrator
        let enhanced_args = format!(
            "Create SPEC with ID: {}, Directory: {}, Description: {}",
            spec_id, spec_dir_name, args
        );

        // Routed to subagent orchestrators
        // Use format_subagent_command and submit
        let formatted = codex_core::slash_commands::format_subagent_command(
            "speckit.new",
            &enhanced_args,
            Some(&widget.config.agents),
            Some(&widget.config.subagent_commands),
        );

        let display = format!("{} ({})", args, spec_id);
        widget.submit_prompt_with_display(display, formatted.prompt);
    }

    fn requires_args(&self) -> bool {
        true
    }
}

/// Command: /speckit.specify
/// Generate PRD with multi-agent consensus
pub struct SpecKitSpecifyCommand;

impl SpecKitCommand for SpecKitSpecifyCommand {
    fn name(&self) -> &'static str {
        "speckit.specify"
    }

    fn aliases(&self) -> &[&'static str] {
        &[]
    }

    fn description(&self) -> &'static str {
        "generate PRD with multi-agent consensus"
    }

    fn execute(&self, widget: &mut ChatWidget, args: String) {
        // Routed to subagent orchestrators
        let formatted = codex_core::slash_commands::format_subagent_command(
            "speckit.specify",
            &args,
            Some(&widget.config.agents),
            Some(&widget.config.subagent_commands),
        );
        widget.submit_prompt_with_display(args, formatted.prompt);
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
        // Find constitution.md in the repository
        let constitution_path = widget.config.cwd.join("memory").join("constitution.md");

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

        widget.history_push(crate::history_cell::PlainHistoryCell::new(
            vec![ratatui::text::Line::from(format!(
                "Extracted {} bullets from constitution, pinning to ACE...",
                bullets.len()
            ))],
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
                    vec![ratatui::text::Line::from(format!(
                        "Successfully pinned {} bullets to ACE playbook (global + phase scopes)",
                        pinned_count
                    ))],
                    crate::history_cell::HistoryCellType::Notice,
                ));
            }
            Err(e) => {
                widget.history_push(crate::history_cell::new_error_event(format!(
                    "Failed to pin bullets to ACE: {}",
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
