pub mod board;
pub mod board_mgmt;
pub mod delete_confirm;
pub mod detail;
pub mod help_bar;
pub mod modal;
pub mod search_bar;
pub mod sort_menu;
pub mod tab_bar;
pub mod tag_screen;

use ratatui::layout::Alignment;
use ratatui::layout::{Constraint, Layout, Rect};
use ratatui::style::{Color, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Paragraph};
use ratatui::Frame;

use crate::app::{App, AppMode};

const MIN_WIDTH: u16 = 80;
const MIN_HEIGHT: u16 = 30;

pub fn render(frame: &mut Frame, app: &App) {
    let area = frame.area();

    if area.width < MIN_WIDTH || area.height < MIN_HEIGHT {
        render_too_small(frame, area);
        return;
    }

    // Layout: tab bar + board + optional search bar + status bar
    let mut constraints = vec![
        Constraint::Length(1), // tab bar
        Constraint::Min(1),    // board area
    ];
    if app.search_active || app.mode == AppMode::SearchFilter {
        constraints.push(Constraint::Length(1));
    }
    constraints.push(Constraint::Length(1)); // status bar

    let chunks = Layout::vertical(constraints).split(area);

    let mut idx = 0;
    tab_bar::render(frame, app, chunks[idx]);
    idx += 1;

    board::render(frame, app, chunks[idx]);
    idx += 1;

    if app.search_active || app.mode == AppMode::SearchFilter {
        search_bar::render(frame, app, chunks[idx]);
        idx += 1;
    }

    render_status_bar(frame, app, chunks[idx]);

    // Overlays
    match app.mode {
        AppMode::NewTask | AppMode::EditTask => modal::render(frame, app),
        AppMode::DetailView => detail::render(frame, app),
        AppMode::SortMenu => sort_menu::render(frame, app),
        AppMode::DeleteConfirm | AppMode::ClearDoneConfirm => delete_confirm::render(frame, app),
        AppMode::TagManagement => tag_screen::render(frame, app),
        AppMode::BoardManagement => board_mgmt::render(frame, app),
        AppMode::BoardDeleteConfirm => board_mgmt::render_delete_confirm(frame, app),
        _ => {}
    }

    if app.show_help {
        help_bar::render(frame, app);
    }
}

fn render_status_bar(frame: &mut Frame, app: &App, area: Rect) {
    let sort_label = match app.sort_mode {
        crate::app::SortMode::DueDate => "Due Date",
        crate::app::SortMode::Priority => "Priority",
    };

    let mut spans = Vec::new();

    if let Some(ref msg) = app.flash_message {
        spans.push(Span::styled(
            msg.as_str(),
            Style::default().fg(Color::Green),
        ));
        spans.push(Span::raw("  "));
    }

    if app.mode == AppMode::Selected {
        spans.push(Span::styled(
            " SELECTED ",
            Style::default().fg(Color::Black).bg(Color::Yellow),
        ));
        spans.push(Span::raw("  J/L: move task  K/Esc: deselect  "));
    }

    spans.push(Span::styled("Sort: ", Style::default().fg(Color::Gray)));
    spans.push(Span::styled(sort_label, Style::default().fg(Color::Cyan)));

    if let Some(tag_id) = app.filter_tag {
        if let Some(tag) = app.tags.iter().find(|t| t.id == tag_id) {
            spans.push(Span::raw("  "));
            spans.push(Span::styled("Tag: ", Style::default().fg(Color::Gray)));
            spans.push(Span::styled(&tag.name, Style::default().fg(Color::Yellow)));
        }
    }

    let left = Line::from(spans);

    let mut right_spans = Vec::new();

    if let Some(ref version) = app.available_update {
        right_spans.push(Span::styled(
            format!("Update v{} available ", version),
            Style::default().fg(Color::Yellow),
        ));
        right_spans.push(Span::styled(
            "(rk update) ",
            Style::default().fg(Color::Gray),
        ));
    }

    match &app.sync_status {
        crate::app::SyncStatus::NotLoggedIn => {}
        crate::app::SyncStatus::Idle {
            last_synced: Some(ts),
        } => {
            right_spans.push(Span::styled(
                format!("Synced: {} ", ts),
                Style::default().fg(Color::Green),
            ));
        }
        crate::app::SyncStatus::Idle { last_synced: None } => {
            right_spans.push(Span::styled(
                "Not synced ",
                Style::default().fg(Color::Gray),
            ));
        }
        crate::app::SyncStatus::Syncing => {
            right_spans.push(Span::styled(
                "Syncing... ",
                Style::default().fg(Color::Yellow),
            ));
        }
        crate::app::SyncStatus::Error(_) => {
            right_spans.push(Span::styled("Offline ", Style::default().fg(Color::Red)));
        }
    }

    right_spans.push(Span::styled("?", Style::default().fg(Color::Cyan)));
    right_spans.push(Span::styled(": help", Style::default().fg(Color::Gray)));

    let right = Line::from(right_spans);

    let right_width = right.width() as u16;
    let chunks =
        Layout::horizontal([Constraint::Min(1), Constraint::Length(right_width)]).split(area);

    frame.render_widget(Paragraph::new(left), chunks[0]);
    frame.render_widget(Paragraph::new(right), chunks[1]);
}

pub fn centered_rect(percent_x: u16, percent_y: u16, area: Rect) -> Rect {
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

fn render_too_small(frame: &mut Frame, area: Rect) {
    let msg = Paragraph::new("Terminal too small (need 80x30)")
        .style(Style::default().fg(Color::Red))
        .alignment(Alignment::Center)
        .block(Block::default().borders(Borders::NONE));
    frame.render_widget(msg, area);
}
