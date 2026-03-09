use ratatui::layout::Rect;
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::Paragraph;
use ratatui::Frame;

use crate::app::App;

pub fn render(frame: &mut Frame, app: &App, area: Rect) {
    let mut spans: Vec<Span> = Vec::new();

    for (i, board) in app.boards.iter().enumerate() {
        if i > 0 {
            spans.push(Span::styled(
                " | ",
                Style::default().fg(app.theme.unfocused_border),
            ));
        }

        let label = format!("{} {}", i + 1, board.name);
        let style = if board.uuid == app.active_board_uuid {
            Style::default()
                .fg(app.theme.cursor)
                .add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(Color::DarkGray)
        };
        spans.push(Span::styled(label, style));
    }

    let line = Line::from(spans);
    let paragraph = Paragraph::new(line);
    frame.render_widget(paragraph, area);
}
