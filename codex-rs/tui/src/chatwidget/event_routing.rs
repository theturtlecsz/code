//! Event routing for ChatWidget.
//!
//! Extracted from mod.rs to reduce file size and merge conflict risk.
//! Contains: handle_key_event, handle_mouse_event, handle_codex_event.

use super::*;

impl ChatWidget<'_> {
    pub(crate) fn handle_key_event(&mut self, key_event: KeyEvent) {
        // PM-004 Batch C: F11 toggles high-contrast mode globally
        if matches!(key_event.code, crossterm::event::KeyCode::F(11)) {
            crate::theme::toggle_high_contrast();
            self.request_redraw();
            return;
        }

        if terminal_handlers::handle_terminal_key(self, key_event) {
            return;
        }
        if self.terminal.overlay.is_some() {
            // Block background input while the terminal overlay is visible.
            return;
        }
        if limits_handlers::handle_limits_key(self, key_event) {
            return;
        }
        if self.limits.overlay.is_some() {
            return;
        }
        // Intercept keys for overlays when active (help first, then diff)
        if help_handlers::handle_help_key(self, key_event) {
            return;
        }
        if self.help.overlay.is_some() {
            return;
        }
        if diff_handlers::handle_diff_key(self, key_event) {
            return;
        }
        if self.diffs.overlay.is_some() {
            return;
        }
        if pm_handlers::handle_pm_key(self, key_event) {
            return;
        }
        if self.pm.overlay.is_some() {
            return;
        }
        if self.pro.overlay_visible {
            if self.handle_pro_overlay_key(key_event) {
                return;
            }
            if self.pro.overlay_visible {
                return;
            }
        }
        if key_event.kind == KeyEventKind::Press {
            self.bottom_pane.clear_ctrl_c_quit_hint();
        }

        // Esc cancels running spec_auto pipeline
        if let crossterm::event::KeyEvent {
            code: crossterm::event::KeyCode::Esc,
            kind: KeyEventKind::Press,
            ..
        } = key_event
        {
            if self.spec_auto_state.is_some() {
                // Cancel the running pipeline
                spec_kit::halt_spec_auto_with_error(self, "Cancelled by user (Esc)".to_string());
                self.push_background_tail("Pipeline cancelled.".to_string());
                self.request_redraw();
                return;
            }
        }

        // Global HUD toggles (avoid conflicting with common editor keys):
        // - Ctrl+A: toggle Agents terminal mode
        if let KeyEvent {
            code: crossterm::event::KeyCode::Char('a'),
            modifiers: crossterm::event::KeyModifiers::CONTROL,
            kind: KeyEventKind::Press | KeyEventKind::Repeat,
            ..
        } = key_event
        {
            self.toggle_agents_hud();
            return;
        }

        if self.agents_terminal.active {
            use crossterm::event::KeyCode;
            if !matches!(key_event.kind, KeyEventKind::Press | KeyEventKind::Repeat) {
                return;
            }
            match key_event.code {
                KeyCode::Esc => {
                    if self.agents_terminal.focus() == AgentsTerminalFocus::Detail {
                        self.agents_terminal.focus_sidebar();
                        self.request_redraw();
                    } else {
                        self.exit_agents_terminal_mode();
                    }
                    return;
                }
                KeyCode::Right | KeyCode::Enter => {
                    if self.agents_terminal.focus() == AgentsTerminalFocus::Sidebar
                        && self.agents_terminal.current_agent_id().is_some()
                    {
                        self.agents_terminal.focus_detail();
                        self.request_redraw();
                    }
                    return;
                }
                KeyCode::Left => {
                    if self.agents_terminal.focus() == AgentsTerminalFocus::Detail {
                        self.agents_terminal.focus_sidebar();
                        self.request_redraw();
                    }
                    return;
                }
                KeyCode::Up => {
                    if self.agents_terminal.focus() == AgentsTerminalFocus::Detail {
                        layout_scroll::line_up(self);
                        self.record_current_agent_scroll();
                    } else {
                        self.navigate_agents_terminal_selection(-1);
                    }
                    return;
                }
                KeyCode::Down => {
                    if self.agents_terminal.focus() == AgentsTerminalFocus::Detail {
                        layout_scroll::line_down(self);
                        self.record_current_agent_scroll();
                    } else {
                        self.navigate_agents_terminal_selection(1);
                    }
                    return;
                }
                KeyCode::Tab => {
                    self.agents_terminal.focus_sidebar();
                    self.navigate_agents_terminal_selection(1);
                    return;
                }
                KeyCode::BackTab => {
                    self.agents_terminal.focus_sidebar();
                    self.navigate_agents_terminal_selection(-1);
                    return;
                }
                KeyCode::PageUp => {
                    layout_scroll::page_up(self);
                    self.record_current_agent_scroll();
                    return;
                }
                KeyCode::PageDown => {
                    layout_scroll::page_down(self);
                    self.record_current_agent_scroll();
                    return;
                }
                _ => {
                    return;
                }
            }
        }

        if let KeyEvent {
            code: crossterm::event::KeyCode::Char('p'),
            modifiers,
            kind: KeyEventKind::Press | KeyEventKind::Repeat,
            ..
        } = key_event
        {
            use crossterm::event::KeyModifiers;
            if modifiers.contains(KeyModifiers::CONTROL) && modifiers.contains(KeyModifiers::SHIFT)
            {
                self.toggle_pro_hud();
                return;
            }
            if modifiers == KeyModifiers::CONTROL {
                self.toggle_pro_overlay();
                return;
            }
        }

        // Fast-path PageUp/PageDown to scroll the transcript by a viewport at a time.
        if let crossterm::event::KeyEvent {
            code: crossterm::event::KeyCode::PageUp,
            kind: KeyEventKind::Press | KeyEventKind::Repeat,
            ..
        } = key_event
        {
            layout_scroll::page_up(self);
            return;
        }
        if let crossterm::event::KeyEvent {
            code: crossterm::event::KeyCode::PageDown,
            kind: KeyEventKind::Press | KeyEventKind::Repeat,
            ..
        } = key_event
        {
            layout_scroll::page_down(self);
            return;
        }
        // Home/End: when the composer is empty, jump the history to start/end
        if let crossterm::event::KeyEvent {
            code: crossterm::event::KeyCode::Home,
            kind: KeyEventKind::Press | KeyEventKind::Repeat,
            ..
        } = key_event
            && self.composer_is_empty()
        {
            layout_scroll::to_top(self);
            return;
        }
        if let crossterm::event::KeyEvent {
            code: crossterm::event::KeyCode::End,
            kind: KeyEventKind::Press | KeyEventKind::Repeat,
            ..
        } = key_event
            && self.composer_is_empty()
        {
            layout_scroll::to_bottom(self);
            return;
        }

        match self.bottom_pane.handle_key_event(key_event) {
            InputResult::Submitted(text) => {
                // Commit pending jump-back (make trimming permanent) before submission
                if self.pending_jump_back.is_some() {
                    self.pending_jump_back = None;
                }
                if self.try_handle_terminal_shortcut(&text) {
                    return;
                }
                let user_message = self.parse_message_with_images(text);
                self.submit_user_message(user_message);
            }
            InputResult::Command(_cmd) => {
                // Command was dispatched at the App layer; request redraw.
                self.app_event_tx.send(AppEvent::RequestRedraw);
            }
            InputResult::ScrollUp => {
                // Only allow Up to navigate command history when the top view
                // cannot be scrolled at all (no scrollback available).
                if self.layout.last_max_scroll.get() == 0 && self.bottom_pane.try_history_up() {
                    return;
                }
                // Scroll up in chat history (increase offset, towards older content)
                // Use last_max_scroll computed during the previous render to avoid overshoot
                let new_offset = self
                    .layout
                    .scroll_offset
                    .saturating_add(3)
                    .min(self.layout.last_max_scroll.get());
                self.layout.scroll_offset = new_offset;
                self.flash_scrollbar();
                // Enable compact mode so history can use the spacer line
                if self.layout.scroll_offset > 0 {
                    self.bottom_pane.set_compact_compose(true);
                    self.height_manager
                        .borrow_mut()
                        .record_event(HeightEvent::ComposerModeChange);
                    // Mark that the very next Down should continue scrolling chat (sticky)
                    self.bottom_pane.mark_next_down_scrolls_history();
                }
                self.app_event_tx.send(AppEvent::RequestRedraw);
                self.height_manager
                    .borrow_mut()
                    .record_event(HeightEvent::UserScroll);
                self.maybe_show_history_nav_hint_on_first_scroll();
            }
            InputResult::ScrollDown => {
                // Only allow Down to navigate command history when the top view
                // cannot be scrolled at all (no scrollback available).
                if self.layout.last_max_scroll.get() == 0
                    && self.bottom_pane.history_is_browsing()
                    && self.bottom_pane.try_history_down()
                {
                    return;
                }
                // Scroll down in chat history (decrease offset, towards bottom)
                if self.layout.scroll_offset == 0 {
                    // Already at bottom: ensure spacer above input is enabled.
                    self.bottom_pane.set_compact_compose(false);
                    self.app_event_tx.send(AppEvent::RequestRedraw);
                    self.height_manager
                        .borrow_mut()
                        .record_event(HeightEvent::UserScroll);
                    self.maybe_show_history_nav_hint_on_first_scroll();
                    self.height_manager
                        .borrow_mut()
                        .record_event(HeightEvent::ComposerModeChange);
                } else if self.layout.scroll_offset >= 3 {
                    // Move towards bottom but do NOT toggle spacer yet; wait until
                    // the user confirms by pressing Down again at bottom.
                    self.layout.scroll_offset = self.layout.scroll_offset.saturating_sub(3);
                    self.app_event_tx.send(AppEvent::RequestRedraw);
                    self.height_manager
                        .borrow_mut()
                        .record_event(HeightEvent::UserScroll);
                    self.maybe_show_history_nav_hint_on_first_scroll();
                } else if self.layout.scroll_offset > 0 {
                    // Land exactly at bottom without toggling spacer yet; require
                    // a subsequent Down to re-enable the spacer so the input
                    // doesn't move when scrolling into the line above it.
                    self.layout.scroll_offset = 0;
                    self.app_event_tx.send(AppEvent::RequestRedraw);
                    self.height_manager
                        .borrow_mut()
                        .record_event(HeightEvent::UserScroll);
                    self.maybe_show_history_nav_hint_on_first_scroll();
                }
                self.flash_scrollbar();
            }
            InputResult::None => {
                // Trigger redraw so input wrapping/height reflects immediately
                self.app_event_tx.send(AppEvent::RequestRedraw);
            }
        }
    }

    pub(crate) fn handle_mouse_event(&mut self, mouse_event: crossterm::event::MouseEvent) {
        use crossterm::event::KeyModifiers;
        use crossterm::event::MouseEventKind;

        // Check if Shift is held - if so, let the terminal handle selection
        if mouse_event.modifiers.contains(KeyModifiers::SHIFT) {
            // Don't handle any mouse events when Shift is held
            // This allows the terminal's native text selection to work
            return;
        }

        match mouse_event.kind {
            MouseEventKind::ScrollUp => layout_scroll::mouse_scroll(self, true),
            MouseEventKind::ScrollDown => layout_scroll::mouse_scroll(self, false),
            _ => {
                // Ignore other mouse events for now
            }
        }
    }

    // MAINT-11: handle_pro_event, describe_pro_category, describe_pro_phase moved to pro_overlay.rs

    pub(crate) fn handle_codex_event(&mut self, event: Event) {
        tracing::debug!(
            "handle_codex_event({})",
            serde_json::to_string_pretty(&event).unwrap_or_default()
        );
        // Strict ordering: all LLM/tool events must carry OrderMeta; internal events use synthetic keys.
        // Track provider order to anchor internal inserts at the bottom of the active request.
        self.note_order(event.order.as_ref());

        let Event { id, msg, .. } = event.clone();
        match msg {
            EventMsg::SessionConfigured(event) => {
                // Remove stale "Connecting MCP servers…" status from the startup notice
                // now that MCP initialization has completed in core.
                self.remove_connecting_mcp_notice();
                // Record session id for potential future fork/backtrack features
                self.session_id = Some(event.session_id);
                self.bottom_pane
                    .set_history_metadata(event.history_log_id, event.history_entry_count);
                // Record session information at the top of the conversation.
                // If we already showed the startup prelude (Popular commands),
                // avoid inserting a duplicate. Still surface a notice if the
                // model actually changed from the requested one.
                let is_first = !self.welcome_shown;
                if is_first || self.config.model != event.model {
                    if is_first {
                        self.welcome_shown = true;
                    }
                    self.history_push_top_next_req(history_cell::new_session_info(
                        &self.config,
                        event,
                        is_first,
                        self.latest_upgrade_version.as_deref(),
                    )); // tag: prelude
                }

                if let Some(user_message) = self.initial_user_message.take() {
                    // If the user provided an initial message, add it to the
                    // conversation history.
                    self.submit_user_message(user_message);
                }

                self.request_redraw();
            }
            EventMsg::Pro(event) => {
                self.handle_pro_event(event);
            }
            EventMsg::WebSearchBegin(ev) => {
                // Enforce order presence (tool events should carry it)
                let ok = match event.order.as_ref() {
                    Some(om) => Self::order_key_from_order_meta(om),
                    None => {
                        tracing::warn!("missing OrderMeta on WebSearchBegin; using synthetic key");
                        self.next_internal_key()
                    }
                };
                tracing::info!(
                    "[order] WebSearchBegin call_id={} seq={}",
                    ev.call_id,
                    event.event_seq
                );
                tools::web_search_begin(self, ev.call_id, ev.query, ok)
            }
            EventMsg::AgentMessage(AgentMessageEvent { message }) => {
                // If the user requested an interrupt, ignore late final answers.
                if self.stream_state.drop_streaming {
                    tracing::debug!("Ignoring AgentMessage after interrupt");
                    return;
                }
                self.stream_state.seq_answer_final = Some(event.event_seq);

                // SPEC-954-FIX: Update user cell OrderKey when first OrderMeta arrives
                if let Some(om) = event.order.as_ref()
                    && let Some(cell_idx) = self.pending_user_cell_updates.remove(&id)
                {
                    if cell_idx < self.cell_order_seq.len() {
                        let old_key = self.cell_order_seq[cell_idx];
                        let new_key = OrderKey {
                            req: om.request_ordinal,
                            out: old_key.out,
                            seq: old_key.seq,
                        };

                        let req_diff = (new_key.req as i64 - old_key.req as i64).abs();

                        tracing::info!(
                            "🔵 ORDER_UPDATE (AgentMessage): task={} | old=req:{},out:{},seq:{} | new=req:{},out:{},seq:{} | diff={}",
                            id,
                            old_key.req,
                            old_key.out,
                            old_key.seq,
                            new_key.req,
                            new_key.out,
                            new_key.seq,
                            req_diff
                        );

                        self.cell_order_seq[cell_idx] = new_key;

                        // SPEC-954-FIX: Always resort when req changes (even diff=1 needs reordering)
                        if req_diff > 0 {
                            tracing::debug!("🔄 RESORT: req changed, diff={}", req_diff);
                            self.resort_history_by_order();
                        }
                    } else {
                        tracing::error!(
                            "🔴 ORDER_UPDATE_FAILED (AgentMessage): cell_idx={} out of bounds (len={})",
                            cell_idx,
                            self.cell_order_seq.len()
                        );
                    }
                }

                // Strict order for the stream id
                let ok = match event.order.as_ref() {
                    Some(om) => Self::order_key_from_order_meta(om),
                    None => {
                        tracing::warn!("missing OrderMeta on AgentMessage; using synthetic key");
                        self.next_internal_key()
                    }
                };
                self.seed_stream_order_key(StreamKind::Answer, &id, ok);

                tracing::debug!(
                    "AgentMessage final id={} bytes={} preview={:?}",
                    id,
                    message.len(),
                    message.chars().take(80).collect::<String>()
                );

                // Close out any running tool/exec indicators before inserting final answer.
                self.finalize_all_running_due_to_answer();

                // Route final message through streaming controller so AppEvent::InsertFinalAnswer
                // is the single source of truth for assistant content.
                let sink = AppEventHistorySink(self.app_event_tx.clone());
                streaming::begin(self, StreamKind::Answer, Some(id.clone()));
                let _ = self.stream.apply_final_answer(&message, &sink);

                // Track last message for potential dedup heuristics.
                self.last_assistant_message = Some(message);
                // Mark this Answer stream id as closed for the rest of the turn so any late
                // AgentMessageDelta for the same id is ignored. In the full App runtime,
                // the InsertFinalAnswer path also marks closed; setting it here makes
                // unit tests (which do not route AppEvents back) behave identically.
                self.stream_state
                    .closed_answer_ids
                    .insert(StreamId(id.clone()));
                // Receiving a final answer means this task has finished even if we have not yet
                // observed the corresponding TaskComplete event. Clear the active marker now so
                // the status spinner can hide promptly when nothing else is running.
                self.active_task_ids.remove(&id);
                self.maybe_hide_spinner();
            }
            EventMsg::ReplayHistory(ev) => {
                let codex_core::protocol::ReplayHistoryEvent { items, events } = ev;
                let mut max_req = self.last_seen_request_index;
                if events.is_empty() {
                    for item in &items {
                        self.render_replay_item(item.clone());
                    }
                } else {
                    for recorded in events {
                        if matches!(recorded.msg, EventMsg::ReplayHistory(_)) {
                            continue;
                        }
                        if let Some(order) = recorded.order.as_ref() {
                            max_req = max_req.max(order.request_ordinal);
                        }
                        let event = Event {
                            id: recorded.id,
                            event_seq: recorded.event_seq,
                            msg: recorded.msg,
                            order: recorded.order,
                        };
                        self.handle_codex_event(event);
                    }
                }
                if !items.is_empty() {
                    // History items were inserted using synthetic keys; promote current request
                    // index so subsequent messages append to the end instead of the top.
                    self.last_seen_request_index =
                        self.last_seen_request_index.max(self.current_request_index);
                }
                if max_req > 0 {
                    self.last_seen_request_index = self.last_seen_request_index.max(max_req);
                    self.current_request_index = self.last_seen_request_index;
                }
                self.request_redraw();
            }
            EventMsg::WebSearchComplete(ev) => {
                tools::web_search_complete(self, ev.call_id, ev.query)
            }
            EventMsg::AgentMessageDelta(AgentMessageDeltaEvent { delta }) => {
                tracing::debug!("AgentMessageDelta: {:?}", delta);
                // If the user requested an interrupt, ignore late deltas.
                if self.stream_state.drop_streaming {
                    tracing::debug!("Ignoring Answer delta after interrupt");
                    return;
                }
                // Ignore late deltas for ids that have already finalized in this turn
                if self
                    .stream_state
                    .closed_answer_ids
                    .contains(&StreamId(id.clone()))
                {
                    tracing::debug!("Ignoring Answer delta for closed id={}", id);
                    return;
                }
                // SPEC-954-FIX: Update user cell OrderKey when first OrderMeta arrives
                if let Some(om) = event.order.as_ref()
                    && let Some(cell_idx) = self.pending_user_cell_updates.remove(&id)
                {
                    if cell_idx < self.cell_order_seq.len() {
                        let old_key = self.cell_order_seq[cell_idx];
                        let new_key = OrderKey {
                            req: om.request_ordinal, // ✅ Provider's number
                            out: old_key.out,        // Keep MIN+1
                            seq: old_key.seq,        // Keep original seq
                        };

                        let req_diff = (new_key.req as i64 - old_key.req as i64).abs();

                        tracing::info!(
                            "🔵 ORDER_UPDATE: task={} | old=req:{},out:{},seq:{} | new=req:{},out:{},seq:{} | diff={} | will_resort={}",
                            id,
                            old_key.req,
                            old_key.out,
                            old_key.seq,
                            new_key.req,
                            new_key.out,
                            new_key.seq,
                            req_diff,
                            req_diff > 1
                        );

                        self.cell_order_seq[cell_idx] = new_key;

                        // SPEC-954-FIX: Always resort when req changes (even diff=1 needs reordering)
                        if req_diff > 0 {
                            tracing::debug!("🔄 RESORT: req changed, diff={}", req_diff);
                            self.resort_history_by_order();
                        } else {
                            tracing::debug!("⏭️  RESORT_SKIPPED: req unchanged");
                        }
                    } else {
                        tracing::error!(
                            "🔴 ORDER_UPDATE_FAILED: cell_idx={} out of bounds (len={})",
                            cell_idx,
                            self.cell_order_seq.len()
                        );
                    }
                }

                // Seed/refresh order key for this Answer stream id (must have OrderMeta)
                let ok = match event.order.as_ref() {
                    Some(om) => Self::order_key_from_order_meta(om),
                    None => {
                        tracing::warn!(
                            "missing OrderMeta on AgentMessageDelta; using synthetic key"
                        );
                        self.next_internal_key()
                    }
                };
                self.seed_stream_order_key(StreamKind::Answer, &id, ok);
                // Stream answer delta through StreamController
                streaming::delta_text(
                    self,
                    StreamKind::Answer,
                    id.clone(),
                    delta,
                    event.order.as_ref().and_then(|o| o.sequence_number),
                );
                // Show responding state while assistant streams
                self.bottom_pane
                    .update_status_text("responding".to_string());
            }
            EventMsg::AgentReasoning(AgentReasoningEvent { text }) => {
                // Ignore late reasoning if we've dropped streaming due to interrupt.
                if self.stream_state.drop_streaming {
                    tracing::debug!("Ignoring AgentReasoning after interrupt");
                    return;
                }
                tracing::debug!(
                    "AgentReasoning event with text: {:?}...",
                    text.chars().take(100).collect::<String>()
                );
                // Guard duplicates for this id within the task
                if self
                    .stream_state
                    .closed_reasoning_ids
                    .contains(&StreamId(id.clone()))
                {
                    tracing::warn!("Ignoring duplicate AgentReasoning for closed id={}", id);
                    return;
                }
                // Seed strict order key for this Reasoning stream
                let ok = match event.order.as_ref() {
                    Some(om) => Self::order_key_from_order_meta(om),
                    None => {
                        tracing::warn!("missing OrderMeta on AgentReasoning; using synthetic key");
                        self.next_internal_key()
                    }
                };
                tracing::info!("[order] EventMsg::AgentReasoning id={} key={:?}", id, ok);
                self.seed_stream_order_key(StreamKind::Reasoning, &id, ok);
                // Fallback: if any tools/execs are still marked running, complete them now.
                self.finalize_all_running_due_to_answer();
                // Use StreamController for final reasoning
                let sink = AppEventHistorySink(self.app_event_tx.clone());
                streaming::begin(self, StreamKind::Reasoning, Some(id.clone()));

                // The StreamController now properly handles duplicate detection and prevents
                // re-injecting content when we're already finishing a stream
                let _finished = self.stream.apply_final_reasoning(&text, &sink);
                // Stream finishing is handled by StreamController
                // Mark this id closed for further reasoning deltas in this turn
                self.stream_state
                    .closed_reasoning_ids
                    .insert(StreamId(id.clone()));
                // Clear in-progress flags on the most recent reasoning cell(s)
                if let Some(last) = self.history_cells.iter().rposition(|c| {
                    c.as_any()
                        .downcast_ref::<history_cell::CollapsibleReasoningCell>()
                        .is_some()
                }) && let Some(reason) = self.history_cells[last]
                    .as_any()
                    .downcast_ref::<history_cell::CollapsibleReasoningCell>(
                ) {
                    reason.set_in_progress(false);
                }
                self.mark_needs_redraw();
            }
            EventMsg::AgentReasoningDelta(AgentReasoningDeltaEvent { delta }) => {
                tracing::debug!("AgentReasoningDelta: {:?}", delta);
                if self.stream_state.drop_streaming {
                    tracing::debug!("Ignoring Reasoning delta after interrupt");
                    return;
                }
                // Ignore late deltas for ids that have already finalized in this turn
                if self
                    .stream_state
                    .closed_reasoning_ids
                    .contains(&StreamId(id.clone()))
                {
                    tracing::debug!("Ignoring Reasoning delta for closed id={}", id);
                    return;
                }
                // Seed strict order key for this Reasoning stream
                let ok = match event.order.as_ref() {
                    Some(om) => Self::order_key_from_order_meta(om),
                    None => {
                        tracing::warn!(
                            "missing OrderMeta on AgentReasoningDelta; using synthetic key"
                        );
                        self.next_internal_key()
                    }
                };
                tracing::info!(
                    "[order] EventMsg::AgentReasoningDelta id={} key={:?}",
                    id,
                    ok
                );
                self.seed_stream_order_key(StreamKind::Reasoning, &id, ok);
                // Stream reasoning delta through StreamController
                streaming::delta_text(
                    self,
                    StreamKind::Reasoning,
                    id.clone(),
                    delta,
                    event.order.as_ref().and_then(|o| o.sequence_number),
                );
                // Show thinking state while reasoning streams
                self.bottom_pane.update_status_text("thinking".to_string());
            }
            EventMsg::AgentReasoningSectionBreak(AgentReasoningSectionBreakEvent {}) => {
                // Insert section break in reasoning stream
                let sink = AppEventHistorySink(self.app_event_tx.clone());
                self.stream.insert_reasoning_section_break(&sink);
            }
            EventMsg::TaskStarted => {
                tracing::warn!("DEBUG: TaskStarted event received, id={}", id);
                spec_kit::on_spec_auto_task_started(self, &id);
                // This begins the new turn; clear the pending prompt anchor count
                // so subsequent background events use standard placement.
                self.pending_user_prompts_for_next_turn = 0;

                // SPEC-954: Clear timeout tracking - provider has responded
                self.pending_message_timestamps.clear();

                // SPEC-954-FIX: Create deferred user cell with temporary OrderKey
                if let Some(user_text) = self.pending_dispatched_user_messages.pop_front() {
                    // Use next_req_key_prompt() to properly increment counters and generate unique req
                    let temp_key = self.next_req_key_prompt();

                    tracing::info!(
                        "🔵 USER_CELL_CREATED: task={} | temp_req={} | temp_out={} | seq={} | pending_updates={}",
                        id,
                        temp_key.req,
                        temp_key.out,
                        temp_key.seq,
                        self.pending_user_cell_updates.len()
                    );

                    let cell_idx = self.history_insert_with_key_global_tagged(
                        Box::new(history_cell::new_user_prompt(user_text)),
                        temp_key,
                        "prompt-deferred",
                    );

                    self.pending_user_cell_updates.insert(id.clone(), cell_idx);
                }

                // Reset stream headers for new turn
                self.stream.reset_headers_for_new_turn();
                self.stream_state.current_kind = None;
                // New turn: clear closed id guards
                self.stream_state.closed_answer_ids.clear();
                self.stream_state.closed_reasoning_ids.clear();
                self.ended_call_ids.clear();
                self.bottom_pane.clear_ctrl_c_quit_hint();
                // Accept streaming again for this turn
                self.stream_state.drop_streaming = false;
                // Mark this task id as active and ensure the status stays visible
                self.active_task_ids.insert(id.clone());
                // Reset per-turn UI indicators; ordering is now global-only
                self.reasoning_index.clear();
                self.bottom_pane.set_task_running(true);
                self.bottom_pane
                    .update_status_text("waiting for model".to_string());
                tracing::info!("[order] EventMsg::TaskStarted id={}", id);

                // Don't add loading cell - we have progress in the input area
                // self.add_to_history(history_cell::new_loading_cell("waiting for model".to_string()));

                self.mark_needs_redraw();
            }
            EventMsg::TaskComplete(TaskCompleteEvent {
                last_agent_message: _,
            }) => {
                tracing::warn!("DEBUG: TaskComplete event received, id={}", id);
                spec_kit::on_spec_auto_task_complete(self, &id);
                // Finalize any active streams
                if self.stream.is_write_cycle_active() {
                    // Finalize both streams via streaming facade
                    streaming::finalize(self, StreamKind::Reasoning, true);
                    streaming::finalize(self, StreamKind::Answer, true);
                }
                // Remove this id from the active set (it may be a sub‑agent)
                self.active_task_ids.remove(&id);
                // Defensive: clear transient agents-preparing state
                self.agents_ready_to_start = false;
                // Convert any lingering running exec/tool cells to completed so the UI doesn't hang
                self.finalize_all_running_due_to_answer();
                // Mark any running web searches as completed
                if !self.tools_state.running_web_search.is_empty() {
                    // Replace each running web search cell in-place with a completed one
                    // Iterate over a snapshot of keys to avoid borrow issues
                    let entries: Vec<(ToolCallId, (usize, Option<String>))> = self
                        .tools_state
                        .running_web_search
                        .iter()
                        .map(|(k, v)| (k.clone(), v.clone()))
                        .collect();
                    for (call_id, (idx, query_opt)) in entries {
                        // Try exact index; if out of bounds or shifted, search nearby from end
                        let mut target_idx = None;
                        if idx < self.history_cells.len() {
                            // Verify this index is still a running web search cell
                            let is_ws = self.history_cells[idx]
                                .as_any()
                                .downcast_ref::<history_cell::RunningToolCallCell>()
                                .is_some_and(|rt| rt.has_title("Web Search..."));
                            if is_ws {
                                target_idx = Some(idx);
                            }
                        }
                        if target_idx.is_none() {
                            for i in (0..self.history_cells.len()).rev() {
                                if let Some(rt) = self.history_cells[i]
                                    .as_any()
                                    .downcast_ref::<history_cell::RunningToolCallCell>(
                                ) && rt.has_title("Web Search...")
                                {
                                    target_idx = Some(i);
                                    break;
                                }
                            }
                        }
                        if let Some(i) = target_idx
                            && let Some(rt) = self.history_cells[i]
                                .as_any()
                                .downcast_ref::<history_cell::RunningToolCallCell>()
                        {
                            let completed = rt.finalize_web_search(true, query_opt);
                            self.history_replace_at(i, Box::new(completed));
                        }
                        // Remove from running set
                        self.tools_state.running_web_search.remove(&call_id);
                    }
                }
                // Now that streaming is complete, flush any queued interrupts
                self.flush_interrupt_queue();

                // Only drop the working status if nothing is actually running.
                let any_tools_running = !self.exec.running_commands.is_empty()
                    || !self.tools_state.running_custom_tools.is_empty()
                    || !self.tools_state.running_web_search.is_empty();
                let any_streaming = self.stream.is_write_cycle_active();
                let any_agents_active = self.agents_are_actively_running();
                let any_tasks_active = !self.active_task_ids.is_empty();

                if !(any_tools_running || any_streaming || any_agents_active || any_tasks_active) {
                    self.bottom_pane.set_task_running(false);
                    // Ensure any transient footer text like "responding" is cleared when truly idle
                    self.bottom_pane.update_status_text(String::new());
                }
                self.stream_state.current_kind = None;
                // Final re-check for idle state
                self.maybe_hide_spinner();
                self.mark_needs_redraw();
            }
            EventMsg::AgentReasoningRawContentDelta(AgentReasoningRawContentDeltaEvent {
                delta,
            }) => {
                if self.stream_state.drop_streaming {
                    tracing::debug!("Ignoring RawContent delta after interrupt");
                    return;
                }
                // Treat raw reasoning content the same as summarized reasoning
                if self
                    .stream_state
                    .closed_reasoning_ids
                    .contains(&StreamId(id.clone()))
                {
                    tracing::debug!("Ignoring RawContent delta for closed id={}", id);
                    return;
                }
                // Seed strict order key for this reasoning stream id
                let ok = match event.order.as_ref() {
                    Some(om) => Self::order_key_from_order_meta(om),
                    None => {
                        tracing::warn!(
                            "missing OrderMeta on Tools::PlanUpdate; using synthetic key"
                        );
                        self.next_internal_key()
                    }
                };
                self.seed_stream_order_key(StreamKind::Reasoning, &id, ok);

                streaming::delta_text(
                    self,
                    StreamKind::Reasoning,
                    id.clone(),
                    delta,
                    event.order.as_ref().and_then(|o| o.sequence_number),
                );
            }
            EventMsg::AgentReasoningRawContent(AgentReasoningRawContentEvent { text }) => {
                if self.stream_state.drop_streaming {
                    tracing::debug!("Ignoring AgentReasoningRawContent after interrupt");
                    return;
                }
                tracing::debug!(
                    "AgentReasoningRawContent event with text: {:?}...",
                    text.chars().take(100).collect::<String>()
                );
                if self
                    .stream_state
                    .closed_reasoning_ids
                    .contains(&StreamId(id.clone()))
                {
                    tracing::warn!(
                        "Ignoring duplicate AgentReasoningRawContent for closed id={}",
                        id
                    );
                    return;
                }
                // Seed strict order key now so upcoming insert uses the correct key.
                let ok = match event.order.as_ref() {
                    Some(om) => Self::order_key_from_order_meta(om),
                    None => {
                        tracing::warn!(
                            "missing OrderMeta on Tools::ReasoningBegin; using synthetic key"
                        );
                        self.next_internal_key()
                    }
                };
                self.seed_stream_order_key(StreamKind::Reasoning, &id, ok);
                // Use StreamController for final raw reasoning
                let sink = AppEventHistorySink(self.app_event_tx.clone());
                streaming::begin(self, StreamKind::Reasoning, Some(id.clone()));
                let _finished = self.stream.apply_final_reasoning(&text, &sink);
                // Stream finishing is handled by StreamController
                self.stream_state
                    .closed_reasoning_ids
                    .insert(StreamId(id.clone()));
                if let Some(last) = self.history_cells.iter().rposition(|c| {
                    c.as_any()
                        .downcast_ref::<history_cell::CollapsibleReasoningCell>()
                        .is_some()
                }) && let Some(reason) = self.history_cells[last]
                    .as_any()
                    .downcast_ref::<history_cell::CollapsibleReasoningCell>(
                ) {
                    reason.set_in_progress(false);
                }
                self.mark_needs_redraw();
            }
            EventMsg::TokenCount(event) => {
                if let Some(info) = &event.info {
                    self.total_token_usage = info.total_token_usage.clone();
                    self.last_token_usage = info.last_token_usage.clone();
                }
                if let Some(snapshot) = event.rate_limits {
                    self.update_rate_limit_resets(&snapshot);
                    let warnings = self.rate_limit_warnings.take_warnings(
                        snapshot.secondary_used_percent,
                        snapshot.primary_used_percent,
                    );
                    if !warnings.is_empty() {
                        for warning in warnings {
                            self.history_push(history_cell::new_warning_event(warning));
                        }
                        self.request_redraw();
                    }

                    self.rate_limit_snapshot = Some(snapshot);
                    self.rate_limit_last_fetch_at = Some(Utc::now());
                    self.rate_limit_fetch_inflight = false;
                    if self.limits.overlay.is_some() {
                        self.rebuild_limits_overlay();
                        self.request_redraw();
                    }
                }
                self.bottom_pane.set_token_usage(
                    self.total_token_usage.clone(),
                    self.last_token_usage.clone(),
                    self.config.model_context_window,
                );
            }
            EventMsg::Error(ErrorEvent { message }) => {
                self.on_error(message);
            }
            EventMsg::PlanUpdate(update) => {
                let (plan_title, plan_active) = {
                    let title = update
                        .name
                        .as_ref()
                        .map(|s| s.trim())
                        .filter(|s| !s.is_empty())
                        .map(|s| s.to_string());
                    let total = update.plan.len();
                    let completed = update
                        .plan
                        .iter()
                        .filter(|p| matches!(p.status, StepStatus::Completed))
                        .count();
                    let active = total > 0 && completed < total;
                    (title, active)
                };
                // Insert plan updates at the time they occur. If the provider
                // supplied OrderMeta, honor it. Otherwise, derive a key within
                // the current (last-seen) request — do NOT advance to the next
                // request when a prompt is already queued, since these belong
                // to the in-flight turn.
                let key = self.near_time_key_current_req(event.order.as_ref());
                let _ = self.history_insert_with_key_global(
                    Box::new(history_cell::new_plan_update(update)),
                    key,
                );
                // If we inserted during streaming, keep the reasoning ellipsis visible.
                self.restore_reasoning_in_progress_if_streaming();
                let desired_title = if plan_active {
                    Some(plan_title.unwrap_or_else(|| "Plan".to_string()))
                } else {
                    None
                };
                self.apply_plan_terminal_title(desired_title);
            }
            EventMsg::ExecApprovalRequest(ev) => {
                let id2 = id.clone();
                let ev2 = ev.clone();
                let seq = event.event_seq;
                self.defer_or_handle(
                    move |interrupts| interrupts.push_exec_approval(seq, id, ev),
                    |this| {
                        this.finalize_active_stream();
                        this.flush_interrupt_queue();
                        this.handle_exec_approval_now(id2, ev2);
                        this.request_redraw();
                    },
                );
            }
            EventMsg::ApplyPatchApprovalRequest(ev) => {
                let id2 = id.clone();
                let ev2 = ev.clone();
                self.defer_or_handle(
                    move |interrupts| interrupts.push_apply_patch_approval(event.event_seq, id, ev),
                    |this| {
                        this.finalize_active_stream();
                        this.flush_interrupt_queue();
                        // Push approval UI state to bottom pane and surface the patch summary there.
                        // (Avoid inserting a duplicate summary here; handle_apply_patch_approval_now
                        // is responsible for rendering the proposed patch once.)
                        this.handle_apply_patch_approval_now(id2, ev2);
                        this.request_redraw();
                    },
                );
            }
            EventMsg::ExecCommandBegin(ev) => {
                let ev2 = ev.clone();
                let seq = event.event_seq;
                let om_begin = event
                    .order
                    .clone()
                    .expect("missing OrderMeta for ExecCommandBegin");
                let om_begin_for_handler = om_begin.clone();
                self.defer_or_handle(
                    move |interrupts| interrupts.push_exec_begin(seq, ev, Some(om_begin)),
                    move |this| {
                        // Finalize any active streaming sections, then establish
                        // the running Exec cell before flushing queued interrupts.
                        // This prevents an out‑of‑order ExecCommandEnd from being
                        // applied first (which would fall back to showing call_id).
                        this.finalize_active_stream();
                        tracing::info!(
                            "[order] ExecCommandBegin call_id={} seq={}",
                            ev2.call_id,
                            seq
                        );
                        this.handle_exec_begin_now(ev2.clone(), &om_begin_for_handler);
                        // If an ExecEnd for this call_id arrived earlier and is waiting,
                        // apply it immediately now that we have a matching Begin.
                        if let Some((pending_end, order2, _ts)) = this
                            .exec
                            .pending_exec_ends
                            .remove(&ExecCallId(ev2.call_id.clone()))
                        {
                            // Use the same order for the pending end
                            this.handle_exec_end_now(pending_end, &order2);
                        }
                        this.flush_interrupt_queue();
                    },
                );
            }
            EventMsg::ExecCommandOutputDelta(ev) => {
                let call_id = ExecCallId(ev.call_id.clone());
                if let Some(running) = self.exec.running_commands.get_mut(&call_id) {
                    let chunk = String::from_utf8_lossy(&ev.chunk).to_string();
                    match ev.stream {
                        ExecOutputStream::Stdout => running.stdout.push_str(&chunk),
                        ExecOutputStream::Stderr => running.stderr.push_str(&chunk),
                    }
                    if let Some(idx) = running.history_index
                        && idx < self.history_cells.len()
                        && let Some(exec) = self.history_cells[idx]
                            .as_any_mut()
                            .downcast_mut::<history_cell::ExecCell>()
                    {
                        exec.update_stream_preview(&running.stdout, &running.stderr);
                    }
                    self.invalidate_height_cache();
                    self.autoscroll_if_near_bottom();
                    self.request_redraw();
                }
            }
            EventMsg::PatchApplyBegin(PatchApplyBeginEvent {
                call_id,
                auto_approved,
                changes,
            }) => {
                let exec_call_id = ExecCallId(call_id.clone());
                self.exec.suppress_exec_end(exec_call_id);
                // Store for session diff popup (clone before moving into history)
                self.diffs.session_patch_sets.push(changes.clone());
                // Capture/adjust baselines, including rename moves
                if let Some(last) = self.diffs.session_patch_sets.last() {
                    for (src_path, chg) in last.iter() {
                        match chg {
                            codex_core::protocol::FileChange::Update {
                                move_path: Some(dest_path),
                                ..
                            } => {
                                // Prefer to carry forward existing baseline from src to dest.
                                if let Some(baseline) =
                                    self.diffs.baseline_file_contents.remove(src_path)
                                {
                                    self.diffs
                                        .baseline_file_contents
                                        .insert(dest_path.clone(), baseline);
                                } else if !self.diffs.baseline_file_contents.contains_key(dest_path)
                                {
                                    // Fallback: snapshot current contents of src (pre-apply) under dest key.
                                    let baseline =
                                        std::fs::read_to_string(src_path).unwrap_or_default();
                                    self.diffs
                                        .baseline_file_contents
                                        .insert(dest_path.clone(), baseline);
                                }
                            }
                            _ => {
                                if !self.diffs.baseline_file_contents.contains_key(src_path) {
                                    let baseline =
                                        std::fs::read_to_string(src_path).unwrap_or_default();
                                    self.diffs
                                        .baseline_file_contents
                                        .insert(src_path.clone(), baseline);
                                }
                            }
                        }
                    }
                }
                // Enable Ctrl+D footer hint now that we have diffs to show
                self.bottom_pane.set_diffs_hint(true);
                // Strict order
                let ok = match event.order.as_ref() {
                    Some(om) => Self::order_key_from_order_meta(om),
                    None => {
                        tracing::warn!("missing OrderMeta on ExecEnd flush; using synthetic key");
                        self.next_internal_key()
                    }
                };
                let cell = history_cell::new_patch_event(
                    PatchEventType::ApplyBegin { auto_approved },
                    changes,
                );
                let _ = self.history_insert_with_key_global(Box::new(cell), ok);
            }
            EventMsg::PatchApplyEnd(ev) => {
                let ev2 = ev.clone();
                self.defer_or_handle(
                    move |interrupts| interrupts.push_patch_end(event.event_seq, ev),
                    |this| this.handle_patch_apply_end_now(ev2),
                );
            }
            EventMsg::ExecCommandEnd(ev) => {
                let ev2 = ev.clone();
                let seq = event.event_seq;
                let order_meta_end = event
                    .order
                    .clone()
                    .expect("missing OrderMeta for ExecCommandEnd");
                let om_for_send = order_meta_end.clone();
                let om_for_insert = order_meta_end.clone();
                self.defer_or_handle(
                    move |interrupts| interrupts.push_exec_end(seq, ev, Some(om_for_send)),
                    move |this| {
                        tracing::info!(
                            "[order] ExecCommandEnd call_id={} seq={}",
                            ev2.call_id,
                            seq
                        );
                        // If we already have a running command for this call_id, finish it now.
                        let has_running = this
                            .exec
                            .running_commands
                            .contains_key(&ExecCallId(ev2.call_id.clone()));
                        if has_running {
                            this.handle_exec_end_now(ev2, &order_meta_end);
                        } else {
                            // Otherwise, stash it briefly and schedule a flush in case the
                            // matching Begin arrives shortly. This avoids rendering a fallback
                            // "call_<id>" cell when events are slightly out of order.
                            this.exec.pending_exec_ends.insert(
                                ExecCallId(ev2.call_id.clone()),
                                (ev2, om_for_insert, std::time::Instant::now()),
                            );
                            let tx = this.app_event_tx.clone();
                            std::thread::spawn(move || {
                                std::thread::sleep(std::time::Duration::from_millis(120));
                                tx.send(crate::app_event::AppEvent::FlushPendingExecEnds);
                            });
                        }
                    },
                );
            }
            EventMsg::McpToolCallBegin(ev) => {
                let ev2 = ev.clone();
                let seq = event.event_seq;
                let order_ok = match event.order.as_ref() {
                    Some(om) => Self::order_key_from_order_meta(om),
                    None => {
                        tracing::warn!("missing OrderMeta on McpBegin; using synthetic key");
                        self.next_internal_key()
                    }
                };
                self.defer_or_handle(
                    move |interrupts| interrupts.push_mcp_begin(seq, ev, event.order.clone()),
                    |this| {
                        this.finalize_active_stream();
                        this.flush_interrupt_queue();
                        tracing::info!(
                            "[order] McpToolCallBegin call_id={} seq={}",
                            ev2.call_id,
                            seq
                        );
                        tools::mcp_begin(this, ev2, order_ok);
                    },
                );
            }
            EventMsg::McpToolCallEnd(ev) => {
                let ev2 = ev.clone();
                let seq = event.event_seq;
                let order_ok = match event.order.as_ref() {
                    Some(om) => Self::order_key_from_order_meta(om),
                    None => {
                        tracing::warn!("missing OrderMeta on McpEnd; using synthetic key");
                        self.next_internal_key()
                    }
                };
                self.defer_or_handle(
                    move |interrupts| interrupts.push_mcp_end(seq, ev, event.order.clone()),
                    |this| {
                        tracing::info!(
                            "[order] McpToolCallEnd call_id={} seq={}",
                            ev2.call_id,
                            seq
                        );
                        tools::mcp_end(this, ev2, order_ok)
                    },
                );
            }
            EventMsg::CustomToolCallBegin(CustomToolCallBeginEvent {
                call_id,
                tool_name,
                parameters,
            }) => {
                // Any custom tool invocation should fade out the welcome animation
                for cell in &self.history_cells {
                    cell.trigger_fade();
                }
                self.finalize_active_stream();
                // Flush any queued interrupts when streaming ends
                self.flush_interrupt_queue();
                // Show an active entry immediately for all custom tools so the user sees progress
                let params_string = parameters.map(|p| p.to_string());
                if tool_name == "wait"
                    && let Some(exec_call_id) =
                        wait_exec_call_id_from_params(params_string.as_ref())
                {
                    self.tools_state
                        .running_wait_tools
                        .insert(ToolCallId(call_id.clone()), exec_call_id.clone());

                    if let Some(running) = self.exec.running_commands.get_mut(&exec_call_id) {
                        running.wait_active = true;
                        running.wait_notes.clear();
                        let history_index = running.history_index;
                        if let Some(idx) = history_index
                            && idx < self.history_cells.len()
                            && let Some(exec_cell) = self.history_cells[idx]
                                .as_any_mut()
                                .downcast_mut::<history_cell::ExecCell>()
                        {
                            exec_cell.set_waiting(true);
                            exec_cell.clear_wait_notes();
                        }
                    }
                    self.bottom_pane
                        .update_status_text("waiting for command".to_string());
                    self.invalidate_height_cache();
                    self.request_redraw();
                    return;
                }
                if tool_name == "kill"
                    && let Some(exec_call_id) =
                        wait_exec_call_id_from_params(params_string.as_ref())
                {
                    self.tools_state
                        .running_kill_tools
                        .insert(ToolCallId(call_id.clone()), exec_call_id);
                    self.bottom_pane
                        .update_status_text("cancelling command".to_string());
                    self.invalidate_height_cache();
                    self.request_redraw();
                    return;
                }
                // Animated running cell with live timer and formatted args
                let cell = if tool_name.starts_with("browser_") {
                    history_cell::new_running_browser_tool_call(
                        tool_name.clone(),
                        params_string.clone(),
                    )
                } else {
                    history_cell::new_running_custom_tool_call(
                        tool_name.clone(),
                        params_string.clone(),
                    )
                };
                // Enforce ordering for custom tool begin
                let ok = match event.order.as_ref() {
                    Some(om) => Self::order_key_from_order_meta(om),
                    None => {
                        tracing::warn!(
                            "missing OrderMeta on CustomToolCallBegin; using synthetic key"
                        );
                        self.next_internal_key()
                    }
                };
                let idx = self.history_insert_with_key_global(Box::new(cell), ok);
                // Track index so we can replace it on completion
                if idx < self.history_cells.len() {
                    self.tools_state
                        .running_custom_tools
                        .insert(ToolCallId(call_id.clone()), RunningToolEntry::new(ok, idx));
                }

                // Update border status based on tool
                if tool_name.starts_with("browser_") {
                    self.bottom_pane
                        .update_status_text("using browser".to_string());
                } else if tool_name.starts_with("agent_") {
                    self.bottom_pane
                        .update_status_text("agents coordinating".to_string());
                } else {
                    self.bottom_pane
                        .update_status_text(format!("using tool: {}", tool_name));
                }
            }
            EventMsg::CustomToolCallEnd(CustomToolCallEndEvent {
                call_id,
                tool_name,
                parameters,
                duration,
                result,
            }) => {
                let ok = match event.order.as_ref() {
                    Some(om) => Self::order_key_from_order_meta(om),
                    None => {
                        tracing::warn!(
                            "missing OrderMeta on CustomToolCallEnd; using synthetic key"
                        );
                        self.next_internal_key()
                    }
                };
                tracing::info!(
                    "[order] CustomToolCallEnd call_id={} tool={} seq={}",
                    call_id,
                    tool_name,
                    event.event_seq
                );
                // Convert parameters to String if present
                let params_string = parameters.map(|p| p.to_string());
                // Determine success and content from Result
                let (success, content) = match result {
                    Ok(content) => (true, content),
                    Err(error) => (false, error),
                };
                if tool_name == "wait"
                    && let Some(exec_call_id) = self
                        .tools_state
                        .running_wait_tools
                        .remove(&ToolCallId(call_id.clone()))
                {
                    let trimmed = content.trim();
                    let wait_still_pending = !success && trimmed != "Cancelled by user.";
                    let mut note_lines: Vec<(String, bool)> = Vec::new();
                    let suppress_json_notes = serde_json::from_str::<serde_json::Value>(trimmed)
                        .ok()
                        .and_then(|value| {
                            value.as_object().map(|obj| {
                                obj.contains_key("output") || obj.contains_key("metadata")
                            })
                        })
                        .unwrap_or(false);
                    if !suppress_json_notes {
                        for line in content.lines() {
                            let note_text = line.trim();
                            if note_text.is_empty() {
                                continue;
                            }
                            let is_error_note = note_text == "Cancelled by user.";
                            note_lines.push((note_text.to_string(), is_error_note));
                        }
                    }
                    let mut history_index: Option<usize> = None;
                    if let Some(running) = self.exec.running_commands.get_mut(&exec_call_id) {
                        let base = running.wait_total.unwrap_or_default();
                        let total = base.saturating_add(duration);
                        running.wait_total = Some(total);
                        history_index = running.history_index;
                        running.wait_active = wait_still_pending;
                        for (text, is_error_note) in &note_lines {
                            if running
                                .wait_notes
                                .last()
                                .map(|(existing, existing_err)| {
                                    existing == text && existing_err == is_error_note
                                })
                                .unwrap_or(false)
                            {
                                continue;
                            }
                            running.wait_notes.push((text.clone(), *is_error_note));
                        }
                    }

                    let mut updated = false;
                    if let Some(idx) = history_index
                        && idx < self.history_cells.len()
                        && let Some(exec_cell) = self.history_cells[idx]
                            .as_any_mut()
                            .downcast_mut::<history_cell::ExecCell>()
                    {
                        let total = exec_cell
                            .wait_total()
                            .unwrap_or_default()
                            .saturating_add(duration);
                        exec_cell.set_wait_total(Some(total));
                        if wait_still_pending {
                            exec_cell.set_waiting(true);
                        } else {
                            exec_cell.set_waiting(false);
                        }
                        for (text, is_error_note) in &note_lines {
                            exec_cell.push_wait_note(text, *is_error_note);
                        }
                        updated = true;
                    }
                    if !updated
                        && let Some(exec_cell) =
                            self.history_cells.iter_mut().rev().find_map(|cell| {
                                cell.as_any_mut().downcast_mut::<history_cell::ExecCell>()
                            })
                    {
                        let total = exec_cell
                            .wait_total()
                            .unwrap_or_default()
                            .saturating_add(duration);
                        exec_cell.set_wait_total(Some(total));
                        if wait_still_pending {
                            exec_cell.set_waiting(true);
                        } else {
                            exec_cell.set_waiting(false);
                        }
                        for (text, is_error_note) in &note_lines {
                            exec_cell.push_wait_note(text, *is_error_note);
                        }
                    }

                    if success {
                        self.remove_background_completion_message(&call_id);
                        self.bottom_pane
                            .update_status_text("responding".to_string());
                        self.maybe_hide_spinner();
                    } else if trimmed == "Cancelled by user." {
                        self.bottom_pane
                            .update_status_text("wait cancelled".to_string());
                    } else {
                        self.bottom_pane
                            .update_status_text("waiting for command".to_string());
                    }
                    self.invalidate_height_cache();
                    self.request_redraw();
                    return;
                }
                let running_entry = self
                    .tools_state
                    .running_custom_tools
                    .remove(&ToolCallId(call_id.clone()));
                let resolved_idx = running_entry
                    .as_ref()
                    .and_then(|entry| self.resolve_running_tool_index(entry));

                if tool_name == "apply_patch" && success {
                    if let Some(idx) = resolved_idx
                        && idx < self.history_cells.len()
                    {
                        let is_running_tool = self.history_cells[idx]
                            .as_any()
                            .downcast_ref::<history_cell::RunningToolCallCell>()
                            .is_some();
                        if is_running_tool {
                            self.history_remove_at(idx);
                        }
                    }
                    self.bottom_pane
                        .update_status_text("responding".to_string());
                    self.maybe_hide_spinner();
                    return;
                }

                if tool_name == "wait" && success {
                    let target = wait_target_from_params(params_string.as_ref(), &call_id);
                    let wait_cell = history_cell::new_completed_wait_tool_call(target, duration);
                    if let Some(idx) = resolved_idx {
                        self.history_replace_at(idx, Box::new(wait_cell));
                    } else {
                        let _ = self.history_insert_with_key_global(Box::new(wait_cell), ok);
                    }
                    self.remove_background_completion_message(&call_id);
                    self.bottom_pane
                        .update_status_text("responding".to_string());
                    self.maybe_hide_spinner();
                    return;
                }
                if tool_name == "wait" && !success && content.trim() == "Cancelled by user." {
                    let wait_cancelled_cell = PlainHistoryCell::new(
                        vec![Line::styled(
                            "Wait cancelled",
                            Style::default()
                                .fg(crate::colors::error())
                                .add_modifier(Modifier::BOLD),
                        )],
                        HistoryCellType::Error,
                    );

                    if let Some(idx) = resolved_idx {
                        self.history_replace_at(idx, Box::new(wait_cancelled_cell));
                    } else {
                        let _ =
                            self.history_insert_with_key_global(Box::new(wait_cancelled_cell), ok);
                    }

                    self.bottom_pane
                        .update_status_text("responding".to_string());
                    self.maybe_hide_spinner();
                    return;
                }
                if tool_name == "kill" {
                    let _ = self
                        .tools_state
                        .running_kill_tools
                        .remove(&ToolCallId(call_id.clone()));
                    if success {
                        self.remove_background_completion_message(&call_id);
                        self.bottom_pane
                            .update_status_text("responding".to_string());
                    } else {
                        let trimmed = content.trim();
                        if !trimmed.is_empty() {
                            self.push_background_tail(trimmed.to_string());
                        }
                        self.bottom_pane
                            .update_status_text("kill failed".to_string());
                    }
                    self.maybe_hide_spinner();
                    self.invalidate_height_cache();
                    self.request_redraw();
                    return;
                }
                // Special-case web_fetch to render returned markdown nicely.
                if tool_name == "web_fetch" {
                    let completed = history_cell::new_completed_web_fetch_tool_call(
                        &self.config,
                        params_string,
                        duration,
                        success,
                        content,
                    );
                    if let Some(idx) = resolved_idx {
                        self.history_replace_at(idx, Box::new(completed));
                    } else {
                        let _ = self.history_insert_with_key_global(Box::new(completed), ok);
                    }

                    // After tool completes, likely transitioning to response
                    self.bottom_pane
                        .update_status_text("responding".to_string());
                    self.maybe_hide_spinner();
                    return;
                }
                let completed = history_cell::new_completed_custom_tool_call(
                    tool_name,
                    params_string,
                    duration,
                    success,
                    content,
                );
                if let Some(idx) = resolved_idx {
                    self.history_replace_at(idx, Box::new(completed));
                } else {
                    let _ = self.history_insert_with_key_global(Box::new(completed), ok);
                }

                // After tool completes, likely transitioning to response
                self.bottom_pane
                    .update_status_text("responding".to_string());
                self.maybe_hide_spinner();
            }
            EventMsg::GetHistoryEntryResponse(event) => {
                let codex_core::protocol::GetHistoryEntryResponseEvent {
                    offset,
                    log_id,
                    entry,
                } = event;

                // Inform bottom pane / composer.
                self.bottom_pane
                    .on_history_entry_response(log_id, offset, entry.map(|e| e.text));
            }
            EventMsg::ShutdownComplete => {
                self.push_background_tail("🟡 ShutdownComplete".to_string());
                self.app_event_tx.send(AppEvent::ExitRequest);
            }
            EventMsg::TurnDiff(TurnDiffEvent { unified_diff }) => {
                info!("TurnDiffEvent: {unified_diff}");
            }
            EventMsg::BackgroundEvent(BackgroundEventEvent { message }) => {
                info!("BackgroundEvent: {message}");
                // Route through unified system notice helper. If the core ties the
                // event to a turn (order present), prefer placing it before the next
                // provider output; else append to the tail. Use the event.id for
                // in-place replacement.
                let placement = if event.order.as_ref().is_some() {
                    SystemPlacement::EarlyInCurrent
                } else {
                    SystemPlacement::EndOfCurrent
                };
                let id_for_replace = Some(id.clone());
                self.push_system_cell(
                    history_cell::new_background_event(message.clone()),
                    placement,
                    id_for_replace,
                    event.order.as_ref(),
                    "background",
                );
                // If we inserted during streaming, keep the reasoning ellipsis visible.
                self.restore_reasoning_in_progress_if_streaming();

                // Also reflect CDP connect success in the status line.
                if message.starts_with("✅ Connected to Chrome via CDP") {
                    self.bottom_pane
                        .update_status_text("using browser (CDP)".to_string());
                }
            }
            EventMsg::AgentStatusUpdate(AgentStatusUpdateEvent {
                agents,
                context,
                task,
            }) => {
                tracing::warn!(
                    "DEBUG: AgentStatusUpdate event received, {} agents",
                    agents.len()
                );
                // Update the active agents list from the event and track timing
                self.active_agents.clear();
                let now = Instant::now();
                for agent in agents.iter() {
                    let parsed_status = agent_status_from_str(agent.status.as_str());
                    // Update runtime map
                    let entry = self.agent_runtime.entry(agent.id.clone()).or_default();
                    entry.last_update = Some(now);
                    match parsed_status {
                        AgentStatus::Running => {
                            if entry.started_at.is_none() {
                                entry.started_at = Some(now);
                            }
                        }
                        AgentStatus::Completed | AgentStatus::Failed => {
                            if entry.completed_at.is_none() {
                                entry.completed_at = entry.completed_at.or(Some(now));
                            }
                        }
                        _ => {}
                    }

                    // Mirror agent list for rendering
                    self.active_agents.push(AgentInfo {
                        id: agent.id.clone(),
                        name: agent.name.clone(),
                        status: parsed_status.clone(),
                        batch_id: agent.batch_id.clone(),
                        model: agent.model.clone(),
                        result: agent.result.clone(),
                        error: agent.error.clone(),
                        last_progress: agent.last_progress.clone(),
                    });
                }

                spec_kit::handler::record_agent_costs(self, &agents);

                self.update_agents_terminal_state(&agents, context.clone(), task.clone());

                // Store shared context and task
                self.agent_context = context;
                self.agent_task = task;

                // Fallback: if every agent we know about has reached a terminal state and
                // there is no active streaming or tooling, clear the spinner even if the
                // backend hasn't sent TaskComplete yet. This prevents the footer from
                // getting stuck on "Responding..." after multi-agent runs that yield
                // early.
                if self.bottom_pane.is_task_running() {
                    let all_agents_terminal = !self.agent_runtime.is_empty()
                        && self
                            .agent_runtime
                            .values()
                            .all(|rt| rt.completed_at.is_some());
                    tracing::warn!(
                        "DEBUG: Agent terminal check - all_terminal={}, runtime_count={}",
                        all_agents_terminal,
                        self.agent_runtime.len()
                    );
                    if all_agents_terminal {
                        let any_tools_running = !self.exec.running_commands.is_empty()
                            || !self.tools_state.running_custom_tools.is_empty()
                            || !self.tools_state.running_web_search.is_empty();
                        let any_streaming = self.stream.is_write_cycle_active();
                        tracing::warn!(
                            "DEBUG: Tools running={}, streaming={}",
                            any_tools_running,
                            any_streaming
                        );

                        // Log completion check for spec-auto observability
                        if let Some(state) = self.spec_auto_state.as_ref()
                            && let Some(run_id) = &state.run_id
                            && let Some(stage) = state.current_stage()
                        {
                            let completed_count = self
                                .active_agents
                                .iter()
                                .filter(|a| {
                                    matches!(a.status, crate::chatwidget::AgentStatus::Completed)
                                })
                                .count();

                            state.execution_logger.log_event(
                                spec_kit::execution_logger::ExecutionEvent::CompletionCheck {
                                    run_id: run_id.clone(),
                                    stage: stage.display_name().to_string(),
                                    all_agents_terminal,
                                    tools_running: any_tools_running,
                                    streaming_active: any_streaming,
                                    will_proceed: !(any_tools_running || any_streaming),
                                    agent_count: self.agent_runtime.len(),
                                    completed_count,
                                    timestamp: spec_kit::execution_logger::ExecutionEvent::now(),
                                },
                            );
                        }

                        if !(any_tools_running || any_streaming) {
                            tracing::warn!(
                                "DEBUG: All agents terminal, no tools/streaming, calling spec_kit completion handler"
                            );
                            self.bottom_pane.set_task_running(false);
                            self.bottom_pane.update_status_text(String::new());

                            // NEW: Check if this is part of spec-auto pipeline
                            tracing::warn!(
                                "DEBUG: About to call spec_kit::on_spec_auto_agents_complete"
                            );
                            spec_kit::on_spec_auto_agents_complete(self);
                            tracing::warn!(
                                "DEBUG: Returned from spec_kit::on_spec_auto_agents_complete"
                            );
                            self.finish_manual_validate_runs_if_idle();
                        }
                    }
                }

                // Update overall task status based on agent states
                self.overall_task_status = if self.active_agents.is_empty() {
                    "preparing".to_string()
                } else if self
                    .active_agents
                    .iter()
                    .any(|a| matches!(a.status, AgentStatus::Running))
                {
                    "running".to_string()
                } else if self
                    .active_agents
                    .iter()
                    .all(|a| matches!(a.status, AgentStatus::Completed))
                {
                    "complete".to_string()
                } else if self
                    .active_agents
                    .iter()
                    .any(|a| matches!(a.status, AgentStatus::Failed))
                {
                    "failed".to_string()
                } else {
                    "planning".to_string()
                };

                // Reflect concise agent status in the input border
                let count = self.active_agents.len();
                let msg = match self.overall_task_status.as_str() {
                    "preparing" => format!("agents: preparing ({} ready)", count),
                    "running" => format!("agents: running ({})", count),
                    "complete" => format!("agents: complete ({} ok)", count),
                    "failed" => "agents: failed".to_string(),
                    _ => "agents: planning".to_string(),
                };
                self.bottom_pane.update_status_text(msg);

                // Keep agents visible after completion so users can see final messages/errors.
                // HUD will be reset automatically when a new agent batch starts.

                // Reset ready to start flag when we get actual agent updates
                if !self.active_agents.is_empty() {
                    self.agents_ready_to_start = false;
                }
                // Re-evaluate spinner visibility now that agent states changed.
                self.maybe_hide_spinner();
                self.request_redraw();
            }
            // Newer protocol variants we currently ignore in the TUI
            EventMsg::BrowserScreenshotUpdate(_) => {}
            EventMsg::UserMessage(_) => {}
            EventMsg::TurnAborted(_) => {}
            EventMsg::ConversationPath(_) => {}
            EventMsg::EnteredReviewMode(review_request) => {
                let hint = review_request.user_facing_hint.trim();
                let banner = if hint.is_empty() {
                    ">> Code review started <<".to_string()
                } else {
                    format!(">> Code review started: {hint} <<")
                };
                self.active_review_hint = Some(review_request.user_facing_hint.clone());
                self.active_review_prompt = Some(review_request.prompt.clone());
                self.push_background_before_next_output(banner);

                let prompt_text = review_request.prompt.trim();
                if !prompt_text.is_empty() {
                    let mut lines: Vec<Line<'static>> = Vec::new();
                    lines.push(Line::from(vec![RtSpan::styled(
                        "Review focus",
                        Style::default().add_modifier(Modifier::BOLD),
                    )]));
                    lines.push(Line::from(""));
                    for line in prompt_text.lines() {
                        lines.push(Line::from(line.to_string()));
                    }
                    self.history_push(history_cell::PlainHistoryCell::new(
                        lines,
                        history_cell::HistoryCellType::Notice,
                    ));
                }
                self.request_redraw();
            }
            EventMsg::ExitedReviewMode(review_output) => {
                let hint = self.active_review_hint.take();
                let prompt = self.active_review_prompt.take();
                match review_output {
                    Some(output) => {
                        let summary_cell = self.build_review_summary_cell(
                            hint.as_deref(),
                            prompt.as_deref(),
                            &output,
                        );
                        self.history_push(summary_cell);
                        let finish_banner = match hint.as_deref() {
                            Some(h) if !h.trim().is_empty() => {
                                let trimmed = h.trim();
                                format!("<< Code review finished: {trimmed} >>")
                            }
                            _ => "<< Code review finished >>".to_string(),
                        };
                        self.push_background_tail(finish_banner);
                    }
                    None => {
                        let banner = match hint.as_deref() {
                            Some(h) if !h.trim().is_empty() => {
                                let trimmed = h.trim();
                                format!(
                                    "<< Code review finished without a final response ({trimmed}) >>"
                                )
                            }
                            _ => "<< Code review finished without a final response >>".to_string(),
                        };
                        self.push_background_tail(banner);
                        self.history_push(history_cell::new_warning_event(
                            "Review session ended without returning findings. Try `/review` again if you still need feedback.".to_string(),
                        ));
                    }
                }
                self.request_redraw();
            }
            // New event variants - no-op in TUI for now
            EventMsg::UndoStarted(_)
            | EventMsg::UndoCompleted(_)
            | EventMsg::ListSkillsResponse(_) => {}
        }
    }
}

// --- Phase 3: reasoning/visibility methods ---

impl ChatWidget<'_> {
    pub(super) fn normalize_text(s: &str) -> String {
        // 1) Normalize newlines
        let s = s.replace("\r\n", "\n");
        // 2) Trim trailing whitespace per line; collapse repeated blank lines
        let mut out: Vec<String> = Vec::new();
        let mut saw_blank = false;
        for line in s.lines() {
            // Replace common Unicode bullets with ASCII to stabilize equality checks
            let line = line.replace(['\u{2022}', '\u{25E6}', '\u{2219}'], "-"); // ∙
            let trimmed = line.trim_end();
            if trimmed.chars().all(|c| c.is_whitespace()) {
                if !saw_blank {
                    out.push(String::new());
                }
                saw_blank = true;
            } else {
                out.push(trimmed.to_string());
                saw_blank = false;
            }
        }
        // 3) Remove trailing blank lines
        while out.last().is_some_and(|l| l.is_empty()) {
            out.pop();
        }
        out.join("\n")
    }

    pub(crate) fn toggle_reasoning_visibility(&mut self) {
        // Track whether any reasoning cells are found and their new state
        let mut has_reasoning_cells = false;
        let mut new_collapsed_state = false;

        // Toggle all CollapsibleReasoningCell instances in history
        for cell in &self.history_cells {
            // Try to downcast to CollapsibleReasoningCell
            if let Some(reasoning_cell) = cell
                .as_any()
                .downcast_ref::<history_cell::CollapsibleReasoningCell>()
            {
                reasoning_cell.toggle_collapsed();
                has_reasoning_cells = true;
                new_collapsed_state = reasoning_cell.is_collapsed();
            }
        }

        // Update the config to reflect the current state (inverted because collapsed means hidden)
        if has_reasoning_cells {
            self.config.tui.show_reasoning = !new_collapsed_state;
            // Brief status to confirm the toggle to the user
            let status = if self.config.tui.show_reasoning {
                "Reasoning shown"
            } else {
                "Reasoning hidden"
            };
            self.bottom_pane.update_status_text(status.to_string());
            // Update footer label to reflect current state
            self.bottom_pane
                .set_reasoning_state(self.config.tui.show_reasoning);
        } else {
            // No reasoning cells exist; inform the user
            self.bottom_pane
                .update_status_text("No reasoning to toggle".to_string());
        }
        self.refresh_reasoning_collapsed_visibility();
        // Collapsed state changes affect heights; clear cache
        self.invalidate_height_cache();
        self.request_redraw();
        // In standard terminal mode, re-mirror the transcript so scrollback reflects
        // the new collapsed/expanded state. We cannot edit prior lines in scrollback,
        // so append a fresh view.
        if self.standard_terminal_mode {
            let mut lines = Vec::new();
            lines.push(ratatui::text::Line::from(""));
            lines.extend(self.export_transcript_lines_for_buffer());
            self.app_event_tx
                .send(crate::app_event::AppEvent::InsertHistory(lines));
        }
    }

    fn refresh_standard_terminal_hint(&mut self) {
        if self.standard_terminal_mode {
            let message = "Standard terminal mode active. Press Ctrl+T to return to full UI.";
            self.bottom_pane
                .set_standard_terminal_hint(Some(message.to_string()));
        } else {
            self.bottom_pane.set_standard_terminal_hint(None);
        }
    }

    pub(crate) fn set_standard_terminal_mode(&mut self, enabled: bool) {
        self.standard_terminal_mode = enabled;
        self.refresh_standard_terminal_hint();
    }

    pub(crate) fn is_reasoning_shown(&self) -> bool {
        // Check if any reasoning cell exists and if it's expanded
        for cell in &self.history_cells {
            if let Some(reasoning_cell) = cell
                .as_any()
                .downcast_ref::<history_cell::CollapsibleReasoningCell>()
            {
                return !reasoning_cell.is_collapsed();
            }
        }
        // If no reasoning cells exist, return the config default
        self.config.tui.show_reasoning
    }
}
