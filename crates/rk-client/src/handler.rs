use ratatui::crossterm::event::{KeyCode, KeyEvent, MouseButton, MouseEvent, MouseEventKind};

use crate::app::{App, AppMode};
use crate::keybindings::{Action, KeyContext};

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
        AppMode::BoardManagement => handle_board_management(app, key),
        AppMode::BoardDeleteConfirm => handle_board_delete_confirm(app, key),
        AppMode::Options => handle_options(app, key),
    }
}

fn handle_board(app: &mut App, key: KeyEvent) {
    // KeyMap dispatch
    if let Some(action) = app.keymap.action_for(KeyContext::Board, &key) {
        match action {
            Action::Quit => app.quit(),
            Action::NewTask => app.open_new_task_modal(),
            Action::MoveLeft => app.move_column_left(),
            Action::MoveRight => app.move_column_right(),
            Action::MoveUp => app.move_cursor_up(),
            Action::MoveDown => app.move_cursor_down(),
            Action::SelectTask => app.select_task(),
            Action::CyclePriority => app.cycle_priority(),
            Action::SortMenu => app.open_sort_menu(),
            Action::EditTask => app.open_edit_task_modal(),
            Action::DuplicateTask => app.duplicate_task(),
            Action::DeleteTask => app.open_delete_confirm(),
            Action::ClearDone => app.open_clear_done_confirm(),
            Action::ViewDetail => app.open_detail_view(),
            Action::TagManagement => app.open_tag_management(),
            Action::Search => app.open_search(),
            Action::Board1 => app.switch_board_by_index(0),
            Action::Board2 => app.switch_board_by_index(1),
            Action::Board3 => app.switch_board_by_index(2),
            Action::Board4 => app.switch_board_by_index(3),
            Action::Board5 => app.switch_board_by_index(4),
            Action::Boards => app.open_board_management(),
            Action::Sync => app.do_sync(),
            Action::Options => app.open_options(),
            _ => {}
        }
    }
}

fn handle_selected(app: &mut App, key: KeyEvent) {
    if let Some(action) = app.keymap.action_for(KeyContext::Board, &key) {
        match action {
            Action::SelectTask | Action::Quit => app.deselect_task(),
            Action::MoveLeft => app.move_selected_left(),
            Action::MoveRight => app.move_selected_right(),
            Action::MoveUp => app.move_cursor_up(),
            Action::MoveDown => app.move_cursor_down(),
            _ => {}
        }
    }
}

fn handle_modal(app: &mut App, key: KeyEvent) {
    // Check for configurable modal actions first (save, field navigation)
    if let Some(action) = app.keymap.action_for(KeyContext::Modal, &key) {
        match action {
            Action::Save => {
                app.save_modal();
                return;
            }
            Action::NextField => {
                app.modal_next_field();
                return;
            }
            Action::PrevField => {
                app.modal_prev_field();
                return;
            }
            _ => {}
        }
    }

    // Text editing keys (hardcoded — not configurable)
    match key.code {
        KeyCode::Esc => app.close_modal(),
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
            KeyCode::Left => app.tag_edit_cursor_left(),
            KeyCode::Right => app.tag_edit_cursor_right(),
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

fn handle_board_management(app: &mut App, key: KeyEvent) {
    if app.board_editing || app.board_creating {
        // Text input mode
        match key.code {
            KeyCode::Enter => app.confirm_board_edit(),
            KeyCode::Esc => app.cancel_board_edit(),
            KeyCode::Backspace => {
                app.board_edit_name.pop();
            }
            KeyCode::Char(c) => app.board_edit_insert_char(c),
            _ => {}
        }
        return;
    }

    match key.code {
        KeyCode::Esc | KeyCode::Char('q') => app.close_board_management(),
        KeyCode::Up => app.board_mgmt_cursor_up(),
        KeyCode::Down => app.board_mgmt_cursor_down(),
        KeyCode::Char('n') => app.start_board_create(),
        KeyCode::Char('r') => app.start_board_rename(),
        KeyCode::Char('d') => app.open_board_delete_confirm(),
        KeyCode::Enter => {
            // Switch to the selected board and close
            if let Some(board) = app.boards.get(app.board_cursor) {
                let uuid = board.uuid.clone();
                app.switch_board(&uuid);
            }
            app.close_board_management();
        }
        _ => {}
    }
}

fn handle_board_delete_confirm(app: &mut App, key: KeyEvent) {
    match key.code {
        KeyCode::Char('y') | KeyCode::Char('Y') => app.confirm_board_delete(),
        KeyCode::Char('n') | KeyCode::Char('N') | KeyCode::Esc => app.cancel_board_delete(),
        _ => {}
    }
}

fn handle_options(app: &mut App, key: KeyEvent) {
    // Rebinding mode: capture the next key
    if app.options_rebinding {
        handle_options_rebind(app, key);
        return;
    }

    match key.code {
        KeyCode::Esc | KeyCode::Char('q') | KeyCode::Char('Q') => app.close_options(),
        KeyCode::Tab => app.options_next_tab(),
        KeyCode::BackTab => app.options_prev_tab(),
        KeyCode::Up => app.options_cursor_up(),
        KeyCode::Down => app.options_cursor_down(),
        KeyCode::Enter => {
            if app.options_tab == 0 {
                app.start_rebinding();
            } else {
                app.cycle_theme_color();
            }
        }
        KeyCode::Char('r') => {
            if app.options_tab == 0 {
                app.reset_selected_binding();
            } else {
                app.reset_selected_theme_property();
            }
        }
        KeyCode::Char('R') => {
            if app.options_tab == 0 {
                app.reset_all_bindings();
            } else {
                app.reset_all_theme_properties();
            }
        }
        _ => {}
    }
}

fn handle_options_rebind(app: &mut App, key: KeyEvent) {
    use ratatui::crossterm::event::KeyModifiers;
    match key.code {
        KeyCode::Esc => app.cancel_rebinding(),
        _ => {
            if key.modifiers.contains(KeyModifiers::CONTROL) && key.code == KeyCode::Char('c') {
                app.cancel_rebinding();
                return;
            }
            app.finish_rebinding(key);
        }
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
    // Tab bar click (first row) — handle before mode check so it works from any mode
    if matches!(mouse.kind, MouseEventKind::Down(MouseButton::Left)) && mouse.row == 0 {
        let mut x_offset: u16 = 0;
        for (i, board) in app.boards.iter().enumerate() {
            let label_len = format!("{} {}", i + 1, board.name).len() as u16;
            if i > 0 {
                x_offset += 3; // " | " separator
            }
            if mouse.column >= x_offset && mouse.column < x_offset + label_len {
                app.switch_board_by_index(i);
                return;
            }
            x_offset += label_len;
        }
        return;
    }

    // Only handle mouse in board/selected modes
    match app.mode {
        AppMode::Board | AppMode::Selected => {}
        _ => return,
    }

    // Adjust y coordinate: subtract 1 for the tab bar row
    let board_y = mouse.row.saturating_sub(1);

    match mouse.kind {
        MouseEventKind::Down(MouseButton::Left) => {
            if let Some(col) = app.column_at_x(mouse.column) {
                app.focused_column = col;
                if let Some(idx) = app.task_at_y(col, board_y) {
                    app.cursor_positions[col.index()] = idx;
                    // Start drag
                    if let Some(task_id) = app.current_task_id() {
                        app.drag_task = Some((task_id, col));
                    }
                }
            }
        }
        MouseEventKind::Drag(MouseButton::Left) | MouseEventKind::Moved => {
            if app.drag_task.is_some() {
                app.drag_hover_column = app.column_at_x(mouse.column);
            }
        }
        MouseEventKind::Up(MouseButton::Left) => {
            app.drag_hover_column = None;
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
