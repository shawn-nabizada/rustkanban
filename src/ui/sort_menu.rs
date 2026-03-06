use ratatui::Frame;
use ratatui::layout::{Constraint, Layout, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Clear, Paragraph};

use crate::app::App;

pub fn render(frame: &mut Frame, app: &App) {
    let area = frame.area();
    let has_tags = !app.tags.is_empty();
    let popup_height = if has_tags { 30 + app.tags.len() as u16 * 3 } else { 20 };
    let popup = centered_rect(35, popup_height.min(60), area);
    frame.render_widget(Clear, popup);

    let block = Block::default()
        .title("Sort / Filter")
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::Cyan));

    let inner = block.inner(popup);
    frame.render_widget(block, popup);

    let mut lines = Vec::new();

    // Sort options
    let sort_options = ["Due Date", "Priority"];
    for (i, opt) in sort_options.iter().enumerate() {
        let is_selected = i == app.sort_menu_index;
        let marker = if is_selected { "> " } else { "  " };
        let style = if is_selected {
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(Color::White)
        };
        lines.push(Line::from(Span::styled(format!("{}{}", marker, opt), style)));
    }

    // Tag filter section
    if has_tags {
        lines.push(Line::from(""));
        lines.push(Line::from(Span::styled(
            "  Filter by Tag:",
            Style::default().fg(Color::Gray),
        )));

        for (i, tag) in app.tags.iter().enumerate() {
            let menu_idx = 3 + i;
            let is_selected = menu_idx == app.sort_menu_index;
            let is_active = app.filter_tag == Some(tag.id);
            let marker = if is_selected { "> " } else { "  " };
            let active_marker = if is_active { " *" } else { "" };
            let style = if is_selected {
                Style::default()
                    .fg(Color::Cyan)
                    .add_modifier(Modifier::BOLD)
            } else if is_active {
                Style::default().fg(Color::Yellow)
            } else {
                Style::default().fg(Color::White)
            };
            lines.push(Line::from(Span::styled(
                format!("{}  {}{}", marker, tag.name, active_marker),
                style,
            )));
        }

        // Clear filter option
        let clear_idx = 3 + app.tags.len();
        let is_selected = clear_idx == app.sort_menu_index;
        let marker = if is_selected { "> " } else { "  " };
        let style = if is_selected {
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(Color::Gray)
        };
        lines.push(Line::from(Span::styled(
            format!("{}  Clear filter", marker),
            style,
        )));
    }

    lines.push(Line::from(""));
    lines.push(Line::from(vec![
        Span::styled("Enter", Style::default().fg(Color::Cyan)),
        Span::raw(": select  "),
        Span::styled("Esc", Style::default().fg(Color::Cyan)),
        Span::raw(": cancel"),
    ]));

    frame.render_widget(Paragraph::new(lines), inner);
}

fn centered_rect(percent_x: u16, percent_y: u16, area: Rect) -> Rect {
    let vertical = Layout::vertical([
        Constraint::Percentage((100 - percent_y) / 2),
        Constraint::Percentage(percent_y),
        Constraint::Percentage((100 - percent_y) / 2),
    ])
    .split(area);

    Layout::horizontal([
        Constraint::Percentage((100 - percent_x) / 2),
        Constraint::Percentage(percent_x),
        Constraint::Percentage((100 - percent_x) / 2),
    ])
    .split(vertical[1])[1]
}
