# RustKanban Improvements Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Add BSL license, tests, CI improvements, mouse support, duplicate task, persistent preferences, search highlighting, and distribution packaging.

**Architecture:** Non-breaking additions. Tests use in-memory SQLite. Mouse support extends the event layer to return `AppEvent` enum. Preferences stored in a new SQLite table. Distribution files (Homebrew, AUR, man page) are standalone.

**Tech Stack:** Rust, ratatui, crossterm (mouse events), rusqlite, clap, clap_mangen, vhs

---

### Task 1: BSL 1.1 License

**Files:**
- Replace: `LICENSE`
- Modify: `Cargo.toml:5`
- Modify: `README.md:5`

**Step 1: Replace LICENSE file**

Replace contents of `LICENSE` with the BSL 1.1 text. Parameters:
- Licensor: Shawn Nabizada
- Licensed Work: RustKanban
- Additional Use Grant: Non-commercial use is permitted
- Change Date: Four years from each release date
- Change License: Apache License, Version 2.0

Use the standard BSL 1.1 template from https://mariadb.com/bsl11/

**Step 2: Update Cargo.toml license field**

Change `license = "MIT"` to `license = "BSL-1.1"`.

**Step 3: Update README badge**

Change `[![License: MIT](https://img.shields.io/badge/license-MIT-blue.svg)](LICENSE)` to `[![License: BSL-1.1](https://img.shields.io/badge/license-BSL--1.1-blue.svg)](LICENSE)`.

**Step 4: Commit**

```
git add LICENSE Cargo.toml README.md
git commit -m "Switch license from MIT to BSL-1.1"
```

---

### Task 2: Tests for db.rs

**Files:**
- Modify: `src/db.rs` (add `#[cfg(test)] mod tests` at bottom)

All tests use `Connection::open_in_memory()` + `run_migrations()`. Need to make `run_migrations` pub(crate) or call `init_db` with a temp path. Simplest: add a `pub fn init_db_memory()` helper for tests.

**Step 1: Add test helper**

Add at end of `src/db.rs`:

```rust
#[cfg(test)]
pub fn init_db_memory() -> Connection {
    let conn = Connection::open_in_memory().unwrap();
    conn.execute_batch("PRAGMA foreign_keys = ON;").unwrap();
    run_migrations(&conn).unwrap();
    conn
}
```

**Step 2: Write db tests**

Add `#[cfg(test)] mod tests` block covering:

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::{Column, Priority};

    fn setup() -> Connection {
        init_db_memory()
    }

    #[test]
    fn test_insert_and_load_task() {
        let conn = setup();
        let id = insert_task(&conn, "Test", "Desc", Priority::High, Column::Todo, None).unwrap();
        let tasks = load_tasks(&conn).unwrap();
        assert_eq!(tasks.len(), 1);
        assert_eq!(tasks[0].id, id);
        assert_eq!(tasks[0].title, "Test");
        assert_eq!(tasks[0].priority, Priority::High);
        assert_eq!(tasks[0].column, Column::Todo);
    }

    #[test]
    fn test_update_task() {
        let conn = setup();
        let id = insert_task(&conn, "Old", "", Priority::Low, Column::Todo, None).unwrap();
        update_task(&conn, id, "New", "desc", Priority::High, None).unwrap();
        let tasks = load_tasks(&conn).unwrap();
        assert_eq!(tasks[0].title, "New");
        assert_eq!(tasks[0].priority, Priority::High);
    }

    #[test]
    fn test_update_task_column() {
        let conn = setup();
        let id = insert_task(&conn, "T", "", Priority::Medium, Column::Todo, None).unwrap();
        update_task_column(&conn, id, Column::Done).unwrap();
        let tasks = load_tasks(&conn).unwrap();
        assert_eq!(tasks[0].column, Column::Done);
    }

    #[test]
    fn test_update_task_priority() {
        let conn = setup();
        let id = insert_task(&conn, "T", "", Priority::Low, Column::Todo, None).unwrap();
        update_task_priority(&conn, id, Priority::High).unwrap();
        let tasks = load_tasks(&conn).unwrap();
        assert_eq!(tasks[0].priority, Priority::High);
    }

    #[test]
    fn test_delete_task() {
        let conn = setup();
        let id = insert_task(&conn, "T", "", Priority::Medium, Column::Todo, None).unwrap();
        delete_task(&conn, id).unwrap();
        assert!(load_tasks(&conn).unwrap().is_empty());
    }

    #[test]
    fn test_tags_crud() {
        let conn = setup();
        let id = insert_tag(&conn, "bug").unwrap();
        let tags = load_tags(&conn).unwrap();
        assert_eq!(tags.len(), 1);
        assert_eq!(tags[0].name, "bug");

        rename_tag(&conn, id, "feature").unwrap();
        let tags = load_tags(&conn).unwrap();
        assert_eq!(tags[0].name, "feature");

        delete_tag(&conn, id).unwrap();
        assert!(load_tags(&conn).unwrap().is_empty());
    }

    #[test]
    fn test_task_tags() {
        let conn = setup();
        let task_id = insert_task(&conn, "T", "", Priority::Medium, Column::Todo, None).unwrap();
        let tag1 = insert_tag(&conn, "a").unwrap();
        let tag2 = insert_tag(&conn, "b").unwrap();
        set_task_tags(&conn, task_id, &[tag1, tag2]).unwrap();

        let ids = get_task_tag_ids(&conn, task_id).unwrap();
        assert_eq!(ids.len(), 2);

        let tasks = load_tasks(&conn).unwrap();
        assert_eq!(tasks[0].tags.len(), 2);
    }

    #[test]
    fn test_reset_db() {
        let conn = setup();
        insert_task(&conn, "T", "", Priority::Medium, Column::Todo, None).unwrap();
        insert_tag(&conn, "x").unwrap();
        reset_db(&conn).unwrap();
        assert!(load_tasks(&conn).unwrap().is_empty());
        assert!(load_tags(&conn).unwrap().is_empty());
    }

    #[test]
    fn test_due_date_roundtrip() {
        let conn = setup();
        let date = chrono::NaiveDate::from_ymd_opt(2026, 6, 15);
        insert_task(&conn, "T", "", Priority::Medium, Column::Todo, date).unwrap();
        let tasks = load_tasks(&conn).unwrap();
        assert_eq!(tasks[0].due_date, date);
    }
}
```

**Step 3: Run tests**

Run: `cargo test -- db::tests`
Expected: all pass

**Step 4: Commit**

```
git add src/db.rs
git commit -m "Add unit tests for db module"
```

---

### Task 3: Tests for undo.rs, export.rs, theme.rs

**Files:**
- Modify: `src/undo.rs` (add tests at bottom)
- Modify: `src/export.rs` (add tests at bottom)
- Modify: `src/theme.rs` (add tests at bottom)

**Step 1: Write undo tests**

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::{Column, Priority};

    #[test]
    fn test_push_pop() {
        let mut stack = UndoStack::new();
        stack.push(UndoAction::MoveTask { task_id: 1, from_column: Column::Todo });
        assert!(stack.pop().is_some());
        assert!(stack.pop().is_none());
    }

    #[test]
    fn test_max_capacity() {
        let mut stack = UndoStack::new(); // max 20
        for i in 0..25 {
            stack.push(UndoAction::MoveTask { task_id: i, from_column: Column::Todo });
        }
        // Should have 20, oldest dropped
        let mut count = 0;
        while stack.pop().is_some() {
            count += 1;
        }
        assert_eq!(count, 20);
    }

    #[test]
    fn test_lifo_order() {
        let mut stack = UndoStack::new();
        stack.push(UndoAction::MoveTask { task_id: 1, from_column: Column::Todo });
        stack.push(UndoAction::MoveTask { task_id: 2, from_column: Column::Done });
        if let Some(UndoAction::MoveTask { task_id, .. }) = stack.pop() {
            assert_eq!(task_id, 2);
        }
    }
}
```

**Step 2: Write export tests**

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use crate::db;

    #[test]
    fn test_export_empty() {
        let conn = db::init_db_memory();
        let json = export_json(&conn).unwrap();
        assert!(json.contains("\"tasks\": []"));
        assert!(json.contains("\"version\": 1"));
    }

    #[test]
    fn test_export_import_roundtrip() {
        let conn = db::init_db_memory();
        db::insert_tag(&conn, "bug").unwrap();
        let task_id = db::insert_task(&conn, "Fix it", "desc", Priority::High, Column::InProgress, None).unwrap();
        let tag_ids = db::load_tags(&conn).unwrap();
        db::set_task_tags(&conn, task_id, &[tag_ids[0].id]).unwrap();

        let json = export_json(&conn).unwrap();

        // Import into fresh DB
        let conn2 = db::init_db_memory();
        let count = import_json(&conn2, &json).unwrap();
        assert_eq!(count, 1);

        let tasks = db::load_tasks(&conn2).unwrap();
        assert_eq!(tasks[0].title, "Fix it");
        assert_eq!(tasks[0].priority, Priority::High);
        assert_eq!(tasks[0].tags, vec!["bug"]);
    }

    #[test]
    fn test_import_deduplicates_tags() {
        let conn = db::init_db_memory();
        db::insert_tag(&conn, "existing").unwrap();

        let json = r#"{"version":1,"tasks":[],"tags":["existing","new"]}"#;
        import_json(&conn, json).unwrap();

        let tags = db::load_tags(&conn).unwrap();
        assert_eq!(tags.len(), 2);
    }
}
```

**Step 3: Write theme tests**

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use ratatui::style::Color;

    #[test]
    fn test_parse_named_colors() {
        assert_eq!(parse_color("Red"), Some(Color::Red));
        assert_eq!(parse_color("cyan"), Some(Color::Cyan));
        assert_eq!(parse_color("YELLOW"), Some(Color::Yellow));
    }

    #[test]
    fn test_parse_hex_color() {
        assert_eq!(parse_color("#FF0000"), Some(Color::Rgb(255, 0, 0)));
        assert_eq!(parse_color("#00ff00"), Some(Color::Rgb(0, 255, 0)));
    }

    #[test]
    fn test_parse_invalid_color() {
        assert_eq!(parse_color("notacolor"), None);
        assert_eq!(parse_color("#GG0000"), None);
        assert_eq!(parse_color(""), None);
    }

    #[test]
    fn test_default_theme() {
        let theme = Theme::default();
        assert_eq!(theme.cursor, Color::Cyan);
        assert_eq!(theme.priority_high, Color::Red);
    }

    #[test]
    fn test_priority_color() {
        let theme = Theme::default();
        assert_eq!(theme.priority_color(&crate::model::Priority::High), Color::Red);
        assert_eq!(theme.priority_color(&crate::model::Priority::Low), Color::Green);
    }
}
```

Note: `parse_color` is currently private. Change to `pub(crate)` or keep tests in the same module (which has access to private fns). Since tests are `#[cfg(test)] mod tests` inside the file, they can access private fns.

**Step 4: Run all tests**

Run: `cargo test`
Expected: all pass

**Step 5: Commit**

```
git add src/undo.rs src/export.rs src/theme.rs
git commit -m "Add unit tests for undo, export, and theme modules"
```

---

### Task 4: CI Improvements + Pre-commit Hooks

**Files:**
- Modify: `.github/workflows/ci.yml`
- Create: `.githooks/pre-commit`

**Step 1: Add fmt check to CI**

Add after the Clippy step in `ci.yml`:

```yaml
      - name: Format check
        run: cargo fmt -- --check
```

Also add `rustfmt` to the components list:

```yaml
          components: clippy, rustfmt
```

**Step 2: Create pre-commit hook**

Create `.githooks/pre-commit`:

```bash
#!/bin/sh
set -e

echo "Running cargo fmt check..."
cargo fmt -- --check
if [ $? -ne 0 ]; then
    echo "Format check failed. Run 'cargo fmt' to fix."
    exit 1
fi

echo "Running cargo clippy..."
cargo clippy -- -D warnings
if [ $? -ne 0 ]; then
    echo "Clippy check failed."
    exit 1
fi

echo "All checks passed."
```

Make executable: `chmod +x .githooks/pre-commit`

**Step 3: Commit**

```
git add .github/workflows/ci.yml .githooks/pre-commit
git commit -m "Add cargo fmt to CI, add pre-commit hook"
```

---

### Task 5: CLAUDE.md

**Files:**
- Create: `CLAUDE.md`

**Step 1: Write CLAUDE.md**

```markdown
# RustKanban Development Guide

## Build & Test
- `cargo build` — compile
- `cargo test` — run all tests
- `cargo clippy` — lint
- `cargo fmt` — format
- `cargo install --path .` — install locally as `rk`

## Architecture
Single-threaded TUI app. Event loop: render → poll (100ms) → handle → tick → repeat.

### Module Map
- `main.rs` — CLI (clap), terminal setup, event loop
- `app.rs` — App state struct, all business logic methods
- `db.rs` — SQLite CRUD (rusqlite). All mutations go through DB first, then `reload_tasks()`
- `model.rs` — Task, Tag, Priority, Column types
- `handler.rs` — Input dispatch: matches AppMode, delegates to App methods
- `event.rs` — Crossterm event polling wrapper
- `undo.rs` — UndoAction enum + VecDeque stack (cap 20)
- `export.rs` — JSON export/import (serde)
- `theme.rs` — Theme config from ~/.config/rustkanban/theme.toml
- `ui/` — All rendering. `mod.rs` is entry point, delegates to submodules

### Key Patterns
- State machine: `AppMode` enum drives which handler + UI overlay is active
- DB-first: mutate DB, call `reload_tasks()`, never cache separately
- Theme: `app.theme` has all colors. Use `app.theme.priority_color(&p)` not hardcoded colors
- Tests: use `db::init_db_memory()` for in-memory SQLite

## Conventions
- No `unwrap()` in production code (ok in tests)
- Keep `handler.rs` thin — it maps keys to App methods, no logic
- All user-facing strings in render code, not in App methods (except flash messages)
- `cargo clippy` must pass with zero warnings before commit
```

**Step 2: Commit**

```
git add CLAUDE.md
git commit -m "Add CLAUDE.md project guide"
```

---

### Task 6: CHANGELOG.md

**Files:**
- Create: `CHANGELOG.md`

**Step 1: Write initial CHANGELOG**

```markdown
# Changelog

All notable changes to this project will be documented in this file.

Format based on [Keep a Changelog](https://keepachangelog.com/).

## [Unreleased]

## [0.1.0] - 2026-03-06

### Added
- 3-column kanban board (Todo, In Progress, Done)
- Vim-inspired navigation (J/L columns, Up/Down/Tab tasks)
- Task management (create, edit, delete, move, cycle priority)
- Multiple tags per task with toggle selection in modal
- Tag management screen (create, rename, delete)
- Tag filtering via sort menu
- Live search filtering by title or description
- Sort by due date (default) or priority
- Due date warnings with color-coded urgency
- Undo up to 20 actions (move, edit, delete, priority change)
- SQLite persistence at ~/.local/share/rustkanban/kanban.db
- JSON export (`rk export`) and import (`rk import <file>`)
- Theme configuration via ~/.config/rustkanban/theme.toml
- Shell completions (bash, zsh, fish, powershell)
- Cross-platform binaries (Linux x86/ARM, macOS Intel/Silicon, Windows)
- Automated releases via GitHub Actions
```

**Step 2: Commit**

```
git add CHANGELOG.md
git commit -m "Add CHANGELOG.md"
```

---

### Task 7: Duplicate Task

**Files:**
- Modify: `src/app.rs` (add `duplicate_task` method, ~line 497)
- Modify: `src/handler.rs:43` (add `C` keybinding)
- Modify: `src/db.rs` (uses existing `insert_task` + `set_task_tags`)

**Step 1: Add `duplicate_task` to App**

Add after `confirm_delete` in `src/app.rs`:

```rust
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
                ) {
                    if !tag_ids.is_empty() {
                        let _ = db::set_task_tags(&self.db, new_id, &tag_ids);
                    }
                    self.undo_stack.push(UndoAction::DeleteTask {
                        title: task.title.clone(),
                        description: task.description.clone(),
                        priority: task.priority,
                        column: task.column,
                        due_date: task.due_date,
                    });
                    self.reload_tasks();
                    self.set_cursor_to_task(new_id, task.column);
                    self.set_flash(format!("Duplicated '{}'", task.title));
                }
            }
        }
    }
```

**Step 2: Add keybinding in handler.rs**

In `handle_board`, add after the `KeyCode::Char('d')` line:

```rust
        KeyCode::Char('c') | KeyCode::Char('C') => app.duplicate_task(),
```

**Step 3: Run tests + build**

Run: `cargo build && cargo test`
Expected: pass

**Step 4: Commit**

```
git add src/app.rs src/handler.rs
git commit -m "Add duplicate task with C key"
```

---

### Task 8: Persistent Preferences — DB Layer

**Files:**
- Modify: `src/db.rs` (add table + get/set functions)

**Step 1: Add preferences table to migrations**

In `run_migrations`, add after the `task_tags` CREATE:

```sql
        CREATE TABLE IF NOT EXISTS preferences (
            key TEXT PRIMARY KEY,
            value TEXT NOT NULL
        );
```

**Step 2: Add get/set functions**

Add to `src/db.rs`:

```rust
pub fn get_preference(conn: &Connection, key: &str) -> Option<String> {
    conn.query_row(
        "SELECT value FROM preferences WHERE key = ?1",
        rusqlite::params![key],
        |row| row.get(0),
    )
    .ok()
}

pub fn set_preference(conn: &Connection, key: &str, value: &str) -> SqliteResult<()> {
    conn.execute(
        "INSERT INTO preferences (key, value) VALUES (?1, ?2)
         ON CONFLICT(key) DO UPDATE SET value = ?2",
        rusqlite::params![key, value],
    )?;
    Ok(())
}
```

**Step 3: Add tests**

Add to the db tests module:

```rust
    #[test]
    fn test_preferences() {
        let conn = setup();
        assert_eq!(get_preference(&conn, "sort_mode"), None);
        set_preference(&conn, "sort_mode", "Priority").unwrap();
        assert_eq!(get_preference(&conn, "sort_mode"), Some("Priority".to_string()));
        set_preference(&conn, "sort_mode", "DueDate").unwrap();
        assert_eq!(get_preference(&conn, "sort_mode"), Some("DueDate".to_string()));
    }
```

**Step 4: Run tests**

Run: `cargo test -- db::tests`
Expected: pass

**Step 5: Commit**

```
git add src/db.rs
git commit -m "Add preferences table with get/set functions"
```

---

### Task 9: Persistent Preferences — App Integration

**Files:**
- Modify: `src/app.rs` (load on init, save on change and quit)

**Step 1: Load preferences in App::new**

After `sort_mode: SortMode::DueDate,` in `App::new`, replace with preference loading:

```rust
            sort_mode: match db::get_preference(&db, "sort_mode").as_deref() {
                Some("Priority") => SortMode::Priority,
                _ => SortMode::DueDate,
            },
```

And for `focused_column`:

```rust
            focused_column: match db::get_preference(&db, "focused_column").as_deref() {
                Some("in_progress") => Column::InProgress,
                Some("done") => Column::Done,
                _ => Column::Todo,
            },
```

**Step 2: Save sort_mode on change**

In `sort_menu_select`, after setting `self.sort_mode`, add:

```rust
        let _ = db::set_preference(&self.db, "sort_mode", match self.sort_mode {
            SortMode::DueDate => "DueDate",
            SortMode::Priority => "Priority",
        });
```

**Step 3: Save focused_column on quit**

In `quit`, before `self.running = false`, add:

```rust
        let _ = db::set_preference(&self.db, "focused_column", self.focused_column.as_str());
```

Note: `Column::as_str()` returns "todo", "in_progress", "done" — matches the loading logic.

**Step 4: Build and test**

Run: `cargo build && cargo test`
Expected: pass

**Step 5: Commit**

```
git add src/app.rs
git commit -m "Persist sort mode and focused column across sessions"
```

---

### Task 10: Mouse Support — Event Layer

**Files:**
- Modify: `src/event.rs` (return AppEvent enum, enable mouse capture)
- Modify: `src/main.rs` (dispatch on AppEvent, enable/disable mouse)
- Modify: `src/handler.rs` (add handle_mouse function)
- Modify: `src/app.rs` (add terminal_size and drag_state fields)

**Step 1: Update event.rs**

Replace `src/event.rs` entirely:

```rust
use std::time::Duration;

use ratatui::crossterm::event::{
    self, Event, KeyEvent, KeyEventKind, MouseEvent, MouseEventKind,
};

pub enum AppEvent {
    Key(KeyEvent),
    Mouse(MouseEvent),
}

pub fn poll_event(timeout: Duration) -> std::io::Result<Option<AppEvent>> {
    if event::poll(timeout)? {
        match event::read()? {
            Event::Key(key) if key.kind == KeyEventKind::Press => {
                return Ok(Some(AppEvent::Key(key)));
            }
            Event::Mouse(mouse) => {
                match mouse.kind {
                    MouseEventKind::Down(_)
                    | MouseEventKind::Up(_)
                    | MouseEventKind::ScrollDown
                    | MouseEventKind::ScrollUp => {
                        return Ok(Some(AppEvent::Mouse(mouse)));
                    }
                    _ => {}
                }
            }
            _ => {}
        }
    }
    Ok(None)
}
```

**Step 2: Add mouse state to App**

In `src/app.rs`, add to `App` struct:

```rust
    pub terminal_width: u16,
    pub terminal_height: u16,
    // Mouse drag state
    pub drag_task: Option<(i64, Column)>, // (task_id, from_column)
```

Initialize in `App::new`:

```rust
            terminal_width: 0,
            terminal_height: 0,
            drag_task: None,
```

**Step 3: Update main.rs event loop**

Enable mouse capture after `ratatui::init()`:

```rust
    use ratatui::crossterm::execute;
    use ratatui::crossterm::event::{EnableMouseCapture, DisableMouseCapture};
    execute!(std::io::stdout(), EnableMouseCapture)?;
```

Before `ratatui::restore()`:

```rust
    execute!(std::io::stdout(), DisableMouseCapture)?;
```

Update the event dispatch:

```rust
        app.terminal_width = size.width;
        app.terminal_height = size.height;

        if let Some(ev) = event::poll_event(Duration::from_millis(100))? {
            match ev {
                event::AppEvent::Key(key) => handler::handle_event(&mut app, key),
                event::AppEvent::Mouse(mouse) => handler::handle_mouse(&mut app, mouse),
            }
        }
```

**Step 4: Add stub handle_mouse to handler.rs**

```rust
use ratatui::crossterm::event::MouseEvent;

pub fn handle_mouse(app: &mut App, _mouse: MouseEvent) {
    // Implemented in next task
    let _ = app;
}
```

**Step 5: Build**

Run: `cargo build`
Expected: pass (mouse events captured but not handled yet)

**Step 6: Commit**

```
git add src/event.rs src/main.rs src/handler.rs src/app.rs
git commit -m "Add mouse event layer with AppEvent enum"
```

---

### Task 11: Mouse Support — Click, Scroll, and Drag

**Files:**
- Modify: `src/handler.rs` (implement handle_mouse)
- Modify: `src/app.rs` (add helper methods for mouse hit detection)

**Step 1: Add mouse helper methods to App**

Add to `src/app.rs`:

```rust
    /// Determine which column a screen x-coordinate falls in.
    pub fn column_at_x(&self, x: u16) -> Option<Column> {
        if self.terminal_width == 0 {
            return None;
        }
        let col_width = self.terminal_width / 3;
        let idx = (x / col_width).min(2) as usize;
        Column::from_index(idx)
    }

    /// Determine which task index a screen y-coordinate maps to within a column.
    /// Returns None if y is outside the board area or on a border.
    pub fn task_at_y(&self, col: Column, y: u16) -> Option<usize> {
        // Board inner area starts at y=1 (after top border)
        if y == 0 {
            return None;
        }
        let inner_y = (y - 1) as usize; // subtract top border

        let col_width = (self.terminal_width / 3).saturating_sub(2) as usize;
        let prefix_len = 6;
        let title_width = col_width.saturating_sub(prefix_len).max(1);

        let tasks = self.tasks_for_column(col);
        let scroll = self.scroll_offsets[col.index()];
        let target_line = inner_y + scroll;

        let mut line = 0;
        for (i, task) in tasks.iter().enumerate() {
            let title_lines = wrapped_line_count(&task.title, title_width);
            let tag_lines = if task.tags.is_empty() { 0 } else { 1 };
            let task_height = title_lines + tag_lines + 1;

            if target_line >= line && target_line < line + task_height {
                return Some(i);
            }
            line += task_height;
        }
        None
    }

    pub fn focus_column_and_task(&mut self, col: Column, task_idx: usize) {
        self.focused_column = col;
        self.cursor_positions[col.index()] = task_idx;
    }

    pub fn scroll_column(&mut self, col: Column, delta: i32) {
        let idx = col.index();
        if delta < 0 {
            self.scroll_offsets[idx] = self.scroll_offsets[idx].saturating_sub((-delta) as usize);
        } else {
            self.scroll_offsets[idx] = self.scroll_offsets[idx].saturating_add(delta as usize);
        }
    }
```

**Step 2: Implement handle_mouse**

Replace the stub in `handler.rs`:

```rust
use ratatui::crossterm::event::{MouseButton, MouseEvent, MouseEventKind};

pub fn handle_mouse(app: &mut App, mouse: MouseEvent) {
    // Don't handle mouse in modal modes
    match app.mode {
        AppMode::Board | AppMode::Selected => {}
        _ => return,
    }

    if app.show_help {
        // Click anywhere dismisses help
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
                        let _ = db::update_task_column(&app.db, task_id, to_col);
                        app.undo_stack.push(UndoAction::MoveTask {
                            task_id,
                            from_column: from_col,
                        });
                        app.reload_tasks();
                        app.clamp_cursor(from_col);
                        app.focused_column = to_col;
                        app.set_cursor_to_task(task_id, to_col);
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
```

Note: add `use crate::db;` and `use crate::undo::UndoAction;` at top of handler.rs.

**Step 3: Build and test**

Run: `cargo build && cargo clippy`
Expected: pass

**Step 4: Commit**

```
git add src/handler.rs src/app.rs
git commit -m "Implement mouse click, scroll, and drag-to-move"
```

---

### Task 12: Search Match Highlighting

**Files:**
- Modify: `src/ui/board.rs` (change title rendering when search is active)

**Step 1: Add highlight helper function**

Add to `src/ui/board.rs`:

```rust
fn highlight_matches<'a>(text: &'a str, query: &str, base_style: Style, highlight_style: Style) -> Vec<Span<'a>> {
    if query.is_empty() {
        return vec![Span::styled(text, base_style)];
    }

    let lower = text.to_lowercase();
    let query_lower = query.to_lowercase();
    let mut spans = Vec::new();
    let mut last_end = 0;

    for (start, _) in lower.match_indices(&query_lower) {
        if start > last_end {
            spans.push(Span::styled(&text[last_end..start], base_style));
        }
        let end = start + query.len();
        spans.push(Span::styled(
            &text[start..end],
            highlight_style,
        ));
        last_end = end;
    }

    if last_end < text.len() {
        spans.push(Span::styled(&text[last_end..], base_style));
    }

    if spans.is_empty() {
        spans.push(Span::styled(text, base_style));
    }

    spans
}
```

**Step 2: Use in title rendering**

In the task rendering loop, where the first title chunk is rendered (the line with `Span::styled(chunk.clone(), title_style)`), replace with highlighted version when search is active:

For j == 0 (first line with cursor marker + priority):
```rust
                if j == 0 {
                    let mut line_spans = vec![
                        Span::styled(String::from(cursor_marker), cursor_style),
                        Span::styled(
                            indicator.clone(),
                            Style::default()
                                .fg(priority_color)
                                .add_modifier(Modifier::BOLD),
                        ),
                    ];
                    if app.search_active && !app.search_query.is_empty() {
                        let hl_style = title_style.add_modifier(Modifier::UNDERLINED);
                        line_spans.extend(highlight_matches(chunk, &app.search_query, title_style, hl_style));
                    } else {
                        line_spans.push(Span::styled(chunk.clone(), title_style));
                    }
                    all_lines.push(Line::from(line_spans));
```

For continuation lines (j > 0):
```rust
                } else {
                    let mut line_spans = vec![Span::raw(indent.clone())];
                    if app.search_active && !app.search_query.is_empty() {
                        let hl_style = title_style.add_modifier(Modifier::UNDERLINED);
                        line_spans.extend(highlight_matches(chunk, &app.search_query, title_style, hl_style));
                    } else {
                        line_spans.push(Span::styled(chunk.clone(), title_style));
                    }
                    all_lines.push(Line::from(line_spans));
                }
```

**Step 3: Build**

Run: `cargo build && cargo clippy`
Expected: pass

**Step 4: Commit**

```
git add src/ui/board.rs
git commit -m "Highlight search matches in task titles"
```

---

### Task 13: VHS Demo Tape

**Files:**
- Create: `demo.tape`

**Step 1: Create demo.tape**

```
Output demo.gif

Set FontSize 14
Set Width 1200
Set Height 600
Set Theme "Catppuccin Mocha"

Type "rk"
Enter
Sleep 1s

# Create first task
Type " "
Sleep 500ms
Type "Review pull request"
Tab
Type "Check for security issues"
Tab
Sleep 300ms
Type " "
Sleep 300ms
Tab
Tab
Type "2026"
Tab
Type "3"
Tab
Type "15"
Ctrl+s
Sleep 1s

# Create second task
Type " "
Sleep 500ms
Type "Write documentation"
Ctrl+s
Sleep 1s

# Navigate and move task
Type "k"
Sleep 500ms
Type "l"
Sleep 500ms
Type "l"
Sleep 500ms
Type "k"
Sleep 1s

# Search
Type "/"
Sleep 300ms
Type "review"
Sleep 1s
Escape
Sleep 500ms

# Quit
Type "q"
Sleep 500ms
```

**Step 2: Document recording**

To record: `vhs demo.tape` (requires `vhs` installed: `go install github.com/charmbracelet/vhs@latest`)

The resulting `demo.gif` is referenced in README.

**Step 3: Commit**

```
git add demo.tape
git commit -m "Add VHS demo tape for GIF recording"
```

---

### Task 14: Update README

**Files:**
- Modify: `README.md`

**Step 1: Update all sections**

Key changes:
- Badge: update license to BSL-1.1
- Add demo GIF after badges: `![Demo](demo.gif)` (placeholder until recorded)
- Feature list: add export/import, theme, multiple tags, mouse, duplicate task
- Usage section: add `rk export`, `rk import`, `rk theme`, `rk theme --init`
- Keybindings: add `C` for duplicate, update Tag description, add mouse section
- Add Theme section
- Add Export/Import section
- Update Shell Completions section (already there)

**Step 2: Commit**

```
git add README.md
git commit -m "Update README with new features and BSL license"
```

---

### Task 15: Man Page

**Files:**
- Modify: `Cargo.toml` (add clap_mangen build dep)
- Modify: `src/main.rs` (add Manpage subcommand)

**Step 1: Add clap_mangen dependency**

Add to `[dependencies]` in Cargo.toml:

```toml
clap_mangen = "0.2"
```

**Step 2: Add Manpage subcommand**

Add to `Commands` enum:

```rust
    /// Generate man page
    Manpage,
```

Add handler in main():

```rust
        Some(Commands::Manpage) => {
            let cmd = Cli::command();
            let man = clap_mangen::Man::new(cmd);
            man.render(&mut io::stdout())?;
        }
```

**Step 3: Build**

Run: `cargo build`
Expected: pass

**Step 4: Commit**

```
git add Cargo.toml src/main.rs
git commit -m "Add man page generation via rk manpage"
```

---

### Task 16: Homebrew Formula

**Files:**
- Create: `HomebrewFormula/rk.rb`

**Step 1: Create formula**

```ruby
class Rk < Formula
  desc "A terminal (TUI) kanban board with vim-inspired navigation"
  homepage "https://github.com/shawn-nabizada/rustkanban"
  version "0.1.0"
  license "BSL-1.1"

  on_macos do
    on_arm do
      url "https://github.com/shawn-nabizada/rustkanban/releases/latest/download/rk-macos-aarch64"
      sha256 "PLACEHOLDER"
    end
    on_intel do
      url "https://github.com/shawn-nabizada/rustkanban/releases/latest/download/rk-macos-x86_64"
      sha256 "PLACEHOLDER"
    end
  end

  on_linux do
    on_arm do
      url "https://github.com/shawn-nabizada/rustkanban/releases/latest/download/rk-linux-aarch64"
      sha256 "PLACEHOLDER"
    end
    on_intel do
      url "https://github.com/shawn-nabizada/rustkanban/releases/latest/download/rk-linux-x86_64"
      sha256 "PLACEHOLDER"
    end
  end

  def install
    bin.install stable.url.split("/").last => "rk"
  end

  test do
    assert_match "kanban", shell_output("#{bin}/rk --help")
  end
end
```

Note: SHA256 values must be updated after each release. To create a tap, the user creates a `homebrew-rustkanban` repo containing `Formula/rk.rb`.

**Step 2: Commit**

```
git add HomebrewFormula/rk.rb
git commit -m "Add Homebrew formula"
```

---

### Task 17: AUR Package

**Files:**
- Create: `aur/PKGBUILD`

**Step 1: Create PKGBUILD**

```bash
# Maintainer: Shawn Nabizada
pkgname=rustkanban-bin
pkgver=0.1.0
pkgrel=1
pkgdesc="A terminal (TUI) kanban board with vim-inspired navigation"
arch=('x86_64' 'aarch64')
url="https://github.com/shawn-nabizada/rustkanban"
license=('BSL-1.1')
provides=('rk')
conflicts=('rustkanban')

source_x86_64=("https://github.com/shawn-nabizada/rustkanban/releases/download/v${pkgver}/rk-linux-x86_64")
source_aarch64=("https://github.com/shawn-nabizada/rustkanban/releases/download/v${pkgver}/rk-linux-aarch64")
sha256sums_x86_64=('SKIP')
sha256sums_aarch64=('SKIP')

package() {
    if [ "$CARCH" = "x86_64" ]; then
        install -Dm755 "rk-linux-x86_64" "$pkgdir/usr/bin/rk"
    elif [ "$CARCH" = "aarch64" ]; then
        install -Dm755 "rk-linux-aarch64" "$pkgdir/usr/bin/rk"
    fi
}
```

**Step 2: Commit**

```
git add aur/PKGBUILD
git commit -m "Add AUR PKGBUILD"
```

---

### Task 18: Install Script Checksums

**Files:**
- Modify: `install.sh` (add SHA256 verification)
- Modify: `.github/workflows/release.yml` (generate checksums)

**Step 1: Update release workflow to generate checksums**

In the `release` job, after downloading artifacts and before creating the release, add:

```yaml
      - name: Generate checksums
        run: |
          cd artifacts
          sha256sum * > checksums.sha256
          cat checksums.sha256
```

Update the release files line to include checksums:

```yaml
          files: |
            artifacts/*
```

(checksums.sha256 is already in artifacts/ so this works as-is)

**Step 2: Update install.sh with verification**

Replace the download section in `install.sh`:

```bash
CHECKSUMS_URL="https://github.com/$REPO/releases/latest/download/checksums.sha256"

echo "Downloading $BINARY for $(uname -s) $(uname -m)..."
curl -sL -o "$BINARY" "$DOWNLOAD_URL"
chmod +x "$BINARY"

echo "Verifying checksum..."
EXPECTED=$(curl -sL "$CHECKSUMS_URL" | grep "$PLATFORM" | awk '{print $1}')
if [ -n "$EXPECTED" ]; then
    ACTUAL=$(sha256sum "$BINARY" 2>/dev/null || shasum -a 256 "$BINARY" 2>/dev/null | awk '{print $1}')
    ACTUAL=$(echo "$ACTUAL" | awk '{print $1}')
    if [ "$EXPECTED" != "$ACTUAL" ]; then
        echo "Checksum verification FAILED!" >&2
        echo "Expected: $EXPECTED" >&2
        echo "Actual:   $ACTUAL" >&2
        rm -f "$BINARY"
        exit 1
    fi
    echo "Checksum verified."
else
    echo "Warning: could not fetch checksums, skipping verification."
fi
```

**Step 3: Commit**

```
git add install.sh .github/workflows/release.yml
git commit -m "Add SHA256 checksum verification to install script and releases"
```

---

### Task 19: Man Page in Release Workflow

**Files:**
- Modify: `.github/workflows/release.yml`

**Step 1: Add man page generation to release workflow**

Add a new job `manpage` that runs after `version`:

```yaml
  manpage:
    needs: version
    if: needs.version.outputs.skipped != 'true'
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4

      - name: Install Rust
        uses: dtolnay/rust-toolchain@stable

      - name: Generate man page
        run: cargo run -- manpage > rk.1

      - name: Upload man page
        uses: actions/upload-artifact@v4
        with:
          name: rk.1
          path: rk.1
```

Update the `release` job `needs` to include `manpage`:

```yaml
    needs: [version, build, manpage]
```

**Step 2: Commit**

```
git add .github/workflows/release.yml
git commit -m "Generate and include man page in releases"
```

---

## Task Dependency Graph

```
Task 1 (License) ──────────────────────────────────────┐
Task 2 (DB tests) ─────────────────────────────────────┤
Task 3 (Undo/Export/Theme tests) ───────────────────────┤
Task 4 (CI + hooks) ───────────────────────────────────┤
Task 5 (CLAUDE.md) ────────────────────────────────────┤
Task 6 (CHANGELOG) ────────────────────────────────────┤
Task 7 (Duplicate task) ───────────────────────────────┤
Task 8 (Prefs DB) ──→ Task 9 (Prefs App) ──────────────┤
Task 10 (Mouse events) ──→ Task 11 (Mouse handling) ───┤
Task 12 (Search highlight) ────────────────────────────┤
Task 13 (VHS) ──→ Task 14 (README update) ─────────────┤
Task 15 (Man page) ──→ Task 19 (Man in release) ───────┤
Task 16 (Homebrew) ────────────────────────────────────┤
Task 17 (AUR) ─────────────────────────────────────────┤
Task 18 (Install checksums) ───────────────────────────┘
```

Independent tasks can be done in any order. Sequential dependencies:
- Task 9 requires Task 8
- Task 11 requires Task 10
- Task 14 should come after Task 13
- Task 19 requires Task 15
