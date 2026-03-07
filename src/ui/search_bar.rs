use ratatui::layout::Rect;
use ratatui::style::{Color, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::Paragraph;
use ratatui::Frame;

use crate::app::{App, AppMode};

pub fn render(frame: &mut Frame, app: &App, area: Rect) {
    let is_typing = app.mode == AppMode::SearchFilter;

    let mut spans = vec![
        Span::styled("/", Style::default().fg(Color::Cyan)),
        Span::raw(": "),
    ];

    if app.search_query.is_empty() && !is_typing {
        spans.push(Span::styled("search...", Style::default().fg(Color::Gray)));
    } else {
        spans.push(Span::styled(
            &app.search_query,
            Style::default().fg(Color::White),
        ));
        if is_typing {
            spans.push(Span::styled("_", Style::default().fg(Color::Yellow)));
        }
    }

    if app.search_active && !app.search_query.is_empty() && !is_typing {
        spans.push(Span::styled(
            " (Esc to clear)",
            Style::default().fg(Color::Gray),
        ));
    }

    let line = Line::from(spans);
    frame.render_widget(Paragraph::new(line), area);
}
