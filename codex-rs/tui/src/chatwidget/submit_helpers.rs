//! Submit helper methods for ChatWidget.
//!
//! This module contains convenience methods for submitting user messages
//! with various transformations (display vs prompt separation, ACE injection,
//! hidden prefaces).
//!
//! Extracted from mod.rs as part of MAINT-11 to reduce cognitive load
//! and improve code organization.

use codex_core::protocol::InputItem;

use super::ChatWidget;
use super::message::UserMessage;
use super::spec_kit;
use super::spec_kit::state::{
    ValidateBeginOutcome, ValidateLifecycle, ValidateLifecycleEvent, ValidateMode, ValidateRunInfo,
};
use crate::history_cell::{HistoryCellType, PlainHistoryCell};
use crate::spec_prompts::SpecStage;

/// Parse /speckit.validate command for lifecycle tracking.
///
/// Returns `Some((spec_id, args))` if the input is a valid validate command.
fn parse_validate_command(input: &str) -> Option<(String, String)> {
    let trimmed = input.trim();
    if !trimmed.starts_with('/') {
        return None;
    }

    let mut parts = trimmed[1..].split_whitespace();
    let command = parts.next()?.to_ascii_lowercase();
    // SPEC-KIT-902: Only recognize speckit.validate (legacy aliases removed)
    if command != "speckit.validate" {
        return None;
    }

    let spec_id = parts.next()?.to_string();
    let remainder = parts.collect::<Vec<_>>().join(" ");
    Some((spec_id, remainder))
}

impl ChatWidget<'_> {
    /// Programmatically submit a user text message as if typed in the
    /// composer. The text will be added to conversation history and sent to
    /// the agent. This also handles slash command expansion.
    pub(crate) fn submit_text_message(&mut self, text: String) {
        if text.is_empty() {
            return;
        }
        self.submit_user_message(text.into());
    }

    /// Submit a message where the user sees `display` in history, but the
    /// model receives only `prompt`. This is used for prompt-expanding
    /// slash commands selected via the popup where expansion happens before
    /// reaching the normal composer pipeline.
    pub(crate) fn submit_prompt_with_display(&mut self, display: String, prompt: String) {
        if display.is_empty() && prompt.is_empty() {
            return;
        }

        let mut manual_validate_context: Option<(
            String,
            ValidateLifecycle,
            ValidateRunInfo,
            String,
        )> = None;

        if let Some((spec_id, _args)) = parse_validate_command(display.trim()) {
            let lifecycle = self.ensure_validate_lifecycle(&spec_id);
            let payload_hash = spec_kit::compute_validate_payload_hash(
                ValidateMode::Manual,
                SpecStage::Validate,
                &spec_id,
                prompt.as_str(),
            );

            match lifecycle.begin(ValidateMode::Manual, &payload_hash) {
                ValidateBeginOutcome::Started(info) => {
                    spec_kit::record_validate_lifecycle_event(
                        self,
                        &spec_id,
                        &info.run_id,
                        info.attempt,
                        info.dedupe_count,
                        &payload_hash,
                        info.mode,
                        ValidateLifecycleEvent::Queued,
                    );
                    manual_validate_context =
                        Some((spec_id.clone(), lifecycle.clone(), info, payload_hash));
                }
                ValidateBeginOutcome::Duplicate(info) | ValidateBeginOutcome::Conflict(info) => {
                    spec_kit::record_validate_lifecycle_event(
                        self,
                        &spec_id,
                        &info.run_id,
                        info.attempt,
                        info.dedupe_count,
                        &payload_hash,
                        info.mode,
                        ValidateLifecycleEvent::Deduped,
                    );

                    let mut lines: Vec<ratatui::text::Line<'static>> = Vec::new();
                    lines.push(ratatui::text::Line::from(format!(
                        "⚠ Validate run already active (run_id: {}, attempt: {})",
                        info.run_id, info.attempt
                    )));
                    lines.push(ratatui::text::Line::from(
                        "Current run must finish or be cancelled before triggering another.",
                    ));
                    self.history_push(PlainHistoryCell::new(lines, HistoryCellType::Notice));
                    return;
                }
            }
        }

        let mut ordered = Vec::new();
        if !prompt.trim().is_empty() {
            ordered.push(InputItem::Text {
                text: prompt.clone(),
            });
        }
        let msg = UserMessage {
            display_text: display,
            ordered_items: ordered,
        };
        self.submit_user_message(msg);

        if let Some((spec_id, lifecycle, info, payload_hash)) = manual_validate_context
            && let Some(updated) = lifecycle.mark_dispatched(&info.run_id)
        {
            spec_kit::record_validate_lifecycle_event(
                self,
                &spec_id,
                &updated.run_id,
                updated.attempt,
                updated.dedupe_count,
                &payload_hash,
                updated.mode,
                ValidateLifecycleEvent::Dispatched,
            );
        }
    }

    /// Submit prompt with ACE bullet injection (async).
    ///
    /// Fetches bullets from ACE playbook asynchronously and injects before submission.
    /// Shows "preparing" message while fetching bullets.
    pub(crate) fn submit_prompt_with_ace(
        &mut self,
        display: String,
        prompt: String,
        command_name: &str,
    ) {
        // If ACE disabled or not applicable, submit immediately
        if !spec_kit::ace_prompt_injector::should_use_ace(&self.config.ace, command_name) {
            self.submit_prompt_with_display(display, prompt);
            return;
        }

        // Show preparing message
        self.history_push(PlainHistoryCell::new(
            vec![ratatui::text::Line::from(
                "⏳ Preparing prompt with ACE context...",
            )],
            HistoryCellType::Notice,
        ));

        // Clone data for async task
        let config = self.config.ace.clone();
        let repo_root =
            spec_kit::routing::get_repo_root(&self.config.cwd).unwrap_or_else(|| ".".to_string());
        let branch = spec_kit::routing::get_current_branch(&self.config.cwd)
            .unwrap_or_else(|| "main".to_string());
        let cmd_name = command_name.to_string();
        let tx = self.app_event_tx.clone();

        // Spawn async injection task
        tokio::spawn(async move {
            let scope = spec_kit::ace_prompt_injector::command_to_scope(&cmd_name);

            let enhanced_prompt = if let Some(scope) = scope {
                match spec_kit::ace_client::playbook_slice(
                    repo_root,
                    branch,
                    scope.to_string(),
                    config.slice_size,
                    false,
                )
                .await
                {
                    spec_kit::ace_client::AceResult::Ok(response) => {
                        // Format and inject bullets
                        let selected = spec_kit::ace_prompt_injector::select_bullets(
                            response.bullets,
                            config.slice_size,
                        );
                        let (ace_section, _ids) =
                            spec_kit::ace_prompt_injector::format_ace_section(&selected);

                        if !ace_section.is_empty() {
                            // Inject before <task>
                            if let Some(pos) = prompt.find("<task>") {
                                let mut enhanced = prompt.clone();
                                enhanced.insert_str(pos, &ace_section);
                                enhanced
                            } else {
                                format!("{}\n\n{}", ace_section, prompt)
                            }
                        } else {
                            prompt
                        }
                    }
                    _ => prompt,
                }
            } else {
                prompt
            };

            // Submit via event
            tx.send(crate::app_event::AppEvent::SubmitPreparedPrompt {
                display,
                prompt: enhanced_prompt,
            });
        });
    }

    /// Submit a visible text message, but prepend a hidden instruction that is
    /// sent to the agent in the same turn. The hidden text is not added to the
    /// chat history; only `visible` appears to the user.
    pub(crate) fn submit_text_message_with_preface(&mut self, visible: String, preface: String) {
        if visible.is_empty() {
            return;
        }
        let mut ordered = Vec::new();
        if !preface.trim().is_empty() {
            ordered.push(InputItem::Text { text: preface });
        }
        ordered.push(InputItem::Text {
            text: visible.clone(),
        });
        let msg = UserMessage {
            display_text: visible,
            ordered_items: ordered,
        };
        self.submit_user_message(msg);
    }

    /// Queue a note that will be delivered to the agent as a hidden system
    /// message immediately before the next user input is sent. Notes are
    /// drained in FIFO order so multiple updates retain their sequencing.
    pub(crate) fn queue_agent_note<S: Into<String>>(&mut self, note: S) {
        let note = note.into();
        if note.trim().is_empty() {
            return;
        }
        self.pending_agent_notes.push(note);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_validate_command_valid() {
        let result = parse_validate_command("/speckit.validate SPEC-001 --force");
        assert!(result.is_some());
        let (spec_id, args) = result.unwrap();
        assert_eq!(spec_id, "SPEC-001");
        assert_eq!(args, "--force");
    }

    #[test]
    fn test_parse_validate_command_no_args() {
        let result = parse_validate_command("/speckit.validate SPEC-002");
        assert!(result.is_some());
        let (spec_id, args) = result.unwrap();
        assert_eq!(spec_id, "SPEC-002");
        assert_eq!(args, "");
    }

    #[test]
    fn test_parse_validate_command_invalid() {
        // Not a slash command
        assert!(parse_validate_command("speckit.validate SPEC-001").is_none());
        // Wrong command
        assert!(parse_validate_command("/speckit.new SPEC-001").is_none());
        // Missing spec_id
        assert!(parse_validate_command("/speckit.validate").is_none());
    }

    #[test]
    fn test_parse_validate_command_case_insensitive() {
        let result = parse_validate_command("/SPECKIT.VALIDATE SPEC-003");
        assert!(result.is_some());
        let (spec_id, _) = result.unwrap();
        assert_eq!(spec_id, "SPEC-003");
    }
}
