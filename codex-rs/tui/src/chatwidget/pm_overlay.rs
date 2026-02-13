//! SPEC-PM-004: PM overview overlay
//!
//! Read-only list view showing a hierarchical tree of work items
//! (Feature > Spec > Task) with adaptive columns, summary bar, and
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
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum NodeType {
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

    // Feature 0
    nodes.push(TreeNode {
        id: "FEAT-001".into(),
        title: "User Authentication".into(),
        node_type: NodeType::Feature,
        state: "InProgress".into(),
        updated_at: "2026-02-12T10:30:00Z".into(),
        latest_run: String::new(),
        depth: 0,
        parent_idx: None,
        children: vec![1, 4],
    });
    // Spec 1
    nodes.push(TreeNode {
        id: "SPEC-AUTH-001".into(),
        title: "OAuth2 Integration".into(),
        node_type: NodeType::Spec,
        state: "InProgress".into(),
        updated_at: "2026-02-12T09:15:00Z".into(),
        latest_run: "run-042".into(),
        depth: 1,
        parent_idx: Some(0),
        children: vec![2, 3],
    });
    // Task 2
    nodes.push(TreeNode {
        id: "TASK-001".into(),
        title: "Implement token refresh".into(),
        node_type: NodeType::Task,
        state: "Done".into(),
        updated_at: "2026-02-11T14:00:00Z".into(),
        latest_run: "run-041".into(),
        depth: 2,
        parent_idx: Some(1),
        children: vec![],
    });
    // Task 3
    nodes.push(TreeNode {
        id: "TASK-002".into(),
        title: "Add PKCE flow".into(),
        node_type: NodeType::Task,
        state: "InProgress".into(),
        updated_at: "2026-02-12T09:15:00Z".into(),
        latest_run: "run-042".into(),
        depth: 2,
        parent_idx: Some(1),
        children: vec![],
    });
    // Spec 4
    nodes.push(TreeNode {
        id: "SPEC-AUTH-002".into(),
        title: "Session Management".into(),
        node_type: NodeType::Spec,
        state: "Draft".into(),
        updated_at: "2026-02-10T16:00:00Z".into(),
        latest_run: String::new(),
        depth: 1,
        parent_idx: Some(0),
        children: vec![5],
    });
    // Task 5
    nodes.push(TreeNode {
        id: "TASK-003".into(),
        title: "Design session store".into(),
        node_type: NodeType::Task,
        state: "Draft".into(),
        updated_at: "2026-02-10T16:00:00Z".into(),
        latest_run: String::new(),
        depth: 2,
        parent_idx: Some(4),
        children: vec![],
    });

    // Feature 6
    nodes.push(TreeNode {
        id: "FEAT-002".into(),
        title: "Pipeline Orchestration".into(),
        node_type: NodeType::Feature,
        state: "Blocked".into(),
        updated_at: "2026-02-11T11:00:00Z".into(),
        latest_run: String::new(),
        depth: 0,
        parent_idx: None,
        children: vec![7, 9],
    });
    // Spec 7
    nodes.push(TreeNode {
        id: "SPEC-PIPE-001".into(),
        title: "Stage execution engine".into(),
        node_type: NodeType::Spec,
        state: "Blocked".into(),
        updated_at: "2026-02-11T11:00:00Z".into(),
        latest_run: "run-038".into(),
        depth: 1,
        parent_idx: Some(6),
        children: vec![8],
    });
    // Task 8
    nodes.push(TreeNode {
        id: "TASK-004".into(),
        title: "Implement retry logic".into(),
        node_type: NodeType::Task,
        state: "Blocked".into(),
        updated_at: "2026-02-11T11:00:00Z".into(),
        latest_run: "run-038".into(),
        depth: 2,
        parent_idx: Some(7),
        children: vec![],
    });
    // Spec 9
    nodes.push(TreeNode {
        id: "SPEC-PIPE-002".into(),
        title: "Guardrail validation".into(),
        node_type: NodeType::Spec,
        state: "Done".into(),
        updated_at: "2026-02-09T08:00:00Z".into(),
        latest_run: "run-035".into(),
        depth: 1,
        parent_idx: Some(6),
        children: vec![10, 11],
    });
    // Task 10
    nodes.push(TreeNode {
        id: "TASK-005".into(),
        title: "Schema validation".into(),
        node_type: NodeType::Task,
        state: "Done".into(),
        updated_at: "2026-02-09T08:00:00Z".into(),
        latest_run: "run-034".into(),
        depth: 2,
        parent_idx: Some(9),
        children: vec![],
    });
    // Task 11
    nodes.push(TreeNode {
        id: "TASK-006".into(),
        title: "Evidence checks".into(),
        node_type: NodeType::Task,
        state: "Done".into(),
        updated_at: "2026-02-08T17:30:00Z".into(),
        latest_run: "run-033".into(),
        depth: 2,
        parent_idx: Some(9),
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

        // Title bar
        let dim = Style::default().fg(colors::text_dim());
        let bright = Style::default().fg(colors::text());
        let accent = Style::default().fg(colors::function());
        let title = RLine::from(vec![
            Span::styled(" PM Overview ", bright),
            Span::styled("--- ", dim),
            Span::styled("Up/Dn", accent),
            Span::styled(" navigate  ", dim),
            Span::styled("Left/Right", accent),
            Span::styled(" expand/collapse  ", dim),
            Span::styled("Esc", bright),
            Span::styled(" close ", dim),
        ]);

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

// ---------------------------------------------------------------------------
// Summary bar
// ---------------------------------------------------------------------------

fn render_summary_bar(overlay: &PmOverlay, area: Rect, buf: &mut Buffer) {
    // Count states
    let mut counts: std::collections::HashMap<&str, usize> = std::collections::HashMap::new();
    for node in &overlay.nodes {
        *counts.entry(node.state.as_str()).or_default() += 1;
    }

    let dim = Style::default().fg(colors::text_dim());

    // Line 1: state chips + active runs
    let mut chips: Vec<Span<'static>> = Vec::new();
    for state in &["Draft", "InProgress", "Done", "Blocked", "Failed"] {
        if let Some(&count) = counts.get(state) {
            if !chips.is_empty() {
                chips.push(Span::styled("  ", dim));
            }
            let color = state_color(state);
            chips.push(Span::styled(
                format!("{state}:{count}"),
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
    match state {
        "Done" => colors::success(),
        "InProgress" => colors::function(),
        "Draft" => colors::text_dim(),
        "Blocked" | "Failed" => colors::error(),
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
        assert!(nodes.len() >= 10, "Demo tree should have >= 10 nodes");
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
        // All collapsed: only root (depth=0) nodes visible
        let count = overlay.visible_count();
        assert_eq!(count, 2, "Only 2 features should be visible when collapsed");
    }

    #[test]
    fn test_overlay_expand_collapse() {
        let overlay = PmOverlay::new(false);
        assert_eq!(overlay.visible_count(), 2);

        // Expand first feature
        assert!(overlay.expand(0));
        // Now feature + its 2 specs visible = 4
        assert_eq!(overlay.visible_count(), 4);

        // Expand first spec (idx 1)
        assert!(overlay.expand(1));
        // Now feature + spec1 + 2 tasks + spec2 = 6
        assert_eq!(overlay.visible_count(), 6);

        // Collapse feature
        assert!(overlay.collapse(0));
        assert_eq!(overlay.visible_count(), 2);
    }

    #[test]
    fn test_node_idx_of_visible_second_root() {
        let overlay = PmOverlay::new(false);
        // Collapsed: visible = [0, 6] (two root features)
        assert_eq!(overlay.node_idx_of_visible(0), Some(0));
        assert_eq!(overlay.node_idx_of_visible(1), Some(6));
        assert_eq!(overlay.visible_count(), 2);

        // Expand visible index 1 → should expand node 6 (FEAT-002)
        assert!(overlay.expand_visible(1));
        // Now visible: [0, 6, 7, 9] = 4 items
        assert_eq!(overlay.visible_count(), 4);
        assert!(overlay.is_expanded(6)); // node 6 is expanded
        assert!(!overlay.is_expanded(1)); // node 1 is NOT expanded
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
    fn test_state_color_variants() {
        // Just ensure no panics for all known states
        for state in &[
            "Done",
            "InProgress",
            "Draft",
            "Blocked",
            "Failed",
            "Unknown",
        ] {
            let _ = state_color(state);
        }
    }
}
