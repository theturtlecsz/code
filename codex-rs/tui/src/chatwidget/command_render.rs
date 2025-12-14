//! Command box rendering utilities.
//!
//! This module contains functions for rendering the pending command box
//! that appears when the user is about to execute a terminal command.
//!
//! Extracted from mod.rs as part of MAINT-11 to reduce cognitive load
//! and improve code organization.

use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::style::Style;
use ratatui::text::Line as RtLine;
use ratatui::text::Text as RtText;
use ratatui::widgets::{Block, Borders, Paragraph, Widget};
use textwrap::wrap;
use unicode_segmentation::UnicodeSegmentation;
use unicode_width::UnicodeWidthStr;

use super::terminal::PendingCommand;
use crate::colors;

/// Represents a single line of text in the command display,
/// tracking its position in the original input string.
pub(crate) struct CommandDisplayLine {
    pub text: String,
    pub start: usize,
    pub end: usize,
}

/// Wraps the pending command input into display lines that fit within the given width.
///
/// Uses grapheme-aware wrapping to handle Unicode correctly.
pub(crate) fn wrap_pending_command_lines(input: &str, width: usize) -> Vec<CommandDisplayLine> {
    if width == 0 {
        return vec![CommandDisplayLine {
            text: String::new(),
            start: 0,
            end: input.len(),
        }];
    }

    let mut lines = Vec::new();
    let mut current = String::new();
    let mut current_width = 0usize;
    let mut current_start = 0usize;

    for (byte_idx, grapheme) in input.grapheme_indices(true) {
        let g_width = UnicodeWidthStr::width(grapheme);
        if current_width + g_width > width && !current.is_empty() {
            lines.push(CommandDisplayLine {
                text: current,
                start: current_start,
                end: byte_idx,
            });
            current = String::new();
            current_width = 0;
            current_start = byte_idx;
        }
        current.push_str(grapheme);
        current_width += g_width;
    }

    let end = input.len();
    lines.push(CommandDisplayLine {
        text: current,
        start: current_start,
        end,
    });

    if lines.is_empty() {
        lines.push(CommandDisplayLine {
            text: String::new(),
            start: 0,
            end: 0,
        });
    }

    lines
}

/// Builds the lines for the pending command box, including instructions and the command itself.
///
/// Returns `None` if the area is too small to render.
/// Returns `Some((lines, height))` with the rendered lines and the required height.
pub(crate) fn pending_command_box_lines(
    pending: &PendingCommand,
    width: u16,
) -> Option<(Vec<RtLine<'static>>, u16)> {
    if width <= 4 {
        return None;
    }
    let inner_width = width.saturating_sub(2);
    if inner_width <= 4 {
        return None;
    }

    let padded_width = inner_width.saturating_sub(2).max(1) as usize;
    let command_width = inner_width.saturating_sub(4).max(1) as usize;

    const INSTRUCTION_TEXT: &str = "Press Enter to run this command. Press Esc to cancel.";
    let instruction_segments = wrap(INSTRUCTION_TEXT, padded_width);
    let instruction_style = Style::default().fg(colors::text_dim());
    let mut lines: Vec<RtLine<'static>> = instruction_segments
        .into_iter()
        .map(|segment| {
            ratatui::text::Line::from(vec![
                ratatui::text::Span::raw(" "),
                ratatui::text::Span::styled(segment.into_owned(), instruction_style),
                ratatui::text::Span::raw(" "),
            ])
        })
        .collect();

    let command_lines = wrap_pending_command_lines(pending.input(), command_width);
    let cursor_line_idx = command_line_index_for_cursor(&command_lines, pending.cursor());
    let prefix_style = Style::default().fg(colors::primary());
    let text_style = Style::default().fg(colors::text());
    let cursor_style = Style::default()
        .bg(colors::primary())
        .fg(colors::background());

    if !lines.is_empty() {
        lines.push(ratatui::text::Line::from(vec![ratatui::text::Span::raw(
            String::new(),
        )]));
    }

    for (idx, line) in command_lines.iter().enumerate() {
        let mut spans = Vec::new();
        spans.push(ratatui::text::Span::raw(" "));
        if idx == 0 {
            spans.push(ratatui::text::Span::styled("$ ", prefix_style));
        } else {
            spans.push(ratatui::text::Span::raw("  "));
        }

        if idx == cursor_line_idx {
            let cursor_offset = pending.cursor().saturating_sub(line.start);
            let cursor_offset = cursor_offset.min(line.text.len());
            let (before, cursor_span, after) = split_line_for_cursor(&line.text, cursor_offset);
            if !before.is_empty() {
                spans.push(ratatui::text::Span::styled(before, text_style));
            }
            match cursor_span {
                Some(token) => spans.push(ratatui::text::Span::styled(token, cursor_style)),
                None => spans.push(ratatui::text::Span::styled(" ", cursor_style)),
            }
            if let Some(after_text) = after
                && !after_text.is_empty()
            {
                spans.push(ratatui::text::Span::styled(after_text, text_style));
            }
        } else {
            spans.push(ratatui::text::Span::styled(line.text.clone(), text_style));
        }

        spans.push(ratatui::text::Span::raw(" "));
        lines.push(ratatui::text::Line::from(spans));
    }

    let height = (lines.len() as u16).saturating_add(2).max(3);
    Some((lines, height))
}

/// Finds which line index contains the cursor position.
pub(crate) fn command_line_index_for_cursor(lines: &[CommandDisplayLine], cursor: usize) -> usize {
    if lines.is_empty() {
        return 0;
    }
    for (idx, line) in lines.iter().enumerate() {
        if cursor < line.end {
            return idx;
        }
        if cursor == line.end {
            return (idx + 1).min(lines.len().saturating_sub(1));
        }
    }
    lines.len().saturating_sub(1)
}

/// Splits a line at the cursor position into three parts:
/// - Text before the cursor
/// - The grapheme at the cursor (if any)
/// - Text after the cursor (if any)
pub(crate) fn split_line_for_cursor(
    text: &str,
    cursor_offset: usize,
) -> (String, Option<String>, Option<String>) {
    if cursor_offset >= text.len() {
        return (text.to_string(), None, None);
    }

    let (before, remainder) = text.split_at(cursor_offset);
    let mut graphemes = remainder.graphemes(true);
    if let Some(first) = graphemes.next() {
        let after = graphemes.collect::<String>();
        (
            before.to_string(),
            Some(first.to_string()),
            if after.is_empty() { None } else { Some(after) },
        )
    } else {
        (before.to_string(), None, None)
    }
}

/// Renders a bordered text box with a title.
pub(crate) fn render_text_box(
    area: Rect,
    title: &str,
    border_color: ratatui::style::Color,
    lines: Vec<RtLine<'static>>,
    buf: &mut Buffer,
) {
    let block = Block::default()
        .borders(Borders::ALL)
        .style(Style::default().bg(colors::background()))
        .border_style(Style::default().fg(border_color))
        .title(ratatui::text::Span::styled(
            title.to_string(),
            Style::default().fg(border_color),
        ));
    block.render(area, buf);

    let inner = area.inner(ratatui::layout::Margin::new(1, 1));
    if inner.height == 0 || inner.width == 0 {
        return;
    }

    let inner_bg = Style::default().bg(colors::background());
    for y in inner.y..inner.y + inner.height {
        for x in inner.x..inner.x + inner.width {
            buf[(x, y)].set_style(inner_bg);
        }
    }

    Paragraph::new(RtText::from(lines))
        .wrap(ratatui::widgets::Wrap { trim: false })
        .render(inner, buf);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_wrap_pending_command_lines_empty() {
        let lines = wrap_pending_command_lines("", 80);
        assert_eq!(lines.len(), 1);
        assert_eq!(lines[0].text, "");
    }

    #[test]
    fn test_wrap_pending_command_lines_short() {
        let lines = wrap_pending_command_lines("ls -la", 80);
        assert_eq!(lines.len(), 1);
        assert_eq!(lines[0].text, "ls -la");
    }

    #[test]
    fn test_wrap_pending_command_lines_wraps() {
        let lines = wrap_pending_command_lines("echo hello world", 10);
        assert!(lines.len() > 1);
    }

    #[test]
    fn test_command_line_index_for_cursor_empty() {
        let lines: Vec<CommandDisplayLine> = vec![];
        assert_eq!(command_line_index_for_cursor(&lines, 0), 0);
    }

    #[test]
    fn test_command_line_index_for_cursor_single_line() {
        let lines = wrap_pending_command_lines("hello", 80);
        assert_eq!(command_line_index_for_cursor(&lines, 0), 0);
        assert_eq!(command_line_index_for_cursor(&lines, 2), 0);
        assert_eq!(command_line_index_for_cursor(&lines, 5), 0);
    }

    #[test]
    fn test_split_line_for_cursor_at_start() {
        let (before, cursor, after) = split_line_for_cursor("hello", 0);
        assert_eq!(before, "");
        assert_eq!(cursor, Some("h".to_string()));
        assert_eq!(after, Some("ello".to_string()));
    }

    #[test]
    fn test_split_line_for_cursor_at_end() {
        let (before, cursor, after) = split_line_for_cursor("hello", 5);
        assert_eq!(before, "hello");
        assert_eq!(cursor, None);
        assert_eq!(after, None);
    }

    #[test]
    fn test_split_line_for_cursor_in_middle() {
        let (before, cursor, after) = split_line_for_cursor("hello", 2);
        assert_eq!(before, "he");
        assert_eq!(cursor, Some("l".to_string()));
        assert_eq!(after, Some("lo".to_string()));
    }
}
