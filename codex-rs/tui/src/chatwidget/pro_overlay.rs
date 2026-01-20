// MAINT-11 Phase 10: Pro overlay types and handlers extracted from mod.rs
// Contains ProState, ProOverlay UI management, and Pro event handling

use std::cell::Cell;

use chrono::{DateTime, Local};
use crossterm::event::{KeyCode, KeyEvent, KeyEventKind, KeyModifiers};
use ratatui::buffer::Buffer;
use ratatui::layout::{Margin, Rect};
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line as RLine, Span};
use ratatui::widgets::{Block, Borders, Clear, Paragraph, Widget, Wrap};

use codex_core::protocol::{ProAction, ProCategory, ProEvent, ProPhase, ProStats};

use crate::colors;
use crate::util::buffer::fill_rect;

use super::ChatWidget;

// ---------------------------------------------------------------------------
// Pro-related types
// ---------------------------------------------------------------------------

#[derive(Default)]
pub(super) struct ProState {
    pub(super) enabled: bool,
    pub(super) auto_enabled: bool,
    pub(super) status: Option<ProStatusSnapshot>,
    pub(super) last_status_update: Option<DateTime<Local>>,
    pub(super) log: Vec<ProLogEntry>,
    pub(super) overlay: Option<ProOverlay>,
    pub(super) overlay_visible: bool,
}

#[derive(Clone)]
pub(super) struct ProStatusSnapshot {
    pub(super) phase: ProPhase,
    pub(super) stats: ProStats,
}

#[derive(Clone)]
pub(super) struct ProLogEntry {
    pub(super) timestamp: DateTime<Local>,
    pub(super) title: String,
    pub(super) body: Option<String>,
    pub(super) category: ProLogCategory,
}

#[derive(Clone, Copy)]
pub(super) enum ProLogCategory {
    Status,
    Recommendation,
    Agent,
    Note,
}

pub(super) struct ProOverlay {
    scroll: Cell<u16>,
    max_scroll: Cell<u16>,
    visible_rows: Cell<u16>,
}

// ---------------------------------------------------------------------------
// ProOverlay implementation
// ---------------------------------------------------------------------------

impl ProOverlay {
    pub(super) fn new() -> Self {
        Self {
            scroll: Cell::new(0),
            max_scroll: Cell::new(0),
            visible_rows: Cell::new(0),
        }
    }

    pub(super) fn scroll(&self) -> u16 {
        self.scroll.get()
    }

    pub(super) fn set_scroll(&self, value: u16) {
        let max = self.max_scroll.get();
        self.scroll.set(value.min(max));
    }

    pub(super) fn set_max_scroll(&self, max: u16) {
        self.max_scroll.set(max);
        self.set_scroll(self.scroll.get());
    }

    pub(super) fn set_visible_rows(&self, rows: u16) {
        self.visible_rows.set(rows);
    }

    pub(super) fn visible_rows(&self) -> u16 {
        self.visible_rows.get()
    }

    pub(super) fn max_scroll(&self) -> u16 {
        self.max_scroll.get()
    }
}

// ---------------------------------------------------------------------------
// ProLogEntry implementation
// ---------------------------------------------------------------------------

impl ProLogEntry {
    pub(super) fn new(
        title: impl Into<String>,
        body: Option<String>,
        category: ProLogCategory,
    ) -> Self {
        Self {
            timestamp: Local::now(),
            title: title.into(),
            body,
            category,
        }
    }
}

// ---------------------------------------------------------------------------
// ProState implementation
// ---------------------------------------------------------------------------

impl ProState {
    pub(super) fn set_enabled(&mut self, enabled: bool) {
        self.enabled = enabled;
    }

    pub(super) fn set_auto_enabled(&mut self, enabled: bool) {
        self.auto_enabled = enabled;
    }

    pub(super) fn update_status(&mut self, phase: ProPhase, stats: ProStats) {
        self.status = Some(ProStatusSnapshot { phase, stats });
        self.last_status_update = Some(Local::now());
    }

    pub(super) fn push_log(&mut self, entry: ProLogEntry) {
        const MAX_LOG_ENTRIES: usize = 200;
        self.log.push(entry);
        if self.log.len() > MAX_LOG_ENTRIES {
            let excess = self.log.len() - MAX_LOG_ENTRIES;
            self.log.drain(0..excess);
        }
    }

    pub(super) fn ensure_overlay(&mut self) -> &mut ProOverlay {
        if self.overlay.is_none() {
            self.overlay = Some(ProOverlay::new());
        }
        self.overlay.as_mut().unwrap()
    }
}

// ---------------------------------------------------------------------------
// ChatWidget Pro-related methods
// ---------------------------------------------------------------------------

impl ChatWidget<'_> {
    pub(super) fn toggle_pro_overlay(&mut self) {
        let new_state = !self.pro.overlay_visible;
        self.pro.overlay_visible = new_state;
        if new_state {
            let overlay = self.pro.ensure_overlay();
            overlay.set_scroll(0);
        }
        self.request_redraw();
    }

    pub(super) fn close_pro_overlay(&mut self) {
        if self.pro.overlay_visible {
            self.pro.overlay_visible = false;
            self.request_redraw();
        }
    }

    pub(super) fn handle_pro_overlay_key(&mut self, key_event: KeyEvent) -> bool {
        if !self.pro.overlay_visible {
            return false;
        }
        let Some(overlay) = self.pro.overlay.as_ref() else {
            return false;
        };
        if !matches!(key_event.kind, KeyEventKind::Press | KeyEventKind::Repeat) {
            return true;
        }
        match key_event.code {
            KeyCode::Esc => {
                self.close_pro_overlay();
                true
            }
            KeyCode::Char('p') if key_event.modifiers.contains(KeyModifiers::CONTROL) => {
                self.toggle_pro_overlay();
                true
            }
            KeyCode::Up => {
                let current = overlay.scroll();
                if current > 0 {
                    overlay.set_scroll(current.saturating_sub(1));
                    self.request_redraw();
                }
                true
            }
            KeyCode::Down => {
                let current = overlay.scroll();
                let max = overlay.max_scroll();
                let next = current.saturating_add(1).min(max);
                if next != current {
                    overlay.set_scroll(next);
                    self.request_redraw();
                }
                true
            }
            KeyCode::PageUp => {
                let step = overlay.visible_rows().max(1);
                let current = overlay.scroll();
                let next = current.saturating_sub(step);
                overlay.set_scroll(next);
                self.request_redraw();
                true
            }
            KeyCode::PageDown => {
                let step = overlay.visible_rows().max(1);
                let current = overlay.scroll();
                let max = overlay.max_scroll();
                let next = current.saturating_add(step).min(max);
                overlay.set_scroll(next);
                self.request_redraw();
                true
            }
            KeyCode::Home => {
                overlay.set_scroll(0);
                self.request_redraw();
                true
            }
            KeyCode::End => {
                overlay.set_scroll(overlay.max_scroll());
                self.request_redraw();
                true
            }
            _ => false,
        }
    }

    pub(super) fn handle_pro_event(&mut self, event: ProEvent) {
        match event {
            ProEvent::Toggled { enabled } => {
                self.pro.set_enabled(enabled);
                if !enabled {
                    self.layout.pro_hud_expanded = false;
                    if self.pro.overlay_visible {
                        self.pro.overlay_visible = false;
                    }
                }
                let title = if enabled {
                    "Pro mode enabled"
                } else {
                    "Pro mode disabled"
                };
                self.pro
                    .push_log(ProLogEntry::new(title, None, ProLogCategory::Status));
            }
            ProEvent::Status { phase, stats } => {
                self.pro.update_status(phase.clone(), stats.clone());
            }
            ProEvent::DeveloperNote {
                turn_id,
                note,
                artifacts,
            } => {
                let lower = note.to_ascii_lowercase();
                if lower.contains("autonomous") && lower.contains("enabled") {
                    self.pro.set_auto_enabled(true);
                } else if lower.contains("autonomous") && lower.contains("disabled") {
                    self.pro.set_auto_enabled(false);
                }
                let mut body_lines = vec![note.clone()];
                for artifact in artifacts {
                    if !artifact.summary.is_empty() {
                        body_lines.push(format!("{}: {}", artifact.kind, artifact.summary));
                    }
                }
                let body = if body_lines.is_empty() {
                    None
                } else {
                    Some(body_lines.join("\n"))
                };
                let category = if turn_id.contains("observer") {
                    ProLogCategory::Recommendation
                } else {
                    ProLogCategory::Note
                };
                self.pro
                    .push_log(ProLogEntry::new("Developer note", body, category));
            }
            ProEvent::AgentSpawned {
                category,
                budget_ms,
                ..
            } => {
                let title = format!("{} helper spawned", self.describe_pro_category(&category));
                let body = if budget_ms > 0 {
                    Some(format!("Budget: {} ms", budget_ms))
                } else {
                    None
                };
                self.pro
                    .push_log(ProLogEntry::new(title, body, ProLogCategory::Agent));
            }
            ProEvent::AgentResult {
                category,
                ok,
                note,
                artifacts,
                ..
            } => {
                let status = if ok { "completed" } else { "failed" };
                let title = format!(
                    "{} helper {}",
                    self.describe_pro_category(&category),
                    status
                );
                let mut body_lines = Vec::new();
                if let Some(note) = note
                    && !note.is_empty()
                {
                    body_lines.push(note);
                }
                for artifact in artifacts {
                    if !artifact.summary.is_empty() {
                        body_lines.push(format!("{}: {}", artifact.kind, artifact.summary));
                    }
                }
                let body = if body_lines.is_empty() {
                    None
                } else {
                    Some(body_lines.join("\n"))
                };
                self.pro
                    .push_log(ProLogEntry::new(title, body, ProLogCategory::Agent));
            }
        }
        self.request_redraw();
    }

    pub(super) fn describe_pro_category(&self, category: &ProCategory) -> &'static str {
        match category {
            ProCategory::Planning => "Planning",
            ProCategory::Research => "Research",
            ProCategory::Debugging => "Debugging",
            ProCategory::Review => "Review",
            ProCategory::Background => "Background",
        }
    }

    pub(super) fn describe_pro_phase(&self, phase: &ProPhase) -> &'static str {
        match phase {
            ProPhase::Idle => "Idle",
            ProPhase::Planning => "Planning",
            ProPhase::Research => "Research",
            ProPhase::Debug => "Debug",
            ProPhase::Review => "Review",
            ProPhase::Background => "Background",
        }
    }

    pub(super) fn render_pro_overlay(
        &self,
        frame_area: Rect,
        history_area: Rect,
        buf: &mut Buffer,
    ) {
        let Some(overlay) = self.pro.overlay.as_ref() else {
            return;
        };

        // Dim entire frame as scrim
        let scrim_style = Style::default()
            .bg(colors::overlay_scrim())
            .fg(colors::text_dim());
        fill_rect(buf, frame_area, None, scrim_style);

        // Match horizontal padding used by history content
        let padding = 1u16;
        let overlay_area = Rect {
            x: history_area.x + padding,
            y: history_area.y,
            width: history_area.width.saturating_sub(padding * 2),
            height: history_area.height,
        };

        Clear.render(overlay_area, buf);

        let block = Block::default()
            .borders(Borders::ALL)
            .title(RLine::from(vec![
                Span::styled(" Pro activity ", Style::default().fg(colors::text())),
                Span::styled("— Esc close  ", Style::default().fg(colors::text_dim())),
                Span::styled("Ctrl+P overlay  ", Style::default().fg(colors::text_dim())),
                Span::styled("↑↓ scroll", Style::default().fg(colors::text_dim())),
            ]))
            .style(Style::default().bg(colors::background()))
            .border_style(
                Style::default()
                    .fg(colors::border())
                    .bg(colors::background()),
            );
        let inner = block.inner(overlay_area);
        block.render(overlay_area, buf);

        let body = inner.inner(Margin::new(1, 1));
        if body.height == 0 {
            return;
        }

        let mut lines: Vec<RLine<'static>> = Vec::new();
        let summary_style = Style::default()
            .fg(colors::text())
            .add_modifier(Modifier::BOLD);
        lines.push(RLine::from(vec![Span::styled(
            self.pro_summary_line(),
            summary_style,
        )]));
        lines.push(RLine::from(" "));

        if self.pro.log.is_empty() {
            lines.push(RLine::from(vec![Span::styled(
                "No Pro activity captured yet",
                Style::default().fg(colors::text_dim()),
            )]));
        } else {
            for entry in self.pro.log.iter().rev() {
                for line in self.format_pro_log_entry(entry) {
                    lines.push(line);
                }
                lines.push(RLine::from(" "));
            }
        }

        while lines
            .last()
            .map(|line| line.spans.iter().all(|s| s.content.trim().is_empty()))
            .unwrap_or(false)
        {
            lines.pop();
        }

        let total_lines = lines.len();
        let visible_rows = body.height as usize;
        overlay.set_visible_rows(body.height);
        let max_scroll = total_lines.saturating_sub(visible_rows.max(1));
        overlay.set_max_scroll(max_scroll.min(u16::MAX as usize) as u16);
        let skip = overlay.scroll().min(overlay.max_scroll()) as usize;
        let end = (skip + visible_rows).min(total_lines);
        let slice = if skip < total_lines {
            lines[skip..end].to_vec()
        } else {
            Vec::new()
        };

        let paragraph = Paragraph::new(slice).wrap(Wrap { trim: false });
        paragraph.render(body, buf);
    }

    pub(super) fn pro_summary_line(&self) -> String {
        let mut parts: Vec<String> = Vec::new();
        parts.push(if self.pro.enabled { "on" } else { "off" }.to_string());
        parts.push(format!(
            "auto {}",
            if self.pro.auto_enabled { "on" } else { "off" }
        ));
        if let Some(status) = &self.pro.status {
            parts.push(self.describe_pro_phase(&status.phase).to_string());
            parts.push(format!(
                "A{}/C{}/S{}",
                status.stats.active, status.stats.completed, status.stats.spawned
            ));
        }
        if let Some(ts) = self.pro.last_status_update {
            parts.push(format!("updated {}", self.format_recent_timestamp(ts)));
        }
        parts.join(" · ")
    }

    pub(super) fn format_pro_log_entry(&self, entry: &ProLogEntry) -> Vec<RLine<'static>> {
        let mut lines: Vec<RLine<'static>> = Vec::new();
        let timestamp = entry.timestamp.format("%H:%M:%S").to_string();
        let mut header_spans: Vec<Span<'static>> = Vec::new();
        header_spans.push(Span::styled(
            timestamp,
            Style::default().fg(colors::text_dim()),
        ));
        header_spans.push(Span::raw("  "));
        header_spans.push(Span::styled(
            entry.title.clone(),
            Style::default()
                .fg(self.pro_category_color(entry.category))
                .add_modifier(Modifier::BOLD),
        ));
        lines.push(RLine::from(header_spans));

        if let Some(body) = &entry.body {
            for body_line in body.lines() {
                let trimmed = body_line.trim();
                if trimmed.is_empty() {
                    continue;
                }
                lines.push(RLine::from(Span::raw(format!("  {}", trimmed))));
            }
        }

        lines
    }

    pub(super) fn pro_category_color(&self, category: ProLogCategory) -> ratatui::style::Color {
        match category {
            ProLogCategory::Status => colors::text(),
            ProLogCategory::Recommendation => colors::primary(),
            ProLogCategory::Agent => colors::info(),
            ProLogCategory::Note => colors::text_mid(),
        }
    }

    pub(crate) fn parse_pro_action(&self, args: &str) -> Result<ProAction, String> {
        let trimmed = args.trim();
        if trimmed.is_empty() {
            return Ok(ProAction::Status);
        }
        let mut parts = trimmed.split_whitespace();
        let first = parts.next().unwrap_or("").to_ascii_lowercase();
        let ensure_no_extra = |iter: &mut dyn Iterator<Item = &str>| {
            if iter.next().is_some() {
                Err("Too many arguments for /pro [auto] command".to_string())
            } else {
                Ok(())
            }
        };
        match first.as_str() {
            "toggle" | "switch" => {
                ensure_no_extra(&mut parts)?;
                Ok(ProAction::Toggle)
            }
            "on" | "enable" | "start" => {
                ensure_no_extra(&mut parts)?;
                Ok(ProAction::On)
            }
            "off" | "disable" | "stop" => {
                ensure_no_extra(&mut parts)?;
                Ok(ProAction::Off)
            }
            "status" | "state" => {
                ensure_no_extra(&mut parts)?;
                Ok(ProAction::Status)
            }
            "auto" => {
                let next = parts.next().map(|s| s.to_ascii_lowercase());
                match next.as_deref() {
                    None => Ok(ProAction::AutoToggle),
                    Some("toggle" | "switch") => {
                        ensure_no_extra(&mut parts)?;
                        Ok(ProAction::AutoToggle)
                    }
                    Some("on" | "enable" | "start") => {
                        ensure_no_extra(&mut parts)?;
                        Ok(ProAction::AutoOn)
                    }
                    Some("off" | "disable" | "stop") => {
                        ensure_no_extra(&mut parts)?;
                        Ok(ProAction::AutoOff)
                    }
                    Some("status" | "state") => {
                        ensure_no_extra(&mut parts)?;
                        Ok(ProAction::AutoStatus)
                    }
                    Some(other) => Err(format!("Unknown /pro auto option: {}", other)),
                }
            }
            other => Err(format!("Unknown /pro subcommand: {}", other)),
        }
    }

    pub(super) fn pro_surface_present(&self) -> bool {
        if !(self.pro.enabled || self.pro.auto_enabled) {
            return false;
        }
        self.pro.status.is_some() || !self.pro.log.is_empty() || self.pro.overlay_visible
    }

    pub(super) fn format_recent_timestamp(&self, timestamp: DateTime<Local>) -> String {
        let now = Local::now();
        let delta = now.signed_duration_since(timestamp);
        if delta.num_seconds() < 0 {
            return "just now".to_string();
        }
        if delta.num_seconds() < 10 {
            return "just now".to_string();
        }
        if delta.num_seconds() < 60 {
            return format!("{}s ago", delta.num_seconds());
        }
        if delta.num_minutes() < 60 {
            return format!("{}m ago", delta.num_minutes());
        }
        if delta.num_hours() < 24 {
            return format!("{}h ago", delta.num_hours());
        }
        timestamp.format("%b %e %H:%M").to_string()
    }
}
