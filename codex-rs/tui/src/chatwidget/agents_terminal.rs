//! Agents terminal overlay state and rendering.
//!
//! This module contains the agents terminal overlay which displays
//! agent execution details in a split-pane interface with a sidebar
//! list of agents and a detail view showing logs/progress.
//!
//! Extracted from mod.rs as part of MAINT-11 Phase 9.

use std::collections::HashMap;

use chrono::Local;
use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::widgets::Widget;

use super::agent_status::{
    agent_log_color, agent_log_label, agent_status_color, agent_status_from_str,
    agent_status_label, AgentLogEntry, AgentLogKind, AgentStatus,
};
use super::ChatWidget;
use crate::util::buffer::fill_rect;

// =========================================================================
// Types
// =========================================================================

/// Entry for a single agent in the agents terminal.
pub(crate) struct AgentTerminalEntry {
    pub(crate) name: String,
    pub(crate) batch_id: Option<String>,
    pub(crate) model: Option<String>,
    pub(crate) status: AgentStatus,
    pub(crate) last_progress: Option<String>,
    pub(crate) result: Option<String>,
    pub(crate) error: Option<String>,
    pub(crate) logs: Vec<AgentLogEntry>,
}

impl AgentTerminalEntry {
    pub(crate) fn new(
        name: String,
        model: Option<String>,
        status: AgentStatus,
        batch_id: Option<String>,
    ) -> Self {
        Self {
            name,
            batch_id,
            model,
            status,
            last_progress: None,
            result: None,
            error: None,
            logs: Vec::new(),
        }
    }

    pub(crate) fn push_log(&mut self, kind: AgentLogKind, message: impl Into<String>) {
        let msg = message.into();
        if self
            .logs
            .last()
            .map(|entry| entry.kind == kind && entry.message == msg)
            .unwrap_or(false)
        {
            return;
        }
        self.logs.push(AgentLogEntry {
            timestamp: Local::now(),
            kind,
            message: msg,
        });
        const MAX_HISTORY: usize = 500;
        if self.logs.len() > MAX_HISTORY {
            let excess = self.logs.len() - MAX_HISTORY;
            self.logs.drain(0..excess);
        }
    }
}

/// State for the agents terminal overlay.
pub(crate) struct AgentsTerminalState {
    pub(crate) active: bool,
    pub(crate) selected_index: usize,
    pub(crate) order: Vec<String>,
    pub(crate) entries: HashMap<String, AgentTerminalEntry>,
    pub(crate) scroll_offsets: HashMap<String, u16>,
    pub(crate) saved_scroll_offset: u16,
    pub(crate) shared_context: Option<String>,
    pub(crate) shared_task: Option<String>,
    focus: AgentsTerminalFocus,
}

impl AgentsTerminalState {
    pub(crate) fn new() -> Self {
        Self {
            active: false,
            selected_index: 0,
            order: Vec::new(),
            entries: HashMap::new(),
            scroll_offsets: HashMap::new(),
            saved_scroll_offset: 0,
            shared_context: None,
            shared_task: None,
            focus: AgentsTerminalFocus::Sidebar,
        }
    }

    pub(crate) fn reset(&mut self) {
        self.selected_index = 0;
        self.order.clear();
        self.entries.clear();
        self.scroll_offsets.clear();
        self.shared_context = None;
        self.shared_task = None;
        self.focus = AgentsTerminalFocus::Sidebar;
    }

    pub(crate) fn current_agent_id(&self) -> Option<&str> {
        self.order.get(self.selected_index).map(String::as_str)
    }

    pub(crate) fn focus_sidebar(&mut self) {
        self.focus = AgentsTerminalFocus::Sidebar;
    }

    pub(crate) fn focus_detail(&mut self) {
        self.focus = AgentsTerminalFocus::Detail;
    }

    pub(crate) fn focus(&self) -> AgentsTerminalFocus {
        self.focus
    }
}

/// Focus state for the agents terminal split-pane.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum AgentsTerminalFocus {
    Sidebar,
    Detail,
}

// =========================================================================
// ChatWidget impl - Agents Terminal Methods
// =========================================================================

impl ChatWidget<'_> {
    /// Update agents terminal state from agent info events.
    pub(crate) fn update_agents_terminal_state(
        &mut self,
        agents: &[codex_core::protocol::AgentInfo],
        context: Option<String>,
        task: Option<String>,
    ) {
        self.agents_terminal.shared_context = context;
        self.agents_terminal.shared_task = task;

        let mut saw_new_agent = false;
        for info in agents {
            let status = agent_status_from_str(info.status.as_str());
            let is_new = !self.agents_terminal.entries.contains_key(&info.id);
            if is_new && !self.agents_terminal.order.iter().any(|id| id == &info.id) {
                self.agents_terminal.order.push(info.id.clone());
                saw_new_agent = true;
            }

            let entry = self.agents_terminal.entries.entry(info.id.clone());
            let entry = entry.or_insert_with(|| {
                saw_new_agent = true;
                let mut new_entry = AgentTerminalEntry::new(
                    info.name.clone(),
                    info.model.clone(),
                    status.clone(),
                    info.batch_id.clone(),
                );
                new_entry.push_log(
                    AgentLogKind::Status,
                    format!("Status → {}", agent_status_label(status.clone())),
                );
                new_entry
            });

            entry.name = info.name.clone();
            entry.batch_id = info.batch_id.clone();
            entry.model = info.model.clone();

            if entry.status != status {
                entry.status = status.clone();
                entry.push_log(
                    AgentLogKind::Status,
                    format!("Status → {}", agent_status_label(status.clone())),
                );
            }

            if let Some(progress) = info.last_progress.as_ref()
                && entry.last_progress.as_ref() != Some(progress)
            {
                entry.last_progress = Some(progress.clone());
                entry.push_log(AgentLogKind::Progress, progress.clone());
            }

            if let Some(result) = info.result.as_ref()
                && entry.result.as_ref() != Some(result)
            {
                entry.result = Some(result.clone());
                entry.push_log(AgentLogKind::Result, result.clone());
            }

            if let Some(error) = info.error.as_ref()
                && entry.error.as_ref() != Some(error)
            {
                entry.error = Some(error.clone());
                entry.push_log(AgentLogKind::Error, error.clone());
            }
        }

        if self.agents_terminal.selected_index >= self.agents_terminal.order.len()
            && !self.agents_terminal.order.is_empty()
        {
            self.agents_terminal.selected_index = self.agents_terminal.order.len() - 1;
        }

        if saw_new_agent && self.agents_terminal.active {
            self.layout.scroll_offset = 0;
        }
    }

    /// Enter agents terminal overlay mode.
    pub(crate) fn enter_agents_terminal_mode(&mut self) {
        if self.agents_terminal.active {
            return;
        }
        self.agents_terminal.active = true;
        self.agents_terminal.focus_sidebar();
        self.bottom_pane.set_input_focus(false);
        self.agents_terminal.saved_scroll_offset = self.layout.scroll_offset;
        self.layout.agents_hud_expanded = false;
        if self.agents_terminal.order.is_empty() {
            for agent in &self.active_agents {
                if !self.agents_terminal.entries.contains_key(&agent.id) {
                    self.agents_terminal.order.push(agent.id.clone());
                    let mut entry = AgentTerminalEntry::new(
                        agent.name.clone(),
                        agent.model.clone(),
                        agent.status.clone(),
                        agent.batch_id.clone(),
                    );
                    if let Some(progress) = agent.last_progress.as_ref() {
                        entry.last_progress = Some(progress.clone());
                        entry.push_log(AgentLogKind::Progress, progress.clone());
                    }
                    if let Some(result) = agent.result.as_ref() {
                        entry.result = Some(result.clone());
                        entry.push_log(AgentLogKind::Result, result.clone());
                    }
                    if let Some(error) = agent.error.as_ref() {
                        entry.error = Some(error.clone());
                        entry.push_log(AgentLogKind::Error, error.clone());
                    }
                    self.agents_terminal.entries.insert(agent.id.clone(), entry);
                }
            }
        }
        self.restore_selected_agent_scroll();
        self.request_redraw();
    }

    /// Exit agents terminal overlay mode.
    pub(crate) fn exit_agents_terminal_mode(&mut self) {
        if !self.agents_terminal.active {
            return;
        }
        self.record_current_agent_scroll();
        self.agents_terminal.active = false;
        self.agents_terminal.focus_sidebar();
        self.layout.scroll_offset = self.agents_terminal.saved_scroll_offset;
        self.bottom_pane.set_input_focus(true);
        self.request_redraw();
    }

    /// Toggle agents terminal overlay on/off.
    pub(crate) fn toggle_agents_hud(&mut self) {
        if self.agents_terminal.active {
            self.exit_agents_terminal_mode();
        } else {
            self.enter_agents_terminal_mode();
        }
    }

    /// Record scroll offset for current agent.
    pub(crate) fn record_current_agent_scroll(&mut self) {
        if let Some(id) = self.agents_terminal.current_agent_id() {
            let capped = self
                .layout
                .scroll_offset
                .min(self.layout.last_max_scroll.get());
            self.agents_terminal
                .scroll_offsets
                .insert(id.to_string(), capped);
        }
    }

    /// Restore scroll offset for selected agent.
    pub(crate) fn restore_selected_agent_scroll(&mut self) {
        let offset = self
            .agents_terminal
            .current_agent_id()
            .and_then(|id| self.agents_terminal.scroll_offsets.get(id).copied())
            .unwrap_or(0);
        self.layout.scroll_offset = offset;
    }

    /// Navigate agents terminal selection by delta.
    pub(crate) fn navigate_agents_terminal_selection(&mut self, delta: isize) {
        if self.agents_terminal.order.is_empty() {
            return;
        }
        self.agents_terminal.focus_sidebar();
        let len = self.agents_terminal.order.len() as isize;
        self.record_current_agent_scroll();
        let mut new_index = self.agents_terminal.selected_index as isize + delta;
        if new_index >= len {
            new_index %= len;
        }
        while new_index < 0 {
            new_index += len;
        }
        self.agents_terminal.selected_index = new_index as usize;
        self.restore_selected_agent_scroll();
        self.request_redraw();
    }

    /// Render the agents terminal overlay.
    pub(crate) fn render_agents_terminal_overlay(
        &self,
        frame_area: Rect,
        history_area: Rect,
        bottom_pane_area: Rect,
        buf: &mut Buffer,
    ) {
        use ratatui::layout::{Constraint, Direction, Layout, Margin, Rect as RtRect};
        use ratatui::style::{Modifier, Style};
        use ratatui::text::{Line, Span};
        use ratatui::widgets::{Block, Borders, Clear, List, ListItem, ListState, Paragraph, Wrap};

        let scrim_style = Style::default()
            .bg(crate::colors::overlay_scrim())
            .fg(crate::colors::text_dim());
        fill_rect(buf, frame_area, None, scrim_style);

        let padding = 1u16;
        let footer_reserved = bottom_pane_area.height.min(1);
        let overlay_bottom =
            (bottom_pane_area.y + bottom_pane_area.height).saturating_sub(footer_reserved);
        let overlay_height = overlay_bottom
            .saturating_sub(history_area.y)
            .max(1)
            .min(frame_area.height);

        let window_area = Rect {
            x: history_area.x + padding,
            y: history_area.y,
            width: history_area.width.saturating_sub(padding * 2),
            height: overlay_height,
        };
        Clear.render(window_area, buf);

        let title_spans = vec![
            Span::styled(" Agents ", Style::default().fg(crate::colors::text())),
            Span::styled(
                "— Ctrl+A to close",
                Style::default().fg(crate::colors::text_dim()),
            ),
        ];

        let block = Block::default()
            .borders(Borders::ALL)
            .title(Line::from(title_spans))
            .style(Style::default().bg(crate::colors::background()))
            .border_style(
                Style::default()
                    .fg(crate::colors::border())
                    .bg(crate::colors::background()),
            );
        let inner = block.inner(window_area);
        block.render(window_area, buf);

        let inner_bg = Style::default().bg(crate::colors::background());
        for y in inner.y..inner.y + inner.height {
            for x in inner.x..inner.x + inner.width {
                buf[(x, y)].set_style(inner_bg);
            }
        }

        let content = inner.inner(Margin::new(1, 1));
        if content.width == 0 || content.height == 0 {
            return;
        }

        let hint_height = if content.height >= 2 { 1 } else { 0 };
        let body_height = content.height.saturating_sub(hint_height);
        let body_area = RtRect {
            x: content.x,
            y: content.y,
            width: content.width,
            height: body_height,
        };
        let hint_area = RtRect {
            x: content.x,
            y: content.y.saturating_add(body_height),
            width: content.width,
            height: hint_height,
        };

        let sidebar_target = 28u16;
        let sidebar_width = if body_area.width <= sidebar_target + 12 {
            (body_area.width.saturating_mul(35) / 100).clamp(16, body_area.width)
        } else {
            sidebar_target
                .min(body_area.width.saturating_sub(12))
                .max(16)
        };

        let constraints = if body_area.width <= sidebar_width {
            [Constraint::Length(body_area.width), Constraint::Length(0)]
        } else {
            [Constraint::Length(sidebar_width), Constraint::Min(12)]
        };

        let chunks = Layout::default()
            .direction(Direction::Horizontal)
            .constraints(constraints)
            .split(body_area);

        // Sidebar list of agents grouped by batch id
        let mut items: Vec<ListItem> = Vec::new();
        let mut display_ids: Vec<Option<String>> = Vec::new();
        if !self.agents_terminal.order.is_empty() {
            let mut groups: Vec<(Option<String>, Vec<String>)> = Vec::new();
            let mut group_lookup: HashMap<Option<String>, usize> = HashMap::new();

            for id in &self.agents_terminal.order {
                if let Some(entry) = self.agents_terminal.entries.get(id) {
                    let key = entry.batch_id.clone();
                    let idx = if let Some(idx) = group_lookup.get(&key) {
                        *idx
                    } else {
                        let idx = groups.len();
                        group_lookup.insert(key.clone(), idx);
                        groups.push((key.clone(), Vec::new()));
                        idx
                    };
                    groups[idx].1.push(id.clone());
                }
            }

            for (batch_id, ids) in groups {
                let count_label = if ids.len() == 1 {
                    "1 agent".to_string()
                } else {
                    format!("{} agents", ids.len())
                };
                let header_label = match batch_id.as_ref() {
                    Some(batch) => {
                        let short: String = batch.chars().take(8).collect();
                        if short.is_empty() {
                            format!("Batch · {count_label}")
                        } else {
                            format!("Batch {short} · {count_label}")
                        }
                    }
                    None => format!("Ad-hoc · {count_label}"),
                };
                items.push(ListItem::new(Line::from(vec![
                    Span::raw(" "),
                    Span::styled(
                        header_label,
                        Style::default()
                            .fg(crate::colors::text_dim())
                            .add_modifier(Modifier::BOLD),
                    ),
                ])));
                display_ids.push(None);

                for id in ids {
                    if let Some(entry) = self.agents_terminal.entries.get(&id) {
                        let status_color = agent_status_color(entry.status.clone());
                        let spans = vec![
                            Span::raw(" "),
                            Span::styled("• ", Style::default().fg(status_color)),
                            Span::styled(
                                entry.name.clone(),
                                Style::default().fg(crate::colors::text()),
                            ),
                        ];
                        items.push(ListItem::new(Line::from(spans)));
                        display_ids.push(Some(id));
                    }
                }
            }
        }

        if items.is_empty() {
            items.push(ListItem::new(Line::from(vec![
                Span::raw(" "),
                Span::styled(
                    "No agents yet",
                    Style::default().fg(crate::colors::text_dim()),
                ),
            ])));
        }

        let mut list_state = ListState::default();
        if !display_ids.is_empty() && !self.agents_terminal.order.is_empty() {
            let idx = self
                .agents_terminal
                .selected_index
                .min(self.agents_terminal.order.len().saturating_sub(1));
            if let Some(selected_id) = self.agents_terminal.order.get(idx)
                && let Some(list_idx) = display_ids
                    .iter()
                    .position(|maybe_id| maybe_id.as_ref() == Some(selected_id))
            {
                list_state.select(Some(list_idx));
            }
        }

        let sidebar_has_focus = self.agents_terminal.focus() == AgentsTerminalFocus::Sidebar;
        let sidebar_border_color = if sidebar_has_focus {
            crate::colors::border_focused()
        } else {
            crate::colors::border()
        };
        let sidebar_block = Block::default()
            .borders(Borders::ALL)
            .title(" Agents ")
            .border_style(Style::default().fg(sidebar_border_color));
        let sidebar = List::new(items)
            .block(sidebar_block)
            .highlight_style(
                Style::default()
                    .fg(crate::colors::primary())
                    .add_modifier(Modifier::BOLD),
            )
            .highlight_symbol("➤ ");
        ratatui::widgets::StatefulWidget::render(sidebar, chunks[0], buf, &mut list_state);

        let right_area = if chunks.len() > 1 {
            chunks[1]
        } else {
            chunks[0]
        };
        let mut lines: Vec<Line> = Vec::new();

        if let Some(agent_id) = self.agents_terminal.current_agent_id() {
            if let Some(entry) = self.agents_terminal.entries.get(agent_id) {
                lines.push(Line::from(vec![
                    Span::raw(" "),
                    Span::styled(
                        entry.name.clone(),
                        Style::default()
                            .fg(crate::colors::text())
                            .add_modifier(Modifier::BOLD),
                    ),
                    Span::raw("  "),
                    Span::styled(
                        agent_status_label(entry.status.clone()),
                        Style::default().fg(agent_status_color(entry.status.clone())),
                    ),
                    Span::raw("  "),
                    Span::styled(
                        format!("#{}", agent_id.chars().take(7).collect::<String>()),
                        Style::default().fg(crate::colors::text_dim()),
                    ),
                ]));

                if let Some(model) = entry.model.as_ref() {
                    lines.push(Line::from(vec![
                        Span::raw(" "),
                        Span::styled(
                            format!("Model: {model}"),
                            Style::default().fg(crate::colors::text_dim()),
                        ),
                    ]));
                }
                if let Some(context) = self.agents_terminal.shared_context.as_ref() {
                    lines.push(Line::from(vec![
                        Span::raw(" "),
                        Span::styled(
                            format!("Context: {context}"),
                            Style::default().fg(crate::colors::text_dim()),
                        ),
                    ]));
                }
                if let Some(task) = self.agents_terminal.shared_task.as_ref() {
                    lines.push(Line::from(vec![
                        Span::raw(" "),
                        Span::styled(
                            format!("Task: {task}"),
                            Style::default().fg(crate::colors::text_dim()),
                        ),
                    ]));
                }

                lines.push(Line::from(""));

                if entry.logs.is_empty() {
                    lines.push(Line::from(vec![
                        Span::raw(" "),
                        Span::styled(
                            "No updates yet",
                            Style::default().fg(crate::colors::text_dim()),
                        ),
                    ]));
                } else {
                    for (idx, log) in entry.logs.iter().enumerate() {
                        let timestamp = log.timestamp.format("%H:%M:%S");
                        let label = agent_log_label(log.kind);
                        let color = agent_log_color(log.kind);
                        let label_style = Style::default().fg(color).add_modifier(Modifier::BOLD);

                        match log.kind {
                            AgentLogKind::Result => {
                                lines.push(Line::from(vec![
                                    Span::raw(" "),
                                    Span::styled(
                                        format!("[{timestamp}] "),
                                        Style::default().fg(crate::colors::text_dim()),
                                    ),
                                    Span::styled(label, label_style),
                                    Span::raw(": "),
                                ]));

                                let mut markdown_lines: Vec<Line<'static>> = Vec::new();
                                crate::markdown::append_markdown(
                                    log.message.as_str(),
                                    &mut markdown_lines,
                                    &self.config,
                                );

                                if markdown_lines.is_empty() {
                                    lines.push(Line::from(vec![
                                        Span::raw(" "),
                                        Span::styled(
                                            "(no result)",
                                            Style::default().fg(crate::colors::text_dim()),
                                        ),
                                    ]));
                                } else {
                                    for line in markdown_lines.into_iter() {
                                        let mut spans = line.spans;
                                        spans.insert(0, Span::raw(" "));
                                        lines.push(Line::from(spans));
                                    }
                                }

                                if idx + 1 < entry.logs.len() {
                                    lines.push(Line::from(""));
                                }
                            }
                            _ => {
                                lines.push(Line::from(vec![
                                    Span::raw(" "),
                                    Span::styled(
                                        format!("[{timestamp}] "),
                                        Style::default().fg(crate::colors::text_dim()),
                                    ),
                                    Span::styled(label, label_style),
                                    Span::raw(": "),
                                    Span::styled(
                                        log.message.clone(),
                                        Style::default().fg(crate::colors::text()),
                                    ),
                                ]));
                            }
                        }
                    }
                }
            } else {
                lines.push(Line::from(vec![
                    Span::raw(" "),
                    Span::styled(
                        "No data for selected agent",
                        Style::default().fg(crate::colors::text_dim()),
                    ),
                ]));
            }
        } else {
            lines.push(Line::from(vec![
                Span::raw(" "),
                Span::styled(
                    "No agents available",
                    Style::default().fg(crate::colors::text_dim()),
                ),
            ]));
        }

        let viewport_height = right_area.height.max(1);
        let total_lines = lines.len() as u16;
        let max_scroll = total_lines.saturating_sub(viewport_height);
        self.layout
            .last_history_viewport_height
            .set(viewport_height);
        self.layout.last_max_scroll.set(max_scroll);

        let detail_has_focus = self.agents_terminal.focus() == AgentsTerminalFocus::Detail;
        let detail_border_color = if detail_has_focus {
            crate::colors::border_focused()
        } else {
            crate::colors::border()
        };
        let history_block = Block::default()
            .borders(Borders::ALL)
            .title(" Agent History ")
            .border_style(Style::default().fg(detail_border_color));

        Paragraph::new(lines)
            .block(history_block)
            .wrap(Wrap { trim: false })
            .scroll((self.layout.scroll_offset.min(max_scroll), 0))
            .render(right_area, buf);

        if hint_height == 1 {
            let hint_line = Line::from(vec![
                Span::styled("↑/↓", Style::default().fg(crate::colors::function())),
                Span::styled(
                    " Navigate/Scroll  ",
                    Style::default().fg(crate::colors::text_dim()),
                ),
                Span::styled("→/Enter", Style::default().fg(crate::colors::function())),
                Span::styled(
                    " Focus output  ",
                    Style::default().fg(crate::colors::text_dim()),
                ),
                Span::styled("←", Style::default().fg(crate::colors::function())),
                Span::styled(
                    " Back to list  ",
                    Style::default().fg(crate::colors::text_dim()),
                ),
                Span::styled("Tab", Style::default().fg(crate::colors::function())),
                Span::styled(
                    " Next agent  ",
                    Style::default().fg(crate::colors::text_dim()),
                ),
                Span::styled("PgUp/PgDn", Style::default().fg(crate::colors::function())),
                Span::styled(
                    " Page scroll  ",
                    Style::default().fg(crate::colors::text_dim()),
                ),
                Span::styled("Esc", Style::default().fg(crate::colors::error())),
                Span::styled(" Exit", Style::default().fg(crate::colors::text_dim())),
            ]);
            Paragraph::new(hint_line)
                .style(Style::default().bg(crate::colors::background()))
                .alignment(ratatui::layout::Alignment::Center)
                .render(hint_area, buf);
        }
    }
}
