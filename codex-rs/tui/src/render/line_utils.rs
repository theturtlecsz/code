use ratatui::text::Line;
use ratatui::text::Span;

/// Clone a borrowed ratatui `Line` into an owned `'static` line.
#[allow(dead_code)]
pub fn line_to_static(line: &Line<'_>) -> Line<'static> {
    Line {
        style: line.style,
        alignment: line.alignment,
        spans: line
            .spans
            .iter()
            .map(|s| Span {
                style: s.style,
                content: std::borrow::Cow::Owned(s.content.to_string()),
            })
            .collect(),
    }
}

/// Append owned copies of borrowed lines to `out`.
#[allow(dead_code)]
pub fn push_owned_lines<'a>(src: &[Line<'a>], out: &mut Vec<Line<'static>>) {
    for l in src {
        out.push(line_to_static(l));
    }
}

/// Consider a line blank if it has no spans or only spans whose contents are
/// empty or consist solely of spaces (no tabs/newlines).
pub fn is_blank_line_spaces_only(line: &Line<'_>) -> bool {
    if line.spans.is_empty() {
        return true;
    }
    line.spans
        .iter()
        .all(|s| s.content.is_empty() || s.content.chars().all(|c| c == ' '))
}

/// Consider a line blank if its spans are empty or all span contents are
/// whitespace when trimmed.
#[allow(dead_code)]
pub fn is_blank_line_trim(line: &Line<'_>) -> bool {
    if line.spans.is_empty() {
        return true;
    }
    line.spans.iter().all(|s| s.content.trim().is_empty())
}

/// Prefix each line with `initial_prefix` for the first line and
/// `subsequent_prefix` for following lines. Returns a new Vec of owned lines.
#[allow(dead_code)] // Used by footer module, will be wired in SYNC-009
pub fn prefix_lines(
    lines: Vec<Line<'static>>,
    initial_prefix: Span<'static>,
    subsequent_prefix: Span<'static>,
) -> Vec<Line<'static>> {
    lines
        .into_iter()
        .enumerate()
        .map(|(i, l)| {
            let mut spans = Vec::with_capacity(l.spans.len() + 1);
            spans.push(if i == 0 {
                initial_prefix.clone()
            } else {
                subsequent_prefix.clone()
            });
            spans.extend(l.spans);
            Line::from(spans).style(l.style)
        })
        .collect()
}

/// Whether this line is painted with the code-block background color.
/// Used to distinguish a truly blank paragraph separator (no background)
/// from a blank line that is part of a code block (should not be dropped
/// during streaming commit logic).
pub fn is_code_block_painted(line: &Line<'_>) -> bool {
    let code_bg = crate::colors::code_block_bg();
    if line.style.bg == Some(code_bg) {
        return true;
    }
    if line.spans.iter().any(|s| s.style.bg == Some(code_bg)) {
        return true;
    }
    // Treat our hidden language sentinel as code so it groups with the block.
    let flat: String = line.spans.iter().map(|s| s.content.as_ref()).collect();
    if flat.contains("⟦LANG:") {
        return true;
    }
    false
}
