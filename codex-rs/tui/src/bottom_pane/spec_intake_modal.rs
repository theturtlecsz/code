//! Spec intake modal for "Architect-in-a-box" spec creation.
//!
//! Collects required baseline (and optional --deep) intake answers before
//! any SPEC directory is created.

use crossterm::event::{KeyCode, KeyEvent, KeyEventKind};
use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::prelude::*;
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, BorderType, Borders, Clear, Paragraph, Wrap};

use crate::app_event::AppEvent;
use crate::app_event_sender::AppEventSender;

use super::{BottomPane, BottomPaneView, CancellationEvent};

#[derive(Clone)]
pub(crate) struct SpecIntakeQuestion {
    pub key: &'static str,
    pub title: &'static str,
    pub hint: &'static str,
    pub options: Vec<SpecIntakeOption>,
}

#[derive(Clone)]
pub(crate) struct SpecIntakeOption {
    pub label: char,
    pub text: &'static str,
    pub is_custom: bool,
}

pub(crate) struct SpecIntakeModal {
    description: String,
    deep: bool,
    questions: Vec<SpecIntakeQuestion>,
    current_index: usize,
    answers: Vec<String>,
    current_input: String,
    custom_mode: bool,
    app_event_tx: AppEventSender,
    done: bool,
    /// If Some, this is a backfill for existing spec (don't generate new ID)
    existing_spec_id: Option<String>,
}

impl SpecIntakeModal {
    /// Create a new spec intake modal for creating a new spec
    pub fn new(description: String, deep: bool, app_event_tx: AppEventSender) -> Self {
        Self::new_with_spec_id(description, deep, app_event_tx, None)
    }

    /// Create a new spec intake modal for backfilling an existing spec
    pub fn new_backfill(spec_id: String, app_event_tx: AppEventSender) -> Self {
        Self::new_with_spec_id(
            format!("Backfill intake for {}", spec_id),
            false, // backfill uses baseline questions only
            app_event_tx,
            Some(spec_id),
        )
    }

    fn new_with_spec_id(
        description: String,
        deep: bool,
        app_event_tx: AppEventSender,
        existing_spec_id: Option<String>,
    ) -> Self {
        let mut questions = baseline_questions();
        if deep {
            questions.extend(deep_questions());
        }

        Self {
            description,
            deep,
            questions,
            current_index: 0,
            answers: Vec::new(),
            current_input: String::new(),
            custom_mode: false,
            app_event_tx,
            done: false,
            existing_spec_id,
        }
    }

    fn current_question(&self) -> Option<&SpecIntakeQuestion> {
        self.questions.get(self.current_index)
    }

    fn select_option(&mut self, label: char) {
        let Some(question) = self.current_question() else {
            return;
        };
        let Some(option) = question
            .options
            .iter()
            .find(|opt| opt.label.eq_ignore_ascii_case(&label))
        else {
            return;
        };

        if option.is_custom {
            self.custom_mode = true;
            self.current_input.clear();
        } else {
            self.answers.push(option.text.to_string());
            self.advance_or_submit();
        }
    }

    fn submit_custom_answer(&mut self) {
        if !self.custom_mode {
            return;
        }
        let answer = self.current_input.trim().to_string();
        self.answers.push(answer);
        self.custom_mode = false;
        self.current_input.clear();
        self.advance_or_submit();
    }

    fn advance_or_submit(&mut self) {
        self.current_index += 1;
        if self.current_index >= self.questions.len() {
            self.submit();
        }
    }

    fn submit(&mut self) {
        use std::collections::HashMap;

        let mut answers_by_key: HashMap<String, String> = HashMap::new();
        for (idx, question) in self.questions.iter().enumerate() {
            let answer = self.answers.get(idx).cloned().unwrap_or_default();
            answers_by_key.insert(question.key.to_string(), answer);
        }

        self.done = true;
        self.app_event_tx.send(AppEvent::SpecIntakeSubmitted {
            description: self.description.clone(),
            deep: self.deep,
            answers: answers_by_key,
            existing_spec_id: self.existing_spec_id.clone(),
        });
    }

    fn handle_escape(&mut self) {
        if self.custom_mode {
            self.custom_mode = false;
            self.current_input.clear();
        } else {
            self.done = true;
            self.app_event_tx.send(AppEvent::SpecIntakeCancelled {
                description: self.description.clone(),
                existing_spec_id: self.existing_spec_id.clone(),
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

impl BottomPaneView<'_> for SpecIntakeModal {
    fn handle_key_event(&mut self, _pane: &mut BottomPane, key_event: KeyEvent) {
        if self.done {
            return;
        }

        if matches!(key_event.kind, KeyEventKind::Press | KeyEventKind::Repeat) {
            if self.custom_mode {
                match key_event.code {
                    KeyCode::Char(c) => self.handle_text_input(c),
                    KeyCode::Backspace => self.handle_backspace(),
                    KeyCode::Enter => self.submit_custom_answer(),
                    KeyCode::Esc => self.handle_escape(),
                    _ => {}
                }
            } else {
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
        if self.custom_mode { 18 } else { 16 }
    }

    fn render(&self, area: Rect, buf: &mut Buffer) {
        Clear.render(area, buf);

        let title = if self.deep {
            "Spec Intake (Deep)"
        } else {
            "Spec Intake"
        };

        let block = Block::default()
            .borders(Borders::ALL)
            .border_type(BorderType::Double)
            .border_style(Style::default().fg(Color::Magenta))
            .title(format!(
                "{} — {}/{}",
                title,
                self.current_index + 1,
                self.questions.len()
            ));

        let inner = block.inner(area);
        block.render(area, buf);

        let mut lines: Vec<Line> = Vec::new();
        lines.push(Line::from(vec![
            Span::styled("Description: ", Style::default().fg(Color::Gray)),
            Span::raw(self.description.clone()),
        ]));
        lines.push(Line::from(""));

        if let Some(question) = self.current_question() {
            lines.push(Line::from(vec![Span::styled(
                question.title,
                Style::default().fg(Color::Yellow),
            )]));
            lines.push(Line::from(vec![
                Span::styled("Hint: ", Style::default().fg(Color::Gray)),
                Span::raw(question.hint),
            ]));
            lines.push(Line::from(""));

            if self.custom_mode {
                lines.push(Line::from(vec![
                    Span::styled("Your answer: ", Style::default().fg(Color::Cyan)),
                    Span::raw(self.current_input.clone()),
                ]));
                lines.push(Line::from(""));
                lines.push(Line::from(vec![
                    Span::styled(
                        "Enter",
                        Style::default()
                            .fg(Color::Green)
                            .add_modifier(Modifier::BOLD),
                    ),
                    Span::raw(" submit | "),
                    Span::styled(
                        "Esc",
                        Style::default().fg(Color::Red).add_modifier(Modifier::BOLD),
                    ),
                    Span::raw(" back"),
                ]));
            } else {
                for opt in &question.options {
                    lines.push(Line::from(vec![
                        Span::styled(
                            format!("[{}] ", opt.label),
                            Style::default().fg(Color::Cyan),
                        ),
                        Span::raw(opt.text),
                    ]));
                }
                lines.push(Line::from(""));
                lines.push(Line::from(vec![
                    Span::styled("A-D", Style::default().fg(Color::Green)),
                    Span::raw(" select | "),
                    Span::styled("Esc", Style::default().fg(Color::Red)),
                    Span::raw(" cancel"),
                ]));
            }
        }

        Paragraph::new(lines)
            .wrap(Wrap { trim: false })
            .render(inner, buf);
    }
}

fn baseline_questions() -> Vec<SpecIntakeQuestion> {
    vec![
        SpecIntakeQuestion {
            key: "problem",
            title: "Problem (required)",
            hint: "What problem does this spec solve? Be explicit.",
            options: vec![SpecIntakeOption {
                label: 'D',
                text: "Custom...",
                is_custom: true,
            }],
        },
        SpecIntakeQuestion {
            key: "target_users",
            title: "Target Users (required)",
            hint: "Semicolon-separated list, e.g. \"Developers; Admins\"",
            options: vec![SpecIntakeOption {
                label: 'D',
                text: "Custom...",
                is_custom: true,
            }],
        },
        SpecIntakeQuestion {
            key: "outcome",
            title: "Outcome (required)",
            hint: "What changes for the user/system when this is done?",
            options: vec![SpecIntakeOption {
                label: 'D',
                text: "Custom...",
                is_custom: true,
            }],
        },
        SpecIntakeQuestion {
            key: "scope_in",
            title: "Scope In (required)",
            hint: "3–7 bullets, semicolon-separated.",
            options: vec![SpecIntakeOption {
                label: 'D',
                text: "Custom...",
                is_custom: true,
            }],
        },
        SpecIntakeQuestion {
            key: "non_goals",
            title: "Non-Goals (required)",
            hint: "3–7 bullets, semicolon-separated.",
            options: vec![SpecIntakeOption {
                label: 'D',
                text: "Custom...",
                is_custom: true,
            }],
        },
        SpecIntakeQuestion {
            key: "acceptance_criteria",
            title: "Acceptance Criteria (required)",
            hint: "Format: \"<criterion> (verify: <how>)\"; separate items with semicolons.",
            options: vec![SpecIntakeOption {
                label: 'D',
                text: "Custom...",
                is_custom: true,
            }],
        },
        SpecIntakeQuestion {
            key: "constraints",
            title: "Constraints (required)",
            hint: "Semicolon-separated (compatibility, policy, time/budget, etc).",
            options: vec![SpecIntakeOption {
                label: 'D',
                text: "Custom...",
                is_custom: true,
            }],
        },
        SpecIntakeQuestion {
            key: "integration_points",
            title: "Integration Points (required, >= 1)",
            hint: "Semicolon-separated. Must not be \"unknown\". \"hypothesized: <...>\" is OK.",
            options: vec![SpecIntakeOption {
                label: 'D',
                text: "Custom...",
                is_custom: true,
            }],
        },
        SpecIntakeQuestion {
            key: "risks",
            title: "Risks (required, >= 1)",
            hint: "Semicolon-separated list of concerns.",
            options: vec![SpecIntakeOption {
                label: 'D',
                text: "Custom...",
                is_custom: true,
            }],
        },
        SpecIntakeQuestion {
            key: "open_questions",
            title: "Open Questions (required, >= 1)",
            hint: "Semicolon-separated. What must be clarified/decided?",
            options: vec![SpecIntakeOption {
                label: 'D',
                text: "Custom...",
                is_custom: true,
            }],
        },
        SpecIntakeQuestion {
            key: "assumptions",
            title: "Assumptions (optional)",
            hint: "Semicolon-separated; can be blank.",
            options: vec![SpecIntakeOption {
                label: 'D',
                text: "Custom...",
                is_custom: true,
            }],
        },
    ]
}

fn deep_questions() -> Vec<SpecIntakeQuestion> {
    vec![
        SpecIntakeQuestion {
            key: "architecture_components",
            title: "Architecture Components (deep, required)",
            hint: "Semicolon-separated components/modules/services.",
            options: vec![SpecIntakeOption {
                label: 'D',
                text: "Custom...",
                is_custom: true,
            }],
        },
        SpecIntakeQuestion {
            key: "architecture_dataflows",
            title: "Architecture Dataflow (deep, required)",
            hint: "Semicolon-separated edges like \"A->B\" or \"Client->API\".",
            options: vec![SpecIntakeOption {
                label: 'D',
                text: "Custom...",
                is_custom: true,
            }],
        },
        SpecIntakeQuestion {
            key: "integration_mapping",
            title: "Integration Mapping (deep, required)",
            hint: "For each integration point: \"<point> => <modules/files/APIs>\"; semicolon-separated.",
            options: vec![SpecIntakeOption {
                label: 'D',
                text: "Custom...",
                is_custom: true,
            }],
        },
        SpecIntakeQuestion {
            key: "test_plan",
            title: "Test Plan (deep, required)",
            hint: "Semicolon-separated; include unit/integration/e2e + key cases.",
            options: vec![SpecIntakeOption {
                label: 'D',
                text: "Custom...",
                is_custom: true,
            }],
        },
        SpecIntakeQuestion {
            key: "threat_model",
            title: "Threat Model (deep, required)",
            hint: "Semicolon-separated; include threats + controls.",
            options: vec![SpecIntakeOption {
                label: 'D',
                text: "Custom...",
                is_custom: true,
            }],
        },
        SpecIntakeQuestion {
            key: "rollout_plan",
            title: "Rollout Plan (deep, required)",
            hint: "Semicolon-separated; flags, rollout phases, rollback.",
            options: vec![SpecIntakeOption {
                label: 'D',
                text: "Custom...",
                is_custom: true,
            }],
        },
        SpecIntakeQuestion {
            key: "risk_register",
            title: "Risk Register (deep, required)",
            hint: "Ranked risks with mitigations: \"<risk> => <mitigation>\"; semicolon-separated.",
            options: vec![SpecIntakeOption {
                label: 'D',
                text: "Custom...",
                is_custom: true,
            }],
        },
        SpecIntakeQuestion {
            key: "non_goals_rationale",
            title: "Non-Goals Rationale (deep, required)",
            hint: "Explain exclusions: \"<non-goal> => <why excluded>\"; semicolon-separated.",
            options: vec![SpecIntakeOption {
                label: 'D',
                text: "Custom...",
                is_custom: true,
            }],
        },
    ]
}
