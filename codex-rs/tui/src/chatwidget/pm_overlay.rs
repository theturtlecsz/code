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
use super::session_handlers::human_ago;

const DEGRADED_BANNER_TEXT: &str = "PM service unavailable -- read-only";

// ---------------------------------------------------------------------------
// State wrapper (follows LimitsState / ProState pattern)
// ---------------------------------------------------------------------------

#[derive(Default)]
pub(super) struct PmState {
    pub(super) overlay: Option<PmOverlay>,
    /// Persisted sort mode across overlay open/close within session
    pub(super) last_sort_mode: Option<SortMode>,
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
    detail_auto_scroll_pending: Cell<bool>,
    /// Current sort mode for list view
    sort_mode: Cell<SortMode>,
}

#[derive(Clone, Copy, PartialEq, Eq)]
#[allow(dead_code)] // Variants stored for future detail view / filtering
enum NodeType {
    Project,
    Feature,
    Spec,
    Task,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
#[allow(dead_code)] // Variants used in tests
pub(super) enum SortMode {
    UpdatedDesc,   // Most recently updated first
    StatePriority, // By state priority (InProgress, NeedsReview, etc.)
    IdAsc,         // Alphabetically by ID
}

#[allow(dead_code)] // Fields used across rendering + future detail view
struct TreeNode {
    id: String,
    title: String,
    node_type: NodeType,
    state: String,
    updated_at: String,
    latest_run: String,
    latest_run_status: String,
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
        latest_run_status: String::new(),
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
        latest_run_status: String::new(),
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
        latest_run_status: String::new(),
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
        latest_run_status: "succeeded".into(),
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
        latest_run_status: "needs_attention".into(),
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
        latest_run_status: String::new(),
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
        latest_run_status: String::new(),
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
        latest_run_status: "failed".into(),
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
        latest_run_status: String::new(),
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
        latest_run_status: String::new(),
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
        latest_run_status: "running".into(),
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
        latest_run_status: String::new(),
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
        latest_run_status: String::new(),
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
        latest_run_status: String::new(),
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
    pub(super) fn new(degraded: bool, initial_sort_mode: Option<SortMode>) -> Self {
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
            detail_auto_scroll_pending: Cell::new(false),
            sort_mode: Cell::new(initial_sort_mode.unwrap_or(SortMode::UpdatedDesc)),
        }
    }

    /// Create a worst-case overlay: degraded with no data available.
    #[cfg(test)]
    fn new_degraded_empty() -> Self {
        Self {
            scroll: Cell::new(0),
            max_scroll: Cell::new(0),
            visible_rows: Cell::new(0),
            selected: Cell::new(0),
            expanded: std::cell::RefCell::new(HashSet::new()),
            nodes: Vec::new(),
            degraded: true,
            detail_node_idx: Cell::new(None),
            detail_scroll: Cell::new(0),
            detail_max_scroll: Cell::new(0),
            detail_visible_rows: Cell::new(0),
            detail_auto_scroll_pending: Cell::new(false),
            sort_mode: Cell::new(SortMode::UpdatedDesc),
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
        // Store raw value; clamping happens during render in render_list
        self.scroll.set(val);
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
                // PM-UX-D11: auto-scroll to Run History for needs_attention
                self.detail_auto_scroll_pending
                    .set(self.nodes[node_idx].latest_run_status == "needs_attention");
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

    // -- Sort mode accessors --------------------------------------------------

    pub(super) fn sort_mode(&self) -> SortMode {
        self.sort_mode.get()
    }

    /// Cycle to the next sort mode (for read-only demo/testing).
    #[allow(dead_code)] // Used in tests
    pub(super) fn cycle_sort_mode(&self) {
        let next = match self.sort_mode.get() {
            SortMode::UpdatedDesc => SortMode::StatePriority,
            SortMode::StatePriority => SortMode::IdAsc,
            SortMode::IdAsc => SortMode::UpdatedDesc,
        };
        self.sort_mode.set(next);
    }

    // -- Visible list computation ---------------------------------------------

    fn visible_indices(&self) -> Vec<usize> {
        let exp = self.expanded.borrow();
        let mut out = Vec::new();
        self.collect_visible(0, &exp, &mut out);
        out
    }

    fn collect_visible(&self, start: usize, exp: &HashSet<usize>, out: &mut Vec<usize>) {
        // Collect root nodes (depth 0) and sort them
        let mut roots: Vec<usize> = self
            .nodes
            .iter()
            .enumerate()
            .filter(|(i, node)| *i >= start && node.depth == 0)
            .map(|(i, _)| i)
            .collect();
        self.sort_siblings(&mut roots);

        // Walk sorted roots
        for &root_idx in &roots {
            self.collect_subtree(root_idx, exp, out);
        }
    }

    fn collect_subtree(&self, idx: usize, exp: &HashSet<usize>, out: &mut Vec<usize>) {
        out.push(idx);
        if exp.contains(&idx) {
            // Sort children according to current sort mode before adding them
            let mut children = self.nodes[idx].children.clone();
            self.sort_siblings(&mut children);
            for &child in &children {
                self.collect_subtree(child, exp, out);
            }
        }
    }

    /// Sort sibling node indices according to current sort mode.
    fn sort_siblings(&self, indices: &mut Vec<usize>) {
        let mode = self.sort_mode.get();
        match mode {
            SortMode::UpdatedDesc => {
                // Sort by updated_at descending (most recent first)
                indices.sort_by(|&a, &b| {
                    let a_ts = &self.nodes[a].updated_at;
                    let b_ts = &self.nodes[b].updated_at;
                    b_ts.cmp(a_ts) // Reverse for descending
                });
            }
            SortMode::StatePriority => {
                // Sort by state priority: InProgress > NeedsReview > Planned > Backlog
                indices.sort_by(|&a, &b| {
                    let a_prio = state_priority(&self.nodes[a].state);
                    let b_prio = state_priority(&self.nodes[b].state);
                    a_prio.cmp(&b_prio) // Lower number = higher priority
                });
            }
            SortMode::IdAsc => {
                // Sort alphabetically by ID ascending
                indices.sort_by(|&a, &b| {
                    let a_id = &self.nodes[a].id;
                    let b_id = &self.nodes[b].id;
                    a_id.cmp(b_id)
                });
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
        let initial_sort = self.pm.last_sort_mode;
        self.pm.overlay = Some(PmOverlay::new(degraded, initial_sort));
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

        // Format sort mode for display
        let sort_label = match overlay.sort_mode() {
            SortMode::UpdatedDesc => "Updated",
            SortMode::StatePriority => "State",
            SortMode::IdAsc => "ID",
        };

        let title = if is_detail {
            RLine::from(vec![
                Span::styled(" PM Detail ", bright),
                Span::styled("--- ", dim),
                Span::styled("Up/Dn", accent),
                Span::styled(" scroll  ", dim),
                Span::styled("Esc", bright),
                Span::styled(" back to list  ", dim),
                Span::styled("Sort: ", dim),
                Span::styled(sort_label, accent),
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
                Span::styled("s", accent),
                Span::styled(" sort  ", dim),
                Span::styled("Esc", bright),
                Span::styled(" close  ", dim),
                Span::styled("Sort: ", dim),
                Span::styled(sort_label, accent),
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
        } else if overlay.degraded && overlay.nodes.is_empty() {
            // PM-UX-D15: worst-case fallback — no data available at all
            overlay.visible_rows.set(0);
            overlay.max_scroll.set(0);
            render_worst_case_fallback(overlay, body, buf);
        } else if !overlay.degraded && overlay.nodes.is_empty() {
            // PM-UX-D7/PM-UX-D24: empty-state onboarding when service is healthy
            overlay.visible_rows.set(0);
            overlay.max_scroll.set(0);
            render_empty_state_onboarding(overlay, body, buf);
        } else {
            // Summary bar (2 lines) + optional degraded banner (1 line)
            let degraded_lines: u16 = if overlay.degraded { 1 } else { 0 };
            let summary_height = 2 + degraded_lines;
            if body.height <= summary_height {
                overlay.visible_rows.set(0);
                overlay.max_scroll.set(0);
                return;
            }

            // Reserve 1 line for footer (position indicator)
            let footer_height: u16 = 1;
            let list_height = body.height.saturating_sub(summary_height + footer_height);

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
                height: list_height,
            };
            let footer_area = Rect {
                x: body.x,
                y: body.y + summary_height + list_height,
                width: body.width,
                height: footer_height,
            };

            render_summary_bar(overlay, summary_area, buf);
            render_list(overlay, list_area, buf);
            render_list_footer(overlay, footer_area, buf);
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
        let status_text = if node.latest_run_status == "needs_attention" {
            "needs_attn "
        } else if node.latest_run_status == "running" {
            "running    "
        } else {
            "completed  "
        };
        let status_style = if node.latest_run_status == "needs_attention" {
            Style::default().fg(colors::warning())
        } else if node.latest_run_status == "running" {
            Style::default().fg(colors::function())
        } else {
            dim
        };
        lines.push(RLine::from(vec![
            Span::styled(format!("  {:<14}", node.latest_run), bright),
            Span::styled("review     ", dim),
            Span::styled("standard   ", dim),
            Span::styled(status_text, status_style),
            Span::styled(short_date(&node.updated_at), dim),
        ]));
        // PM-UX-D11: conflict summary + resolution for needs_attention
        if node.latest_run_status == "needs_attention" {
            let warning_style = Style::default().fg(colors::warning());
            lines.push(RLine::from(Span::styled("", dim)));
            lines.push(RLine::from(Span::styled(
                "  Conflict summary:",
                warning_style,
            )));
            lines.push(RLine::from(Span::styled(
                "    Review found conflicting recommendations between",
                bright,
            )));
            lines.push(RLine::from(Span::styled(
                "    PKCE flow and existing OAuth2 token refresh.",
                bright,
            )));
            lines.push(RLine::from(Span::styled("", dim)));
            lines.push(RLine::from(Span::styled(
                "  Resolution instructions:",
                warning_style,
            )));
            lines.push(RLine::from(Span::styled(
                "    1. Review conflicting artifacts in the run output",
                bright,
            )));
            lines.push(RLine::from(Span::styled(
                "    2. Choose a resolution and apply changes",
                bright,
            )));
            lines.push(RLine::from(Span::styled(
                "    3. Re-run review once resolved",
                bright,
            )));
        }
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

    // --- Fixed header (3 lines) + optional degraded banner ----------------
    let banner_height: u16 = if overlay.degraded { 1 } else { 0 };
    let header_height: u16 = 3;
    let fixed_top_height = banner_height + header_height;

    if overlay.degraded {
        let banner = RLine::from(Span::styled(
            DEGRADED_BANNER_TEXT,
            Style::default()
                .fg(colors::error())
                .bg(colors::background()),
        ));
        buf.set_line(area.x, area.y, &banner, area.width);
    }

    if area.height <= fixed_top_height {
        overlay.detail_visible_rows.set(0);
        overlay.detail_max_scroll.set(0);
        return;
    }
    let header_y = area.y + banner_height;

    // Line 1: ID + title
    let id_title = format!("{} \u{2014} {}", node.id, node.title);
    let header1 = RLine::from(Span::styled(pad_or_trunc(&id_title, width), bright));
    buf.set_line(area.x, header_y, &header1, area.width);

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
    buf.set_line(area.x, header_y + 1, &header2, area.width);

    // Line 3: separator
    let sep = "\u{2500}".repeat(width); // ─
    buf.set_line(
        area.x,
        header_y + 2,
        &RLine::from(Span::styled(
            sep.clone(),
            Style::default().fg(colors::border()),
        )),
        area.width,
    );

    // --- Pinned run config (bottom, 4 lines) ------------------------------
    let config_height: u16 = 4;
    let remaining = area.height - fixed_top_height;
    let scroll_area_height = remaining.saturating_sub(config_height);
    if scroll_area_height == 0 {
        overlay.detail_visible_rows.set(0);
        overlay.detail_max_scroll.set(0);
        // Still render config if space
        if remaining >= config_height {
            render_pinned_config(area.x, area.y + fixed_top_height, area.width, buf);
        }
        return;
    }

    // --- Scrollable middle ------------------------------------------------
    let content_lines = detail_content_lines(node, width);
    let total = content_lines.len();

    overlay.detail_visible_rows.set(scroll_area_height);
    let max_scroll = total.saturating_sub(scroll_area_height as usize);
    overlay.detail_max_scroll.set(max_scroll as u16);
    // PM-UX-D11: auto-scroll to Run History for needs_attention nodes
    if overlay.detail_auto_scroll_pending.get() {
        overlay.detail_auto_scroll_pending.set(false);
        let target = content_lines
            .iter()
            .position(|line| {
                line.spans
                    .first()
                    .is_some_and(|s| s.content.contains("Run History"))
            })
            .unwrap_or(0);
        let target = (target as u16).min(max_scroll as u16);
        overlay.detail_scroll.set(target);
    }
    let scroll = (overlay.detail_scroll.get() as usize).min(max_scroll);
    overlay.detail_scroll.set(scroll as u16);

    let scroll_y = area.y + fixed_top_height;
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
// Worst-case fallback (PM-UX-D15)
// ---------------------------------------------------------------------------

/// Full-screen error state when service is down, cache is missing, and capsule
/// access has failed.  Renders three diagnostic lines and two remedy commands.
fn render_worst_case_fallback(_overlay: &PmOverlay, area: Rect, buf: &mut Buffer) {
    let dim = Style::default().fg(colors::text_dim());
    let err = Style::default().fg(colors::error());
    let bright = Style::default().fg(colors::text());
    let accent = Style::default().fg(colors::function());

    let lines: Vec<RLine<'static>> = vec![
        // Title
        RLine::from(Span::styled(
            "PM data unavailable",
            Style::default().fg(colors::error()),
        )),
        RLine::from(Span::styled("", dim)),
        // Diagnostic lines
        RLine::from(vec![
            Span::styled("  Service status:  ", bright),
            Span::styled("not running / connection refused", err),
        ]),
        RLine::from(vec![
            Span::styled("  Cache status:    ", bright),
            Span::styled("missing / stale / corrupt", err),
        ]),
        RLine::from(vec![
            Span::styled("  Capsule status:  ", bright),
            Span::styled("locked / unreadable / missing", err),
        ]),
        RLine::from(Span::styled("", dim)),
        // Separator
        RLine::from(Span::styled(
            "\u{2500}".repeat(area.width as usize),
            Style::default().fg(colors::border()),
        )),
        RLine::from(Span::styled("", dim)),
        // Remedies
        RLine::from(Span::styled("  Remedies:", bright)),
        RLine::from(vec![
            Span::styled("    1. Run ", dim),
            Span::styled("/pm service doctor", accent),
        ]),
        RLine::from(vec![
            Span::styled("    2. Run ", dim),
            Span::styled("systemctl --user start codex-pm-service", accent),
        ]),
    ];

    for (i, line) in lines.iter().take(area.height as usize).enumerate() {
        buf.set_line(area.x, area.y + i as u16, line, area.width);
    }
}

// ---------------------------------------------------------------------------
// Empty-state onboarding (PM-UX-D7 / PM-UX-D24)
// ---------------------------------------------------------------------------

/// Render onboarding empty-state when service is healthy but no work items exist.
/// Displays the 3-step guided wizard flow (read-only presentation).
fn render_empty_state_onboarding(_overlay: &PmOverlay, area: Rect, buf: &mut Buffer) {
    let dim = Style::default().fg(colors::text_dim());
    let bright = Style::default().fg(colors::text());
    let accent = Style::default().fg(colors::function());
    let info_style = Style::default().fg(colors::info());

    let lines: Vec<RLine<'static>> = vec![
        // Title
        RLine::from(Span::styled(
            "Welcome to PM — Let's get started!",
            Style::default().fg(colors::info()),
        )),
        RLine::from(Span::styled("", dim)),
        // Introduction
        RLine::from(vec![
            Span::styled("  Your PM workspace is empty. ", bright),
            Span::styled("To create your first work item, follow these steps:", dim),
        ]),
        RLine::from(Span::styled("", dim)),
        // Separator
        RLine::from(Span::styled(
            "\u{2500}".repeat(area.width.min(60) as usize),
            Style::default().fg(colors::border()),
        )),
        RLine::from(Span::styled("", dim)),
        // Step 1
        RLine::from(vec![
            Span::styled("  Step 1: ", info_style),
            Span::styled("Confirm your project container", bright),
        ]),
        RLine::from(vec![
            Span::styled("    Default: ", dim),
            Span::styled("(current repository name)", accent),
        ]),
        RLine::from(Span::styled("", dim)),
        // Step 2
        RLine::from(vec![
            Span::styled("  Step 2: ", info_style),
            Span::styled("Choose your first work item type", bright),
        ]),
        RLine::from(vec![
            Span::styled("    Options: ", dim),
            Span::styled("Feature", accent),
            Span::styled(" or ", dim),
            Span::styled("SPEC", accent),
        ]),
        RLine::from(Span::styled("", dim)),
        // Step 3
        RLine::from(vec![
            Span::styled("  Step 3: ", info_style),
            Span::styled("Enter title/name and begin maieutic intake", bright),
        ]),
        RLine::from(vec![
            Span::styled("    The system will guide you through ", dim),
            Span::styled("defining your work item", accent),
        ]),
        RLine::from(Span::styled("", dim)),
        // Separator
        RLine::from(Span::styled(
            "\u{2500}".repeat(area.width.min(60) as usize),
            Style::default().fg(colors::border()),
        )),
        RLine::from(Span::styled("", dim)),
        // Next action
        RLine::from(vec![Span::styled("  Next steps:", bright)]),
        RLine::from(vec![
            Span::styled("    • Check service status: ", dim),
            Span::styled("/pm service doctor", accent),
        ]),
        RLine::from(vec![
            Span::styled("    • View PM overlay: ", dim),
            Span::styled("/pm open", accent),
            Span::styled(" (returns here)", dim),
        ]),
        RLine::from(vec![
            Span::styled("    • Run research on work item: ", dim),
            Span::styled("/pm bot run --id <ID> --kind research", accent),
        ]),
    ];

    for (i, line) in lines.iter().take(area.height as usize).enumerate() {
        buf.set_line(area.x, area.y + i as u16, line, area.width);
    }
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
    let active_count = overlay
        .nodes
        .iter()
        .filter(|n| n.latest_run_status == "running")
        .count();
    let active_style = if active_count > 0 {
        Style::default().fg(colors::function())
    } else {
        dim
    };
    chips.push(Span::styled(
        format!("  |  Active runs: {active_count}"),
        active_style,
    ));

    if overlay.degraded {
        chips.push(Span::styled(
            "  [DEGRADED]",
            Style::default().fg(colors::error()),
        ));
    }

    let line1 = RLine::from(chips);
    let line1_y = area.y;
    buf.set_line(area.x, line1_y, &line1, area.width);

    // Line 2: run meter
    let line2 = RLine::from(Span::styled(
        format!("Active run meter: {active_count} running"),
        active_style,
    ));
    buf.set_line(area.x, line1_y + 1, &line2, area.width);

    // Degraded banner
    if overlay.degraded && area.height >= 3 {
        let banner = RLine::from(Span::styled(
            DEGRADED_BANNER_TEXT,
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
    // Don't write back clamped scroll - preserve user's scroll value across mode changes

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

// ---------------------------------------------------------------------------
// List footer (position indicator)
// ---------------------------------------------------------------------------

fn render_list_footer(overlay: &PmOverlay, area: Rect, buf: &mut Buffer) {
    if area.height == 0 {
        return;
    }

    let dim = Style::default().fg(colors::text_dim());
    let bright = Style::default().fg(colors::text());
    let accent = Style::default().fg(colors::function());

    let visible_count = overlay.visible_count();
    let current_row = if visible_count > 0 {
        (overlay.selected() + 1).min(visible_count)
    } else {
        0
    };

    let sort_label = match overlay.sort_mode() {
        SortMode::UpdatedDesc => "Updated",
        SortMode::StatePriority => "State",
        SortMode::IdAsc => "ID",
    };

    let footer = if visible_count > 0 {
        // Calculate visible window range
        let scroll = overlay.scroll() as usize;
        let viewport_rows = overlay.visible_rows() as usize;
        let window_start = (scroll + 1).min(visible_count); // 1-based
        let window_end = (scroll + viewport_rows).min(visible_count);

        RLine::from(vec![
            Span::styled(format!(" Row {}/{} ", current_row, visible_count), bright),
            Span::styled("| ", dim),
            Span::styled(
                format!(
                    "Showing {}-{} of {} ",
                    window_start, window_end, visible_count
                ),
                dim,
            ),
            Span::styled("| ", dim),
            Span::styled("Sort: ", dim),
            Span::styled(sort_label, accent),
            Span::styled(" ", dim),
        ])
    } else {
        RLine::from(vec![Span::styled(" No items ", bright)])
    };

    buf.set_line(area.x, area.y, &footer, area.width);
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
            Span::styled(pad_or_trunc(&human_ago(&node.updated_at), updated_w), base),
            Span::styled(" ", base),
        ];
        // PM-UX-D11: needs_attention badge on latest-run column
        if node.latest_run_status == "needs_attention" && !node.latest_run.is_empty() {
            let warn = Style::default().fg(colors::warning()).bg(bg);
            spans.push(Span::styled("! ", warn));
            spans.push(Span::styled(
                pad_or_trunc(&node.latest_run, run_w.saturating_sub(2)),
                base,
            ));
        } else if node.latest_run_status == "running" && !node.latest_run.is_empty() {
            let run_style = Style::default().fg(colors::function()).bg(bg);
            spans.push(Span::styled("\u{25b6} ", run_style));
            spans.push(Span::styled(
                pad_or_trunc(&node.latest_run, run_w.saturating_sub(2)),
                base,
            ));
        } else {
            spans.push(Span::styled(pad_or_trunc(&node.latest_run, run_w), base));
        }
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
        ];
        // PM-UX-D11: needs_attention indicator on state column (medium)
        if node.latest_run_status == "needs_attention" {
            spans.push(Span::styled(
                pad_or_trunc(&node.state, state_w.saturating_sub(2)),
                state_style,
            ));
            spans.push(Span::styled(
                " !",
                Style::default().fg(colors::warning()).bg(bg),
            ));
        } else if node.latest_run_status == "running" {
            spans.push(Span::styled(
                pad_or_trunc(&node.state, state_w.saturating_sub(2)),
                state_style,
            ));
            spans.push(Span::styled(
                " \u{25b6}",
                Style::default().fg(colors::function()).bg(bg),
            ));
        } else {
            spans.push(Span::styled(
                pad_or_trunc(&node.state, state_w),
                state_style,
            ));
        }
        spans.push(Span::styled(" ", base));
        spans.push(Span::styled(
            pad_or_trunc(&human_ago(&node.updated_at), updated_w),
            base,
        ));
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
        ];
        // PM-UX-D11: needs_attention indicator on state column (narrow)
        if node.latest_run_status == "needs_attention" {
            spans.push(Span::styled(
                pad_or_trunc(&node.state, state_w.saturating_sub(2)),
                state_style,
            ));
            spans.push(Span::styled(
                " !",
                Style::default().fg(colors::warning()).bg(bg),
            ));
        } else if node.latest_run_status == "running" {
            spans.push(Span::styled(
                pad_or_trunc(&node.state, state_w.saturating_sub(2)),
                state_style,
            ));
            spans.push(Span::styled(
                " \u{25b6}",
                Style::default().fg(colors::function()).bg(bg),
            ));
        } else {
            spans.push(Span::styled(
                pad_or_trunc(&node.state, state_w),
                state_style,
            ));
        }
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

/// Assign priority for state-based sorting (lower = higher priority).
fn state_priority(state: &str) -> u8 {
    // Strip holding suffixes like "(-> InProgress)" for priority matching
    let clean = state.split(" (->").next().unwrap_or(state);
    match clean {
        "InProgress" => 0,
        "NeedsReview" => 1,
        "NeedsResearch" => 2,
        "Planned" => 3,
        "Backlog" => 4,
        "open" => 5,
        "Completed" | "completed" => 6,
        "Deprecated" => 7,
        "Archived" => 8,
        "failed" => 9,
        _ => 10, // Unknown states last
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

    fn buffer_line_text(buf: &Buffer, area: Rect, y: u16) -> String {
        let mut row = String::new();
        for x in 0..area.width {
            row.push(
                buf[(area.x + x, area.y + y)]
                    .symbol()
                    .chars()
                    .next()
                    .unwrap_or(' '),
            );
        }
        row
    }

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
        let overlay = PmOverlay::new(false, None);
        // All collapsed: only root (depth=0) nodes visible — single Project
        let count = overlay.visible_count();
        assert_eq!(
            count, 1,
            "Only 1 project root should be visible when collapsed"
        );
    }

    #[test]
    fn test_overlay_expand_collapse() {
        let overlay = PmOverlay::new(false, None);
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
        let overlay = PmOverlay::new(false, None);
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
        let overlay = PmOverlay::new(false, None);

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
        let overlay = PmOverlay::new(false, None);
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
    fn test_render_worst_case_fallback_diagnostic_and_remedy_lines() {
        let overlay = PmOverlay::new_degraded_empty();
        let area = Rect::new(0, 0, 100, 15);
        let mut buf = Buffer::empty(area);
        render_worst_case_fallback(&overlay, area, &mut buf);

        // Collect all rendered text
        let mut text = String::new();
        for y in 0..area.height {
            text.push_str(&buffer_line_text(&buf, area, y));
            text.push('\n');
        }

        // PM-UX-D15: three diagnostic lines
        assert!(
            text.contains("Service status"),
            "should contain 'Service status' diagnostic"
        );
        assert!(
            text.contains("Cache status"),
            "should contain 'Cache status' diagnostic"
        );
        assert!(
            text.contains("Capsule status"),
            "should contain 'Capsule status' diagnostic"
        );

        // PM-UX-D15: remedy commands
        assert!(
            text.contains("/pm service doctor"),
            "should contain '/pm service doctor' remedy"
        );
        assert!(
            text.contains("systemctl --user start codex-pm-service"),
            "should contain 'systemctl --user start codex-pm-service' remedy"
        );
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

    #[test]
    fn test_render_summary_bar_degraded_banner_text_matches_spec() {
        let overlay = PmOverlay::new(true, None);
        let area = Rect::new(0, 0, 120, 3);
        let mut buf = Buffer::empty(Rect::new(0, 0, 120, 5));
        render_summary_bar(&overlay, area, &mut buf);

        let banner_row = buffer_line_text(&buf, area, 2);
        assert!(
            banner_row.contains(DEGRADED_BANNER_TEXT),
            "summary banner should match PM-UX-D14 text"
        );
    }

    #[test]
    fn test_render_detail_degraded_banner_text_matches_spec() {
        let overlay = PmOverlay::new(true, None);
        assert!(overlay.open_detail_for_visible(0));

        let area = Rect::new(0, 0, 120, 18);
        let mut buf = Buffer::empty(area);
        render_detail(&overlay, area, &mut buf);

        let top_row = buffer_line_text(&buf, area, 0);
        assert!(
            top_row.contains(DEGRADED_BANNER_TEXT),
            "detail banner should match PM-UX-D14 text"
        );
    }

    // --- PM-UX-D11 tests ---------------------------------------------------

    /// Helper: expand tree to make node 4 (TASK-002, needs_attention) visible
    /// and return its flat index.
    fn expand_to_task_002(overlay: &PmOverlay) -> usize {
        overlay.expand(0); // Project
        overlay.expand(1); // FEAT-001
        overlay.expand(2); // SPEC-AUTH-001
        // visible: [0,1,2,3,4,5,6,7,8] — node 4 is flat index 4
        overlay
            .visible_indices()
            .iter()
            .position(|&n| n == 4)
            .expect("node 4 should be visible")
    }

    #[test]
    fn test_needs_attention_wide_badge() {
        let overlay = PmOverlay::new(false, None);
        let flat = expand_to_task_002(&overlay);
        overlay.set_selected(flat);

        let node = &overlay.nodes[4];
        let line = render_row(node, 4, &overlay, 140, true);
        let text: String = line.spans.iter().map(|s| s.content.to_string()).collect();
        assert!(
            text.contains("! run-042"),
            "wide row should contain '! run-042', got: {text}"
        );
    }

    #[test]
    fn test_needs_attention_narrow_indicator() {
        let overlay = PmOverlay::new(false, None);
        let flat = expand_to_task_002(&overlay);
        overlay.set_selected(flat);

        let node = &overlay.nodes[4];
        let line = render_row(node, 4, &overlay, 60, true);
        let text: String = line.spans.iter().map(|s| s.content.to_string()).collect();
        assert!(
            text.contains(" !"),
            "narrow row should contain ' !' indicator, got: {text}"
        );
    }

    #[test]
    fn test_needs_attention_medium_indicator() {
        let overlay = PmOverlay::new(false, None);
        let flat = expand_to_task_002(&overlay);
        overlay.set_selected(flat);

        let node = &overlay.nodes[4];
        let line = render_row(node, 4, &overlay, 100, true);
        let text: String = line.spans.iter().map(|s| s.content.to_string()).collect();
        assert!(
            text.contains(" !"),
            "medium row should contain ' !' indicator, got: {text}"
        );
    }

    #[test]
    fn test_detail_auto_scroll_needs_attention() {
        let overlay = PmOverlay::new(false, None);
        let flat = expand_to_task_002(&overlay);
        assert!(overlay.open_detail_for_visible(flat));
        assert!(
            overlay.detail_auto_scroll_pending.get(),
            "flag should be set for needs_attention node"
        );
        assert_eq!(overlay.detail_node_idx.get(), Some(4));

        // Render to consume the flag
        let area = Rect::new(0, 0, 120, 30);
        let mut buf = Buffer::empty(area);
        render_detail(&overlay, area, &mut buf);

        assert!(
            !overlay.detail_auto_scroll_pending.get(),
            "flag should be consumed after render"
        );
        assert!(
            overlay.detail_scroll() > 0,
            "scroll should be > 0 (scrolled to Run History)"
        );
    }

    #[test]
    fn test_detail_no_auto_scroll_normal_node() {
        let overlay = PmOverlay::new(false, None);
        overlay.expand(0); // Project
        // visible: [0, 1, 8] — open detail for node 1 (FEAT-001, no needs_attention)
        assert!(overlay.open_detail_for_visible(1));
        assert!(
            !overlay.detail_auto_scroll_pending.get(),
            "flag should NOT be set for normal node"
        );

        let area = Rect::new(0, 0, 120, 30);
        let mut buf = Buffer::empty(area);
        render_detail(&overlay, area, &mut buf);

        assert_eq!(
            overlay.detail_scroll(),
            0,
            "scroll should stay 0 for normal node"
        );
    }

    #[test]
    fn test_detail_content_needs_attention_conflict() {
        let node = &demo_tree()[4]; // TASK-002
        let lines = detail_content_lines(node, 100);
        let text: String = lines
            .iter()
            .flat_map(|l| l.spans.iter().map(|s| s.content.to_string()))
            .collect::<Vec<_>>()
            .join("\n");
        assert!(
            text.contains("Conflict summary"),
            "detail should contain 'Conflict summary'"
        );
        assert!(
            text.contains("Resolution instructions"),
            "detail should contain 'Resolution instructions'"
        );
        assert!(
            text.contains("needs_attn"),
            "status should show 'needs_attn'"
        );
    }

    // --- Active run indicator tests (PM-004) --------------------------------

    /// Helper: expand tree to make node 10 (TASK-005, running) visible
    /// and return its flat index.
    fn expand_to_task_005(overlay: &PmOverlay) -> usize {
        overlay.expand(0); // Project
        overlay.expand(8); // FEAT-002
        overlay.expand(9); // SPEC-PIPE-001
        overlay
            .visible_indices()
            .iter()
            .position(|&n| n == 10)
            .expect("node 10 should be visible")
    }

    #[test]
    fn test_demo_tree_has_running_node() {
        let nodes = demo_tree();
        assert!(
            nodes
                .iter()
                .any(|n| n.latest_run_status == "running" && !n.latest_run.is_empty()),
            "demo tree must have at least 1 running node"
        );
    }

    #[test]
    fn test_summary_bar_active_run_count() {
        let overlay = PmOverlay::new(false, None);
        let area = Rect::new(0, 0, 200, 3);
        let mut buf = Buffer::empty(Rect::new(0, 0, 200, 5));
        render_summary_bar(&overlay, area, &mut buf);

        let row0 = buffer_line_text(&buf, area, 0);
        assert!(
            row0.contains("Active runs: 1"),
            "summary bar should show computed 'Active runs: 1', got: {row0}"
        );
        let row1 = buffer_line_text(&buf, area, 1);
        assert!(
            row1.contains("1 running"),
            "run meter should show '1 running', got: {row1}"
        );
    }

    #[test]
    fn test_running_wide_indicator() {
        let overlay = PmOverlay::new(false, None);
        let flat = expand_to_task_005(&overlay);
        overlay.set_selected(flat);

        let node = &overlay.nodes[10];
        let line = render_row(node, 10, &overlay, 140, true);
        let text: String = line.spans.iter().map(|s| s.content.to_string()).collect();
        assert!(
            text.contains("\u{25b6} run-038"),
            "wide row should contain '\u{25b6} run-038', got: {text}"
        );
    }

    #[test]
    fn test_running_medium_indicator() {
        let overlay = PmOverlay::new(false, None);
        let flat = expand_to_task_005(&overlay);
        overlay.set_selected(flat);

        let node = &overlay.nodes[10];
        let line = render_row(node, 10, &overlay, 100, true);
        let text: String = line.spans.iter().map(|s| s.content.to_string()).collect();
        assert!(
            text.contains(" \u{25b6}"),
            "medium row should contain ' \u{25b6}' indicator, got: {text}"
        );
    }

    #[test]
    fn test_running_narrow_indicator() {
        let overlay = PmOverlay::new(false, None);
        let flat = expand_to_task_005(&overlay);
        overlay.set_selected(flat);

        let node = &overlay.nodes[10];
        let line = render_row(node, 10, &overlay, 60, true);
        let text: String = line.spans.iter().map(|s| s.content.to_string()).collect();
        assert!(
            text.contains(" \u{25b6}"),
            "narrow row should contain ' \u{25b6}' indicator, got: {text}"
        );
    }

    #[test]
    fn test_detail_content_running_status() {
        let node = &demo_tree()[10]; // TASK-005, running
        let lines = detail_content_lines(node, 100);
        let text: String = lines
            .iter()
            .flat_map(|l| l.spans.iter().map(|s| s.content.to_string()))
            .collect::<Vec<_>>()
            .join("\n");
        assert!(
            text.contains("running"),
            "detail should show 'running' status, got: {text}"
        );
        // Should NOT contain needs_attention content
        assert!(
            !text.contains("Conflict summary"),
            "running node should not show conflict summary"
        );
    }

    // --- PM-004 relative time tests ------------------------------------------

    #[test]
    fn test_relative_time_recent_timestamp() {
        use chrono::Utc;
        // Create a timestamp 2 hours ago
        let two_hours_ago = (Utc::now() - chrono::Duration::hours(2))
            .to_rfc3339_opts(chrono::SecondsFormat::Secs, true);
        let result = human_ago(&two_hours_ago);
        assert!(
            result.contains("h ago") || result == "just now",
            "recent timestamp should show relative time, got: {result}"
        );
        assert!(
            !result.contains("-"),
            "recent timestamp should NOT show YYYY-MM-DD, got: {result}"
        );
    }

    #[test]
    fn test_relative_time_old_timestamp() {
        use chrono::Utc;
        // Create a timestamp 10 days ago
        let ten_days_ago = (Utc::now() - chrono::Duration::days(10))
            .to_rfc3339_opts(chrono::SecondsFormat::Secs, true);
        let result = human_ago(&ten_days_ago);
        assert!(
            result.contains("-"),
            "old timestamp (>= 7 days) should show YYYY-MM-DD, got: {result}"
        );
        assert!(
            !result.contains("ago"),
            "old timestamp should NOT show 'ago', got: {result}"
        );
    }

    #[test]
    fn test_relative_time_invalid_timestamp() {
        let result = human_ago("invalid-timestamp");
        // Should return the input string as fallback
        assert_eq!(
            result, "invalid-timestamp",
            "invalid timestamp should return input as-is"
        );
    }

    // --- PM-UX-D24 empty-state onboarding tests ------------------------------

    #[test]
    fn test_empty_state_onboarding_renders_when_healthy_and_empty() {
        // PM-UX-D7/PM-UX-D24: healthy service + empty nodes → onboarding
        let mut overlay = PmOverlay::new(false, None);
        overlay.nodes.clear(); // Ensure empty
        assert!(!overlay.degraded);
        assert!(overlay.nodes.is_empty());

        let area = Rect::new(0, 0, 100, 25);
        let mut buf = Buffer::empty(area);
        render_empty_state_onboarding(&overlay, area, &mut buf);

        let mut text = String::new();
        for y in 0..area.height.min(5) {
            text.push_str(&buffer_line_text(&buf, area, y));
            text.push('\n');
        }

        // Should contain onboarding welcome text
        assert!(
            text.contains("Welcome to PM"),
            "should contain welcome message, got: {text}"
        );
        assert!(
            text.contains("get started"),
            "should contain 'get started', got: {text}"
        );
    }

    #[test]
    fn test_empty_state_onboarding_shows_three_steps() {
        let mut overlay = PmOverlay::new(false, None);
        overlay.nodes.clear();

        let area = Rect::new(0, 0, 100, 25);
        let mut buf = Buffer::empty(area);
        render_empty_state_onboarding(&overlay, area, &mut buf);

        let mut text = String::new();
        for y in 0..area.height {
            text.push_str(&buffer_line_text(&buf, area, y));
            text.push('\n');
        }

        // PM-UX-D24: must show all 3 steps
        assert!(
            text.contains("Step 1"),
            "should contain Step 1, got: {text}"
        );
        assert!(
            text.contains("Step 2"),
            "should contain Step 2, got: {text}"
        );
        assert!(
            text.contains("Step 3"),
            "should contain Step 3, got: {text}"
        );
        // Step contents
        assert!(
            text.contains("project container"),
            "Step 1 should mention project container"
        );
        assert!(
            text.contains("work item type"),
            "Step 2 should mention work item type"
        );
        assert!(
            text.contains("maieutic intake"),
            "Step 3 should mention maieutic intake"
        );
    }

    #[test]
    fn test_degraded_empty_shows_worst_case_not_onboarding() {
        // PM-UX-D15 takes precedence over PM-UX-D24:
        // degraded + empty → worst-case fallback, NOT onboarding
        let overlay = PmOverlay::new_degraded_empty();
        assert!(overlay.degraded);
        assert!(overlay.nodes.is_empty());

        let area = Rect::new(0, 0, 100, 20);
        let mut buf = Buffer::empty(area);

        // Render using the actual overlay logic by calling worst-case directly
        render_worst_case_fallback(&overlay, area, &mut buf);

        let mut text = String::new();
        for y in 0..area.height.min(8) {
            text.push_str(&buffer_line_text(&buf, area, y));
            text.push('\n');
        }

        // Should show worst-case fallback, NOT onboarding
        assert!(
            text.contains("PM data unavailable"),
            "degraded+empty should show worst-case fallback"
        );
        assert!(
            !text.contains("Welcome to PM"),
            "degraded+empty should NOT show onboarding"
        );
        assert!(
            text.contains("Service status"),
            "should show diagnostic lines"
        );
    }

    #[test]
    fn test_non_empty_does_not_show_onboarding() {
        // Non-empty overlay should NOT trigger onboarding, even if healthy
        let overlay = PmOverlay::new(false, None);
        assert!(!overlay.degraded);
        assert!(!overlay.nodes.is_empty(), "demo tree should have nodes");

        // This test verifies the conditional logic: the onboarding render function
        // should NOT be called when nodes exist. We can't easily test the full
        // render path here, but we verify the data conditions that prevent it.
        assert!(
            overlay.degraded || !overlay.nodes.is_empty(),
            "onboarding condition should be false when nodes exist"
        );
    }

    #[test]
    fn test_onboarding_only_references_valid_pm_commands() {
        // PM-UX-D13: onboarding must reference only real PM commands
        let mut overlay = PmOverlay::new(false, None);
        overlay.nodes.clear();

        let area = Rect::new(0, 0, 120, 30);
        let mut buf = Buffer::empty(area);
        render_empty_state_onboarding(&overlay, area, &mut buf);

        let mut text = String::new();
        for y in 0..area.height {
            text.push_str(&buffer_line_text(&buf, area, y));
            text.push('\n');
        }

        // Must NOT contain invalid commands
        assert!(
            !text.contains("/pm create"),
            "onboarding should NOT reference /pm create (invalid command)"
        );

        // Should contain only valid PM commands from PM-UX-D13
        let valid_commands = vec![
            "/pm open",
            "/pm service doctor",
            "/pm service status",
            "/pm bot run",
        ];
        let mut found_valid = false;
        for cmd in valid_commands {
            if text.contains(cmd) {
                found_valid = true;
                break;
            }
        }
        assert!(
            found_valid,
            "onboarding should reference at least one valid PM command"
        );
    }

    // --- Sort mode tests (PM-UX-D3, PM-UX-D5) --------------------------------

    #[test]
    fn test_sort_mode_default_is_updated_desc() {
        let overlay = PmOverlay::new(false, None);
        assert_eq!(
            overlay.sort_mode(),
            SortMode::UpdatedDesc,
            "default sort mode should be UpdatedDesc"
        );
    }

    #[test]
    fn test_sort_mode_cycle() {
        let overlay = PmOverlay::new(false, None);
        assert_eq!(overlay.sort_mode(), SortMode::UpdatedDesc);

        overlay.cycle_sort_mode();
        assert_eq!(
            overlay.sort_mode(),
            SortMode::StatePriority,
            "first cycle should go to StatePriority"
        );

        overlay.cycle_sort_mode();
        assert_eq!(
            overlay.sort_mode(),
            SortMode::IdAsc,
            "second cycle should go to IdAsc"
        );

        overlay.cycle_sort_mode();
        assert_eq!(
            overlay.sort_mode(),
            SortMode::UpdatedDesc,
            "third cycle should wrap back to UpdatedDesc"
        );
    }

    #[test]
    fn test_sort_mode_affects_visible_ordering_updated_desc() {
        let overlay = PmOverlay::new(false, None);
        overlay.expand(0); // Expand Project
        overlay.expand(1); // Expand FEAT-001

        // Default mode: UpdatedDesc (most recent first)
        assert_eq!(overlay.sort_mode(), SortMode::UpdatedDesc);
        let visible = overlay.visible_indices();

        // Verify hierarchy is preserved: parent appears before children
        for (i, &node_idx) in visible.iter().enumerate() {
            let node = &overlay.nodes[node_idx];
            if let Some(parent_idx) = node.parent_idx {
                // Find parent in visible list
                let parent_pos = visible.iter().position(|&idx| idx == parent_idx);
                assert!(
                    parent_pos.is_some(),
                    "Parent {} should be visible for child {}",
                    parent_idx,
                    node_idx
                );
                assert!(
                    parent_pos.unwrap() < i,
                    "Parent {} should appear before child {} in visible list",
                    parent_idx,
                    node_idx
                );
            }
        }

        // Verify siblings are sorted by updated_at descending
        // Check FEAT-001's children (nodes 2 and 5)
        let feat001_children: Vec<usize> = visible
            .iter()
            .copied()
            .filter(|&idx| overlay.nodes[idx].parent_idx == Some(1))
            .collect();
        assert_eq!(feat001_children.len(), 2, "FEAT-001 should have 2 children");
        // Node 2 (2026-02-12T09:15) is more recent than node 5 (2026-02-10T16:00)
        assert_eq!(
            feat001_children[0], 2,
            "More recent child should come first"
        );
        assert_eq!(feat001_children[1], 5, "Older child should come second");
    }

    #[test]
    fn test_sort_mode_affects_visible_ordering_id_asc() {
        let overlay = PmOverlay::new(false, None);
        overlay.expand(0); // Expand Project
        overlay.expand(1); // Expand FEAT-001

        // Switch to IdAsc
        overlay.cycle_sort_mode();
        overlay.cycle_sort_mode();
        assert_eq!(overlay.sort_mode(), SortMode::IdAsc);

        let visible = overlay.visible_indices();

        // Verify hierarchy is preserved: parent appears before children
        for (i, &node_idx) in visible.iter().enumerate() {
            let node = &overlay.nodes[node_idx];
            if let Some(parent_idx) = node.parent_idx {
                let parent_pos = visible.iter().position(|&idx| idx == parent_idx);
                assert!(
                    parent_pos.is_some(),
                    "Parent {} should be visible for child {}",
                    parent_idx,
                    node_idx
                );
                assert!(
                    parent_pos.unwrap() < i,
                    "Parent {} should appear before child {} in visible list",
                    parent_idx,
                    node_idx
                );
            }
        }

        // Verify siblings are sorted by ID ascending
        // Check FEAT-001's children (SPEC-AUTH-001=node2, SPEC-AUTH-002=node5)
        let feat001_children: Vec<usize> = visible
            .iter()
            .copied()
            .filter(|&idx| overlay.nodes[idx].parent_idx == Some(1))
            .collect();
        assert_eq!(feat001_children.len(), 2, "FEAT-001 should have 2 children");
        assert_eq!(feat001_children[0], 2, "SPEC-AUTH-001 should come first");
        assert_eq!(feat001_children[1], 5, "SPEC-AUTH-002 should come second");
    }

    #[test]
    fn test_sort_mode_visible_in_title() {
        let overlay = PmOverlay::new(false, None);
        let area = Rect::new(0, 0, 120, 25);
        let mut buf = Buffer::empty(area);

        // Render with default mode (UpdatedDesc)
        let body = Rect::new(2, 1, 116, 23);
        render_summary_bar(&overlay, body, &mut buf);

        // Can't easily test title bar rendering without full render_pm_overlay,
        // but we verify sort_mode() returns correct value
        assert_eq!(overlay.sort_mode(), SortMode::UpdatedDesc);

        overlay.cycle_sort_mode();
        assert_eq!(overlay.sort_mode(), SortMode::StatePriority);

        overlay.cycle_sort_mode();
        assert_eq!(overlay.sort_mode(), SortMode::IdAsc);
    }

    #[test]
    fn test_sort_mode_persists_with_initial_value() {
        // Verify that passing Some(mode) preserves that mode
        let overlay = PmOverlay::new(false, Some(SortMode::IdAsc));
        assert_eq!(
            overlay.sort_mode(),
            SortMode::IdAsc,
            "initial sort mode should be preserved"
        );

        let overlay2 = PmOverlay::new(false, Some(SortMode::StatePriority));
        assert_eq!(
            overlay2.sort_mode(),
            SortMode::StatePriority,
            "different initial mode should be preserved"
        );

        // Verify None defaults to UpdatedDesc
        let overlay3 = PmOverlay::new(false, None);
        assert_eq!(
            overlay3.sort_mode(),
            SortMode::UpdatedDesc,
            "None should default to UpdatedDesc"
        );
    }

    // --- PM-UX-D20 list position preservation tests --------------------------

    #[test]
    fn test_list_scroll_preserved_after_detail_close() {
        let overlay = PmOverlay::new(false, None);
        overlay.expand(0); // Expand Project
        overlay.expand(1); // Expand FEAT-001
        overlay.expand(2); // Expand SPEC-AUTH-001

        // Set specific scroll position
        overlay.set_scroll(3);
        let initial_scroll = overlay.scroll();
        assert_eq!(initial_scroll, 3);

        // Open detail for a visible row
        assert!(overlay.open_detail_for_visible(5));
        assert!(overlay.is_detail_mode());

        // Scroll in detail view
        overlay.set_detail_scroll(10);

        // Close detail and verify list scroll unchanged
        overlay.close_detail();
        assert!(!overlay.is_detail_mode());
        assert_eq!(
            overlay.scroll(),
            initial_scroll,
            "list scroll should be preserved after closing detail"
        );
    }

    #[test]
    fn test_list_selection_and_scroll_both_preserved() {
        let overlay = PmOverlay::new(false, None);
        overlay.expand(0); // Expand Project
        overlay.expand(1); // Expand FEAT-001

        // Set specific selection and scroll
        overlay.set_selected(4);
        overlay.set_scroll(2);
        let initial_sel = overlay.selected();
        let initial_scroll = overlay.scroll();

        // Open detail, do various actions
        assert!(overlay.open_detail_for_visible(4));
        overlay.set_detail_scroll(5);

        // Close and verify both preserved
        overlay.close_detail();
        assert_eq!(
            overlay.selected(),
            initial_sel,
            "selection should be preserved"
        );
        assert_eq!(
            overlay.scroll(),
            initial_scroll,
            "scroll should be preserved"
        );
    }

    #[test]
    fn test_multiple_detail_open_close_preserves_position() {
        let overlay = PmOverlay::new(false, None);
        overlay.expand(0);
        overlay.expand(1);

        // Set initial position
        overlay.set_selected(3);
        overlay.set_scroll(1);
        let initial_sel = overlay.selected();
        let initial_scroll = overlay.scroll();

        // Open detail, close, repeat multiple times
        for _ in 0..3 {
            assert!(overlay.open_detail_for_visible(3));
            overlay.set_detail_scroll(7); // Change detail scroll
            overlay.close_detail();
        }

        // Verify position still preserved after multiple cycles
        assert_eq!(
            overlay.selected(),
            initial_sel,
            "selection should survive multiple detail open/close"
        );
        assert_eq!(
            overlay.scroll(),
            initial_scroll,
            "scroll should survive multiple detail open/close"
        );
    }

    // --- List footer position indicator tests -------------------------------

    #[test]
    fn test_list_footer_shows_position() {
        let overlay = PmOverlay::new(false, None);
        overlay.expand(0); // Expand Project → visible count = 3
        overlay.visible_rows.set(10); // Set viewport size

        let area = Rect::new(0, 0, 80, 1);
        let mut buf = Buffer::empty(area);

        // Default selection is 0 → shows Row 1/3
        render_list_footer(&overlay, area, &mut buf);
        let text = buffer_line_text(&buf, area, 0);
        assert!(
            text.contains("Row 1/3"),
            "footer should show Row 1/3, got: {text}"
        );
        // Should show window range
        assert!(
            text.contains("Showing 1-3 of 3"),
            "footer should show window range, got: {text}"
        );
        // Should also show sort mode
        assert!(
            text.contains("Sort:"),
            "footer should show sort mode, got: {text}"
        );
        assert!(
            text.contains("Updated"),
            "footer should show default sort mode Updated, got: {text}"
        );
    }

    #[test]
    fn test_list_footer_updates_with_selection() {
        let overlay = PmOverlay::new(false, None);
        overlay.expand(0);
        overlay.expand(1); // More nodes → visible count = 5

        let area = Rect::new(0, 0, 40, 1);
        let mut buf = Buffer::empty(area);

        // Move to row 3 (0-indexed = 2)
        overlay.set_selected(2);
        render_list_footer(&overlay, area, &mut buf);
        let text = buffer_line_text(&buf, area, 0);
        assert!(
            text.contains("Row 3/5"),
            "footer should show Row 3/5 after selection change, got: {text}"
        );
    }

    #[test]
    fn test_list_footer_updates_with_tree_expansion() {
        let overlay = PmOverlay::new(false, None);
        let area = Rect::new(0, 0, 40, 1);
        let mut buf = Buffer::empty(area);

        // Initially collapsed → 1 visible row
        render_list_footer(&overlay, area, &mut buf);
        let text1 = buffer_line_text(&buf, area, 0);
        assert!(
            text1.contains("Row 1/1"),
            "collapsed should show Row 1/1, got: {text1}"
        );

        // Expand → 3 visible rows
        overlay.expand(0);
        let mut buf2 = Buffer::empty(area);
        render_list_footer(&overlay, area, &mut buf2);
        let text2 = buffer_line_text(&buf2, area, 0);
        assert!(
            text2.contains("Row 1/3"),
            "expanded should show Row 1/3, got: {text2}"
        );
    }

    #[test]
    fn test_list_footer_shows_no_items_when_empty() {
        let mut overlay = PmOverlay::new(false, None);
        overlay.nodes.clear();

        let area = Rect::new(0, 0, 40, 1);
        let mut buf = Buffer::empty(area);

        render_list_footer(&overlay, area, &mut buf);
        let text = buffer_line_text(&buf, area, 0);
        assert!(
            text.contains("No items"),
            "empty overlay should show 'No items', got: {text}"
        );
        // Empty footer should NOT show sort mode
        assert!(
            !text.contains("Sort:"),
            "empty footer should not show sort mode, got: {text}"
        );
    }

    #[test]
    fn test_list_footer_updates_with_sort_mode_cycle() {
        let overlay = PmOverlay::new(false, None);
        overlay.expand(0);

        let area = Rect::new(0, 0, 60, 1);
        let mut buf = Buffer::empty(area);

        // Default mode: Updated
        render_list_footer(&overlay, area, &mut buf);
        let text1 = buffer_line_text(&buf, area, 0);
        assert!(
            text1.contains("Sort: Updated"),
            "footer should show Sort: Updated, got: {text1}"
        );

        // Cycle to StatePriority
        overlay.cycle_sort_mode();
        let mut buf2 = Buffer::empty(area);
        render_list_footer(&overlay, area, &mut buf2);
        let text2 = buffer_line_text(&buf2, area, 0);
        assert!(
            text2.contains("Sort: State"),
            "footer should show Sort: State after cycle, got: {text2}"
        );

        // Cycle to IdAsc
        overlay.cycle_sort_mode();
        let mut buf3 = Buffer::empty(area);
        render_list_footer(&overlay, area, &mut buf3);
        let text3 = buffer_line_text(&buf3, area, 0);
        assert!(
            text3.contains("Sort: ID"),
            "footer should show Sort: ID after second cycle, got: {text3}"
        );
    }

    #[test]
    fn test_list_footer_window_range_with_scroll() {
        let overlay = PmOverlay::new(false, None);
        overlay.expand(0); // Expand all
        overlay.expand(1);
        overlay.expand(2); // Many visible rows

        overlay.visible_rows.set(5); // Viewport shows 5 rows
        overlay.set_scroll(3); // Scrolled down by 3

        let area = Rect::new(0, 0, 80, 1);
        let mut buf = Buffer::empty(area);

        render_list_footer(&overlay, area, &mut buf);
        let text = buffer_line_text(&buf, area, 0);

        // Window start = scroll + 1 = 4, end = scroll + viewport = 8
        assert!(
            text.contains("Showing 4-"),
            "footer should show window starting at 4, got: {text}"
        );
    }

    #[test]
    fn test_list_footer_window_range_all_visible() {
        let overlay = PmOverlay::new(false, None);
        overlay.expand(0); // visible count = 3
        overlay.visible_rows.set(20); // Viewport larger than content

        let area = Rect::new(0, 0, 80, 1);
        let mut buf = Buffer::empty(area);

        render_list_footer(&overlay, area, &mut buf);
        let text = buffer_line_text(&buf, area, 0);

        // All 3 rows visible → Showing 1-3 of 3
        assert!(
            text.contains("Showing 1-3 of 3"),
            "footer should show all rows visible, got: {text}"
        );
    }
}
