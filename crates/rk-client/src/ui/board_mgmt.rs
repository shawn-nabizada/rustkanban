use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Clear, Paragraph};
use ratatui::Frame;

use crate::app::App;

pub fn render(frame: &mut Frame, app: &App) {
    let area = super::centered_rect(40, 50, frame.area());
    frame.render_widget(Clear, area);

    let block = Block::default()
        .title(" Boards — Esc to close ")
        .borders(Borders::ALL)
        .border_style(Style::default().fg(app.theme.unfocused_border));

    let inner = block.inner(area);
    frame.render_widget(block, area);

    let mut lines: Vec<Line> = Vec::new();

    // Help line
    lines.push(Line::from(vec![
        Span::styled(
            "N",
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        ),
        Span::raw(": new  "),
        Span::styled(
            "R",
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        ),
        Span::raw(": rename  "),
        Span::styled(
            "D",
            Style::default().fg(Color::Red).add_modifier(Modifier::BOLD),
        ),
        Span::raw(": delete"),
    ]));
    lines.push(Line::from(""));

    for (i, board) in app.boards.iter().enumerate() {
        let is_cursor = i == app.board_cursor && !app.board_creating;
        let is_active = board.uuid == app.active_board_uuid;

        // If renaming this board
        if app.board_editing && i == app.board_cursor {
            let label = format!("  > {}\u{2588}", app.board_edit_name);
            lines.push(Line::from(Span::styled(
                label,
                Style::default()
                    .fg(Color::Yellow)
                    .add_modifier(Modifier::BOLD),
            )));
            continue;
        }

        let marker = if is_cursor { "> " } else { "  " };
        let active_indicator = if is_active { " *" } else { "" };
        let label = format!("{}{}{}", marker, board.name, active_indicator);

        let style = if is_cursor {
            Style::default()
                .fg(app.theme.cursor)
                .add_modifier(Modifier::BOLD)
        } else if is_active {
            Style::default().fg(Color::White)
        } else {
            Style::default().fg(Color::DarkGray)
        };

        lines.push(Line::from(Span::styled(label, style)));
    }

    // New board entry (when creating)
    if app.board_creating {
        let is_cursor = app.board_cursor == app.boards.len();
        let label = if is_cursor {
            format!("  > {}\u{2588}", app.board_edit_name)
        } else {
            "  [New board...]".to_string()
        };
        lines.push(Line::from(Span::styled(
            label,
            Style::default()
                .fg(Color::Yellow)
                .add_modifier(Modifier::BOLD),
        )));
    }

    let paragraph = Paragraph::new(lines);
    frame.render_widget(paragraph, inner);
}

pub fn render_delete_confirm(frame: &mut Frame, app: &App) {
    let area = super::centered_rect(40, 15, frame.area());
    frame.render_widget(Clear, area);

    let block = Block::default()
        .title(" Confirm Delete ")
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::Red));
    let inner = block.inner(area);
    frame.render_widget(block, area);

    let board_name = app
        .boards
        .get(app.board_cursor)
        .map(|b| b.name.as_str())
        .unwrap_or("?");

    let lines = vec![
        Line::from(""),
        Line::from(Span::styled(
            format!("Delete '{}' and all its tasks?", board_name),
            Style::default()
                .fg(Color::White)
                .add_modifier(Modifier::BOLD),
        )),
        Line::from(""),
        Line::from(vec![
            Span::styled(
                "Y",
                Style::default().fg(Color::Red).add_modifier(Modifier::BOLD),
            ),
            Span::raw(": yes  "),
            Span::styled("N/Esc", Style::default().fg(Color::Cyan)),
            Span::raw(": cancel"),
        ]),
    ];

    let paragraph = Paragraph::new(lines);
    frame.render_widget(paragraph, inner);
}
