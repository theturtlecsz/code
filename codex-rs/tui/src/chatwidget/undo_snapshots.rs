// MAINT-11 Phase 9: Undo/Snapshots functions extracted from mod.rs
// Contains ghost snapshot management, /undo command handling, and restore functionality

use chrono::Local;
use codex_common::elapsed::format_duration;
use codex_git_tooling::{
    CreateGhostCommitOptions, GitToolingError, create_ghost_commit, restore_ghost_commit,
};

use crate::app_event::AppEvent;
use crate::app_event_sender::AppEventSender;
use crate::bottom_pane::list_selection_view::{ListSelectionView, SelectionAction, SelectionItem};
use crate::bottom_pane::UndoRestoreView;
use crate::history_cell;

use super::{
    ChatWidget, ConversationSnapshot, GhostSnapshot, GhostSnapshotsDisabledReason, GhostState,
    UndoSnapshotPreview, MAX_TRACKED_GHOST_COMMITS,
};

impl ChatWidget<'_> {
    /// Capture a ghost commit snapshot after agent commits changes.
    pub(crate) fn capture_ghost_snapshot(&mut self, summary: Option<String>) {
        if self.ghost_snapshots_disabled {
            return;
        }

        let conversation = self.current_conversation_snapshot();
        let options = CreateGhostCommitOptions::new(&self.config.cwd);
        match create_ghost_commit(&options) {
            Ok(commit) => {
                self.ghost_snapshots_disabled = false;
                self.ghost_snapshots_disabled_reason = None;
                self.ghost_snapshots
                    .push(GhostSnapshot::new(commit, summary, conversation));
                if self.ghost_snapshots.len() > MAX_TRACKED_GHOST_COMMITS {
                    self.ghost_snapshots.remove(0);
                }
            }
            Err(err) => {
                self.ghost_snapshots_disabled = true;
                let (message, hint) = match &err {
                    GitToolingError::NotAGitRepository { .. } => (
                        "Snapshots disabled: this workspace is not inside a Git repository."
                            .to_string(),
                        None,
                    ),
                    _ => (
                        format!("Snapshots disabled after Git error: {err}"),
                        Some(
                            "Restart Code after resolving the issue to re-enable snapshots."
                                .to_string(),
                        ),
                    ),
                };
                self.ghost_snapshots_disabled_reason = Some(GhostSnapshotsDisabledReason {
                    message: message.clone(),
                    hint: hint.clone(),
                });
                self.push_background_tail(message);
                if let Some(hint) = hint {
                    self.push_background_tail(hint);
                }
                tracing::warn!("failed to create ghost snapshot: {err}");
            }
        }
    }

    /// Get the current conversation snapshot (user/assistant turn counts).
    fn current_conversation_snapshot(&self) -> ConversationSnapshot {
        use crate::history_cell::HistoryCellType;
        let mut user_turns = 0usize;
        let mut assistant_turns = 0usize;
        for cell in &self.history_cells {
            match cell.kind() {
                HistoryCellType::User => user_turns = user_turns.saturating_add(1),
                HistoryCellType::Assistant => assistant_turns = assistant_turns.saturating_add(1),
                _ => {}
            }
        }
        let mut snapshot = ConversationSnapshot::new(user_turns, assistant_turns);
        snapshot.history_len = self.history_cells.len();
        snapshot.order_len = self.cell_order_seq.len();
        snapshot.order_dbg_len = self.cell_order_dbg.len();
        snapshot
    }

    /// Calculate the delta in turns since a given snapshot.
    fn conversation_delta_since(&self, snapshot: &ConversationSnapshot) -> (usize, usize) {
        let current = self.current_conversation_snapshot();
        let user_delta = current.user_turns.saturating_sub(snapshot.user_turns);
        let assistant_delta = current
            .assistant_turns
            .saturating_sub(snapshot.assistant_turns);
        (user_delta, assistant_delta)
    }

    /// Snapshot the current ghost state for session transfer.
    pub(crate) fn snapshot_ghost_state(&self) -> GhostState {
        GhostState {
            snapshots: self.ghost_snapshots.clone(),
            disabled: self.ghost_snapshots_disabled,
            disabled_reason: self.ghost_snapshots_disabled_reason.clone(),
        }
    }

    /// Adopt ghost state from a previous session.
    pub(crate) fn adopt_ghost_state(&mut self, state: GhostState) {
        self.ghost_snapshots = state.snapshots;
        if self.ghost_snapshots.len() > MAX_TRACKED_GHOST_COMMITS {
            self.ghost_snapshots.truncate(MAX_TRACKED_GHOST_COMMITS);
        }
        self.ghost_snapshots_disabled = state.disabled;
        self.ghost_snapshots_disabled_reason = state.disabled_reason;
    }

    /// Create a preview of a snapshot at a given index.
    fn snapshot_preview(&self, index: usize) -> Option<UndoSnapshotPreview> {
        self.ghost_snapshots.get(index).map(|snapshot| {
            let (user_delta, assistant_delta) =
                self.conversation_delta_since(&snapshot.conversation);
            UndoSnapshotPreview {
                index,
                short_id: snapshot.short_id(),
                summary: snapshot.summary.clone(),
                captured_at: snapshot.captured_at,
                age: snapshot.age_from(Local::now()),
                user_delta,
                assistant_delta,
            }
        })
    }

    /// Handle the /undo command.
    pub(crate) fn handle_undo_command(&mut self) {
        if self.ghost_snapshots_disabled {
            let reason = self
                .ghost_snapshots_disabled_reason
                .as_ref()
                .map(|reason| reason.message.clone())
                .unwrap_or_else(|| "Snapshots are currently disabled.".to_string());
            self.push_background_tail(format!("/undo unavailable: {reason}"));
            self.show_undo_snapshots_disabled();
            return;
        }

        if self.ghost_snapshots.is_empty() {
            self.push_background_tail(
                "/undo unavailable: no snapshots captured yet. Run a file-modifying command to create one.".to_string(),
            );
            self.show_undo_empty_state();
            return;
        }

        self.show_undo_snapshot_picker();
    }

    /// Show UI when snapshots are disabled.
    fn show_undo_snapshots_disabled(&mut self) {
        let mut lines: Vec<String> = Vec::new();
        if let Some(reason) = &self.ghost_snapshots_disabled_reason {
            lines.push(reason.message.clone());
            if let Some(hint) = &reason.hint {
                lines.push(hint.clone());
            }
        } else {
            lines.push(
                "Snapshots are currently disabled. Resolve the Git issue and restart Code to re-enable them.".to_string(),
            );
        }

        self.show_undo_status_popup(
            "Snapshots unavailable",
            Some(
                "Restores workspace files only. Conversation history remains unchanged."
                    .to_string(),
            ),
            Some(
                "Automatic snapshotting failed, so /undo cannot restore the workspace.".to_string(),
            ),
            lines,
        );
    }

    /// Show UI when no snapshots exist yet.
    fn show_undo_empty_state(&mut self) {
        self.show_undo_status_popup(
            "No snapshots yet",
            Some(
                "Restores workspace files only. Conversation history remains unchanged."
                    .to_string(),
            ),
            Some("Snapshots appear once Code captures a Git checkpoint.".to_string()),
            vec![
                "No snapshot is available to restore.".to_string(),
                "Run a command that modifies files to create the first snapshot.".to_string(),
            ],
        );
    }

    /// Show a status popup with undo information.
    fn show_undo_status_popup(
        &mut self,
        title: &str,
        scope_hint: Option<String>,
        subtitle: Option<String>,
        mut lines: Vec<String>,
    ) {
        if lines.is_empty() {
            lines.push("No snapshot information available.".to_string());
        }

        let headline = lines.remove(0);
        let description = if lines.is_empty() {
            None
        } else {
            Some(lines.join("\n"))
        };

        let mut composed_subtitle = Vec::new();
        if let Some(hint) = scope_hint {
            composed_subtitle.push(hint);
        }
        if let Some(extra) = subtitle {
            composed_subtitle.push(extra);
        }
        let subtitle_for_view = if composed_subtitle.is_empty() {
            None
        } else {
            Some(composed_subtitle.join("\n"))
        };

        let items = vec![SelectionItem {
            name: headline,
            description,
            is_current: true,
            actions: Vec::new(),
        }];

        let view = ListSelectionView::new(
            format!(" {title} "),
            subtitle_for_view,
            Some("Esc close".to_string()),
            items,
            self.app_event_tx.clone(),
            1,
        );

        self.bottom_pane.show_list_selection(
            title.to_string(),
            None,
            Some("Esc close".to_string()),
            view,
        );
    }

    /// Show the snapshot picker UI.
    fn show_undo_snapshot_picker(&mut self) {
        let now = Local::now();
        let mut entries: Vec<(usize, &GhostSnapshot)> =
            self.ghost_snapshots.iter().enumerate().collect();
        entries.reverse();

        let mut items: Vec<SelectionItem> = Vec::new();
        for (display_idx, (actual_idx, snapshot)) in entries.into_iter().enumerate() {
            let idx = actual_idx;
            let short_id = snapshot.short_id();
            let name = snapshot
                .summary_snippet(80)
                .unwrap_or_else(|| format!("Snapshot {short_id}"));

            let mut details: Vec<String> = Vec::new();
            if let Some(age) = snapshot.age_from(now) {
                details.push(format!("captured {} ago", format_duration(age)));
            } else {
                details.push("captured moments ago".to_string());
            }
            details.push(snapshot.captured_at.format("%Y-%m-%d %H:%M:%S").to_string());
            details.push(format!("commit {short_id}"));
            let description = Some(details.join(" • "));

            let actions: Vec<SelectionAction> = vec![Box::new(move |tx: &AppEventSender| {
                tx.send(AppEvent::ShowUndoOptions { index: idx });
            })];

            items.push(SelectionItem {
                name,
                description,
                is_current: display_idx == 0,
                actions,
            });
        }

        if items.is_empty() {
            self.push_background_tail(
                "/undo unavailable: no snapshots captured yet. Run a file-modifying command to create one.".to_string(),
            );
            self.show_undo_empty_state();
            return;
        }

        let mut subtitle_lines: Vec<String> = Vec::new();
        subtitle_lines
            .push("Restores workspace files only; chat history stays unchanged.".to_string());
        subtitle_lines.push("Select a snapshot to jump back in time.".to_string());
        let view = ListSelectionView::new(
            " Restore a workspace snapshot ".to_string(),
            Some(subtitle_lines.join("\n")),
            Some("Enter restore • Esc cancel".to_string()),
            items,
            self.app_event_tx.clone(),
            8,
        );

        self.bottom_pane.show_list_selection(
            "Restore snapshot".to_string(),
            Some("Restores workspace files only; chat history stays unchanged.".to_string()),
            Some("Enter restore • Esc cancel".to_string()),
            view,
        );
    }

    /// Show restore options for a selected snapshot.
    pub(crate) fn show_undo_restore_options(&mut self, index: usize) {
        let Some(preview) = self.snapshot_preview(index) else {
            self.push_background_tail("Selected snapshot is no longer available.".to_string());
            return;
        };

        let timestamp = preview.captured_at.format("%Y-%m-%d %H:%M:%S").to_string();
        let timestamp_line = preview
            .age
            .map(|age| format!("Captured {} ({})", timestamp, format_duration(age)))
            .unwrap_or_else(|| format!("Captured {}", timestamp));
        let title_line = "Select what to restore".to_string();
        let conversation_available = preview.user_delta > 0;

        let view = UndoRestoreView::new(
            preview.index,
            preview.short_id.clone(),
            title_line,
            preview.summary.clone(),
            timestamp_line,
            preview.user_delta,
            preview.assistant_delta,
            false,
            conversation_available,
            self.app_event_tx.clone(),
        );
        self.bottom_pane.show_undo_restore_view(view);
    }

    /// Perform the actual undo restore operation.
    pub(crate) fn perform_undo_restore(
        &mut self,
        index: usize,
        restore_files: bool,
        restore_conversation: bool,
    ) {
        if index >= self.ghost_snapshots.len() {
            self.push_background_tail("Selected snapshot is no longer available.".to_string());
            return;
        }

        if !restore_files && !restore_conversation {
            self.push_background_tail("No restore options selected.".to_string());
            return;
        }

        let snapshot = self.ghost_snapshots[index].clone();
        let mut files_restored = false;
        let mut conversation_rewind_requested = false;
        let mut errors: Vec<String> = Vec::new();
        let mut pre_restore_snapshot: Option<GhostSnapshot> = None;

        if restore_files {
            let previous_len = self.ghost_snapshots.len();
            let pre_summary = Some("Pre-undo checkpoint".to_string());
            self.capture_ghost_snapshot(pre_summary);
            if self.ghost_snapshots.len() > previous_len {
                pre_restore_snapshot = self.ghost_snapshots.last().cloned();
            }

            match restore_ghost_commit(&self.config.cwd, snapshot.commit()) {
                Ok(()) => {
                    files_restored = true;
                    self.ghost_snapshots.truncate(index);
                    if let Some(pre) = pre_restore_snapshot {
                        self.ghost_snapshots.push(pre);
                        if self.ghost_snapshots.len() > MAX_TRACKED_GHOST_COMMITS {
                            self.ghost_snapshots.remove(0);
                        }
                    }
                }
                Err(err) => {
                    if self.ghost_snapshots.len() > previous_len {
                        self.ghost_snapshots.pop();
                    }
                    errors.push(format!("Failed to restore workspace files: {err}"));
                }
            }
        }

        if restore_conversation {
            let (user_delta, assistant_delta) =
                self.conversation_delta_since(&snapshot.conversation);
            if user_delta == 0 {
                self.push_background_tail(
                    "Conversation already matches selected snapshot; nothing to rewind."
                        .to_string(),
                );
            } else {
                self.app_event_tx.send(AppEvent::JumpBack {
                    nth: user_delta,
                    prefill: String::new(),
                });
                if assistant_delta > 0 {
                    self.push_background_tail(format!(
                        "Rewinding conversation by {} user turn{} and {} assistant repl{}",
                        user_delta,
                        if user_delta == 1 { "" } else { "s" },
                        assistant_delta,
                        if assistant_delta == 1 { "y" } else { "ies" }
                    ));
                } else {
                    self.push_background_tail(format!(
                        "Rewinding conversation by {} user turn{}",
                        user_delta,
                        if user_delta == 1 { "" } else { "s" }
                    ));
                }
                conversation_rewind_requested = true;
            }
        }

        for err in errors {
            self.history_push(history_cell::new_error_event(err));
        }

        if files_restored {
            let mut message = format!(
                "Restored workspace files to snapshot {}",
                snapshot.short_id()
            );
            if let Some(snippet) = snapshot.summary_snippet(60) {
                message.push_str(&format!(" • {}", snippet));
            }
            if let Some(age) = snapshot.age_from(Local::now()) {
                message.push_str(&format!(" • captured {} ago", format_duration(age)));
            }
            if !restore_conversation {
                message.push_str(" • chat history unchanged");
            }
            self.push_background_tail(message);
        }

        if conversation_rewind_requested {
            // Conversation rewind will reload the chat widget via AppEvent::JumpBack.
            self.reset_after_conversation_restore();
        }

        self.request_redraw();
    }

    /// Reset widget state after a conversation restore.
    fn reset_after_conversation_restore(&mut self) {
        self.pending_dispatched_user_messages.clear();
        self.pending_user_prompts_for_next_turn = 0;
        self.queued_user_messages.clear();
        self.refresh_queued_user_messages();
        self.bottom_pane.clear_composer();
        self.bottom_pane.clear_ctrl_c_quit_hint();
        self.bottom_pane.clear_live_ring();
        self.bottom_pane.set_task_running(false);
        self.active_task_ids.clear();
        self.pending_jump_back = None;
        if !self.agents_terminal.active {
            self.bottom_pane.ensure_input_focus();
        }
    }

    /// Undo a pending jump back operation.
    pub(crate) fn undo_jump_back(&mut self) {
        if let Some(mut st) = self.pending_jump_back.take() {
            // Restore removed cells in original order
            self.history_cells.append(&mut st.removed_cells);
            // Clear composer (no reliable way to restore prior text)
            self.insert_str("");
            self.request_redraw();
        }
    }

    /// Check if there's a pending jump back operation.
    pub(crate) fn has_pending_jump_back(&self) -> bool {
        self.pending_jump_back.is_some()
    }
}
