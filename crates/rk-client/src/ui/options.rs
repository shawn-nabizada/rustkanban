use ratatui::layout::{Constraint, Layout};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Clear, Paragraph};
use ratatui::Frame;

use crate::app::App;
use crate::keybindings::KeyContext;

pub fn render(frame: &mut Frame, app: &App) {
    let area = frame.area();
    let popup = super::centered_rect(70, 80, area);
    frame.render_widget(Clear, popup);

    let block = Block::default()
        .title(" Options — Esc to close ")
        .borders(Borders::ALL)
        .border_style(Style::default().fg(app.theme.unfocused_border));

    let inner = block.inner(popup);
    frame.render_widget(block, popup);

    // Split: tab bar (1 line) + content
    let chunks = Layout::vertical([
        Constraint::Length(1), // tab bar
        Constraint::Min(1),    // content
    ])
    .split(inner);

    // Tab bar
    render_tab_bar(frame, app, chunks[0]);

    // Content based on active tab
    match app.options_tab {
        0 => render_keybindings_tab(frame, app, chunks[1]),
        1 => render_theme_tab(frame, app, chunks[1]),
        _ => {}
    }
}

fn render_tab_bar(frame: &mut Frame, app: &App, area: ratatui::layout::Rect) {
    let tabs = vec![("Keybindings", 0), ("Theme", 1)];

    let mut spans = Vec::new();
    for (name, idx) in &tabs {
        if *idx > 0 {
            spans.push(Span::raw(" | "));
        }
        if app.options_tab == *idx {
            spans.push(Span::styled(
                format!(" {} ", name),
                Style::default()
                    .fg(Color::Black)
                    .bg(app.theme.cursor)
                    .add_modifier(Modifier::BOLD),
            ));
        } else {
            spans.push(Span::styled(
                format!(" {} ", name),
                Style::default().fg(Color::Gray),
            ));
        }
    }
    spans.push(Span::styled(
        "  (Tab to switch)",
        Style::default().fg(Color::DarkGray),
    ));

    frame.render_widget(Paragraph::new(Line::from(spans)), area);
}

fn render_keybindings_tab(frame: &mut Frame, app: &App, area: ratatui::layout::Rect) {
    let width = area.width as usize;
    let mut lines: Vec<Line> = Vec::new();

    let key_col = 18; // key binding column

    // Helper: build one entry line
    let build_line = |cursor_idx: usize,
                      action: crate::keybindings::Action,
                      ctx: KeyContext,
                      app: &App|
     -> Line {
        let is_selected = app.options_cursor == cursor_idx;
        let is_rebinding = app.options_rebinding
            && app.options_rebind_action == Some(action)
            && app.options_rebind_context == Some(ctx);

        let marker = if is_selected { " > " } else { "   " };

        let key_str = if is_rebinding {
            format!("{:<width$}", "[Press a key...]", width = key_col)
        } else {
            let binding = app.keymap.key_for(ctx, action);
            let k = binding
                .map(|kb| kb.to_key_string())
                .unwrap_or_else(|| "(unbound)".to_string());
            format!("{:<width$}", k, width = key_col)
        };

        let display = action.display_name();

        let style = if is_rebinding {
            Style::default()
                .fg(Color::Yellow)
                .add_modifier(Modifier::BOLD)
        } else if is_selected {
            Style::default()
                .fg(Color::White)
                .add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(Color::Gray)
        };

        Line::from(vec![
            Span::styled(
                marker.to_string(),
                if is_selected {
                    Style::default()
                        .fg(app.theme.cursor)
                        .add_modifier(Modifier::BOLD)
                } else {
                    Style::default().fg(Color::DarkGray)
                },
            ),
            Span::styled(key_str, style),
            Span::styled(display.to_string(), style),
        ])
    };

    // --- Board header ---
    lines.push(Line::from(Span::styled(
        " Board",
        Style::default()
            .fg(Color::Cyan)
            .add_modifier(Modifier::BOLD | Modifier::UNDERLINED),
    )));

    // Board actions
    for (i, &action) in App::BOARD_ACTIONS.iter().enumerate() {
        lines.push(build_line(i, action, KeyContext::Board, app));
    }

    let board_count = App::BOARD_ACTIONS.len();

    // --- Modal header separator ---
    lines.push(Line::from(Span::styled(
        " Modal",
        Style::default()
            .fg(Color::Cyan)
            .add_modifier(Modifier::BOLD | Modifier::UNDERLINED),
    )));

    // Modal actions
    for (i, &action) in App::MODAL_ACTIONS.iter().enumerate() {
        lines.push(build_line(board_count + i, action, KeyContext::Modal, app));
    }

    // --- Help footer ---
    lines.push(Line::from(""));
    let help_text = if width > 50 {
        " Enter: rebind  r: reset  R: reset all"
    } else {
        " Enter/r/R"
    };
    lines.push(Line::from(Span::styled(
        help_text,
        Style::default().fg(Color::DarkGray),
    )));

    // Apply scrolling: determine visible window
    let visible_height = area.height as usize;
    let total_lines = lines.len();

    // Auto-scroll so the cursor line is always visible.
    // The cursor line index in `lines` is: 1 (board header) + cursor_pos for board,
    // or 1 (board header) + board_count + 1 (modal header) + modal_idx for modal.
    let cursor_line_idx = if app.options_cursor < board_count {
        1 + app.options_cursor // +1 for "Board" header
    } else {
        1 + board_count + 1 + (app.options_cursor - board_count) // +1 board header, +1 modal header
    };

    // Compute scroll offset to keep cursor visible
    let scroll = if cursor_line_idx >= visible_height {
        cursor_line_idx.saturating_sub(visible_height - 1)
    } else {
        0
    };

    let visible_lines: Vec<Line> = lines
        .into_iter()
        .skip(scroll)
        .take(visible_height)
        .collect();

    // Use a paragraph that doesn't wrap (important for alignment)
    let _ = total_lines; // silence unused warning
    frame.render_widget(Paragraph::new(visible_lines), area);
}

fn render_theme_tab(frame: &mut Frame, app: &App, area: ratatui::layout::Rect) {
    use crate::theme::color_to_string;

    // Theme properties grouped by category.
    // Each entry: (category_name, &[(property_label, flat_index)])
    struct ThemeCategory {
        name: &'static str,
        props: &'static [(&'static str, usize)],
    }

    let categories: &[ThemeCategory] = &[
        ThemeCategory {
            name: "Board",
            props: &[
                ("focused_border", 0),
                ("unfocused_border", 1),
                ("cursor", 2),
                ("selected", 3),
                ("title", 4),
            ],
        },
        ThemeCategory {
            name: "Priority",
            props: &[("high", 5), ("medium", 6), ("low", 7)],
        },
        ThemeCategory {
            name: "Tags",
            props: &[("color", 8)],
        },
        ThemeCategory {
            name: "Due Date",
            props: &[("overdue", 9), ("today", 10), ("soon", 11), ("far", 12)],
        },
        ThemeCategory {
            name: "Modal",
            props: &[("border", 13), ("focused", 14), ("error", 15)],
        },
    ];

    let width = area.width as usize;
    let mut lines: Vec<Line> = Vec::new();

    for cat in categories {
        // Category header
        lines.push(Line::from(Span::styled(
            format!(" {}", cat.name),
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD | Modifier::UNDERLINED),
        )));

        for &(label, idx) in cat.props {
            let is_selected = app.options_cursor == idx;
            let color = app.get_theme_property(idx);
            let color_name = color_to_string(color);

            let marker = if is_selected { " > " } else { "   " };
            let label_col = 22;
            let label_str = format!("{:<width$}", label, width = label_col);

            lines.push(Line::from(vec![
                Span::styled(
                    marker.to_string(),
                    if is_selected {
                        Style::default()
                            .fg(app.theme.cursor)
                            .add_modifier(Modifier::BOLD)
                    } else {
                        Style::default().fg(Color::DarkGray)
                    },
                ),
                Span::styled(
                    label_str,
                    if is_selected {
                        Style::default()
                            .fg(Color::White)
                            .add_modifier(Modifier::BOLD)
                    } else {
                        Style::default().fg(Color::Gray)
                    },
                ),
                Span::styled("\u{2588}\u{2588} ", Style::default().fg(color)),
                Span::styled(
                    color_name,
                    if is_selected {
                        Style::default()
                            .fg(Color::White)
                            .add_modifier(Modifier::BOLD)
                    } else {
                        Style::default().fg(Color::Gray)
                    },
                ),
            ]));
        }
    }

    // Help footer
    lines.push(Line::from(""));
    let help_text = if width > 50 {
        " Enter: cycle color  r: reset  R: reset all"
    } else {
        " Enter/r/R"
    };
    lines.push(Line::from(Span::styled(
        help_text,
        Style::default().fg(Color::DarkGray),
    )));

    // Scrolling: compute which line the cursor is on
    // Lines layout: for each category, 1 header + N props.
    // Find the line index of the currently selected property.
    let mut cursor_line_idx = 0;
    let mut found = false;
    let mut line_idx = 0;
    for cat in categories {
        line_idx += 1; // header
        for &(_, idx) in cat.props {
            if idx == app.options_cursor {
                cursor_line_idx = line_idx;
                found = true;
            }
            line_idx += 1;
        }
    }
    if !found {
        cursor_line_idx = 0;
    }

    let visible_height = area.height as usize;
    let scroll = if cursor_line_idx >= visible_height {
        cursor_line_idx.saturating_sub(visible_height - 1)
    } else {
        0
    };

    let visible_lines: Vec<Line> = lines
        .into_iter()
        .skip(scroll)
        .take(visible_height)
        .collect();

    frame.render_widget(Paragraph::new(visible_lines), area);
}
