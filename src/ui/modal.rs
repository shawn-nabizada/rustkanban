use ratatui::Frame;
use ratatui::layout::{Constraint, Layout, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Clear, Paragraph, Wrap};

use crate::app::{App, AppMode, ModalField};

pub fn render(frame: &mut Frame, app: &App) {
    let area = frame.area();
    let modal_area = super::centered_rect(60, 70, area);

    frame.render_widget(Clear, modal_area);

    let title = match app.mode {
        AppMode::NewTask => "New Task",
        AppMode::EditTask => "Edit Task",
        _ => return,
    };

    let block = Block::default()
        .title(title)
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::Cyan));

    let inner = block.inner(modal_area);
    frame.render_widget(block, modal_area);

    let chunks = Layout::vertical([
        Constraint::Length(4), // Title (2 lines + borders)
        Constraint::Length(6), // Description
        Constraint::Length(3), // Priority
        Constraint::Length(3), // Tag
        Constraint::Length(3), // Due Date
        Constraint::Length(2), // Error / help
    ])
    .split(inner);

    render_text_field(frame, app, chunks[0], ModalField::Title, "Title *");
    render_text_field(frame, app, chunks[1], ModalField::Description, "Description");
    render_priority_field(frame, app, chunks[2]);
    render_tag_field(frame, app, chunks[3]);
    render_due_date_field(frame, app, chunks[4]);
    render_footer(frame, app, chunks[5]);
}

fn render_text_field(
    frame: &mut Frame,
    app: &App,
    area: Rect,
    field: ModalField,
    label: &str,
) {
    let focused = app.modal.focused_field == field;
    let text = match field {
        ModalField::Title => &app.modal.title,
        ModalField::Description => &app.modal.description,
        _ => return,
    };

    let has_error = field == ModalField::Title
        && app.modal.error.is_some()
        && text.trim().is_empty();

    let border_color = if has_error {
        Color::Red
    } else if focused {
        Color::Yellow
    } else {
        Color::Gray
    };

    let block = Block::default()
        .title(label)
        .borders(Borders::ALL)
        .border_style(Style::default().fg(border_color));

    let display_text = if focused {
        let pos = app.modal.cursor_pos.min(text.len());
        let (before, after) = text.split_at(pos);
        format!("{}\u{2502}{}", before, after) // │ as cursor
    } else {
        text.clone()
    };

    let inner_width = area.width.saturating_sub(2) as usize;
    let visible_height = area.height.saturating_sub(2);
    let cursor_offset = if focused {
        app.modal.cursor_pos.min(text.len())
    } else {
        0
    };
    let scroll = cursor_scroll(text, cursor_offset, inner_width, visible_height);

    let paragraph = Paragraph::new(display_text)
        .block(block)
        .wrap(Wrap { trim: false })
        .scroll((scroll, 0));
    frame.render_widget(paragraph, area);
}

fn render_priority_field(frame: &mut Frame, app: &App, area: Rect) {
    let focused = app.modal.focused_field == ModalField::Priority;
    let border_color = if focused { Color::Yellow } else { Color::Gray };

    let priority_color = app.modal.priority.color();

    let block = Block::default()
        .title("Priority (Space to cycle)")
        .borders(Borders::ALL)
        .border_style(Style::default().fg(border_color));

    let paragraph = Paragraph::new(Span::styled(
        app.modal.priority.as_str(),
        Style::default().fg(priority_color).add_modifier(Modifier::BOLD),
    ))
    .block(block);

    frame.render_widget(paragraph, area);
}

fn render_tag_field(frame: &mut Frame, app: &App, area: Rect) {
    let focused = app.modal.focused_field == ModalField::Tag;
    let border_color = if focused { Color::Yellow } else { Color::Gray };

    let block = Block::default()
        .title("Tag (Space to cycle)")
        .borders(Borders::ALL)
        .border_style(Style::default().fg(border_color));

    let (label, style) = if let Some(&tag_id) = app.modal_tag_ids.first() {
        if let Some(tag) = app.tags.iter().find(|t| t.id == tag_id) {
            (
                tag.name.clone(),
                Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD),
            )
        } else {
            ("None".to_string(), Style::default().fg(Color::Gray))
        }
    } else {
        ("None".to_string(), Style::default().fg(Color::Gray))
    };

    let paragraph = Paragraph::new(Span::styled(label, style)).block(block);
    frame.render_widget(paragraph, area);
}

fn render_due_date_field(frame: &mut Frame, app: &App, area: Rect) {
    let fields = [
        (ModalField::DueDateYear, "Year", &app.modal.due_year),
        (ModalField::DueDateMonth, "Month", &app.modal.due_month),
        (ModalField::DueDateDay, "Day", &app.modal.due_day),
    ];

    let chunks = Layout::horizontal([
        Constraint::Ratio(1, 3),
        Constraint::Ratio(1, 3),
        Constraint::Ratio(1, 3),
    ])
    .split(area);

    for (i, (field, label, value)) in fields.iter().enumerate() {
        let focused = app.modal.focused_field == *field;
        let border_color = if focused { Color::Yellow } else { Color::Gray };

        let display = if value.is_empty() && !focused {
            label.to_string()
        } else if focused {
            format!("{}_", value)
        } else {
            value.to_string()
        };

        let style = if value.is_empty() && !focused {
            Style::default().fg(Color::Gray)
        } else {
            Style::default().fg(Color::White)
        };

        let block = Block::default()
            .title(format!("Due {}", label))
            .borders(Borders::ALL)
            .border_style(Style::default().fg(border_color));

        let p = Paragraph::new(display).style(style).block(block);
        frame.render_widget(p, chunks[i]);
    }
}

fn render_footer(frame: &mut Frame, app: &App, area: Rect) {
    let mut lines = Vec::new();

    if let Some(ref err) = app.modal.error {
        lines.push(Line::from(Span::styled(
            err.as_str(),
            Style::default().fg(Color::Red),
        )));
    }

    lines.push(Line::from(vec![
        Span::styled("Tab", Style::default().fg(Color::Cyan)),
        Span::raw(": next field  "),
        Span::styled("Ctrl+S", Style::default().fg(Color::Cyan)),
        Span::raw(": save  "),
        Span::styled("Esc", Style::default().fg(Color::Cyan)),
        Span::raw(": cancel"),
    ]));

    let paragraph = Paragraph::new(lines);
    frame.render_widget(paragraph, area);
}

/// Calculate vertical scroll offset to keep the cursor line visible.
/// `cursor_byte` is the byte offset of the cursor in the original text.
fn cursor_scroll(text: &str, cursor_byte: usize, width: usize, visible_height: u16) -> u16 {
    if width == 0 || visible_height == 0 {
        return 0;
    }

    // Count wrapped lines up to the cursor position
    let mut cursor_line: u16 = 0;
    let mut consumed: usize = 0;

    for line in text.split('\n') {
        let line_end = consumed + line.len();
        if cursor_byte <= line_end {
            // Cursor is on this logical line
            let offset_in_line = cursor_byte.saturating_sub(consumed);
            cursor_line += (offset_in_line / width) as u16;
            break;
        }
        // Full wrapped lines for this logical line
        if line.is_empty() {
            cursor_line += 1;
        } else {
            cursor_line += ((line.len() + width - 1) / width) as u16;
        }
        consumed = line_end + 1; // +1 for the \n
    }

    // Scroll so cursor line is within the visible area
    cursor_line.saturating_sub(visible_height.saturating_sub(1))
}