//! Search and filter operations.

use super::*;

impl App {
    pub fn open_search(&mut self) {
        self.search_query.clear();
        self.search_active = true;
        self.mode = AppMode::SearchFilter;
    }

    pub fn close_search(&mut self) {
        self.search_active = false;
        self.search_query.clear();
        self.mode = AppMode::Board;
        for col in Column::all() {
            self.clamp_cursor(col);
        }
    }

    pub fn lock_search(&mut self) {
        // Keep filter active, go back to board
        self.mode = AppMode::Board;
        for col in Column::all() {
            self.clamp_cursor(col);
        }
    }

    pub fn search_insert_char(&mut self, c: char) {
        self.search_query.push(c);
        for col in Column::all() {
            self.clamp_cursor(col);
        }
    }

    pub fn search_backspace(&mut self) {
        self.search_query.pop();
        for col in Column::all() {
            self.clamp_cursor(col);
        }
    }
}
