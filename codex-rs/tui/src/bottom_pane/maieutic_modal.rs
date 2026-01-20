//! Maieutic elicitation modal for pre-flight clarification (D130)
//!
//! Displays mandatory questions before automation proceeds.
//! Captures the delegation contract for the pipeline run.

use crossterm::event::{KeyCode, KeyEvent, KeyEventKind};
use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::prelude::*;
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, BorderType, Borders, Clear, Paragraph, Wrap};
use std::collections::HashMap;

use crate::app_event::AppEvent;
use crate::app_event_sender::AppEventSender;
use crate::chatwidget::spec_kit::maieutic::MaieuticQuestion;

use super::{BottomPane, BottomPaneView, CancellationEvent};

/// Modal state for maieutic elicitation
pub(crate) struct MaieuticModal {
    spec_id: String,
    questions: Vec<MaieuticQuestion>,
    current_index: usize,
    answers: HashMap<String, String>,
    current_input: String,
    custom_mode: bool,
    selected_options: Vec<char>, // For multi-select questions
    app_event_tx: AppEventSender,
    done: bool,
    started_at: std::time::Instant,
}

impl MaieuticModal {
    /// Create modal with maieutic questions
    pub fn new(
        spec_id: String,
        questions: Vec<MaieuticQuestion>,
        app_event_tx: AppEventSender,
    ) -> Self {
        Self {
            spec_id,
            questions,
            current_index: 0,
            answers: HashMap::new(),
            current_input: String::new(),
            custom_mode: false,
            selected_options: Vec::new(),
            app_event_tx,
            done: false,
            started_at: std::time::Instant::now(),
        }
    }

    fn current_question(&self) -> Option<&MaieuticQuestion> {
        self.questions.get(self.current_index)
    }

    fn total_questions(&self) -> usize {
        self.questions.len()
    }

    fn is_last_question(&self) -> bool {
        self.current_index >= self.questions.len().saturating_sub(1)
    }

    fn select_option(&mut self, label: char) {
        let Some(question) = self.current_question() else {
            return;
        };

        let Some(option) = question.options.iter().find(|o| o.label == label) else {
            return;
        };

        if option.is_custom {
            // Enter custom input mode
            self.custom_mode = true;
            self.current_input.clear();
            return;
        }

        if question.multi_select {
            // Toggle selection for multi-select
            if let Some(pos) = self.selected_options.iter().position(|&c| c == label) {
                self.selected_options.remove(pos);
            } else {
                self.selected_options.push(label);
            }
        } else {
            // Single select - use predefined answer immediately
            self.submit_answer(option.text.to_string());
        }
    }

    fn submit_multi_select(&mut self) {
        let Some(question) = self.current_question() else {
            return;
        };

        if self.selected_options.is_empty() {
            // Must select at least one
            return;
        }

        // Collect selected option texts
        let selected_texts: Vec<String> = self
            .selected_options
            .iter()
            .filter_map(|&label| {
                question
                    .options
                    .iter()
                    .find(|o| o.label == label)
                    .map(|o| o.text.to_string())
            })
            .collect();

        let answer = selected_texts.join(", ");
        self.selected_options.clear();
        self.submit_answer(answer);
    }

    fn submit_answer(&mut self, answer: String) {
        if let Some(question) = self.current_question() {
            self.answers.insert(question.id.to_string(), answer);
        }

        self.custom_mode = false;
        self.current_input.clear();
        self.selected_options.clear();

        if self.is_last_question() {
            // All questions answered - send completion event
            self.send_completion_event();
            self.done = true;
        } else {
            // Move to next question
            self.current_index += 1;
        }
    }

    fn submit_custom_answer(&mut self) {
        if !self.current_input.trim().is_empty() {
            let answer = self.current_input.trim().to_string();
            self.submit_answer(answer);
        }
    }

    fn send_completion_event(&self) {
        let duration_ms = self.started_at.elapsed().as_millis() as u64;

        self.app_event_tx.send(AppEvent::MaieuticSubmitted {
            spec_id: self.spec_id.clone(),
            answers: self.answers.clone(),
            duration_ms,
        });
    }

    fn handle_escape(&mut self) {
        if self.custom_mode {
            // Exit custom mode, go back to options
            self.custom_mode = false;
            self.current_input.clear();
        } else {
            // Cancel entire maieutic
            self.done = true;
            self.app_event_tx.send(AppEvent::MaieuticCancelled {
                spec_id: self.spec_id.clone(),
            });
        }
    }

    fn handle_text_input(&mut self, c: char) {
        if self.custom_mode && !self.done {
            self.current_input.push(c);
        }
    }

    fn handle_backspace(&mut self) {
        if self.custom_mode && !self.done {
            self.current_input.pop();
        }
    }
}

impl BottomPaneView<'_> for MaieuticModal {
    fn handle_key_event(&mut self, _pane: &mut BottomPane, key_event: KeyEvent) {
        if self.done {
            return;
        }

        if matches!(key_event.kind, KeyEventKind::Press | KeyEventKind::Repeat) {
            if self.custom_mode {
                // Custom input mode
                match key_event.code {
                    KeyCode::Char(c) => self.handle_text_input(c),
                    KeyCode::Backspace => self.handle_backspace(),
                    KeyCode::Enter => self.submit_custom_answer(),
                    KeyCode::Esc => self.handle_escape(),
                    _ => {}
                }
            } else {
                // Option selection mode
                let is_multi_select = self
                    .current_question()
                    .map(|q| q.multi_select)
                    .unwrap_or(false);

                match key_event.code {
                    KeyCode::Char('a') | KeyCode::Char('A') => self.select_option('A'),
                    KeyCode::Char('b') | KeyCode::Char('B') => self.select_option('B'),
                    KeyCode::Char('c') | KeyCode::Char('C') => self.select_option('C'),
                    KeyCode::Char('d') | KeyCode::Char('D') => self.select_option('D'),
                    KeyCode::Enter if is_multi_select => self.submit_multi_select(),
                    KeyCode::Esc => self.handle_escape(),
                    _ => {}
                }
            }
        }
    }

    fn on_ctrl_c(&mut self, _pane: &mut BottomPane) -> CancellationEvent {
        self.handle_escape();
        CancellationEvent::Handled
    }

    fn is_complete(&self) -> bool {
        self.done
    }

    fn desired_height(&self, _width: u16) -> u16 {
        // Title + blank + question + blank + 4 options + blank + footer
        if self.custom_mode {
            14 // Extra space for input
        } else {
            13
        }
    }

    fn render(&self, area: Rect, buf: &mut Buffer) {
        // Clear background
        Clear.render(area, buf);

        // Main border
        let block = Block::default()
            .borders(Borders::ALL)
            .border_type(BorderType::Double)
            .border_style(Style::default().fg(Color::Cyan))
            .title(format!(
                " Pre-flight Clarification [{}/{}] ",
                self.current_index + 1,
                self.total_questions()
            ));

        let inner = block.inner(area);
        block.render(area, buf);

        let Some(question) = self.current_question() else {
            return;
        };

        let mut lines = Vec::new();

        // SPEC ID display
        lines.push(Line::from(vec![
            Span::styled("SPEC: ", Style::default().dim()),
            Span::styled(&self.spec_id, Style::default().fg(Color::Magenta).bold()),
            Span::styled(
                "  (D130: mandatory maieutic elicitation)",
                Style::default().dim(),
            ),
        ]));
        lines.push(Line::from(""));

        // Category badge
        lines.push(Line::from(vec![Span::styled(
            format!(" {} ", question.category.to_uppercase()),
            Style::default().fg(Color::Black).bg(Color::Cyan).bold(),
        )]));
        lines.push(Line::from(""));

        // Question
        lines.push(Line::from(vec![Span::styled(
            question.text.to_string(),
            Style::default().fg(Color::Yellow).bold(),
        )]));
        lines.push(Line::from(""));

        if self.custom_mode {
            // Custom input mode
            lines.push(Line::from(vec![Span::styled(
                "Your answer: ",
                Style::default().fg(Color::Cyan).bold(),
            )]));
            lines.push(Line::from(vec![
                Span::styled(&self.current_input, Style::default().fg(Color::White)),
                Span::styled("_", Style::default().fg(Color::Gray)), // Cursor
            ]));
            lines.push(Line::from(""));
            lines.push(Line::from(vec![
                Span::styled("[Enter]", Style::default().fg(Color::Green)),
                Span::raw(" Submit  "),
                Span::styled("[Esc]", Style::default().fg(Color::Yellow)),
                Span::raw(" Back to options"),
            ]));
        } else {
            // Options
            for option in &question.options {
                let is_selected = self.selected_options.contains(&option.label);
                let checkbox = if question.multi_select {
                    if is_selected { "[x]" } else { "[ ]" }
                } else {
                    ""
                };

                let style = if option.is_custom {
                    Style::default().fg(Color::Magenta)
                } else if is_selected {
                    Style::default().fg(Color::Green).bold()
                } else {
                    Style::default().fg(Color::Green)
                };

                if question.multi_select {
                    lines.push(Line::from(vec![
                        Span::styled(format!("  {} [{}] ", checkbox, option.label), style.bold()),
                        Span::raw(option.text),
                    ]));
                } else {
                    lines.push(Line::from(vec![
                        Span::styled(format!("  [{}] ", option.label), style.bold()),
                        Span::raw(option.text),
                    ]));
                }
            }
            lines.push(Line::from(""));

            // Footer hints
            if question.multi_select {
                lines.push(Line::from(vec![
                    Span::styled("[A-D]", Style::default().fg(Color::Green)),
                    Span::raw(" Toggle  "),
                    Span::styled("[Enter]", Style::default().fg(Color::Cyan)),
                    Span::raw(" Confirm  "),
                    Span::styled("[Esc]", Style::default().fg(Color::Red)),
                    Span::raw(" Cancel"),
                ]));
            } else {
                lines.push(Line::from(vec![
                    Span::styled("[A-D]", Style::default().fg(Color::Green)),
                    Span::raw(" Select  "),
                    Span::styled("[Esc]", Style::default().fg(Color::Red)),
                    Span::raw(" Cancel"),
                ]));
            }
        }

        let paragraph = Paragraph::new(lines)
            .wrap(Wrap { trim: false })
            .scroll((0, 0));

        paragraph.render(inner, buf);
    }
}
