//! Stage details widget for pipeline configurator
//!
//! SPEC-947: Pipeline UI Configurator - Phase 3 Task 3.2
//!
//! Renders detailed information for the selected stage including
//! description, agents, cost, duration, quality gate info,
//! dependencies, and validation warnings.

use ratatui::{
    layout::Rect,
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph, Wrap},
    Frame,
};

use super::pipeline_config::StageType;
use super::pipeline_configurator::PipelineConfiguratorState;

/// Render stage details widget (right pane)
///
/// Displays comprehensive information about the currently selected stage:
/// - Stage name and description
/// - Agents used (number and tier)
/// - Cost estimate (in USD)
/// - Duration estimate (in minutes)
/// - Quality gate information (if applicable)
/// - Dependencies with status indicators (✓/✗)
/// - Validation warnings (color-coded: red for errors, yellow for warnings)
///
/// # Arguments
/// * `frame` - Ratatui frame for rendering
/// * `area` - Rectangle area for this widget
/// * `state` - Configurator state (contains selected stage, config, warnings)
pub fn render_stage_details(frame: &mut Frame, area: Rect, state: &PipelineConfiguratorState) {
    let selected_stage = &state.all_stages[state.selected_index];

    // Build detail text
    let mut lines = Vec::new();

    // Stage header (name + description)
    let stage_name = capitalize_stage_name(&selected_stage.to_string());
    lines.push(Line::from(vec![
        Span::styled(
            format!("> {}: ", stage_name),
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        ),
        Span::raw(get_stage_description(selected_stage)),
    ]));

    lines.push(Line::raw(""));

    // Agents (if multi-agent stage)
    let agents_info = get_stage_agents(selected_stage);
    if !agents_info.is_empty() {
        lines.push(Line::from(vec![
            Span::styled("Agents: ", Style::default().fg(Color::Yellow)),
            Span::raw(agents_info),
        ]));
        lines.push(Line::raw(""));
    }

    // Cost and duration
    lines.push(Line::from(vec![
        Span::styled("Cost: ", Style::default().fg(Color::Green)),
        Span::raw(format!("~${:.2}", selected_stage.cost_estimate())),
    ]));

    lines.push(Line::from(vec![
        Span::styled("Duration: ", Style::default().fg(Color::Green)),
        Span::raw(format!("~{} min", selected_stage.duration_estimate())),
    ]));

    lines.push(Line::raw(""));

    // Quality gate
    if selected_stage.has_quality_gate() {
        lines.push(Line::from(vec![
            Span::styled("Quality Gate: ", Style::default().fg(Color::Magenta)),
            Span::raw("Post-stage checkpoint (3 agents vote)"),
        ]));
        lines.push(Line::raw(""));
    }

    // Dependencies
    let deps = selected_stage.dependencies();
    if !deps.is_empty() {
        lines.push(Line::from(vec![Span::styled(
            "Dependencies:",
            Style::default().fg(Color::Blue),
        )]));

        for dep in deps {
            let dep_enabled = state.pending_config.is_enabled(dep);
            let (status, status_color) = if dep_enabled {
                ("✓", Color::Green)
            } else {
                ("✗", Color::Red)
            };

            let dep_name = capitalize_stage_name(&dep.to_string());
            lines.push(Line::from(vec![
                Span::styled(format!("  • {} ", status), Style::default().fg(status_color)),
                Span::raw(dep_name),
            ]));
        }
        lines.push(Line::raw(""));
    }

    // Warnings section
    if !state.warnings.is_empty() {
        lines.push(Line::from(vec![Span::styled(
            "Warnings:",
            Style::default()
                .fg(Color::Red)
                .add_modifier(Modifier::BOLD),
        )]));

        for warning in &state.warnings {
            let style = if warning.starts_with("Error:") {
                Style::default().fg(Color::Red)
            } else if warning.starts_with("⚠") || warning.starts_with("Warning:") {
                Style::default().fg(Color::Yellow)
            } else {
                Style::default().fg(Color::Gray)
            };
            lines.push(Line::from(vec![Span::styled(warning, style)]));
        }
    } else {
        // Show positive message when no warnings
        lines.push(Line::from(vec![Span::styled(
            "✓ No warnings",
            Style::default().fg(Color::Green),
        )]));
    }

    // Render paragraph
    let paragraph = Paragraph::new(lines)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title("Stage Details"),
        )
        .wrap(Wrap { trim: true });

    frame.render_widget(paragraph, area);
}

// ============================================================================
// Helper Functions
// ============================================================================

/// Get description for a stage
///
/// Returns human-readable description explaining what each stage does
fn get_stage_description(stage: &StageType) -> &'static str {
    match stage {
        StageType::New => "Create SPEC directory and skeleton files",
        StageType::Specify => "Draft PRD with single-agent analysis",
        StageType::Plan => "Create work breakdown with multi-agent consensus",
        StageType::Tasks => "Decompose plan into executable tasks",
        StageType::Implement => "Generate code with specialist models",
        StageType::Validate => "Test strategy with multi-agent consensus",
        StageType::Audit => "Security and compliance validation",
        StageType::Unlock => "Final ship/no-ship decision",
    }
}

/// Get agents information for a stage
///
/// Returns string describing number and tier of agents used
fn get_stage_agents(stage: &StageType) -> &'static str {
    match stage {
        StageType::New => "Native (0 agents, instant)",
        StageType::Specify => "1 agent (gpt5_1_mini)",
        StageType::Plan => "3 agents (gemini-flash, claude-haiku, gpt5-medium)",
        StageType::Tasks => "1 agent (gpt5_1_mini)",
        StageType::Implement => "2 agents (gpt5_1_codex, claude-haiku validator)",
        StageType::Validate => "3 agents (gemini-flash, claude-haiku, gpt5-medium)",
        StageType::Audit => "3 premium (gemini-pro, claude-sonnet, gpt5-high)",
        StageType::Unlock => "3 premium (gemini-pro, claude-sonnet, gpt5-high)",
    }
}

/// Capitalize first letter of stage name
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
