//! Template management commands (/speckit.install-templates, /speckit.template-status)
//!
//! SPEC-KIT-962: Template installation architecture
//!
//! Commands for managing the layered template resolution system:
//! - install-templates: Copy embedded templates to user config for customization
//! - template-status: Show where each template resolves from

use super::super::super::ChatWidget;
use super::super::command_registry::SpecKitCommand;
use crate::app_event::BackgroundPlacement;
use crate::templates::{TemplateSource, all_template_status, install_templates};

/// Command: /speckit.install-templates
/// Copies embedded templates to ~/.config/code/templates/ for customization
pub struct SpecKitInstallTemplatesCommand;

impl SpecKitCommand for SpecKitInstallTemplatesCommand {
    fn name(&self) -> &'static str {
        "speckit.install-templates"
    }

    fn aliases(&self) -> &[&'static str] {
        &["install-templates"]
    }

    fn description(&self) -> &'static str {
        "install templates to user config for customization"
    }

    fn execute(&self, widget: &mut ChatWidget, args: String) {
        let force = args.trim() == "--force";

        match install_templates(force) {
            Ok(result) => {
                let mut output = String::new();
                output.push_str("## Template Installation\n\n");
                output.push_str(&format!("**Location**: `{}`\n\n", result.path.display()));

                if !result.installed.is_empty() {
                    output.push_str("### Installed\n");
                    for file in &result.installed {
                        output.push_str(&format!("- {}\n", file));
                    }
                    output.push('\n');
                }

                if !result.skipped.is_empty() {
                    output.push_str("### Skipped (already exist)\n");
                    for file in &result.skipped {
                        output.push_str(&format!("- {}\n", file));
                    }
                    output.push_str("\n*Use `--force` to overwrite*\n");
                }

                if result.installed.is_empty() && result.skipped.is_empty() {
                    output.push_str("No templates to install.\n");
                } else if !result.installed.is_empty() {
                    output.push_str(&format!(
                        "\n**{}** templates installed. Edit them to customize spec-kit behavior.\n",
                        result.installed.len()
                    ));
                }

                widget.insert_background_event_with_placement(output, BackgroundPlacement::Tail);
                widget.request_redraw();
            }
            Err(e) => {
                widget.insert_background_event_with_placement(
                    format!("## Template Installation Failed\n\n**Error**: {}\n", e),
                    BackgroundPlacement::Tail,
                );
                widget.request_redraw();
            }
        }
    }

    fn requires_args(&self) -> bool {
        false
    }

    fn is_prompt_expanding(&self) -> bool {
        false
    }
}

/// Command: /speckit.template-status
/// Shows where each template resolves from in the layered system
pub struct SpecKitTemplateStatusCommand;

impl SpecKitCommand for SpecKitTemplateStatusCommand {
    fn name(&self) -> &'static str {
        "speckit.template-status"
    }

    fn aliases(&self) -> &[&'static str] {
        &["template-status"]
    }

    fn description(&self) -> &'static str {
        "show template resolution sources"
    }

    fn execute(&self, widget: &mut ChatWidget, _args: String) {
        let status = all_template_status();

        let mut output = String::new();
        output.push_str("## Template Resolution Status\n\n");
        output.push_str("Templates are resolved in priority order:\n");
        // SPEC-KIT-964: Hermetic isolation - only project-local and embedded
        output.push_str("1. **Project-local**: `./templates/*.md`\n");
        output.push_str("2. **Embedded**: Compiled into binary\n\n");
        output.push_str("| Template | Source | Status |\n");
        output.push_str("|----------|--------|--------|\n");

        for s in &status {
            let source_str = match &s.source {
                TemplateSource::ProjectLocal(p) => format!("`{}`", p.display()),
                TemplateSource::Embedded => "[embedded]".to_string(),
            };
            let status_icon = if s.available { "OK" } else { "MISSING" };
            output.push_str(&format!(
                "| {} | {} | {} |\n",
                s.name, source_str, status_icon
            ));
        }

        output.push_str("\n*Run `/speckit.install-templates` to copy embedded templates to `./templates/` for customization.*\n");

        widget.insert_background_event_with_placement(output, BackgroundPlacement::Tail);
        widget.request_redraw();
    }

    fn requires_args(&self) -> bool {
        false
    }

    fn is_prompt_expanding(&self) -> bool {
        false
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_install_templates_command_name() {
        let cmd = SpecKitInstallTemplatesCommand;
        assert_eq!(cmd.name(), "speckit.install-templates");
        assert!(cmd.aliases().contains(&"install-templates"));
    }

    #[test]
    fn test_template_status_command_name() {
        let cmd = SpecKitTemplateStatusCommand;
        assert_eq!(cmd.name(), "speckit.template-status");
        assert!(cmd.aliases().contains(&"template-status"));
    }
}
