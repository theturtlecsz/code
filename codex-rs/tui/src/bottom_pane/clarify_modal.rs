//! Clarify modal for interactive clarification resolution (SPEC-KIT-971)
//!
//! Displays [NEEDS CLARIFICATION: ...] markers one at a time and collects
//! freeform answers from the user. Simpler than PrdBuilderModal - no predefined
//! options, just freeform text input.

use crossterm::event::{KeyCode, KeyEvent, KeyEventKind};
use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::prelude::*;
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, BorderType, Borders, Clear, Paragraph, Wrap};
use std::path::PathBuf;

use crate::app_event::AppEvent;
use crate::app_event_sender::AppEventSender;

use super::{BottomPane, BottomPaneView, CancellationEvent};

/// Marker info passed to the modal
#[derive(Debug, Clone)]
pub(crate) struct ClarifyQuestion {
    pub id: String,
    pub question: String,
    pub file_path: PathBuf,
    pub line_number: usize,
    pub original_text: String,
}

/// Modal state for clarification questions
pub(crate) struct ClarifyModal {
    spec_id: String,
    questions: Vec<ClarifyQuestion>,
    current_index: usize,
    answers: Vec<String>,
    current_input: String,
    app_event_tx: AppEventSender,
    done: bool,
}

impl ClarifyModal {
    pub fn new(
        spec_id: String,
        questions: Vec<ClarifyQuestion>,
        app_event_tx: AppEventSender,
    ) -> Self {
        Self {
            spec_id,
            questions,
            current_index: 0,
            answers: Vec::new(),
            current_input: String::new(),
            app_event_tx,
            done: false,
        }
    }

    fn current_question(&self) -> Option<&ClarifyQuestion> {
        self.questions.get(self.current_index)
    }

    fn total_questions(&self) -> usize {
        self.questions.len()
    }

    fn is_last_question(&self) -> bool {
        self.current_index >= self.questions.len().saturating_sub(1)
    }

    fn submit_answer(&mut self) {
        if self.current_input.trim().is_empty() {
            return; // Don't accept empty answers
        }

        self.answers.push(self.current_input.trim().to_string());
        self.current_input.clear();

        if self.is_last_question() {
            // All questions answered - send completion event
            self.send_completion_event();
            self.done = true;
        } else {
            // Move to next question
            self.current_index += 1;
        }
    }

    fn send_completion_event(&self) {
        // Build resolutions: pair each question with its answer
        let resolutions: Vec<(ClarifyQuestion, String)> = self
            .questions
            .iter()
            .zip(self.answers.iter())
            .map(|(q, a)| (q.clone(), a.clone()))
            .collect();

        self.app_event_tx.send(AppEvent::ClarifySubmitted {
            spec_id: self.spec_id.clone(),
            resolutions,
        });
    }

    fn handle_escape(&mut self) {
        // Cancel entire clarify session
        self.done = true;
        self.app_event_tx.send(AppEvent::ClarifyCancelled {
            spec_id: self.spec_id.clone(),
        });
    }

    fn handle_text_input(&mut self, c: char) {
        if !self.done {
            self.current_input.push(c);
        }
    }

    fn handle_backspace(&mut self) {
        if !self.done {
            self.current_input.pop();
        }
    }

    fn skip_question(&mut self) {
        // Skip with the original marker text (no change)
        if let Some(q) = self.current_question() {
            self.answers.push(q.original_text.clone());
        }
        self.current_input.clear();

        if self.is_last_question() {
            self.send_completion_event();
            self.done = true;
        } else {
            self.current_index += 1;
        }
    }
}

impl BottomPaneView<'_> for ClarifyModal {
    fn handle_key_event(&mut self, _pane: &mut BottomPane, key_event: KeyEvent) {
        if self.done {
            return;
        }

        if matches!(key_event.kind, KeyEventKind::Press | KeyEventKind::Repeat) {
            match key_event.code {
                KeyCode::Char(c) => self.handle_text_input(c),
                KeyCode::Backspace => self.handle_backspace(),
                KeyCode::Enter => self.submit_answer(),
                KeyCode::Esc => self.handle_escape(),
                KeyCode::Tab => self.skip_question(), // Tab to skip
                _ => {}
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
        14 // Fixed height for modal
    }

    fn render(&self, area: Rect, buf: &mut Buffer) {
        // Clear background
        Clear.render(area, buf);

        // Main border
        let block = Block::default()
            .borders(Borders::ALL)
            .border_type(BorderType::Double)
            .border_style(Style::default().fg(Color::Yellow))
            .title(format!(
                " Resolve Clarifications [{}/{}] ",
                self.current_index + 1,
                self.total_questions()
            ));

        let inner = block.inner(area);
        block.render(area, buf);

        let Some(question) = self.current_question() else {
            return;
        };

        let mut lines = Vec::new();

        // SPEC ID
        lines.push(Line::from(vec![
            Span::styled("SPEC: ", Style::default().dim()),
            Span::styled(&self.spec_id, Style::default().fg(Color::Cyan)),
        ]));
        lines.push(Line::from(""));

        // Question ID badge
        lines.push(Line::from(vec![Span::styled(
            format!(" {} ", question.id),
            Style::default().fg(Color::Black).bg(Color::Yellow).bold(),
        )]));
        lines.push(Line::from(""));

        // Question text
        lines.push(Line::from(vec![Span::styled(
            &question.question,
            Style::default().fg(Color::White).bold(),
        )]));
        lines.push(Line::from(""));

        // File location (dimmed)
        let filename = question
            .file_path
            .file_name()
            .map(|s| s.to_string_lossy().to_string())
            .unwrap_or_else(|| "unknown".to_string());
        lines.push(Line::from(vec![
            Span::styled("Location: ", Style::default().dim()),
            Span::styled(
                format!("{}:{}", filename, question.line_number),
                Style::default().fg(Color::Gray),
            ),
        ]));
        lines.push(Line::from(""));

        // Input field
        lines.push(Line::from(vec![Span::styled(
            "Your answer: ",
            Style::default().fg(Color::Cyan).bold(),
        )]));
        lines.push(Line::from(vec![
            Span::styled(&self.current_input, Style::default().fg(Color::White)),
            Span::styled("_", Style::default().fg(Color::Gray)), // Cursor
        ]));
        lines.push(Line::from(""));

        // Footer hints
        lines.push(Line::from(vec![
            Span::styled("[Enter]", Style::default().fg(Color::Green)),
            Span::raw(" Submit  "),
            Span::styled("[Tab]", Style::default().fg(Color::Yellow)),
            Span::raw(" Skip  "),
            Span::styled("[Esc]", Style::default().fg(Color::Red)),
            Span::raw(" Cancel"),
        ]));

        let paragraph = Paragraph::new(lines)
            .wrap(Wrap { trim: false })
            .scroll((0, 0));

        paragraph.render(inner, buf);
    }
}
