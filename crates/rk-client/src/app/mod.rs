mod boards;
mod modal;
mod options;
mod search;
mod tags;
pub mod text_utils;

use std::time::Instant;

use rusqlite::Connection;

use crate::db;
use crate::model::{Board, Column, Priority, Tag, Task};
use crate::theme::Theme;

use text_utils::task_visual_height;

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
    Options,
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
    // Search
    pub search_query: String,
    pub search_active: bool,
    // Tag filter
    pub filter_tag: Option<i64>,
    // Tag management
    pub tag_cursor: usize,
    pub tag_edit_name: String,
    pub tag_edit_cursor: usize,
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
    pub drag_hover_column: Option<Column>,
    // Boards
    pub boards: Vec<Board>,
    pub active_board_uuid: String,
    pub board_states: std::collections::HashMap<String, ([usize; 3], [usize; 3])>, // uuid -> (cursor_positions, scroll_offsets)
    // Board management
    pub board_cursor: usize,
    pub board_edit_name: String,
    pub board_editing: bool,  // true when renaming inline
    pub board_creating: bool, // true when creating new board
    pub sync_status: SyncStatus,
    pub available_update: Option<String>,
    pub keymap: crate::keybindings::KeyMap,
    // Options modal state
    pub options_tab: usize,
    pub options_scroll: usize,
    pub options_cursor: usize,
    pub options_rebinding: bool,
    pub options_rebind_action: Option<crate::keybindings::Action>,
    pub options_rebind_context: Option<crate::keybindings::KeyContext>,
}

impl App {
    pub fn new(db: Connection, theme: Theme) -> Self {
        let tasks = db::load_tasks(&db).unwrap_or_default();
        let tags = db::load_tags(&db).unwrap_or_default();
        let creds = crate::auth::load_credentials();
        let keymap = crate::keybindings::load_keymap();
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
            search_query: String::new(),
            search_active: false,
            filter_tag: None,
            tag_cursor: 0,
            tag_edit_name: String::new(),
            tag_edit_cursor: 0,
            tag_editing: false,
            modal_tag_ids: Vec::new(),
            modal_tag_cursor: 0,
            theme,
            db,
            terminal_width: 0,
            terminal_height: 0,
            drag_task: None,
            drag_hover_column: None,
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
            keymap,
            options_tab: 0,
            options_scroll: 0,
            options_cursor: 0,
            options_rebinding: false,
            options_rebind_action: None,
            options_rebind_context: None,
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

    // Selection

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

    // Priority cycling

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

    // Sort menu

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

    // Detail view

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

    // Delete

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

    // Scroll management -- call before render
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
}
