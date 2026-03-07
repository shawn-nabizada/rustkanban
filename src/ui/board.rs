use ratatui::layout::{Constraint, Layout, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Paragraph};
use ratatui::Frame;

use crate::app::{App, AppMode};
use crate::model::Column;

pub fn render(frame: &mut Frame, app: &App, area: Rect) {
    let columns = Layout::horizontal([
        Constraint::Ratio(1, 3),
        Constraint::Ratio(1, 3),
        Constraint::Ratio(1, 3),
    ])
    .split(area);

    let today = chrono::Local::now().date_naive();

    for (i, col) in Column::all().iter().enumerate() {
        render_column(frame, app, *col, columns[i], today);
    }
}

fn render_column(frame: &mut Frame, app: &App, col: Column, area: Rect, today: chrono::NaiveDate) {
    let tasks = app.tasks_for_column(col);
    let count = tasks.len();
    let title = format!("{} ({})", col.display_name(), count);

    let is_focused = app.focused_column == col;
    let cursor_pos = app.cursor_positions[col.index()];

    let border_style = if is_focused {
        Style::default()
            .fg(app.theme.focused_border)
            .add_modifier(Modifier::BOLD)
    } else {
        Style::default().fg(app.theme.unfocused_border)
    };

    let block = Block::default()
        .title(title)
        .borders(Borders::ALL)
        .border_style(border_style);

    let inner = block.inner(area);
    frame.render_widget(block, area);

    if tasks.is_empty() {
        return;
    }

    let avail_width = inner.width as usize;
    let prefix_len = 6;
    let title_width = avail_width.saturating_sub(prefix_len).max(1);
    let indent: String = " ".repeat(prefix_len);

    let query_lower = if app.search_active && !app.search_query.is_empty() {
        Some(app.search_query.to_lowercase())
    } else {
        None
    };

    let mut all_lines: Vec<Line> = Vec::new();

    for (i, task) in tasks.iter().enumerate() {
        let is_cursor = is_focused && i == cursor_pos;
        let is_selected = app.mode == AppMode::Selected && app.selected_task_id == Some(task.id);

        let priority_color = app.theme.priority_color(&task.priority);

        let show_marker = is_cursor || is_selected;
        let cursor_marker = if show_marker { "> " } else { "  " };
        let cursor_style = if is_selected {
            Style::default()
                .fg(app.theme.selected)
                .add_modifier(Modifier::BOLD)
        } else if is_cursor {
            Style::default()
                .fg(app.theme.cursor)
                .add_modifier(Modifier::BOLD)
        } else {
            Style::default()
        };

        let title_style = if is_selected {
            Style::default()
                .fg(Color::Black)
                .bg(app.theme.selected)
                .add_modifier(Modifier::BOLD)
        } else if is_cursor {
            Style::default()
                .fg(app.theme.title)
                .add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(app.theme.title)
        };

        let indicator = format!("[{}] ", task.priority.indicator());

        let title_chunks = wrap_text(&task.title, title_width);
        for (j, chunk) in title_chunks.iter().enumerate() {
            let mut line_spans = if j == 0 {
                vec![
                    Span::styled(String::from(cursor_marker), cursor_style),
                    Span::styled(
                        indicator.clone(),
                        Style::default()
                            .fg(priority_color)
                            .add_modifier(Modifier::BOLD),
                    ),
                ]
            } else {
                vec![Span::raw(indent.clone())]
            };
            if let Some(ref ql) = query_lower {
                let hl_style = title_style.add_modifier(Modifier::UNDERLINED);
                line_spans.extend(highlight_matches(chunk, ql, title_style, hl_style));
            } else {
                line_spans.push(Span::styled(chunk.clone(), title_style));
            }
            all_lines.push(Line::from(line_spans));
        }

        // Tags line (if any)
        if !task.tags.is_empty() {
            let tag_text = format!("{}[{}]", indent, task.tags.join(", "));
            all_lines.push(Line::from(Span::styled(
                tag_text,
                Style::default().fg(app.theme.tag),
            )));
        }

        // Due date with urgency colors
        let (due_text, due_style) = match task.due_date {
            Some(d) => {
                let days_until = (d - today).num_days();
                let style = if days_until < 0 {
                    Style::default()
                        .fg(app.theme.due_overdue)
                        .add_modifier(Modifier::CROSSED_OUT)
                } else if days_until == 0 {
                    Style::default()
                        .fg(app.theme.due_today)
                        .add_modifier(Modifier::BOLD)
                } else if days_until <= 3 {
                    Style::default().fg(app.theme.due_soon)
                } else {
                    Style::default().fg(app.theme.due_far)
                };
                (format!("{}Due: {}", indent, d.format("%Y-%m-%d")), style)
            }
            None => (
                format!("{}Due: None", indent),
                Style::default().fg(app.theme.due_far),
            ),
        };
        all_lines.push(Line::from(Span::styled(due_text, due_style)));
    }

    // Scrolling: ensure cursor is visible
    let scroll_offset = app.scroll_offsets[col.index()];

    let paragraph = Paragraph::new(all_lines).scroll((scroll_offset as u16, 0));
    frame.render_widget(paragraph, inner);
}

fn wrap_text(text: &str, width: usize) -> Vec<String> {
    if width == 0 {
        return vec![text.to_string()];
    }
    let mut lines = Vec::new();
    let mut remaining = text;
    while !remaining.is_empty() {
        if remaining.len() <= width {
            lines.push(remaining.to_string());
            break;
        }
        // Find a char boundary at width
        let boundary = char_boundary_at(remaining, width);
        // Look backward for a space to break at a word boundary
        let split = if let Some(space_pos) = remaining[..boundary].rfind(' ') {
            lines.push(remaining[..space_pos].to_string());
            space_pos + 1 // skip the space
        } else {
            // No space found — single long word, fall back to char split
            lines.push(remaining[..boundary].to_string());
            boundary
        };
        remaining = &remaining[split..];
    }
    if lines.is_empty() {
        lines.push(String::new());
    }
    lines
}

fn highlight_matches(
    text: &str,
    query_lower: &str,
    base_style: Style,
    highlight_style: Style,
) -> Vec<Span<'static>> {
    if query_lower.is_empty() {
        return vec![Span::styled(text.to_string(), base_style)];
    }

    // Build a char-level mapping to handle Unicode case changes safely.
    // We iterate chars of both `text` and its lowercase form in lockstep,
    // tracking byte offsets into the *original* text.
    let lower = text.to_lowercase();
    let mut spans = Vec::new();
    let mut last_end = 0; // byte offset into `lower`
    let mut orig_last_end = 0; // byte offset into `text`

    // Build mapping: for each byte offset in `lower`, the corresponding byte offset in `text`.
    let mut lower_to_orig: Vec<usize> = Vec::with_capacity(lower.len() + 1);
    let mut orig_chars = text.chars();
    for lc in lower.chars() {
        let orig_c = orig_chars.next().unwrap_or(lc);
        let orig_byte_len = orig_c.len_utf8();
        let lower_byte_len = lc.len_utf8();
        let orig_offset = if lower_to_orig.is_empty() {
            0
        } else {
            *lower_to_orig.last().unwrap()
        };
        for _ in 0..lower_byte_len {
            lower_to_orig.push(orig_offset);
        }
        // After this char, advance orig offset
        let next_orig = orig_offset + orig_byte_len;
        // The last entry needs to point to the end of this char
        let len = lower_to_orig.len();
        lower_to_orig[len - 1] = next_orig;
    }
    // Sentinel for end-of-string
    let orig_end = text.len();

    for (start, matched) in lower.match_indices(query_lower) {
        let end = start + matched.len();
        let orig_start = if start < lower_to_orig.len() {
            lower_to_orig[start]
        } else {
            orig_end
        };
        let orig_match_end = if end > 0 && (end - 1) < lower_to_orig.len() {
            lower_to_orig[end - 1]
        } else {
            orig_end
        };

        // Text before the match
        let orig_gap_start = if last_end > 0 && (last_end - 1) < lower_to_orig.len() {
            lower_to_orig[last_end - 1]
        } else if last_end == 0 {
            0
        } else {
            orig_end
        };

        if orig_start > orig_gap_start {
            spans.push(Span::styled(
                text[orig_gap_start..orig_start].to_string(),
                base_style,
            ));
        }
        if orig_match_end > orig_start {
            spans.push(Span::styled(
                text[orig_start..orig_match_end].to_string(),
                highlight_style,
            ));
        }
        last_end = end;
        orig_last_end = orig_match_end;
    }

    if orig_last_end < text.len() {
        spans.push(Span::styled(text[orig_last_end..].to_string(), base_style));
    }

    if spans.is_empty() {
        spans.push(Span::styled(text.to_string(), base_style));
    }

    spans
}

fn char_boundary_at(s: &str, pos: usize) -> usize {
    let mut p = pos.min(s.len());
    while p > 0 && !s.is_char_boundary(p) {
        p -= 1;
    }
    p
}
