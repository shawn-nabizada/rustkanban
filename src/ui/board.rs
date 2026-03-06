use ratatui::Frame;
use ratatui::layout::{Constraint, Layout, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Paragraph};

use crate::app::{App, AppMode};
use crate::model::Column;

pub fn render(frame: &mut Frame, app: &App, area: Rect) {
    let columns = Layout::horizontal([
        Constraint::Ratio(1, 3),
        Constraint::Ratio(1, 3),
        Constraint::Ratio(1, 3),
    ])
    .split(area);

    for (i, col) in Column::all().iter().enumerate() {
        render_column(frame, app, *col, columns[i]);
    }
}

fn render_column(frame: &mut Frame, app: &App, col: Column, area: Rect) {
    let tasks = app.tasks_for_column(col);
    let count = tasks.len();
    let title = format!("{} ({})", col.display_name(), count);

    let is_focused = app.focused_column == col;
    let cursor_pos = app.cursor_positions[col.index()];

    let border_style = if is_focused {
        Style::default().fg(Color::Green).add_modifier(Modifier::BOLD)
    } else {
        Style::default().fg(Color::Gray)
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

    let today = chrono::Local::now().date_naive();

    let mut all_lines: Vec<Line> = Vec::new();

    for (i, task) in tasks.iter().enumerate() {
        let is_cursor = is_focused && i == cursor_pos;
        let is_selected =
            app.mode == AppMode::Selected && app.selected_task_id == Some(task.id);

        let priority_color = task.priority.color();

        let show_marker = is_cursor || is_selected;
        let cursor_marker = if show_marker { "> " } else { "  " };
        let cursor_style = if is_selected {
            Style::default()
                .fg(Color::Yellow)
                .add_modifier(Modifier::BOLD)
        } else if is_cursor {
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD)
        } else {
            Style::default()
        };

        let title_style = if is_selected {
            Style::default()
                .fg(Color::Black)
                .bg(Color::Yellow)
                .add_modifier(Modifier::BOLD)
        } else if is_cursor {
            Style::default()
                .fg(Color::White)
                .add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(Color::White)
        };

        let indicator = format!("[{}] ", task.priority.indicator());

        let title_chunks = wrap_text(&task.title, title_width);
        for (j, chunk) in title_chunks.iter().enumerate() {
            if j == 0 {
                all_lines.push(Line::from(vec![
                    Span::styled(String::from(cursor_marker), cursor_style),
                    Span::styled(
                        indicator.clone(),
                        Style::default()
                            .fg(priority_color)
                            .add_modifier(Modifier::BOLD),
                    ),
                    Span::styled(chunk.clone(), title_style),
                ]));
            } else {
                all_lines.push(Line::from(vec![
                    Span::raw(indent.clone()),
                    Span::styled(chunk.clone(), title_style),
                ]));
            }
        }

        // Tags line (if any)
        if !task.tags.is_empty() {
            let tag_text = format!("{}[{}]", indent, task.tags.join(", "));
            all_lines.push(Line::from(Span::styled(
                tag_text,
                Style::default().fg(Color::Cyan),
            )));
        }

        // Due date with urgency colors
        let (due_text, due_style) = match task.due_date {
            Some(d) => {
                let days_until = (d - today).num_days();
                let style = if days_until < 0 {
                    Style::default()
                        .fg(Color::Red)
                        .add_modifier(Modifier::CROSSED_OUT)
                } else if days_until == 0 {
                    Style::default()
                        .fg(Color::Red)
                        .add_modifier(Modifier::BOLD)
                } else if days_until <= 3 {
                    Style::default().fg(Color::Yellow)
                } else {
                    Style::default().fg(Color::Gray)
                };
                (format!("{}Due: {}", indent, d.format("%Y-%m-%d")), style)
            }
            None => (format!("{}Due: None", indent), Style::default().fg(Color::Gray)),
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

fn char_boundary_at(s: &str, pos: usize) -> usize {
    let mut p = pos.min(s.len());
    while p > 0 && !s.is_char_boundary(p) {
        p -= 1;
    }
    p
}
