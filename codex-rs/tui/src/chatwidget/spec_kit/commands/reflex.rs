//! /speckit.reflex command implementation
//!
//! SPEC-KIT-978: Reflex (local inference) management commands
//!
//! Commands:
//! - /speckit.reflex health - Check reflex server health
//! - /speckit.reflex status - Show reflex configuration
//! - /speckit.reflex models - List available models

use super::super::super::ChatWidget;
use super::super::command_registry::SpecKitCommand;
use super::super::handler;

/// Command: /speckit.reflex
/// Manage local reflex inference server (SPEC-KIT-978)
pub struct SpecKitReflexCommand;

impl SpecKitCommand for SpecKitReflexCommand {
    fn name(&self) -> &'static str {
        "speckit.reflex"
    }

    fn aliases(&self) -> &[&'static str] {
        &["reflex"]
    }

    fn description(&self) -> &'static str {
        "manage local reflex inference server"
    }

    fn execute(&self, widget: &mut ChatWidget, args: String) {
        handler::handle_speckit_reflex(widget, args);
    }

    fn requires_args(&self) -> bool {
        true // Subcommand required (health, status, models)
    }

    fn is_prompt_expanding(&self) -> bool {
        false
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_reflex_command_name() {
        let cmd = SpecKitReflexCommand;
        assert_eq!(cmd.name(), "speckit.reflex");
        assert!(cmd.aliases().contains(&"reflex"));
        assert!(cmd.requires_args());
    }
}
