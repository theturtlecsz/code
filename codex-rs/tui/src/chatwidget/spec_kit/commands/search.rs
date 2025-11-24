//! Search command for spec-kit
//!
//! FORK-SPECIFIC (just-every/code): Spec-kit multi-agent automation framework
//!
//! This module implements the `/search` command, allowing users to find text
//! within the conversation history.

use super::super::command_registry::SpecKitCommand;
use crate::chatwidget::ChatWidget;
use crate::history_cell::{HistoryCellType, PlainHistoryCell};
use ratatui::text::Line;
use shlex::Shlex;

/// Command to search conversation history
pub struct SearchCommand;

impl SpecKitCommand for SearchCommand {
    fn name(&self) -> &'static str {
        "search"
    }

    fn aliases(&self) -> &[&'static str] {
        &["history_search"]
    }

    fn description(&self) -> &'static str {
        "Searches the conversation history for a given query. Usage: /search <query> [--agent <user|assistant|tool>]"
    }

    fn execute(&self, widget: &mut ChatWidget, args: String) {
        let parsed_args: Vec<String> = Shlex::new(&args).collect();

        let mut query: Option<String> = None;
        let mut agent_filter: Option<String> = None;
        let mut parse_agent_next = false;
        let mut help_requested = false;

        for arg in parsed_args {
            if arg == "--agent" {
                parse_agent_next = true;
            } else if arg == "--help" {
                help_requested = true;
                break;
            } else if parse_agent_next {
                agent_filter = Some(arg.to_lowercase());
                parse_agent_next = false;
            } else {
                query = Some(arg);
            }
        }

        if help_requested {
            widget.history_push(PlainHistoryCell::new(
                vec![
                    Line::from("Usage: /search <query> [--agent <user|assistant|tool>]"),
                    Line::from(""),
                    Line::from("  <query>               The text to search for."),
                    Line::from("  --agent <user|assistant|tool> Filter results by message author."),
                    Line::from(""),
                    Line::from("Examples:"),
                    Line::from("  /search \"bug fix\""),
                    Line::from("  /search error --agent assistant"),
                    Line::from("  /history.search --agent user \"how to fix\""),
                ],
                HistoryCellType::Notice,
            ));
            widget.request_redraw();
            return;
        }

        let query = match query {
            Some(q) => q,
            None => {
                widget.history_push(PlainHistoryCell::new(
                    vec![Line::from(
                        "Error: Search query cannot be empty. Use /search --help for usage.",
                    )],
                    HistoryCellType::Error,
                ));
                widget.request_redraw();
                return;
            }
        };

        let mut results: Vec<Line<'static>> = Vec::new();
        results.push(Line::from(format!("🔍 Searching for \"{}\"...", query)));

        // TODO: Implement actual search logic here.
        // Iterate through widget.history_cells, extract text, filter by agent, and find matches.
        // For now, this is a placeholder.

        results.push(Line::from(""));
        results.push(Line::from("--- Search Results (placeholder) ---"));
        results.push(Line::from("No matches found."));

        widget.history_push(PlainHistoryCell::new(results, HistoryCellType::Notice));
        widget.request_redraw();
    }
}
