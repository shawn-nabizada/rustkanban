# RustKanban Sync — Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Add cross-machine sync to RustKanban via a hosted Axum server with GitHub OAuth, so a single user's kanban board stays in sync across multiple devices.

**Architecture:** Cargo workspace with three crates: `rk-client` (existing TUI, moved), `rk-server` (Axum + Postgres), and `rk-shared` (sync payload types). Client syncs via REST API using bearer tokens. Last-write-wins conflict resolution. Soft deletes propagate across devices.

**Tech Stack:** Rust, ratatui, rusqlite, Axum, PostgreSQL (sqlx), GitHub OAuth, ureq (blocking HTTP client), uuid, serde

**Design doc:** `docs/plans/2026-03-08-sync-design.md`

---

## Phase 1: Workspace Restructuring

### Task 1: Convert to Cargo Workspace

Move the existing single-crate project into a workspace with `crates/rk-client/`.

**Files:**
- Modify: `Cargo.toml` (root — becomes workspace root)
- Create: `crates/rk-client/Cargo.toml`
- Move: `src/` → `crates/rk-client/src/`

**Step 1: Create workspace structure**

```bash
mkdir -p crates/rk-client
git mv src crates/rk-client/src
```

**Step 2: Create `crates/rk-client/Cargo.toml`**

Move all `[package]`, `[[bin]]`, and `[dependencies]` from the root `Cargo.toml` into this file. Update `[[bin]]` path:

```toml
[package]
name = "rk-client"
version = "0.1.0"
edition = "2021"
description = "A Rust terminal (TUI) kanban board with vim-inspired navigation, tags, search, and SQLite persistence"
license-file = "../../LICENSE"
keywords = ["kanban", "tui", "terminal", "productivity", "task"]
categories = ["command-line-utilities"]
repository = "https://github.com/shawn-nabizada/rustkanban"

[[bin]]
name = "rk"
path = "src/main.rs"

[dependencies]
ratatui = "0.29"
crossterm = "0.28"
rusqlite = { version = "0.32", features = ["bundled"] }
chrono = "0.4"
clap = { version = "4.5", features = ["derive"] }
clap_complete = "4.5"
dirs = "6.0"
unicode-width = "0.2"
serde = { version = "1", features = ["derive"] }
serde_json = "1"
toml = "0.8"
clap_mangen = "0.2"
```

**Step 3: Update root `Cargo.toml` to workspace**

```toml
[workspace]
members = ["crates/rk-client"]
resolver = "2"

[workspace.metadata.dist]
cargo-dist-version = "0.31.0"
ci = "github"
installers = ["shell"]
targets = [
    "aarch64-apple-darwin",
    "x86_64-apple-darwin",
    "x86_64-unknown-linux-gnu",
    "aarch64-unknown-linux-gnu",
    "x86_64-pc-windows-msvc",
]
install-path = "CARGO_HOME"

[profile.dist]
inherits = "release"
lto = "thin"
```

**Step 4: Verify build**

```bash
cargo build
cargo test
cargo clippy -- -D warnings
```

Expected: all pass with zero warnings. Binary `rk` still works.

**Step 5: Update CI and docs references**

- `.github/workflows/release.yml` — add `-p rk-client` to cross-compilation commands
- `CLAUDE.md` — update `cargo install --path .` → `cargo install --path crates/rk-client`

**Step 6: Commit**

```bash
git add -A
git commit -m "refactor: convert to cargo workspace, move client to crates/rk-client"
```

---

## Phase 2: Shared Types Crate

### Task 2: Create `rk-shared` Crate

**Files:**
- Create: `crates/rk-shared/Cargo.toml`
- Create: `crates/rk-shared/src/lib.rs`
- Modify: `Cargo.toml` (root — add member)

**Step 1: Create crate**

```bash
mkdir -p crates/rk-shared/src
```

**Step 2: Write `crates/rk-shared/Cargo.toml`**

```toml
[package]
name = "rk-shared"
version = "0.1.0"
edition = "2021"

[dependencies]
serde = { version = "1", features = ["derive"] }
serde_json = "1"
```

**Step 3: Write shared types in `crates/rk-shared/src/lib.rs`**

```rust
use std::collections::HashMap;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SyncPayload {
    pub tasks: Vec<SyncTask>,
    pub tags: Vec<SyncTag>,
    pub last_synced_at: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SyncTask {
    pub uuid: String,
    pub title: String,
    #[serde(default)]
    pub description: String,
    #[serde(default = "default_priority")]
    pub priority: String,
    #[serde(default = "default_column")]
    pub column: String,
    #[serde(default)]
    pub due_date: Option<String>,
    #[serde(default)]
    pub tags: Vec<String>,
    pub created_at: String,
    pub updated_at: String,
    #[serde(default)]
    pub deleted: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SyncTag {
    pub uuid: String,
    pub name: String,
    pub updated_at: String,
    #[serde(default)]
    pub deleted: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SyncResponse {
    pub tasks: Vec<SyncTask>,
    pub tags: Vec<SyncTag>,
    #[serde(default)]
    pub tag_uuid_mappings: HashMap<String, String>,
    pub synced_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApiError {
    pub error: String,
    pub message: String,
}

fn default_priority() -> String {
    "Medium".to_string()
}

fn default_column() -> String {
    "todo".to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sync_task_defaults() {
        let json = r#"{"uuid":"abc","title":"t","created_at":"2026-01-01T00:00:00","updated_at":"2026-01-01T00:00:00"}"#;
        let task: SyncTask = serde_json::from_str(json).unwrap();
        assert_eq!(task.priority, "Medium");
        assert_eq!(task.column, "todo");
        assert_eq!(task.description, "");
        assert!(!task.deleted);
        assert!(task.tags.is_empty());
    }

    #[test]
    fn test_sync_response_defaults() {
        let json = r#"{"tasks":[],"tags":[],"synced_at":"2026-01-01T00:00:00"}"#;
        let resp: SyncResponse = serde_json::from_str(json).unwrap();
        assert!(resp.tag_uuid_mappings.is_empty());
    }

    #[test]
    fn test_roundtrip_payload() {
        let payload = SyncPayload {
            tasks: vec![SyncTask {
                uuid: "uuid-1".into(),
                title: "Test".into(),
                description: "desc".into(),
                priority: "High".into(),
                column: "done".into(),
                due_date: Some("2026-06-15".into()),
                tags: vec!["tag-uuid-1".into()],
                created_at: "2026-01-01T00:00:00".into(),
                updated_at: "2026-01-01T00:00:00".into(),
                deleted: false,
            }],
            tags: vec![SyncTag {
                uuid: "tag-uuid-1".into(),
                name: "bug".into(),
                updated_at: "2026-01-01T00:00:00".into(),
                deleted: false,
            }],
            last_synced_at: Some("2026-01-01T00:00:00".into()),
        };
        let json = serde_json::to_string(&payload).unwrap();
        let roundtrip: SyncPayload = serde_json::from_str(&json).unwrap();
        assert_eq!(roundtrip.tasks.len(), 1);
        assert_eq!(roundtrip.tags.len(), 1);
    }
}
```

**Step 4: Add to workspace**

In root `Cargo.toml`:
```toml
members = ["crates/rk-client", "crates/rk-shared"]
```

**Step 5: Verify**

```bash
cargo test -p rk-shared
cargo clippy -p rk-shared -- -D warnings
```

**Step 6: Commit**

```bash
git add crates/rk-shared Cargo.toml
git commit -m "feat: add rk-shared crate with sync payload types"
```

---

## Phase 3: Client Database Evolution

### Task 3: Schema Versioning

Add a `schema_version` preference key and migration runner that checks version before applying migrations.

**Files:**
- Modify: `crates/rk-client/src/db.rs`

**Step 1: Write failing test**

Add to the test module in `db.rs`:

```rust
#[test]
fn test_schema_version_tracking() {
    let conn = init_db_memory();
    let v = get_preference(&conn, "schema_version");
    assert_eq!(v, Some("2".to_string()));
}
```

**Step 2: Run test to verify it fails**

```bash
cargo test -p rk-client test_schema_version_tracking
```

Expected: FAIL — schema_version preference doesn't exist yet.

**Step 3: Implement versioned migration**

Refactor `run_migrations` in `db.rs` to:
1. Run the v1 schema creation (existing `CREATE TABLE IF NOT EXISTS` statements — these are idempotent)
2. Check `schema_version` from preferences (default "1" if missing)
3. If version < 2, run migration v2, set `schema_version = "2"`

```rust
pub(crate) fn run_migrations(conn: &Connection) -> SqliteResult<()> {
    // v1: base schema (idempotent)
    conn.execute_batch(
        "
        CREATE TABLE IF NOT EXISTS tasks (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            title TEXT NOT NULL,
            description TEXT NOT NULL DEFAULT '',
            priority TEXT NOT NULL DEFAULT 'Medium',
            column_name TEXT NOT NULL DEFAULT 'todo',
            due_date TEXT,
            created_at TEXT NOT NULL,
            updated_at TEXT NOT NULL
        );
        CREATE TABLE IF NOT EXISTS tags (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            name TEXT NOT NULL UNIQUE
        );
        CREATE TABLE IF NOT EXISTS task_tags (
            task_id INTEGER NOT NULL,
            tag_id INTEGER NOT NULL,
            PRIMARY KEY (task_id, tag_id),
            FOREIGN KEY (task_id) REFERENCES tasks(id) ON DELETE CASCADE,
            FOREIGN KEY (tag_id) REFERENCES tags(id) ON DELETE CASCADE
        );
        CREATE TABLE IF NOT EXISTS preferences (
            key TEXT PRIMARY KEY,
            value TEXT NOT NULL
        );
        ",
    )?;

    let version = get_schema_version(conn);

    if version < 2 {
        migrate_v2(conn)?;
    }

    Ok(())
}

fn get_schema_version(conn: &Connection) -> i32 {
    conn.query_row(
        "SELECT value FROM preferences WHERE key = 'schema_version'",
        [],
        |row| {
            let s: String = row.get(0)?;
            Ok(s.parse::<i32>().unwrap_or(1))
        },
    )
    .unwrap_or(1)
}

fn migrate_v2(conn: &Connection) -> SqliteResult<()> {
    // Add sync columns to tasks
    conn.execute_batch(
        "
        ALTER TABLE tasks ADD COLUMN uuid TEXT;
        ALTER TABLE tasks ADD COLUMN deleted INTEGER NOT NULL DEFAULT 0;
        ALTER TABLE tasks ADD COLUMN deleted_at TEXT;
        ALTER TABLE tags ADD COLUMN uuid TEXT;
        ALTER TABLE tags ADD COLUMN deleted INTEGER NOT NULL DEFAULT 0;
        ALTER TABLE tags ADD COLUMN deleted_at TEXT;
        ",
    )?;

    // Backfill UUIDs for existing rows
    backfill_uuids(conn)?;

    // Now add UNIQUE constraint via index (can't ALTER TABLE ADD UNIQUE in SQLite)
    conn.execute_batch(
        "
        CREATE UNIQUE INDEX IF NOT EXISTS idx_tasks_uuid ON tasks(uuid);
        CREATE UNIQUE INDEX IF NOT EXISTS idx_tags_uuid ON tags(uuid);
        ",
    )?;

    set_preference(conn, "schema_version", "2")?;
    Ok(())
}

fn backfill_uuids(conn: &Connection) -> SqliteResult<()> {
    use uuid::Uuid;

    let task_ids: Vec<i64> = {
        let mut stmt = conn.prepare("SELECT id FROM tasks WHERE uuid IS NULL")?;
        stmt.query_map([], |row| row.get(0))?
            .collect::<SqliteResult<Vec<_>>>()?
    };
    for id in task_ids {
        conn.execute(
            "UPDATE tasks SET uuid = ?1 WHERE id = ?2",
            rusqlite::params![Uuid::new_v4().to_string(), id],
        )?;
    }

    let tag_ids: Vec<i64> = {
        let mut stmt = conn.prepare("SELECT id FROM tags WHERE uuid IS NULL")?;
        stmt.query_map([], |row| row.get(0))?
            .collect::<SqliteResult<Vec<_>>>()?
    };
    for id in tag_ids {
        conn.execute(
            "UPDATE tags SET uuid = ?1 WHERE id = ?2",
            rusqlite::params![Uuid::new_v4().to_string(), id],
        )?;
    }

    Ok(())
}
```

**Step 4: Add `uuid` dependency to `crates/rk-client/Cargo.toml`**

```toml
uuid = { version = "1", features = ["v4"] }
```

**Step 5: Run tests**

```bash
cargo test -p rk-client
```

Expected: ALL pass (existing tests + new test). The v2 migration runs automatically for all in-memory DBs.

**Step 6: Commit**

```bash
git commit -m "feat: add schema versioning and v2 migration (uuid, deleted columns)"
```

---

### Task 4: Update Model Types for Sync Fields

Add `uuid`, `deleted`, `deleted_at` fields to `Task` and `Tag` structs.

**Files:**
- Modify: `crates/rk-client/src/model.rs`
- Modify: `crates/rk-client/src/db.rs` (load functions)

**Step 1: Update model types**

In `model.rs`, add fields to `Task`:

```rust
pub struct Task {
    pub id: i64,
    pub uuid: String,
    pub title: String,
    pub description: String,
    pub priority: Priority,
    pub column: Column,
    pub due_date: Option<NaiveDate>,
    pub tags: Vec<String>,
    pub created_at: NaiveDateTime,
    pub updated_at: NaiveDateTime,
    pub deleted: bool,
    pub deleted_at: Option<NaiveDateTime>,
}
```

Add fields to `Tag`:

```rust
pub struct Tag {
    pub id: i64,
    pub uuid: String,
    pub name: String,
    pub deleted: bool,
    pub deleted_at: Option<NaiveDateTime>,
}
```

**Step 2: Update `load_tasks()` in `db.rs`**

Update the SELECT to include new columns and update the `Task` construction. The existing `load_tasks()` becomes the "active only" loader (adds `WHERE deleted = 0`):

```rust
pub fn load_tasks(conn: &Connection) -> SqliteResult<Vec<Task>> {
    load_tasks_filtered(conn, true)
}

pub fn load_all_tasks(conn: &Connection) -> SqliteResult<Vec<Task>> {
    load_tasks_filtered(conn, false)
}

fn load_tasks_filtered(conn: &Connection, active_only: bool) -> SqliteResult<Vec<Task>> {
    let query = if active_only {
        "SELECT id, uuid, title, description, priority, column_name, due_date, created_at, updated_at, deleted, deleted_at
         FROM tasks WHERE deleted = 0"
    } else {
        "SELECT id, uuid, title, description, priority, column_name, due_date, created_at, updated_at, deleted, deleted_at
         FROM tasks"
    };
    let mut stmt = conn.prepare(query)?;
    // ... parse rows including uuid (col 1), deleted (col 9), deleted_at (col 10)
}
```

**Step 3: Update `load_tags()` similarly**

```rust
pub fn load_tags(conn: &Connection) -> SqliteResult<Vec<Tag>> {
    load_tags_filtered(conn, true)
}

pub fn load_all_tags(conn: &Connection) -> SqliteResult<Vec<Tag>> {
    load_tags_filtered(conn, false)
}

fn load_tags_filtered(conn: &Connection, active_only: bool) -> SqliteResult<Vec<Tag>> {
    let query = if active_only {
        "SELECT id, uuid, name, deleted, deleted_at FROM tags WHERE deleted = 0 ORDER BY name"
    } else {
        "SELECT id, uuid, name, deleted, deleted_at FROM tags ORDER BY name"
    };
    // ...
}
```

**Step 4: Update `insert_task()` to generate UUID**

```rust
pub fn insert_task(
    conn: &Connection,
    title: &str,
    description: &str,
    priority: Priority,
    column: Column,
    due_date: Option<chrono::NaiveDate>,
) -> SqliteResult<i64> {
    let now = now_timestamp();
    let uuid = uuid::Uuid::new_v4().to_string();
    let due_date_str = due_date.map(|d| d.format("%Y-%m-%d").to_string());

    conn.execute(
        "INSERT INTO tasks (uuid, title, description, priority, column_name, due_date, created_at, updated_at)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
        rusqlite::params![uuid, title, description, priority.as_str(), column.as_str(), due_date_str, now, now],
    )?;
    Ok(conn.last_insert_rowid())
}
```

**Step 5: Update `insert_tag()` to generate UUID**

```rust
pub fn insert_tag(conn: &Connection, name: &str) -> SqliteResult<i64> {
    let uuid = uuid::Uuid::new_v4().to_string();
    conn.execute(
        "INSERT INTO tags (uuid, name) VALUES (?1, ?2)",
        rusqlite::params![uuid, name],
    )?;
    Ok(conn.last_insert_rowid())
}
```

**Step 6: Update the `tag_map` join query in `load_tasks_filtered`**

The tag join query needs to also filter `WHERE t.deleted = 0` when loading active tasks:

```rust
let tag_query = if active_only {
    "SELECT tt.task_id, t.name FROM tags t
     JOIN task_tags tt ON t.id = tt.tag_id
     WHERE t.deleted = 0
     ORDER BY t.name"
} else {
    "SELECT tt.task_id, t.name FROM tags t
     JOIN task_tags tt ON t.id = tt.tag_id
     ORDER BY t.name"
};
```

**Step 7: Fix all compilation errors**

Every place that constructs a `Task` or `Tag` (including tests) needs the new fields. For tests, add sensible defaults. The `undo.rs` `DeleteTask` variant needs updating to store `uuid`.

**Step 8: Run tests**

```bash
cargo test -p rk-client
cargo clippy -p rk-client -- -D warnings
```

**Step 9: Write new tests**

```rust
#[test]
fn test_load_tasks_excludes_deleted() {
    let conn = init_db_memory();
    let id = insert_task(&conn, "Active", "", Priority::Medium, Column::Todo, None).unwrap();
    let id2 = insert_task(&conn, "Deleted", "", Priority::Low, Column::Todo, None).unwrap();
    soft_delete_task(&conn, id2).unwrap();
    let tasks = load_tasks(&conn).unwrap();
    assert_eq!(tasks.len(), 1);
    assert_eq!(tasks[0].id, id);
}

#[test]
fn test_load_all_tasks_includes_deleted() {
    let conn = init_db_memory();
    insert_task(&conn, "Active", "", Priority::Medium, Column::Todo, None).unwrap();
    let id2 = insert_task(&conn, "Deleted", "", Priority::Low, Column::Todo, None).unwrap();
    soft_delete_task(&conn, id2).unwrap();
    let tasks = load_all_tasks(&conn).unwrap();
    assert_eq!(tasks.len(), 2);
}

#[test]
fn test_tasks_have_uuids() {
    let conn = init_db_memory();
    insert_task(&conn, "T", "", Priority::Medium, Column::Todo, None).unwrap();
    let tasks = load_tasks(&conn).unwrap();
    assert!(!tasks[0].uuid.is_empty());
    assert_eq!(tasks[0].uuid.len(), 36); // UUID v4 string length
}

#[test]
fn test_tags_have_uuids() {
    let conn = init_db_memory();
    insert_tag(&conn, "bug").unwrap();
    let tags = load_tags(&conn).unwrap();
    assert!(!tags[0].uuid.is_empty());
}
```

**Step 10: Run and verify**

```bash
cargo test -p rk-client
```

**Step 11: Commit**

```bash
git commit -m "feat: add uuid/deleted fields to Task and Tag, split load functions"
```

---

### Task 5: Convert Hard Deletes to Soft Deletes

Replace all `DELETE FROM` operations with `UPDATE SET deleted=1`.

**Files:**
- Modify: `crates/rk-client/src/db.rs`
- Modify: `crates/rk-client/src/app.rs` (undo delete, clear done, tag delete)
- Modify: `crates/rk-client/src/undo.rs` (DeleteTask stores task_id instead of re-inserting)

**Step 1: Add `soft_delete_task()` to `db.rs`**

```rust
pub fn soft_delete_task(conn: &Connection, task_id: i64) -> SqliteResult<()> {
    let now = now_timestamp();
    conn.execute(
        "UPDATE tasks SET deleted = 1, deleted_at = ?1, updated_at = ?1 WHERE id = ?2",
        rusqlite::params![now, task_id],
    )?;
    // Also remove from task_tags (since we don't display deleted tasks, this is fine)
    // Actually keep task_tags — needed for sync to propagate tag associations
    Ok(())
}

pub fn undelete_task(conn: &Connection, task_id: i64) -> SqliteResult<()> {
    let now = now_timestamp();
    conn.execute(
        "UPDATE tasks SET deleted = 0, deleted_at = NULL, updated_at = ?1 WHERE id = ?2",
        rusqlite::params![now, task_id],
    )?;
    Ok(())
}

pub fn soft_delete_tag(conn: &Connection, tag_id: i64) -> SqliteResult<()> {
    let now = now_timestamp();
    conn.execute(
        "UPDATE tags SET deleted = 1, deleted_at = ?1 WHERE id = ?2",
        rusqlite::params![now, tag_id],
    )?;
    // Remove task_tags associations (logical cascade)
    conn.execute(
        "DELETE FROM task_tags WHERE tag_id = ?1",
        rusqlite::params![tag_id],
    )?;
    Ok(())
}
```

**Step 2: Add local cleanup function for non-logged-in users**

```rust
pub fn cleanup_old_soft_deletes(conn: &Connection, days: i64) -> SqliteResult<()> {
    let cutoff = (chrono::Local::now() - chrono::Duration::days(days))
        .naive_local()
        .format("%Y-%m-%dT%H:%M:%S")
        .to_string();
    conn.execute(
        "DELETE FROM task_tags WHERE task_id IN (SELECT id FROM tasks WHERE deleted = 1 AND deleted_at < ?1)",
        rusqlite::params![cutoff],
    )?;
    conn.execute(
        "DELETE FROM tasks WHERE deleted = 1 AND deleted_at < ?1",
        rusqlite::params![cutoff],
    )?;
    conn.execute(
        "DELETE FROM tags WHERE deleted = 1 AND deleted_at < ?1",
        rusqlite::params![cutoff],
    )?;
    Ok(())
}
```

**Step 3: Update `UndoAction::DeleteTask` in `undo.rs`**

Change from storing all fields (for re-insertion) to storing just `task_id` (for undelete flip):

```rust
pub enum UndoAction {
    MoveTask { task_id: i64, from_column: Column },
    PriorityChange { task_id: i64, previous: Priority },
    DeleteTask { task_id: i64, title: String },
    EditTask { task_id: i64, prev_title: String, prev_description: String, prev_priority: Priority, prev_due_date: Option<chrono::NaiveDate> },
    DuplicateTask { new_id: i64 },
}
```

**Step 4: Update `confirm_delete()` in `app.rs`**

```rust
pub fn confirm_delete(&mut self) {
    if let Some(task_id) = self.current_task_id() {
        if let Some(task) = self.find_task(task_id).cloned() {
            self.undo_stack.push(UndoAction::DeleteTask {
                task_id,
                title: task.title.clone(),
            });
            let _ = db::soft_delete_task(&self.db, task_id);
            self.reload_tasks();
            self.clamp_cursor(task.column);
            self.set_flash(format!("Deleted '{}'", task.title));
        }
    }
    self.mode = AppMode::Board;
}
```

**Step 5: Update undo handler for DeleteTask**

```rust
UndoAction::DeleteTask { task_id, title } => {
    let _ = db::undelete_task(&self.db, task_id);
    self.reload_tasks();
    // Find which column the task is in now
    if let Some(task) = self.find_task(task_id) {
        self.focused_column = task.column;
        self.set_cursor_to_task(task_id, task.column);
    }
    self.set_flash(format!("Undone: delete '{}'", title));
}
```

**Step 6: Update `confirm_clear_done()` to use soft delete**

```rust
pub fn confirm_clear_done(&mut self) {
    let done_tasks: Vec<_> = self.tasks.iter()
        .filter(|t| t.column == Column::Done)
        .cloned()
        .collect();
    let count = done_tasks.len();
    for task in &done_tasks {
        let _ = db::soft_delete_task(&self.db, task.id);
    }
    self.reload_tasks();
    self.clamp_cursor(Column::Done);
    self.set_flash(format!("Cleared {} done task{}", count, if count == 1 { "" } else { "s" }));
    self.mode = AppMode::Board;
}
```

**Step 7: Update `tag_delete()` in `app.rs` to use soft delete**

```rust
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
```

**Step 8: Update `DuplicateTask` undo to use soft delete**

In `undo()`:
```rust
UndoAction::DuplicateTask { new_id } => {
    let _ = db::soft_delete_task(&self.db, new_id);
    self.reload_tasks();
    self.clamp_cursor(self.focused_column);
    self.set_flash("Undone: duplicate task".to_string());
}
```

**Step 9: Keep the old hard-delete `delete_task()` for `reset_db()` only**

`reset_db()` still does hard deletes (it wipes everything). Keep the existing `delete_task()` as `hard_delete_task()` for internal use:

```rust
pub fn hard_delete_task(conn: &Connection, task_id: i64) -> SqliteResult<()> {
    conn.execute("DELETE FROM task_tags WHERE task_id = ?1", rusqlite::params![task_id])?;
    conn.execute("DELETE FROM tasks WHERE id = ?1", rusqlite::params![task_id])?;
    Ok(())
}
```

**Step 10: Add local cleanup to startup**

In `app.rs` `App::new()`, run cleanup if not logged in:

```rust
// Clean up old soft deletes for non-syncing users (30 days)
if !Self::is_logged_in() {
    let _ = db::cleanup_old_soft_deletes(&conn, 30);
}
```

Note: `is_logged_in()` will be implemented in Task 8. For now, add a stub that returns `false`.

**Step 11: Write tests**

```rust
#[test]
fn test_soft_delete_task() {
    let conn = init_db_memory();
    let id = insert_task(&conn, "T", "", Priority::Medium, Column::Todo, None).unwrap();
    soft_delete_task(&conn, id).unwrap();
    assert!(load_tasks(&conn).unwrap().is_empty()); // filtered
    assert_eq!(load_all_tasks(&conn).unwrap().len(), 1); // unfiltered
}

#[test]
fn test_undelete_task() {
    let conn = init_db_memory();
    let id = insert_task(&conn, "T", "", Priority::Medium, Column::Todo, None).unwrap();
    soft_delete_task(&conn, id).unwrap();
    undelete_task(&conn, id).unwrap();
    assert_eq!(load_tasks(&conn).unwrap().len(), 1);
    assert!(!load_tasks(&conn).unwrap()[0].deleted);
}

#[test]
fn test_soft_delete_tag() {
    let conn = init_db_memory();
    let tag_id = insert_tag(&conn, "bug").unwrap();
    let task_id = insert_task(&conn, "T", "", Priority::Medium, Column::Todo, None).unwrap();
    set_task_tags(&conn, task_id, &[tag_id]).unwrap();
    soft_delete_tag(&conn, tag_id).unwrap();
    assert!(load_tags(&conn).unwrap().is_empty());
    // task_tags should be removed
    assert!(get_task_tag_ids(&conn, task_id).unwrap().is_empty());
}

#[test]
fn test_cleanup_old_soft_deletes() {
    let conn = init_db_memory();
    let id = insert_task(&conn, "T", "", Priority::Medium, Column::Todo, None).unwrap();
    // Manually set deleted_at to 31 days ago
    let old_date = (chrono::Local::now() - chrono::Duration::days(31))
        .naive_local().format("%Y-%m-%dT%H:%M:%S").to_string();
    conn.execute(
        "UPDATE tasks SET deleted = 1, deleted_at = ?1 WHERE id = ?2",
        rusqlite::params![old_date, id],
    ).unwrap();
    cleanup_old_soft_deletes(&conn, 30).unwrap();
    assert!(load_all_tasks(&conn).unwrap().is_empty()); // hard deleted
}
```

**Step 12: Run all tests**

```bash
cargo test -p rk-client
cargo clippy -p rk-client -- -D warnings
```

**Step 13: Commit**

```bash
git commit -m "feat: convert hard deletes to soft deletes, add undelete and cleanup"
```

---

### Task 6: Update Export/Import for UUIDs (Version 2)

**Files:**
- Modify: `crates/rk-client/src/export.rs`
- Modify: `crates/rk-client/src/db.rs` (add `insert_task_with_uuid`)

**Step 1: Update export structs**

```rust
#[derive(Serialize, Deserialize)]
struct ExportData {
    version: u32,
    tasks: Vec<ExportTask>,
    tags: Vec<ExportTag>,
}

#[derive(Serialize, Deserialize)]
struct ExportTask {
    #[serde(default)]
    uuid: Option<String>,
    title: String,
    description: String,
    priority: String,
    column: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    due_date: Option<String>,
    tags: Vec<String>,
}

#[derive(Serialize, Deserialize)]
#[serde(untagged)]
enum ExportTagEntry {
    Named { uuid: Option<String>, name: String },
    Plain(String),
}
```

**Step 2: Update `export_json()` to version 2 with UUIDs**

```rust
pub fn export_json(conn: &rusqlite::Connection) -> Result<String, Box<dyn std::error::Error>> {
    let tasks = db::load_tasks(conn)?;
    let tags = db::load_tags(conn)?;

    let data = ExportData {
        version: 2,
        tasks: tasks.iter().map(|t| ExportTask {
            uuid: Some(t.uuid.clone()),
            title: t.title.clone(),
            description: t.description.clone(),
            priority: t.priority.as_str().to_string(),
            column: t.column.as_str().to_string(),
            due_date: t.due_date.map(|d| d.format("%Y-%m-%d").to_string()),
            tags: t.tags.clone(),
        }).collect(),
        tags: tags.iter().map(|t| ExportTag {
            uuid: Some(t.uuid.clone()),
            name: t.name.clone(),
        }).collect(),
    };

    Ok(serde_json::to_string_pretty(&data)?)
}
```

**Step 3: Update `import_json()` for backward compatibility**

- If a task has a `uuid` and that UUID already exists locally → skip it
- If a task has a `uuid` that's new → insert with that UUID
- If no `uuid` field (v1 import) → generate new UUID

Add `insert_task_with_uuid()` to `db.rs`:

```rust
pub fn insert_task_with_uuid(
    conn: &Connection,
    uuid: &str,
    title: &str,
    description: &str,
    priority: Priority,
    column: Column,
    due_date: Option<chrono::NaiveDate>,
) -> SqliteResult<i64> {
    let now = now_timestamp();
    let due_date_str = due_date.map(|d| d.format("%Y-%m-%d").to_string());
    conn.execute(
        "INSERT INTO tasks (uuid, title, description, priority, column_name, due_date, created_at, updated_at)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
        rusqlite::params![uuid, title, description, priority.as_str(), column.as_str(), due_date_str, now, now],
    )?;
    Ok(conn.last_insert_rowid())
}

pub fn insert_tag_with_uuid(conn: &Connection, uuid: &str, name: &str) -> SqliteResult<i64> {
    conn.execute(
        "INSERT INTO tags (uuid, name) VALUES (?1, ?2)",
        rusqlite::params![uuid, name],
    )?;
    Ok(conn.last_insert_rowid())
}

pub fn task_uuid_exists(conn: &Connection, uuid: &str) -> SqliteResult<bool> {
    let count: i64 = conn.query_row(
        "SELECT COUNT(*) FROM tasks WHERE uuid = ?1",
        rusqlite::params![uuid],
        |row| row.get(0),
    )?;
    Ok(count > 0)
}

pub fn tag_uuid_exists(conn: &Connection, uuid: &str) -> SqliteResult<bool> {
    let count: i64 = conn.query_row(
        "SELECT COUNT(*) FROM tags WHERE uuid = ?1",
        rusqlite::params![uuid],
        |row| row.get(0),
    )?;
    Ok(count > 0)
}
```

**Step 4: Write tests**

```rust
#[test]
fn test_export_v2_has_uuids() {
    let conn = crate::db::init_db_memory();
    crate::db::insert_task(&conn, "T", "", Priority::Medium, Column::Todo, None).unwrap();
    let json = export_json(&conn).unwrap();
    assert!(json.contains("\"version\": 2"));
    assert!(json.contains("\"uuid\""));
}

#[test]
fn test_import_v1_generates_uuids() {
    let conn = crate::db::init_db_memory();
    let json = r#"{"version":1,"tasks":[{"title":"T","description":"","priority":"Medium","column":"todo","tags":[]}],"tags":[]}"#;
    import_json(&conn, json).unwrap();
    let tasks = crate::db::load_tasks(&conn).unwrap();
    assert!(!tasks[0].uuid.is_empty());
}

#[test]
fn test_import_v2_skips_existing_uuid() {
    let conn = crate::db::init_db_memory();
    let id = crate::db::insert_task(&conn, "Original", "", Priority::Medium, Column::Todo, None).unwrap();
    let tasks = crate::db::load_tasks(&conn).unwrap();
    let uuid = tasks[0].uuid.clone();

    let json = format!(r#"{{"version":2,"tasks":[{{"uuid":"{}","title":"Duplicate","description":"","priority":"Medium","column":"todo","tags":[]}}],"tags":[]}}"#, uuid);
    let count = import_json(&conn, &json).unwrap();
    assert_eq!(count, 0); // skipped
    let tasks = crate::db::load_tasks(&conn).unwrap();
    assert_eq!(tasks.len(), 1);
    assert_eq!(tasks[0].title, "Original"); // not overwritten
}
```

**Step 5: Run tests**

```bash
cargo test -p rk-client
cargo clippy -p rk-client -- -D warnings
```

**Step 6: Commit**

```bash
git commit -m "feat: update export/import to v2 with UUIDs, backward-compatible v1 import"
```

---

### Task 7: Text Length Validation

Add character limits to modal text fields and tag names, enforced in the TUI.

**Files:**
- Modify: `crates/rk-client/src/app.rs` (modal_insert_char, tag_edit_insert_char)

**Step 1: Add constants**

In `app.rs`:

```rust
const MAX_TITLE_LEN: usize = 500;
const MAX_DESCRIPTION_LEN: usize = 5000;
const MAX_TAG_NAME_LEN: usize = 50;
const MAX_DEVICE_NAME_LEN: usize = 100;
```

**Step 2: Add length checks in `modal_insert_char()`**

In `app.rs`, inside `modal_insert_char()`, before inserting into the title or description field, check length:

```rust
ModalField::Title => {
    if self.modal.title.len() < MAX_TITLE_LEN {
        self.modal.title.insert(self.modal.cursor_pos, c);
        self.modal.cursor_pos += c.len_utf8();
    }
}
ModalField::Description => {
    if self.modal.description.len() < MAX_DESCRIPTION_LEN {
        self.modal.description.insert(self.modal.cursor_pos, c);
        self.modal.cursor_pos += c.len_utf8();
    }
}
```

**Step 3: Add length check in `tag_edit_insert_char()`**

```rust
pub fn tag_edit_insert_char(&mut self, c: char) {
    if self.tag_edit_name.len() < MAX_TAG_NAME_LEN {
        self.tag_edit_name.push(c);
    }
}
```

**Step 4: Verify**

```bash
cargo build -p rk-client
cargo clippy -p rk-client -- -D warnings
```

**Step 5: Commit**

```bash
git commit -m "feat: add text length validation to modal and tag inputs"
```

---

## Phase 4: Client Auth Module

### Task 8: Credentials File Management

**Files:**
- Create: `crates/rk-client/src/auth.rs`
- Modify: `crates/rk-client/src/main.rs` (add `mod auth`)

**Step 1: Write `auth.rs`**

```rust
use std::fs;
use std::path::PathBuf;

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Credentials {
    pub token: String,
    pub device_id: String,
    pub device_name: String,
    pub server_url: String,
    pub last_synced_at: Option<String>,
}

pub fn credentials_path() -> PathBuf {
    let config_dir = dirs::config_dir().expect("Could not determine config directory");
    config_dir.join("rustkanban").join("credentials.json")
}

pub fn load_credentials() -> Option<Credentials> {
    let path = credentials_path();
    let data = fs::read_to_string(&path).ok()?;
    serde_json::from_str(&data).ok()
}

pub fn save_credentials(creds: &Credentials) -> Result<(), Box<dyn std::error::Error>> {
    let path = credentials_path();
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    let json = serde_json::to_string_pretty(creds)?;
    fs::write(&path, &json)?;

    // Set file permissions to 0600 (Unix only)
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        fs::set_permissions(&path, fs::Permissions::from_mode(0o600))?;
    }

    Ok(())
}

pub fn delete_credentials() -> Result<(), Box<dyn std::error::Error>> {
    let path = credentials_path();
    if path.exists() {
        fs::remove_file(&path)?;
    }
    Ok(())
}

pub fn is_logged_in() -> bool {
    load_credentials().is_some()
}

pub fn update_last_synced(synced_at: &str) -> Result<(), Box<dyn std::error::Error>> {
    if let Some(mut creds) = load_credentials() {
        creds.last_synced_at = Some(synced_at.to_string());
        save_credentials(&creds)?;
    }
    Ok(())
}
```

**Step 2: Write tests**

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use std::env;

    fn with_temp_home<F: FnOnce()>(f: F) {
        let dir = tempfile::tempdir().unwrap();
        env::set_var("XDG_CONFIG_HOME", dir.path());
        f();
    }

    #[test]
    fn test_save_and_load_credentials() {
        with_temp_home(|| {
            let creds = Credentials {
                token: "rk_test123".into(),
                device_id: "device-uuid".into(),
                device_name: "test-machine".into(),
                server_url: "https://sync.example.com".into(),
                last_synced_at: None,
            };
            save_credentials(&creds).unwrap();
            let loaded = load_credentials().unwrap();
            assert_eq!(loaded.token, "rk_test123");
            assert_eq!(loaded.device_name, "test-machine");
        });
    }

    #[test]
    fn test_delete_credentials() {
        with_temp_home(|| {
            let creds = Credentials {
                token: "rk_test".into(),
                device_id: "d".into(),
                device_name: "m".into(),
                server_url: "https://example.com".into(),
                last_synced_at: None,
            };
            save_credentials(&creds).unwrap();
            assert!(is_logged_in());
            delete_credentials().unwrap();
            assert!(!is_logged_in());
        });
    }
}
```

Note: Add `tempfile = "3"` as a dev-dependency in `crates/rk-client/Cargo.toml`.

**Step 3: Update `app.rs` to use `auth::is_logged_in()`**

Replace the stub `is_logged_in()` from Task 5 with:

```rust
use crate::auth;

// In App::new():
if !auth::is_logged_in() {
    let _ = db::cleanup_old_soft_deletes(&conn, 30);
}
```

**Step 4: Run tests**

```bash
cargo test -p rk-client
```

**Step 5: Commit**

```bash
git commit -m "feat: add auth module with credential file management"
```

---

### Task 9: Login Flow + CLI Subcommands

**Files:**
- Modify: `crates/rk-client/src/auth.rs` (add `login()`)
- Modify: `crates/rk-client/src/main.rs` (add Login/Logout/Sync/Status subcommands)
- Add dependency: `ureq`, `open` in `crates/rk-client/Cargo.toml`

**Step 1: Add dependencies**

In `crates/rk-client/Cargo.toml`:

```toml
ureq = "3"
open = "5"
```

**Step 2: Implement `login()` in `auth.rs`**

```rust
use std::io::{BufRead, BufReader, Write};
use std::net::TcpListener;

const DEFAULT_SERVER: &str = "https://sync.rustkanban.com";
const LOGIN_TIMEOUT_SECS: u64 = 300; // 5 minutes

pub fn login(server_url: Option<&str>, device_name: Option<&str>) -> Result<Credentials, Box<dyn std::error::Error>> {
    if is_logged_in() {
        let creds = load_credentials().unwrap();
        return Err(format!(
            "Already logged in as device '{}'. Run `rk logout` first to switch accounts or re-authenticate.",
            creds.device_name
        ).into());
    }

    let server = server_url.unwrap_or(DEFAULT_SERVER);
    let hostname = device_name
        .map(String::from)
        .unwrap_or_else(|| gethostname().unwrap_or_else(|| "unknown".into()));

    // Start local callback server
    let listener = TcpListener::bind("127.0.0.1:0")?;
    let port = listener.local_addr()?.port();
    listener.set_nonblocking(false)?;

    let login_url = format!(
        "{}/login?redirect_port={}&device_name={}",
        server,
        port,
        urlencoding::encode(&hostname)
    );

    // Try to open browser
    let browser_opened = open::that(&login_url).is_ok();
    if !browser_opened {
        // Headless fallback
        let headless_url = format!(
            "{}/login?device_name={}&mode=headless",
            server,
            urlencoding::encode(&hostname)
        );
        println!("Open this URL in any browser:");
        println!("  {}", headless_url);
        println!("\nThen paste the token here:");

        let mut token_input = String::new();
        std::io::stdin().read_line(&mut token_input)?;
        let token = token_input.trim().to_string();

        println!("Enter device ID:");
        let mut device_input = String::new();
        std::io::stdin().read_line(&mut device_input)?;
        let device_id = device_input.trim().to_string();

        let creds = Credentials {
            token,
            device_id,
            device_name: hostname,
            server_url: server.to_string(),
            last_synced_at: None,
        };
        save_credentials(&creds)?;
        return Ok(creds);
    }

    println!("Waiting for authentication... (Ctrl+C to cancel)");

    // Wait for callback with timeout
    listener.set_nonblocking(false)?;
    // Use SO_RCVTIMEO for timeout
    let stream = listener.accept();
    // Parse the callback to extract token and device_id
    // ... (HTTP parsing of GET /callback?token=...&device_id=...)

    // Save credentials and return
    todo!("Parse callback, save credentials")
}

fn gethostname() -> Option<String> {
    hostname::get().ok()?.into_string().ok()
}
```

Note: Add `hostname = "0.4"` and `urlencoding = "2"` to dependencies.

The full HTTP callback parsing implementation will handle:
1. Accept TCP connection
2. Read HTTP GET line
3. Parse query params for `token` and `device_id`
4. Send "200 OK" response with "You can close this tab" HTML
5. Close connection

**Step 3: Add CLI subcommands in `main.rs`**

```rust
#[derive(Subcommand)]
enum Commands {
    Reset,
    Completions { shell: clap_complete::Shell },
    Export,
    Import { file: std::path::PathBuf },
    Manpage,
    Theme { #[arg(long)] init: bool },
    /// Log in to sync service
    Login {
        /// Custom server URL
        #[arg(long)]
        server: Option<String>,
        /// Override device name (default: hostname)
        #[arg(long)]
        device_name: Option<String>,
        /// Manually provide token (skip browser)
        #[arg(long)]
        token: Option<String>,
        /// Manually provide device ID (with --token)
        #[arg(long)]
        device_id: Option<String>,
    },
    /// Log out from sync service
    Logout,
    /// Sync with server (pull + push)
    Sync,
    /// Show sync status
    Status,
}
```

**Step 4: Implement subcommand handlers in `main.rs`**

```rust
Some(Commands::Login { server, device_name, token, device_id }) => {
    if let (Some(token), Some(device_id)) = (token, device_id) {
        // Manual credential entry
        let creds = auth::Credentials {
            token,
            device_id,
            device_name: device_name.unwrap_or_else(|| hostname::get().map(|h| h.to_string_lossy().to_string()).unwrap_or("unknown".into())),
            server_url: server.unwrap_or_else(|| "https://sync.rustkanban.com".into()),
            last_synced_at: None,
        };
        auth::save_credentials(&creds)?;
        println!("Logged in as device '{}'.", creds.device_name);
    } else {
        match auth::login(server.as_deref(), device_name.as_deref()) {
            Ok(creds) => println!("Logged in as device '{}'.", creds.device_name),
            Err(e) => eprintln!("{}", e),
        }
    }
}
Some(Commands::Logout) => {
    if !auth::is_logged_in() {
        println!("Not logged in.");
    } else {
        // TODO: attempt final push before logout
        auth::delete_credentials()?;
        println!("Logged out. Local data preserved.");
    }
}
Some(Commands::Status) => {
    if let Some(creds) = auth::load_credentials() {
        println!("Logged in as \"{}\"", creds.device_name);
        println!("Server:      {}", creds.server_url);
        match &creds.last_synced_at {
            Some(ts) => println!("Last synced: {}", ts),
            None => println!("Last synced: never"),
        }
    } else {
        println!("Not logged in. Run `rk login` to enable sync.");
    }
}
Some(Commands::Sync) => {
    if !auth::is_logged_in() {
        eprintln!("Not logged in. Run `rk login` first.");
    } else {
        // TODO: call sync::sync()
        println!("Sync not yet implemented.");
    }
}
```

**Step 5: Update `rk reset` to warn if logged in**

```rust
Some(Commands::Reset) => {
    if auth::is_logged_in() {
        println!("Warning: You are logged in to sync. This will only reset local data —");
        println!("synced tasks will reappear on next sync. To also delete server data,");
        println!("use your account page.");
    }
    print!("Delete all tasks and tags? (Y/N) ");
    // ... existing logic
}
```

**Step 6: Verify**

```bash
cargo build -p rk-client
cargo clippy -p rk-client -- -D warnings
```

**Step 7: Commit**

```bash
git commit -m "feat: add login/logout/sync/status CLI subcommands and auth flow"
```

---

## Phase 5: Client Sync Module

### Task 10: SyncClient — Core Sync Logic

**Files:**
- Create: `crates/rk-client/src/sync.rs`
- Modify: `crates/rk-client/src/main.rs` (add `mod sync`)
- Add dependency: `rk-shared` in `crates/rk-client/Cargo.toml`

**Step 1: Add rk-shared dependency**

In `crates/rk-client/Cargo.toml`:
```toml
rk-shared = { path = "../rk-shared" }
```

**Step 2: Write `sync.rs`**

```rust
use rusqlite::Connection;
use rk_shared::{SyncPayload, SyncResponse, SyncTag, SyncTask};

use crate::auth::{self, Credentials};
use crate::db;

#[derive(Debug)]
pub enum SyncError {
    NotLoggedIn,
    Network(String),
    AuthExpired,
    AccountNotFound,
    ServerError(String),
    Other(String),
}

impl std::fmt::Display for SyncError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SyncError::NotLoggedIn => write!(f, "Not logged in"),
            SyncError::Network(e) => write!(f, "Sync failed — working offline ({})", e),
            SyncError::AuthExpired => write!(f, "Session expired — run `rk login` to re-authenticate"),
            SyncError::AccountNotFound => write!(f, "Account not found — run `rk logout` to clear credentials"),
            SyncError::ServerError(e) => write!(f, "Server error — try again later ({})", e),
            SyncError::Other(e) => write!(f, "Sync error: {}", e),
        }
    }
}

pub fn pull(conn: &Connection) -> Result<String, SyncError> {
    let creds = auth::load_credentials().ok_or(SyncError::NotLoggedIn)?;

    let payload = SyncPayload {
        tasks: vec![],
        tags: vec![],
        last_synced_at: creds.last_synced_at.clone(),
    };

    let response = post_sync(&creds, "/api/v1/sync/pull", &payload)?;
    apply_pull_response(conn, &response)?;

    let _ = auth::update_last_synced(&response.synced_at);
    Ok(response.synced_at)
}

pub fn push(conn: &Connection) -> Result<String, SyncError> {
    let creds = auth::load_credentials().ok_or(SyncError::NotLoggedIn)?;

    let payload = build_push_payload(conn, &creds)?;
    let response = post_sync(&creds, "/api/v1/sync/push", &payload)?;

    // Apply any server-side corrections (LWW rejections, tag remappings)
    apply_pull_response(conn, &response)?;

    let _ = auth::update_last_synced(&response.synced_at);
    Ok(response.synced_at)
}

pub fn sync(conn: &Connection) -> Result<String, SyncError> {
    let creds = auth::load_credentials().ok_or(SyncError::NotLoggedIn)?;

    let payload = build_push_payload(conn, &creds)?;
    let response = post_sync(&creds, "/api/v1/sync", &payload)?;

    apply_pull_response(conn, &response)?;
    apply_tag_uuid_mappings(conn, &response.tag_uuid_mappings)?;

    let _ = auth::update_last_synced(&response.synced_at);
    Ok(response.synced_at)
}

fn build_push_payload(conn: &Connection, creds: &Credentials) -> Result<SyncPayload, SyncError> {
    let all_tasks = db::load_all_tasks(conn).map_err(|e| SyncError::Other(e.to_string()))?;
    let all_tags = db::load_all_tags(conn).map_err(|e| SyncError::Other(e.to_string()))?;

    let tasks: Vec<SyncTask> = if creds.last_synced_at.is_some() {
        // Delta: only changed since last sync
        let last = creds.last_synced_at.as_deref().unwrap();
        all_tasks.iter()
            .filter(|t| t.updated_at.format("%Y-%m-%dT%H:%M:%S").to_string().as_str() > last)
            .map(|t| task_to_sync(t, conn))
            .collect()
    } else {
        // Full sync: send everything
        all_tasks.iter().map(|t| task_to_sync(t, conn)).collect()
    };

    let tags: Vec<SyncTag> = if creds.last_synced_at.is_some() {
        let last = creds.last_synced_at.as_deref().unwrap();
        all_tags.iter()
            .filter(|t| {
                t.deleted_at
                    .map(|d| d.format("%Y-%m-%dT%H:%M:%S").to_string().as_str() > last)
                    .unwrap_or(false)
                // Tags don't have updated_at in current schema — we'll need to add this
                // For now, send all tags on every push
            })
            .map(tag_to_sync)
            .collect()
    } else {
        all_tags.iter().map(tag_to_sync).collect()
    };

    Ok(SyncPayload {
        tasks,
        tags,
        last_synced_at: creds.last_synced_at.clone(),
    })
}

fn task_to_sync(task: &crate::model::Task, conn: &Connection) -> SyncTask {
    let tag_uuids = db::get_task_tag_uuids(conn, task.id).unwrap_or_default();
    SyncTask {
        uuid: task.uuid.clone(),
        title: task.title.clone(),
        description: task.description.clone(),
        priority: task.priority.as_str().to_string(),
        column: task.column.as_str().to_string(),
        due_date: task.due_date.map(|d| d.format("%Y-%m-%d").to_string()),
        tags: tag_uuids,
        created_at: task.created_at.format("%Y-%m-%dT%H:%M:%S").to_string(),
        updated_at: task.updated_at.format("%Y-%m-%dT%H:%M:%S").to_string(),
        deleted: task.deleted,
    }
}

fn tag_to_sync(tag: &crate::model::Tag) -> SyncTag {
    SyncTag {
        uuid: tag.uuid.clone(),
        name: tag.name.clone(),
        updated_at: tag.deleted_at
            .map(|d| d.format("%Y-%m-%dT%H:%M:%S").to_string())
            .unwrap_or_default(),
        deleted: tag.deleted,
    }
}

fn post_sync(creds: &Credentials, path: &str, payload: &SyncPayload) -> Result<SyncResponse, SyncError> {
    let url = format!("{}{}", creds.server_url, path);
    let body = serde_json::to_string(payload).map_err(|e| SyncError::Other(e.to_string()))?;

    let response = ureq::post(&url)
        .header("Authorization", &format!("Bearer {}", creds.token))
        .header("Content-Type", "application/json")
        .send_bytes(body.as_bytes());

    match response {
        Ok(resp) => {
            let text = resp.body_mut().read_to_string()
                .map_err(|e| SyncError::Other(e.to_string()))?;
            serde_json::from_str(&text).map_err(|e| SyncError::Other(e.to_string()))
        }
        Err(ureq::Error::StatusCode(401)) => Err(SyncError::AuthExpired),
        Err(ureq::Error::StatusCode(403)) => Err(SyncError::AccountNotFound),
        Err(ureq::Error::StatusCode(code)) if code >= 500 => {
            Err(SyncError::ServerError(format!("HTTP {}", code)))
        }
        Err(e) => Err(SyncError::Network(e.to_string())),
    }
}

fn apply_pull_response(conn: &Connection, response: &SyncResponse) -> Result<(), SyncError> {
    let map_err = |e: rusqlite::Error| SyncError::Other(e.to_string());

    // Tags first (dependency order)
    for tag in &response.tags {
        db::upsert_tag_from_sync(conn, tag).map_err(map_err)?;
    }

    // Then tasks
    for task in &response.tasks {
        db::upsert_task_from_sync(conn, task).map_err(map_err)?;
    }

    Ok(())
}

fn apply_tag_uuid_mappings(
    conn: &Connection,
    mappings: &std::collections::HashMap<String, String>,
) -> Result<(), SyncError> {
    let map_err = |e: rusqlite::Error| SyncError::Other(e.to_string());
    for (old_uuid, new_uuid) in mappings {
        db::remap_tag_uuid(conn, old_uuid, new_uuid).map_err(map_err)?;
    }
    Ok(())
}
```

**Step 3: Add sync-related DB helpers**

In `db.rs`, add functions needed by sync:

```rust
pub fn get_task_tag_uuids(conn: &Connection, task_id: i64) -> SqliteResult<Vec<String>> {
    let mut stmt = conn.prepare(
        "SELECT t.uuid FROM tags t JOIN task_tags tt ON t.id = tt.tag_id WHERE tt.task_id = ?1"
    )?;
    stmt.query_map(rusqlite::params![task_id], |row| row.get(0))?
        .collect::<SqliteResult<Vec<_>>>()
}

pub fn upsert_task_from_sync(conn: &Connection, task: &rk_shared::SyncTask) -> SqliteResult<()> {
    // Check if task exists locally
    let existing: Option<(i64, String)> = conn.query_row(
        "SELECT id, updated_at FROM tasks WHERE uuid = ?1",
        rusqlite::params![task.uuid],
        |row| Ok((row.get(0)?, row.get(1)?)),
    ).ok();

    match existing {
        Some((local_id, local_updated)) => {
            // LWW: server wins if newer
            if task.updated_at > local_updated {
                let due_date = task.due_date.as_deref();
                conn.execute(
                    "UPDATE tasks SET title=?1, description=?2, priority=?3, column_name=?4, due_date=?5, updated_at=?6, deleted=?7, deleted_at=?8 WHERE uuid=?9",
                    rusqlite::params![
                        task.title, task.description, task.priority, task.column,
                        due_date, task.updated_at,
                        task.deleted as i32,
                        if task.deleted { Some(&task.updated_at) } else { None },
                        task.uuid
                    ],
                )?;
                // Update task_tags
                update_task_tags_by_uuid(conn, local_id, &task.tags)?;
            }
        }
        None => {
            // New task from server
            let due_date = task.due_date.as_deref();
            conn.execute(
                "INSERT INTO tasks (uuid, title, description, priority, column_name, due_date, created_at, updated_at, deleted, deleted_at)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)",
                rusqlite::params![
                    task.uuid, task.title, task.description, task.priority, task.column,
                    due_date, task.created_at, task.updated_at,
                    task.deleted as i32,
                    if task.deleted { Some(&task.updated_at) } else { None }
                ],
            )?;
            let local_id = conn.query_row(
                "SELECT id FROM tasks WHERE uuid = ?1",
                rusqlite::params![task.uuid],
                |row| row.get::<_, i64>(0),
            )?;
            update_task_tags_by_uuid(conn, local_id, &task.tags)?;
        }
    }
    Ok(())
}

fn update_task_tags_by_uuid(conn: &Connection, task_id: i64, tag_uuids: &[String]) -> SqliteResult<()> {
    conn.execute("DELETE FROM task_tags WHERE task_id = ?1", rusqlite::params![task_id])?;
    for tag_uuid in tag_uuids {
        if let Ok(tag_id) = conn.query_row(
            "SELECT id FROM tags WHERE uuid = ?1",
            rusqlite::params![tag_uuid],
            |row| row.get::<_, i64>(0),
        ) {
            conn.execute(
                "INSERT OR IGNORE INTO task_tags (task_id, tag_id) VALUES (?1, ?2)",
                rusqlite::params![task_id, tag_id],
            )?;
        }
    }
    Ok(())
}

pub fn upsert_tag_from_sync(conn: &Connection, tag: &rk_shared::SyncTag) -> SqliteResult<()> {
    let existing: Option<(i64, String)> = conn.query_row(
        "SELECT id, COALESCE(deleted_at, '') FROM tags WHERE uuid = ?1",
        rusqlite::params![tag.uuid],
        |row| Ok((row.get(0)?, row.get(1)?)),
    ).ok();

    match existing {
        Some((_local_id, _local_updated)) => {
            // LWW: always accept server version for tags (server is authoritative after dedup)
            conn.execute(
                "UPDATE tags SET name=?1, deleted=?2, deleted_at=?3 WHERE uuid=?4",
                rusqlite::params![
                    tag.name,
                    tag.deleted as i32,
                    if tag.deleted { Some(&tag.updated_at) } else { None::<&str> },
                    tag.uuid
                ],
            )?;
        }
        None => {
            conn.execute(
                "INSERT INTO tags (uuid, name, deleted, deleted_at) VALUES (?1, ?2, ?3, ?4)",
                rusqlite::params![
                    tag.uuid, tag.name,
                    tag.deleted as i32,
                    if tag.deleted { Some(&tag.updated_at) } else { None::<&str> }
                ],
            )?;
        }
    }
    Ok(())
}

pub fn remap_tag_uuid(conn: &Connection, old_uuid: &str, new_uuid: &str) -> SqliteResult<()> {
    conn.execute(
        "UPDATE tags SET uuid = ?1 WHERE uuid = ?2",
        rusqlite::params![new_uuid, old_uuid],
    )?;
    Ok(())
}
```

**Step 4: Write tests for sync DB operations**

```rust
#[test]
fn test_upsert_task_from_sync_insert() {
    let conn = init_db_memory();
    let task = rk_shared::SyncTask {
        uuid: "remote-uuid-1".into(),
        title: "Remote Task".into(),
        description: "".into(),
        priority: "High".into(),
        column: "todo".into(),
        due_date: None,
        tags: vec![],
        created_at: "2026-01-01T00:00:00".into(),
        updated_at: "2026-01-01T00:00:00".into(),
        deleted: false,
    };
    upsert_task_from_sync(&conn, &task).unwrap();
    let tasks = load_tasks(&conn).unwrap();
    assert_eq!(tasks.len(), 1);
    assert_eq!(tasks[0].uuid, "remote-uuid-1");
    assert_eq!(tasks[0].title, "Remote Task");
}

#[test]
fn test_upsert_task_lww_server_wins() {
    let conn = init_db_memory();
    insert_task(&conn, "Local", "", Priority::Medium, Column::Todo, None).unwrap();
    let tasks = load_tasks(&conn).unwrap();
    let uuid = tasks[0].uuid.clone();

    let remote = rk_shared::SyncTask {
        uuid,
        title: "Server Version".into(),
        description: "".into(),
        priority: "High".into(),
        column: "done".into(),
        due_date: None,
        tags: vec![],
        created_at: "2026-01-01T00:00:00".into(),
        updated_at: "2099-01-01T00:00:00".into(), // far future = wins
        deleted: false,
    };
    upsert_task_from_sync(&conn, &remote).unwrap();
    let tasks = load_tasks(&conn).unwrap();
    assert_eq!(tasks[0].title, "Server Version");
}

#[test]
fn test_upsert_task_lww_local_wins() {
    let conn = init_db_memory();
    insert_task(&conn, "Local", "", Priority::Medium, Column::Todo, None).unwrap();
    let tasks = load_tasks(&conn).unwrap();
    let uuid = tasks[0].uuid.clone();

    let remote = rk_shared::SyncTask {
        uuid,
        title: "Old Server".into(),
        description: "".into(),
        priority: "Low".into(),
        column: "todo".into(),
        due_date: None,
        tags: vec![],
        created_at: "2020-01-01T00:00:00".into(),
        updated_at: "2020-01-01T00:00:00".into(), // old = loses
        deleted: false,
    };
    upsert_task_from_sync(&conn, &remote).unwrap();
    let tasks = load_tasks(&conn).unwrap();
    assert_eq!(tasks[0].title, "Local"); // unchanged
}
```

**Step 5: Run tests**

```bash
cargo test -p rk-client
cargo clippy -p rk-client -- -D warnings
```

**Step 6: Commit**

```bash
git commit -m "feat: add sync module with pull/push logic and DB upsert operations"
```

---

### Task 11: TUI Sync Integration

**Files:**
- Modify: `crates/rk-client/src/main.rs` (startup pull, quit push)
- Modify: `crates/rk-client/src/handler.rs` (Ctrl+R keybinding)
- Modify: `crates/rk-client/src/app.rs` (sync state, status bar, undo clear)
- Modify: `crates/rk-client/src/ui/mod.rs` (status bar sync indicator)

**Step 1: Add sync state to App**

In `app.rs`, add fields:

```rust
pub struct App {
    // ... existing fields ...
    pub sync_status: SyncStatus,
}

pub enum SyncStatus {
    NotLoggedIn,
    Idle { last_synced: Option<String> },
    Syncing,
    Error(String),
}
```

Initialize in `App::new()`:

```rust
sync_status: if auth::is_logged_in() {
    let creds = auth::load_credentials().unwrap();
    SyncStatus::Idle { last_synced: creds.last_synced_at }
} else {
    SyncStatus::NotLoggedIn
},
```

**Step 2: Add `do_sync()` method to App**

```rust
pub fn do_sync(&mut self) {
    if !auth::is_logged_in() {
        return;
    }
    self.sync_status = SyncStatus::Syncing;
    match sync::sync(&self.db) {
        Ok(synced_at) => {
            self.reload_tasks();
            self.reload_tags();
            self.undo_stack = UndoStack::new(); // clear undo after sync
            self.sync_status = SyncStatus::Idle { last_synced: Some(synced_at) };
            self.set_flash("Synced successfully".to_string());
        }
        Err(e) => {
            self.sync_status = SyncStatus::Error(e.to_string());
            self.set_flash(e.to_string());
        }
    }
}
```

**Step 3: Add Ctrl+R handler**

In `handler.rs`, in `handle_board()`, add before the existing match:

```rust
if key.modifiers.contains(KeyModifiers::CONTROL) && key.code == KeyCode::Char('r') {
    app.do_sync();
    return;
}
```

**Step 4: Add startup pull and quit push in `main.rs`**

In `run_tui()`, before the event loop:

```rust
// Sync on startup
if auth::is_logged_in() {
    print!("Syncing...");
    io::stdout().flush().ok();
    match sync::pull(&app.db) {
        Ok(_) => {
            app.reload_tasks();
            app.reload_tags();
            println!(" done.");
        }
        Err(e) => println!(" {}", e),
    }
}
```

After the event loop (after `restore_terminal()`):

```rust
// Sync on quit
if auth::is_logged_in() {
    match sync::push(&app.db) {
        Ok(_) => {}
        Err(sync::SyncError::Network(_)) => {
            eprintln!("Changes saved locally, will sync next time.");
        }
        Err(_) => {} // silently skip auth errors on quit
    }
}
```

**Step 5: Update status bar to show sync state**

In `ui/mod.rs`, in `render_status_bar()`, add sync indicator:

```rust
match &app.sync_status {
    SyncStatus::NotLoggedIn => {} // show nothing
    SyncStatus::Idle { last_synced: Some(ts) } => {
        spans.push(Span::raw("  "));
        spans.push(Span::styled("Synced ", Style::default().fg(Color::Gray)));
        spans.push(Span::styled(format_time_ago(ts), Style::default().fg(Color::Green)));
    }
    SyncStatus::Idle { last_synced: None } => {
        spans.push(Span::raw("  "));
        spans.push(Span::styled("Not synced", Style::default().fg(Color::Gray)));
    }
    SyncStatus::Syncing => {
        spans.push(Span::raw("  "));
        spans.push(Span::styled("Syncing...", Style::default().fg(Color::Yellow)));
    }
    SyncStatus::Error(_) => {
        spans.push(Span::raw("  "));
        spans.push(Span::styled("Offline", Style::default().fg(Color::Red)));
    }
}
```

**Step 6: Verify**

```bash
cargo build -p rk-client
cargo clippy -p rk-client -- -D warnings
```

**Step 7: Commit**

```bash
git commit -m "feat: integrate sync into TUI (Ctrl+R, startup pull, quit push, status bar)"
```

---

## Phase 6: Server Foundation

### Task 12: Create `rk-server` Crate Skeleton

**Files:**
- Create: `crates/rk-server/Cargo.toml`
- Create: `crates/rk-server/src/main.rs`
- Create: `crates/rk-server/src/config.rs`
- Create: `crates/rk-server/src/error.rs`
- Modify: `Cargo.toml` (root — add member)

**Step 1: Create directory structure**

```bash
mkdir -p crates/rk-server/src
mkdir -p crates/rk-server/migrations
mkdir -p crates/rk-server/templates
```

**Step 2: Write `crates/rk-server/Cargo.toml`**

```toml
[package]
name = "rk-server"
version = "0.1.0"
edition = "2021"

[[bin]]
name = "rk-server"
path = "src/main.rs"

[dependencies]
rk-shared = { path = "../rk-shared" }
axum = "0.8"
tokio = { version = "1", features = ["full"] }
sqlx = { version = "0.8", features = ["runtime-tokio", "tls-rustls", "postgres", "uuid", "chrono"] }
tower = "0.5"
tower-http = { version = "0.6", features = ["cors", "limit"] }
oauth2 = "5"
sha2 = "0.10"
uuid = { version = "1", features = ["v4"] }
dotenvy = "0.15"
chrono = { version = "0.4", features = ["serde"] }
tracing = "0.1"
tracing-subscriber = { version = "0.3", features = ["env-filter"] }
serde = { version = "1", features = ["derive"] }
serde_json = "1"
askama = "0.12"
askama_axum = "0.4"
```

**Step 3: Write `config.rs`**

```rust
#[derive(Clone)]
pub struct Config {
    pub database_url: String,
    pub github_client_id: String,
    pub github_client_secret: String,
    pub server_url: String,
    pub session_secret: String,
    pub port: u16,
}

impl Config {
    pub fn from_env() -> Self {
        Config {
            database_url: std::env::var("DATABASE_URL").expect("DATABASE_URL required"),
            github_client_id: std::env::var("GITHUB_CLIENT_ID").expect("GITHUB_CLIENT_ID required"),
            github_client_secret: std::env::var("GITHUB_CLIENT_SECRET").expect("GITHUB_CLIENT_SECRET required"),
            server_url: std::env::var("SERVER_URL").unwrap_or_else(|_| "http://localhost:3000".into()),
            session_secret: std::env::var("SESSION_SECRET").expect("SESSION_SECRET required"),
            port: std::env::var("PORT").unwrap_or_else(|_| "3000".into()).parse().expect("PORT must be a number"),
        }
    }
}
```

**Step 4: Write initial `main.rs`**

```rust
mod config;
mod error;

use axum::{Router, routing::get};
use sqlx::postgres::PgPoolOptions;
use tracing_subscriber::EnvFilter;

use config::Config;

#[tokio::main]
async fn main() {
    dotenvy::dotenv().ok();

    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::from_default_env())
        .init();

    let config = Config::from_env();

    let pool = PgPoolOptions::new()
        .max_connections(5)
        .connect(&config.database_url)
        .await
        .expect("Failed to connect to database");

    sqlx::migrate!("./migrations")
        .run(&pool)
        .await
        .expect("Failed to run migrations");

    let app = Router::new()
        .route("/health", get(|| async { "OK" }));

    let addr = format!("0.0.0.0:{}", config.port);
    tracing::info!("Starting server on {}", addr);
    let listener = tokio::net::TcpListener::bind(&addr).await.unwrap();
    axum::serve(listener, app).await.unwrap();
}
```

**Step 5: Add to workspace**

In root `Cargo.toml`:
```toml
members = ["crates/rk-client", "crates/rk-shared", "crates/rk-server"]
```

**Step 6: Verify (client-only build, since server needs Postgres)**

```bash
cargo build -p rk-shared
cargo build -p rk-client
cargo check -p rk-server
```

**Step 7: Commit**

```bash
git commit -m "feat: add rk-server crate skeleton with Axum, config, and health endpoint"
```

---

### Task 13: PostgreSQL Schema Migration

**Files:**
- Create: `crates/rk-server/migrations/001_initial.sql`

**Step 1: Write migration**

```sql
-- Users
CREATE TABLE users (
    id UUID PRIMARY KEY,
    github_id BIGINT UNIQUE NOT NULL,
    username TEXT NOT NULL,
    email TEXT,
    created_at TIMESTAMP NOT NULL DEFAULT NOW()
);

-- Devices
CREATE TABLE devices (
    id UUID PRIMARY KEY,
    user_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    name TEXT NOT NULL,
    last_synced_at TIMESTAMP,
    created_at TIMESTAMP NOT NULL DEFAULT NOW(),
    stale BOOLEAN NOT NULL DEFAULT FALSE
);

-- Auth tokens (SHA-256 hashed)
CREATE TABLE auth_tokens (
    token_hash TEXT PRIMARY KEY,
    user_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    device_id UUID NOT NULL REFERENCES devices(id) ON DELETE CASCADE,
    expires_at TIMESTAMP,
    created_at TIMESTAMP NOT NULL DEFAULT NOW()
);

-- Tasks
CREATE TABLE tasks (
    uuid UUID PRIMARY KEY,
    user_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    title TEXT NOT NULL,
    description TEXT NOT NULL DEFAULT '',
    priority TEXT NOT NULL DEFAULT 'Medium',
    column_name TEXT NOT NULL DEFAULT 'todo',
    due_date DATE,
    created_at TIMESTAMP NOT NULL,
    updated_at TIMESTAMP NOT NULL,
    deleted BOOLEAN NOT NULL DEFAULT FALSE,
    deleted_at TIMESTAMP
);

-- Tags
CREATE TABLE tags (
    uuid UUID PRIMARY KEY,
    user_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    name TEXT NOT NULL,
    updated_at TIMESTAMP NOT NULL,
    deleted BOOLEAN NOT NULL DEFAULT FALSE,
    deleted_at TIMESTAMP
);

-- Enforce unique tag names per user (only among active tags)
CREATE UNIQUE INDEX idx_tags_user_name_active ON tags (user_id, name) WHERE deleted = FALSE;

-- Task-tag associations
CREATE TABLE task_tags (
    task_uuid UUID NOT NULL REFERENCES tasks(uuid) ON DELETE CASCADE,
    tag_uuid UUID NOT NULL REFERENCES tags(uuid) ON DELETE CASCADE,
    PRIMARY KEY (task_uuid, tag_uuid)
);

-- Performance indexes
CREATE INDEX idx_tasks_user_updated ON tasks (user_id, updated_at);
CREATE INDEX idx_tags_user_updated ON tags (user_id, updated_at);
CREATE INDEX idx_devices_user ON devices (user_id);
CREATE INDEX idx_auth_tokens_user ON auth_tokens (user_id);
```

**Step 2: Commit**

```bash
git commit -m "feat: add PostgreSQL schema migration for sync"
```

---

### Task 14: Server Auth — Bearer Token Middleware + GitHub OAuth

**Files:**
- Create: `crates/rk-server/src/auth.rs`
- Create: `crates/rk-server/src/routes/mod.rs`
- Create: `crates/rk-server/src/routes/auth.rs`
- Modify: `crates/rk-server/src/main.rs`

**Step 1: Create auth middleware**

`crates/rk-server/src/auth.rs`:

```rust
use axum::{extract::FromRequestParts, http::request::Parts};
use sha2::{Digest, Sha256};
use sqlx::PgPool;
use uuid::Uuid;

#[derive(Debug, Clone)]
pub struct AuthUser {
    pub user_id: Uuid,
    pub device_id: Uuid,
}

#[axum::async_trait]
impl<S: Send + Sync> FromRequestParts<S> for AuthUser {
    type Rejection = axum::http::StatusCode;

    async fn from_request_parts(parts: &mut Parts, _state: &S) -> Result<Self, Self::Rejection> {
        let pool = parts.extensions.get::<PgPool>()
            .ok_or(axum::http::StatusCode::INTERNAL_SERVER_ERROR)?;

        let auth_header = parts.headers.get("Authorization")
            .and_then(|v| v.to_str().ok())
            .and_then(|v| v.strip_prefix("Bearer "))
            .ok_or(axum::http::StatusCode::UNAUTHORIZED)?;

        let hash = hash_token(auth_header);

        let record = sqlx::query_as::<_, (Uuid, Uuid)>(
            "SELECT user_id, device_id FROM auth_tokens WHERE token_hash = $1 AND (expires_at IS NULL OR expires_at > NOW())"
        )
        .bind(&hash)
        .fetch_optional(pool)
        .await
        .map_err(|_| axum::http::StatusCode::INTERNAL_SERVER_ERROR)?
        .ok_or(axum::http::StatusCode::UNAUTHORIZED)?;

        Ok(AuthUser {
            user_id: record.0,
            device_id: record.1,
        })
    }
}

pub fn hash_token(token: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(token.as_bytes());
    format!("{:x}", hasher.finalize())
}

pub fn generate_token() -> String {
    format!("rk_{}", Uuid::new_v4().to_string().replace('-', ""))
}
```

**Step 2: Create OAuth route handlers**

`crates/rk-server/src/routes/auth.rs`:

Implement:
- `GET /login` — store `redirect_port` and `device_name` in session, redirect to GitHub OAuth
- `GET /auth/callback` — exchange code, create/find user, create device + token, redirect to localhost callback
- `POST /auth/logout` — clear session

The OAuth flow uses the `oauth2` crate with GitHub's endpoints:
- Authorization: `https://github.com/login/oauth/authorize`
- Token: `https://github.com/login/oauth/access_token`
- User info: `https://api.github.com/user`

**Step 3: Wire routes in `main.rs`**

```rust
let app = Router::new()
    .route("/health", get(|| async { "OK" }))
    .route("/login", get(routes::auth::login))
    .route("/auth/callback", get(routes::auth::callback))
    .route("/auth/logout", post(routes::auth::logout))
    .layer(Extension(pool.clone()))
    .layer(Extension(config.clone()));
```

**Step 4: Commit**

```bash
git commit -m "feat: add GitHub OAuth login flow and bearer token auth middleware"
```

---

### Task 15: Server Sync API Endpoints

**Files:**
- Create: `crates/rk-server/src/routes/sync.rs`
- Create: `crates/rk-server/src/sync_logic.rs`

**Step 1: Implement pull endpoint**

```rust
pub async fn pull(
    auth: AuthUser,
    State(pool): State<PgPool>,
    Json(payload): Json<SyncPayload>,
) -> Result<Json<SyncResponse>, AppError> {
    let tasks = if payload.last_synced_at.is_some() {
        // Delta pull
        sqlx::query_as("SELECT ... FROM tasks WHERE user_id = $1 AND updated_at > $2")
            .bind(auth.user_id)
            .bind(&payload.last_synced_at)
            .fetch_all(&pool).await?
    } else {
        // Full pull
        sqlx::query_as("SELECT ... FROM tasks WHERE user_id = $1")
            .bind(auth.user_id)
            .fetch_all(&pool).await?
    };

    // Similar for tags

    // Update device.last_synced_at
    sqlx::query("UPDATE devices SET last_synced_at = NOW(), stale = FALSE WHERE id = $1")
        .bind(auth.device_id)
        .execute(&pool).await?;

    Ok(Json(SyncResponse {
        tasks: tasks.into_iter().map(to_sync_task).collect(),
        tags: tags.into_iter().map(to_sync_tag).collect(),
        tag_uuid_mappings: HashMap::new(),
        synced_at: chrono::Utc::now().format("%Y-%m-%dT%H:%M:%S").to_string(),
    }))
}
```

**Step 2: Implement push endpoint with LWW merge + tag deduplication**

The push handler processes within a single transaction:

1. **Tags first:** For each incoming tag:
   - Check if `(user_id, name)` exists with a different UUID → deduplicate (map client UUID to server UUID)
   - Otherwise upsert with LWW
2. **Remap task tag references** using dedup mappings
3. **Tasks:** Upsert with LWW
4. **Task-tags:** Replace associations

Return: any records where server version won + `tag_uuid_mappings`.

```rust
pub async fn push(
    auth: AuthUser,
    State(pool): State<PgPool>,
    Json(payload): Json<SyncPayload>,
) -> Result<Json<SyncResponse>, AppError> {
    // Validate limits
    validate_limits(&pool, auth.user_id, &payload).await?;

    let mut tx = pool.begin().await?;
    let mut tag_uuid_mappings = HashMap::new();
    let mut rejected_tasks = Vec::new();
    let mut rejected_tags = Vec::new();

    // Process tags (dedup by name)
    for tag in &payload.tags {
        let result = process_tag(&mut tx, auth.user_id, tag, &mut tag_uuid_mappings).await?;
        if let Some(rejected) = result {
            rejected_tags.push(rejected);
        }
    }

    // Process tasks (with remapped tag UUIDs)
    for task in &payload.tasks {
        let remapped_tags: Vec<String> = task.tags.iter()
            .map(|t| tag_uuid_mappings.get(t).cloned().unwrap_or_else(|| t.clone()))
            .collect();
        let result = process_task(&mut tx, auth.user_id, task, &remapped_tags).await?;
        if let Some(rejected) = result {
            rejected_tasks.push(rejected);
        }
    }

    // Update device
    sqlx::query("UPDATE devices SET last_synced_at = NOW(), stale = FALSE WHERE id = $1")
        .bind(auth.device_id)
        .execute(&mut *tx).await?;

    tx.commit().await?;

    Ok(Json(SyncResponse {
        tasks: rejected_tasks,
        tags: rejected_tags,
        tag_uuid_mappings,
        synced_at: chrono::Utc::now().format("%Y-%m-%dT%H:%M:%S").to_string(),
    }))
}
```

**Step 3: Implement combined sync endpoint**

Combines pull + push in one round trip within a single transaction.

**Step 4: Wire routes**

```rust
.route("/api/v1/sync/pull", post(routes::sync::pull))
.route("/api/v1/sync/push", post(routes::sync::push))
.route("/api/v1/sync", post(routes::sync::combined))
```

**Step 5: Add request body limit**

```rust
use tower_http::limit::RequestBodyLimitLayer;

.layer(RequestBodyLimitLayer::new(10 * 1024 * 1024)) // 10MB
```

**Step 6: Add validation**

```rust
async fn validate_limits(pool: &PgPool, user_id: Uuid, payload: &SyncPayload) -> Result<(), AppError> {
    let task_count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM tasks WHERE user_id = $1")
        .bind(user_id).fetch_one(pool).await?;
    if task_count + payload.tasks.len() as i64 > 200 {
        return Err(AppError::Validation("Task limit exceeded (max 200)".into()));
    }
    // Similar for tags (15), devices (5)
    Ok(())
}
```

**Step 7: Commit**

```bash
git commit -m "feat: add sync pull/push/combined API endpoints with LWW merge and tag dedup"
```

---

### Task 16: Server Device Management + Purge Job

**Files:**
- Create: `crates/rk-server/src/routes/account.rs`
- Create: `crates/rk-server/src/purge.rs`
- Modify: `crates/rk-server/src/main.rs`

**Step 1: Implement device/account routes**

```rust
// GET /account/devices — list devices for authenticated user (session-based)
// POST /account/devices/:id/revoke — delete device + its auth token
// POST /account/delete — delete user + cascade all data
```

**Step 2: Implement purge background task**

`crates/rk-server/src/purge.rs`:

```rust
pub fn spawn_purge_job(pool: PgPool) {
    tokio::spawn(async move {
        let mut interval = tokio::time::interval(std::time::Duration::from_secs(86400)); // 24h
        loop {
            interval.tick().await;
            if let Err(e) = run_purge(&pool).await {
                tracing::error!("Purge job failed: {}", e);
            }
        }
    });
}

async fn run_purge(pool: &PgPool) -> Result<(), sqlx::Error> {
    // For each user: find oldest non-stale device's last_synced_at
    // Hard-delete records where deleted=true AND deleted_at < that timestamp
    let users = sqlx::query_scalar::<_, Uuid>("SELECT DISTINCT user_id FROM devices WHERE stale = FALSE")
        .fetch_all(pool).await?;

    for user_id in users {
        let oldest_sync: Option<chrono::NaiveDateTime> = sqlx::query_scalar(
            "SELECT MIN(last_synced_at) FROM devices WHERE user_id = $1 AND stale = FALSE AND last_synced_at IS NOT NULL"
        ).bind(user_id).fetch_one(pool).await?;

        if let Some(cutoff) = oldest_sync {
            // Hard-delete task_tags for deleted tasks
            sqlx::query(
                "DELETE FROM task_tags WHERE task_uuid IN (SELECT uuid FROM tasks WHERE user_id = $1 AND deleted = TRUE AND deleted_at < $2)"
            ).bind(user_id).bind(cutoff).execute(pool).await?;

            // Hard-delete tasks
            sqlx::query("DELETE FROM tasks WHERE user_id = $1 AND deleted = TRUE AND deleted_at < $2")
                .bind(user_id).bind(cutoff).execute(pool).await?;

            // Hard-delete tags
            sqlx::query("DELETE FROM tags WHERE user_id = $1 AND deleted = TRUE AND deleted_at < $2")
                .bind(user_id).bind(cutoff).execute(pool).await?;
        }
    }

    // Mark stale devices (no sync in 90 days)
    sqlx::query("UPDATE devices SET stale = TRUE WHERE last_synced_at < NOW() - INTERVAL '90 days' AND stale = FALSE")
        .execute(pool).await?;

    Ok(())
}
```

**Step 3: Spawn purge on startup**

In `main.rs`:

```rust
purge::spawn_purge_job(pool.clone());
```

**Step 4: Commit**

```bash
git commit -m "feat: add device management routes and background purge job"
```

---

## Phase 7: Server Website + Polish

### Task 17: Website Templates

**Files:**
- Create: `crates/rk-server/templates/base.html`
- Create: `crates/rk-server/templates/home.html`
- Create: `crates/rk-server/templates/account.html`
- Create: `crates/rk-server/src/routes/pages.rs`

**Step 1: Create Askama templates**

Three server-rendered pages:
1. **Homepage** — hero section, features, install instructions, "Login with GitHub" nav link
2. **Account** — GitHub username/avatar, devices table with revoke buttons, danger zone
3. **Login completion** — "You can close this tab" (shown after OAuth redirect to localhost)

Use Pico CSS (CDN) for minimal styling. No JS framework — vanilla JS only for delete confirmation.

**Step 2: Wire page routes**

```rust
.route("/", get(routes::pages::home))
.route("/account", get(routes::pages::account))
.route("/account/devices", get(routes::pages::devices))
```

**Step 3: Commit**

```bash
git commit -m "feat: add server-rendered website pages (home, account, devices)"
```

---

### Task 18: Documentation Updates

**Files:**
- Modify: `README.md` — add sync section, new keybindings (Ctrl+R), new CLI commands
- Modify: `CHANGELOG.md` — add sync feature entry
- Modify: `CLAUDE.md` — update module map, add sync/auth modules, update build instructions
- Modify: `docs/USE_CASES.md` — add sync use cases
- Modify: `demo.tape` — (optional) add sync demo if practical

**Step 1: Update all documentation per CLAUDE.md conventions**

Key additions:
- New CLI commands: `rk login`, `rk logout`, `rk sync`, `rk status`
- New keybinding: `Ctrl+R` for manual sync
- New modules: `auth.rs`, `sync.rs`
- New architecture: cargo workspace with 3 crates
- Status bar now shows sync state

**Step 2: Commit**

```bash
git commit -m "docs: update README, CHANGELOG, CLAUDE.md for sync feature"
```

---

## Dependency Graph

```
Task 1 (Workspace) ──► Task 2 (rk-shared) ──► Task 10 (Sync Module)
                                                    │
Task 3 (Schema Version) ──► Task 4 (Model Update)  │
                              │                     │
                              ▼                     │
                          Task 5 (Soft Delete)      │
                              │                     │
                              ├─► Task 6 (Export v2) │
                              │                     │
                              └─► Task 7 (Validation)│
                                                    │
Task 8 (Auth Module) ──► Task 9 (Login CLI) ──► Task 11 (TUI Integration)
                                                    │
Task 12 (Server Skeleton) ──► Task 13 (PG Schema) ──► Task 14 (Server Auth)
                                                        │
                                                    Task 15 (Sync API)
                                                        │
                                                    Task 16 (Devices/Purge)
                                                        │
                                                    Task 17 (Website)
                                                        │
                                                    Task 18 (Docs)
```

**Independent tracks that can be parallelized:**
- Client DB evolution (Tasks 3-7) runs independently of server work (Tasks 12-17)
- Client auth (Tasks 8-9) can start after Task 3
- Server work (Tasks 12-17) can start after Task 2

---

## Verification Checklist

After all tasks are complete:

1. `cargo build` — builds all workspace members
2. `cargo test` — all tests pass across all crates
3. `cargo clippy -- -D warnings` — zero warnings
4. `cargo fmt -- --check` — formatting clean
5. `rk` — TUI works without login (no sync indicators)
6. `rk login --token test --device-id test --server http://localhost:3000` — saves credentials
7. `rk status` — shows login state
8. `rk logout` — clears credentials
9. Soft deletes: D → Y deletes, Ctrl+Z undeletes, Clear Done soft-deletes
10. Export/import: v2 format with UUIDs, v1 backward compatible
11. Text limits: can't type beyond 500 chars in title, 5000 in description, 50 in tag name
12. Server: `rk-server` starts, `/health` returns 200, OAuth flow works, sync endpoints accept/return correct payloads
