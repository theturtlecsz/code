//! Project intake modal for /speckit.projectnew flow
//!
//! Displays Q&A wizard to capture project brief information.
//! Answers are persisted to capsule as SoR, then projected to docs/PROJECT_BRIEF.md.

use crossterm::event::{KeyCode, KeyEvent, KeyEventKind};
use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::prelude::*;
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, BorderType, Borders, Clear, Paragraph, Wrap};

use crate::app_event::AppEvent;
use crate::app_event_sender::AppEventSender;

use super::{BottomPane, BottomPaneView, CancellationEvent};

/// Project intake question with predefined options
#[derive(Clone)]
pub(crate) struct ProjectIntakeQuestion {
    pub key: &'static str,
    pub question: &'static str,
    pub hint: &'static str,
    pub options: Vec<ProjectIntakeOption>,
    pub is_deep: bool, // Only shown in deep mode
}

/// Option for a project intake question
#[derive(Clone)]
pub(crate) struct ProjectIntakeOption {
    pub label: char,
    pub text: &'static str,
    pub is_custom: bool,
}

/// Modal state for project intake Q&A wizard
pub(crate) struct ProjectIntakeModal {
    project_id: String,
    deep: bool,
    questions: Vec<ProjectIntakeQuestion>,
    current_index: usize,
    answers: Vec<String>,
    current_input: String,
    custom_mode: bool,
    app_event_tx: AppEventSender,
    done: bool,
}

impl ProjectIntakeModal {
    /// Create modal with project intake questions
    pub fn new(project_id: String, deep: bool, app_event_tx: AppEventSender) -> Self {
        let mut questions = baseline_questions();
        if deep {
            questions.extend(deep_questions());
        }

        Self {
            project_id,
            deep,
            questions,
            current_index: 0,
            answers: Vec::new(),
            current_input: String::new(),
            custom_mode: false,
            app_event_tx,
            done: false,
        }
    }

    fn current_question(&self) -> Option<&ProjectIntakeQuestion> {
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
                answers_map.insert(q.key.to_string(), answer.clone());
            }
        }

        self.app_event_tx.send(AppEvent::ProjectIntakeSubmitted {
            project_id: self.project_id.clone(),
            deep: self.deep,
            answers: answers_map,
        });
    }

    fn handle_escape(&mut self) {
        if self.custom_mode {
            // Exit custom mode, go back to options
            self.custom_mode = false;
            self.current_input.clear();
        } else {
            // Cancel entire intake
            self.done = true;
            self.app_event_tx.send(AppEvent::ProjectIntakeCancelled {
                project_id: self.project_id.clone(),
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

impl BottomPaneView<'_> for ProjectIntakeModal {
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

        let mode_indicator = if self.deep { "Deep" } else { "Baseline" };

        // Main border
        let block = Block::default()
            .borders(Borders::ALL)
            .border_type(BorderType::Double)
            .border_style(Style::default().fg(Color::Cyan))
            .title(format!(
                " Project Intake [{}/{}] ({}) ",
                self.current_index + 1,
                self.total_questions(),
                mode_indicator
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
                "Project Brief Builder",
                Style::default().fg(Color::Cyan).bold(),
            ),
            Span::styled(" - ", Style::default().dim()),
            Span::styled(
                format!("Project: {}", self.project_id),
                Style::default().fg(Color::White).dim(),
            ),
        ]));
        lines.push(Line::from(""));

        // Category badge
        lines.push(Line::from(vec![Span::styled(
            format!(" {} ", question.key.to_uppercase()),
            Style::default().fg(Color::Black).bg(Color::Cyan).bold(),
        )]));
        lines.push(Line::from(""));

        // Question
        lines.push(Line::from(vec![Span::styled(
            question.question.to_string(),
            Style::default().fg(Color::Cyan).bold(),
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
                let style = if option.is_custom {
                    Style::default().fg(Color::Yellow)
                } else {
                    Style::default().fg(Color::White)
                };
                lines.push(Line::from(vec![
                    Span::styled(
                        format!("[{}] ", option.label),
                        Style::default().fg(Color::Cyan).bold(),
                    ),
                    Span::styled(option.text, style),
                ]));
            }
            lines.push(Line::from(""));
            lines.push(Line::from(vec![
                Span::styled("[A-D]", Style::default().fg(Color::Green)),
                Span::raw(" Select  "),
                Span::styled("[Esc]", Style::default().fg(Color::Yellow)),
                Span::raw(" Cancel"),
            ]));
        }

        // Render paragraph
        let para = Paragraph::new(lines)
            .wrap(Wrap { trim: false })
            .style(Style::default());
        para.render(inner, buf);
    }
}

// =============================================================================
// Question Definitions
// =============================================================================

fn baseline_questions() -> Vec<ProjectIntakeQuestion> {
    vec![
        ProjectIntakeQuestion {
            key: "users",
            question: "Who is this project for?",
            hint: "Describe your target users/audience",
            options: vec![
                ProjectIntakeOption {
                    label: 'A',
                    text: "Developers building applications",
                    is_custom: false,
                },
                ProjectIntakeOption {
                    label: 'B',
                    text: "End-users of the application",
                    is_custom: false,
                },
                ProjectIntakeOption {
                    label: 'C',
                    text: "Internal team members",
                    is_custom: false,
                },
                ProjectIntakeOption {
                    label: 'D',
                    text: "Custom...",
                    is_custom: true,
                },
            ],
            is_deep: false,
        },
        ProjectIntakeQuestion {
            key: "problem",
            question: "What problem does this project solve?",
            hint: "The core pain point or challenge",
            options: vec![
                ProjectIntakeOption {
                    label: 'A',
                    text: "Improves developer productivity",
                    is_custom: false,
                },
                ProjectIntakeOption {
                    label: 'B',
                    text: "Automates manual workflows",
                    is_custom: false,
                },
                ProjectIntakeOption {
                    label: 'C',
                    text: "Reduces operational complexity",
                    is_custom: false,
                },
                ProjectIntakeOption {
                    label: 'D',
                    text: "Custom...",
                    is_custom: true,
                },
            ],
            is_deep: false,
        },
        ProjectIntakeQuestion {
            key: "goals",
            question: "What are the primary goals? (3-5 success criteria)",
            hint: "Enter each goal separated by semicolons",
            options: vec![
                ProjectIntakeOption {
                    label: 'A',
                    text: "High performance; Scalability; Reliability",
                    is_custom: false,
                },
                ProjectIntakeOption {
                    label: 'B',
                    text: "Easy to use; Well documented; Maintainable",
                    is_custom: false,
                },
                ProjectIntakeOption {
                    label: 'C',
                    text: "Security; Compliance; Auditability",
                    is_custom: false,
                },
                ProjectIntakeOption {
                    label: 'D',
                    text: "Custom...",
                    is_custom: true,
                },
            ],
            is_deep: false,
        },
        ProjectIntakeQuestion {
            key: "non_goals",
            question: "What do we explicitly NOT build?",
            hint: "Enter each non-goal separated by semicolons",
            options: vec![
                ProjectIntakeOption {
                    label: 'A',
                    text: "No UI components; No mobile support",
                    is_custom: false,
                },
                ProjectIntakeOption {
                    label: 'B',
                    text: "No backwards compatibility; No legacy support",
                    is_custom: false,
                },
                ProjectIntakeOption {
                    label: 'C',
                    text: "No third-party integrations; No custom branding",
                    is_custom: false,
                },
                ProjectIntakeOption {
                    label: 'D',
                    text: "Custom...",
                    is_custom: true,
                },
            ],
            is_deep: false,
        },
        ProjectIntakeQuestion {
            key: "principles",
            question: "What are the key design principles?",
            hint: "Architectural values (semicolon-separated)",
            options: vec![
                ProjectIntakeOption {
                    label: 'A',
                    text: "Simplicity over features; Convention over configuration",
                    is_custom: false,
                },
                ProjectIntakeOption {
                    label: 'B',
                    text: "Type safety; Explicit error handling; Immutability",
                    is_custom: false,
                },
                ProjectIntakeOption {
                    label: 'C',
                    text: "Test-driven; Documentation-first; API stability",
                    is_custom: false,
                },
                ProjectIntakeOption {
                    label: 'D',
                    text: "Custom...",
                    is_custom: true,
                },
            ],
            is_deep: false,
        },
        ProjectIntakeQuestion {
            key: "guardrails",
            question: "What are your hard constraints?",
            hint: "Security, privacy, compliance requirements (semicolon-separated)",
            options: vec![
                ProjectIntakeOption {
                    label: 'A',
                    text: "No PII in logs; GDPR compliance; Data encryption at rest",
                    is_custom: false,
                },
                ProjectIntakeOption {
                    label: 'B',
                    text: "No external API calls; Air-gapped deployment; Local-only data",
                    is_custom: false,
                },
                ProjectIntakeOption {
                    label: 'C',
                    text: "SOC2 compliance; Audit logging; Role-based access control",
                    is_custom: false,
                },
                ProjectIntakeOption {
                    label: 'D',
                    text: "Custom...",
                    is_custom: true,
                },
            ],
            is_deep: false,
        },
        ProjectIntakeQuestion {
            key: "artifact_kind",
            question: "What kind of artifact is this project?",
            hint: "Choose the primary delivery type",
            options: vec![
                ProjectIntakeOption {
                    label: 'A',
                    text: "Library (reusable code, published as package)",
                    is_custom: false,
                },
                ProjectIntakeOption {
                    label: 'B',
                    text: "Service (long-running process, API endpoints)",
                    is_custom: false,
                },
                ProjectIntakeOption {
                    label: 'C',
                    text: "Tool (CLI, executable, automation script)",
                    is_custom: false,
                },
                ProjectIntakeOption {
                    label: 'D',
                    text: "Application (user-facing, GUI or web app)",
                    is_custom: false,
                },
            ],
            is_deep: false,
        },
    ]
}

fn deep_questions() -> Vec<ProjectIntakeQuestion> {
    vec![
        ProjectIntakeQuestion {
            key: "deployment_target",
            question: "Where will this project run?",
            hint: "Deployment environment and infrastructure",
            options: vec![
                ProjectIntakeOption {
                    label: 'A',
                    text: "Cloud (AWS, GCP, Azure)",
                    is_custom: false,
                },
                ProjectIntakeOption {
                    label: 'B',
                    text: "On-premises / Self-hosted",
                    is_custom: false,
                },
                ProjectIntakeOption {
                    label: 'C',
                    text: "Edge / Embedded devices",
                    is_custom: false,
                },
                ProjectIntakeOption {
                    label: 'D',
                    text: "Custom...",
                    is_custom: true,
                },
            ],
            is_deep: true,
        },
        ProjectIntakeQuestion {
            key: "data_classification",
            question: "What kind of data will this handle?",
            hint: "Data sensitivity classification",
            options: vec![
                ProjectIntakeOption {
                    label: 'A',
                    text: "Public / Non-sensitive",
                    is_custom: false,
                },
                ProjectIntakeOption {
                    label: 'B',
                    text: "Internal / Confidential business data",
                    is_custom: false,
                },
                ProjectIntakeOption {
                    label: 'C',
                    text: "PII / PHI / Regulated data",
                    is_custom: false,
                },
                ProjectIntakeOption {
                    label: 'D',
                    text: "Custom...",
                    is_custom: true,
                },
            ],
            is_deep: true,
        },
        ProjectIntakeQuestion {
            key: "nfr_budgets",
            question: "What are the non-functional requirements?",
            hint: "Latency, throughput, availability targets",
            options: vec![
                ProjectIntakeOption {
                    label: 'A',
                    text: "p99 < 100ms; 99.9% uptime; 10k RPS",
                    is_custom: false,
                },
                ProjectIntakeOption {
                    label: 'B',
                    text: "p99 < 1s; 99% uptime; 1k RPS",
                    is_custom: false,
                },
                ProjectIntakeOption {
                    label: 'C',
                    text: "Best effort; No SLA; Batch processing",
                    is_custom: false,
                },
                ProjectIntakeOption {
                    label: 'D',
                    text: "Custom...",
                    is_custom: true,
                },
            ],
            is_deep: true,
        },
        ProjectIntakeQuestion {
            key: "ops_baseline",
            question: "What operational capabilities are needed?",
            hint: "Monitoring, alerting, on-call requirements",
            options: vec![
                ProjectIntakeOption {
                    label: 'A',
                    text: "Full observability; 24/7 on-call; Incident response",
                    is_custom: false,
                },
                ProjectIntakeOption {
                    label: 'B',
                    text: "Basic metrics; Business hours support",
                    is_custom: false,
                },
                ProjectIntakeOption {
                    label: 'C',
                    text: "Minimal; Best-effort monitoring",
                    is_custom: false,
                },
                ProjectIntakeOption {
                    label: 'D',
                    text: "Custom...",
                    is_custom: true,
                },
            ],
            is_deep: true,
        },
        ProjectIntakeQuestion {
            key: "security_posture",
            question: "What security requirements apply?",
            hint: "Authentication, authorization, compliance",
            options: vec![
                ProjectIntakeOption {
                    label: 'A',
                    text: "OAuth2/OIDC; RBAC; Audit logging; Encryption",
                    is_custom: false,
                },
                ProjectIntakeOption {
                    label: 'B',
                    text: "API keys; Basic auth; HTTPS only",
                    is_custom: false,
                },
                ProjectIntakeOption {
                    label: 'C',
                    text: "Internal only; No authentication required",
                    is_custom: false,
                },
                ProjectIntakeOption {
                    label: 'D',
                    text: "Custom...",
                    is_custom: true,
                },
            ],
            is_deep: true,
        },
        ProjectIntakeQuestion {
            key: "release_rollout",
            question: "How will releases be deployed?",
            hint: "Feature flags, canary, rollout strategy",
            options: vec![
                ProjectIntakeOption {
                    label: 'A',
                    text: "Feature flags; Canary releases; A/B testing",
                    is_custom: false,
                },
                ProjectIntakeOption {
                    label: 'B',
                    text: "Blue-green; Rolling updates",
                    is_custom: false,
                },
                ProjectIntakeOption {
                    label: 'C',
                    text: "Direct deploy; Manual promotion",
                    is_custom: false,
                },
                ProjectIntakeOption {
                    label: 'D',
                    text: "Custom...",
                    is_custom: true,
                },
            ],
            is_deep: true,
        },
        ProjectIntakeQuestion {
            key: "primary_components",
            question: "What are the major components? (semicolon-separated)",
            hint: "Main modules, services, or subsystems",
            options: vec![
                ProjectIntakeOption {
                    label: 'A',
                    text: "API; Database; Worker; Cache",
                    is_custom: false,
                },
                ProjectIntakeOption {
                    label: 'B',
                    text: "Frontend; Backend; Auth service",
                    is_custom: false,
                },
                ProjectIntakeOption {
                    label: 'C',
                    text: "Core library; CLI; SDK",
                    is_custom: false,
                },
                ProjectIntakeOption {
                    label: 'D',
                    text: "Custom...",
                    is_custom: true,
                },
            ],
            is_deep: true,
        },
    ]
}
