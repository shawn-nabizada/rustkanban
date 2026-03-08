use ratatui::layout::{Constraint, Layout};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Clear, Paragraph};
use ratatui::Frame;

use crate::app::App;

pub fn render(frame: &mut Frame, app: &App) {
    let area = frame.area();
    let popup = super::centered_rect(50, 60, area);
    frame.render_widget(Clear, popup);

    let block = Block::default()
        .title("Tag Management")
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::Cyan));

    let inner = block.inner(popup);
    frame.render_widget(block, popup);

    let chunks = Layout::vertical([Constraint::Min(3), Constraint::Length(2)]).split(inner);

    let mut lines: Vec<Line> = Vec::new();

    if app.tags.is_empty() && !app.tag_editing {
        lines.push(Line::from(Span::styled(
            "  No tags yet",
            Style::default().fg(Color::Gray),
        )));
    }

    for (i, tag) in app.tags.iter().enumerate() {
        let is_cursor = i == app.tag_cursor;

        if is_cursor && app.tag_editing && i < app.tags.len() {
            // Editing this tag
            let text = format!("  > {}_", app.tag_edit_name);
            lines.push(Line::from(Span::styled(
                text,
                Style::default().fg(Color::Yellow),
            )));
        } else {
            let marker = if is_cursor { "> " } else { "  " };
            let style = if is_cursor {
                Style::default()
                    .fg(Color::Cyan)
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(Color::White)
            };
            lines.push(Line::from(Span::styled(
                format!("{}{}", marker, tag.name),
                style,
            )));
        }
    }

    // New tag input line
    if app.tag_editing && app.tag_cursor >= app.tags.len() {
        let text = format!("  > {}_", app.tag_edit_name);
        lines.push(Line::from(Span::styled(
            text,
            Style::default().fg(Color::Yellow),
        )));
    }

    frame.render_widget(Paragraph::new(lines), chunks[0]);

    // Help
    let help = if app.tag_editing {
        Line::from(vec![
            Span::styled("Enter", Style::default().fg(Color::Cyan)),
            Span::raw(": confirm  "),
            Span::styled("Esc", Style::default().fg(Color::Cyan)),
            Span::raw(": cancel"),
        ])
    } else {
        Line::from(vec![
            Span::styled("Space/A", Style::default().fg(Color::Cyan)),
            Span::raw(": add  "),
            Span::styled("E/Enter", Style::default().fg(Color::Cyan)),
            Span::raw(": rename  "),
            Span::styled("D", Style::default().fg(Color::Cyan)),
            Span::raw(": delete  "),
            Span::styled("Esc", Style::default().fg(Color::Cyan)),
            Span::raw(": close"),
        ])
    };
    frame.render_widget(Paragraph::new(help), chunks[1]);
}
