use diffy::Hunk;
use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::style::Color;
use ratatui::style::Modifier;
use ratatui::style::Style;
use ratatui::style::Stylize;
use ratatui::text::Line as RtLine;
use ratatui::text::Span as RtSpan;
use ratatui::widgets::Paragraph;
use std::collections::HashMap;
use std::path::Path;
use std::path::PathBuf;

use crate::exec_command::relativize_to_home;
use crate::render::Insets;
use crate::render::line_utils::prefix_lines;
use crate::render::renderable::ColumnRenderable;
use crate::render::renderable::InsetRenderable;
use crate::render::renderable::Renderable;
use codex_core::git_info::get_git_repo_root;
use codex_core::protocol::FileChange;

// Internal representation for diff line rendering
enum DiffLineType {
    Insert,
    Delete,
    Context,
}

pub struct DiffSummary {
    changes: HashMap<PathBuf, FileChange>,
    cwd: PathBuf,
}

impl DiffSummary {
    pub fn new(changes: HashMap<PathBuf, FileChange>, cwd: PathBuf) -> Self {
        Self { changes, cwd }
    }
}

impl Renderable for FileChange {
    fn render(&self, area: Rect, buf: &mut Buffer) {
        let mut lines = vec![];
        render_change(self, &mut lines, area.width as usize);
        Paragraph::new(lines).render(area, buf);
    }

    fn desired_height(&self, width: u16) -> u16 {
        let mut lines = vec![];
        render_change(self, &mut lines, width as usize);
        lines.len() as u16
    }
}

impl From<DiffSummary> for Box<dyn Renderable> {
    fn from(val: DiffSummary) -> Self {
        let mut rows: Vec<Box<dyn Renderable>> = vec![];

        for (i, row) in collect_rows(&val.changes).into_iter().enumerate() {
            if i > 0 {
                rows.push(Box::new(RtLine::from("")));
            }
            let mut path = RtLine::from(display_path_for(&row.path, &val.cwd));
            path.push_span(" ");
            path.extend(render_line_count_summary(row.added, row.removed));
            rows.push(Box::new(path));
            rows.push(Box::new(RtLine::from("")));
            rows.push(Box::new(InsetRenderable::new(
                Box::new(row.change) as Box<dyn Renderable>,
                Insets::tlbr(0, 2, 0, 0),
            )));
        }

        Box::new(ColumnRenderable::with(rows))
    }
}

pub(crate) fn create_diff_summary(
    changes: &HashMap<PathBuf, FileChange>,
    cwd: &Path,
    wrap_cols: usize,
) -> Vec<RtLine<'static>> {
    let rows = collect_rows(changes);
    render_changes_block(rows, wrap_cols, cwd)
}

// Shared row for per-file presentation
#[derive(Clone)]
struct Row {
    #[allow(dead_code)]
    path: PathBuf,
    move_path: Option<PathBuf>,
    added: usize,
    removed: usize,
    change: FileChange,
}

fn collect_rows(changes: &HashMap<PathBuf, FileChange>) -> Vec<Row> {
    let mut rows: Vec<Row> = Vec::new();
    for (path, change) in changes.iter() {
        let (added, removed) = match change {
            FileChange::Add { content } => (content.lines().count(), 0),
            // NOTE: Fork's Delete variant has no content field
            FileChange::Delete => (0, 0),
            FileChange::Update { unified_diff, .. } => calculate_add_remove_from_diff(unified_diff),
        };
        let move_path = match change {
            FileChange::Update {
                move_path: Some(new),
                ..
            } => Some(new.clone()),
            _ => None,
        };
        rows.push(Row {
            path: path.clone(),
            move_path,
            added,
            removed,
            change: change.clone(),
        });
    }
    rows.sort_by_key(|r| r.path.clone());
    rows
}

fn render_line_count_summary(added: usize, removed: usize) -> Vec<RtSpan<'static>> {
    let mut spans = Vec::new();
    spans.push("(".into());
    spans.push(format!("+{added}").green());
    spans.push(" ".into());
    spans.push(format!("-{removed}").red());
    spans.push(")".into());
    spans
}

fn render_changes_block(rows: Vec<Row>, wrap_cols: usize, cwd: &Path) -> Vec<RtLine<'static>> {
    let mut out: Vec<RtLine<'static>> = Vec::new();

    let render_path = |row: &Row| -> Vec<RtSpan<'static>> {
        let mut spans = Vec::new();
        spans.push(display_path_for(&row.path, cwd).into());
        if let Some(move_path) = &row.move_path {
            spans.push(format!(" → {}", display_path_for(move_path, cwd)).into());
        }
        spans
    };

    // Header
    let total_added: usize = rows.iter().map(|r| r.added).sum();
    let total_removed: usize = rows.iter().map(|r| r.removed).sum();
    let file_count = rows.len();
    let noun = if file_count == 1 { "file" } else { "files" };
    let mut header_spans: Vec<RtSpan<'static>> = vec!["• ".dim()];
    if let [row] = &rows[..] {
        let verb = match &row.change {
            FileChange::Add { .. } => "Added",
            FileChange::Delete => "Deleted",
            _ => "Edited",
        };
        header_spans.push(verb.bold());
        header_spans.push(" ".into());
        header_spans.extend(render_path(row));
        header_spans.push(" ".into());
        header_spans.extend(render_line_count_summary(row.added, row.removed));
    } else {
        header_spans.push("Edited".bold());
        header_spans.push(format!(" {file_count} {noun} ").into());
        header_spans.extend(render_line_count_summary(total_added, total_removed));
    }
    out.push(RtLine::from(header_spans));

    for (idx, r) in rows.into_iter().enumerate() {
        // Insert a blank separator between file chunks (except before the first)
        if idx > 0 {
            out.push("".into());
        }
        // File header line (skip when single-file header already shows the name)
        let skip_file_header = file_count == 1;
        if !skip_file_header {
            let mut header: Vec<RtSpan<'static>> = Vec::new();
            header.push("  └ ".dim());
            header.extend(render_path(&r));
            header.push(" ".into());
            header.extend(render_line_count_summary(r.added, r.removed));
            out.push(RtLine::from(header));
        }

        let mut lines = vec![];
        render_change(&r.change, &mut lines, wrap_cols - 4);
        out.extend(prefix_lines(lines, "    ".into(), "    ".into()));
    }

    out
}

fn render_change(change: &FileChange, out: &mut Vec<RtLine<'static>>, width: usize) {
    match change {
        FileChange::Add { content } => {
            let line_number_width = line_number_width(content.lines().count());
            for (i, raw) in content.lines().enumerate() {
                out.extend(push_wrapped_diff_line(
                    i + 1,
                    DiffLineType::Insert,
                    raw,
                    width,
                    line_number_width,
                ));
            }
        }
        // NOTE: Fork's Delete variant has no content field
        FileChange::Delete => {
            out.push(RtLine::from("(file deleted)".red()));
        }
        FileChange::Update { unified_diff, .. } => {
            if let Ok(patch) = diffy::Patch::from_str(unified_diff) {
                let mut max_line_number = 0;
                for h in patch.hunks() {
                    let mut old_ln = h.old_range().start();
                    let mut new_ln = h.new_range().start();
                    for l in h.lines() {
                        match l {
                            diffy::Line::Insert(_) => {
                                max_line_number = max_line_number.max(new_ln);
                                new_ln += 1;
                            }
                            diffy::Line::Delete(_) => {
                                max_line_number = max_line_number.max(old_ln);
                                old_ln += 1;
                            }
                            diffy::Line::Context(_) => {
                                max_line_number = max_line_number.max(new_ln);
                                old_ln += 1;
                                new_ln += 1;
                            }
                        }
                    }
                }
                let line_number_width = line_number_width(max_line_number);
                let mut is_first_hunk = true;
                for h in patch.hunks() {
                    if !is_first_hunk {
                        let spacer = format!("{:width$} ", "", width = line_number_width.max(1));
                        let spacer_span = RtSpan::styled(spacer, style_gutter());
                        out.push(RtLine::from(vec![spacer_span, "⋮".dim()]));
                    }
                    is_first_hunk = false;

                    let mut old_ln = h.old_range().start();
                    let mut new_ln = h.new_range().start();
                    for l in h.lines() {
                        match l {
                            diffy::Line::Insert(text) => {
                                let s = text.trim_end_matches('\n');
                                out.extend(push_wrapped_diff_line(
                                    new_ln,
                                    DiffLineType::Insert,
                                    s,
                                    width,
                                    line_number_width,
                                ));
                                new_ln += 1;
                            }
                            diffy::Line::Delete(text) => {
                                let s = text.trim_end_matches('\n');
                                out.extend(push_wrapped_diff_line(
                                    old_ln,
                                    DiffLineType::Delete,
                                    s,
                                    width,
                                    line_number_width,
                                ));
                                old_ln += 1;
                            }
                            diffy::Line::Context(text) => {
                                let s = text.trim_end_matches('\n');
                                out.extend(push_wrapped_diff_line(
                                    new_ln,
                                    DiffLineType::Context,
                                    s,
                                    width,
                                    line_number_width,
                                ));
                                old_ln += 1;
                                new_ln += 1;
                            }
                        }
                    }
                }
            }
        }
    }
}

pub(crate) fn display_path_for(path: &Path, cwd: &Path) -> String {
    // Prefer a stable, user-local relative path when the file is under the current working
    // directory. This keeps output deterministic in jj-only repos (no `.git`) and matches user
    // expectations for "files in this project".
    if let Some(rel) = pathdiff::diff_paths(path, cwd)
        && !rel
            .components()
            .any(|c| matches!(c, std::path::Component::ParentDir))
    {
        return rel.display().to_string();
    }

    let path_in_same_repo = match (get_git_repo_root(cwd), get_git_repo_root(path)) {
        (Some(cwd_repo), Some(path_repo)) => cwd_repo == path_repo,
        _ => false,
    };
    let chosen = if path_in_same_repo {
        pathdiff::diff_paths(path, cwd).unwrap_or_else(|| path.to_path_buf())
    } else {
        relativize_to_home(path)
            .map(|p| PathBuf::from_iter([Path::new("~"), p.as_path()]))
            .unwrap_or_else(|| path.to_path_buf())
    };
    chosen.display().to_string()
}

fn calculate_add_remove_from_diff(diff: &str) -> (usize, usize) {
    if let Ok(patch) = diffy::Patch::from_str(diff) {
        patch
            .hunks()
            .iter()
            .flat_map(Hunk::lines)
            .fold((0, 0), |(a, d), l| match l {
                diffy::Line::Insert(_) => (a + 1, d),
                diffy::Line::Delete(_) => (a, d + 1),
                diffy::Line::Context(_) => (a, d),
            })
    } else {
        // For unparsable diffs, return 0 for both counts.
        (0, 0)
    }
}

fn push_wrapped_diff_line(
    line_number: usize,
    kind: DiffLineType,
    text: &str,
    width: usize,
    line_number_width: usize,
) -> Vec<RtLine<'static>> {
    let ln_str = line_number.to_string();
    let mut remaining_text: &str = text;

    // Reserve a fixed number of spaces (equal to the widest line number plus a
    // trailing spacer) so the sign column stays aligned across the diff block.
    let gutter_width = line_number_width.max(1);
    let prefix_cols = gutter_width + 1;

    let mut first = true;
    let (sign_char, line_style) = match kind {
        DiffLineType::Insert => ('+', style_add()),
        DiffLineType::Delete => ('-', style_del()),
        DiffLineType::Context => (' ', style_context()),
    };
    let mut lines: Vec<RtLine<'static>> = Vec::new();

    loop {
        // Fit the content for the current terminal row:
        // compute how many columns are available after the prefix, then split
        // at a UTF-8 character boundary so this row's chunk fits exactly.
        let available_content_cols = width.saturating_sub(prefix_cols + 1).max(1);
        let split_at_byte_index = remaining_text
            .char_indices()
            .nth(available_content_cols)
            .map(|(i, _)| i)
            .unwrap_or_else(|| remaining_text.len());
        let (chunk, rest) = remaining_text.split_at(split_at_byte_index);
        remaining_text = rest;

        if first {
            // Build gutter (right-aligned line number plus spacer) as a dimmed span
            let gutter = format!("{ln_str:>gutter_width$} ");
            // Content with a sign ('+'/'-'/' ') styled per diff kind
            let content = format!("{sign_char}{chunk}");
            lines.push(RtLine::from(vec![
                RtSpan::styled(gutter, style_gutter()),
                RtSpan::styled(content, line_style),
            ]));
            first = false;
        } else {
            // Continuation lines keep a space for the sign column so content aligns
            let gutter = format!("{:gutter_width$}  ", "");
            lines.push(RtLine::from(vec![
                RtSpan::styled(gutter, style_gutter()),
                RtSpan::styled(chunk.to_string(), line_style),
            ]));
        }
        if remaining_text.is_empty() {
            break;
        }
    }
    lines
}

fn line_number_width(max_line_number: usize) -> usize {
    if max_line_number == 0 {
        1
    } else {
        max_line_number.to_string().len()
    }
}

fn style_gutter() -> Style {
    Style::default().add_modifier(Modifier::DIM)
}

fn style_context() -> Style {
    Style::default()
}

fn style_add() -> Style {
    Style::default().fg(Color::Green)
}

fn style_del() -> Style {
    Style::default().fg(Color::Red)
}
