//! Pipeline configurator view for bottom pane modal
//!
//! SPEC-947 Phase 4 Task 4.1: Interactive TUI modal for pipeline stage selection

use crossterm::event::{KeyCode, KeyEvent, KeyEventKind};
use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::Frame;

use crate::app_event::AppEvent;
use crate::app_event_sender::AppEventSender;
use crate::chatwidget::spec_kit::pipeline_config::PipelineConfig;
use crate::chatwidget::spec_kit::pipeline_configurator::{
    ConfigAction, PipelineConfiguratorState, PipelineConfiguratorWidget, ViewMode,
};
use crate::chatwidget::spec_kit::stage_details;

use super::{BottomPane, BottomPaneView, CancellationEvent};

/// Modal state for pipeline configurator
pub(crate) struct PipelineConfiguratorView {
    state: PipelineConfiguratorState,
    app_event_tx: AppEventSender,
    done: bool,
}

impl PipelineConfiguratorView {
    pub fn new(
        spec_id: String,
        initial_config: PipelineConfig,
        app_event_tx: AppEventSender,
    ) -> Self {
        Self {
            state: PipelineConfiguratorState::new(spec_id, initial_config),
            app_event_tx,
            done: false,
        }
    }

    fn handle_save(&mut self) {
        // Save configuration to TOML
        let config_path = format!("docs/{}/pipeline.toml", self.state.spec_id);
        match self.state.pending_config.save(&config_path) {
            Ok(()) => {
                // Send success event with configuration summary
                let _ = self
                    .app_event_tx
                    .send(AppEvent::PipelineConfigurationSaved {
                        spec_id: self.state.spec_id.clone(),
                        config_path,
                        enabled_count: self.state.pending_config.enabled_stages.len(),
                        total_cost: self.state.total_cost(),
                        total_duration: self.state.total_duration(),
                    });
                self.done = true;
            }
            Err(err) => {
                // Send error event
                let _ = self
                    .app_event_tx
                    .send(AppEvent::PipelineConfigurationError {
                        spec_id: self.state.spec_id.clone(),
                        error: format!("Failed to save configuration: {}", err),
                    });
                // Keep modal open on error
            }
        }
    }

    fn handle_cancel(&mut self) {
        // Send cancellation event
        let _ = self
            .app_event_tx
            .send(AppEvent::PipelineConfigurationCancelled {
                spec_id: self.state.spec_id.clone(),
            });
        self.done = true;
    }
}

impl<'a> BottomPaneView<'a> for PipelineConfiguratorView {
    fn handle_key_event(&mut self, _pane: &mut BottomPane<'a>, key_event: KeyEvent) {
        // Only handle press events (not release)
        if key_event.kind != KeyEventKind::Press {
            return;
        }

        // Delegate to state machine (handles ESC at appropriate level)
        match self.state.handle_key_event(key_event) {
            ConfigAction::SaveAndExit => {
                self.handle_save();
            }
            ConfigAction::CancelAndExit => {
                self.handle_cancel();
            }
            ConfigAction::ShowConfirmation => {
                // TODO: Implement confirmation dialog (Phase 4 Task 4.4 optional)
                // For now, save anyway if no errors
                if !self.state.has_errors() {
                    self.handle_save();
                }
            }
            ConfigAction::Continue => {
                // Keep modal open, state updated
            }
        }
    }

    fn is_complete(&self) -> bool {
        self.done
    }

    fn on_ctrl_c(&mut self, _pane: &mut BottomPane<'a>) -> CancellationEvent {
        // Treat Ctrl-C as cancel
        self.handle_cancel();
        CancellationEvent::Handled
    }

    fn desired_height(&self, _width: u16) -> u16 {
        // Modal takes 70% of terminal height (from implementation plan)
        // Return a reasonable fixed height for the bottom pane
        // (actual modal overlay is rendered in full screen by widget)
        25
    }

    fn render(&self, area: Rect, buf: &mut Buffer) {
        use ratatui::layout::{Constraint, Direction, Layout};
        use ratatui::style::{Color, Modifier, Style};
        use ratatui::text::{Line, Span};
        use ratatui::widgets::{Block, Borders, Clear, List, ListItem, Paragraph, Widget, Wrap};

        // Clear background for modal
        Clear.render(area, buf);

        // Main border block
        let block = Block::default()
            .borders(Borders::ALL)
            .title(format!(" Pipeline Configuration: {} ", self.state.spec_id));
        let inner_area = block.inner(area);
        block.render(area, buf);

        // Split into left (40%) and right (60%) panes
        let chunks = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Percentage(40), Constraint::Percentage(60)])
            .split(inner_area);

        // Left pane: Stage selector (simplified version)
        let stage_items: Vec<ListItem> = self
            .state
            .all_stages
            .iter()
            .enumerate()
            .map(|(i, stage)| {
                let checkbox = if self.state.stage_states[i] {
                    "[✓]"
                } else {
                    "[ ]"
                };
                let cost = stage.cost_estimate();
                let line = format!("{} {} (${:.2})", checkbox, stage, cost);

                let style = if i == self.state.selected_index {
                    Style::default()
                        .bg(Color::Blue)
                        .add_modifier(Modifier::BOLD)
                } else {
                    Style::default()
                };

                ListItem::new(line).style(style)
            })
            .collect();

        let stage_list = List::new(stage_items).block(
            Block::default()
                .borders(Borders::ALL)
                .title("Stage Selection"),
        );
        stage_list.render(chunks[0], buf);

        // Right pane: Stage details (simplified version)
        let selected_stage = &self.state.all_stages[self.state.selected_index];
        let mut detail_lines = Vec::new();

        detail_lines.push(Line::from(vec![
            Span::styled("> Stage: ", Style::default().fg(Color::Cyan)),
            Span::raw(selected_stage.to_string()),
        ]));
        detail_lines.push(Line::raw(""));
        detail_lines.push(Line::from(vec![
            Span::styled("Cost: ", Style::default().fg(Color::Green)),
            Span::raw(format!("${:.2}", selected_stage.cost_estimate())),
        ]));
        detail_lines.push(Line::from(vec![
            Span::styled("Duration: ", Style::default().fg(Color::Green)),
            Span::raw(format!("~{} min", selected_stage.duration_estimate())),
        ]));
        detail_lines.push(Line::raw(""));

        // Models section (if in StageDetails mode)
        if self.state.view_mode == ViewMode::StageDetails {
            let selected_models = self.state.get_selected_models(selected_stage);

            if !selected_models.is_empty() {
                detail_lines.push(Line::from(vec![Span::styled(
                    "Models:",
                    Style::default().fg(Color::Magenta),
                )]));

                if self.state.reasoning_picker_mode {
                    // Reasoning level picker mode: show reasoning levels for selected model
                    let model = self.state.temp_selected_model.as_ref().unwrap();
                    let reasoning_levels = PipelineConfiguratorState::get_reasoning_levels(model);
                    let slot_index = self.state.selected_model_index;
                    let role = stage_details::get_model_role(selected_stage, model);

                    detail_lines.push(Line::from(vec![Span::styled(
                        format!("  Choose reasoning level for {} (slot {}):", role, slot_index + 1),
                        Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD),
                    )]));
                    detail_lines.push(Line::from(vec![Span::styled(
                        format!("  Model: {} - now select reasoning effort:", model),
                        Style::default().fg(Color::Cyan),
                    )]));
                    detail_lines.push(Line::from(vec![Span::styled(
                        "  [↑↓ Navigate | Enter Select | Esc Back to Models]",
                        Style::default().fg(Color::DarkGray),
                    )]));

                    for (i, level) in reasoning_levels.iter().enumerate() {
                        let is_current = i == self.state.reasoning_selected_index;

                        let style = if is_current {
                            Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD)
                        } else {
                            Style::default()
                        };

                        let marker = if is_current { ">" } else { " " };
                        let description = match level.as_str() {
                            "none" => "no reasoning (fastest, cheapest)",
                            "auto" => "automatic (default, balanced)",
                            "low" => "light reasoning (faster, ~0.8× cost)",
                            "medium" => "moderate reasoning (balanced, ~1.2× cost)",
                            "high" => "deep reasoning (slower, ~2-3× cost)",
                            _ => "",
                        };

                        detail_lines.push(Line::from(vec![
                            Span::raw(format!("  {} ", marker)),
                            Span::styled(level.clone(), style),
                            Span::styled(format!(" - {}", description), Style::default().fg(Color::DarkGray)),
                        ]));
                    }
                } else if self.state.model_picker_mode {
                    // Model picker mode: show ALL available models for current slot
                    let all_models = PipelineConfiguratorState::get_all_available_models();
                    let slot_index = self.state.selected_model_index;
                    let role = stage_details::get_model_role(
                        selected_stage,
                        &selected_models[slot_index]
                    );

                    detail_lines.push(Line::from(vec![Span::styled(
                        format!("  Choose model for {} (slot {}):", role, slot_index + 1),
                        Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD),
                    )]));
                    detail_lines.push(Line::from(vec![Span::styled(
                        "  [↑↓ Navigate | Enter Select | Esc Cancel]",
                        Style::default().fg(Color::DarkGray),
                    )]));

                    for (i, model) in all_models.iter().enumerate() {
                        let is_current = i == self.state.picker_selected_index;
                        let display_name = stage_details::get_model_display_name(model);
                        let tier = stage_details::get_model_tier_public(model);

                        let style = if is_current {
                            Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD)
                        } else {
                            Style::default()
                        };

                        let marker = if is_current { ">" } else { " " };
                        detail_lines.push(Line::from(vec![
                            Span::raw(format!("  {} ", marker)),
                            Span::styled(display_name, style),
                            Span::styled(format!(" ({})", tier), Style::default().fg(Color::DarkGray)),
                        ]));
                    }
                } else if self.state.model_selection_mode {
                    // Model selection mode: show slots with assigned models
                    detail_lines.push(Line::from(vec![Span::styled(
                        "  [↑↓ Navigate Slots | Enter Change Model | m/Esc Exit]",
                        Style::default().fg(Color::DarkGray),
                    )]));

                    for (i, model_str) in selected_models.iter().enumerate() {
                        let is_current = i == self.state.selected_model_index;

                        // Parse model:reasoning format
                        let (model, reasoning) = if model_str.contains(':') {
                            let parts: Vec<&str> = model_str.split(':').collect();
                            (parts[0].to_string(), Some(parts[1].to_uppercase()))
                        } else {
                            (model_str.clone(), None)
                        };

                        let display_name = stage_details::get_model_display_name(&model);
                        let tier = stage_details::get_model_tier_public(&model);
                        let role = stage_details::get_model_role(selected_stage, &model);

                        let style = if is_current {
                            Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD)
                        } else {
                            Style::default()
                        };

                        let marker = if is_current { ">" } else { " " };

                        // Build slot line with optional reasoning badge
                        let mut spans = vec![
                            Span::styled(format!("  {} [{}] ", marker, i + 1), style),
                            Span::styled(display_name, style),
                        ];

                        // Add reasoning badge if present
                        if let Some(reasoning_level) = reasoning {
                            spans.push(Span::styled(
                                format!(" [{}]", reasoning_level),
                                Style::default().fg(Color::Magenta).add_modifier(Modifier::BOLD),
                            ));
                        }

                        spans.push(Span::styled(format!(" - {}", role), Style::default().fg(Color::Yellow)));
                        spans.push(Span::styled(format!(" ({})", tier), Style::default().fg(Color::DarkGray)));

                        detail_lines.push(Line::from(spans));
                    }
                } else {
                    // View mode: show current selection
                    detail_lines.push(Line::from(vec![Span::styled(
                        "  [Press Enter or 'm' to configure]",
                        Style::default().fg(Color::DarkGray),
                    )]));

                    for (i, model_str) in selected_models.iter().enumerate() {
                        // Parse model:reasoning format
                        let (model, reasoning) = if model_str.contains(':') {
                            let parts: Vec<&str> = model_str.split(':').collect();
                            (parts[0].to_string(), Some(parts[1].to_uppercase()))
                        } else {
                            (model_str.clone(), None)
                        };

                        let display_name = stage_details::get_model_display_name(&model);
                        let tier = stage_details::get_model_tier_public(&model);
                        let role = stage_details::get_model_role(selected_stage, &model);

                        let mut spans = vec![
                            Span::styled(format!("  [{}] ", i + 1), Style::default().fg(Color::DarkGray)),
                            Span::styled(display_name, Style::default().fg(Color::Cyan)),
                        ];

                        // Add reasoning badge if present
                        if let Some(reasoning_level) = reasoning {
                            spans.push(Span::styled(
                                format!(" [{}]", reasoning_level),
                                Style::default().fg(Color::Magenta).add_modifier(Modifier::BOLD),
                            ));
                        }

                        spans.push(Span::styled(format!(" - {}", role), Style::default().fg(Color::Yellow)));
                        spans.push(Span::styled(format!(" ({})", tier), Style::default().fg(Color::DarkGray)));

                        detail_lines.push(Line::from(spans));
                    }
                }

                detail_lines.push(Line::raw(""));
            }
        }

        // Warnings section
        if !self.state.warnings.is_empty() {
            detail_lines.push(Line::from(vec![Span::styled(
                "Warnings:",
                Style::default().fg(Color::Red),
            )]));
            for warning in &self.state.warnings {
                let style = if warning.starts_with("Error:") {
                    Style::default().fg(Color::Red)
                } else {
                    Style::default().fg(Color::Yellow)
                };
                detail_lines.push(Line::from(vec![Span::styled(warning, style)]));
            }
            detail_lines.push(Line::raw(""));
        }

        // Help text (context-sensitive)
        detail_lines.push(Line::raw(""));
        let help_text = if self.state.reasoning_picker_mode {
            "Keys: ↑↓ Navigate Reasoning Levels | Enter Select | Esc Back to Models"
        } else if self.state.model_picker_mode {
            "Keys: ↑↓ Navigate ALL Models | Enter Select | Esc Cancel"
        } else if self.state.model_selection_mode {
            "Keys: ↑↓ Navigate Slots | Enter Change Model | m/Esc Exit"
        } else if self.state.view_mode == ViewMode::StageDetails {
            "Keys: m/Enter Configure Models | Esc Back to List | q Save"
        } else {
            "Keys: ↑↓ Navigate | Space Toggle | Enter Details | q Save | Esc Cancel"
        };
        detail_lines.push(Line::from(vec![Span::styled(
            help_text,
            Style::default().fg(Color::Gray),
        )]));

        let details_paragraph = Paragraph::new(detail_lines)
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .title("Stage Details"),
            )
            .wrap(Wrap { trim: true });
        details_paragraph.render(chunks[1], buf);
    }
}

