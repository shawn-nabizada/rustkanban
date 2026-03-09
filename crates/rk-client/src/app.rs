use std::time::Instant;

use rusqlite::Connection;

use crate::db;
use crate::model::{Board, Column, Priority, Tag, Task};
use crate::theme::Theme;

#[derive(Debug, Clone)]
pub enum SyncStatus {
    NotLoggedIn,
    Idle {
        last_synced: Option<String>,
    },
    Syncing,
    #[allow(dead_code)]
    Error(String),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AppMode {
    Board,
    Selected,
    NewTask,
    EditTask,
    DetailView,
    SortMenu,
    DeleteConfirm,
    ClearDoneConfirm,
    TagManagement,
    SearchFilter,
    BoardManagement,
    BoardDeleteConfirm,
}

const PREF_SORT_MODE: &str = "sort_mode";
const PREF_FOCUSED_COLUMN: &str = "focused_column";
const PREF_ACTIVE_BOARD: &str = "active_board";

const MAX_TITLE_LEN: usize = 500;
const MAX_DESCRIPTION_LEN: usize = 5000;
const MAX_TAG_NAME_LEN: usize = 50;
const MAX_BOARD_NAME_LEN: usize = 50;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SortMode {
    DueDate,
    Priority,
}

impl SortMode {
    pub fn as_str(&self) -> &'static str {
        match self {
            SortMode::DueDate => "DueDate",
            SortMode::Priority => "Priority",
        }
    }

    pub fn from_str(s: &str) -> Self {
        match s {
            "Priority" => SortMode::Priority,
            _ => SortMode::DueDate,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ModalField {
    Title,
    Description,
    Priority,
    Tag,
    DueDateYear,
    DueDateMonth,
    DueDateDay,
}

impl ModalField {
    pub fn all() -> &'static [ModalField] {
        &[
            ModalField::Title,
            ModalField::Description,
            ModalField::Priority,
            ModalField::Tag,
            ModalField::DueDateYear,
            ModalField::DueDateMonth,
            ModalField::DueDateDay,
        ]
    }

    pub fn index(&self) -> usize {
        match self {
            ModalField::Title => 0,
            ModalField::Description => 1,
            ModalField::Priority => 2,
            ModalField::Tag => 3,
            ModalField::DueDateYear => 4,
            ModalField::DueDateMonth => 5,
            ModalField::DueDateDay => 6,
        }
    }

    pub fn from_index(i: usize) -> ModalField {
        match i {
            0 => ModalField::Title,
            1 => ModalField::Description,
            2 => ModalField::Priority,
            3 => ModalField::Tag,
            4 => ModalField::DueDateYear,
            5 => ModalField::DueDateMonth,
            6 => ModalField::DueDateDay,
            _ => ModalField::Title,
        }
    }
}

#[derive(Debug, Clone)]
pub struct ModalState {
    pub title: String,
    pub description: String,
    pub priority: Priority,
    pub due_year: String,
    pub due_month: String,
    pub due_day: String,
    pub focused_field: ModalField,
    pub error: Option<String>,
    pub editing_task_id: Option<i64>,
    pub cursor_pos: usize, // byte offset within the active text field
    pub wrap_width: usize, // inner width of the text field (set by renderer)
}

impl ModalState {
    pub fn new() -> Self {
        ModalState {
            title: String::new(),
            description: String::new(),
            priority: Priority::Medium,
            due_year: String::new(),
            due_month: String::new(),
            due_day: String::new(),
            focused_field: ModalField::Title,
            error: None,
            editing_task_id: None,
            cursor_pos: 0,
            wrap_width: 80,
        }
    }
}

pub struct App {
    pub mode: AppMode,
    pub running: bool,
    pub focused_column: Column,
    pub cursor_positions: [usize; 3],
    pub scroll_offsets: [usize; 3],
    pub tasks: Vec<Task>,
    pub tags: Vec<Tag>,
    pub sort_mode: SortMode,
    pub modal: ModalState,
    pub selected_task_id: Option<i64>,
    pub detail_task_id: Option<i64>,

    pub flash_message: Option<String>,
    pub flash_expire: Option<Instant>,
    pub sort_menu_index: usize,
    pub show_help: bool,
    // Search
    pub search_query: String,
    pub search_active: bool,
    // Tag filter
    pub filter_tag: Option<i64>,
    // Tag management
    pub tag_cursor: usize,
    pub tag_edit_name: String,
    pub tag_editing: bool,
    // Modal tag selection
    pub modal_tag_ids: Vec<i64>,
    pub modal_tag_cursor: usize,
    pub theme: Theme,
    pub db: Connection,
    // Mouse support
    pub terminal_width: u16,
    pub terminal_height: u16,
    pub drag_task: Option<(i64, Column)>, // (task_id, from_column)
    // Boards
    pub boards: Vec<Board>,
    pub active_board_uuid: String,
    pub board_states: std::collections::HashMap<String, ([usize; 3], [usize; 3])>, // uuid → (cursor_positions, scroll_offsets)
    // Board management
    pub board_cursor: usize,
    pub board_edit_name: String,
    pub board_editing: bool,  // true when renaming inline
    pub board_creating: bool, // true when creating new board
    pub sync_status: SyncStatus,
    pub available_update: Option<String>,
}

impl App {
    pub fn new(db: Connection, theme: Theme) -> Self {
        let tasks = db::load_tasks(&db).unwrap_or_default();
        let tags = db::load_tags(&db).unwrap_or_default();
        let creds = crate::auth::load_credentials();
        // Clean up old soft deletes for non-syncing users (30 days)
        if creds.is_none() {
            let _ = db::cleanup_old_soft_deletes(&db, 30);
        }

        let boards = db::load_boards(&db).unwrap_or_default();
        let active_board_uuid = db::get_preference(&db, PREF_ACTIVE_BOARD)
            .filter(|uuid| boards.iter().any(|b| b.uuid == *uuid))
            .unwrap_or_else(|| boards.first().map(|b| b.uuid.clone()).unwrap_or_default());

        App {
            mode: AppMode::Board,
            running: true,
            focused_column: match db::get_preference(&db, PREF_FOCUSED_COLUMN).as_deref() {
                Some("in_progress") => Column::InProgress,
                Some("done") => Column::Done,
                _ => Column::Todo,
            },
            cursor_positions: [0; 3],
            scroll_offsets: [0; 3],
            tasks,
            tags,
            sort_mode: db::get_preference(&db, PREF_SORT_MODE)
                .map(|s| SortMode::from_str(&s))
                .unwrap_or(SortMode::DueDate),
            modal: ModalState::new(),
            selected_task_id: None,
            detail_task_id: None,

            flash_message: None,
            flash_expire: None,
            sort_menu_index: 0,
            show_help: false,
            search_query: String::new(),
            search_active: false,
            filter_tag: None,
            tag_cursor: 0,
            tag_edit_name: String::new(),
            tag_editing: false,
            modal_tag_ids: Vec::new(),
            modal_tag_cursor: 0,
            theme,
            db,
            terminal_width: 0,
            terminal_height: 0,
            drag_task: None,
            boards,
            active_board_uuid,
            board_states: std::collections::HashMap::new(),
            board_cursor: 0,
            board_edit_name: String::new(),
            board_editing: false,
            board_creating: false,
            available_update: None,
            sync_status: if let Some(c) = creds {
                SyncStatus::Idle {
                    last_synced: c.last_synced_at,
                }
            } else {
                SyncStatus::NotLoggedIn
            },
        }
    }

    pub fn tick(&mut self) {
        if let Some(expire) = self.flash_expire {
            if Instant::now() >= expire {
                self.flash_message = None;
                self.flash_expire = None;
            }
        }
    }

    pub fn set_flash(&mut self, msg: String) {
        self.flash_message = Some(msg);
        self.flash_expire = Some(Instant::now() + std::time::Duration::from_secs(2));
    }

    pub fn reload_tasks(&mut self) {
        self.tasks = db::load_tasks(&self.db).unwrap_or_default();
    }

    pub fn reload_tags(&mut self) {
        self.tags = db::load_tags(&self.db).unwrap_or_default();
    }

    pub fn reload_boards(&mut self) {
        self.boards = db::load_boards(&self.db).unwrap_or_default();
        if !self.boards.iter().any(|b| b.uuid == self.active_board_uuid) {
            if let Some(first) = self.boards.first() {
                self.active_board_uuid = first.uuid.clone();
                let _ = db::set_preference(&self.db, PREF_ACTIVE_BOARD, &self.active_board_uuid);
            }
        }
    }

    pub fn switch_board(&mut self, board_uuid: &str) {
        if board_uuid == self.active_board_uuid {
            return;
        }
        if !self.boards.iter().any(|b| b.uuid == board_uuid) {
            return;
        }
        // Save current board's cursor/scroll state
        self.board_states.insert(
            self.active_board_uuid.clone(),
            (self.cursor_positions, self.scroll_offsets),
        );
        // Switch
        self.active_board_uuid = board_uuid.to_string();
        let _ = db::set_preference(&self.db, PREF_ACTIVE_BOARD, board_uuid);
        // Restore or reset
        if let Some((cursors, scrolls)) = self.board_states.get(board_uuid) {
            self.cursor_positions = *cursors;
            self.scroll_offsets = *scrolls;
        } else {
            self.cursor_positions = [0; 3];
            self.scroll_offsets = [0; 3];
        }
        // Clear search when switching
        self.search_query.clear();
        self.search_active = false;
        self.filter_tag = None;
    }

    pub fn switch_board_by_index(&mut self, index: usize) {
        if let Some(board) = self.boards.get(index) {
            let uuid = board.uuid.clone();
            self.switch_board(&uuid);
        }
    }

    #[allow(dead_code)]
    pub fn active_board_name(&self) -> &str {
        self.boards
            .iter()
            .find(|b| b.uuid == self.active_board_uuid)
            .map(|b| b.name.as_str())
            .unwrap_or("Board")
    }

    pub fn do_sync(&mut self) {
        if !crate::auth::is_logged_in() {
            return;
        }
        self.sync_status = SyncStatus::Syncing;
        match crate::sync::sync(&self.db) {
            Ok(synced_at) => {
                self.reload_tasks();
                self.reload_tags();
                self.reload_boards();

                self.sync_status = SyncStatus::Idle {
                    last_synced: Some(synced_at),
                };
                self.set_flash("Synced successfully".to_string());
            }
            Err(e) => {
                self.sync_status = SyncStatus::Error(e.to_string());
                self.set_flash(e.to_string());
            }
        }
    }

    pub fn tasks_for_column(&self, col: Column) -> Vec<&Task> {
        let mut tasks: Vec<&Task> = self
            .tasks
            .iter()
            .filter(|t| t.column == col && t.board_id == self.active_board_uuid)
            .collect();

        // Apply search filter
        if self.search_active && !self.search_query.is_empty() {
            let q = self.search_query.to_lowercase();
            tasks.retain(|t| {
                t.title.to_lowercase().contains(&q) || t.description.to_lowercase().contains(&q)
            });
        }

        // Apply tag filter
        if let Some(tag_id) = self.filter_tag {
            if let Some(tag) = self.tags.iter().find(|t| t.id == tag_id) {
                let tag_name = &tag.name;
                tasks.retain(|t| t.tags.contains(tag_name));
            }
        }

        match self.sort_mode {
            SortMode::DueDate => {
                tasks.sort_by(|a, b| {
                    let a_date = a.due_date.unwrap_or(chrono::NaiveDate::MAX);
                    let b_date = b.due_date.unwrap_or(chrono::NaiveDate::MAX);
                    a_date.cmp(&b_date)
                });
            }
            SortMode::Priority => {
                tasks.sort_by(|a, b| {
                    let priority_order = |p: &Priority| match p {
                        Priority::High => 0,
                        Priority::Medium => 1,
                        Priority::Low => 2,
                    };
                    priority_order(&a.priority).cmp(&priority_order(&b.priority))
                });
            }
        }

        tasks
    }

    pub fn current_task_id(&self) -> Option<i64> {
        let tasks = self.tasks_for_column(self.focused_column);
        let cursor = self.cursor_positions[self.focused_column.index()];
        tasks.get(cursor).map(|t| t.id)
    }

    fn find_task(&self, id: i64) -> Option<&Task> {
        self.tasks.iter().find(|t| t.id == id)
    }

    pub fn clamp_cursor(&mut self, col: Column) {
        let count = self.tasks_for_column(col).len();
        let idx = col.index();
        if count == 0 {
            self.cursor_positions[idx] = 0;
        } else if self.cursor_positions[idx] >= count {
            self.cursor_positions[idx] = count - 1;
        }
    }

    pub fn set_cursor_to_task(&mut self, task_id: i64, col: Column) {
        let tasks = self.tasks_for_column(col);
        if let Some(pos) = tasks.iter().position(|t| t.id == task_id) {
            self.cursor_positions[col.index()] = pos;
        }
    }

    pub fn quit(&mut self) {
        let _ = db::set_preference(&self.db, PREF_FOCUSED_COLUMN, self.focused_column.as_str());
        self.running = false;
    }

    // Navigation

    pub fn move_column_left(&mut self) {
        let idx = self.focused_column.index();
        if let Some(col) = idx.checked_sub(1).and_then(Column::from_index) {
            self.focused_column = col;
        }
    }

    pub fn move_column_right(&mut self) {
        let idx = self.focused_column.index();
        if let Some(col) = Column::from_index(idx + 1) {
            self.focused_column = col;
        }
    }

    pub fn move_cursor_up(&mut self) {
        let idx = self.focused_column.index();
        let count = self.tasks_for_column(self.focused_column).len();
        if count == 0 {
            return;
        }
        if self.cursor_positions[idx] > 0 {
            self.cursor_positions[idx] -= 1;
        } else {
            self.cursor_positions[idx] = count - 1;
        }
    }

    pub fn move_cursor_down(&mut self) {
        let idx = self.focused_column.index();
        let count = self.tasks_for_column(self.focused_column).len();
        if count == 0 {
            return;
        }
        if self.cursor_positions[idx] < count - 1 {
            self.cursor_positions[idx] += 1;
        } else {
            self.cursor_positions[idx] = 0;
        }
    }

    // Selection (Phase 4)

    pub fn select_task(&mut self) {
        if let Some(id) = self.current_task_id() {
            self.selected_task_id = Some(id);
            self.mode = AppMode::Selected;
        }
    }

    pub fn deselect_task(&mut self) {
        self.selected_task_id = None;
        self.mode = AppMode::Board;
    }

    pub fn move_task_to_column(&mut self, task_id: i64, from_col: Column, to_col: Column) {
        let _ = db::update_task_column(&self.db, task_id, to_col);
        self.reload_tasks();
        self.clamp_cursor(from_col);
        self.focused_column = to_col;
        self.set_cursor_to_task(task_id, to_col);
    }

    pub fn move_selected_left(&mut self) {
        if let Some(task_id) = self.selected_task_id {
            if let Some(task) = self.find_task(task_id) {
                let from_col = task.column;
                if let Some(to_col) = from_col.index().checked_sub(1).and_then(Column::from_index) {
                    self.move_task_to_column(task_id, from_col, to_col);
                }
            }
        }
    }

    pub fn move_selected_right(&mut self) {
        if let Some(task_id) = self.selected_task_id {
            if let Some(task) = self.find_task(task_id) {
                let from_col = task.column;
                if let Some(to_col) = Column::from_index(from_col.index() + 1) {
                    self.move_task_to_column(task_id, from_col, to_col);
                }
            }
        }
    }

    // Priority cycling (Phase 5)

    pub fn cycle_priority(&mut self) {
        if let Some(task_id) = self.current_task_id() {
            if let Some(task) = self.find_task(task_id) {
                let old_priority = task.priority;
                let new_priority = match old_priority {
                    Priority::Low => Priority::Medium,
                    Priority::Medium => Priority::High,
                    Priority::High => Priority::Low,
                };
                let _ = db::update_task_priority(&self.db, task_id, new_priority);
                self.reload_tasks();
                self.set_cursor_to_task(task_id, self.focused_column);
            }
        }
    }

    // Sort menu (Phase 5)

    pub fn open_sort_menu(&mut self) {
        self.sort_menu_index = match self.sort_mode {
            SortMode::DueDate => 0,
            SortMode::Priority => 1,
        };
        self.mode = AppMode::SortMenu;
    }

    pub fn close_sort_menu(&mut self) {
        self.mode = AppMode::Board;
    }

    pub fn sort_menu_select(&mut self) {
        self.sort_mode = match self.sort_menu_index {
            0 => SortMode::DueDate,
            1 => SortMode::Priority,
            _ => SortMode::DueDate,
        };
        let _ = db::set_preference(&self.db, PREF_SORT_MODE, self.sort_mode.as_str());
        self.mode = AppMode::Board;
    }

    // Detail view (Phase 6)

    pub fn open_detail_view(&mut self) {
        if let Some(id) = self.current_task_id() {
            self.detail_task_id = Some(id);
            self.mode = AppMode::DetailView;
        }
    }

    pub fn close_detail_view(&mut self) {
        self.detail_task_id = None;
        self.mode = AppMode::Board;
    }

    // Edit modal (Phase 6)

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

    // Delete (Phase 7)

    pub fn open_delete_confirm(&mut self) {
        if self.current_task_id().is_some() {
            self.mode = AppMode::DeleteConfirm;
        }
    }

    pub fn cancel_delete(&mut self) {
        self.mode = AppMode::Board;
    }

    pub fn confirm_delete(&mut self) {
        if let Some(task_id) = self.current_task_id() {
            if let Some(task) = self.find_task(task_id).cloned() {
                let _ = db::soft_delete_task(&self.db, task_id);
                self.reload_tasks();
                self.clamp_cursor(task.column);
                self.set_flash(format!("Deleted '{}'", task.title));
            }
        }
        self.mode = AppMode::Board;
    }

    // Duplicate task

    pub fn duplicate_task(&mut self) {
        if let Some(task_id) = self.current_task_id() {
            if let Some(task) = self.find_task(task_id).cloned() {
                let tag_ids = db::get_task_tag_ids(&self.db, task_id).unwrap_or_default();
                if let Ok(new_id) = db::insert_task(
                    &self.db,
                    &task.title,
                    &task.description,
                    task.priority,
                    task.column,
                    task.due_date,
                    &self.active_board_uuid,
                ) {
                    if !tag_ids.is_empty() {
                        let _ = db::set_task_tags(&self.db, new_id, &tag_ids);
                    }

                    self.reload_tasks();
                    self.set_cursor_to_task(new_id, task.column);
                    self.set_flash(format!("Duplicated '{}'", task.title));
                }
            }
        }
    }

    // Clear Done column

    pub fn open_clear_done_confirm(&mut self) {
        let done_count = self.tasks_for_column(Column::Done).len();
        if done_count > 0 {
            self.mode = AppMode::ClearDoneConfirm;
        }
    }

    pub fn cancel_clear_done(&mut self) {
        self.mode = AppMode::Board;
    }

    pub fn confirm_clear_done(&mut self) {
        let done_tasks: Vec<_> = self
            .tasks
            .iter()
            .filter(|t| t.column == Column::Done && t.board_id == self.active_board_uuid)
            .cloned()
            .collect();
        let count = done_tasks.len();
        for task in &done_tasks {
            let _ = db::soft_delete_task(&self.db, task.id);
        }
        self.reload_tasks();
        self.clamp_cursor(Column::Done);
        self.set_flash(format!(
            "Cleared {} done task{}",
            count,
            if count == 1 { "" } else { "s" }
        ));
        self.mode = AppMode::Board;
    }

    // Modal operations

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

    // Help toggle (Phase 11)

    // Scroll management — call before render
    pub fn update_scroll(&mut self, col_width: usize, col_height: usize) {
        let prefix_len = 6;
        let title_width = col_width.saturating_sub(prefix_len).max(1);

        for col in Column::all() {
            let tasks = self.tasks_for_column(col);
            let cursor = self.cursor_positions[col.index()];

            // Calculate visual line for each task
            let mut cursor_start_line: usize = 0;
            let mut cursor_end_line: usize = 0;

            for (i, task) in tasks.iter().enumerate() {
                let height = task_visual_height(task, title_width);
                if i == cursor {
                    cursor_start_line = cursor_end_line;
                    cursor_end_line = cursor_start_line + height;
                    break;
                }
                cursor_end_line += height;
            }

            let offset = &mut self.scroll_offsets[col.index()];
            if cursor_start_line < *offset {
                *offset = cursor_start_line;
            } else if cursor_end_line > *offset + col_height {
                *offset = cursor_end_line.saturating_sub(col_height);
            }
        }
    }

    pub fn toggle_help(&mut self) {
        self.show_help = !self.show_help;
    }

    // Search (Phase 9)

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

    // Tag management (Phase 8)

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
        self.tag_editing = true;
    }

    pub fn tag_start_rename(&mut self) {
        if let Some(tag) = self.tags.get(self.tag_cursor) {
            self.tag_edit_name = tag.name.clone();
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
        self.tag_edit_name.push(c);
    }

    pub fn tag_edit_backspace(&mut self) {
        self.tag_edit_name.pop();
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

    // Mouse helpers

    pub fn column_at_x(&self, x: u16) -> Option<Column> {
        if self.terminal_width == 0 {
            return None;
        }
        let col_width = self.terminal_width / 3;
        let idx = (x / col_width).min(2) as usize;
        Column::from_index(idx)
    }

    pub fn task_at_y(&self, col: Column, y: u16) -> Option<usize> {
        // Board inner area starts at y=1 (after top border)
        if y == 0 {
            return None;
        }
        let inner_y = (y - 1) as usize;

        let col_width = (self.terminal_width / 3).saturating_sub(2) as usize;
        let prefix_len = 6;
        let title_width = col_width.saturating_sub(prefix_len).max(1);

        let tasks = self.tasks_for_column(col);
        let scroll = self.scroll_offsets[col.index()];
        let target_line = inner_y + scroll;

        let mut line = 0;
        for (i, task) in tasks.iter().enumerate() {
            let height = task_visual_height(task, title_width);
            if target_line >= line && target_line < line + height {
                return Some(i);
            }
            line += height;
        }
        None
    }

    pub fn scroll_column(&mut self, col: Column, delta: i32) {
        let idx = col.index();
        if delta < 0 {
            self.scroll_offsets[idx] = self.scroll_offsets[idx].saturating_sub((-delta) as usize);
        } else {
            // Clamp to content height to avoid scrolling past the end
            let col_width = (self.terminal_width / 3).saturating_sub(2) as usize;
            let prefix_len = 6;
            let title_width = col_width.saturating_sub(prefix_len).max(1);
            let total_height: usize = self
                .tasks_for_column(col)
                .iter()
                .map(|t| task_visual_height(t, title_width))
                .sum();
            let new_offset = self.scroll_offsets[idx].saturating_add(delta as usize);
            self.scroll_offsets[idx] = new_offset.min(total_height.saturating_sub(1));
        }
    }

    // Tag filter (Phase 10)

    pub fn set_tag_filter(&mut self, tag_id: Option<i64>) {
        self.filter_tag = tag_id;
        for col in Column::all() {
            self.clamp_cursor(col);
        }
        self.mode = AppMode::Board;
    }

    // Board management

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

fn prev_char_boundary(s: &str, pos: usize) -> usize {
    let mut p = pos.saturating_sub(1);
    while p > 0 && !s.is_char_boundary(p) {
        p -= 1;
    }
    p
}

fn next_char_boundary(s: &str, pos: usize) -> usize {
    let mut p = pos + 1;
    while p < s.len() && !s.is_char_boundary(p) {
        p += 1;
    }
    p
}

/// Build a flat list of visual rows as (line_start_byte, line_len) accounting for wrapping.
fn visual_rows(text: &str, wrap_width: usize) -> Vec<(usize, usize)> {
    let mut rows = Vec::new();
    let mut offset: usize = 0;
    for line in text.split('\n') {
        if line.is_empty() {
            rows.push((offset, 0));
        } else {
            let mut remaining = line.len();
            let mut pos = 0;
            while remaining > 0 {
                let chunk = remaining.min(wrap_width);
                rows.push((offset + pos, chunk));
                pos += chunk;
                remaining -= chunk;
            }
        }
        offset += line.len() + 1; // +1 for the \n
    }
    rows
}

fn byte_to_row_col_with(rows: &[(usize, usize)], byte_pos: usize) -> (usize, usize) {
    for (i, &(start, len)) in rows.iter().enumerate() {
        let end = start + len;
        // Cursor can be at end of row (after last char) only if it's the last row
        // or if the next row starts a new logical line
        if byte_pos >= start && byte_pos <= end {
            // If exactly at end and there's a next row that continues this wrap, go to next row
            if byte_pos == end && i + 1 < rows.len() {
                let next_start = rows[i + 1].0;
                if next_start == end {
                    // next row is a continuation of the same logical line
                    return (i + 1, 0);
                }
            }
            return (i, byte_pos - start);
        }
    }
    // Past end: last row
    let last = rows.len().saturating_sub(1);
    let col = if let Some(&(start, _)) = rows.last() {
        byte_pos.saturating_sub(start)
    } else {
        0
    };
    (last, col)
}

fn task_visual_height(task: &crate::model::Task, title_width: usize) -> usize {
    let title_lines = wrapped_line_count(&task.title, title_width);
    let tag_lines = if task.tags.is_empty() { 0 } else { 1 };
    title_lines + tag_lines + 1 // +1 for due date line
}

fn wrapped_line_count(text: &str, width: usize) -> usize {
    if width == 0 || text.is_empty() {
        return 1;
    }
    let mut count = 0;
    let mut remaining = text;
    while !remaining.is_empty() {
        count += 1;
        if remaining.len() <= width {
            break;
        }
        let boundary = width.min(remaining.len());
        let split = if let Some(space_pos) = remaining[..boundary].rfind(' ') {
            space_pos + 1
        } else {
            boundary
        };
        remaining = &remaining[split..];
    }
    count
}

fn row_col_to_byte_with(rows: &[(usize, usize)], row: usize, col: usize, text_len: usize) -> usize {
    if let Some(&(start, len)) = rows.get(row) {
        let clamped_col = col.min(len);
        (start + clamped_col).min(text_len)
    } else {
        text_len
    }
}
