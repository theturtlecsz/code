//! PRD builder modal for interactive spec creation (SPEC-KIT-970)
//!
//! Displays required questions before generating a PRD:
//! 1. Problem - What problem does this solve?
//! 2. Target User - Who is the primary user?
//! 3. Success Criteria - How will you know it's complete?

use crossterm::event::{KeyCode, KeyEvent, KeyEventKind};
use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::prelude::*;
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, BorderType, Borders, Clear, Paragraph, Wrap};

use crate::app_event::AppEvent;
use crate::app_event_sender::AppEventSender;

use super::{BottomPane, BottomPaneView, CancellationEvent};

/// Question with predefined options
#[derive(Clone)]
pub(crate) struct PrdQuestion {
    pub category: &'static str,
    pub question: &'static str,
    pub options: Vec<PrdOption>,
}

/// Option for a question
#[derive(Clone)]
pub(crate) struct PrdOption {
    pub label: char,
    pub text: &'static str,
    pub is_custom: bool,
}

/// Modal state for PRD builder questions
pub(crate) struct PrdBuilderModal {
    description: String,
    questions: Vec<PrdQuestion>,
    current_index: usize,
    answers: Vec<String>,
    current_input: String,
    custom_mode: bool,
    app_event_tx: AppEventSender,
    done: bool,
}

impl PrdBuilderModal {
    pub fn new(description: String, app_event_tx: AppEventSender) -> Self {
        let questions = vec![
            PrdQuestion {
                category: "Problem",
                question: "What problem does this solve?",
                options: vec![
                    PrdOption { label: 'A', text: "Performance issue", is_custom: false },
                    PrdOption { label: 'B', text: "Missing functionality", is_custom: false },
                    PrdOption { label: 'C', text: "Developer experience", is_custom: false },
                    PrdOption { label: 'D', text: "Custom...", is_custom: true },
                ],
            },
            PrdQuestion {
                category: "Target",
                question: "Who is the primary user?",
                options: vec![
                    PrdOption { label: 'A', text: "Developer", is_custom: false },
                    PrdOption { label: 'B', text: "End-user", is_custom: false },
                    PrdOption { label: 'C', text: "Admin/Operator", is_custom: false },
                    PrdOption { label: 'D', text: "Custom...", is_custom: true },
                ],
            },
            PrdQuestion {
                category: "Success",
                question: "How will you know it's complete?",
                options: vec![
                    PrdOption { label: 'A', text: "Tests pass", is_custom: false },
                    PrdOption { label: 'B', text: "Feature works end-to-end", is_custom: false },
                    PrdOption { label: 'C', text: "Performance target met", is_custom: false },
                    PrdOption { label: 'D', text: "Custom...", is_custom: true },
                ],
            },
        ];

        Self {
            description,
            questions,
            current_index: 0,
            answers: Vec::new(),
            current_input: String::new(),
            custom_mode: false,
            app_event_tx,
            done: false,
        }
    }

    fn current_question(&self) -> Option<&PrdQuestion> {
        self.questions.get(self.current_index)
    }

    fn total_questions(&self) -> usize {
        self.questions.len()
    }

    fn is_last_question(&self) -> bool {
        self.current_index >= self.questions.len().saturating_sub(1)
    }

    fn select_option(&mut self, label: char) {
        if let Some(question) = self.current_question() {
            if let Some(option) = question.options.iter().find(|o| o.label == label) {
                if option.is_custom {
                    // Enter custom input mode
                    self.custom_mode = true;
                    self.current_input.clear();
                } else {
                    // Use predefined answer
                    self.submit_answer(option.text.to_string());
                }
            }
        }
    }

    fn submit_answer(&mut self, answer: String) {
        self.answers.push(answer);
        self.custom_mode = false;
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

    fn submit_custom_answer(&mut self) {
        if !self.current_input.trim().is_empty() {
            let answer = self.current_input.trim().to_string();
            self.submit_answer(answer);
        }
    }

    fn send_completion_event(&self) {
        // Build answers map
        let mut answers_map = std::collections::HashMap::new();
        for (i, answer) in self.answers.iter().enumerate() {
            if let Some(q) = self.questions.get(i) {
                answers_map.insert(q.category.to_string(), answer.clone());
            }
        }

        self.app_event_tx.send(AppEvent::PrdBuilderSubmitted {
            description: self.description.clone(),
            answers: answers_map,
        });
    }

    fn handle_escape(&mut self) {
        if self.custom_mode {
            // Exit custom mode, go back to options
            self.custom_mode = false;
            self.current_input.clear();
        } else {
            // Cancel entire PRD builder
            self.done = true;
            self.app_event_tx.send(AppEvent::PrdBuilderCancelled {
                description: self.description.clone(),
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

impl BottomPaneView<'_> for PrdBuilderModal {
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
                match key_event.code {
                    KeyCode::Char('a') | KeyCode::Char('A') => self.select_option('A'),
                    KeyCode::Char('b') | KeyCode::Char('B') => self.select_option('B'),
                    KeyCode::Char('c') | KeyCode::Char('C') => self.select_option('C'),
                    KeyCode::Char('d') | KeyCode::Char('D') => self.select_option('D'),
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
            12
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
                " Interactive PRD Builder [{}/{}] ",
                self.current_index + 1,
                self.total_questions()
            ));

        let inner = block.inner(area);
        block.render(area, buf);

        let Some(question) = self.current_question() else {
            return;
        };

        let mut lines = Vec::new();

        // Description (truncated)
        let desc_display = if self.description.len() > 50 {
            format!("{}...", &self.description[..47])
        } else {
            self.description.clone()
        };
        lines.push(Line::from(vec![
            Span::styled("Feature: ", Style::default().dim()),
            Span::styled(desc_display, Style::default().fg(Color::White)),
        ]));
        lines.push(Line::from(""));

        // Category badge
        lines.push(Line::from(vec![
            Span::styled(
                format!(" {} ", question.category.to_uppercase()),
                Style::default().fg(Color::Black).bg(Color::Cyan).bold(),
            ),
        ]));
        lines.push(Line::from(""));

        // Question
        let question_text = question.question.to_string();
        lines.push(Line::from(vec![
            Span::styled(question_text, Style::default().fg(Color::Yellow).bold()),
        ]));
        lines.push(Line::from(""));

        if self.custom_mode {
            // Custom input mode
            lines.push(Line::from(vec![
                Span::styled("Your answer: ", Style::default().fg(Color::Cyan).bold()),
            ]));
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
                let style = if option.is_custom {
                    Style::default().fg(Color::Magenta)
                } else {
                    Style::default().fg(Color::Green)
                };
                lines.push(Line::from(vec![
                    Span::styled(format!("  [{}] ", option.label), style.bold()),
                    Span::raw(option.text),
                ]));
            }
            lines.push(Line::from(""));

            // Footer hints
            lines.push(Line::from(vec![
                Span::styled("[A-D]", Style::default().fg(Color::Green)),
                Span::raw(" Select  "),
                Span::styled("[Esc]", Style::default().fg(Color::Red)),
                Span::raw(" Cancel"),
            ]));
        }

        let paragraph = Paragraph::new(lines)
            .wrap(Wrap { trim: false })
            .scroll((0, 0));

        paragraph.render(inner, buf);
    }
}
