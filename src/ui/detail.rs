use ratatui::Frame;
use ratatui::layout::{Constraint, Layout};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Clear, Paragraph, Wrap};

use crate::app::App;

pub fn render(frame: &mut Frame, app: &App) {
    let task_id = match app.detail_task_id {
        Some(id) => id,
        None => return,
    };

    let task = match app.tasks.iter().find(|t| t.id == task_id) {
        Some(t) => t,
        None => return,
    };

    let area = frame.area();
    let modal_area = super::centered_rect(70, 80, area);
    frame.render_widget(Clear, modal_area);

    let block = Block::default()
        .title("Task Detail")
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::Cyan));

    let inner = block.inner(modal_area);
    frame.render_widget(block, modal_area);

    let chunks = Layout::vertical([
        Constraint::Length(2), // Title
        Constraint::Length(1), // Priority + Column
        Constraint::Length(1), // Due date
        Constraint::Min(4),   // Description
        Constraint::Length(2), // Timestamps
        Constraint::Length(1), // Help
    ])
    .split(inner);

    // Title
    let title_line = Line::from(Span::styled(
        &task.title,
        Style::default()
            .fg(Color::White)
            .add_modifier(Modifier::BOLD),
    ));
    frame.render_widget(Paragraph::new(title_line), chunks[0]);

    // Priority + Column
    let priority_color = task.priority.color();
    let info_line = Line::from(vec![
        Span::styled("Priority: ", Style::default().fg(Color::Gray)),
        Span::styled(
            task.priority.as_str(),
            Style::default()
                .fg(priority_color)
                .add_modifier(Modifier::BOLD),
        ),
        Span::raw("  "),
        Span::styled("Column: ", Style::default().fg(Color::Gray)),
        Span::styled(
            task.column.display_name(),
            Style::default().fg(Color::Cyan),
        ),
    ]);
    frame.render_widget(Paragraph::new(info_line), chunks[1]);

    // Due date
    let due_text = match task.due_date {
        Some(d) => d.format("%Y-%m-%d").to_string(),
        None => "None".to_string(),
    };
    let due_line = Line::from(vec![
        Span::styled("Due: ", Style::default().fg(Color::Gray)),
        Span::styled(due_text, Style::default().fg(Color::White)),
    ]);
    frame.render_widget(Paragraph::new(due_line), chunks[2]);

    // Description
    let desc_block = Block::default()
        .title("Description")
        .borders(Borders::TOP)
        .border_style(Style::default().fg(Color::Gray));
    let desc_text = if task.description.is_empty() {
        "(no description)".to_string()
    } else {
        task.description.clone()
    };
    let desc = Paragraph::new(desc_text)
        .style(Style::default().fg(Color::White))
        .block(desc_block)
        .wrap(Wrap { trim: false });
    frame.render_widget(desc, chunks[3]);

    // Timestamps
    let ts_lines = vec![Line::from(vec![
        Span::styled("Created: ", Style::default().fg(Color::Gray)),
        Span::styled(
            task.created_at.format("%Y-%m-%d %H:%M").to_string(),
            Style::default().fg(Color::Gray),
        ),
        Span::raw("  "),
        Span::styled("Updated: ", Style::default().fg(Color::Gray)),
        Span::styled(
            task.updated_at.format("%Y-%m-%d %H:%M").to_string(),
            Style::default().fg(Color::Gray),
        ),
    ])];
    frame.render_widget(Paragraph::new(ts_lines), chunks[4]);

    // Help
    let help = Line::from(vec![
        Span::styled("E", Style::default().fg(Color::Cyan)),
        Span::raw(": edit  "),
        Span::styled("Esc/Enter", Style::default().fg(Color::Cyan)),
        Span::raw(": close"),
    ]);
    frame.render_widget(Paragraph::new(help), chunks[5]);
}
