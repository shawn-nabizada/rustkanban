//! Tag management operations.

use super::*;

impl App {
    pub fn open_tag_management(&mut self) {
        self.reload_tags();
        self.tag_cursor = 0;
        self.tag_editing = false;
        self.tag_edit_name.clear();
        self.mode = AppMode::TagManagement;
    }

    pub fn close_tag_management(&mut self) {
        self.tag_editing = false;
        self.mode = AppMode::Board;
    }

    pub fn tag_cursor_up(&mut self) {
        if self.tag_cursor > 0 {
            self.tag_cursor -= 1;
        }
    }

    pub fn tag_cursor_down(&mut self) {
        if !self.tags.is_empty() && self.tag_cursor < self.tags.len() - 1 {
            self.tag_cursor += 1;
        }
    }

    pub fn tag_start_create(&mut self) {
        self.tag_edit_name.clear();
        self.tag_edit_cursor = 0;
        self.tag_editing = true;
    }

    pub fn tag_start_rename(&mut self) {
        if let Some(tag) = self.tags.get(self.tag_cursor) {
            self.tag_edit_name = tag.name.clone();
            self.tag_edit_cursor = self.tag_edit_name.len();
            self.tag_editing = true;
        }
    }

    pub fn tag_confirm_edit(&mut self) {
        let name = self.tag_edit_name.trim().to_string();
        if name.is_empty() {
            self.tag_editing = false;
            return;
        }
        // If cursor is past end of tags, we're creating; otherwise renaming
        if self.tag_cursor >= self.tags.len() {
            let _ = db::insert_tag(&self.db, &name);
        } else if let Some(tag) = self.tags.get(self.tag_cursor) {
            let _ = db::rename_tag(&self.db, tag.id, &name);
        }
        self.tag_editing = false;
        self.reload_tags();
        if self.tag_cursor >= self.tags.len() && !self.tags.is_empty() {
            self.tag_cursor = self.tags.len() - 1;
        }
    }

    pub fn tag_cancel_edit(&mut self) {
        self.tag_editing = false;
        // If cursor was past the end (create mode), snap it back to the last tag
        if self.tag_cursor >= self.tags.len() && !self.tags.is_empty() {
            self.tag_cursor = self.tags.len() - 1;
        }
    }

    pub fn tag_delete(&mut self) {
        if self.tag_cursor < self.tags.len() {
            let tag = &self.tags[self.tag_cursor];
            let tag_id = tag.id;
            if self.filter_tag == Some(tag_id) {
                self.filter_tag = None;
            }
            let _ = db::soft_delete_tag(&self.db, tag_id);
            self.reload_tags();
            self.reload_tasks();
            if self.tag_cursor > 0 && self.tag_cursor >= self.tags.len() {
                self.tag_cursor = self.tags.len().saturating_sub(1);
            }
        }
    }

    pub fn tag_edit_insert_char(&mut self, c: char) {
        if self.tag_edit_name.chars().count() >= MAX_TAG_NAME_LEN {
            return;
        }
        self.tag_edit_name.insert(self.tag_edit_cursor, c);
        self.tag_edit_cursor += c.len_utf8();
    }

    pub fn tag_edit_backspace(&mut self) {
        if self.tag_edit_cursor > 0 {
            // Find the previous char boundary
            let prev = self.tag_edit_name[..self.tag_edit_cursor]
                .char_indices()
                .next_back()
                .map(|(i, _)| i)
                .unwrap_or(0);
            self.tag_edit_name.remove(prev);
            self.tag_edit_cursor = prev;
        }
    }

    pub fn tag_edit_cursor_left(&mut self) {
        if self.tag_edit_cursor > 0 {
            self.tag_edit_cursor = self.tag_edit_name[..self.tag_edit_cursor]
                .char_indices()
                .next_back()
                .map(|(i, _)| i)
                .unwrap_or(0);
        }
    }

    pub fn tag_edit_cursor_right(&mut self) {
        if self.tag_edit_cursor < self.tag_edit_name.len() {
            self.tag_edit_cursor = self.tag_edit_name[self.tag_edit_cursor..]
                .char_indices()
                .nth(1)
                .map(|(i, _)| self.tag_edit_cursor + i)
                .unwrap_or(self.tag_edit_name.len());
        }
    }

    // Modal tag toggling (multiple tags)
    pub fn toggle_modal_tag(&mut self) {
        if let Some(tag) = self.tags.get(self.modal_tag_cursor) {
            let tag_id = tag.id;
            if let Some(pos) = self.modal_tag_ids.iter().position(|&id| id == tag_id) {
                self.modal_tag_ids.remove(pos);
            } else {
                self.modal_tag_ids.push(tag_id);
            }
        }
    }

    pub fn set_tag_filter(&mut self, tag_id: Option<i64>) {
        self.filter_tag = tag_id;
        for col in Column::all() {
            self.clamp_cursor(col);
        }
        self.mode = AppMode::Board;
    }
}
