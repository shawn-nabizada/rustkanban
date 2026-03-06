use ratatui::Frame;
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Clear, Paragraph};

use crate::app::{App, AppMode};
use crate::model::Column;

pub fn render(frame: &mut Frame, app: &App) {
    let message = match app.mode {
        AppMode::ClearDoneConfirm => {
            let count = app.tasks_for_column(Column::Done).len();
            format!(
                "Delete all {} task{} in Done?",
                count,
                if count == 1 { "" } else { "s" }
            )
        }
        _ => {
            let task_title = app
                .current_task_id()
                .and_then(|id| app.tasks.iter().find(|t| t.id == id))
                .map(|t| t.title.as_str())
                .unwrap_or("task");
            format!("Delete '{}'?", task_title)
        }
    };

    let area = frame.area();
    let popup = super::centered_rect(40, 15, area);
    frame.render_widget(Clear, popup);

    let block = Block::default()
        .title("Confirm Delete")
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::Red));

    let inner = block.inner(popup);
    frame.render_widget(block, popup);

    let lines = vec![
        Line::from(""),
        Line::from(Span::styled(
            message,
            Style::default()
                .fg(Color::White)
                .add_modifier(Modifier::BOLD),
        )),
        Line::from(""),
        Line::from(vec![
            Span::styled("Y", Style::default().fg(Color::Red).add_modifier(Modifier::BOLD)),
            Span::raw(": yes  "),
            Span::styled("N/Esc", Style::default().fg(Color::Cyan)),
            Span::raw(": cancel"),
        ]),
    ];

    frame.render_widget(Paragraph::new(lines), inner);
}
