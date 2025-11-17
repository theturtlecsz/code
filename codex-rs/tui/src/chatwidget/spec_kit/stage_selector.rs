//! Stage selector widget for pipeline configurator
//!
//! SPEC-947: Pipeline UI Configurator - Phase 3 Task 3.1
//!
//! Renders checkbox list for stage selection with cost display,
//! visual indicators, row highlighting, and summary footer.

use ratatui::{
    layout::Rect,
    style::{Color, Modifier, Style},
    text::Line,
    widgets::{Block, Borders, List, ListItem},
    Frame,
};

use super::pipeline_configurator::PipelineConfiguratorState;

/// Render stage selector widget (left pane)
///
/// Displays interactive checkbox list with:
/// - [✓]/[ ] checkboxes for enabled/disabled stages
/// - Stage names (capitalized)
/// - Cost estimates per stage
/// - Visual indicators: [$] high cost, [🔒] quality gate, [⚠] warnings
/// - Selected row highlighting (blue background)
/// - Footer with totals: X/8 stages, $X.XX, ~X min
///
/// # Arguments
/// * `frame` - Ratatui frame for rendering
/// * `area` - Rectangle area for this widget
/// * `state` - Configurator state (contains stage states, selection, warnings)
pub fn render_stage_selector(frame: &mut Frame, area: Rect, state: &PipelineConfiguratorState) {
    let mut items: Vec<ListItem> = state
        .all_stages
        .iter()
        .enumerate()
        .map(|(i, stage)| {
            // Checkbox: [✓] if enabled, [ ] if disabled
            let checkbox = if state.stage_states[i] {
                "[✓]"
            } else {
                "[ ]"
            };

            // Stage name (capitalize first letter for better UX)
            let stage_name = capitalize_stage_name(&stage.to_string());

            // Cost estimate
            let cost = stage.cost_estimate();

            // Visual indicators
            let mut indicators = Vec::new();

            // High-cost indicator (>$0.50)
            if cost > 0.50 {
                indicators.push("[$]");
            }

            // Quality gate indicator
            if stage.has_quality_gate() {
                indicators.push("[🔒]");
            }

            // Warning indicator (check if this stage appears in warnings)
            // Simple heuristic: if warnings mention this stage name, show ⚠
            let stage_str = stage.to_string();
            let has_warning = state
                .warnings
                .iter()
                .any(|w| w.to_lowercase().contains(&stage_str));
            if has_warning {
                indicators.push("[⚠]");
            }

            // Format line: [✓] Implement ($0.10) [🔒]
            let line = format!(
                "{} {} (${:.2}) {}",
                checkbox,
                stage_name,
                cost,
                indicators.join(" ")
            );

            // Style: highlight selected row with blue background + bold
            let style = if i == state.selected_index {
                Style::default()
                    .bg(Color::Blue)
                    .fg(Color::White)
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default()
            };

            ListItem::new(line).style(style)
        })
        .collect();

    // Footer with totals
    let total_cost = state.total_cost();
    let total_duration = state.total_duration();
    let enabled_count = state.stage_states.iter().filter(|&&s| s).count();

    let footer = format!(
        "\nTotal: {}/{} stages, ${:.2}, ~{} min",
        enabled_count,
        state.all_stages.len(),
        total_cost,
        total_duration
    );

    items.push(ListItem::new(footer).style(Style::default().fg(Color::Gray)));

    // Create list widget with border and title
    let list = List::new(items)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title("Stage Selection"),
        )
        .highlight_symbol("▶ ");

    frame.render_widget(list, area);
}

/// Capitalize first letter of stage name for better UX
///
/// Converts "implement" → "Implement", "new" → "New"
///
/// # Arguments
/// * `s` - Stage name in lowercase
///
/// # Returns
/// Stage name with first letter capitalized
fn capitalize_stage_name(s: &str) -> String {
    let mut chars = s.chars();
    match chars.next() {
        None => String::new(),
        Some(first) => first.to_uppercase().chain(chars).collect(),
    }
}
