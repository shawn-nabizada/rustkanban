//! Task create/edit modal operations.

use super::text_utils::*;
use super::*;

impl App {
    pub fn open_edit_task_modal(&mut self) {
        let task_id = match self.mode {
            AppMode::DetailView => self.detail_task_id,
            _ => self.current_task_id(),
        };

        if let Some(id) = task_id {
            if let Some(task) = self.find_task(id).cloned() {
                let cursor_pos = task.title.len();
                let wrap_width = self.modal.wrap_width; // preserve from current render
                self.modal = ModalState {
                    title: task.title.clone(),
                    description: task.description.clone(),
                    priority: task.priority,
                    due_year: task
                        .due_date
                        .map(|d| d.format("%Y").to_string())
                        .unwrap_or_default(),
                    due_month: task
                        .due_date
                        .map(|d| d.format("%-m").to_string())
                        .unwrap_or_default(),
                    due_day: task
                        .due_date
                        .map(|d| d.format("%-d").to_string())
                        .unwrap_or_default(),
                    focused_field: ModalField::Title,
                    error: None,
                    editing_task_id: Some(id),
                    cursor_pos,
                    wrap_width,
                };
                self.modal_tag_ids = db::get_task_tag_ids(&self.db, id).unwrap_or_default();
                self.modal_tag_cursor = 0;
                self.reload_tags();
                self.mode = AppMode::EditTask;
            }
        }
    }

    pub fn open_new_task_modal(&mut self) {
        self.modal = ModalState::new();
        self.modal_tag_ids.clear();
        self.modal_tag_cursor = 0;
        self.reload_tags();
        self.mode = AppMode::NewTask;
    }

    pub fn close_modal(&mut self) {
        self.mode = AppMode::Board;
        self.modal.error = None;
    }

    pub fn save_modal(&mut self) {
        let title = self.modal.title.trim().to_string();
        if title.is_empty() {
            self.modal.error = Some("Title is required".to_string());
            return;
        }

        let due_date = self.parse_modal_due_date();

        match self.mode {
            AppMode::NewTask => {
                if let Ok(new_id) = db::insert_task(
                    &self.db,
                    &title,
                    &self.modal.description,
                    self.modal.priority,
                    Column::Todo,
                    due_date,
                    &self.active_board_uuid,
                ) {
                    let _ = db::set_task_tags(&self.db, new_id, &self.modal_tag_ids);
                }
                self.reload_tasks();
                self.mode = AppMode::Board;
            }
            AppMode::EditTask => {
                if let Some(task_id) = self.modal.editing_task_id {
                    let _ = db::update_task(
                        &self.db,
                        task_id,
                        &title,
                        &self.modal.description,
                        self.modal.priority,
                        due_date,
                    );
                    let _ = db::set_task_tags(&self.db, task_id, &self.modal_tag_ids);
                    self.reload_tasks();
                }
                self.mode = AppMode::Board;
            }
            _ => {}
        }

        self.modal.error = None;
    }

    fn parse_modal_due_date(&self) -> Option<chrono::NaiveDate> {
        let year: i32 = self.modal.due_year.parse().ok()?;
        let month: u32 = self.modal.due_month.parse().ok()?;
        let day: u32 = self.modal.due_day.parse().ok()?;
        chrono::NaiveDate::from_ymd_opt(year, month, day)
    }

    fn sync_cursor_to_end(&mut self) {
        self.modal.cursor_pos = match self.modal.focused_field {
            ModalField::Title => self.modal.title.len(),
            ModalField::Description => self.modal.description.len(),
            _ => 0,
        };
    }

    pub fn modal_next_field(&mut self) {
        let fields = ModalField::all();
        let current = self.modal.focused_field.index();
        let next = (current + 1) % fields.len();
        self.modal.focused_field = ModalField::from_index(next);
        self.sync_cursor_to_end();
    }

    pub fn modal_prev_field(&mut self) {
        let fields = ModalField::all();
        let current = self.modal.focused_field.index();
        let prev = if current == 0 {
            fields.len() - 1
        } else {
            current - 1
        };
        self.modal.focused_field = ModalField::from_index(prev);
        self.sync_cursor_to_end();
    }

    pub fn modal_insert_char(&mut self, c: char) {
        match self.modal.focused_field {
            ModalField::Title => {
                if self.modal.title.chars().count() >= MAX_TITLE_LEN {
                    return;
                }
                let pos = self.modal.cursor_pos.min(self.modal.title.len());
                self.modal.title.insert(pos, c);
                self.modal.cursor_pos = pos + c.len_utf8();
            }
            ModalField::Description => {
                if self.modal.description.chars().count() >= MAX_DESCRIPTION_LEN {
                    return;
                }
                let pos = self.modal.cursor_pos.min(self.modal.description.len());
                self.modal.description.insert(pos, c);
                self.modal.cursor_pos = pos + c.len_utf8();
            }
            ModalField::Priority => {
                if c == ' ' {
                    self.modal.priority = match self.modal.priority {
                        Priority::Low => Priority::Medium,
                        Priority::Medium => Priority::High,
                        Priority::High => Priority::Low,
                    };
                }
            }
            ModalField::Tag => {
                if c == ' ' {
                    self.toggle_modal_tag();
                }
            }
            ModalField::DueDateYear => {
                if c.is_ascii_digit() && self.modal.due_year.len() < 4 {
                    self.modal.due_year.push(c);
                }
            }
            ModalField::DueDateMonth => {
                if c.is_ascii_digit() && self.modal.due_month.len() < 2 {
                    self.modal.due_month.push(c);
                }
            }
            ModalField::DueDateDay => {
                if c.is_ascii_digit() && self.modal.due_day.len() < 2 {
                    self.modal.due_day.push(c);
                }
            }
        }
        self.modal.error = None;
    }

    pub fn modal_insert_newline(&mut self) {
        if self.modal.focused_field == ModalField::Description
            && self.modal.description.chars().count() < MAX_DESCRIPTION_LEN
        {
            let pos = self.modal.cursor_pos.min(self.modal.description.len());
            self.modal.description.insert(pos, '\n');
            self.modal.cursor_pos = pos + 1;
        }
    }

    pub fn modal_backspace(&mut self) {
        match self.modal.focused_field {
            ModalField::Title => {
                if self.modal.cursor_pos > 0 {
                    let prev = prev_char_boundary(&self.modal.title, self.modal.cursor_pos);
                    self.modal.title.remove(prev);
                    self.modal.cursor_pos = prev;
                }
            }
            ModalField::Description => {
                if self.modal.cursor_pos > 0 {
                    let prev = prev_char_boundary(&self.modal.description, self.modal.cursor_pos);
                    self.modal.description.remove(prev);
                    self.modal.cursor_pos = prev;
                }
            }
            ModalField::Priority | ModalField::Tag => {}
            ModalField::DueDateYear => {
                self.modal.due_year.pop();
            }
            ModalField::DueDateMonth => {
                self.modal.due_month.pop();
            }
            ModalField::DueDateDay => {
                self.modal.due_day.pop();
            }
        }
        self.modal.error = None;
    }

    pub fn modal_cursor_left(&mut self) {
        match self.modal.focused_field {
            ModalField::Title => {
                if self.modal.cursor_pos > 0 {
                    self.modal.cursor_pos =
                        prev_char_boundary(&self.modal.title, self.modal.cursor_pos);
                }
            }
            ModalField::Description => {
                if self.modal.cursor_pos > 0 {
                    self.modal.cursor_pos =
                        prev_char_boundary(&self.modal.description, self.modal.cursor_pos);
                }
            }
            _ => {}
        }
    }

    pub fn modal_cursor_right(&mut self) {
        match self.modal.focused_field {
            ModalField::Title => {
                if self.modal.cursor_pos < self.modal.title.len() {
                    self.modal.cursor_pos =
                        next_char_boundary(&self.modal.title, self.modal.cursor_pos);
                }
            }
            ModalField::Description => {
                if self.modal.cursor_pos < self.modal.description.len() {
                    self.modal.cursor_pos =
                        next_char_boundary(&self.modal.description, self.modal.cursor_pos);
                }
            }
            _ => {}
        }
    }

    pub fn modal_cursor_up(&mut self) {
        if self.modal.focused_field == ModalField::Tag {
            if self.modal_tag_cursor > 0 {
                self.modal_tag_cursor -= 1;
            }
            return;
        }
        let text = match self.modal.focused_field {
            ModalField::Title => &self.modal.title,
            ModalField::Description => &self.modal.description,
            _ => return,
        };
        let w = self.modal.wrap_width.max(1);
        let pos = self.modal.cursor_pos.min(text.len());
        let rows = visual_rows(text, w);
        let (row, col) = byte_to_row_col_with(&rows, pos);
        if row == 0 {
            return;
        }
        self.modal.cursor_pos = row_col_to_byte_with(&rows, row - 1, col, text.len());
    }

    pub fn modal_cursor_down(&mut self) {
        if self.modal.focused_field == ModalField::Tag {
            if !self.tags.is_empty() && self.modal_tag_cursor < self.tags.len() - 1 {
                self.modal_tag_cursor += 1;
            }
            return;
        }
        let text = match self.modal.focused_field {
            ModalField::Title => &self.modal.title,
            ModalField::Description => &self.modal.description,
            _ => return,
        };
        let w = self.modal.wrap_width.max(1);
        let pos = self.modal.cursor_pos.min(text.len());
        let rows = visual_rows(text, w);
        let (row, col) = byte_to_row_col_with(&rows, pos);
        if row + 1 >= rows.len() {
            return;
        }
        self.modal.cursor_pos = row_col_to_byte_with(&rows, row + 1, col, text.len());
    }
}
