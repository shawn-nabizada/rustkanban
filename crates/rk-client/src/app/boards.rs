//! Board management operations.

use super::*;

impl App {
    pub fn open_board_management(&mut self) {
        self.board_cursor = self
            .boards
            .iter()
            .position(|b| b.uuid == self.active_board_uuid)
            .unwrap_or(0);
        self.board_editing = false;
        self.board_creating = false;
        self.board_edit_name.clear();
        self.mode = AppMode::BoardManagement;
    }

    pub fn close_board_management(&mut self) {
        self.board_editing = false;
        self.board_creating = false;
        self.mode = AppMode::Board;
    }

    pub fn board_mgmt_cursor_up(&mut self) {
        if self.board_cursor > 0 {
            self.board_cursor -= 1;
        }
    }

    pub fn board_mgmt_cursor_down(&mut self) {
        let max = if self.board_creating {
            self.boards.len()
        } else {
            self.boards.len().saturating_sub(1)
        };
        if self.board_cursor < max {
            self.board_cursor += 1;
        }
    }

    pub fn start_board_create(&mut self) {
        if db::board_count(&self.db).unwrap_or(0) >= 5 {
            self.set_flash("Maximum of 5 boards reached".to_string());
            return;
        }
        self.board_creating = true;
        self.board_edit_name.clear();
        self.board_cursor = self.boards.len();
    }

    pub fn start_board_rename(&mut self) {
        if let Some(board) = self.boards.get(self.board_cursor) {
            self.board_editing = true;
            self.board_edit_name = board.name.clone();
        }
    }

    pub fn board_edit_insert_char(&mut self, c: char) {
        if self.board_edit_name.chars().count() >= MAX_BOARD_NAME_LEN {
            return;
        }
        self.board_edit_name.push(c);
    }

    pub fn confirm_board_edit(&mut self) {
        let name = self.board_edit_name.trim().to_string();
        if name.is_empty() || name.len() > MAX_BOARD_NAME_LEN {
            self.set_flash("Board name must be 1-50 characters".to_string());
            return;
        }
        if self.boards.iter().any(|b| {
            b.name == name
                && (self.board_creating
                    || self.boards.get(self.board_cursor).map(|x| x.id) != Some(b.id))
        }) {
            self.set_flash("Board name already exists".to_string());
            return;
        }

        if self.board_creating {
            let _ = db::insert_board(&self.db, &name);
            self.board_creating = false;
        } else if self.board_editing {
            if let Some(board) = self.boards.get(self.board_cursor) {
                let _ = db::update_board_name(&self.db, board.id, &name);
            }
            self.board_editing = false;
        }
        self.board_edit_name.clear();
        self.reload_boards();
    }

    pub fn cancel_board_edit(&mut self) {
        self.board_creating = false;
        self.board_editing = false;
        self.board_edit_name.clear();
        if self.board_cursor >= self.boards.len() {
            self.board_cursor = self.boards.len().saturating_sub(1);
        }
    }

    pub fn open_board_delete_confirm(&mut self) {
        if self.boards.len() <= 1 {
            self.set_flash("Cannot delete the last board".to_string());
            return;
        }
        self.mode = AppMode::BoardDeleteConfirm;
    }

    pub fn confirm_board_delete(&mut self) {
        if let Some(board) = self.boards.get(self.board_cursor) {
            let deleted_uuid = board.uuid.clone();
            let _ = db::soft_delete_board_cascade(&self.db, board.id);
            self.board_states.remove(&deleted_uuid);
            self.reload_boards();
            self.reload_tasks();
            if deleted_uuid == self.active_board_uuid {
                if let Some(first) = self.boards.first() {
                    self.active_board_uuid = first.uuid.clone();
                    let _ =
                        db::set_preference(&self.db, PREF_ACTIVE_BOARD, &self.active_board_uuid);
                }
            }
            self.board_cursor = self.board_cursor.min(self.boards.len().saturating_sub(1));
        }
        self.mode = AppMode::BoardManagement;
    }

    pub fn cancel_board_delete(&mut self) {
        self.mode = AppMode::BoardManagement;
    }
}
