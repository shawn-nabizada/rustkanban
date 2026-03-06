use ratatui::Frame;
use ratatui::layout::{Constraint, Layout, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Clear, Paragraph};

use crate::app::App;

pub fn render(frame: &mut Frame, _app: &App) {
    let area = frame.area();
    let popup = centered_rect(50, 60, area);
    frame.render_widget(Clear, popup);

    let block = Block::default()
        .title("Help — ? to close")
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::Cyan));

    let inner = block.inner(popup);
    frame.render_widget(block, popup);

    let sections: Vec<(&str, Vec<(&str, &str)>)> = vec![
        ("Navigation", vec![
            ("J / Left", "Focus left column"),
            ("L / Right", "Focus right column"),
            ("Up / Down", "Move cursor in column"),
        ]),
        ("Tasks", vec![
            ("Space", "New task (in Todo)"),
            ("Enter", "View task details"),
            ("E", "Edit task"),
            ("d", "Delete task"),
            ("Shift+D", "Clear done column"),
            ("P", "Cycle priority"),
            ("Ctrl+Z", "Undo last action"),
        ]),
        ("Selection", vec![
            ("K", "Select / deselect task"),
            ("J / L", "Move selected task between columns"),
        ]),
        ("Other", vec![
            ("S", "Sort / filter menu"),
            ("T", "Tag management"),
            ("/", "Search tasks"),
            ("?", "Toggle this help"),
            ("Esc / Q", "Quit"),
        ]),
    ];

    let mut lines: Vec<Line> = Vec::new();
    for (i, (title, bindings)) in sections.iter().enumerate() {
        if i > 0 {
            lines.push(Line::from(""));
        }
        lines.push(Line::from(Span::styled(
            *title,
            Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD),
        )));
        for (key, desc) in bindings {
            lines.push(Line::from(vec![
                Span::styled(format!("  {:14}", key), Style::default().fg(Color::Cyan)),
                Span::styled(*desc, Style::default().fg(Color::White)),
            ]));
        }
    }

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
