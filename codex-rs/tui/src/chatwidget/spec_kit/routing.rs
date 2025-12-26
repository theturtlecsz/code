//! Spec-Kit command routing
//!
//! FORK-SPECIFIC (just-every/code): Spec-kit multi-agent automation framework
//!
//! This module handles routing slash commands to the spec-kit command registry,
//! isolating all routing logic from app.rs to minimize rebase conflicts.

use super::super::ChatWidget;
use super::command_registry::SPEC_KIT_REGISTRY;
use super::subagent_defaults;
use crate::app_event::AppEvent;
use crate::app_event_sender::AppEventSender;
use codex_core::protocol::Op;
use std::path::Path;
use std::process::Command;

/// Get the git repository root, if available
pub fn get_repo_root(cwd: &Path) -> Option<String> {
    Command::new("git")
        .arg("rev-parse")
        .arg("--show-toplevel")
        .current_dir(cwd)
        .output()
        .ok()
        .and_then(|output| {
            if output.status.success() {
                String::from_utf8(output.stdout)
                    .ok()
                    .map(|s| s.trim().to_string())
            } else {
                None
            }
        })
}

/// Get the current git branch, if available
pub fn get_current_branch(cwd: &Path) -> Option<String> {
    Command::new("git")
        .arg("rev-parse")
        .arg("--abbrev-ref")
        .arg("HEAD")
        .current_dir(cwd)
        .output()
        .ok()
        .and_then(|output| {
            if output.status.success() {
                String::from_utf8(output.stdout)
                    .ok()
                    .map(|s| s.trim().to_string())
            } else {
                None
            }
        })
}

/// Try to dispatch a command through the spec-kit command registry
///
/// Returns true if the command was handled by the registry, false otherwise.
/// This allows app.rs to fall through to upstream command handling if the
/// command is not a spec-kit command.
///
/// # Arguments
/// * `widget` - The chat widget to execute the command on
/// * `command_text` - The full command text (e.g., "/speckit.plan SPEC-KIT-065")
/// * `app_event_tx` - Event sender for history persistence
///
/// # Returns
/// - `true` if command was handled by spec-kit registry
/// - `false` if command should fall through to upstream routing
pub fn try_dispatch_spec_kit_command(
    widget: &mut ChatWidget,
    command_text: &str,
    app_event_tx: &AppEventSender,
) -> bool {
    // DEBUG: Trace registry dispatch (SPEC-DOGFOOD-001 Session 29)
    widget.history_push(crate::history_cell::PlainHistoryCell::new(
        vec![ratatui::text::Line::from(format!(
            "📍 DEBUG: try_dispatch_spec_kit_command({})",
            command_text
        ))],
        crate::history_cell::HistoryCellType::Notice,
    ));

    // Extract command name (first token after /)
    let command_name = command_text
        .trim_start_matches('/')
        .split_whitespace()
        .next()
        .unwrap_or("");

    // Try to find command in registry
    let Ok(registry) = SPEC_KIT_REGISTRY.lock() else {
        // Registry mutex poisoned - fall through to upstream
        return false;
    };

    let Some(spec_cmd) = registry.find(command_name) else {
        // Not a spec-kit command - fall through to upstream
        return false;
    };

    // Extract arguments (everything after command name)
    let args = command_text
        .trim_start_matches('/')
        .trim_start_matches(command_name)
        .trim()
        .to_string();

    // Handle prompt-expanding vs direct execution
    // SPEC-KIT-070 Phase 2+3: Native commands ALWAYS use direct execution
    // SPEC-KIT-900: All control/utility commands are native (not prompt-expanding)
    let is_native_command = matches!(
        command_name,
        // Native quality commands (Tier 0: FREE, <1s)
        "speckit.clarify"
        | "speckit.analyze"
        | "speckit.checklist"
        | "speckit.new"
        // Native control commands (direct execution through registry)
        | "speckit.auto"           // Pipeline coordinator
        | "speckit.cancel"         // SPEC-DOGFOOD-001: Cancel stale state
        | "speckit.status"         // Status dashboard
        | "speckit.constitution"   // ACE constitution extraction
        | "speckit.ace-status"     // ACE playbook status
        // Legacy aliases
        | "spec-auto"
        | "spec-cancel"            // SPEC-DOGFOOD-001
        | "spec-status"
        | "new-spec"
    );

    if !is_native_command && spec_cmd.expand_prompt(&args).is_some() {
        // Prompt-expanding command: need to re-format with config to get orchestrator instructions
        // Use command_name directly - config.toml entries match (e.g., "speckit.new")
        let config_name = command_name;

        // Combine user-provided subagent command config with Spec-Kit defaults
        let mut merged_commands = widget.config.subagent_commands.clone();
        if !merged_commands
            .iter()
            .any(|cfg| cfg.name.eq_ignore_ascii_case(config_name))
            && let Some(default_cfg) = subagent_defaults::default_for(config_name)
        {
            merged_commands.push(default_cfg);
        }

        // Format with the resolved configuration to get orchestrator instructions
        let formatted = codex_core::slash_commands::format_subagent_command(
            config_name,
            &args,
            Some(&widget.config.agents),
            Some(merged_commands.as_slice()),
        );

        // Submit with ACE injection (async, event-based)
        widget.submit_prompt_with_ace(command_text.to_string(), formatted.prompt, config_name);
    } else {
        // Direct execution: persist to history then execute
        app_event_tx.send(AppEvent::CodexOp(Op::AddToHistory {
            text: command_text.to_string(),
        }));
        spec_cmd.execute(widget, args);
    }

    // Command was handled
    true
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_command_name_extraction() {
        // Test command name parsing
        let test_cases = vec![
            ("/speckit.status", "speckit.status"),
            ("/speckit.plan SPEC-KIT-065", "speckit.plan"),
            ("/guardrail.auto SPEC-KIT-065 --from plan", "guardrail.auto"),
            ("/spec-consensus SPEC-KIT-065 plan", "spec-consensus"),
        ];

        for (input, expected) in test_cases {
            let name = input
                .trim_start_matches('/')
                .split_whitespace()
                .next()
                .unwrap_or("");
            assert_eq!(name, expected, "Failed for input: {}", input);
        }
    }

    #[test]
    fn test_args_extraction() {
        // Test argument extraction
        let test_cases = vec![
            ("/speckit.status", "speckit.status", ""),
            ("/speckit.plan SPEC-KIT-065", "speckit.plan", "SPEC-KIT-065"),
            (
                "/guardrail.auto SPEC-KIT-065 --from plan",
                "guardrail.auto",
                "SPEC-KIT-065 --from plan",
            ),
        ];

        for (command_text, cmd_name, expected_args) in test_cases {
            let args = command_text
                .trim_start_matches('/')
                .trim_start_matches(cmd_name)
                .trim();
            assert_eq!(args, expected_args, "Failed for input: {}", command_text);
        }
    }

    #[test]
    fn test_registry_find_returns_true() {
        // Verify that known commands return true from try_dispatch
        let registry = SPEC_KIT_REGISTRY.lock().unwrap();

        // All registered commands should be findable
        assert!(registry.find("speckit.status").is_some());
        assert!(registry.find("speckit.plan").is_some());
        assert!(registry.find("guardrail.plan").is_some());
        assert!(registry.find("spec-status").is_some());

        // Unknown commands should not be found
        assert!(registry.find("unknown-command").is_none());
        assert!(registry.find("browser").is_none()); // upstream command
    }
}
