//! Device Code Login View
//!
//! FORK-SPECIFIC (just-every/code): P6-SYNC Phase 7
//!
//! Interactive TUI component for OAuth 2.0 Device Code Authorization flow.
//! Displays user code and verification URL, polls in background for completion.
//!
//! Flow:
//! 1. Request device code from provider
//! 2. Display user_code and verification_uri to user
//! 3. Optionally open browser automatically
//! 4. Poll token endpoint in background
//! 5. On success: store token, update footer, show confirmation
//! 6. On error/expiry: show retry option

use std::cell::RefCell;
use std::rc::Rc;
use std::time::Instant;

use codex_login::DeviceCodeProvider;
use crossterm::event::{KeyCode, KeyEvent};
use ratatui::buffer::Buffer;
use ratatui::layout::{Alignment, Rect};
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Clear, Paragraph, Widget};

use crate::app_event::AppEvent;
use crate::app_event_sender::AppEventSender;

use super::BottomPane;
use super::bottom_pane_view::{BottomPaneView, ConditionalUpdate};

/// Interactive view for device code OAuth login flow.
pub(crate) struct DeviceCodeLoginView {
    state: Rc<RefCell<DeviceCodeLoginState>>,
}

impl DeviceCodeLoginView {
    pub fn new(
        provider: DeviceCodeProvider,
        app_event_tx: AppEventSender,
    ) -> (Self, Rc<RefCell<DeviceCodeLoginState>>) {
        let state = Rc::new(RefCell::new(DeviceCodeLoginState::new(
            provider,
            app_event_tx,
        )));
        (
            Self {
                state: state.clone(),
            },
            state,
        )
    }
}

impl<'a> BottomPaneView<'a> for DeviceCodeLoginView {
    fn handle_key_event(&mut self, pane: &mut BottomPane<'a>, key_event: KeyEvent) {
        let mut state = self.state.borrow_mut();
        state.handle_key_event(key_event);
        pane.request_redraw();
    }

    fn is_complete(&self) -> bool {
        self.state.borrow().is_complete
    }

    fn desired_height(&self, _width: u16) -> u16 {
        let state = self.state.borrow();
        state.desired_height() as u16
    }

    fn render(&self, area: Rect, buf: &mut Buffer) {
        let state = self.state.borrow();
        state.render(area, buf);
    }

    fn handle_paste(&mut self, _text: String) -> ConditionalUpdate {
        ConditionalUpdate::NoRedraw
    }
}

/// Current state of the device code login flow
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum LoginFlowState {
    /// Initial state - requesting device code
    Starting,
    /// Displaying code and polling for completion
    WaitingForUser {
        user_code: String,
        verification_uri: String,
        verification_uri_complete: Option<String>,
        poll_count: u32,
        started_at: Instant,
    },
    /// Token received successfully
    Success,
    /// Flow failed with error
    Error(String),
    /// Device code expired
    Expired,
}

/// Feedback message for UI display
#[derive(Clone, Debug)]
struct Feedback {
    message: String,
    is_error: bool,
}

/// State for the device code login view
pub(crate) struct DeviceCodeLoginState {
    provider: DeviceCodeProvider,
    app_event_tx: AppEventSender,
    flow_state: LoginFlowState,
    feedback: Option<Feedback>,
    is_complete: bool,
    browser_opened: bool,
}

impl DeviceCodeLoginState {
    fn new(provider: DeviceCodeProvider, app_event_tx: AppEventSender) -> Self {
        Self {
            provider,
            app_event_tx,
            flow_state: LoginFlowState::Starting,
            feedback: Some(Feedback {
                message: format!("Starting {} device authorization...", provider_display_name(provider)),
                is_error: false,
            }),
            is_complete: false,
            browser_opened: false,
        }
    }

    /// Called when device authorization response is received
    pub fn on_device_auth_response(
        &mut self,
        user_code: String,
        verification_uri: String,
        verification_uri_complete: Option<String>,
    ) {
        self.flow_state = LoginFlowState::WaitingForUser {
            user_code: user_code.clone(),
            verification_uri: verification_uri.clone(),
            verification_uri_complete: verification_uri_complete.clone(),
            poll_count: 0,
            started_at: Instant::now(),
        };
        self.feedback = Some(Feedback {
            message: format!(
                "Enter code {} at the URL below to authenticate.",
                user_code
            ),
            is_error: false,
        });
    }

    /// Called on each poll attempt to update UI
    pub fn on_poll_attempt(&mut self, poll_count: u32) {
        if let LoginFlowState::WaitingForUser {
            poll_count: ref mut count,
            ..
        } = self.flow_state
        {
            *count = poll_count;
        }
    }

    /// Called when token is successfully obtained
    pub fn on_success(&mut self) {
        self.flow_state = LoginFlowState::Success;
        self.feedback = Some(Feedback {
            message: format!(
                "{} authenticated successfully!",
                provider_display_name(self.provider)
            ),
            is_error: false,
        });
        // Auto-close after success
        self.is_complete = true;
        self.app_event_tx.send(AppEvent::ShowLoginAccounts);
    }

    /// Called when the flow fails
    pub fn on_error(&mut self, error: String) {
        self.flow_state = LoginFlowState::Error(error.clone());
        self.feedback = Some(Feedback {
            message: error,
            is_error: true,
        });
    }

    /// Called when the device code expires
    pub fn on_expired(&mut self) {
        self.flow_state = LoginFlowState::Expired;
        self.feedback = Some(Feedback {
            message: "Device code expired. Press Enter to retry.".to_string(),
            is_error: true,
        });
    }

    /// Called when user denies access
    pub fn on_access_denied(&mut self) {
        self.flow_state = LoginFlowState::Error("Access denied by user".to_string());
        self.feedback = Some(Feedback {
            message: "Authorization was denied. Press Enter to retry or Esc to cancel.".to_string(),
            is_error: true,
        });
    }
    fn handle_key_event(&mut self, key_event: KeyEvent) {
        match key_event.code {
            KeyCode::Esc | KeyCode::Char('q') => {
                // Cancel the flow
                self.is_complete = true;
                self.send_cancel_event();
            }
            KeyCode::Enter => {
                match &self.flow_state {
                    LoginFlowState::Error(_) | LoginFlowState::Expired => {
                        // Retry the flow
                        self.flow_state = LoginFlowState::Starting;
                        self.feedback = Some(Feedback {
                            message: format!(
                                "Retrying {} device authorization...",
                                provider_display_name(self.provider)
                            ),
                            is_error: false,
                        });
                        self.send_retry_event();
                    }
                    LoginFlowState::WaitingForUser {
                        verification_uri,
                        verification_uri_complete,
                        ..
                    } => {
                        // Try to open browser
                        let url = verification_uri_complete
                            .as_ref()
                            .unwrap_or(verification_uri);
                        if !self.browser_opened {
                            let _ = open::that(url);
                            self.browser_opened = true;
                            self.feedback = Some(Feedback {
                                message: "Browser opened. Complete sign-in to continue.".to_string(),
                                is_error: false,
                            });
                        }
                    }
                    _ => {}
                }
            }
            KeyCode::Char('o') | KeyCode::Char('O') => {
                // Open browser manually
                if let LoginFlowState::WaitingForUser {
                    verification_uri,
                    verification_uri_complete,
                    ..
                } = &self.flow_state
                {
                    let url = verification_uri_complete
                        .as_ref()
                        .unwrap_or(verification_uri);
                    let _ = open::that(url);
                    self.browser_opened = true;
                    self.feedback = Some(Feedback {
                        message: "Browser opened. Complete sign-in to continue.".to_string(),
                        is_error: false,
                    });
                }
            }
            _ => {}
        }
    }

    fn send_cancel_event(&self) {
        match self.provider {
            DeviceCodeProvider::OpenAI => {
                self.app_event_tx.send(AppEvent::LoginCancelChatGpt);
            }
            DeviceCodeProvider::Anthropic => {
                self.app_event_tx.send(AppEvent::LoginCancelClaude);
            }
            DeviceCodeProvider::Google => {
                self.app_event_tx.send(AppEvent::LoginCancelGemini);
            }
        }
    }

    fn send_retry_event(&self) {
        match self.provider {
            DeviceCodeProvider::OpenAI => {
                self.app_event_tx.send(AppEvent::DeviceCodeLoginStart {
                    provider: DeviceCodeProvider::OpenAI,
                });
            }
            DeviceCodeProvider::Anthropic => {
                self.app_event_tx.send(AppEvent::DeviceCodeLoginStart {
                    provider: DeviceCodeProvider::Anthropic,
                });
            }
            DeviceCodeProvider::Google => {
                self.app_event_tx.send(AppEvent::DeviceCodeLoginStart {
                    provider: DeviceCodeProvider::Google,
                });
            }
        }
    }

    fn desired_height(&self) -> usize {
        let mut lines = 5; // title + spacing baseline

        if self.feedback.is_some() {
            lines += 2;
        }

        match &self.flow_state {
            LoginFlowState::Starting => {
                lines += 2; // spinner message
            }
            LoginFlowState::WaitingForUser { .. } => {
                lines += 8; // code display, URL, instructions, hints
            }
            LoginFlowState::Success => {
                lines += 3;
            }
            LoginFlowState::Error(_) | LoginFlowState::Expired => {
                lines += 4; // error + retry hint
            }
        }

        lines.max(12) + 2
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
            .title(format!(" {} Device Login ", provider_display_name(self.provider)))
            .title_alignment(Alignment::Center);
        let inner = block.inner(area);
        block.render(area, buf);

        let mut lines = Vec::new();

        // Feedback message
        if let Some(feedback) = &self.feedback {
            let style = if feedback.is_error {
                Style::default()
                    .fg(crate::colors::error())
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default()
                    .fg(crate::colors::success())
                    .add_modifier(Modifier::BOLD)
            };
            lines.push(Line::from(vec![Span::styled(
                feedback.message.clone(),
                style,
            )]));
            lines.push(Line::from(""));
        }

        match &self.flow_state {
            LoginFlowState::Starting => {
                lines.push(Line::from(vec![Span::styled(
                    "Requesting device code...",
                    Style::default().fg(crate::colors::text_dim()),
                )]));
            }
            LoginFlowState::WaitingForUser {
                user_code,
                verification_uri,
                verification_uri_complete,
                poll_count,
                started_at,
            } => {
                // Big code display
                lines.push(Line::from(vec![Span::styled(
                    "Your verification code:",
                    Style::default().add_modifier(Modifier::BOLD),
                )]));
                lines.push(Line::from(""));
                lines.push(Line::from(vec![Span::styled(
                    format!("    {}    ", user_code),
                    Style::default()
                        .fg(crate::colors::primary())
                        .add_modifier(Modifier::BOLD),
                )]));
                lines.push(Line::from(""));

                // URL
                let display_uri = verification_uri_complete
                    .as_ref()
                    .unwrap_or(verification_uri);
                lines.push(Line::from(vec![
                    Span::styled("Go to: ", Style::default()),
                    Span::styled(
                        display_uri.clone(),
                        Style::default()
                            .fg(crate::colors::primary())
                            .add_modifier(Modifier::UNDERLINED),
                    ),
                ]));
                lines.push(Line::from(""));

                // Status
                let elapsed = started_at.elapsed();
                let dots = ".".repeat((*poll_count as usize % 4) + 1);
                lines.push(Line::from(vec![Span::styled(
                    format!(
                        "Waiting for authorization{} ({}s)",
                        dots,
                        elapsed.as_secs()
                    ),
                    Style::default().fg(crate::colors::text_dim()),
                )]));
                lines.push(Line::from(""));

                // Key hints
                lines.push(Line::from(vec![
                    Span::styled("o", Style::default().fg(crate::colors::function())),
                    Span::styled(
                        " Open browser  ",
                        Style::default().fg(crate::colors::text_dim()),
                    ),
                    Span::styled(
                        "Esc",
                        Style::default()
                            .fg(crate::colors::error())
                            .add_modifier(Modifier::BOLD),
                    ),
                    Span::styled(" Cancel", Style::default().fg(crate::colors::text_dim())),
                ]));
            }
            LoginFlowState::Success => {
                lines.push(Line::from(vec![Span::styled(
                    "✓ Authentication successful!",
                    Style::default()
                        .fg(crate::colors::success())
                        .add_modifier(Modifier::BOLD),
                )]));
            }
            LoginFlowState::Error(error) => {
                lines.push(Line::from(vec![Span::styled(
                    format!("Error: {}", error),
                    Style::default().fg(crate::colors::error()),
                )]));
                lines.push(Line::from(""));
                lines.push(Line::from(vec![
                    Span::styled("Enter", Style::default().fg(crate::colors::success())),
                    Span::styled(" Retry  ", Style::default().fg(crate::colors::text_dim())),
                    Span::styled(
                        "Esc",
                        Style::default()
                            .fg(crate::colors::error())
                            .add_modifier(Modifier::BOLD),
                    ),
                    Span::styled(" Cancel", Style::default().fg(crate::colors::text_dim())),
                ]));
            }
            LoginFlowState::Expired => {
                lines.push(Line::from(vec![Span::styled(
                    "Device code expired",
                    Style::default().fg(crate::colors::error()),
                )]));
                lines.push(Line::from(""));
                lines.push(Line::from(vec![
                    Span::styled("Enter", Style::default().fg(crate::colors::success())),
                    Span::styled(
                        " Restart flow  ",
                        Style::default().fg(crate::colors::text_dim()),
                    ),
                    Span::styled(
                        "Esc",
                        Style::default()
                            .fg(crate::colors::error())
                            .add_modifier(Modifier::BOLD),
                    ),
                    Span::styled(" Cancel", Style::default().fg(crate::colors::text_dim())),
                ]));
            }
        }

        Paragraph::new(lines)
            .alignment(Alignment::Left)
            .style(
                Style::default()
                    .bg(crate::colors::background())
                    .fg(crate::colors::text()),
            )
            .render(
                Rect {
                    x: inner.x.saturating_add(1),
                    y: inner.y,
                    width: inner.width.saturating_sub(2),
                    height: inner.height,
                },
                buf,
            );
    }
}

/// Get display name for a provider
fn provider_display_name(provider: DeviceCodeProvider) -> &'static str {
    provider.display_name()
}

#[cfg(test)]
mod tests {
    use super::*;
    use tokio::sync::mpsc::unbounded_channel;

    #[test]
    fn test_initial_state() {
        let (tx, _rx) = unbounded_channel();
        let tx = AppEventSender::new(tx);
        let state = DeviceCodeLoginState::new(DeviceCodeProvider::OpenAI, tx);

        assert!(!state.is_complete);
        assert!(matches!(state.flow_state, LoginFlowState::Starting));
    }

    #[test]
    fn test_on_device_auth_response() {
        let (tx, _rx) = unbounded_channel();
        let tx = AppEventSender::new(tx);
        let mut state = DeviceCodeLoginState::new(DeviceCodeProvider::OpenAI, tx);

        state.on_device_auth_response(
            "ABCD-1234".to_string(),
            "https://example.com/device".to_string(),
            None,
        );

        assert!(matches!(
            state.flow_state,
            LoginFlowState::WaitingForUser { ref user_code, .. } if user_code == "ABCD-1234"
        ));
    }

    #[test]
    fn test_on_success() {
        let (tx, _rx) = unbounded_channel();
        let tx = AppEventSender::new(tx);
        let mut state = DeviceCodeLoginState::new(DeviceCodeProvider::OpenAI, tx);

        state.on_success();

        assert!(matches!(state.flow_state, LoginFlowState::Success));
        assert!(state.is_complete);
    }

    #[test]
    fn test_on_error() {
        let (tx, _rx) = unbounded_channel();
        let tx = AppEventSender::new(tx);
        let mut state = DeviceCodeLoginState::new(DeviceCodeProvider::OpenAI, tx);

        state.on_error("Network error".to_string());

        assert!(matches!(
            state.flow_state,
            LoginFlowState::Error(ref msg) if msg == "Network error"
        ));
    }

    #[test]
    fn test_on_expired() {
        let (tx, _rx) = unbounded_channel();
        let tx = AppEventSender::new(tx);
        let mut state = DeviceCodeLoginState::new(DeviceCodeProvider::OpenAI, tx);

        state.on_expired();

        assert!(matches!(state.flow_state, LoginFlowState::Expired));
    }

    #[test]
    fn test_provider_display_names() {
        assert_eq!(provider_display_name(DeviceCodeProvider::OpenAI), "OpenAI");
        assert_eq!(provider_display_name(DeviceCodeProvider::Anthropic), "Claude");
        assert_eq!(provider_display_name(DeviceCodeProvider::Google), "Gemini");
    }
}
