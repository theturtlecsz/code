//! Modal view for editing global stage→agent defaults (SPEC-KIT-983)
//!
//! Allows users to configure which agent handles each pipeline stage.
//! Changes persist to root config.toml under [speckit.stage_agents],
//! ensuring Tier-1 parity between TUI and CLI/headless modes.

use crossterm::event::{KeyCode, KeyEvent};
use ratatui::buffer::Buffer;
use ratatui::layout::{Alignment, Rect};
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Clear, Paragraph, Widget};

use crate::app_event::AppEvent;
use crate::app_event_sender::AppEventSender;
use codex_core::config_types::SpecKitStageAgents;

use super::bottom_pane_view::BottomPaneView;
use super::{BottomPane, CancellationEvent};

/// Ordered list of stages matching the spec.
const STAGES: &[&str] = &[
    "specify",
    "plan",
    "tasks",
    "implement",
    "validate",
    "audit",
    "unlock",
    "clarify",
    "analyze",
    "checklist",
];

/// Agent options: display name and config value (empty = Default/remove key).
const AGENT_OPTIONS: &[(&str, &str)] = &[
    ("Default", ""),
    ("gpt_pro", "gpt_pro"),
    ("gpt_codex", "gpt_codex"),
    ("gemini", "gemini"),
    ("claude", "claude"),
    ("code", "code"),
];

/// Default agent for each stage when not overridden.
fn default_agent_for_stage(stage: &str) -> &'static str {
    match stage {
        "implement" => "gpt_codex",
        _ => "gpt_pro",
    }
}

/// Row representing a stage with its current agent override.
struct StageRow {
    /// Stage name (e.g., "plan")
    name: &'static str,
    /// Current override value (None = use default)
    current: Option<String>,
}

impl StageRow {
    /// Get the display text for the current agent.
    fn display_agent(&self) -> &str {
        self.current.as_deref().unwrap_or("")
    }

    /// Get the effective agent (override or default).
    fn effective_agent(&self) -> &str {
        self.current
            .as_deref()
            .unwrap_or_else(|| default_agent_for_stage(self.name))
    }

    /// Find index in AGENT_OPTIONS for current value.
    fn agent_index(&self) -> usize {
        let val = self.display_agent();
        AGENT_OPTIONS
            .iter()
            .position(|(_, v)| *v == val)
            .unwrap_or(0)
    }

    /// Cycle to next agent option.
    fn cycle_next(&mut self) {
        let idx = self.agent_index();
        let new_idx = (idx + 1) % AGENT_OPTIONS.len();
        let (_, val) = AGENT_OPTIONS[new_idx];
        self.current = if val.is_empty() {
            None
        } else {
            Some(val.to_string())
        };
    }

    /// Cycle to previous agent option.
    fn cycle_prev(&mut self) {
        let idx = self.agent_index();
        let new_idx = if idx == 0 {
            AGENT_OPTIONS.len() - 1
        } else {
            idx - 1
        };
        let (_, val) = AGENT_OPTIONS[new_idx];
        self.current = if val.is_empty() {
            None
        } else {
            Some(val.to_string())
        };
    }

    /// Set to default (remove override).
    fn set_default(&mut self) {
        self.current = None;
    }
}

/// Modal view for configuring stage→agent defaults.
pub(crate) struct SpecKitStageAgentsView {
    stages: Vec<StageRow>,
    selected_idx: usize,
    is_complete: bool,
    app_event_tx: AppEventSender,
}

impl SpecKitStageAgentsView {
    pub fn new(config: SpecKitStageAgents, app_event_tx: AppEventSender) -> Self {
        let stages = vec![
            StageRow {
                name: "specify",
                current: config.specify,
            },
            StageRow {
                name: "plan",
                current: config.plan,
            },
            StageRow {
                name: "tasks",
                current: config.tasks,
            },
            StageRow {
                name: "implement",
                current: config.implement,
            },
            StageRow {
                name: "validate",
                current: config.validate,
            },
            StageRow {
                name: "audit",
                current: config.audit,
            },
            StageRow {
                name: "unlock",
                current: config.unlock,
            },
            StageRow {
                name: "clarify",
                current: config.clarify,
            },
            StageRow {
                name: "analyze",
                current: config.analyze,
            },
            StageRow {
                name: "checklist",
                current: config.checklist,
            },
        ];
        Self {
            stages,
            selected_idx: 0,
            is_complete: false,
            app_event_tx,
        }
    }

    fn move_up(&mut self) {
        if self.stages.is_empty() {
            return;
        }
        if self.selected_idx == 0 {
            self.selected_idx = self.stages.len() - 1;
        } else {
            self.selected_idx -= 1;
        }
    }

    fn move_down(&mut self) {
        if self.stages.is_empty() {
            return;
        }
        self.selected_idx = (self.selected_idx + 1) % self.stages.len();
    }

    fn cycle_agent_next(&mut self) {
        if let Some(row) = self.stages.get_mut(self.selected_idx) {
            row.cycle_next();
        }
    }

    fn cycle_agent_prev(&mut self) {
        if let Some(row) = self.stages.get_mut(self.selected_idx) {
            row.cycle_prev();
        }
    }

    fn set_selected_to_default(&mut self) {
        if let Some(row) = self.stages.get_mut(self.selected_idx) {
            row.set_default();
        }
    }

    fn reset_all_to_default(&mut self) {
        for row in &mut self.stages {
            row.set_default();
        }
    }

    fn save_and_close(&mut self) {
        // Build SpecKitStageAgents from current state
        let config = SpecKitStageAgents {
            specify: self.stages[0].current.clone(),
            plan: self.stages[1].current.clone(),
            tasks: self.stages[2].current.clone(),
            implement: self.stages[3].current.clone(),
            validate: self.stages[4].current.clone(),
            audit: self.stages[5].current.clone(),
            unlock: self.stages[6].current.clone(),
            clarify: self.stages[7].current.clone(),
            analyze: self.stages[8].current.clone(),
            checklist: self.stages[9].current.clone(),
        };

        self.app_event_tx
            .send(AppEvent::UpdateSpecKitStageAgents(config));
        self.is_complete = true;
    }
}

impl BottomPaneView<'_> for SpecKitStageAgentsView {
    fn handle_key_event(&mut self, _pane: &mut BottomPane<'_>, key_event: KeyEvent) {
        match key_event.code {
            KeyCode::Up | KeyCode::Char('k') => self.move_up(),
            KeyCode::Down | KeyCode::Char('j') => self.move_down(),
            KeyCode::Right | KeyCode::Char('l') => self.cycle_agent_next(),
            KeyCode::Left | KeyCode::Char('h') => self.cycle_agent_prev(),
            KeyCode::Char('d') => self.set_selected_to_default(),
            KeyCode::Char('r') => self.reset_all_to_default(),
            KeyCode::Enter | KeyCode::Char('s') => self.save_and_close(),
            KeyCode::Esc => self.is_complete = true,
            _ => {}
        }
    }

    fn is_complete(&self) -> bool {
        self.is_complete
    }

    fn on_ctrl_c(&mut self, _pane: &mut BottomPane<'_>) -> CancellationEvent {
        self.is_complete = true;
        CancellationEvent::Handled
    }

    fn desired_height(&self, _width: u16) -> u16 {
        // Title (1) + blank (1) + header (1) + separator (1) + 10 stages + blank (1) + footer (1) + borders (2)
        18
    }

    fn render(&self, area: Rect, buf: &mut Buffer) {
        Clear.render(area, buf);

        let block = Block::default()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(crate::colors::border()))
            .style(
                Style::default()
                    .bg(crate::colors::background())
                    .fg(crate::colors::text()),
            )
            .title(" Stage Agent Configuration (Global) ")
            .title_alignment(Alignment::Center);

        let inner = block.inner(area);
        block.render(area, buf);

        let mut lines: Vec<Line<'static>> = Vec::new();

        // Blank line after title
        lines.push(Line::from(""));

        // Header row
        lines.push(Line::from(vec![
            Span::styled(
                format!("  {:<12} ", "Stage"),
                Style::default().add_modifier(Modifier::BOLD),
            ),
            Span::styled(
                format!("{:<12} ", "Agent"),
                Style::default().add_modifier(Modifier::BOLD),
            ),
            Span::raw(""),
        ]));

        // Separator
        lines.push(Line::from("  ──────────── ────────────"));

        // Stage rows
        for (idx, row) in self.stages.iter().enumerate() {
            let is_selected = idx == self.selected_idx;
            let is_default = row.current.is_none();

            let prefix = if is_selected { "▸ " } else { "  " };

            let stage_style = if is_selected {
                Style::default()
                    .fg(crate::colors::primary())
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default()
            };

            let agent_style = if is_selected {
                Style::default()
                    .bg(crate::colors::selection())
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default()
            };

            let effective = row.effective_agent();
            let agent_display = format!("[{:<10}]", effective);

            let default_marker = if is_default { " ◀ Default" } else { "" };
            let marker_style = Style::default().fg(crate::colors::text_dim());

            lines.push(Line::from(vec![
                Span::styled(prefix.to_string(), stage_style),
                Span::styled(format!("{:<12} ", row.name), stage_style),
                Span::styled(agent_display, agent_style),
                Span::styled(default_marker.to_string(), marker_style),
            ]));
        }

        // Blank line before footer
        lines.push(Line::from(""));

        // Footer with keybinds
        let footer_style = Style::default().fg(crate::colors::text_dim());
        lines.push(Line::from(Span::styled(
            "  ↑↓ select  ←→ change  d=default  r=reset all  s/Enter=save  Esc=cancel",
            footer_style,
        )));

        let paragraph = Paragraph::new(lines).alignment(Alignment::Left).style(
            Style::default()
                .bg(crate::colors::background())
                .fg(crate::colors::text()),
        );

        paragraph.render(inner, buf);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn stage_row_cycle_wraps() {
        let mut row = StageRow {
            name: "plan",
            current: None,
        };
        // Should start at Default (index 0)
        assert_eq!(row.agent_index(), 0);

        // Cycle forward through all options
        row.cycle_next();
        assert_eq!(row.current.as_deref(), Some("gpt_pro"));

        row.cycle_next();
        assert_eq!(row.current.as_deref(), Some("gpt_codex"));

        // Continue to end and wrap
        row.cycle_next(); // gemini
        row.cycle_next(); // claude
        row.cycle_next(); // code
        row.cycle_next(); // back to Default
        assert_eq!(row.current, None);
    }

    #[test]
    fn stage_row_cycle_prev_wraps() {
        let mut row = StageRow {
            name: "plan",
            current: None,
        };
        // Cycle backward from Default should go to last option (code)
        row.cycle_prev();
        assert_eq!(row.current.as_deref(), Some("code"));
    }

    #[test]
    fn default_agent_for_implement_is_codex() {
        assert_eq!(default_agent_for_stage("implement"), "gpt_codex");
        assert_eq!(default_agent_for_stage("plan"), "gpt_pro");
    }
}
