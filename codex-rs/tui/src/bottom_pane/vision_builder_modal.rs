//! Vision builder modal for guided constitution creation (P93/SPEC-KIT-105)
//!
//! Displays Q&A wizard to capture project vision, goals, non-goals, and principles.
//! Answers are mapped to constitution memories with appropriate types and priorities.

use crossterm::event::{KeyCode, KeyEvent, KeyEventKind};
use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::prelude::*;
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, BorderType, Borders, Clear, Paragraph, Wrap};

use crate::app_event::AppEvent;
use crate::app_event_sender::AppEventSender;

use super::{BottomPane, BottomPaneView, CancellationEvent};

/// Vision question with predefined options
#[derive(Clone)]
pub(crate) struct VisionQuestion {
    pub category: &'static str,
    pub question: &'static str,
    pub hint: &'static str,
    pub options: Vec<VisionOption>,
    pub allow_multiple: bool,
}

/// Option for a vision question
#[derive(Clone)]
pub(crate) struct VisionOption {
    pub label: char,
    pub text: &'static str,
    pub is_custom: bool,
}

/// Modal state for vision builder Q&A wizard
pub(crate) struct VisionBuilderModal {
    questions: Vec<VisionQuestion>,
    current_index: usize,
    answers: Vec<String>,
    current_input: String,
    custom_mode: bool,
    app_event_tx: AppEventSender,
    done: bool,
}

impl VisionBuilderModal {
    /// Create modal with vision-specific questions
    pub fn new(app_event_tx: AppEventSender) -> Self {
        let questions = vec![
            VisionQuestion {
                category: "Users",
                question: "Who is this project for?",
                hint: "Describe your target users/audience",
                options: vec![
                    VisionOption {
                        label: 'A',
                        text: "Developers building applications",
                        is_custom: false,
                    },
                    VisionOption {
                        label: 'B',
                        text: "End-users of the application",
                        is_custom: false,
                    },
                    VisionOption {
                        label: 'C',
                        text: "Internal team members",
                        is_custom: false,
                    },
                    VisionOption {
                        label: 'D',
                        text: "Custom...",
                        is_custom: true,
                    },
                ],
                allow_multiple: false,
            },
            VisionQuestion {
                category: "Problem",
                question: "What problem does this project solve?",
                hint: "The core pain point or challenge",
                options: vec![
                    VisionOption {
                        label: 'A',
                        text: "Improves developer productivity",
                        is_custom: false,
                    },
                    VisionOption {
                        label: 'B',
                        text: "Automates manual workflows",
                        is_custom: false,
                    },
                    VisionOption {
                        label: 'C',
                        text: "Reduces operational complexity",
                        is_custom: false,
                    },
                    VisionOption {
                        label: 'D',
                        text: "Custom...",
                        is_custom: true,
                    },
                ],
                allow_multiple: false,
            },
            VisionQuestion {
                category: "Goals",
                question: "What are the primary goals? (3-5 success criteria)",
                hint: "Enter each goal separated by semicolons",
                options: vec![
                    VisionOption {
                        label: 'A',
                        text: "High performance; Scalability; Reliability",
                        is_custom: false,
                    },
                    VisionOption {
                        label: 'B',
                        text: "Easy to use; Well documented; Maintainable",
                        is_custom: false,
                    },
                    VisionOption {
                        label: 'C',
                        text: "Security; Compliance; Auditability",
                        is_custom: false,
                    },
                    VisionOption {
                        label: 'D',
                        text: "Custom...",
                        is_custom: true,
                    },
                ],
                allow_multiple: false,
            },
            VisionQuestion {
                category: "NonGoals",
                question: "What do we explicitly NOT build?",
                hint: "Enter each non-goal separated by semicolons",
                options: vec![
                    VisionOption {
                        label: 'A',
                        text: "No UI components; No mobile support",
                        is_custom: false,
                    },
                    VisionOption {
                        label: 'B',
                        text: "No backwards compatibility; No legacy support",
                        is_custom: false,
                    },
                    VisionOption {
                        label: 'C',
                        text: "No third-party integrations; No custom branding",
                        is_custom: false,
                    },
                    VisionOption {
                        label: 'D',
                        text: "Custom...",
                        is_custom: true,
                    },
                ],
                allow_multiple: false,
            },
            VisionQuestion {
                category: "Principles",
                question: "What are the key design principles?",
                hint: "Architectural values and constraints",
                options: vec![
                    VisionOption {
                        label: 'A',
                        text: "Simplicity over features; Convention over configuration",
                        is_custom: false,
                    },
                    VisionOption {
                        label: 'B',
                        text: "Type safety; Explicit error handling; Immutability",
                        is_custom: false,
                    },
                    VisionOption {
                        label: 'C',
                        text: "Test-driven; Documentation-first; API stability",
                        is_custom: false,
                    },
                    VisionOption {
                        label: 'D',
                        text: "Custom...",
                        is_custom: true,
                    },
                ],
                allow_multiple: false,
            },
        ];

        Self {
            questions,
            current_index: 0,
            answers: Vec::new(),
            current_input: String::new(),
            custom_mode: false,
            app_event_tx,
            done: false,
        }
    }

    fn current_question(&self) -> Option<&VisionQuestion> {
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

        self.app_event_tx.send(AppEvent::VisionBuilderSubmitted {
            answers: answers_map,
        });
    }

    fn handle_escape(&mut self) {
        if self.custom_mode {
            // Exit custom mode, go back to options
            self.custom_mode = false;
            self.current_input.clear();
        } else {
            // Cancel entire vision builder
            self.done = true;
            self.app_event_tx.send(AppEvent::VisionBuilderCancelled);
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

impl BottomPaneView<'_> for VisionBuilderModal {
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
        // Title + blank + question + hint + blank + 4 options + blank + footer
        if self.custom_mode {
            16 // Extra space for input
        } else {
            14
        }
    }

    fn render(&self, area: Rect, buf: &mut Buffer) {
        // Clear background
        Clear.render(area, buf);

        // Main border
        let block = Block::default()
            .borders(Borders::ALL)
            .border_type(BorderType::Double)
            .border_style(Style::default().fg(Color::Magenta))
            .title(format!(
                " Vision Q&A [{}/{}] ",
                self.current_index + 1,
                self.total_questions()
            ));

        let inner = block.inner(area);
        block.render(area, buf);

        let Some(question) = self.current_question() else {
            return;
        };

        let mut lines = Vec::new();

        // Header
        lines.push(Line::from(vec![
            Span::styled(
                "Project Vision Builder",
                Style::default().fg(Color::Yellow).bold(),
            ),
            Span::styled(" - ", Style::default().dim()),
            Span::styled(
                "Guided constitution creation",
                Style::default().fg(Color::White).dim(),
            ),
        ]));
        lines.push(Line::from(""));

        // Category badge
        lines.push(Line::from(vec![Span::styled(
            format!(" {} ", question.category.to_uppercase()),
            Style::default().fg(Color::Black).bg(Color::Magenta).bold(),
        )]));
        lines.push(Line::from(""));

        // Question
        let question_text = question.question.to_string();
        lines.push(Line::from(vec![Span::styled(
            question_text,
            Style::default().fg(Color::Yellow).bold(),
        )]));

        // Hint
        lines.push(Line::from(vec![Span::styled(
            format!("  {}", question.hint),
            Style::default().fg(Color::Gray).italic(),
        )]));
        lines.push(Line::from(""));

        if self.custom_mode {
            // Custom input mode
            lines.push(Line::from(vec![Span::styled(
                "Your answer: ",
                Style::default().fg(Color::Magenta).bold(),
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
