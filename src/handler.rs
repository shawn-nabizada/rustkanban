use ratatui::crossterm::event::{
    KeyCode, KeyEvent, KeyModifiers, MouseButton, MouseEvent, MouseEventKind,
};

use crate::app::{App, AppMode};

pub fn handle_event(app: &mut App, key: KeyEvent) {
    match app.mode {
        AppMode::Board => handle_board(app, key),
        AppMode::Selected => handle_selected(app, key),
        AppMode::NewTask | AppMode::EditTask => handle_modal(app, key),
        AppMode::DetailView => handle_detail(app, key),
        AppMode::SortMenu => handle_sort_menu(app, key),
        AppMode::DeleteConfirm => handle_delete_confirm(app, key),
        AppMode::ClearDoneConfirm => handle_clear_done_confirm(app, key),
        AppMode::TagManagement => handle_tag_management(app, key),
        AppMode::SearchFilter => handle_search(app, key),
    }
}

fn handle_board(app: &mut App, key: KeyEvent) {
    if key.modifiers.contains(KeyModifiers::CONTROL) && key.code == KeyCode::Char('z') {
        app.undo();
        return;
    }

    if app.show_help {
        match key.code {
            KeyCode::Esc | KeyCode::Char('?') | KeyCode::Char('q') | KeyCode::Char('Q') => {
                app.show_help = false;
            }
            _ => {}
        }
        return;
    }

    match key.code {
        KeyCode::Char('q') | KeyCode::Char('Q') | KeyCode::Esc => app.quit(),
        KeyCode::Char(' ') => app.open_new_task_modal(),
        KeyCode::Char('j') | KeyCode::Left => app.move_column_left(),
        KeyCode::Char('l') | KeyCode::Right => app.move_column_right(),
        KeyCode::Up | KeyCode::BackTab => app.move_cursor_up(),
        KeyCode::Down | KeyCode::Tab => app.move_cursor_down(),
        KeyCode::Char('k') | KeyCode::Char('K') => app.select_task(),
        KeyCode::Char('p') | KeyCode::Char('P') => app.cycle_priority(),
        KeyCode::Char('s') | KeyCode::Char('S') => app.open_sort_menu(),
        KeyCode::Char('e') | KeyCode::Char('E') => app.open_edit_task_modal(),
        KeyCode::Char('c') | KeyCode::Char('C') => app.duplicate_task(),
        KeyCode::Char('d') => app.open_delete_confirm(),
        KeyCode::Char('D') => app.open_clear_done_confirm(),
        KeyCode::Enter => app.open_detail_view(),
        KeyCode::Char('t') | KeyCode::Char('T') => app.open_tag_management(),
        KeyCode::Char('/') => app.open_search(),
        KeyCode::Char('?') => app.toggle_help(),
        _ => {}
    }
}

fn handle_selected(app: &mut App, key: KeyEvent) {
    match key.code {
        KeyCode::Char('k') | KeyCode::Char('K') | KeyCode::Esc => app.deselect_task(),
        KeyCode::Char('j') | KeyCode::Left => app.move_selected_left(),
        KeyCode::Char('l') | KeyCode::Right => app.move_selected_right(),
        KeyCode::Up | KeyCode::BackTab => app.move_cursor_up(),
        KeyCode::Down | KeyCode::Tab => app.move_cursor_down(),
        _ => {}
    }
}

fn handle_modal(app: &mut App, key: KeyEvent) {
    if key.modifiers.contains(KeyModifiers::CONTROL) {
        match key.code {
            KeyCode::Char('s') | KeyCode::Enter | KeyCode::Char('\n') => {
                app.save_modal();
                return;
            }
            _ => {}
        }
    }

    match key.code {
        KeyCode::Esc => app.close_modal(),
        KeyCode::Tab => app.modal_next_field(),
        KeyCode::BackTab => app.modal_prev_field(),
        KeyCode::Enter => app.modal_insert_newline(),
        KeyCode::Backspace => app.modal_backspace(),
        KeyCode::Left => app.modal_cursor_left(),
        KeyCode::Right => app.modal_cursor_right(),
        KeyCode::Up => app.modal_cursor_up(),
        KeyCode::Down => app.modal_cursor_down(),
        KeyCode::Char(c) => app.modal_insert_char(c),
        _ => {}
    }
}

fn handle_detail(app: &mut App, key: KeyEvent) {
    match key.code {
        KeyCode::Esc | KeyCode::Enter | KeyCode::Char('q') | KeyCode::Char('Q') => {
            app.close_detail_view()
        }
        KeyCode::Char('e') | KeyCode::Char('E') => app.open_edit_task_modal(),
        _ => {}
    }
}

fn handle_sort_menu(app: &mut App, key: KeyEvent) {
    // Options: 0=DueDate, 1=Priority, 2=Filter by Tag (header), 3..=tag entries, last=Clear filter
    let tag_count = app.tags.len();
    let max_index = if tag_count > 0 { 3 + tag_count } else { 1 };

    match key.code {
        KeyCode::Esc | KeyCode::Char('q') | KeyCode::Char('Q') => app.close_sort_menu(),
        KeyCode::Up => {
            if app.sort_menu_index > 0 {
                app.sort_menu_index -= 1;
                // Skip the "Filter by Tag" header (index 2) if it exists
                if tag_count > 0 && app.sort_menu_index == 2 {
                    app.sort_menu_index = 1;
                }
            }
        }
        KeyCode::Down => {
            if app.sort_menu_index < max_index {
                app.sort_menu_index += 1;
                // Skip the "Filter by Tag" header
                if tag_count > 0 && app.sort_menu_index == 2 {
                    app.sort_menu_index = 3;
                }
            }
        }
        KeyCode::Enter => {
            match app.sort_menu_index {
                0 | 1 => app.sort_menu_select(),
                i if tag_count > 0 && i >= 3 && i < 3 + tag_count => {
                    let tag_idx = i - 3;
                    if let Some(tag) = app.tags.get(tag_idx) {
                        let tag_id = tag.id;
                        app.set_tag_filter(Some(tag_id));
                    }
                }
                i if tag_count > 0 && i == 3 + tag_count => {
                    // Clear filter
                    app.set_tag_filter(None);
                }
                _ => {}
            }
        }
        _ => {}
    }
}

fn handle_delete_confirm(app: &mut App, key: KeyEvent) {
    match key.code {
        KeyCode::Char('y') | KeyCode::Char('Y') => app.confirm_delete(),
        KeyCode::Char('n') | KeyCode::Char('N') | KeyCode::Esc => app.cancel_delete(),
        _ => {}
    }
}

fn handle_clear_done_confirm(app: &mut App, key: KeyEvent) {
    match key.code {
        KeyCode::Char('y') | KeyCode::Char('Y') => app.confirm_clear_done(),
        KeyCode::Char('n') | KeyCode::Char('N') | KeyCode::Esc => app.cancel_clear_done(),
        _ => {}
    }
}

fn handle_tag_management(app: &mut App, key: KeyEvent) {
    if app.tag_editing {
        match key.code {
            KeyCode::Enter => app.tag_confirm_edit(),
            KeyCode::Esc => app.tag_cancel_edit(),
            KeyCode::Backspace => app.tag_edit_backspace(),
            KeyCode::Char(c) => app.tag_edit_insert_char(c),
            _ => {}
        }
        return;
    }

    match key.code {
        KeyCode::Esc | KeyCode::Char('q') | KeyCode::Char('Q') => app.close_tag_management(),
        KeyCode::Up => app.tag_cursor_up(),
        KeyCode::Down => app.tag_cursor_down(),
        KeyCode::Char(' ') | KeyCode::Char('a') | KeyCode::Char('A') => {
            app.tag_cursor = app.tags.len(); // point past end = create mode
            app.tag_start_create();
        }
        KeyCode::Char('d') | KeyCode::Char('D') => app.tag_delete(),
        KeyCode::Char('e') | KeyCode::Char('E') | KeyCode::Enter => app.tag_start_rename(),
        _ => {}
    }
}

fn handle_search(app: &mut App, key: KeyEvent) {
    match key.code {
        KeyCode::Esc => app.close_search(),
        KeyCode::Enter => app.lock_search(),
        KeyCode::Backspace => app.search_backspace(),
        KeyCode::Char(c) => app.search_insert_char(c),
        _ => {}
    }
}

pub fn handle_mouse(app: &mut App, mouse: MouseEvent) {
    // Only handle mouse in board/selected modes
    match app.mode {
        AppMode::Board | AppMode::Selected => {}
        _ => return,
    }

    if app.show_help {
        if matches!(mouse.kind, MouseEventKind::Down(_)) {
            app.show_help = false;
        }
        return;
    }

    match mouse.kind {
        MouseEventKind::Down(MouseButton::Left) => {
            if let Some(col) = app.column_at_x(mouse.column) {
                app.focused_column = col;
                if let Some(idx) = app.task_at_y(col, mouse.row) {
                    app.cursor_positions[col.index()] = idx;
                    // Start drag
                    if let Some(task_id) = app.current_task_id() {
                        app.drag_task = Some((task_id, col));
                    }
                }
            }
        }
        MouseEventKind::Up(MouseButton::Left) => {
            if let Some((task_id, from_col)) = app.drag_task.take() {
                if let Some(to_col) = app.column_at_x(mouse.column) {
                    if to_col != from_col {
                        app.move_task_to_column(task_id, from_col, to_col);
                        if app.mode == AppMode::Selected {
                            app.deselect_task();
                        }
                    }
                }
            }
        }
        MouseEventKind::ScrollDown => {
            if let Some(col) = app.column_at_x(mouse.column) {
                app.scroll_column(col, 3);
            }
        }
        MouseEventKind::ScrollUp => {
            if let Some(col) = app.column_at_x(mouse.column) {
                app.scroll_column(col, -3);
            }
        }
        _ => {}
    }
}
