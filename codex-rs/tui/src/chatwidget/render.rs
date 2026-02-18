//! Rendering functions for ChatWidget.
//!
//! Extracted from mod.rs to reduce file size and merge conflict risk.
//! Contains: render_hud, render_pro_header, render_pro_panel,
//! render_limits_overlay, render_agents_header, render_agent_panel,
//! and the WidgetRef implementation (render_ref).

use super::*;

impl ChatWidget<'_> {
    /// Render the combined HUD with agent and/or pro panels (stacked full-width)
    fn render_hud(&self, area: Rect, buf: &mut Buffer) {
        // Check what's active
        let has_active_agents = !self.active_agents.is_empty() || self.agents_ready_to_start;
        let has_pro = self.pro_surface_present();

        if !has_active_agents && !has_pro {
            return;
        }

        // Add same horizontal padding as the Message input (2 chars on each side)
        let horizontal_padding = 1u16;
        let padded_area = Rect {
            x: area.x + horizontal_padding,
            y: area.y,
            width: area.width.saturating_sub(horizontal_padding * 2),
            height: area.height,
        };
        if padded_area.height == 0 {
            return;
        }

        let header_h: u16 = 3;
        let term_h = self.layout.last_frame_height.get().max(1);
        let thirty = ((term_h as u32) * 30 / 100) as u16;
        let sixty = ((term_h as u32) * 60 / 100) as u16;
        let mut expanded_target = if thirty < 25 { 25.min(sixty) } else { thirty };
        let min_expanded = header_h.saturating_add(2);
        if expanded_target < min_expanded {
            expanded_target = min_expanded;
        }

        #[derive(Copy, Clone)]
        enum HudKind {
            Agents,
            Pro,
        }

        let mut panels: Vec<(HudKind, bool)> = Vec::new();
        if has_active_agents {
            panels.push((HudKind::Agents, self.layout.agents_hud_expanded));
        }
        if has_pro {
            panels.push((HudKind::Pro, self.layout.pro_hud_expanded));
        }

        if panels.is_empty() {
            return;
        }

        let mut constraints: Vec<Constraint> = Vec::with_capacity(panels.len());
        let mut remaining = padded_area.height;
        for (idx, (_, expanded)) in panels.iter().enumerate() {
            if remaining == 0 {
                constraints.push(Constraint::Length(0));
                continue;
            }
            let desired = if *expanded {
                expanded_target.min(remaining)
            } else {
                header_h.min(remaining)
            };
            let length = if idx == panels.len() - 1 {
                desired.max(remaining)
            } else {
                desired
            };
            let length = length.min(remaining);
            constraints.push(Constraint::Length(length));
            remaining = remaining.saturating_sub(length);
        }

        let chunks = Layout::vertical(constraints).split(padded_area);
        let count = panels.len().min(chunks.len());
        for idx in 0..count {
            let rect = chunks[idx];
            let (kind, expanded) = panels[idx];
            match (kind, expanded) {
                (HudKind::Agents, true) => self.render_agent_panel(rect, buf),
                (HudKind::Agents, false) => self.render_agents_header(rect, buf),
                (HudKind::Pro, true) => self.render_pro_panel(rect, buf),
                (HudKind::Pro, false) => self.render_pro_header(rect, buf),
            }
        }
    }

    fn render_pro_header(&self, area: Rect, buf: &mut Buffer) {
        use ratatui::layout::Margin;
        use ratatui::text::Line as RLine;
        use ratatui::text::Span;
        use ratatui::widgets::Block;
        use ratatui::widgets::Borders;
        use ratatui::widgets::Paragraph;

        let block = Block::default()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(crate::colors::border()))
            .title(" Pro ");
        let inner = block.inner(area);
        block.render(area, buf);
        let content = inner.inner(Margin::new(1, 0));

        let dot_color = if self.pro.enabled {
            crate::colors::success_green()
        } else {
            crate::colors::text_dim()
        };
        let mut left_spans: Vec<Span> = Vec::new();
        left_spans.push(Span::styled("•", Style::default().fg(dot_color)));
        left_spans.push(Span::raw(" "));
        left_spans.push(Span::raw(self.pro_summary_line()));

        let action = if self.layout.pro_hud_expanded {
            " collapse"
        } else {
            " expand"
        };
        let key_style = Style::default().fg(crate::colors::function());
        let label_style = Style::default().dim();
        let mut right_spans: Vec<Span> = Vec::new();
        right_spans.push(Span::from("Ctrl+Shift+P").style(key_style));
        right_spans.push(Span::styled(action, label_style));
        right_spans.push(Span::raw("  "));
        right_spans.push(Span::from("Ctrl+P").style(key_style));
        right_spans.push(Span::styled(" overlay", label_style));

        let measure =
            |spans: &Vec<Span>| -> usize { spans.iter().map(|s| s.content.chars().count()).sum() };
        let left_len = measure(&left_spans);
        let right_len = measure(&right_spans);
        let total_width = content.width as usize;
        if total_width > left_len + right_len {
            left_spans.push(Span::from(" ".repeat(total_width - left_len - right_len)));
        }
        let mut spans = left_spans;
        spans.extend(right_spans);
        Paragraph::new(RLine::from(spans)).render(content, buf);
    }

    fn render_pro_panel(&self, area: Rect, buf: &mut Buffer) {
        use ratatui::layout::Margin;
        use ratatui::text::Line as RLine;
        use ratatui::text::Span;
        use ratatui::widgets::Block;
        use ratatui::widgets::Borders;
        use ratatui::widgets::Paragraph;
        use ratatui::widgets::Wrap;

        let block = Block::default()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(crate::colors::border()))
            .title(" Pro ");
        let inner = block.inner(area);
        block.render(area, buf);
        let content = inner.inner(Margin::new(1, 0));
        if content.height == 0 {
            return;
        }

        let mut lines: Vec<RLine<'static>> = Vec::new();
        let summary_style = Style::default()
            .fg(crate::colors::text())
            .add_modifier(Modifier::BOLD);
        lines.push(RLine::from(vec![Span::styled(
            self.pro_summary_line(),
            summary_style,
        )]));
        let key_style = Style::default().fg(crate::colors::function());
        let label_style = Style::default().fg(crate::colors::text_dim());
        lines.push(RLine::from(vec![
            Span::raw(" "),
            Span::from("Ctrl+Shift+P").style(key_style),
            Span::styled(" collapse  ", label_style),
            Span::from("Ctrl+P").style(key_style),
            Span::styled(" overlay", label_style),
        ]));
        lines.push(RLine::from(" "));

        if self.pro.log.is_empty() {
            lines.push(RLine::from(vec![Span::styled(
                "No Pro activity yet",
                Style::default().fg(crate::colors::text_dim()),
            )]));
        } else {
            for entry in self.pro.log.iter().rev() {
                for line in self.format_pro_log_entry(entry) {
                    lines.push(line);
                }
                lines.push(RLine::from(" "));
            }
            // Remove trailing blank line for neatness
            if lines
                .last()
                .map(|line| line.spans.iter().all(|s| s.content.trim().is_empty()))
                .unwrap_or(false)
            {
                lines.pop();
            }
        }

        Paragraph::new(lines)
            .wrap(Wrap { trim: true })
            .render(content, buf);
    }

    // MAINT-11: render_pro_overlay moved to pro_overlay.rs

    fn render_limits_overlay(&self, frame_area: Rect, history_area: Rect, buf: &mut Buffer) {
        use ratatui::layout::Margin;
        use ratatui::text::Line as RLine;
        use ratatui::text::Span;
        use ratatui::widgets::Block;
        use ratatui::widgets::Borders;
        use ratatui::widgets::Clear;
        use ratatui::widgets::Paragraph;
        use ratatui::widgets::Wrap;

        let Some(overlay) = self.limits.overlay.as_ref() else {
            return;
        };

        let tab_count = overlay.tab_count();

        let scrim_style = Style::default()
            .bg(crate::colors::overlay_scrim())
            .fg(crate::colors::text_dim());
        fill_rect(buf, frame_area, None, scrim_style);

        let padding = 1u16;
        let overlay_area = Rect {
            x: history_area.x + padding,
            y: history_area.y,
            width: history_area.width.saturating_sub(padding * 2),
            height: history_area.height,
        };

        Clear.render(overlay_area, buf);

        let dim_style = Style::default().fg(crate::colors::text_dim());
        let mut title_spans: Vec<Span<'static>> = vec![Span::styled(
            " Rate limits ",
            Style::default().fg(crate::colors::text()),
        )];
        if tab_count > 1 {
            title_spans.extend_from_slice(&[
                Span::styled("——— ", dim_style),
                Span::styled("◂ ▸", Style::default().fg(crate::colors::function())),
                Span::styled(" change account ", dim_style),
            ]);
        }
        title_spans.extend_from_slice(&[
            Span::styled("——— ", dim_style),
            Span::styled("Esc", Style::default().fg(crate::colors::text())),
            Span::styled(" close ", dim_style),
            Span::styled("——— ", dim_style),
            Span::styled("↑↓", Style::default().fg(crate::colors::function())),
            Span::styled(" scroll", dim_style),
        ]);
        let title = RLine::from(title_spans);

        let block = Block::default()
            .borders(Borders::ALL)
            .title(title)
            .style(Style::default().bg(crate::colors::background()))
            .border_style(
                Style::default()
                    .fg(crate::colors::border())
                    .bg(crate::colors::background()),
            );
        let inner = block.inner(overlay_area);
        block.render(overlay_area, buf);

        let body = inner.inner(Margin::new(1, 1));
        if body.width == 0 || body.height == 0 {
            overlay.set_visible_rows(0);
            overlay.set_max_scroll(0);
            return;
        }

        let (tabs_area, content_area) = if tab_count > 1 {
            let [tabs_area, content_area] =
                Layout::vertical([Constraint::Length(2), Constraint::Fill(1)]).areas(body);
            (Some(tabs_area), content_area)
        } else {
            (None, body)
        };

        if let Some(area) = tabs_area
            && let Some(tabs) = overlay.tabs()
        {
            let labels: Vec<String> = tabs
                .iter()
                .map(|tab| format!("  {}  ", tab.title))
                .collect();

            let mut constraints: Vec<Constraint> = Vec::new();
            let mut consumed: u16 = 0;
            for label in &labels {
                let width = label.chars().count() as u16;
                let remaining = area.width.saturating_sub(consumed);
                let w = width.min(remaining);
                constraints.push(Constraint::Length(w));
                consumed = consumed.saturating_add(w);
                if consumed >= area.width.saturating_sub(4) {
                    break;
                }
            }
            constraints.push(Constraint::Fill(1));

            let chunks = Layout::horizontal(constraints).split(area);

            let tabs_bottom_rule = Block::default()
                .borders(Borders::BOTTOM)
                .border_style(Style::default().fg(crate::colors::border()));
            tabs_bottom_rule.render(area, buf);

            let selected_idx = overlay.selected_tab();

            for (idx, label) in labels.iter().enumerate() {
                if idx >= chunks.len().saturating_sub(1) {
                    break;
                }
                let rect = chunks[idx];
                if rect.width == 0 {
                    continue;
                }

                let selected = idx == selected_idx;
                let bg_style = Style::default().bg(crate::colors::background());
                fill_rect(buf, rect, None, bg_style);

                let label_rect = Rect {
                    x: rect.x + 1,
                    y: rect.y,
                    width: rect.width.saturating_sub(2),
                    height: 1,
                };
                let label_style = if selected {
                    Style::default()
                        .fg(crate::colors::text())
                        .add_modifier(Modifier::BOLD)
                } else {
                    dim_style
                };
                let line = RLine::from(Span::styled(label.clone(), label_style));
                Paragraph::new(RtText::from(vec![line]))
                    .wrap(Wrap { trim: true })
                    .render(label_rect, buf);

                if selected {
                    let accent_width = label.chars().count() as u16;
                    let accent_rect = Rect {
                        x: label_rect.x,
                        y: rect.y + rect.height.saturating_sub(1),
                        width: accent_width.min(label_rect.width).max(1),
                        height: 1,
                    };
                    let underline = Block::default()
                        .borders(Borders::BOTTOM)
                        .border_style(Style::default().fg(crate::colors::text_bright()));
                    underline.render(accent_rect, buf);
                }
            }
        }

        let text_area = content_area;

        let lines = overlay.lines_for_width(text_area.width);
        let total_lines = lines.len();
        let visible_rows = text_area.height as usize;
        overlay.set_visible_rows(text_area.height);
        let max_scroll = total_lines
            .saturating_sub(visible_rows.max(1))
            .min(u16::MAX as usize) as u16;
        overlay.set_max_scroll(max_scroll);

        let scroll = overlay.scroll().min(max_scroll) as usize;
        let end = (scroll + visible_rows).min(total_lines);
        let slice = if scroll < total_lines {
            lines[scroll..end].to_vec()
        } else {
            Vec::new()
        };

        fill_rect(
            buf,
            text_area,
            Some(' '),
            Style::default().bg(crate::colors::background()),
        );

        Paragraph::new(RtText::from(slice))
            .wrap(Wrap { trim: false })
            .render(text_area, buf);
    }

    // MAINT-11: Pro helper functions (pro_summary_line, format_pro_log_entry, etc.) moved to pro_overlay.rs

    /// Render a collapsed header for the agents HUD with counts/list (1 line + border)
    fn render_agents_header(&self, area: Rect, buf: &mut Buffer) {
        use ratatui::layout::Margin;
        use ratatui::text::Line as RLine;
        use ratatui::text::Span;
        use ratatui::widgets::Block;
        use ratatui::widgets::Borders;
        use ratatui::widgets::Paragraph;

        let count = self.active_agents.len();
        let summary = if count == 0 && self.agents_ready_to_start {
            "Starting...".to_string()
        } else if count == 0 {
            "no active agents".to_string()
        } else {
            let mut parts: Vec<String> = Vec::new();
            for a in self.active_agents.iter().take(3) {
                let state = match a.status {
                    AgentStatus::Pending => "pending".to_string(),
                    AgentStatus::Running => {
                        // Show elapsed running time when available
                        if let Some(rt) = self.agent_runtime.get(&a.id) {
                            if let Some(start) = rt.started_at {
                                let now = Instant::now();
                                let elapsed = now.saturating_duration_since(start);
                                format!("running {}", self.fmt_short_duration(elapsed))
                            } else {
                                "running".to_string()
                            }
                        } else {
                            "running".to_string()
                        }
                    }
                    AgentStatus::Completed => "done".to_string(),
                    AgentStatus::Failed => "failed".to_string(),
                };
                let mut label = format!("{} ({})", a.name, state);
                if matches!(a.status, AgentStatus::Running)
                    && let Some(lp) = &a.last_progress
                {
                    let mut lp_trim = lp.trim().to_string();
                    if lp_trim.len() > 60 {
                        lp_trim.truncate(60);
                        lp_trim.push('…');
                    }
                    label.push_str(&format!(" — {}", lp_trim));
                }
                parts.push(label);
            }
            let extra = if count > 3 {
                format!(" +{}", count - 3)
            } else {
                String::new()
            };
            format!("{}{}", parts.join(", "), extra)
        };

        let block = Block::default()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(crate::colors::border()))
            .title(" Agents ");
        let inner = block.inner(area);
        block.render(area, buf);
        let content = inner.inner(Margin::new(1, 0)); // 1 space padding inside border

        let key_hint_style = Style::default().fg(crate::colors::function());
        let label_style = Style::default().dim(); // match top status bar label

        // Left side: status dot + text (no label) and Agents summary
        let mut left_spans: Vec<Span> = Vec::new();
        let is_active = !self.active_agents.is_empty() || self.agents_ready_to_start;
        let dot_style = if is_active {
            Style::default().fg(crate::colors::success_green())
        } else {
            Style::default().fg(crate::colors::text_dim())
        };
        left_spans.push(Span::styled("•", dot_style));
        // no status text; dot conveys status
        // single space between dot and summary; no label/separator
        left_spans.push(Span::raw(" "));
        left_spans.push(Span::raw(summary));

        // Right side: hint for opening terminal (Ctrl+A)
        let right_spans: Vec<Span> = vec![
            Span::from("Ctrl+A").style(key_hint_style),
            Span::styled(" open terminal", label_style),
        ];

        let measure =
            |spans: &Vec<Span>| -> usize { spans.iter().map(|s| s.content.chars().count()).sum() };
        let left_len = measure(&left_spans);
        let right_len = measure(&right_spans);
        let total_width = content.width as usize;
        let trailing_pad = 0usize;
        if total_width > left_len + right_len + trailing_pad {
            let spacer = " ".repeat(total_width - left_len - right_len - trailing_pad);
            left_spans.push(Span::from(spacer));
        }
        let mut spans = left_spans;
        spans.extend(right_spans);
        Paragraph::new(RLine::from(spans)).render(content, buf);
    }

    /// Render the agent status panel in the HUD
    fn render_agent_panel(&self, area: Rect, buf: &mut Buffer) {
        use ratatui::text::Line as RLine;
        use ratatui::text::Span;
        use ratatui::text::Text;
        use ratatui::widgets::Block;
        use ratatui::widgets::Borders;
        use ratatui::widgets::Paragraph;
        use ratatui::widgets::Sparkline;
        use ratatui::widgets::SparklineBar;
        use ratatui::widgets::Widget;
        use ratatui::widgets::Wrap;

        // Update sparkline data for animation
        if !self.active_agents.is_empty() || self.agents_ready_to_start {
            self.update_sparkline_data();
        }

        // Agent status block
        let agent_block = Block::default()
            .borders(Borders::ALL)
            .title(" Agents ")
            .border_style(Style::default().fg(crate::colors::border()));

        let inner_agent = agent_block.inner(area);
        agent_block.render(area, buf);
        // Render a one-line collapsed header inside expanded panel
        use ratatui::layout::Margin;
        let header_pad = inner_agent.inner(Margin::new(1, 0));
        let header_line = Rect {
            x: header_pad.x,
            y: header_pad.y,
            width: header_pad.width,
            height: 1,
        };
        let key_hint_style = Style::default().fg(crate::colors::function());
        let label_style = Style::default().dim();
        let is_active = !self.active_agents.is_empty() || self.agents_ready_to_start;
        let dot_style = if is_active {
            Style::default().fg(crate::colors::success_green())
        } else {
            Style::default().fg(crate::colors::text_dim())
        };
        // Build summary like collapsed header
        let count = self.active_agents.len();
        let summary = if count == 0 && self.agents_ready_to_start {
            "Starting...".to_string()
        } else if count == 0 {
            "no active agents".to_string()
        } else {
            let mut parts: Vec<String> = Vec::new();
            for a in self.active_agents.iter().take(3) {
                let s = match a.status {
                    AgentStatus::Pending => "pending",
                    AgentStatus::Running => "running",
                    AgentStatus::Completed => "done",
                    AgentStatus::Failed => "failed",
                };
                parts.push(format!("{} ({})", a.name, s));
            }
            let extra = if count > 3 {
                format!(" +{}", count - 3)
            } else {
                String::new()
            };
            format!("{}{}", parts.join(", "), extra)
        };
        let mut left_spans: Vec<Span> = Vec::new();
        left_spans.push(Span::styled("•", dot_style));
        // no status text; dot conveys status
        // single space between dot and summary; no label/separator
        left_spans.push(Span::raw(" "));
        left_spans.push(Span::raw(summary));
        let right_spans: Vec<Span> = vec![
            Span::from("Ctrl+A").style(key_hint_style),
            Span::styled(" open terminal", label_style),
        ];
        let measure =
            |spans: &Vec<Span>| -> usize { spans.iter().map(|s| s.content.chars().count()).sum() };
        let left_len = measure(&left_spans);
        let right_len = measure(&right_spans);
        let total_width = header_line.width as usize;
        if total_width > left_len + right_len {
            left_spans.push(Span::from(" ".repeat(total_width - left_len - right_len)));
        }
        let mut spans = left_spans;
        spans.extend(right_spans);
        Paragraph::new(RLine::from(spans)).render(header_line, buf);

        // Body area excludes the header line and a spacer line
        let inner_agent = Rect {
            x: inner_agent.x,
            y: inner_agent.y + 2,
            width: inner_agent.width,
            height: inner_agent.height.saturating_sub(2),
        };

        // Dynamically calculate sparkline height based on agent activity
        // More agents = taller sparkline area
        let agent_count = self.active_agents.len();
        let sparkline_height = if agent_count == 0 && self.agents_ready_to_start {
            1u16 // Minimal height when preparing
        } else if agent_count == 0 {
            0u16 // No sparkline when no agents
        } else {
            (agent_count as u16 + 1).min(4) // 2-4 lines based on agent count
        };

        // Ensure we have enough space for both content and sparkline
        // Reserve at least 3 lines for content (status + blank + message)
        let min_content_height = 3u16;
        let available_height = inner_agent.height;

        let (actual_content_height, actual_sparkline_height) = if sparkline_height > 0 {
            if available_height > min_content_height + sparkline_height {
                // Enough space for both
                (
                    available_height.saturating_sub(sparkline_height),
                    sparkline_height,
                )
            } else if available_height > min_content_height {
                // Limited space - give minimum to content, rest to sparkline
                (
                    min_content_height,
                    available_height
                        .saturating_sub(min_content_height)
                        .min(sparkline_height),
                )
            } else {
                // Very limited space - content only
                (available_height, 0)
            }
        } else {
            // No sparkline needed
            (available_height, 0)
        };

        let content_area = Rect {
            x: inner_agent.x,
            y: inner_agent.y,
            width: inner_agent.width,
            height: actual_content_height,
        };
        let sparkline_area = Rect {
            x: inner_agent.x,
            y: inner_agent.y + actual_content_height,
            width: inner_agent.width,
            height: actual_sparkline_height,
        };

        // Build all content into a single Text structure for proper wrapping
        let mut text_content = vec![];

        // Add blank line at the top
        text_content.push(RLine::from(" "));

        // Add overall task status at the top
        let status_color = match self.overall_task_status.as_str() {
            "planning" => crate::colors::warning(),
            "running" => crate::colors::info(),
            "consolidating" => crate::colors::warning(),
            "complete" => crate::colors::success(),
            "failed" => crate::colors::error(),
            _ => crate::colors::text_dim(),
        };

        text_content.push(RLine::from(vec![
            Span::from(" "),
            Span::styled(
                "Status: ",
                Style::default()
                    .fg(crate::colors::text())
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled(&self.overall_task_status, Style::default().fg(status_color)),
        ]));

        // Add blank line
        text_content.push(RLine::from(" "));

        // Display agent statuses
        if self.agents_ready_to_start && self.active_agents.is_empty() {
            // Show "Building context..." message when agents are expected
            text_content.push(RLine::from(vec![
                Span::from(" "),
                Span::styled(
                    "Building context...",
                    Style::default()
                        .fg(crate::colors::text_dim())
                        .add_modifier(Modifier::ITALIC),
                ),
            ]));
        } else if self.active_agents.is_empty() {
            text_content.push(RLine::from(vec![
                Span::from(" "),
                Span::styled(
                    "No active agents",
                    Style::default().fg(crate::colors::text_dim()),
                ),
            ]));
        } else {
            // Show agent names/models and final messages
            for agent in &self.active_agents {
                let status_color = match agent.status {
                    AgentStatus::Pending => crate::colors::warning(),
                    AgentStatus::Running => crate::colors::info(),
                    AgentStatus::Completed => crate::colors::success(),
                    AgentStatus::Failed => crate::colors::error(),
                };

                // Build status + timing suffix where available
                let status_text = match agent.status {
                    AgentStatus::Pending => "pending".to_string(),
                    AgentStatus::Running => {
                        if let Some(rt) = self.agent_runtime.get(&agent.id) {
                            if let Some(start) = rt.started_at {
                                let now = Instant::now();
                                let elapsed = now.saturating_duration_since(start);
                                format!("running {}", self.fmt_short_duration(elapsed))
                            } else {
                                "running".to_string()
                            }
                        } else {
                            "running".to_string()
                        }
                    }
                    AgentStatus::Completed | AgentStatus::Failed => {
                        if let Some(rt) = self.agent_runtime.get(&agent.id) {
                            if let (Some(start), Some(done)) = (rt.started_at, rt.completed_at) {
                                let dur = done.saturating_duration_since(start);
                                let base = if matches!(agent.status, AgentStatus::Completed) {
                                    "completed"
                                } else {
                                    "failed"
                                };
                                format!("{} {}", base, self.fmt_short_duration(dur))
                            } else {
                                match agent.status {
                                    AgentStatus::Completed => "completed".to_string(),
                                    AgentStatus::Failed => "failed".to_string(),
                                    _ => unreachable!(),
                                }
                            }
                        } else {
                            match agent.status {
                                AgentStatus::Completed => "completed".to_string(),
                                AgentStatus::Failed => "failed".to_string(),
                                _ => unreachable!(),
                            }
                        }
                    }
                };

                let mut line_spans: Vec<Span> = Vec::new();
                line_spans.push(Span::from(" "));
                line_spans.push(Span::styled(
                    agent.name.to_string(),
                    Style::default()
                        .fg(crate::colors::text())
                        .add_modifier(Modifier::BOLD),
                ));
                if let Some(ref model) = agent.model
                    && !model.is_empty()
                {
                    line_spans.push(Span::styled(
                        format!(" ({})", model),
                        Style::default().fg(crate::colors::text_dim()),
                    ));
                }
                line_spans.push(Span::from(": "));
                line_spans.push(Span::styled(status_text, Style::default().fg(status_color)));
                text_content.push(RLine::from(line_spans));

                // For running agents, show latest progress hint if available
                if matches!(agent.status, AgentStatus::Running)
                    && let Some(ref lp) = agent.last_progress
                {
                    let mut lp_trim = lp.trim().to_string();
                    if lp_trim.len() > 120 {
                        lp_trim.truncate(120);
                        lp_trim.push('…');
                    }
                    text_content.push(RLine::from(vec![
                        Span::from("   "),
                        Span::styled(lp_trim, Style::default().fg(crate::colors::text_dim())),
                    ]));
                }

                // For completed/failed agents, show their final message or error
                match agent.status {
                    AgentStatus::Completed => {
                        if let Some(ref msg) = agent.result {
                            text_content.push(RLine::from(vec![
                                Span::from("   "),
                                Span::styled(msg, Style::default().fg(crate::colors::text_dim())),
                            ]));
                        }
                    }
                    AgentStatus::Failed => {
                        if let Some(ref err) = agent.error {
                            text_content.push(RLine::from(vec![
                                Span::from("   "),
                                Span::styled(
                                    err,
                                    Style::default()
                                        .fg(crate::colors::error())
                                        .add_modifier(Modifier::ITALIC),
                                ),
                            ]));
                        }
                    }
                    _ => {}
                }
            }
        }

        // Calculate how much vertical space the fixed content takes
        let fixed_content_height = text_content.len() as u16;

        // Create the first paragraph for the fixed content (status and agents) without wrapping
        let fixed_paragraph = Paragraph::new(Text::from(text_content));

        // Render the fixed content first
        let fixed_area = Rect {
            x: content_area.x,
            y: content_area.y,
            width: content_area.width,
            height: fixed_content_height.min(content_area.height),
        };
        fixed_paragraph.render(fixed_area, buf);

        // Calculate remaining area for wrapped content
        let remaining_height = content_area.height.saturating_sub(fixed_content_height);
        if remaining_height > 0 {
            let wrapped_area = Rect {
                x: content_area.x,
                y: content_area.y + fixed_content_height,
                width: content_area.width,
                height: remaining_height,
            };

            // Add context and task sections with proper wrapping in the remaining area
            let mut wrapped_content = vec![];

            if let Some(ref task) = self.agent_task {
                wrapped_content.push(RLine::from(" ")); // Empty line separator
                wrapped_content.push(RLine::from(vec![
                    Span::from(" "),
                    Span::styled(
                        "Task:",
                        Style::default()
                            .fg(crate::colors::text())
                            .add_modifier(Modifier::BOLD),
                    ),
                    Span::from(" "),
                    Span::styled(task, Style::default().fg(crate::colors::text_dim())),
                ]));
            }

            if !wrapped_content.is_empty() {
                // Create paragraph with wrapping enabled for the long text content
                let wrapped_paragraph =
                    Paragraph::new(Text::from(wrapped_content)).wrap(Wrap { trim: false });
                wrapped_paragraph.render(wrapped_area, buf);
            }
        }

        // Render sparkline at the bottom if we have data and agents are active
        let sparkline_data = self.sparkline_data.borrow();

        // Debug logging
        tracing::debug!(
            "Sparkline render check: data_len={}, agents={}, ready={}, height={}, actual_height={}, area={:?}",
            sparkline_data.len(),
            self.active_agents.len(),
            self.agents_ready_to_start,
            sparkline_height,
            actual_sparkline_height,
            sparkline_area
        );

        if !sparkline_data.is_empty()
            && (!self.active_agents.is_empty() || self.agents_ready_to_start)
            && actual_sparkline_height > 0
        {
            // Convert data to SparklineBar with colors based on completion status
            let bars: Vec<SparklineBar> = sparkline_data
                .iter()
                .map(|(value, is_completed)| {
                    let color = if *is_completed {
                        crate::colors::success() // Green for completed
                    } else {
                        crate::colors::border() // Border color for normal activity
                    };
                    SparklineBar::from(*value).style(Style::default().fg(color))
                })
                .collect();

            // Use dynamic max based on the actual data for better visibility
            // During preparing/planning, values are small (2-3), during running they're larger (5-15)
            // For planning phase with single line, use smaller max for better visibility
            let max_value = if self.agents_ready_to_start && self.active_agents.is_empty() {
                // Planning phase - use smaller max for better visibility of 1-3 range
                sparkline_data
                    .iter()
                    .map(|(v, _)| *v)
                    .max()
                    .unwrap_or(4)
                    .max(4)
            } else {
                // Running phase - use larger max
                sparkline_data
                    .iter()
                    .map(|(v, _)| *v)
                    .max()
                    .unwrap_or(10)
                    .max(10)
            };

            let sparkline = Sparkline::default().data(bars).max(max_value); // Dynamic max for better visibility
            sparkline.render(sparkline_area, buf);
        }
    }
}

impl WidgetRef for &ChatWidget<'_> {
    fn render_ref(&self, area: Rect, buf: &mut Buffer) {
        // Top-level widget render timing
        let _perf_widget_start = if self.perf_state.enabled {
            Some(std::time::Instant::now())
        } else {
            None
        };

        // Ensure a consistent background even when individual widgets skip
        // painting unchanged regions. Without this, gutters and inter‑cell
        // spacing can show through after we reduced full clears.
        // Cost: one Block render across the frame (O(area)); acceptable and
        // fixes visual artifacts reported after redraw reductions.
        if !self.standard_terminal_mode {
            use ratatui::style::Style;
            use ratatui::widgets::Block;
            let bg = Block::default().style(Style::default().bg(crate::colors::background()));
            bg.render(area, buf);
        }

        // Remember full frame height for HUD sizing logic
        self.layout.last_frame_height.set(area.height);
        self.layout.last_frame_width.set(area.width);

        let layout_areas = self.layout_areas(area);
        let (status_bar_area, hud_area, history_area, bottom_pane_area) = if layout_areas.len() == 4
        {
            // Browser HUD is present
            (
                layout_areas[0],
                Some(layout_areas[1]),
                layout_areas[2],
                layout_areas[3],
            )
        } else {
            // No browser HUD
            (layout_areas[0], None, layout_areas[1], layout_areas[2])
        };

        // Record the effective bottom pane height for buffer-mode scrollback inserts.
        self.layout
            .last_bottom_reserved_rows
            .set(bottom_pane_area.height);

        // Render status bar and HUD only in full TUI mode
        if !self.standard_terminal_mode {
            self.render_status_bar(status_bar_area, buf);
            if let Some(hud_area) = hud_area {
                self.render_hud(hud_area, buf);
            }
        }

        // In standard-terminal mode, do not paint the history region: committed
        // content is appended to the terminal's own scrollback via
        // insert_history_lines and repainting here would overwrite it.
        if self.standard_terminal_mode {
            // Render only the bottom pane (composer or its active view) without painting
            // backgrounds to preserve the terminal's native theme.
            ratatui::widgets::WidgetRef::render_ref(&(&self.bottom_pane), bottom_pane_area, buf);
            // Scrub backgrounds in the bottom pane region so any widget-set bg becomes transparent.
            self.clear_backgrounds_in(buf, bottom_pane_area);
            return;
        }

        // Create a unified scrollable container for all chat content
        // Use consistent padding throughout
        let padding = 1u16;
        let content_area = Rect {
            x: history_area.x + padding,
            y: history_area.y,
            width: history_area.width.saturating_sub(padding * 2),
            height: history_area.height,
        };

        // Reset the full history region to the baseline theme background once per frame.
        // Individual cells only repaint when their visuals differ (e.g., assistant tint),
        // which keeps overdraw minimal while ensuring stale characters disappear.
        let base_style = Style::default()
            .bg(crate::colors::background())
            .fg(crate::colors::text());
        fill_rect(buf, history_area, Some(' '), base_style);

        // Collect all content items into a single list
        let mut all_content: Vec<&dyn HistoryCell> = Vec::new();
        for cell in self.history_cells.iter() {
            all_content.push(cell);
        }

        // Add active/streaming cell if present
        if let Some(ref cell) = self.active_exec_cell {
            all_content.push(cell as &dyn HistoryCell);
        }

        // Add live streaming content if present
        let streaming_lines = self
            .live_builder
            .display_rows()
            .into_iter()
            .map(|r| ratatui::text::Line::from(r.text))
            .collect::<Vec<_>>();

        let streaming_cell = if !streaming_lines.is_empty() {
            Some(history_cell::new_streaming_content(streaming_lines))
        } else {
            None
        };

        if let Some(ref cell) = streaming_cell {
            all_content.push(cell);
        }

        let mut assistant_layouts: Vec<Option<crate::history_cell::AssistantLayoutCache>> =
            vec![None; all_content.len()];
        let mut default_layouts: Vec<Option<Rc<CachedLayout>>> = vec![None; all_content.len()];

        // Append any queued user messages as sticky preview cells at the very
        // end so they always render at the bottom until they are dispatched.
        let mut queued_preview_cells: Vec<crate::history_cell::PlainHistoryCell> = Vec::new();
        if !self.queued_user_messages.is_empty() {
            for qm in &self.queued_user_messages {
                queued_preview_cells.push(crate::history_cell::new_queued_user_prompt(
                    qm.display_text.clone(),
                ));
            }
            for c in &queued_preview_cells {
                all_content.push(c as &dyn HistoryCell);
            }
        }

        if assistant_layouts.len() < all_content.len() {
            assistant_layouts.resize(all_content.len(), None);
        }
        if default_layouts.len() < all_content.len() {
            default_layouts.resize(all_content.len(), None);
        }

        // Calculate total content height using prefix sums; build if needed
        let spacing = 1u16; // Standard spacing between cells
        const GUTTER_WIDTH: u16 = 2; // Same as in render loop
        let cache_width = content_area.width.saturating_sub(GUTTER_WIDTH);

        // Opportunistically clear height cache if width changed
        self.history_render.handle_width_change(cache_width);

        // Perf: count a frame
        if self.perf_state.enabled {
            let mut p = self.perf_state.stats.borrow_mut();
            p.frames = p.frames.saturating_add(1);
        }

        // Detect dynamic content that requires per-frame recomputation
        let has_active_animation_early = self.history_cells.iter().any(|cell| cell.is_animating());
        let must_rebuild_prefix = !self.history_render.prefix_valid.get()
            || self.history_render.last_prefix_width.get() != content_area.width
            || self.history_render.last_prefix_count.get() != all_content.len()
            || streaming_cell.is_some()
            || has_active_animation_early;

        let total_height: u16 = if must_rebuild_prefix {
            let perf_enabled = self.perf_state.enabled;
            let total_start = if perf_enabled {
                Some(std::time::Instant::now())
            } else {
                None
            };
            let mut ps = self.history_render.prefix_sums.borrow_mut();
            ps.clear();
            ps.push(0);
            let mut acc = 0u16;
            if perf_enabled {
                let mut p = self.perf_state.stats.borrow_mut();
                p.prefix_rebuilds = p.prefix_rebuilds.saturating_add(1);
            }
            for (idx, item) in all_content.iter().enumerate() {
                let content_width = content_area.width.saturating_sub(GUTTER_WIDTH);
                let maybe_assistant = item
                    .as_any()
                    .downcast_ref::<crate::history_cell::AssistantMarkdownCell>();
                let is_streaming = item
                    .as_any()
                    .downcast_ref::<crate::history_cell::StreamingContentCell>()
                    .is_some();
                let can_use_layout_cache =
                    !item.has_custom_render() && !item.is_animating() && !is_streaming;

                let h = if let Some(assistant) = maybe_assistant {
                    if perf_enabled {
                        let mut p = self.perf_state.stats.borrow_mut();
                        p.height_misses_total = p.height_misses_total.saturating_add(1);
                    }
                    let t0 = perf_enabled.then(Instant::now);
                    let plan = assistant.ensure_layout(content_width);
                    let rows = plan.total_rows();
                    assistant_layouts[idx] = Some(plan);
                    default_layouts[idx] = None;
                    if let (true, Some(start)) = (perf_enabled, t0) {
                        let dt = start.elapsed().as_nanos();
                        let mut p = self.perf_state.stats.borrow_mut();
                        p.record_total((idx, content_width), "assistant", dt);
                    }
                    rows
                } else if can_use_layout_cache {
                    let label = perf_enabled.then(|| self.perf_label_for_item(*item));
                    let start = perf_enabled.then(Instant::now);
                    let layout_ref = self
                        .history_render
                        .ensure_layout(idx, content_width, || item.display_lines_trimmed());
                    if perf_enabled {
                        let mut p = self.perf_state.stats.borrow_mut();
                        if layout_ref.freshly_computed {
                            p.height_misses_total = p.height_misses_total.saturating_add(1);
                        } else {
                            p.height_hits_total = p.height_hits_total.saturating_add(1);
                        }
                    }
                    if layout_ref.freshly_computed
                        && let (true, Some(begin)) = (perf_enabled, start)
                    {
                        let dt = begin.elapsed().as_nanos();
                        let mut p = self.perf_state.stats.borrow_mut();
                        p.record_total(
                            (idx, content_width),
                            label.as_deref().unwrap_or("unknown"),
                            dt,
                        );
                    }
                    let height = layout_ref.line_count().min(u16::MAX as usize) as u16;
                    default_layouts[idx] = Some(layout_ref.layout());
                    height
                } else {
                    if perf_enabled {
                        let mut p = self.perf_state.stats.borrow_mut();
                        p.height_misses_total = p.height_misses_total.saturating_add(1);
                    }
                    let label = perf_enabled.then(|| self.perf_label_for_item(*item));
                    let t0 = perf_enabled.then(Instant::now);
                    let computed = item.desired_height(content_width);
                    default_layouts[idx] = None;
                    if let (true, Some(start)) = (perf_enabled, t0) {
                        let dt = start.elapsed().as_nanos();
                        let mut p = self.perf_state.stats.borrow_mut();
                        p.record_total(
                            (idx, content_width),
                            label.as_deref().unwrap_or("unknown"),
                            dt,
                        );
                    }
                    computed
                };
                acc = acc.saturating_add(h);
                let mut should_add_spacing = idx < all_content.len() - 1 && h > 0;
                if should_add_spacing {
                    let this_is_collapsed_reasoning = item
                        .as_any()
                        .downcast_ref::<crate::history_cell::CollapsibleReasoningCell>()
                        .map(|rc| rc.is_collapsed())
                        .unwrap_or(false);
                    if this_is_collapsed_reasoning && let Some(next_item) = all_content.get(idx + 1)
                    {
                        let next_is_collapsed_reasoning = next_item
                            .as_any()
                            .downcast_ref::<crate::history_cell::CollapsibleReasoningCell>()
                            .map(|rc| rc.is_collapsed())
                            .unwrap_or(false);
                        if next_is_collapsed_reasoning {
                            should_add_spacing = false;
                        }
                    }
                }
                if should_add_spacing {
                    acc = acc.saturating_add(spacing);
                }
                ps.push(acc);
            }

            let total = *ps.last().unwrap_or(&0);
            if let Some(start) = total_start
                && self.perf_state.enabled
            {
                let mut p = self.perf_state.stats.borrow_mut();
                p.ns_total_height = p.ns_total_height.saturating_add(start.elapsed().as_nanos());
            }
            // Update cache keys
            self.history_render
                .last_prefix_width
                .set(content_area.width);
            self.history_render.last_prefix_count.set(all_content.len());
            self.history_render.prefix_valid.set(true);
            total
        } else {
            // Use cached prefix sums
            *self
                .history_render
                .prefix_sums
                .borrow()
                .last()
                .unwrap_or(&0)
        };

        // Check for active animations using the trait method
        let has_active_animation = self.history_cells.iter().any(|cell| cell.is_animating());

        if has_active_animation {
            tracing::debug!("Active animation detected, scheduling next frame");
            // Lower animation cadence to reduce CPU while remaining smooth in terminals.
            // ~50ms ≈ 20 FPS is typically sufficient.
            self.app_event_tx
                .send(AppEvent::ScheduleFrameIn(std::time::Duration::from_millis(
                    50,
                )));
        }

        // Calculate scroll position and vertical alignment
        // Stabilize viewport when input area height changes while scrolled up.
        let prev_viewport_h = self.layout.last_history_viewport_height.get();
        if prev_viewport_h == 0 {
            // Initialize on first render
            self.layout
                .last_history_viewport_height
                .set(content_area.height);
        }

        let (start_y, scroll_pos) = if total_height <= content_area.height {
            // Content fits - always align to bottom so "Popular commands" stays at the bottom
            let start_y = content_area.y + content_area.height.saturating_sub(total_height);
            // Update last_max_scroll cache
            self.layout.last_max_scroll.set(0);
            (start_y, 0u16) // No scrolling needed
        } else {
            // Content overflows - calculate scroll position
            // scroll_offset is measured from the bottom (0 = bottom/newest)
            // Convert to distance from the top for rendering math.
            let max_scroll = total_height.saturating_sub(content_area.height);
            // Update cache and clamp for display only
            self.layout.last_max_scroll.set(max_scroll);
            let clamped_scroll_offset = self.layout.scroll_offset.min(max_scroll);
            let mut scroll_from_top = max_scroll.saturating_sub(clamped_scroll_offset);

            // Viewport stabilization: when user is scrolled up (offset > 0) and the
            // history viewport height changes due to the input area growing/shrinking,
            // adjust the scroll_from_top to keep the top line steady on screen.
            if clamped_scroll_offset > 0 {
                let prev_h = prev_viewport_h as i32;
                let curr_h = content_area.height as i32;
                let delta_h = prev_h - curr_h; // positive if viewport shrank
                if delta_h != 0 {
                    // Adjust in the opposite direction to keep the same top anchor
                    let sft = scroll_from_top as i32 - delta_h;
                    let sft = sft.clamp(0, max_scroll as i32) as u16;
                    scroll_from_top = sft;
                }
            }

            (content_area.y, scroll_from_top)
        };

        // Record current viewport height for the next frame
        self.layout
            .last_history_viewport_height
            .set(content_area.height);

        let _perf_hist_clear_start = if self.perf_state.enabled {
            Some(std::time::Instant::now())
        } else {
            None
        };

        // Render the scrollable content with spacing using prefix sums
        let mut screen_y = start_y; // Position on screen
        let spacing = 1u16; // Spacing between cells
        let viewport_bottom = scroll_pos.saturating_add(content_area.height);
        let ps = self.history_render.prefix_sums.borrow();
        let mut start_idx = match ps.binary_search(&scroll_pos) {
            Ok(i) => i,
            Err(i) => i.saturating_sub(1),
        };
        start_idx = start_idx.min(all_content.len());
        let mut end_idx = match ps.binary_search(&viewport_bottom) {
            Ok(i) => i,
            Err(i) => i,
        };
        // Extend end_idx by one to include the next item when the viewport cuts into spacing
        end_idx = end_idx.saturating_add(1).min(all_content.len());

        let render_loop_start = if self.perf_state.enabled {
            Some(std::time::Instant::now())
        } else {
            None
        };
        for idx in start_idx..end_idx {
            let item = all_content[idx];
            // Calculate height with reduced width due to gutter
            const GUTTER_WIDTH: u16 = 2;
            let content_width = content_area.width.saturating_sub(GUTTER_WIDTH);
            let maybe_assistant = item
                .as_any()
                .downcast_ref::<crate::history_cell::AssistantMarkdownCell>();
            let is_streaming = item
                .as_any()
                .downcast_ref::<crate::history_cell::StreamingContentCell>()
                .is_some();

            let can_use_layout_cache = !item.has_custom_render()
                && !item.is_animating()
                && !is_streaming
                && maybe_assistant.is_none();

            let mut layout_for_render: Option<Rc<CachedLayout>> = None;

            let item_height = if let Some(assistant) = maybe_assistant {
                if self.perf_state.enabled {
                    let mut p = self.perf_state.stats.borrow_mut();
                    p.height_misses_render = p.height_misses_render.saturating_add(1);
                }
                let start = self.perf_state.enabled.then(Instant::now);
                default_layouts[idx] = None;
                let plan_ref = if let Some(plan) = assistant_layouts[idx].as_ref() {
                    plan.clone()
                } else {
                    let new_plan = assistant.ensure_layout(content_width);
                    assistant_layouts[idx] = Some(new_plan);
                    assistant_layouts[idx].as_ref().unwrap().clone()
                };
                if let (true, Some(t0)) = (self.perf_state.enabled, start) {
                    let dt = t0.elapsed().as_nanos();
                    let mut p = self.perf_state.stats.borrow_mut();
                    p.record_render((idx, content_width), "assistant", dt);
                }
                plan_ref.total_rows()
            } else if can_use_layout_cache {
                let mut timing: Option<Instant> = None;
                let label = self
                    .perf_state
                    .enabled
                    .then(|| self.perf_label_for_item(item));
                let layout_ref = if let Some(existing) = default_layouts[idx].as_ref() {
                    LayoutRef {
                        data: Rc::clone(existing),
                        freshly_computed: false,
                    }
                } else {
                    timing = self.perf_state.enabled.then(Instant::now);
                    let lr = self
                        .history_render
                        .ensure_layout(idx, content_width, || item.display_lines_trimmed());
                    default_layouts[idx] = Some(lr.layout());
                    lr
                };

                if self.perf_state.enabled {
                    let mut p = self.perf_state.stats.borrow_mut();
                    if layout_ref.freshly_computed {
                        p.height_misses_render = p.height_misses_render.saturating_add(1);
                    } else {
                        p.height_hits_render = p.height_hits_render.saturating_add(1);
                    }
                }
                if layout_ref.freshly_computed
                    && let (true, Some(t0)) = (self.perf_state.enabled, timing)
                {
                    let dt = t0.elapsed().as_nanos();
                    let mut p = self.perf_state.stats.borrow_mut();
                    p.record_render(
                        (idx, content_width),
                        label.as_deref().unwrap_or("unknown"),
                        dt,
                    );
                }
                layout_for_render = Some(layout_ref.layout());
                layout_ref.line_count().min(u16::MAX as usize) as u16
            } else {
                if self.perf_state.enabled {
                    let mut p = self.perf_state.stats.borrow_mut();
                    p.height_misses_render = p.height_misses_render.saturating_add(1);
                }
                let label = self
                    .perf_state
                    .enabled
                    .then(|| self.perf_label_for_item(item));
                let start = self.perf_state.enabled.then(Instant::now);
                let computed = item.desired_height(content_width);
                if let (true, Some(t0)) = (self.perf_state.enabled, start) {
                    let dt = t0.elapsed().as_nanos();
                    let mut p = self.perf_state.stats.borrow_mut();
                    p.record_render(
                        (idx, content_width),
                        label.as_deref().unwrap_or("unknown"),
                        dt,
                    );
                }
                default_layouts[idx] = None;
                computed
            };

            let content_y = ps[idx];

            // Targeted bottom-row spacer compensation:
            // If we're at the very bottom and the last item starts just after the
            // spacer row, nudge the draw cursor down by at most that spacer (1 row).
            // Previously we used the full `gap = content_y - scroll_pos`, which could
            // be many rows and push the cursor past the viewport, making the bottom
            // appear blank. Clamp strictly to the spacer size.
            if viewport_bottom == total_height && idx == end_idx.saturating_sub(1) {
                let gap = content_y.saturating_sub(scroll_pos);
                if gap > 0 && gap <= spacing {
                    // only compensate a single spacer row
                    let remaining = (content_area.y + content_area.height).saturating_sub(screen_y);
                    let shift = spacing.min(remaining);
                    screen_y = screen_y.saturating_add(shift);
                }
            }

            let skip_top = scroll_pos.saturating_sub(content_y);

            // Stop if we've gone past the bottom of the screen
            if screen_y >= content_area.y + content_area.height {
                break;
            }

            // Calculate how much height is available for this item
            let available_height = (content_area.y + content_area.height).saturating_sub(screen_y);
            let visible_height = item_height.saturating_sub(skip_top).min(available_height);

            if visible_height > 0 {
                // Define gutter width (2 chars: symbol + space)
                const GUTTER_WIDTH: u16 = 2;

                // Split area into gutter and content
                let gutter_area = Rect {
                    x: content_area.x,
                    y: screen_y,
                    width: GUTTER_WIDTH.min(content_area.width),
                    height: visible_height,
                };

                let item_area = Rect {
                    x: content_area.x + GUTTER_WIDTH.min(content_area.width),
                    y: screen_y,
                    width: content_area.width.saturating_sub(GUTTER_WIDTH),
                    height: visible_height,
                };

                if history_cell_logging_enabled() {
                    let row_start = item_area.y;
                    let row_end = item_area.y.saturating_add(visible_height).saturating_sub(1);
                    let cache_hit = layout_for_render.is_some();
                    tracing::info!(
                        target: "codex_tui::history_cells",
                        idx,
                        kind = ?item.kind(),
                        row_start,
                        row_end,
                        height = visible_height,
                        width = item_area.width,
                        skip_rows = skip_top,
                        item_height,
                        content_y,
                        cache_hit,
                        assistant = maybe_assistant.is_some(),
                        streaming = is_streaming,
                        custom = item.has_custom_render(),
                        animating = item.is_animating(),
                        "history cell render",
                    );
                }

                // Paint gutter background. For Assistant, extend the assistant tint under the
                // gutter and also one extra column to the left (so the • has color on both sides),
                // without changing layout or symbol positions.
                let is_assistant =
                    matches!(item.kind(), crate::history_cell::HistoryCellType::Assistant);
                let gutter_bg = if is_assistant {
                    crate::colors::assistant_bg()
                } else {
                    crate::colors::background()
                };

                // Paint gutter background for assistant cells so the tinted
                // strip appears contiguous with the message body. This avoids
                // the light "hole" seen after we reduced redraws. For other
                // cell types keep the default background (already painted by
                // the frame bg fill above).
                if is_assistant && gutter_area.width > 0 && gutter_area.height > 0 {
                    let _perf_gutter_start = if self.perf_state.enabled {
                        Some(std::time::Instant::now())
                    } else {
                        None
                    };
                    let style = Style::default().bg(gutter_bg);
                    let mut tint_x = gutter_area.x;
                    let mut tint_width = gutter_area.width;
                    if content_area.x > history_area.x {
                        tint_x = content_area.x.saturating_sub(1);
                        tint_width = tint_width.saturating_add(1);
                    }
                    let tint_rect =
                        Rect::new(tint_x, gutter_area.y, tint_width, gutter_area.height);
                    fill_rect(buf, tint_rect, Some(' '), style);
                    // Also tint one column immediately to the right of the content area
                    // so the assistant block is visually bookended. This column lives in the
                    // right padding stripe; when the scrollbar is visible it will draw over
                    // the far-right edge, which is fine.
                    let right_col_x = content_area.x.saturating_add(content_area.width);
                    let history_right = history_area.x.saturating_add(history_area.width);
                    if right_col_x < history_right {
                        let right_rect = Rect::new(right_col_x, item_area.y, 1, item_area.height);
                        fill_rect(buf, right_rect, Some(' '), style);
                    }
                    if let Some(t0) = _perf_gutter_start {
                        let dt = t0.elapsed().as_nanos();
                        let mut p = self.perf_state.stats.borrow_mut();
                        p.ns_gutter_paint = p.ns_gutter_paint.saturating_add(dt);
                        // Rough accounting: area of gutter rectangle (clamped to u64)
                        let area_cells: u64 =
                            (gutter_area.width as u64).saturating_mul(gutter_area.height as u64);
                        p.cells_gutter_paint = p.cells_gutter_paint.saturating_add(area_cells);
                    }
                }

                // Render gutter symbol if present
                if let Some(symbol) = item.gutter_symbol() {
                    // Choose color based on symbol/type
                    let color = if symbol == "❯" {
                        // Executed arrow – color reflects exec state
                        if let Some(exec) = item
                            .as_any()
                            .downcast_ref::<crate::history_cell::ExecCell>()
                        {
                            match &exec.output {
                                None => crate::colors::text(), // Running...
                                // Successful runs use the theme success color so the arrow stays visible on all themes
                                Some(o) if o.exit_code == 0 => crate::colors::text(),
                                Some(_) => crate::colors::error(),
                            }
                        } else {
                            // Handle merged exec cells (multi-block "Ran") the same as single execs
                            match item.kind() {
                                crate::history_cell::HistoryCellType::Exec {
                                    kind: crate::history_cell::ExecKind::Run,
                                    status: crate::history_cell::ExecStatus::Success,
                                } => crate::colors::text(),
                                crate::history_cell::HistoryCellType::Exec {
                                    kind: crate::history_cell::ExecKind::Run,
                                    status: crate::history_cell::ExecStatus::Error,
                                } => crate::colors::error(),
                                crate::history_cell::HistoryCellType::Exec { .. } => {
                                    crate::colors::text()
                                }
                                _ => crate::colors::text(),
                            }
                        }
                    } else if symbol == "↯" {
                        // Patch/Updated arrow color – match the header text color
                        match item.kind() {
                            crate::history_cell::HistoryCellType::Patch {
                                kind: crate::history_cell::PatchKind::ApplySuccess,
                            } => crate::colors::success(),
                            crate::history_cell::HistoryCellType::Patch {
                                kind: crate::history_cell::PatchKind::ApplyBegin,
                            } => crate::colors::success(),
                            crate::history_cell::HistoryCellType::Patch {
                                kind: crate::history_cell::PatchKind::Proposed,
                            } => crate::colors::primary(),
                            crate::history_cell::HistoryCellType::Patch {
                                kind: crate::history_cell::PatchKind::ApplyFailure,
                            } => crate::colors::error(),
                            _ => crate::colors::primary(),
                        }
                    } else if matches!(symbol, "◐" | "◓" | "◑" | "◒")
                        && item
                            .as_any()
                            .downcast_ref::<crate::history_cell::RunningToolCallCell>()
                            .is_some_and(|cell| cell.has_title("Waiting"))
                    {
                        crate::colors::text_bright()
                    } else if matches!(symbol, "○" | "◔" | "◑" | "◕" | "●") {
                        if let Some(plan_cell) = item
                            .as_any()
                            .downcast_ref::<crate::history_cell::PlanUpdateCell>()
                        {
                            if plan_cell.is_complete() {
                                crate::colors::success()
                            } else {
                                crate::colors::info()
                            }
                        } else {
                            crate::colors::success()
                        }
                    } else {
                        match symbol {
                            "›" => crate::colors::text(),        // user
                            "⋮" => crate::colors::primary(),     // thinking
                            "•" => crate::colors::text_bright(), // codex/agent
                            "⚙" => crate::colors::info(),        // tool working
                            "✔" => crate::colors::success(),     // tool complete
                            "✖" => crate::colors::error(),       // error
                            "★" => crate::colors::text_bright(), // notice/popular
                            _ => crate::colors::text_dim(),
                        }
                    };

                    // Draw the symbol anchored to the top of the message (not the viewport).
                    // "Top of the message" accounts for any intentional top padding per cell type.
                    // As you scroll past that anchor, the icon scrolls away with the message.
                    if gutter_area.width >= 2 {
                        // Anchor offset counted from the very start of the item's painted area
                        // to the first line of its content that the icon should align with.
                        let anchor_offset: u16 = match item.kind() {
                            // Assistant messages render with one row of top padding so that
                            // the content visually aligns; anchor to that second row.
                            crate::history_cell::HistoryCellType::Assistant => 1,
                            _ => 0,
                        };

                        // If we've scrolled past the anchor line, don't render the icon.
                        if skip_top <= anchor_offset {
                            let rel = anchor_offset - skip_top; // rows from current viewport top
                            let symbol_y = gutter_area.y.saturating_add(rel);
                            if symbol_y < gutter_area.y.saturating_add(gutter_area.height) {
                                let symbol_style = Style::default().fg(color).bg(gutter_bg);
                                buf.set_string(gutter_area.x, symbol_y, symbol, symbol_style);
                            }
                        }
                    }
                }

                // Render only the visible window of the item using vertical skip
                let skip_rows = skip_top;

                // Log all cells being rendered
                let is_animating = item.is_animating();
                let has_custom = item.has_custom_render();

                if is_animating || has_custom {
                    tracing::debug!(
                        ">>> RENDERING ANIMATION Cell[{}]: area={:?}, skip_rows={}",
                        idx,
                        item_area,
                        skip_rows
                    );
                }

                // Render the cell content first
                let mut handled_assistant = false;
                if let Some(assistant) = item
                    .as_any()
                    .downcast_ref::<crate::history_cell::AssistantMarkdownCell>()
                {
                    let plan_ref = if let Some(plan) = assistant_layouts[idx].as_ref() {
                        plan
                    } else {
                        let new_plan = assistant.ensure_layout(content_width);
                        assistant_layouts[idx] = Some(new_plan);
                        assistant_layouts[idx].as_ref().unwrap()
                    };
                    if skip_rows >= plan_ref.total_rows() || item_area.height == 0 {
                        handled_assistant = true;
                    } else {
                        assistant.render_with_layout(plan_ref, item_area, buf, skip_rows);
                        handled_assistant = true;
                    }
                }

                if !handled_assistant {
                    if let Some(layout_rc) = layout_for_render.as_ref() {
                        self.render_cached_lines(
                            item,
                            layout_rc.as_ref(),
                            item_area,
                            buf,
                            skip_rows,
                        );
                    } else {
                        item.render_with_skip(item_area, buf, skip_rows);
                    }
                }

                // Debug: overlay order info on the spacing row below (or above if needed).
                if self.show_order_overlay
                    && let Some(Some(info)) = self.cell_order_dbg.get(idx)
                {
                    let mut text = format!("⟦{}⟧", info);
                    // Live reasoning diagnostics: append current title detection snapshot
                    if let Some(rc) = item
                        .as_any()
                        .downcast_ref::<crate::history_cell::CollapsibleReasoningCell>()
                    {
                        let snap = rc.debug_title_overlay();
                        text.push_str(" | ");
                        text.push_str(&snap);
                    }
                    let style = Style::default().fg(crate::colors::text_dim());
                    // Prefer below the item in the one-row spacing area
                    let below_y = item_area.y.saturating_add(visible_height);
                    let bottom_y = content_area.y.saturating_add(content_area.height);
                    let maxw = item_area.width as usize;
                    // Truncate safely by display width, not by bytes, to avoid
                    // panics on non-UTF-8 boundaries (e.g., emoji/CJK). Use the
                    // same width logic as our live wrap utilities.
                    let draw_text = {
                        use unicode_width::UnicodeWidthStr as _;
                        if text.width() > maxw {
                            crate::live_wrap::take_prefix_by_width(&text, maxw).0
                        } else {
                            text.clone()
                        }
                    };
                    if item_area.width > 0 {
                        if below_y < bottom_y {
                            buf.set_string(item_area.x, below_y, draw_text.clone(), style);
                        } else if item_area.y > content_area.y {
                            // Fall back to above the item if no space below
                            let above_y = item_area.y.saturating_sub(1);
                            buf.set_string(item_area.x, above_y, draw_text.clone(), style);
                        }
                    }
                }
                screen_y += visible_height;
            }

            // Add spacing only if something was actually rendered for this item.
            // Prevent a stray blank when zero-height, and suppress spacing between
            // consecutive collapsed reasoning titles so they appear as a tight list.
            let mut should_add_spacing = idx < all_content.len() - 1 && visible_height > 0;
            if should_add_spacing {
                // Special-case: two adjacent collapsed reasoning cells → no spacer.
                let this_is_collapsed_reasoning = item
                    .as_any()
                    .downcast_ref::<crate::history_cell::CollapsibleReasoningCell>()
                    .map(|rc| rc.is_collapsed())
                    .unwrap_or(false);
                if this_is_collapsed_reasoning && let Some(next_item) = all_content.get(idx + 1) {
                    let next_is_collapsed_reasoning = next_item
                        .as_any()
                        .downcast_ref::<crate::history_cell::CollapsibleReasoningCell>()
                        .map(|rc| rc.is_collapsed())
                        .unwrap_or(false);
                    if next_is_collapsed_reasoning {
                        should_add_spacing = false;
                    }
                }
            }
            if should_add_spacing && screen_y < content_area.y + content_area.height {
                screen_y +=
                    spacing.min((content_area.y + content_area.height).saturating_sub(screen_y));
            }
        }
        if let Some(start) = render_loop_start
            && self.perf_state.enabled
        {
            let mut p = self.perf_state.stats.borrow_mut();
            p.ns_render_loop = p.ns_render_loop.saturating_add(start.elapsed().as_nanos());
        }

        // Clear any bottom gap inside the content area that wasn’t covered by items
        if screen_y < content_area.y + content_area.height {
            let _perf_hist_clear2 = if self.perf_state.enabled {
                Some(std::time::Instant::now())
            } else {
                None
            };
            let gap_height = (content_area.y + content_area.height).saturating_sub(screen_y);
            if gap_height > 0 {
                let gap_rect = Rect::new(content_area.x, screen_y, content_area.width, gap_height);
                fill_rect(buf, gap_rect, Some(' '), base_style);
            }
            if let Some(t0) = _perf_hist_clear2 {
                let dt = t0.elapsed().as_nanos();
                let mut p = self.perf_state.stats.borrow_mut();
                p.ns_history_clear = p.ns_history_clear.saturating_add(dt);
                let cells = (content_area.width as u64)
                    * ((content_area.y + content_area.height - screen_y) as u64);
                p.cells_history_clear = p.cells_history_clear.saturating_add(cells);
            }
        }

        // Render vertical scrollbar when content is scrollable and currently visible
        // Auto-hide after a short delay to avoid copying it along with text.
        let now = std::time::Instant::now();
        let show_scrollbar = total_height > content_area.height
            && self
                .layout
                .scrollbar_visible_until
                .get()
                .map(|t| now < t)
                .unwrap_or(false);
        if show_scrollbar {
            let mut sb_state = self.layout.vertical_scrollbar_state.borrow_mut();
            // Scrollbar expects number of scroll positions, not total rows.
            // For a viewport of H rows and content of N rows, there are
            // max_scroll = N - H positions; valid positions = [0, max_scroll].
            let max_scroll = total_height.saturating_sub(content_area.height);
            let scroll_positions = max_scroll.saturating_add(1).max(1) as usize;
            let pos = scroll_pos.min(max_scroll) as usize;
            *sb_state = sb_state.content_length(scroll_positions).position(pos);
            // Theme-aware scrollbar styling (line + block)
            // Track: thin line using border color; Thumb: block using border_focused.
            let theme = crate::theme::current_theme();
            let sb = Scrollbar::new(ScrollbarOrientation::VerticalRight)
                .symbols(scrollbar_symbols::VERTICAL)
                .begin_symbol(None)
                .end_symbol(None)
                .track_symbol(Some("│"))
                .track_style(
                    Style::default()
                        .fg(crate::colors::border())
                        .bg(crate::colors::background()),
                )
                .thumb_symbol("█")
                .thumb_style(
                    Style::default()
                        .fg(theme.border_focused)
                        .bg(crate::colors::background()),
                );
            // To avoid a small jump at the bottom due to spacer toggling,
            // render the scrollbar in a slightly shorter area (reserve 1 row).
            let sb_area = Rect {
                x: history_area.x,
                y: history_area.y,
                width: history_area.width,
                height: history_area.height.saturating_sub(1),
            };
            StatefulWidget::render(sb, sb_area, buf, &mut sb_state);
        }

        if self.terminal.overlay().is_some() {
            let bg_style = Style::default().bg(crate::colors::background());
            fill_rect(buf, bottom_pane_area, Some(' '), bg_style);
        } else if self.agents_terminal.active {
            let bg_style = Style::default().bg(crate::colors::background());
            fill_rect(buf, bottom_pane_area, Some(' '), bg_style);
        } else {
            // Render the bottom pane directly without a border for now
            // The composer has its own layout with hints at the bottom
            (&self.bottom_pane).render(bottom_pane_area, buf);
        }

        if let Some(overlay) = self.terminal.overlay() {
            let scrim_style = Style::default()
                .bg(crate::colors::overlay_scrim())
                .fg(crate::colors::text_dim());
            fill_rect(buf, area, None, scrim_style);

            let padding = 1u16;
            let footer_reserved = 1.min(bottom_pane_area.height);
            let overlay_bottom =
                (bottom_pane_area.y + bottom_pane_area.height).saturating_sub(footer_reserved);
            let overlay_height = overlay_bottom
                .saturating_sub(history_area.y)
                .max(1)
                .min(area.height);
            let window_area = Rect {
                x: history_area.x + padding,
                y: history_area.y,
                width: history_area.width.saturating_sub(padding * 2),
                height: overlay_height,
            };
            Clear.render(window_area, buf);

            let block = Block::default()
                .borders(Borders::ALL)
                .title(ratatui::text::Line::from(vec![
                    ratatui::text::Span::styled(
                        format!(" Terminal - {} ", overlay.title),
                        Style::default().fg(crate::colors::text()),
                    ),
                ]))
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

            let content = inner.inner(ratatui::layout::Margin::new(1, 0));
            if content.height == 0 || content.width == 0 {
                self.terminal.last_visible_rows.set(0);
                self.terminal.last_visible_cols.set(0);
            } else {
                let header_height = 1.min(content.height);
                let footer_height = if content.height >= 2 { 2 } else { 0 };

                let header_area = Rect {
                    x: content.x,
                    y: content.y,
                    width: content.width,
                    height: header_height,
                };
                let footer_area = if footer_height > 0 {
                    Rect {
                        x: content.x,
                        y: content
                            .y
                            .saturating_add(content.height.saturating_sub(footer_height)),
                        width: content.width,
                        height: footer_height,
                    }
                } else {
                    header_area
                };

                if header_height > 0 {
                    fill_rect(buf, header_area, Some(' '), inner_bg);
                    let width_limit = header_area.width as usize;
                    let mut header_spans: Vec<ratatui::text::Span<'static>> = Vec::new();
                    let mut consumed_width: usize = 0;

                    if overlay.running {
                        let now_ms = std::time::SystemTime::now()
                            .duration_since(std::time::UNIX_EPOCH)
                            .unwrap_or_default()
                            .as_millis();
                        let frame = crate::spinner::frame_at_time(
                            crate::spinner::current_spinner(),
                            now_ms,
                        );
                        if !frame.is_empty() {
                            consumed_width += frame.chars().count();
                            header_spans.push(ratatui::text::Span::styled(
                                frame,
                                Style::default().fg(crate::colors::spinner()),
                            ));
                            header_spans.push(ratatui::text::Span::raw(" "));
                            consumed_width = consumed_width.saturating_add(1);
                        }

                        let status_text = overlay
                            .start_time
                            .map(|start| format!("Running… ({})", format_duration(start.elapsed())))
                            .unwrap_or_else(|| "Running…".to_string());
                        consumed_width = consumed_width
                            .saturating_add(UnicodeWidthStr::width(status_text.as_str()));
                        header_spans.push(ratatui::text::Span::styled(
                            status_text,
                            Style::default().fg(crate::colors::text_dim()),
                        ));

                        let interval = crate::spinner::current_spinner().interval_ms.max(50);
                        self.app_event_tx
                            .send(AppEvent::ScheduleFrameIn(Duration::from_millis(interval)));
                    } else {
                        let (icon, color, status_text) = match overlay.exit_code {
                            Some(0) => (
                                "✔",
                                crate::colors::success(),
                                overlay
                                    .duration
                                    .map(|d| format!("Completed in {}", format_duration(d)))
                                    .unwrap_or_else(|| "Completed".to_string()),
                            ),
                            Some(code) => (
                                "✖",
                                crate::colors::error(),
                                overlay
                                    .duration
                                    .map(|d| format!("Exit {code} in {}", format_duration(d)))
                                    .unwrap_or_else(|| format!("Exit {code}")),
                            ),
                            None => (
                                "⚠",
                                crate::colors::warning(),
                                overlay
                                    .duration
                                    .map(|d| format!("Stopped after {}", format_duration(d)))
                                    .unwrap_or_else(|| "Stopped".to_string()),
                            ),
                        };

                        header_spans.push(ratatui::text::Span::styled(
                            format!("{icon} "),
                            Style::default().fg(color),
                        ));
                        consumed_width = consumed_width.saturating_add(icon.chars().count() + 1);

                        consumed_width = consumed_width
                            .saturating_add(UnicodeWidthStr::width(status_text.as_str()));
                        header_spans.push(ratatui::text::Span::styled(
                            status_text,
                            Style::default().fg(crate::colors::text_dim()),
                        ));
                    }

                    if !overlay.command_display.is_empty() && width_limit > consumed_width + 5 {
                        let remaining = width_limit.saturating_sub(consumed_width + 5);
                        if remaining > 0 {
                            let truncated = ChatWidget::truncate_with_ellipsis(
                                &overlay.command_display,
                                remaining,
                            );
                            if !truncated.is_empty() {
                                header_spans.push(ratatui::text::Span::styled(
                                    "  •  ",
                                    Style::default().fg(crate::colors::text_dim()),
                                ));
                                header_spans.push(ratatui::text::Span::styled(
                                    truncated,
                                    Style::default().fg(crate::colors::text()),
                                ));
                            }
                        }
                    }

                    let header_line = ratatui::text::Line::from(header_spans);
                    Paragraph::new(RtText::from(vec![header_line]))
                        .wrap(ratatui::widgets::Wrap { trim: true })
                        .render(header_area, buf);
                }

                let mut body_space = content
                    .height
                    .saturating_sub(header_height.saturating_add(footer_height));
                let body_top = header_area.y.saturating_add(header_area.height);
                let mut bottom_cursor = body_top.saturating_add(body_space);

                let mut pending_visible = false;
                let mut pending_box: Option<(Rect, Vec<RtLine<'static>>)> = None;
                if let Some(pending) = overlay.pending_command.as_ref()
                    && let Some((pending_lines, pending_height)) =
                        command_render::pending_command_box_lines(pending, content.width)
                    && pending_height <= body_space
                    && pending_height > 0
                {
                    bottom_cursor = bottom_cursor.saturating_sub(pending_height);
                    let pending_area = Rect {
                        x: content.x,
                        y: bottom_cursor,
                        width: content.width,
                        height: pending_height,
                    };
                    body_space = body_space.saturating_sub(pending_height);
                    pending_box = Some((pending_area, pending_lines));
                    pending_visible = true;
                }

                let body_area = Rect {
                    x: content.x,
                    y: body_top,
                    width: content.width,
                    height: body_space,
                };

                // Body content
                let rows = body_area.height;
                let cols = body_area.width;
                let prev_rows = self.terminal.last_visible_rows.replace(rows);
                let prev_cols = self.terminal.last_visible_cols.replace(cols);
                if rows > 0 && cols > 0 && (prev_rows != rows || prev_cols != cols) {
                    self.app_event_tx.send(AppEvent::TerminalResize {
                        id: overlay.id,
                        rows,
                        cols,
                    });
                }

                if rows > 0 && cols > 0 {
                    let mut rendered_rows: Vec<RtLine<'static>> = Vec::new();
                    if overlay.truncated {
                        rendered_rows.push(ratatui::text::Line::from(vec![
                            ratatui::text::Span::styled(
                                "… output truncated (showing last 10,000 lines)",
                                Style::default().fg(crate::colors::text_dim()),
                            ),
                        ]));
                    }
                    rendered_rows.extend(overlay.lines.iter().cloned());
                    let total = rendered_rows.len();
                    let visible = rows as usize;
                    if visible > 0 {
                        let max_scroll = total.saturating_sub(visible);
                        let scroll = (overlay.scroll as usize).min(max_scroll);
                        let end = (scroll + visible).min(total);
                        let window = rendered_rows.get(scroll..end).unwrap_or(&[]);
                        Paragraph::new(RtText::from(window.to_vec()))
                            .wrap(ratatui::widgets::Wrap { trim: false })
                            .render(body_area, buf);
                    }
                }

                if let Some((pending_area, pending_lines)) = pending_box {
                    command_render::render_text_box(
                        pending_area,
                        " Command ",
                        crate::colors::function(),
                        pending_lines,
                        buf,
                    );
                }

                // Footer hints
                let mut footer_spans = vec![
                    ratatui::text::Span::styled(
                        "↑↓",
                        Style::default().fg(crate::colors::function()),
                    ),
                    ratatui::text::Span::styled(
                        " Scroll  ",
                        Style::default().fg(crate::colors::text_dim()),
                    ),
                    ratatui::text::Span::styled("Esc", Style::default().fg(crate::colors::error())),
                    ratatui::text::Span::styled(
                        if overlay.running {
                            " Cancel  "
                        } else {
                            " Close  "
                        },
                        Style::default().fg(crate::colors::text_dim()),
                    ),
                ];
                if overlay.running {
                    footer_spans.push(ratatui::text::Span::styled(
                        "Ctrl+C",
                        Style::default().fg(crate::colors::warning()),
                    ));
                    footer_spans.push(ratatui::text::Span::styled(
                        " Cancel",
                        Style::default().fg(crate::colors::text_dim()),
                    ));
                } else if pending_visible {
                    footer_spans.push(ratatui::text::Span::styled(
                        "Enter",
                        Style::default().fg(crate::colors::primary()),
                    ));
                    footer_spans.push(ratatui::text::Span::styled(
                        " Run",
                        Style::default().fg(crate::colors::text_dim()),
                    ));
                }
                if footer_height > 1 {
                    let spacer_area = Rect {
                        x: footer_area.x,
                        y: footer_area.y,
                        width: footer_area.width,
                        height: footer_area.height.saturating_sub(1),
                    };
                    fill_rect(buf, spacer_area, Some(' '), inner_bg);
                }

                let instructions_area = Rect {
                    x: footer_area.x,
                    y: footer_area
                        .y
                        .saturating_add(footer_area.height.saturating_sub(1)),
                    width: footer_area.width,
                    height: 1,
                };

                Paragraph::new(RtText::from(vec![ratatui::text::Line::from(footer_spans)]))
                    .wrap(ratatui::widgets::Wrap { trim: true })
                    .alignment(ratatui::layout::Alignment::Left)
                    .render(instructions_area, buf);
            }
        }

        if self.terminal.overlay().is_none() && self.agents_terminal.active {
            self.render_agents_terminal_overlay(area, history_area, bottom_pane_area, buf);
        }

        // Terminal overlay takes precedence over other overlays

        // Welcome animation is kept as a normal cell in history; no overlay.

        // The welcome animation is no longer rendered as an overlay.

        if self.terminal.overlay().is_none() && !self.agents_terminal.active {
            if self.limits.overlay.is_some() {
                self.render_limits_overlay(area, history_area, buf);
            } else if self.pm.overlay.is_some() {
                self.render_pm_overlay(area, history_area, buf);
            } else if self.pro.overlay_visible {
                self.render_pro_overlay(area, history_area, buf);
            } else if let Some(overlay) = &self.diffs.overlay {
                // Global scrim: dim the whole background to draw focus to the viewer
                // We intentionally do this across the entire widget area rather than just the
                // history area so the viewer stands out even with browser HUD or status bars.
                let scrim_bg = Style::default()
                    .bg(crate::colors::overlay_scrim())
                    .fg(crate::colors::text_dim());
                let _perf_scrim_start = if self.perf_state.enabled {
                    Some(std::time::Instant::now())
                } else {
                    None
                };
                fill_rect(buf, area, None, scrim_bg);
                if let Some(t0) = _perf_scrim_start {
                    let dt = t0.elapsed().as_nanos();
                    let mut p = self.perf_state.stats.borrow_mut();
                    p.ns_overlay_scrim = p.ns_overlay_scrim.saturating_add(dt);
                    let cells = (area.width as u64) * (area.height as u64);
                    p.cells_overlay_scrim = p.cells_overlay_scrim.saturating_add(cells);
                }
                // Match the horizontal padding used by status bar and input
                let padding = 1u16;
                let area = Rect {
                    x: history_area.x + padding,
                    y: history_area.y,
                    width: history_area.width.saturating_sub(padding * 2),
                    height: history_area.height,
                };

                // Clear and repaint the overlay area with theme scrim background
                Clear.render(area, buf);
                let bg_style = Style::default().bg(crate::colors::overlay_scrim());
                let _perf_overlay_area_bg_start = if self.perf_state.enabled {
                    Some(std::time::Instant::now())
                } else {
                    None
                };
                fill_rect(buf, area, None, bg_style);
                if let Some(t0) = _perf_overlay_area_bg_start {
                    let dt = t0.elapsed().as_nanos();
                    let mut p = self.perf_state.stats.borrow_mut();
                    p.ns_overlay_body_bg = p.ns_overlay_body_bg.saturating_add(dt);
                    let cells = (area.width as u64) * (area.height as u64);
                    p.cells_overlay_body_bg = p.cells_overlay_body_bg.saturating_add(cells);
                }

                // Build a styled title: keys/icons in normal text color; descriptors and dividers dim
                let t_dim = Style::default().fg(crate::colors::text_dim());
                let t_fg = Style::default().fg(crate::colors::text());
                let has_tabs = overlay.tabs.len() > 1;
                let mut title_spans: Vec<ratatui::text::Span<'static>> = vec![
                    ratatui::text::Span::styled(" ", t_dim),
                    ratatui::text::Span::styled("Diff viewer", t_fg),
                ];
                if has_tabs {
                    title_spans.extend_from_slice(&[
                        ratatui::text::Span::styled(" ——— ", t_dim),
                        ratatui::text::Span::styled("◂ ▸", t_fg),
                        ratatui::text::Span::styled(" change tabs ", t_dim),
                    ]);
                }
                title_spans.extend_from_slice(&[
                    ratatui::text::Span::styled("——— ", t_dim),
                    ratatui::text::Span::styled("e", t_fg),
                    ratatui::text::Span::styled(" explain ", t_dim),
                    ratatui::text::Span::styled("——— ", t_dim),
                    ratatui::text::Span::styled("u", t_fg),
                    ratatui::text::Span::styled(" undo ", t_dim),
                    ratatui::text::Span::styled("——— ", t_dim),
                    ratatui::text::Span::styled("Esc", t_fg),
                    ratatui::text::Span::styled(" close ", t_dim),
                ]);
                let block = Block::default()
                    .borders(Borders::ALL)
                    .title(ratatui::text::Line::from(title_spans))
                    // Use normal background for the window itself so it contrasts against the
                    // dimmed scrim behind
                    .style(Style::default().bg(crate::colors::background()))
                    .border_style(
                        Style::default()
                            .fg(crate::colors::border())
                            .bg(crate::colors::background()),
                    );
                let inner = block.inner(area);
                block.render(area, buf);

                // Paint inner content background as the normal theme background
                let inner_bg = Style::default().bg(crate::colors::background());
                let _perf_overlay_inner_bg_start = if self.perf_state.enabled {
                    Some(std::time::Instant::now())
                } else {
                    None
                };
                for y in inner.y..inner.y + inner.height {
                    for x in inner.x..inner.x + inner.width {
                        buf[(x, y)].set_style(inner_bg);
                    }
                }
                if let Some(t0) = _perf_overlay_inner_bg_start {
                    let dt = t0.elapsed().as_nanos();
                    let mut p = self.perf_state.stats.borrow_mut();
                    p.ns_overlay_body_bg = p.ns_overlay_body_bg.saturating_add(dt);
                    let cells = (inner.width as u64) * (inner.height as u64);
                    p.cells_overlay_body_bg = p.cells_overlay_body_bg.saturating_add(cells);
                }

                // Split into header tabs and body/footer
                // Add one cell padding around the entire inside of the window
                let padded_inner = inner.inner(ratatui::layout::Margin::new(1, 1));
                let [tabs_area, body_area] = if has_tabs {
                    Layout::vertical([Constraint::Length(2), Constraint::Fill(1)])
                        .areas(padded_inner)
                } else {
                    // Keep a small header row to show file path and counts
                    let [t, b] = Layout::vertical([Constraint::Length(2), Constraint::Fill(1)])
                        .areas(padded_inner);
                    [t, b]
                };

                // Render tabs only if we have more than one file
                if has_tabs {
                    let labels: Vec<String> = overlay
                        .tabs
                        .iter()
                        .map(|(t, _)| format!("  {}  ", t))
                        .collect();
                    let mut constraints: Vec<Constraint> = Vec::new();
                    let mut total: u16 = 0;
                    for label in &labels {
                        let w = (label.chars().count() as u16)
                            .min(tabs_area.width.saturating_sub(total));
                        constraints.push(Constraint::Length(w));
                        total = total.saturating_add(w);
                        if total >= tabs_area.width.saturating_sub(4) {
                            break;
                        }
                    }
                    constraints.push(Constraint::Fill(1));
                    let chunks = Layout::horizontal(constraints).split(tabs_area);
                    // Draw a light bottom border across the entire tabs strip
                    let tabs_bottom_rule = Block::default()
                        .borders(Borders::BOTTOM)
                        .border_style(Style::default().fg(crate::colors::border()));
                    tabs_bottom_rule.render(tabs_area, buf);
                    for i in 0..labels.len() {
                        // last chunk is filler; guard below
                        if i >= chunks.len().saturating_sub(1) {
                            break;
                        }
                        let rect = chunks[i];
                        if rect.width == 0 {
                            continue;
                        }
                        let selected = i == overlay.selected;

                        // Both selected and unselected tabs use the normal background
                        let tab_bg = crate::colors::background();
                        let bg_style = Style::default().bg(tab_bg);
                        for y in rect.y..rect.y + rect.height {
                            for x in rect.x..rect.x + rect.width {
                                buf[(x, y)].set_style(bg_style);
                            }
                        }

                        // Render label at the top line, with padding
                        let label_rect = Rect {
                            x: rect.x + 1,
                            y: rect.y,
                            width: rect.width.saturating_sub(2),
                            height: 1,
                        };
                        let label_style = if selected {
                            Style::default()
                                .fg(crate::colors::text())
                                .add_modifier(Modifier::BOLD)
                        } else {
                            Style::default().fg(crate::colors::text_dim())
                        };
                        let line = ratatui::text::Line::from(ratatui::text::Span::styled(
                            labels[i].clone(),
                            label_style,
                        ));
                        Paragraph::new(RtText::from(vec![line]))
                            .wrap(ratatui::widgets::Wrap { trim: true })
                            .render(label_rect, buf);
                        // Selected tab: thin underline using text_bright under the label width
                        if selected {
                            let label_len = labels[i].chars().count() as u16;
                            let accent_w = label_len.min(rect.width.saturating_sub(2)).max(1);
                            let accent_rect = Rect {
                                x: label_rect.x,
                                y: rect.y + rect.height.saturating_sub(1),
                                width: accent_w,
                                height: 1,
                            };
                            let underline = Block::default()
                                .borders(Borders::BOTTOM)
                                .border_style(Style::default().fg(crate::colors::text_bright()));
                            underline.render(accent_rect, buf);
                        }
                    }
                } else {
                    // Single-file header: show full path with (+adds -dels)
                    if let Some((label, _)) = overlay.tabs.get(overlay.selected) {
                        let header_line = ratatui::text::Line::from(ratatui::text::Span::styled(
                            label.clone(),
                            Style::default()
                                .fg(crate::colors::text())
                                .add_modifier(Modifier::BOLD),
                        ));
                        let para = Paragraph::new(RtText::from(vec![header_line]))
                            .wrap(ratatui::widgets::Wrap { trim: true });
                        ratatui::widgets::Widget::render(para, tabs_area, buf);
                    }
                }

                // Render selected tab with vertical scroll and highlight current diff block
                if let Some((_, blocks)) = overlay.tabs.get(overlay.selected) {
                    // Flatten blocks into lines and record block start indices
                    let mut all_lines: Vec<ratatui::text::Line<'static>> = Vec::new();
                    let mut block_starts: Vec<(usize, usize)> = Vec::new(); // (start_index, len)
                    for b in blocks {
                        let start = all_lines.len();
                        block_starts.push((start, b.lines.len()));
                        all_lines.extend(b.lines.clone());
                    }

                    let raw_skip = overlay
                        .scroll_offsets
                        .get(overlay.selected)
                        .copied()
                        .unwrap_or(0) as usize;
                    let visible_rows = body_area.height as usize;
                    // Cache visible rows so key handler can clamp
                    self.diffs.body_visible_rows.set(body_area.height);
                    let max_off = all_lines.len().saturating_sub(visible_rows.max(1));
                    let skip = raw_skip.min(max_off);
                    let body_inner = body_area;
                    let visible_rows = body_inner.height as usize;

                    // Collect visible slice
                    let end = (skip + visible_rows).min(all_lines.len());
                    let visible = if skip < all_lines.len() {
                        &all_lines[skip..end]
                    } else {
                        &[]
                    };
                    // Fill body background with a slightly lighter paper-like background
                    let bg = crate::colors::background();
                    #[allow(clippy::disallowed_methods)] // Color blending requires RGB manipulation
                    let paper_color = match bg {
                        ratatui::style::Color::Rgb(r, g, b) => {
                            let alpha = 0.06f32; // subtle lightening toward white
                            let nr = ((r as f32) * (1.0 - alpha) + 255.0 * alpha).round() as u8;
                            let ng = ((g as f32) * (1.0 - alpha) + 255.0 * alpha).round() as u8;
                            let nb = ((b as f32) * (1.0 - alpha) + 255.0 * alpha).round() as u8;
                            ratatui::style::Color::Rgb(nr, ng, nb)
                        }
                        _ => bg,
                    };
                    let body_bg = Style::default().bg(paper_color);
                    let _perf_overlay_body_bg2 = if self.perf_state.enabled {
                        Some(std::time::Instant::now())
                    } else {
                        None
                    };
                    for y in body_inner.y..body_inner.y + body_inner.height {
                        for x in body_inner.x..body_inner.x + body_inner.width {
                            buf[(x, y)].set_style(body_bg);
                        }
                    }
                    if let Some(t0) = _perf_overlay_body_bg2 {
                        let dt = t0.elapsed().as_nanos();
                        let mut p = self.perf_state.stats.borrow_mut();
                        p.ns_overlay_body_bg = p.ns_overlay_body_bg.saturating_add(dt);
                        let cells = (body_inner.width as u64) * (body_inner.height as u64);
                        p.cells_overlay_body_bg = p.cells_overlay_body_bg.saturating_add(cells);
                    }
                    let paragraph = Paragraph::new(RtText::from(visible.to_vec()))
                        .wrap(ratatui::widgets::Wrap { trim: false });
                    ratatui::widgets::Widget::render(paragraph, body_inner, buf);

                    // No explicit current-block highlight for a cleaner look

                    // Render confirmation dialog if active
                    if self.diffs.confirm.is_some() {
                        // Centered small box
                        let w = (body_inner.width as i16 - 10).max(20) as u16;
                        let h = 5u16;
                        let x = body_inner.x + (body_inner.width.saturating_sub(w)) / 2;
                        let y = body_inner.y + (body_inner.height.saturating_sub(h)) / 2;
                        let dialog = Rect {
                            x,
                            y,
                            width: w,
                            height: h,
                        };
                        Clear.render(dialog, buf);
                        let dlg_block = Block::default()
                            .borders(Borders::ALL)
                            .title("Confirm Undo")
                            .style(
                                Style::default()
                                    .bg(crate::colors::background())
                                    .fg(crate::colors::text()),
                            )
                            .border_style(Style::default().fg(crate::colors::border()));
                        let dlg_inner = dlg_block.inner(dialog);
                        dlg_block.render(dialog, buf);
                        // Fill dialog inner area with theme background for consistent look
                        let dlg_bg = Style::default().bg(crate::colors::background());
                        for y in dlg_inner.y..dlg_inner.y + dlg_inner.height {
                            for x in dlg_inner.x..dlg_inner.x + dlg_inner.width {
                                buf[(x, y)].set_style(dlg_bg);
                            }
                        }
                        let lines = vec![
                            ratatui::text::Line::from("Are you sure you want to undo this diff?"),
                            ratatui::text::Line::from(
                                "Press Enter to confirm • Esc to cancel".to_string().dim(),
                            ),
                        ];
                        let para = Paragraph::new(RtText::from(lines))
                            .style(
                                Style::default()
                                    .bg(crate::colors::background())
                                    .fg(crate::colors::text()),
                            )
                            .wrap(ratatui::widgets::Wrap { trim: true });
                        ratatui::widgets::Widget::render(para, dlg_inner, buf);
                    }
                }
            }

            // Render help overlay (covering the history area) if active
            if let Some(overlay) = &self.help.overlay {
                // Global scrim across widget
                let scrim_bg = Style::default()
                    .bg(crate::colors::overlay_scrim())
                    .fg(crate::colors::text_dim());
                for y in area.y..area.y + area.height {
                    for x in area.x..area.x + area.width {
                        buf[(x, y)].set_style(scrim_bg);
                    }
                }
                let padding = 1u16;
                let window_area = Rect {
                    x: history_area.x + padding,
                    y: history_area.y,
                    width: history_area.width.saturating_sub(padding * 2),
                    height: history_area.height,
                };
                Clear.render(window_area, buf);
                let block = Block::default()
                    .borders(Borders::ALL)
                    .title(ratatui::text::Line::from(vec![
                        ratatui::text::Span::styled(
                            " ",
                            Style::default().fg(crate::colors::text_dim()),
                        ),
                        ratatui::text::Span::styled(
                            "Help",
                            Style::default().fg(crate::colors::text()),
                        ),
                        ratatui::text::Span::styled(
                            " ——— ",
                            Style::default().fg(crate::colors::text_dim()),
                        ),
                        ratatui::text::Span::styled(
                            "Esc",
                            Style::default().fg(crate::colors::text()),
                        ),
                        ratatui::text::Span::styled(
                            " close ",
                            Style::default().fg(crate::colors::text_dim()),
                        ),
                    ]))
                    .style(Style::default().bg(crate::colors::background()))
                    .border_style(
                        Style::default()
                            .fg(crate::colors::border())
                            .bg(crate::colors::background()),
                    );
                let inner = block.inner(window_area);
                block.render(window_area, buf);

                // Paint inner bg
                let inner_bg = Style::default().bg(crate::colors::background());
                for y in inner.y..inner.y + inner.height {
                    for x in inner.x..inner.x + inner.width {
                        buf[(x, y)].set_style(inner_bg);
                    }
                }

                // Body area with one cell padding
                let body = inner.inner(ratatui::layout::Margin::new(1, 1));

                // Compute visible slice
                let visible_rows = body.height as usize;
                self.help.body_visible_rows.set(body.height);
                let max_off = overlay.lines.len().saturating_sub(visible_rows.max(1));
                let skip = (overlay.scroll as usize).min(max_off);
                let end = (skip + visible_rows).min(overlay.lines.len());
                let visible = if skip < overlay.lines.len() {
                    &overlay.lines[skip..end]
                } else {
                    &[]
                };
                let paragraph = Paragraph::new(RtText::from(visible.to_vec()))
                    .wrap(ratatui::widgets::Wrap { trim: false });
                ratatui::widgets::Widget::render(paragraph, body, buf);
            }
        }
        // Finalize widget render timing
        if let Some(t0) = _perf_widget_start {
            let dt = t0.elapsed().as_nanos();
            let mut p = self.perf_state.stats.borrow_mut();
            p.ns_widget_render_total = p.ns_widget_render_total.saturating_add(dt);
        }
    }
}

// --- Additional rendering support extracted from first impl block ---

impl ChatWidget<'_> {
    pub(crate) fn token_usage(&self) -> &TokenUsage {
        &self.total_token_usage
    }

    pub(crate) fn clear_token_usage(&mut self) {
        self.total_token_usage = TokenUsage::default();
        self.rate_limit_snapshot = None;
        self.rate_limit_warnings.reset();
        self.rate_limit_last_fetch_at = None;
        self.bottom_pane.set_token_usage(
            self.total_token_usage.clone(),
            self.last_token_usage.clone(),
            self.config.model_context_window,
        );
    }

    // MAINT-11 Phase 8: export_transcript_lines_for_buffer, render_lines_for_terminal
    // moved to session_handlers.rs

    /// Desired bottom pane height (in rows) for a given terminal width.
    pub(crate) fn desired_bottom_height(&self, width: u16) -> u16 {
        self.bottom_pane.desired_height(width)
    }

    // (Removed) Legacy in-place reset method. The /new command now creates a fresh
    // ChatWidget (new core session) to ensure the agent context is fully reset.

    pub fn cursor_pos(&self, area: Rect) -> Option<(u16, u16)> {
        // Hide the terminal cursor whenever a top‑level overlay is active so the
        // caret does not show inside the input while a modal (help/diff) is open.
        if self.diffs.overlay.is_some()
            || self.help.overlay.is_some()
            || self.terminal.overlay().is_some()
            || self.agents_terminal.active
        {
            return None;
        }
        let layout_areas = self.layout_areas(area);
        let bottom_pane_area = if layout_areas.len() == 4 {
            layout_areas[3]
        } else {
            layout_areas[2]
        };
        self.bottom_pane.cursor_pos(bottom_pane_area)
    }

    pub(super) fn measured_font_size(&self) -> (u16, u16) {
        *self.cached_cell_size.get_or_init(|| {
            let size = self.terminal_info.font_size;

            // HACK: On macOS Retina displays, terminals often report physical pixels
            // but ratatui-image expects logical pixels. If we detect suspiciously
            // large cell sizes (likely 2x scaled), divide by 2.
            #[cfg(target_os = "macos")]
            {
                if size.0 >= 14 && size.1 >= 28 {
                    // Likely Retina display reporting physical pixels
                    tracing::info!(
                        "Detected likely Retina display, adjusting cell size from {:?} to {:?}",
                        size,
                        (size.0 / 2, size.1 / 2)
                    );
                    return (size.0 / 2, size.1 / 2);
                }
            }

            size
        })
    }

    fn get_git_branch(&self) -> Option<String> {
        use std::fs;
        use std::path::Path;

        let head_path = self.config.cwd.join(".git/HEAD");
        let mut cache = self.git_branch_cache.borrow_mut();
        let now = Instant::now();

        let needs_refresh = match cache.last_refresh {
            Some(last) => now.duration_since(last) >= Duration::from_millis(500),
            None => true,
        };

        if needs_refresh {
            let modified = fs::metadata(&head_path)
                .and_then(|meta| meta.modified())
                .ok();

            let metadata_changed =
                cache.last_head_mtime != modified || cache.last_refresh.is_none();

            if metadata_changed {
                cache.value = fs::read_to_string(&head_path)
                    .ok()
                    .and_then(|head_contents| {
                        let head = head_contents.trim();

                        if let Some(rest) = head.strip_prefix("ref: ") {
                            return Path::new(rest)
                                .file_name()
                                .and_then(|s| s.to_str())
                                .filter(|s| !s.is_empty())
                                .map(|name| name.to_string());
                        }

                        if head.len() >= 7
                            && head.as_bytes().iter().all(|byte| byte.is_ascii_hexdigit())
                        {
                            return Some(format!("detached: {}", &head[..7]));
                        }

                        None
                    });
                cache.last_head_mtime = modified;
            }

            cache.last_refresh = Some(now);
        }

        cache.value.clone()
    }

    fn render_status_bar(&self, area: Rect, buf: &mut Buffer) {
        use crate::exec_command::relativize_to_home;
        use ratatui::layout::Margin;
        use ratatui::style::Modifier;
        use ratatui::style::Style;
        use ratatui::text::Line;
        use ratatui::text::Span;
        use ratatui::widgets::Block;
        use ratatui::widgets::Borders;
        use ratatui::widgets::Paragraph;

        // Add same horizontal padding as the Message input (2 chars on each side)
        let horizontal_padding = 1u16;
        let padded_area = Rect {
            x: area.x + horizontal_padding,
            y: area.y,
            width: area.width.saturating_sub(horizontal_padding * 2),
            height: area.height,
        };

        // Get current working directory string
        let cwd_str = match relativize_to_home(&self.config.cwd) {
            Some(rel) if !rel.as_os_str().is_empty() => format!("~/{}", rel.display()),
            Some(_) => "~".to_string(),
            None => self.config.cwd.display().to_string(),
        };

        // Build status line spans with dynamic elision based on width.
        // Removal priority when space is tight:
        //   1) Reasoning level
        //   2) Model
        //   3) Branch
        //   4) Directory
        let branch_opt = self.get_git_branch();

        // Helper to assemble spans based on include flags
        let build_spans = |include_reasoning: bool,
                           include_model: bool,
                           include_branch: bool,
                           include_dir: bool| {
            let mut spans: Vec<Span> = Vec::new();
            // Title follows theme text color
            spans.push(Span::styled(
                "Code",
                Style::default()
                    .fg(crate::colors::text())
                    .add_modifier(Modifier::BOLD),
            ));

            if include_model {
                spans.push(Span::styled(
                    "  •  ",
                    Style::default().fg(crate::colors::text_dim()),
                ));
                spans.push(Span::styled(
                    "Model: ",
                    Style::default().fg(crate::colors::text_dim()),
                ));
                spans.push(Span::styled(
                    self.format_model_name(&self.config.model),
                    Style::default().fg(crate::colors::info()),
                ));
            }

            if include_reasoning {
                spans.push(Span::styled(
                    "  •  ",
                    Style::default().fg(crate::colors::text_dim()),
                ));
                spans.push(Span::styled(
                    "Reasoning: ",
                    Style::default().fg(crate::colors::text_dim()),
                ));
                spans.push(Span::styled(
                    format!("{}", self.config.model_reasoning_effort),
                    Style::default().fg(crate::colors::info()),
                ));
            }

            if include_dir {
                spans.push(Span::styled(
                    "  •  ",
                    Style::default().fg(crate::colors::text_dim()),
                ));
                spans.push(Span::styled(
                    "Directory: ",
                    Style::default().fg(crate::colors::text_dim()),
                ));
                spans.push(Span::styled(
                    cwd_str.clone(),
                    Style::default().fg(crate::colors::info()),
                ));
            }

            if include_branch && let Some(branch) = &branch_opt {
                spans.push(Span::styled(
                    "  •  ",
                    Style::default().fg(crate::colors::text_dim()),
                ));
                spans.push(Span::styled(
                    "Branch: ",
                    Style::default().fg(crate::colors::text_dim()),
                ));
                spans.push(Span::styled(
                    branch.clone(),
                    Style::default().fg(crate::colors::success_green()),
                ));
            }

            // Footer already shows the Ctrl+R hint; avoid duplicating it here.

            spans
        };

        // Start with all items
        let mut include_reasoning = true;
        let mut include_model = true;
        let mut include_branch = branch_opt.is_some();
        let mut include_dir = true;
        let mut status_spans = build_spans(
            include_reasoning,
            include_model,
            include_branch,
            include_dir,
        );

        // Now recompute exact available width inside the border + padding before measuring
        // Render a bordered status block and explicitly fill its background.
        // Without a background fill, some terminals blend with prior frame
        // contents, which is especially noticeable on dark themes as dark
        // "caps" at the edges. Match the app background for consistency.
        let status_block = Block::default()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(crate::colors::border()))
            .style(Style::default().bg(crate::colors::background()));
        let inner_area = status_block.inner(padded_area);
        let padded_inner = inner_area.inner(Margin::new(1, 0));
        let inner_width = padded_inner.width as usize;

        // Helper to measure current spans width
        let measure =
            |spans: &Vec<Span>| -> usize { spans.iter().map(|s| s.content.chars().count()).sum() };

        // Elide items in priority order until content fits
        while measure(&status_spans) > inner_width {
            if include_reasoning {
                include_reasoning = false;
            } else if include_model {
                include_model = false;
            } else if include_branch {
                include_branch = false;
            } else if include_dir {
                include_dir = false;
            } else {
                break;
            }
            status_spans = build_spans(
                include_reasoning,
                include_model,
                include_branch,
                include_dir,
            );
        }

        // Note: The reasoning visibility hint is appended inside `build_spans`
        // so it participates in width measurement and elision. Do not append
        // it again here to avoid overflow that caused corrupted glyph boxes on
        // some terminals.

        let status_line = Line::from(status_spans);

        // Render the block first
        status_block.render(padded_area, buf);

        // Then render the text inside with padding, centered
        let status_widget = Paragraph::new(vec![status_line])
            .alignment(ratatui::layout::Alignment::Center)
            .style(
                Style::default()
                    .bg(crate::colors::background())
                    .fg(crate::colors::text()),
            );
        ratatui::widgets::Widget::render(status_widget, padded_inner, buf);
    }
    // Browser screenshot rendering methods removed (MAINT-11 Phase 6)
}
