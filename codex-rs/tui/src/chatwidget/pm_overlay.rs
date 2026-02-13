//! SPEC-PM-004: PM overview overlay
//!
//! Read-only list view showing a hierarchical tree of work items
//! (Project > Feature > Spec > Task) with adaptive columns, summary bar, and
//! degraded-mode detection.

use std::cell::Cell;
use std::collections::HashSet;

use ratatui::buffer::Buffer;
use ratatui::layout::{Margin, Rect};
use ratatui::style::Style;
use ratatui::text::{Line as RLine, Span};
use ratatui::widgets::{Block, Borders, Clear, Widget};

use crate::colors;
use crate::util::buffer::fill_rect;

use super::ChatWidget;

// ---------------------------------------------------------------------------
// State wrapper (follows LimitsState / ProState pattern)
// ---------------------------------------------------------------------------

#[derive(Default)]
pub(super) struct PmState {
    pub(super) overlay: Option<PmOverlay>,
}

// ---------------------------------------------------------------------------
// Overlay data
// ---------------------------------------------------------------------------

pub(super) struct PmOverlay {
    scroll: Cell<u16>,
    max_scroll: Cell<u16>,
    visible_rows: Cell<u16>,
    selected: Cell<usize>,
    expanded: std::cell::RefCell<HashSet<usize>>,
    nodes: Vec<TreeNode>,
    degraded: bool,
    /// When `Some(node_idx)`, the overlay shows a read-only detail view for
    /// that node instead of the tree list.
    detail_node_idx: Cell<Option<usize>>,
    detail_scroll: Cell<u16>,
    detail_max_scroll: Cell<u16>,
    detail_visible_rows: Cell<u16>,
}

#[derive(Clone, Copy, PartialEq, Eq)]
#[allow(dead_code)] // Variants stored for future detail view / filtering
enum NodeType {
    Project,
    Feature,
    Spec,
    Task,
}

#[allow(dead_code)] // Fields used across rendering + future detail view
struct TreeNode {
    id: String,
    title: String,
    node_type: NodeType,
    state: String,
    updated_at: String,
    latest_run: String,
    depth: u16,
    parent_idx: Option<usize>,
    children: Vec<usize>,
}

// ---------------------------------------------------------------------------
// Demo dataset
// ---------------------------------------------------------------------------

fn demo_tree() -> Vec<TreeNode> {
    let mut nodes: Vec<TreeNode> = Vec::new();

    // Project 0
    nodes.push(TreeNode {
        id: "PROJ-001".into(),
        title: "Spec-Kit Platform".into(),
        node_type: NodeType::Project,
        state: String::new(),
        updated_at: "2026-02-12T10:30:00Z".into(),
        latest_run: String::new(),
        depth: 0,
        parent_idx: None,
        children: vec![1, 8],
    });
    // Feature 1
    nodes.push(TreeNode {
        id: "FEAT-001".into(),
        title: "User Authentication".into(),
        node_type: NodeType::Feature,
        state: "InProgress".into(),
        updated_at: "2026-02-12T10:30:00Z".into(),
        latest_run: String::new(),
        depth: 1,
        parent_idx: Some(0),
        children: vec![2, 5],
    });
    // Spec 2
    nodes.push(TreeNode {
        id: "SPEC-AUTH-001".into(),
        title: "OAuth2 Integration".into(),
        node_type: NodeType::Spec,
        state: "NeedsReview (-> InProgress)".into(),
        updated_at: "2026-02-12T09:15:00Z".into(),
        latest_run: "run-042".into(),
        depth: 2,
        parent_idx: Some(1),
        children: vec![3, 4],
    });
    // Task 3
    nodes.push(TreeNode {
        id: "TASK-001".into(),
        title: "Implement token refresh".into(),
        node_type: NodeType::Task,
        state: "completed".into(),
        updated_at: "2026-02-11T14:00:00Z".into(),
        latest_run: "run-041".into(),
        depth: 3,
        parent_idx: Some(2),
        children: vec![],
    });
    // Task 4
    nodes.push(TreeNode {
        id: "TASK-002".into(),
        title: "Add PKCE flow".into(),
        node_type: NodeType::Task,
        state: "open".into(),
        updated_at: "2026-02-12T09:15:00Z".into(),
        latest_run: "run-042".into(),
        depth: 3,
        parent_idx: Some(2),
        children: vec![],
    });
    // Spec 5
    nodes.push(TreeNode {
        id: "SPEC-AUTH-002".into(),
        title: "Session Management".into(),
        node_type: NodeType::Spec,
        state: "Backlog".into(),
        updated_at: "2026-02-10T16:00:00Z".into(),
        latest_run: String::new(),
        depth: 2,
        parent_idx: Some(1),
        children: vec![6, 7],
    });
    // Task 6
    nodes.push(TreeNode {
        id: "TASK-003".into(),
        title: "Design session store".into(),
        node_type: NodeType::Task,
        state: "open".into(),
        updated_at: "2026-02-10T16:00:00Z".into(),
        latest_run: String::new(),
        depth: 3,
        parent_idx: Some(5),
        children: vec![],
    });
    // Task 7
    nodes.push(TreeNode {
        id: "TASK-004".into(),
        title: "Implement session expiry".into(),
        node_type: NodeType::Task,
        state: "failed".into(),
        updated_at: "2026-02-10T18:00:00Z".into(),
        latest_run: "run-039".into(),
        depth: 3,
        parent_idx: Some(5),
        children: vec![],
    });

    // Feature 8
    nodes.push(TreeNode {
        id: "FEAT-002".into(),
        title: "Pipeline Orchestration".into(),
        node_type: NodeType::Feature,
        state: "NeedsResearch (-> Backlog)".into(),
        updated_at: "2026-02-11T11:00:00Z".into(),
        latest_run: String::new(),
        depth: 1,
        parent_idx: Some(0),
        children: vec![9, 11],
    });
    // Spec 9
    nodes.push(TreeNode {
        id: "SPEC-PIPE-001".into(),
        title: "Stage execution engine".into(),
        node_type: NodeType::Spec,
        state: "Planned".into(),
        updated_at: "2026-02-11T11:00:00Z".into(),
        latest_run: "run-038".into(),
        depth: 2,
        parent_idx: Some(8),
        children: vec![10],
    });
    // Task 10
    nodes.push(TreeNode {
        id: "TASK-005".into(),
        title: "Implement retry logic".into(),
        node_type: NodeType::Task,
        state: "open".into(),
        updated_at: "2026-02-11T11:00:00Z".into(),
        latest_run: "run-038".into(),
        depth: 3,
        parent_idx: Some(9),
        children: vec![],
    });
    // Spec 11
    nodes.push(TreeNode {
        id: "SPEC-PIPE-002".into(),
        title: "Guardrail validation".into(),
        node_type: NodeType::Spec,
        state: "Completed".into(),
        updated_at: "2026-02-09T08:00:00Z".into(),
        latest_run: "run-035".into(),
        depth: 2,
        parent_idx: Some(8),
        children: vec![12, 13],
    });
    // Task 12
    nodes.push(TreeNode {
        id: "TASK-006".into(),
        title: "Schema validation".into(),
        node_type: NodeType::Task,
        state: "completed".into(),
        updated_at: "2026-02-09T08:00:00Z".into(),
        latest_run: "run-034".into(),
        depth: 3,
        parent_idx: Some(11),
        children: vec![],
    });
    // Task 13
    nodes.push(TreeNode {
        id: "TASK-007".into(),
        title: "Evidence checks".into(),
        node_type: NodeType::Task,
        state: "completed".into(),
        updated_at: "2026-02-08T17:30:00Z".into(),
        latest_run: "run-033".into(),
        depth: 3,
        parent_idx: Some(11),
        children: vec![],
    });

    nodes
}

// ---------------------------------------------------------------------------
// Construction
// ---------------------------------------------------------------------------

impl PmOverlay {
    pub(super) fn new(degraded: bool) -> Self {
        Self {
            scroll: Cell::new(0),
            max_scroll: Cell::new(0),
            visible_rows: Cell::new(0),
            selected: Cell::new(0),
            expanded: std::cell::RefCell::new(HashSet::new()),
            nodes: demo_tree(),
            degraded,
            detail_node_idx: Cell::new(None),
            detail_scroll: Cell::new(0),
            detail_max_scroll: Cell::new(0),
            detail_visible_rows: Cell::new(0),
        }
    }

    // -- Accessors used by pm_handlers ----------------------------------------

    pub(super) fn selected(&self) -> usize {
        self.selected.get()
    }

    pub(super) fn set_selected(&self, val: usize) {
        self.selected.set(val);
    }

    pub(super) fn scroll(&self) -> u16 {
        self.scroll.get()
    }

    pub(super) fn set_scroll(&self, val: u16) {
        self.scroll.set(val.min(self.max_scroll.get()));
    }

    pub(super) fn visible_rows(&self) -> u16 {
        self.visible_rows.get()
    }

    // -- Detail-mode accessors -------------------------------------------------

    pub(super) fn is_detail_mode(&self) -> bool {
        self.detail_node_idx.get().is_some()
    }

    /// Open detail view for the node at the given *visible-row* index.
    /// Returns `true` if detail was opened.
    pub(super) fn open_detail_for_visible(&self, flat_idx: usize) -> bool {
        match self.node_idx_of_visible(flat_idx) {
            Some(node_idx) => {
                self.detail_node_idx.set(Some(node_idx));
                self.detail_scroll.set(0);
                true
            }
            None => false,
        }
    }

    /// Close detail view, returning to the list.  List selection/scroll are
    /// preserved because we never touch them.
    pub(super) fn close_detail(&self) {
        self.detail_node_idx.set(None);
        self.detail_scroll.set(0);
    }

    /// The node currently shown in detail view, if any.
    fn detail_node(&self) -> Option<&TreeNode> {
        self.detail_node_idx.get().map(|i| &self.nodes[i])
    }

    pub(super) fn detail_scroll(&self) -> u16 {
        self.detail_scroll.get()
    }

    pub(super) fn set_detail_scroll(&self, val: u16) {
        self.detail_scroll
            .set(val.min(self.detail_max_scroll.get()));
    }

    pub(super) fn detail_visible_rows(&self) -> u16 {
        self.detail_visible_rows.get()
    }

    /// Toggle expand on the selected node (returns true if changed).
    #[allow(dead_code)] // Available for Enter key in future slice
    pub(super) fn toggle_expand(&self, idx: usize) -> bool {
        if idx >= self.nodes.len() || self.nodes[idx].children.is_empty() {
            return false;
        }
        let mut exp = self.expanded.borrow_mut();
        if exp.contains(&idx) {
            exp.remove(&idx);
        } else {
            exp.insert(idx);
        }
        true
    }

    pub(super) fn expand(&self, idx: usize) -> bool {
        if idx >= self.nodes.len() || self.nodes[idx].children.is_empty() {
            return false;
        }
        self.expanded.borrow_mut().insert(idx)
    }

    pub(super) fn collapse(&self, idx: usize) -> bool {
        self.expanded.borrow_mut().remove(&idx)
    }

    pub(super) fn is_expanded(&self, idx: usize) -> bool {
        self.expanded.borrow().contains(&idx)
    }

    /// Map a flat visible-row index to the underlying node index.
    pub(super) fn node_idx_of_visible(&self, flat_idx: usize) -> Option<usize> {
        self.visible_indices().get(flat_idx).copied()
    }

    /// Expand the node at the given visible-row index.
    pub(super) fn expand_visible(&self, flat_idx: usize) -> bool {
        match self.node_idx_of_visible(flat_idx) {
            Some(node_idx) => self.expand(node_idx),
            None => false,
        }
    }

    /// Collapse the node at the given visible-row index.
    pub(super) fn collapse_visible(&self, flat_idx: usize) -> bool {
        match self.node_idx_of_visible(flat_idx) {
            Some(node_idx) => self.collapse(node_idx),
            None => false,
        }
    }

    /// Check if the node at the given visible-row index is expanded.
    pub(super) fn is_expanded_visible(&self, flat_idx: usize) -> bool {
        match self.node_idx_of_visible(flat_idx) {
            Some(node_idx) => self.is_expanded(node_idx),
            None => false,
        }
    }

    /// Parent index for the node at `flat_idx` in the visible list.
    pub(super) fn parent_of_visible(&self, flat_idx: usize) -> Option<usize> {
        let visible = self.visible_indices();
        let node_idx = *visible.get(flat_idx)?;
        let parent_node = self.nodes[node_idx].parent_idx?;
        visible.iter().position(|&i| i == parent_node)
    }

    /// Number of items in the visible (expanded) list.
    pub(super) fn visible_count(&self) -> usize {
        self.visible_indices().len()
    }

    // -- Visible list computation ---------------------------------------------

    fn visible_indices(&self) -> Vec<usize> {
        let exp = self.expanded.borrow();
        let mut out = Vec::new();
        self.collect_visible(0, &exp, &mut out);
        out
    }

    fn collect_visible(&self, start: usize, exp: &HashSet<usize>, out: &mut Vec<usize>) {
        // Walk top-level roots (depth 0)
        for (i, node) in self.nodes.iter().enumerate() {
            if i < start {
                continue;
            }
            if node.depth == 0 {
                self.collect_subtree(i, exp, out);
            }
        }
    }

    fn collect_subtree(&self, idx: usize, exp: &HashSet<usize>, out: &mut Vec<usize>) {
        out.push(idx);
        if exp.contains(&idx) {
            for &child in &self.nodes[idx].children {
                self.collect_subtree(child, exp, out);
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Rendering (called from ChatWidget)
// ---------------------------------------------------------------------------

impl ChatWidget<'_> {
    pub(crate) fn open_pm_overlay(&mut self) {
        let degraded = check_service_status();
        self.pm.overlay = Some(PmOverlay::new(degraded));
        self.request_redraw();
    }

    pub(super) fn render_pm_overlay(&self, frame_area: Rect, history_area: Rect, buf: &mut Buffer) {
        let Some(overlay) = self.pm.overlay.as_ref() else {
            return;
        };

        // Scrim
        let scrim_style = Style::default()
            .bg(colors::overlay_scrim())
            .fg(colors::text_dim());
        fill_rect(buf, frame_area, None, scrim_style);

        // Overlay area
        let padding = 1u16;
        let overlay_area = Rect {
            x: history_area.x + padding,
            y: history_area.y,
            width: history_area.width.saturating_sub(padding * 2),
            height: history_area.height,
        };
        Clear.render(overlay_area, buf);

        // Branch on detail vs list mode
        let dim = Style::default().fg(colors::text_dim());
        let bright = Style::default().fg(colors::text());
        let accent = Style::default().fg(colors::function());

        let is_detail = overlay.is_detail_mode();

        let title = if is_detail {
            RLine::from(vec![
                Span::styled(" PM Detail ", bright),
                Span::styled("--- ", dim),
                Span::styled("Up/Dn", accent),
                Span::styled(" scroll  ", dim),
                Span::styled("Esc", bright),
                Span::styled(" back to list ", dim),
            ])
        } else {
            RLine::from(vec![
                Span::styled(" PM Overview ", bright),
                Span::styled("--- ", dim),
                Span::styled("Up/Dn", accent),
                Span::styled(" navigate  ", dim),
                Span::styled("Left/Right", accent),
                Span::styled(" expand/collapse  ", dim),
                Span::styled("Enter", accent),
                Span::styled(" detail  ", dim),
                Span::styled("Esc", bright),
                Span::styled(" close ", dim),
            ])
        };

        let block = Block::default()
            .borders(Borders::ALL)
            .title(title)
            .style(Style::default().bg(colors::background()))
            .border_style(
                Style::default()
                    .fg(colors::border())
                    .bg(colors::background()),
            );
        let inner = block.inner(overlay_area);
        block.render(overlay_area, buf);

        let body = inner.inner(Margin::new(1, 0));
        if body.width == 0 || body.height == 0 {
            overlay.visible_rows.set(0);
            overlay.max_scroll.set(0);
            return;
        }

        if is_detail {
            render_detail(overlay, body, buf);
        } else {
            // Summary bar (2 lines) + optional degraded banner (1 line)
            let degraded_lines: u16 = if overlay.degraded { 1 } else { 0 };
            let summary_height = 2 + degraded_lines;
            if body.height <= summary_height {
                overlay.visible_rows.set(0);
                overlay.max_scroll.set(0);
                return;
            }

            let summary_area = Rect {
                x: body.x,
                y: body.y,
                width: body.width,
                height: summary_height,
            };
            let list_area = Rect {
                x: body.x,
                y: body.y + summary_height,
                width: body.width,
                height: body.height - summary_height,
            };

            render_summary_bar(overlay, summary_area, buf);
            render_list(overlay, list_area, buf);
        }
    }
}

// ---------------------------------------------------------------------------
// Detail view (read-only, PM-UX-D6/D20)
// ---------------------------------------------------------------------------

/// Build all detail-view lines for `node`.  The caller handles scrolling.
fn detail_content_lines(node: &TreeNode, width: usize) -> Vec<RLine<'static>> {
    let dim = Style::default().fg(colors::text_dim());
    let bright = Style::default().fg(colors::text());
    let label_style = Style::default().fg(colors::info());

    let mut lines: Vec<RLine<'static>> = Vec::new();

    // --- Fixed header is rendered separately; these are the scrollable sections ---

    // ── Metadata ──────────────────────────────────────────────
    let sep = "\u{2500}".repeat(width.min(60)); // ─
    lines.push(RLine::from(Span::styled(
        format!("\u{2500}\u{2500} Metadata {sep}"),
        dim,
    )));

    let node_type_label = match node.node_type {
        NodeType::Project => "Project",
        NodeType::Feature => "Feature",
        NodeType::Spec => "Spec",
        NodeType::Task => "Task",
    };
    lines.push(RLine::from(vec![
        Span::styled("  Type:     ", label_style),
        Span::styled(node_type_label.to_string(), bright),
    ]));
    lines.push(RLine::from(vec![
        Span::styled("  Parent:   ", label_style),
        Span::styled(
            node.parent_idx
                .map_or_else(|| "(root)".to_string(), |p| format!("node {p}")),
            bright,
        ),
    ]));
    if matches!(node.node_type, NodeType::Feature) {
        lines.push(RLine::from(vec![
            Span::styled("  Priority: ", label_style),
            Span::styled("(not set)", dim),
        ]));
    }
    if matches!(node.node_type, NodeType::Spec) {
        lines.push(RLine::from(vec![
            Span::styled("  Quality:  ", label_style),
            Span::styled("(not set)", dim),
        ]));
        lines.push(RLine::from(vec![
            Span::styled("  PRD URI:  ", label_style),
            Span::styled("(none)", dim),
        ]));
    }
    lines.push(RLine::from(Span::styled("", dim)));

    // ── State Controls (disabled) ─────────────────────────────
    lines.push(RLine::from(Span::styled(
        format!("\u{2500}\u{2500} State Controls {sep}"),
        dim,
    )));
    lines.push(RLine::from(vec![
        Span::styled("  [F5] Promote   ", dim),
        Span::styled("[F6] Hold   ", dim),
        Span::styled("[F7] Complete", dim),
    ]));
    lines.push(RLine::from(Span::styled(
        "  (disabled \u{2014} read-only view)",
        dim,
    )));
    lines.push(RLine::from(Span::styled("", dim)));

    // ── Run History ───────────────────────────────────────────
    lines.push(RLine::from(Span::styled(
        format!("\u{2500}\u{2500} Run History {sep}"),
        dim,
    )));
    if node.latest_run.is_empty() {
        lines.push(RLine::from(Span::styled("  (no runs)", dim)));
    } else {
        // Header row
        lines.push(RLine::from(vec![
            Span::styled("  Run ID       ", label_style),
            Span::styled("Kind       ", label_style),
            Span::styled("Preset     ", label_style),
            Span::styled("Status     ", label_style),
            Span::styled("Started", label_style),
        ]));
        // Single demo row from latest_run
        lines.push(RLine::from(vec![
            Span::styled(format!("  {:<14}", node.latest_run), bright),
            Span::styled("review     ", dim),
            Span::styled("standard   ", dim),
            Span::styled("completed  ", dim),
            Span::styled(short_date(&node.updated_at), dim),
        ]));
    }
    lines.push(RLine::from(Span::styled("", dim)));

    // ── Checkpoints ───────────────────────────────────────────
    lines.push(RLine::from(Span::styled(
        format!("\u{2500}\u{2500} Checkpoints {sep}"),
        dim,
    )));
    lines.push(RLine::from(Span::styled("  (no active checkpoint)", dim)));
    lines.push(RLine::from(Span::styled("", dim)));

    lines
}

fn render_detail(overlay: &PmOverlay, area: Rect, buf: &mut Buffer) {
    let Some(node) = overlay.detail_node() else {
        return;
    };

    let dim = Style::default().fg(colors::text_dim());
    let bright = Style::default().fg(colors::text());

    let width = area.width as usize;

    // --- Fixed header (3 lines) -------------------------------------------
    let header_height: u16 = 3;
    if area.height <= header_height {
        overlay.detail_visible_rows.set(0);
        overlay.detail_max_scroll.set(0);
        return;
    }

    // Line 1: ID + title
    let id_title = format!("{} \u{2014} {}", node.id, node.title);
    let header1 = RLine::from(Span::styled(pad_or_trunc(&id_title, width), bright));
    buf.set_line(area.x, area.y, &header1, area.width);

    // Line 2: state + updated + latest run
    let state_style = Style::default()
        .fg(state_color(&node.state))
        .bg(colors::background());
    let state_text = if node.state.is_empty() {
        "(container)".to_string()
    } else {
        node.state.clone()
    };
    let updated_label = format!("  Updated: {}  ", short_date(&node.updated_at));
    let run_label = if node.latest_run.is_empty() {
        String::new()
    } else {
        format!("Latest run: {}", node.latest_run)
    };
    let header2 = RLine::from(vec![
        Span::styled(state_text, state_style),
        Span::styled(updated_label, dim),
        Span::styled(run_label, dim),
    ]);
    buf.set_line(area.x, area.y + 1, &header2, area.width);

    // Line 3: separator
    let sep = "\u{2500}".repeat(width); // ─
    buf.set_line(
        area.x,
        area.y + 2,
        &RLine::from(Span::styled(
            sep.clone(),
            Style::default().fg(colors::border()),
        )),
        area.width,
    );

    // --- Pinned run config (bottom, 4 lines) ------------------------------
    let config_height: u16 = 4;
    let remaining = area.height - header_height;
    let scroll_area_height = remaining.saturating_sub(config_height);
    if scroll_area_height == 0 {
        overlay.detail_visible_rows.set(0);
        overlay.detail_max_scroll.set(0);
        // Still render config if space
        if remaining >= config_height {
            render_pinned_config(area.x, area.y + header_height, area.width, buf);
        }
        return;
    }

    // --- Scrollable middle ------------------------------------------------
    let content_lines = detail_content_lines(node, width);
    let total = content_lines.len();

    overlay.detail_visible_rows.set(scroll_area_height);
    let max_scroll = total.saturating_sub(scroll_area_height as usize);
    overlay.detail_max_scroll.set(max_scroll as u16);
    let scroll = (overlay.detail_scroll.get() as usize).min(max_scroll);
    overlay.detail_scroll.set(scroll as u16);

    let scroll_y = area.y + header_height;
    for (i, line) in content_lines
        .iter()
        .skip(scroll)
        .take(scroll_area_height as usize)
        .enumerate()
    {
        buf.set_line(area.x, scroll_y + i as u16, line, area.width);
    }

    // --- Pinned run config ------------------------------------------------
    let config_y = scroll_y + scroll_area_height;
    // Config separator
    buf.set_line(
        area.x,
        config_y,
        &RLine::from(Span::styled(sep, Style::default().fg(colors::border()))),
        area.width,
    );
    render_pinned_config(area.x, config_y + 1, area.width, buf);
}

fn render_pinned_config(x: u16, y: u16, width: u16, buf: &mut Buffer) {
    let dim = Style::default().fg(colors::text_dim());
    let label_style = Style::default().fg(colors::info());

    // Line 1: Presets
    let presets = RLine::from(vec![
        Span::styled("  Preset: ", label_style),
        Span::styled("quick  ", dim),
        Span::styled("[standard]  ", Style::default().fg(colors::text())),
        Span::styled("deep  ", dim),
        Span::styled("exhaustive", dim),
        Span::styled("  (read-only)", dim),
    ]);
    buf.set_line(x, y, &presets, width);

    // Line 2: Scopes
    let scopes = RLine::from(vec![
        Span::styled("  Scopes: ", label_style),
        Span::styled(
            "[x] correctness  [x] security  [x] performance  [x] style  [x] architecture",
            dim,
        ),
    ]);
    buf.set_line(x, y + 1, &scopes, width);

    // Line 3: Actions (disabled)
    let actions = RLine::from(vec![
        Span::styled("  Actions: ", label_style),
        Span::styled("[F8] Run Research  ", dim),
        Span::styled("[F9] Run Review  ", dim),
        Span::styled("(disabled)", dim),
    ]);
    buf.set_line(x, y + 2, &actions, width);
}

// ---------------------------------------------------------------------------
// Summary bar
// ---------------------------------------------------------------------------

fn render_summary_bar(overlay: &PmOverlay, area: Rect, buf: &mut Buffer) {
    let dim = Style::default().fg(colors::text_dim());

    // PM-001 canonical display states (Feature/Spec lifecycle + Task ternary)
    let display_states: &[&str] = &[
        "Backlog",
        "NeedsResearch",
        "Planned",
        "InProgress",
        "NeedsReview",
        "Completed",
        "Deprecated",
        "Archived",
        "open",
        "completed",
        "failed",
    ];

    // Count each label: exact matches + holding variants like "Label (-> ...)"
    let mut chips: Vec<Span<'static>> = Vec::new();
    for &label in display_states {
        let count = overlay
            .nodes
            .iter()
            .filter(|n| !n.state.is_empty())
            .filter(|n| n.state == label || n.state.starts_with(&format!("{label} (->")))
            .count();
        if count > 0 {
            if !chips.is_empty() {
                chips.push(Span::styled("  ", dim));
            }
            let color = state_color(label);
            chips.push(Span::styled(
                format!("{label}:{count}"),
                Style::default().fg(color),
            ));
        }
    }
    chips.push(Span::styled("  |  Active runs: 0", dim));

    if overlay.degraded {
        chips.push(Span::styled(
            "  [DEGRADED]",
            Style::default().fg(colors::error()),
        ));
    }

    let line1 = RLine::from(chips);
    let line1_y = area.y;
    buf.set_line(area.x, line1_y, &line1, area.width);

    // Line 2: run meter placeholder
    let line2 = RLine::from(Span::styled("Active run meter: 0 running", dim));
    buf.set_line(area.x, line1_y + 1, &line2, area.width);

    // Degraded banner
    if overlay.degraded && area.height >= 3 {
        let banner = RLine::from(Span::styled(
            "PM service unavailable -- read-only demo data",
            Style::default()
                .fg(colors::error())
                .bg(colors::background()),
        ));
        buf.set_line(area.x, line1_y + 2, &banner, area.width);
    }

    // Separator
    let sep_y = area.y + area.height.saturating_sub(1);
    if sep_y > area.y {
        let sep = "\u{2500}".repeat(area.width as usize); // ─
        buf.set_line(
            area.x,
            area.y + area.height,
            &RLine::from(Span::styled(sep, Style::default().fg(colors::border()))),
            area.width,
        );
    }
}

// ---------------------------------------------------------------------------
// Tree list (virtualized)
// ---------------------------------------------------------------------------

fn render_list(overlay: &PmOverlay, area: Rect, buf: &mut Buffer) {
    let visible = overlay.visible_indices();
    let total = visible.len();
    let rows = area.height as usize;

    overlay.visible_rows.set(area.height);

    let max_scroll = total.saturating_sub(rows);
    overlay.max_scroll.set(max_scroll as u16);

    let scroll = (overlay.scroll.get() as usize).min(max_scroll);
    overlay.scroll.set(scroll as u16);

    // Ensure selected is in view
    let sel = overlay.selected.get().min(total.saturating_sub(1));
    overlay.selected.set(sel);

    let width = area.width as usize;

    for (row_idx, &node_idx) in visible.iter().skip(scroll).take(rows).enumerate() {
        let y = area.y + row_idx as u16;
        let node = &overlay.nodes[node_idx];
        let flat_idx = scroll + row_idx;
        let is_selected = flat_idx == sel;

        let line = render_row(node, node_idx, overlay, width, is_selected);
        buf.set_line(area.x, y, &line, area.width);
    }
}

fn render_row(
    node: &TreeNode,
    node_idx: usize,
    overlay: &PmOverlay,
    width: usize,
    selected: bool,
) -> RLine<'static> {
    let indent = "  ".repeat(node.depth as usize);
    let arrow = if node.children.is_empty() {
        " "
    } else if overlay.is_expanded(node_idx) {
        "\u{25be}" // ▾
    } else {
        "\u{25b8}" // ▸
    };

    let bg = if selected {
        colors::selection()
    } else {
        colors::background()
    };
    let fg = if selected {
        colors::text()
    } else {
        colors::text_dim()
    };
    let base = Style::default().fg(fg).bg(bg);
    let state_style = Style::default().fg(state_color(&node.state)).bg(bg);

    // Adaptive columns based on width
    if width >= 120 {
        // 5 columns: ID | Title | State | Updated | Latest Run
        let id_w = 18;
        let state_w = 14;
        let updated_w = 22;
        let run_w = 12;
        let title_w = width.saturating_sub(id_w + state_w + updated_w + run_w + 4);
        let prefix = format!("{indent}{arrow} ");
        let id_col = format!("{}{}", prefix, node.id);

        let mut spans = vec![
            Span::styled(pad_or_trunc(&id_col, id_w), base),
            Span::styled(" ", base),
            Span::styled(pad_or_trunc(&node.title, title_w), base),
            Span::styled(" ", base),
            Span::styled(pad_or_trunc(&node.state, state_w), state_style),
            Span::styled(" ", base),
            Span::styled(pad_or_trunc(&short_date(&node.updated_at), updated_w), base),
            Span::styled(" ", base),
            Span::styled(pad_or_trunc(&node.latest_run, run_w), base),
        ];
        // Fill remaining with bg
        let used: usize = spans.iter().map(|s| s.content.len()).sum();
        if used < width {
            spans.push(Span::styled(" ".repeat(width - used), base));
        }
        RLine::from(spans)
    } else if width >= 80 {
        // 4 columns: ID | Title | State | Updated
        let id_w = 18;
        let state_w = 14;
        let updated_w = 13;
        let title_w = width.saturating_sub(id_w + state_w + updated_w + 3);
        let prefix = format!("{indent}{arrow} ");
        let id_col = format!("{}{}", prefix, node.id);

        let mut spans = vec![
            Span::styled(pad_or_trunc(&id_col, id_w), base),
            Span::styled(" ", base),
            Span::styled(pad_or_trunc(&node.title, title_w), base),
            Span::styled(" ", base),
            Span::styled(pad_or_trunc(&node.state, state_w), state_style),
            Span::styled(" ", base),
            Span::styled(pad_or_trunc(&short_date(&node.updated_at), updated_w), base),
        ];
        let used: usize = spans.iter().map(|s| s.content.len()).sum();
        if used < width {
            spans.push(Span::styled(" ".repeat(width - used), base));
        }
        RLine::from(spans)
    } else {
        // 3 columns: ID | Title | State
        let id_w = 16;
        let state_w = 12;
        let title_w = width.saturating_sub(id_w + state_w + 2);
        let prefix = format!("{indent}{arrow} ");
        let id_col = format!("{}{}", prefix, node.id);

        let mut spans = vec![
            Span::styled(pad_or_trunc(&id_col, id_w), base),
            Span::styled(" ", base),
            Span::styled(pad_or_trunc(&node.title, title_w), base),
            Span::styled(" ", base),
            Span::styled(pad_or_trunc(&node.state, state_w), state_style),
        ];
        let used: usize = spans.iter().map(|s| s.content.len()).sum();
        if used < width {
            spans.push(Span::styled(" ".repeat(width - used), base));
        }
        RLine::from(spans)
    }
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

fn state_color(state: &str) -> ratatui::style::Color {
    // Holding states (e.g. "NeedsReview (-> InProgress)") get amber
    if state.contains("(->") {
        return colors::warning();
    }
    match state {
        "Backlog" | "Deprecated" | "Archived" => colors::text_dim(),
        "NeedsResearch" | "NeedsReview" => colors::warning(),
        "Planned" => colors::info(),
        "InProgress" => colors::function(),
        "Completed" | "completed" => colors::success(),
        "failed" => colors::error(),
        "open" => colors::text(),
        _ => colors::text(),
    }
}

fn pad_or_trunc(s: &str, width: usize) -> String {
    if s.len() >= width {
        format!("{}\u{2026}", &s[..width.saturating_sub(1)]) // truncate + ellipsis
    } else {
        format!("{:<width$}", s)
    }
}

fn short_date(iso: &str) -> String {
    // Extract YYYY-MM-DD from ISO timestamp
    if iso.len() >= 10 {
        iso[..10].to_string()
    } else if iso.is_empty() {
        "-".to_string()
    } else {
        iso.to_string()
    }
}

/// Quick service status check. Returns `true` if degraded (service unavailable).
fn check_service_status() -> bool {
    use super::spec_kit::commands::pm::send_rpc;
    send_rpc("service.status", serde_json::json!({})).is_err()
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_demo_tree_valid() {
        let nodes = demo_tree();
        assert!(nodes.len() >= 14, "Demo tree should have >= 14 nodes");
        // All children indices should be valid
        for (i, node) in nodes.iter().enumerate() {
            for &child in &node.children {
                assert!(child < nodes.len(), "Node {i} has invalid child {child}");
                assert_eq!(
                    nodes[child].parent_idx,
                    Some(i),
                    "Child {child} parent should be {i}"
                );
            }
        }
    }

    #[test]
    fn test_overlay_visible_count_collapsed() {
        let overlay = PmOverlay::new(false);
        // All collapsed: only root (depth=0) nodes visible — single Project
        let count = overlay.visible_count();
        assert_eq!(
            count, 1,
            "Only 1 project root should be visible when collapsed"
        );
    }

    #[test]
    fn test_overlay_expand_collapse() {
        let overlay = PmOverlay::new(false);
        assert_eq!(overlay.visible_count(), 1);

        // Expand Project → Project + 2 Features = 3
        assert!(overlay.expand(0));
        assert_eq!(overlay.visible_count(), 3);

        // Expand FEAT-001 (node 1) → +2 Specs = 5
        assert!(overlay.expand(1));
        assert_eq!(overlay.visible_count(), 5);

        // Expand SPEC-AUTH-001 (node 2) → +2 Tasks = 7
        assert!(overlay.expand(2));
        assert_eq!(overlay.visible_count(), 7);

        // Collapse Project → back to 1
        assert!(overlay.collapse(0));
        assert_eq!(overlay.visible_count(), 1);
    }

    #[test]
    fn test_node_idx_of_visible_second_feature() {
        let overlay = PmOverlay::new(false);
        // Collapsed: visible = [0] (single Project root)
        assert_eq!(overlay.node_idx_of_visible(0), Some(0));
        assert_eq!(overlay.visible_count(), 1);

        // Expand Project → visible = [0, 1, 8] (Project, FEAT-001, FEAT-002)
        assert!(overlay.expand(0));
        assert_eq!(overlay.visible_count(), 3);

        // expand_visible(2) → should expand node 8 (FEAT-002)
        assert!(overlay.expand_visible(2));
        // Now visible: [0, 1, 8, 9, 11] = 5 items
        assert_eq!(overlay.visible_count(), 5);
        assert!(overlay.is_expanded(8)); // node 8 (FEAT-002) is expanded
        assert!(!overlay.is_expanded(2)); // node 2 (SPEC-AUTH-001) is NOT expanded
    }

    #[test]
    fn test_pad_or_trunc() {
        assert_eq!(pad_or_trunc("hello", 10), "hello     ");
        assert_eq!(pad_or_trunc("hello world long", 10), "hello wor\u{2026}");
    }

    #[test]
    fn test_short_date() {
        assert_eq!(short_date("2026-02-12T10:30:00Z"), "2026-02-12");
        assert_eq!(short_date(""), "-");
        assert_eq!(short_date("short"), "short");
    }

    #[test]
    fn test_detail_opens_correct_node_for_second_feature() {
        // Guards against the visible-index → node-index mapping bug:
        // When Project is expanded, visible row 2 is FEAT-002 (node 8),
        // NOT node 2 (SPEC-AUTH-001).
        let overlay = PmOverlay::new(false);

        // Expand Project → visible = [0, 1, 8]
        overlay.expand(0);
        assert_eq!(overlay.visible_count(), 3);

        // Select visible row 2 (should be FEAT-002 = node 8)
        overlay.set_selected(2);
        assert_eq!(overlay.node_idx_of_visible(2), Some(8));

        // Open detail for selected row
        assert!(overlay.open_detail_for_visible(2));
        assert!(overlay.is_detail_mode());
        assert_eq!(overlay.detail_node_idx.get(), Some(8));

        // Verify it's FEAT-002, not SPEC-AUTH-001
        let node = overlay.detail_node().expect("detail node should exist");
        assert_eq!(node.id, "FEAT-002");
        assert_eq!(node.title, "Pipeline Orchestration");

        // Close detail and verify list selection preserved
        overlay.close_detail();
        assert!(!overlay.is_detail_mode());
        assert_eq!(overlay.selected(), 2, "list selection should be preserved");
    }

    #[test]
    fn test_detail_scroll_clamped() {
        let overlay = PmOverlay::new(false);
        overlay.expand(0);
        overlay.set_selected(1);
        overlay.open_detail_for_visible(1);

        // detail_max_scroll is 0 initially (no rendering has happened)
        overlay.detail_max_scroll.set(5);
        overlay.set_detail_scroll(100);
        assert_eq!(
            overlay.detail_scroll(),
            5,
            "scroll should be clamped to max"
        );

        overlay.set_detail_scroll(0);
        assert_eq!(overlay.detail_scroll(), 0);
    }

    #[test]
    fn test_state_color_variants() {
        // Ensure no panics for all PM-001 canonical states + holding + empty
        for state in &[
            "Backlog",
            "NeedsResearch",
            "Planned",
            "InProgress",
            "NeedsReview",
            "Completed",
            "Deprecated",
            "Archived",
            "open",
            "completed",
            "failed",
            "NeedsReview (-> InProgress)",
            "NeedsResearch (-> Backlog)",
            "",
            "Unknown",
        ] {
            let _ = state_color(state);
        }
    }
}
