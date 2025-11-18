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

    // Models section
    let selected_models = state.get_selected_models(selected_stage);

    if !selected_models.is_empty() {
        lines.push(Line::from(vec![Span::styled(
            "Models:",
            Style::default().fg(Color::Magenta),
        )]));

        if state.model_selection_mode {
            // Model selection mode: show checkboxes
            lines.push(Line::from(vec![
                Span::styled(
                    "  [Press Space to toggle, Enter/m/Esc to exit]",
                    Style::default().fg(Color::DarkGray),
                )
            ]));

            for (i, model) in selected_models.iter().enumerate() {
                let is_selected = selected_models.contains(model);
                let checkbox = if is_selected { "[✓]" } else { "[ ]" };
                let is_current = i == state.selected_model_index;

                let checkbox_style = if is_current {
                    Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD)
                } else if is_selected {
                    Style::default().fg(Color::Green)
                } else {
                    Style::default().fg(Color::Gray)
                };

                let model_style = if is_current {
                    Style::default().add_modifier(Modifier::BOLD)
                } else {
                    Style::default()
                };

                let tier = get_model_tier_public(model);
                lines.push(Line::from(vec![
                    Span::styled(format!("  {} ", checkbox), checkbox_style),
                    Span::styled(model, model_style),
                    Span::styled(format!(" ({})", tier), Style::default().fg(Color::DarkGray)),
                ]));
            }
        } else {
            // View mode: show current selection (non-interactive)
            lines.push(Line::from(vec![
                Span::styled(
                    "  [Press Enter or 'm' to configure]",
                    Style::default().fg(Color::DarkGray),
                )
            ]));

            for model in &selected_models {
                let tier = get_model_tier_public(model);
                lines.push(Line::from(vec![
                    Span::raw("  • "),
                    Span::styled(model, Style::default().fg(Color::Cyan)),
                    Span::styled(format!(" ({})", tier), Style::default().fg(Color::DarkGray)),
                ]));
            }
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

/// Get model tier classification
///
/// Returns tier label for display (cheap/medium/premium)
///
/// # Arguments
/// * `model` - Model name
///
/// # Returns
/// Tier label string
pub fn get_model_tier_public(model: &str) -> &'static str {
    match model {
        // Cheap models (Tier 0-1)
        "gemini" | "claude" | "code" => "native/cheap",
        "gpt5_1_mini" => "cheap",
        "gemini-flash" | "claude-haiku" | "gpt5_1" => "cheap/medium",

        // Premium models (Tier 3)
        "gpt5_codex" | "claude-sonnet" | "gemini-pro" => "premium",
        "gpt5_1_codex" => "codex (premium)",

        // Unknown - assume expensive for safety
        _ => "unknown",
    }
}

/// Get model role description
///
/// Returns role label describing what the model does in this stage
///
/// # Arguments
/// * `stage` - Stage type
/// * `model` - Model name
///
/// # Returns
/// Role description string
pub fn get_model_role(stage: &StageType, model: &str) -> &'static str {
    match (stage, model) {
        // New stage (native + multi-agent)
        (StageType::New, "gemini") => "consensus agent 1",
        (StageType::New, "claude") => "consensus agent 2",
        (StageType::New, "code") => "consensus agent 3",

        // Specify stage (single agent)
        (StageType::Specify, "gpt5_1_mini") => "PRD elaboration",

        // Plan stage (3-agent consensus)
        (StageType::Plan, "gemini-flash") => "consensus agent 1",
        (StageType::Plan, "claude-haiku") => "consensus agent 2",
        (StageType::Plan, "gpt5_1") => "consensus agent 3",

        // Tasks stage (single agent)
        (StageType::Tasks, "gpt5_1_mini") => "task decomposition",

        // Implement stage (code + validator)
        (StageType::Implement, "gpt5_1_codex") => "code generation",
        (StageType::Implement, "claude-haiku") => "validation",

        // Validate stage (3-agent consensus)
        (StageType::Validate, "gemini-flash") => "consensus agent 1",
        (StageType::Validate, "claude-haiku") => "consensus agent 2",
        (StageType::Validate, "gpt5_1") => "consensus agent 3",

        // Audit stage (3 premium agents)
        (StageType::Audit, "gpt5_codex") => "compliance check 1",
        (StageType::Audit, "claude-sonnet") => "compliance check 2",
        (StageType::Audit, "gemini-pro") => "compliance check 3",

        // Unlock stage (3 premium agents)
        (StageType::Unlock, "gpt5_codex") => "ship decision 1",
        (StageType::Unlock, "claude-sonnet") => "ship decision 2",
        (StageType::Unlock, "gemini-pro") => "ship decision 3",

        // Unknown combination
        _ => "unknown role",
    }
}
