// MAINT-11 Phase 8: Session handler functions extracted from mod.rs
// Contains session management, resume UI, and export functionality

use std::fs;
use std::io::Write;

use chrono::{DateTime, Utc};
use codex_core::config::Config;
use codex_protocol::models::{ContentItem, ResponseItem};
use ratatui::text::Line;
use serde_json::Value as JsonValue;

use crate::app_event::AppEvent;
use crate::bottom_pane::resume_selection_view::ResumeRow;
use crate::history_cell::{self, HistoryCell, HistoryCellType, PlainHistoryCell};
use crate::providers::claude_streaming::ClaudeStreamingProvider;
use crate::providers::gemini_streaming::GeminiStreamingProvider;
use super::streaming;
use crate::streaming::StreamKind;

use super::ChatWidget;

/// Format a timestamp into human-readable relative time (e.g., "2h ago", "3d ago").
pub(crate) fn human_ago(ts: &str) -> String {
    if let Ok(dt) = DateTime::parse_from_rfc3339(ts) {
        let now = Utc::now();
        let delta = now.signed_duration_since(dt.with_timezone(&Utc));
        let secs = delta.num_seconds().max(0);
        let mins = secs / 60;
        let hours = mins / 60;
        let days = hours / 24;
        if days >= 7 {
            // Show date for older entries
            return dt.format("%Y-%m-%d").to_string();
        }
        if days >= 1 {
            return format!("{}d ago", days);
        }
        if hours >= 1 {
            return format!("{}h ago", hours);
        }
        if mins >= 1 {
            return format!("{}m ago", mins);
        }
        return "just now".to_string();
    }
    ts.to_string()
}

/// List all active CLI sessions (implementation).
/// Returns a formatted string with session information.
pub(crate) async fn list_cli_sessions_impl() -> String {
    let mut all_sessions = Vec::new();

    // Get Claude sessions from global provider
    all_sessions.extend(
        ClaudeStreamingProvider::global_provider()
            .list_sessions()
            .await,
    );

    // Get Gemini sessions from global provider
    all_sessions.extend(
        GeminiStreamingProvider::global_provider()
            .list_sessions()
            .await,
    );

    // Format output
    if all_sessions.is_empty() {
        "# Active CLI Sessions\n\nNo active sessions.".to_string()
    } else {
        let mut lines = vec!["# Active CLI Sessions\n".to_string()];
        lines.push(format!("{} active session(s)\n", all_sessions.len()));
        lines.push("```".to_string());
        lines.push(format!(
            "{:<20} {:<12} {:<40} {:<8} {:<8} {}",
            "CONV-ID", "PROVIDER", "SESSION-ID", "TURNS", "PID", "AGE"
        ));
        lines.push("-".repeat(100));

        for session in all_sessions {
            let age = session
                .created_at
                .elapsed()
                .map(|d| format!("{}s", d.as_secs()))
                .unwrap_or_else(|_| "?".to_string());

            let session_id_short = session
                .session_id
                .as_ref()
                .map(|s| {
                    if s.len() > 36 {
                        format!("{}...", &s[..36])
                    } else {
                        s.clone()
                    }
                })
                .unwrap_or_else(|| "none".to_string());

            let pid_str = session
                .current_pid
                .map(|p| p.to_string())
                .unwrap_or_else(|| "-".to_string());

            let conv_id_short = if session.conv_id.len() > 18 {
                format!("{}...", &session.conv_id[..18])
            } else {
                session.conv_id.clone()
            };

            lines.push(format!(
                "{:<20} {:<12} {:<40} {:<8} {:<8} {}",
                conv_id_short,
                session.provider,
                session_id_short,
                session.turn_count,
                pid_str,
                age
            ));
        }

        lines.push("```".to_string());
        lines.push("\nCommands:".to_string());
        lines.push("- `/sessions kill <conv-id>` - Kill specific session".to_string());
        lines.push("- `/sessions kill-all` - Kill all sessions".to_string());

        lines.join("\n")
    }
}

/// Kill a specific CLI session (implementation).
/// Returns a formatted result message.
pub(crate) async fn kill_cli_session_impl(conv_id: &str) -> String {
    let mut killed = false;

    // Try Claude provider
    if ClaudeStreamingProvider::global_provider()
        .kill_session(&conv_id.to_string())
        .await
        .is_ok()
    {
        killed = true;
    }

    // Try Gemini provider
    if GeminiStreamingProvider::global_provider()
        .kill_session(&conv_id.to_string())
        .await
        .is_ok()
    {
        killed = true;
    }

    if killed {
        format!("# Session Kill\n\n✅ Killed session: {}", conv_id)
    } else {
        format!("# Session Kill\n\n⚠️  Session not found: {}", conv_id)
    }
}

/// Kill all CLI sessions (implementation).
/// Returns a formatted result message.
pub(crate) async fn kill_all_cli_sessions_impl() -> String {
    // Get counts before killing
    let claude_count = ClaudeStreamingProvider::global_provider()
        .active_session_count()
        .await;
    let gemini_count = GeminiStreamingProvider::global_provider()
        .active_session_count()
        .await;
    let total = claude_count + gemini_count;

    // Kill all sessions
    let _ = ClaudeStreamingProvider::global_provider()
        .shutdown_all()
        .await;
    let _ = GeminiStreamingProvider::global_provider()
        .shutdown_all()
        .await;

    format!(
        "# Kill All Sessions\n\n✅ Killed {} session(s):\n- Claude: {}\n- Gemini: {}",
        total, claude_count, gemini_count
    )
}

impl ChatWidget<'_> {
    /// Handle /sessions command with subcommands: list, kill, kill-all.
    pub(crate) fn handle_sessions_command(&mut self, args: String) {
        let args = args.trim().to_string();

        // Clone what we need for the async task
        let app_tx = self.app_event_tx.clone();

        // Parse subcommand and spawn appropriate task
        if args.starts_with("kill ") {
            let conv_id = args.strip_prefix("kill ").unwrap().trim().to_string();
            tokio::spawn(async move {
                let output = kill_cli_session_impl(&conv_id).await;
                app_tx.send(AppEvent::SessionsCommandResult(output));
            });
        } else if args == "kill-all" || args == "cleanup" {
            tokio::spawn(async move {
                let output = kill_all_cli_sessions_impl().await;
                app_tx.send(AppEvent::SessionsCommandResult(output));
            });
        } else {
            // Default: list sessions
            tokio::spawn(async move {
                let output = list_cli_sessions_impl().await;
                app_tx.send(AppEvent::SessionsCommandResult(output));
            });
        }
    }

    /// Show the resume session picker UI.
    pub(crate) fn show_resume_picker(&mut self) {
        // Discover candidates
        let cwd = self.config.cwd.clone();
        let codex_home = self.config.codex_home.clone();
        let candidates = crate::resume::discovery::list_sessions_for_cwd(&cwd, &codex_home);
        if candidates.is_empty() {
            self.push_background_tail("No past sessions found for this folder".to_string());
            return;
        }

        // Convert to simple rows with aligned columns and human-friendly times
        let rows: Vec<ResumeRow> = candidates
            .into_iter()
            .map(|c| {
                let modified = human_ago(&c.modified_ts.unwrap_or_default());
                let created = human_ago(&c.created_ts.unwrap_or_default());
                let msgs = format!("{}", c.message_count);
                let branch = c.branch.unwrap_or_else(|| "-".to_string());
                let mut summary = c.snippet.unwrap_or_else(|| c.subtitle.unwrap_or_default());
                const SNIPPET_MAX: usize = 64;
                if summary.chars().count() > SNIPPET_MAX {
                    summary = summary.chars().take(SNIPPET_MAX).collect::<String>() + "…";
                }
                ResumeRow {
                    modified,
                    created,
                    msgs,
                    branch,
                    summary,
                    path: c.path,
                }
            })
            .collect();
        let title = format!("Resume Session — {}", cwd.display());
        let subtitle = Some(String::new());
        self.bottom_pane
            .show_resume_selection(title, subtitle, rows);
    }

    /// Render a single recorded ResponseItem into history without executing tools.
    /// Used when replaying saved sessions.
    pub(crate) fn render_replay_item(&mut self, item: ResponseItem) {
        match item {
            ResponseItem::Message { role, content, .. } => {
                let mut text = String::new();
                for c in content {
                    match c {
                        ContentItem::OutputText { text: t }
                        | ContentItem::InputText { text: t } => {
                            if !text.is_empty() {
                                text.push('\n');
                            }
                            text.push_str(&t);
                        }
                        _ => {}
                    }
                }
                let text = text.trim();
                if text.is_empty() {
                    return;
                }
                if role == "user"
                    && let Some(expected) = self.pending_dispatched_user_messages.front()
                    && expected.trim() == text
                {
                    self.pending_dispatched_user_messages.pop_front();
                    return;
                }
                if text.starts_with("== System Status ==") {
                    return;
                }
                if role == "assistant" {
                    let mut lines: Vec<Line<'static>> = Vec::new();
                    crate::markdown::append_markdown(text, &mut lines, &self.config);
                    self.insert_final_answer_with_id(None, lines, text.to_string());
                    return;
                }
                if role == "user" {
                    let key = self.next_internal_key();
                    let _ = self.history_insert_with_key_global(
                        Box::new(history_cell::new_user_prompt(text.to_string())),
                        key,
                    );

                    if let Some(front) = self.queued_user_messages.front()
                        && front.display_text.trim() == text.trim()
                    {
                        self.queued_user_messages.pop_front();
                        self.refresh_queued_user_messages();
                    }
                } else {
                    let mut lines = Vec::new();
                    crate::markdown::append_markdown(text, &mut lines, &self.config);
                    let key = self.next_internal_key();
                    let _ = self.history_insert_with_key_global(
                        Box::new(PlainHistoryCell::new(lines, HistoryCellType::Assistant)),
                        key,
                    );
                }
            }
            ResponseItem::FunctionCall {
                name,
                arguments,
                call_id,
                ..
            } => {
                let pretty_args = serde_json::from_str::<JsonValue>(&arguments)
                    .and_then(|v| serde_json::to_string_pretty(&v))
                    .unwrap_or_else(|_| arguments.clone());
                let mut message = format!("🔧 Tool call: {}", name);
                if !pretty_args.trim().is_empty() {
                    message.push('\n');
                    message.push_str(&pretty_args);
                }
                if !call_id.is_empty() {
                    message.push_str(&format!("\ncall_id: {}", call_id));
                }
                let key = self.next_internal_key();
                let _ = self.history_insert_with_key_global_tagged(
                    Box::new(history_cell::new_background_event(message)),
                    key,
                    "background",
                );
            }
            ResponseItem::Reasoning { summary, .. } => {
                for s in summary {
                    let codex_protocol::models::ReasoningItemReasoningSummary::SummaryText { text } =
                        s;
                    // Reasoning cell – use the existing reasoning output styling
                    let sink = crate::streaming::controller::AppEventHistorySink(
                        self.app_event_tx.clone(),
                    );
                    streaming::begin(self, StreamKind::Reasoning, None);
                    let _ = self.stream.apply_final_reasoning(&text, &sink);
                    // finalize immediately for static replay
                    self.stream
                        .finalize(StreamKind::Reasoning, true, &sink);
                }
            }
            ResponseItem::FunctionCallOutput {
                output, call_id, ..
            } => {
                let mut content = output.content.clone();
                let mut metadata_summary = String::new();
                if let Ok(v) = serde_json::from_str::<JsonValue>(&content) {
                    if let Some(s) = v.get("output").and_then(|x| x.as_str()) {
                        content = s.to_string();
                    }
                    if let Some(meta) = v.get("metadata").and_then(|m| m.as_object()) {
                        let mut parts = Vec::new();
                        if let Some(code) = meta.get("exit_code").and_then(|x| x.as_i64()) {
                            parts.push(format!("exit_code={}", code));
                        }
                        if let Some(duration) =
                            meta.get("duration_seconds").and_then(|x| x.as_f64())
                        {
                            parts.push(format!("duration={:.2}s", duration));
                        }
                        if !parts.is_empty() {
                            metadata_summary = parts.join(", ");
                        }
                    }
                }
                let mut message = String::new();
                if !content.trim().is_empty() {
                    message.push_str(content.trim_end());
                }
                if !metadata_summary.is_empty() {
                    if !message.is_empty() {
                        message.push_str("\n\n");
                    }
                    message.push_str(&format!("({})", metadata_summary));
                }
                if !call_id.is_empty() {
                    if !message.is_empty() {
                        message.push('\n');
                    }
                    message.push_str(&format!("call_id: {}", call_id));
                }
                if message.trim().is_empty() {
                    return;
                }
                let key = self.next_internal_key();
                let _ = self.history_insert_with_key_global_tagged(
                    Box::new(history_cell::new_background_event(message)),
                    key,
                    "background",
                );
            }
            _ => {
                // Ignore other item kinds for replay (tool calls, etc.)
            }
        }
    }

    /// Export history cells as ResponseItems for serialization.
    pub(crate) fn export_response_items(&self) -> Vec<ResponseItem> {
        let mut items = Vec::new();
        for cell in &self.history_cells {
            match cell.kind() {
                HistoryCellType::User => {
                    let text = cell
                        .display_lines()
                        .iter()
                        .map(|l| {
                            l.spans
                                .iter()
                                .map(|s| s.content.to_string())
                                .collect::<String>()
                        })
                        .collect::<Vec<_>>()
                        .join("\n");
                    items.push(ResponseItem::Message {
                        id: None,
                        role: "user".to_string(),
                        content: vec![ContentItem::OutputText { text }],
                    });
                }
                HistoryCellType::Assistant => {
                    let text = cell
                        .display_lines()
                        .iter()
                        .map(|l| {
                            l.spans
                                .iter()
                                .map(|s| s.content.to_string())
                                .collect::<String>()
                        })
                        .collect::<Vec<_>>()
                        .join("\n");
                    items.push(ResponseItem::Message {
                        id: None,
                        role: "assistant".to_string(),
                        content: vec![ContentItem::OutputText { text }],
                    });
                }
                _ => {}
            }
        }
        items
    }

    /// Handle /feedback command to export session logs.
    /// P53-SYNC: Creates a feedback file with session log snapshot.
    pub(crate) fn handle_feedback_command(&mut self, config: &Config) {
        // Helper to display messages in chat history
        let show_msg = |widget: &mut Self, message: String| {
            let cell = history_cell::new_background_event(message);
            widget.push_system_cell(
                cell,
                super::SystemPlacement::EndOfCurrent,
                None,
                None,
                "feedback:result",
            );
        };

        // Get the feedback collector
        let Some(feedback) = crate::get_feedback_collector() else {
            show_msg(self, String::from("Feedback collection not initialized."));
            return;
        };

        // Take a snapshot of the ring buffer (None = current session)
        let snapshot = feedback.snapshot(None);
        if snapshot.is_empty() {
            show_msg(
                self,
                String::from("No logs captured yet. Try again after performing some actions."),
            );
            return;
        }

        // Create feedback directory
        let feedback_dir = config.codex_home.join("feedback");
        if let Err(e) = fs::create_dir_all(&feedback_dir) {
            show_msg(self, format!("Failed to create feedback directory: {}", e));
            return;
        }

        // Generate filename with timestamp
        let timestamp = chrono::Utc::now().format("%Y%m%d_%H%M%S");
        let filename = format!("feedback_{}.log", timestamp);
        let filepath = feedback_dir.join(&filename);

        // Write snapshot to file
        match fs::File::create(&filepath) {
            Ok(mut file) => {
                if let Err(e) = file.write_all(snapshot.as_bytes()) {
                    show_msg(self, format!("Failed to write feedback file: {}", e));
                    return;
                }

                let msg = format!(
                    "Session logs exported ({} bytes):\n{}\n\nAttach this file when reporting issues.",
                    snapshot.len(),
                    filepath.display()
                );
                show_msg(self, msg);
            }
            Err(e) => {
                show_msg(self, format!("Failed to create feedback file: {}", e));
            }
        }

        self.request_redraw();
    }

    /// Export transcript lines for the buffer/clipboard.
    pub(crate) fn export_transcript_lines_for_buffer(&self) -> Vec<Line<'static>> {
        let mut out: Vec<Line<'static>> = Vec::new();
        for cell in &self.history_cells {
            out.extend(self.render_lines_for_terminal(cell.as_ref()));
        }
        // Include streaming preview if present (treat like assistant output)
        let mut streaming_lines = self
            .live_builder
            .display_rows()
            .into_iter()
            .map(|r| Line::from(r.text))
            .collect::<Vec<_>>();
        if !streaming_lines.is_empty() {
            // Apply gutter to streaming preview (first line gets " • ", continuations get 3 spaces)
            if let Some(first) = streaming_lines.first_mut() {
                first.spans.insert(0, ratatui::text::Span::raw(" • "));
            }
            for line in streaming_lines.iter_mut().skip(1) {
                line.spans.insert(0, ratatui::text::Span::raw("   "));
            }
            out.extend(streaming_lines);
            out.push(Line::from(""));
        }
        out
    }

    /// Render a single history cell into terminal-friendly lines:
    /// - Prepend a gutter icon (symbol + space) to the first line when defined.
    /// - Add a single blank line after the cell as a separator.
    pub(crate) fn render_lines_for_terminal(
        &self,
        cell: &dyn HistoryCell,
    ) -> Vec<Line<'static>> {
        let mut lines = cell.display_lines();
        let _has_icon = cell.gutter_symbol().is_some();
        let first_prefix = if let Some(sym) = cell.gutter_symbol() {
            format!(" {} ", sym) // one space, icon, one space
        } else {
            "   ".to_string() // three spaces when no icon
        };
        if let Some(first) = lines.first_mut() {
            first
                .spans
                .insert(0, ratatui::text::Span::raw(first_prefix));
        }
        // For wrapped/subsequent lines, use a 3-space gutter to maintain alignment
        if lines.len() > 1 {
            for (_idx, line) in lines.iter_mut().enumerate().skip(1) {
                // Always 3 spaces for continuation lines
                line.spans.insert(0, ratatui::text::Span::raw("   "));
            }
        }
        lines.push(Line::from(""));
        lines
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_human_ago_just_now() {
        let now = Utc::now().to_rfc3339();
        assert_eq!(human_ago(&now), "just now");
    }

    #[test]
    fn test_human_ago_minutes() {
        let ts = (Utc::now() - chrono::Duration::minutes(5)).to_rfc3339();
        assert_eq!(human_ago(&ts), "5m ago");
    }

    #[test]
    fn test_human_ago_hours() {
        let ts = (Utc::now() - chrono::Duration::hours(3)).to_rfc3339();
        assert_eq!(human_ago(&ts), "3h ago");
    }

    #[test]
    fn test_human_ago_days() {
        let ts = (Utc::now() - chrono::Duration::days(2)).to_rfc3339();
        assert_eq!(human_ago(&ts), "2d ago");
    }

    #[test]
    fn test_human_ago_weeks() {
        let ts = (Utc::now() - chrono::Duration::days(14)).to_rfc3339();
        // After 7 days, should show date
        let result = human_ago(&ts);
        assert!(result.contains("-"), "Expected date format, got: {}", result);
    }

    #[test]
    fn test_human_ago_invalid() {
        assert_eq!(human_ago("not-a-date"), "not-a-date");
    }
}
