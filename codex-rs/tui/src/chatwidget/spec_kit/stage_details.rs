//! Stage details widget for pipeline configurator
//!
//! SPEC-947: Pipeline UI Configurator - Phase 3 Task 3.2
//!
//! Renders detailed information for the selected stage including
//! description, agents, cost, duration, quality gate info,
//! dependencies, and validation warnings.

#![allow(dead_code)] // Extended widget features pending

use ratatui::{
    Frame,
    layout::Rect,
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph, Wrap},
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
                Span::styled(
                    format!("  • {} ", status),
                    Style::default().fg(status_color),
                ),
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
            lines.push(Line::from(vec![Span::styled(
                "  [Press Space to toggle, Enter/m/Esc to exit]",
                Style::default().fg(Color::DarkGray),
            )]));

            for (i, model) in selected_models.iter().enumerate() {
                let is_selected = selected_models.contains(model);
                let checkbox = if is_selected { "[✓]" } else { "[ ]" };
                let is_current = i == state.selected_model_index;

                let checkbox_style = if is_current {
                    Style::default()
                        .fg(Color::Cyan)
                        .add_modifier(Modifier::BOLD)
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

                let display_name = get_model_display_name(model);
                let tier = get_model_tier_public(model);
                lines.push(Line::from(vec![
                    Span::styled(format!("  {} ", checkbox), checkbox_style),
                    Span::styled(display_name, model_style),
                    Span::styled(format!(" ({})", tier), Style::default().fg(Color::DarkGray)),
                ]));
            }
        } else {
            // View mode: show current selection (non-interactive)
            lines.push(Line::from(vec![Span::styled(
                "  [Press Enter or 'm' to configure]",
                Style::default().fg(Color::DarkGray),
            )]));

            for model in &selected_models {
                let display_name = get_model_display_name(model);
                let tier = get_model_tier_public(model);
                lines.push(Line::from(vec![
                    Span::raw("  • "),
                    Span::styled(display_name, Style::default().fg(Color::Cyan)),
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
            Style::default().fg(Color::Red).add_modifier(Modifier::BOLD),
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
/// Updated: 2025-11-19 - All GPT-5 → GPT-5.1 (latest version)
fn get_stage_agents(stage: &StageType) -> &'static str {
    match stage {
        StageType::New => "Native (0 agents, instant)",
        StageType::Specify => "1 agent (GPT-5.1 Mini)",
        StageType::Plan => "3 agents (Gemini 2.5 Flash, Claude Haiku 4.5, GPT-5.1)",
        StageType::Tasks => "1 agent (GPT-5.1 Mini)",
        StageType::Implement => "2 agents (GPT-5.1 Codex, Claude Haiku 4.5 validator)",
        StageType::Validate => "3 agents (Gemini 2.5 Flash, Claude Haiku 4.5, GPT-5.1)",
        StageType::Audit => "3 premium (Gemini 2.5 Pro, Claude Sonnet 4.5, GPT-5.1)",
        StageType::Unlock => "3 premium (Gemini 2.5 Pro, Claude Sonnet 4.5, GPT-5.1)",
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

/// Get model display name (user-friendly with version)
///
/// Returns clear, descriptive name showing actual model and version
///
/// # Arguments
/// * `model` - Internal model name
///
/// # Returns
/// Display-friendly name with version
pub fn get_model_display_name(model: &str) -> &'static str {
    match model {
        // Aliases (shortcuts to specific models)
        "gemini" => "Gemini 2.5 Flash (alias)",
        "claude" => "Claude Haiku 4.5 (alias)",
        "code" => "GPT-5.1 (TUI default)", // Updated: was Claude Sonnet, now GPT-5.1

        // GPT-5.1 family (Latest: Nov 13, 2025)
        "gpt5_1_mini" | "gpt-5-mini" => "GPT-5.1 Mini",
        "gpt5_1" | "gpt-5" => "GPT-5.1",
        "gpt5_1_codex" | "gpt-5-codex" => "GPT-5.1 Codex",

        // Gemini family
        "gemini-flash" => "Gemini 2.5 Flash",
        "gemini-pro" => "Gemini 2.5 Pro",
        "gemini-3-pro" => "Gemini 3 Pro (LMArena #1)",

        // Claude family
        "claude-haiku" => "Claude Haiku 4.5",
        "claude-sonnet" => "Claude Sonnet 4.5",
        "claude-opus" => "Claude Opus 4.1",

        // Unknown - should not happen if registry is complete
        _ => "(unknown model)",
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

        // Premium models (Tier 2-3)
        "claude-sonnet" | "gemini-pro" => "premium",
        "gemini-3-pro" => "premium (LMArena #1)", // NEW: Top LMArena (1501 Elo)
        "gpt5_1_codex" => "codex (premium)",
        "claude-opus" => "opus (premium)",

        // Unknown - assume expensive for safety
        _ => "unknown",
    }
}

/// Get model role description
///
/// Returns role label describing what the model does in this stage
/// Based on prompts.json role definitions - shows sequential workflow
///
/// # Arguments
/// * `stage` - Stage type
/// * `model` - Model name
///
/// # Returns
/// Role description string
pub fn get_model_role(stage: &StageType, model: &str) -> &'static str {
    // Note: Multi-agent stages use SEQUENTIAL workflow (not parallel consensus):
    // Slot 1 → Researcher, Slot 2 → Synthesizer, Slot 3 → Executor & QA (aggregator)

    // Get slot index by checking defaults
    let defaults =
        super::pipeline_configurator::PipelineConfiguratorState::get_default_models(stage);
    let slot = defaults.iter().position(|m| m == model).unwrap_or(0);

    match stage {
        // New stage (3-agent sequential)
        StageType::New => match slot {
            0 => "researcher",
            1 => "synthesizer",
            2 => "executor & QA (aggregator)",
            _ => "agent",
        },

        // Specify stage (single agent)
        StageType::Specify => "PRD elaboration",

        // Plan stage (3-agent sequential: research → synthesize → validate)
        StageType::Plan => match slot {
            0 => "researcher",
            1 => "synthesizer",
            2 => "executor & QA (aggregator)",
            _ => "agent",
        },

        // Tasks stage (single agent)
        StageType::Tasks => "task decomposition",

        // Implement stage (code + validator sequential)
        StageType::Implement => match slot {
            0 => "code generation specialist",
            1 => "validation & QA (aggregator)",
            _ => "agent",
        },

        // Validate stage (3-agent sequential)
        StageType::Validate => match slot {
            0 => "test researcher",
            1 => "test synthesizer",
            2 => "test validator & QA (aggregator)",
            _ => "agent",
        },

        // Audit stage (3-agent sequential)
        StageType::Audit => match slot {
            0 => "security researcher",
            1 => "compliance synthesizer",
            2 => "audit validator & QA (aggregator)",
            _ => "agent",
        },

        // Unlock stage (3-agent sequential)
        StageType::Unlock => match slot {
            0 => "readiness researcher",
            1 => "decision synthesizer",
            2 => "ship validator & QA (aggregator)",
            _ => "agent",
        },
    }
}
