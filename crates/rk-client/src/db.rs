use rusqlite::{Connection, Result as SqliteResult};

use crate::model::{Board, Column, Priority, Tag, Task};

pub(crate) const TS_FMT: &str = "%Y-%m-%dT%H:%M:%S";

pub fn init_db(path: &std::path::Path) -> SqliteResult<Connection> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).expect("Failed to create database directory");
    }

    let conn = Connection::open(path)?;
    conn.execute_batch("PRAGMA foreign_keys = ON;")?;
    run_migrations(&conn)?;
    Ok(conn)
}

fn get_schema_version(conn: &Connection) -> i32 {
    conn.query_row(
        "SELECT value FROM preferences WHERE key = 'schema_version'",
        [],
        |row| {
            let v: String = row.get(0)?;
            Ok(v.parse::<i32>().unwrap_or(1))
        },
    )
    .unwrap_or(1)
}

fn migrate_v2(conn: &Connection) -> SqliteResult<()> {
    // Add new columns to tasks
    conn.execute_batch(
        "
        ALTER TABLE tasks ADD COLUMN uuid TEXT;
        ALTER TABLE tasks ADD COLUMN deleted INTEGER NOT NULL DEFAULT 0;
        ALTER TABLE tasks ADD COLUMN deleted_at TEXT;
        ",
    )?;

    // Add new columns to tags
    conn.execute_batch(
        "
        ALTER TABLE tags ADD COLUMN uuid TEXT;
        ALTER TABLE tags ADD COLUMN updated_at TEXT;
        ALTER TABLE tags ADD COLUMN deleted INTEGER NOT NULL DEFAULT 0;
        ALTER TABLE tags ADD COLUMN deleted_at TEXT;
        ",
    )?;

    // Backfill UUIDs for existing tasks
    {
        let mut stmt = conn.prepare("SELECT id FROM tasks WHERE uuid IS NULL")?;
        let ids: Vec<i64> = stmt
            .query_map([], |row| row.get(0))?
            .collect::<SqliteResult<Vec<_>>>()?;
        let mut update = conn.prepare("UPDATE tasks SET uuid = ?1 WHERE id = ?2")?;
        for id in ids {
            let new_uuid = uuid::Uuid::new_v4().to_string();
            update.execute(rusqlite::params![new_uuid, id])?;
        }
    }

    // Backfill UUIDs for existing tags
    {
        let mut stmt = conn.prepare("SELECT id FROM tags WHERE uuid IS NULL")?;
        let ids: Vec<i64> = stmt
            .query_map([], |row| row.get(0))?
            .collect::<SqliteResult<Vec<_>>>()?;
        let mut update = conn.prepare("UPDATE tags SET uuid = ?1 WHERE id = ?2")?;
        for id in ids {
            let new_uuid = uuid::Uuid::new_v4().to_string();
            update.execute(rusqlite::params![new_uuid, id])?;
        }
    }

    // Backfill updated_at for existing tags
    {
        let now = now_timestamp();
        conn.execute(
            "UPDATE tags SET updated_at = ?1 WHERE updated_at IS NULL",
            rusqlite::params![now],
        )?;
    }

    // Create unique indexes
    conn.execute_batch(
        "
        CREATE UNIQUE INDEX IF NOT EXISTS idx_tasks_uuid ON tasks(uuid);
        CREATE UNIQUE INDEX IF NOT EXISTS idx_tags_uuid ON tags(uuid);
        ",
    )?;

    // Record schema version
    conn.execute(
        "INSERT INTO preferences (key, value) VALUES ('schema_version', '2')
         ON CONFLICT(key) DO UPDATE SET value = '2'",
        [],
    )?;

    Ok(())
}

fn migrate_v3(conn: &Connection) -> SqliteResult<()> {
    // Create boards table
    conn.execute_batch(
        "
        CREATE TABLE IF NOT EXISTS boards (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            uuid TEXT NOT NULL UNIQUE,
            name TEXT NOT NULL,
            position INTEGER NOT NULL DEFAULT 0,
            created_at TEXT NOT NULL,
            updated_at TEXT NOT NULL,
            deleted INTEGER NOT NULL DEFAULT 0,
            deleted_at TEXT
        );
        ",
    )?;

    // Create the default "Personal" board
    let board_uuid = uuid::Uuid::new_v4().to_string();
    let now = now_timestamp();
    conn.execute(
        "INSERT INTO boards (uuid, name, position, created_at, updated_at) VALUES (?1, ?2, 0, ?3, ?4)",
        rusqlite::params![board_uuid, "Personal", now, now],
    )?;

    // Add board_id column to tasks
    conn.execute_batch("ALTER TABLE tasks ADD COLUMN board_id TEXT NOT NULL DEFAULT '';")?;

    // Backfill all existing tasks to the Personal board
    conn.execute(
        "UPDATE tasks SET board_id = ?1",
        rusqlite::params![board_uuid],
    )?;

    // Save the active board preference
    conn.execute(
        "INSERT INTO preferences (key, value) VALUES ('active_board', ?1)
         ON CONFLICT(key) DO UPDATE SET value = ?1",
        rusqlite::params![board_uuid],
    )?;

    // Record schema version
    conn.execute(
        "INSERT INTO preferences (key, value) VALUES ('schema_version', '3')
         ON CONFLICT(key) DO UPDATE SET value = '3'",
        [],
    )?;

    Ok(())
}

pub(crate) fn run_migrations(conn: &Connection) -> SqliteResult<()> {
    // v1: idempotent table creation
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

    // v2: add uuid, deleted, deleted_at columns
    if get_schema_version(conn) < 2 {
        migrate_v2(conn)?;
    }

    // v3: add boards table, board_id column on tasks
    if get_schema_version(conn) < 3 {
        migrate_v3(conn)?;
    }

    Ok(())
}

/// Load only active (non-deleted) tasks.
pub fn load_tasks(conn: &Connection) -> SqliteResult<Vec<Task>> {
    load_tasks_filtered(conn, true)
}

/// Load all tasks including soft-deleted ones.
#[allow(dead_code)]
pub fn load_all_tasks(conn: &Connection) -> SqliteResult<Vec<Task>> {
    load_tasks_filtered(conn, false)
}

fn load_tasks_filtered(conn: &Connection, active_only: bool) -> SqliteResult<Vec<Task>> {
    let sql = if active_only {
        "SELECT id, uuid, title, description, priority, column_name, due_date, created_at, updated_at, deleted, deleted_at, board_id
         FROM tasks WHERE deleted = 0"
    } else {
        "SELECT id, uuid, title, description, priority, column_name, due_date, created_at, updated_at, deleted, deleted_at, board_id
         FROM tasks"
    };

    let mut stmt = conn.prepare(sql)?;

    let mut tasks = stmt
        .query_map([], |row| {
            let priority_str: String = row.get(4)?;
            let column_str: String = row.get(5)?;
            let due_date_str: Option<String> = row.get(6)?;
            let created_str: String = row.get(7)?;
            let updated_str: String = row.get(8)?;
            let deleted_int: i32 = row.get(9)?;
            let deleted_at_str: Option<String> = row.get(10)?;
            let board_id_str: String = row.get(11)?;

            Ok(Task {
                id: row.get(0)?,
                uuid: row.get(1)?,
                board_id: board_id_str,
                title: row.get(2)?,
                description: row.get(3)?,
                priority: priority_str.parse::<Priority>().unwrap_or(Priority::Medium),
                column: column_str.parse::<Column>().unwrap_or(Column::Todo),
                due_date: due_date_str
                    .and_then(|s| chrono::NaiveDate::parse_from_str(&s, "%Y-%m-%d").ok()),
                tags: Vec::new(),
                created_at: chrono::NaiveDateTime::parse_from_str(&created_str, TS_FMT)
                    .unwrap_or_default(),
                updated_at: chrono::NaiveDateTime::parse_from_str(&updated_str, TS_FMT)
                    .unwrap_or_default(),
                deleted: deleted_int != 0,
                deleted_at: deleted_at_str
                    .and_then(|s| chrono::NaiveDateTime::parse_from_str(&s, TS_FMT).ok()),
            })
        })?
        .collect::<SqliteResult<Vec<_>>>()?;

    // Load all task tags in a single query
    let tag_sql = if active_only {
        "SELECT tt.task_id, t.name FROM tags t
         JOIN task_tags tt ON t.id = tt.tag_id
         WHERE t.deleted = 0
         ORDER BY t.name"
    } else {
        "SELECT tt.task_id, t.name FROM tags t
         JOIN task_tags tt ON t.id = tt.tag_id
         ORDER BY t.name"
    };

    let mut tag_stmt = conn.prepare(tag_sql)?;

    let mut tag_map: std::collections::HashMap<i64, Vec<String>> = std::collections::HashMap::new();
    tag_stmt
        .query_map([], |row| {
            Ok((row.get::<_, i64>(0)?, row.get::<_, String>(1)?))
        })?
        .for_each(|r| {
            if let Ok((task_id, tag_name)) = r {
                tag_map.entry(task_id).or_default().push(tag_name);
            }
        });

    for task in &mut tasks {
        if let Some(tags) = tag_map.remove(&task.id) {
            task.tags = tags;
        }
    }

    Ok(tasks)
}

/// Load only active (non-deleted) tags.
pub fn load_tags(conn: &Connection) -> SqliteResult<Vec<Tag>> {
    load_tags_filtered(conn, true)
}

/// Load all tags including soft-deleted ones.
#[allow(dead_code)]
pub fn load_all_tags(conn: &Connection) -> SqliteResult<Vec<Tag>> {
    load_tags_filtered(conn, false)
}

fn load_tags_filtered(conn: &Connection, active_only: bool) -> SqliteResult<Vec<Tag>> {
    let sql = if active_only {
        "SELECT id, uuid, name, updated_at, deleted, deleted_at FROM tags WHERE deleted = 0 ORDER BY name"
    } else {
        "SELECT id, uuid, name, updated_at, deleted, deleted_at FROM tags ORDER BY name"
    };

    let mut stmt = conn.prepare(sql)?;
    let tags = stmt
        .query_map([], |row| {
            let updated_str: Option<String> = row.get(3)?;
            let deleted_int: i32 = row.get(4)?;
            let deleted_at_str: Option<String> = row.get(5)?;

            Ok(Tag {
                id: row.get(0)?,
                uuid: row.get(1)?,
                name: row.get(2)?,
                updated_at: updated_str
                    .and_then(|s| chrono::NaiveDateTime::parse_from_str(&s, TS_FMT).ok())
                    .unwrap_or_default(),
                deleted: deleted_int != 0,
                deleted_at: deleted_at_str
                    .and_then(|s| chrono::NaiveDateTime::parse_from_str(&s, TS_FMT).ok()),
            })
        })?
        .collect::<SqliteResult<Vec<_>>>()?;
    Ok(tags)
}

fn now_timestamp() -> String {
    chrono::Local::now()
        .naive_local()
        .format(TS_FMT)
        .to_string()
}

pub fn insert_tag(conn: &Connection, name: &str) -> SqliteResult<i64> {
    let tag_uuid = uuid::Uuid::new_v4().to_string();
    let now = now_timestamp();
    conn.execute(
        "INSERT INTO tags (name, uuid, updated_at) VALUES (?1, ?2, ?3)",
        rusqlite::params![name, tag_uuid, now],
    )?;
    Ok(conn.last_insert_rowid())
}

pub fn insert_tag_with_uuid(conn: &Connection, uuid: &str, name: &str) -> SqliteResult<i64> {
    let now = now_timestamp();
    conn.execute(
        "INSERT INTO tags (name, uuid, updated_at) VALUES (?1, ?2, ?3)",
        rusqlite::params![name, uuid, now],
    )?;
    Ok(conn.last_insert_rowid())
}

pub fn tag_uuid_exists(conn: &Connection, uuid: &str) -> SqliteResult<bool> {
    let count: i64 = conn.query_row(
        "SELECT COUNT(*) FROM tags WHERE uuid = ?1",
        rusqlite::params![uuid],
        |row| row.get(0),
    )?;
    Ok(count > 0)
}

#[allow(dead_code)]
pub fn delete_tag(conn: &Connection, tag_id: i64) -> SqliteResult<()> {
    conn.execute("DELETE FROM tags WHERE id = ?1", rusqlite::params![tag_id])?;
    Ok(())
}

pub fn rename_tag(conn: &Connection, tag_id: i64, new_name: &str) -> SqliteResult<()> {
    let now = now_timestamp();
    conn.execute(
        "UPDATE tags SET name = ?1, updated_at = ?2 WHERE id = ?3",
        rusqlite::params![new_name, now, tag_id],
    )?;
    Ok(())
}

pub fn set_task_tags(conn: &Connection, task_id: i64, tag_ids: &[i64]) -> SqliteResult<()> {
    conn.execute(
        "DELETE FROM task_tags WHERE task_id = ?1",
        rusqlite::params![task_id],
    )?;
    let mut stmt = conn.prepare("INSERT INTO task_tags (task_id, tag_id) VALUES (?1, ?2)")?;
    for &tag_id in tag_ids {
        stmt.execute(rusqlite::params![task_id, tag_id])?;
    }
    Ok(())
}

pub fn get_task_tag_ids(conn: &Connection, task_id: i64) -> SqliteResult<Vec<i64>> {
    let mut stmt = conn.prepare("SELECT tag_id FROM task_tags WHERE task_id = ?1")?;
    let ids = stmt
        .query_map(rusqlite::params![task_id], |row| row.get(0))?
        .collect::<SqliteResult<Vec<_>>>()?;
    Ok(ids)
}

pub fn insert_task(
    conn: &Connection,
    title: &str,
    description: &str,
    priority: Priority,
    column: Column,
    due_date: Option<chrono::NaiveDate>,
    board_id: &str,
) -> SqliteResult<i64> {
    let task_uuid = uuid::Uuid::new_v4().to_string();
    insert_task_with_uuid(
        conn,
        &task_uuid,
        title,
        description,
        priority,
        column,
        due_date,
        board_id,
    )
}

#[allow(clippy::too_many_arguments)]
pub fn insert_task_with_uuid(
    conn: &Connection,
    uuid: &str,
    title: &str,
    description: &str,
    priority: Priority,
    column: Column,
    due_date: Option<chrono::NaiveDate>,
    board_id: &str,
) -> SqliteResult<i64> {
    let now = now_timestamp();
    let due_date_str = due_date.map(|d| d.format("%Y-%m-%d").to_string());

    conn.execute(
        "INSERT INTO tasks (title, description, priority, column_name, due_date, created_at, updated_at, uuid, board_id)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
        rusqlite::params![
            title,
            description,
            priority.as_str(),
            column.as_str(),
            due_date_str,
            now,
            now,
            uuid,
            board_id,
        ],
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

pub fn update_task_column(conn: &Connection, task_id: i64, column: Column) -> SqliteResult<()> {
    let now = now_timestamp();
    conn.execute(
        "UPDATE tasks SET column_name = ?1, updated_at = ?2 WHERE id = ?3",
        rusqlite::params![column.as_str(), now, task_id],
    )?;
    Ok(())
}

pub fn update_task_priority(
    conn: &Connection,
    task_id: i64,
    priority: Priority,
) -> SqliteResult<()> {
    let now = now_timestamp();
    conn.execute(
        "UPDATE tasks SET priority = ?1, updated_at = ?2 WHERE id = ?3",
        rusqlite::params![priority.as_str(), now, task_id],
    )?;
    Ok(())
}

pub fn update_task(
    conn: &Connection,
    task_id: i64,
    title: &str,
    description: &str,
    priority: Priority,
    due_date: Option<chrono::NaiveDate>,
) -> SqliteResult<()> {
    let now = now_timestamp();
    let due_date_str = due_date.map(|d| d.format("%Y-%m-%d").to_string());
    conn.execute(
        "UPDATE tasks SET title = ?1, description = ?2, priority = ?3, due_date = ?4, updated_at = ?5 WHERE id = ?6",
        rusqlite::params![title, description, priority.as_str(), due_date_str, now, task_id],
    )?;
    Ok(())
}

/// Hard-delete a task (used by `reset_db()`).
#[allow(dead_code)]
pub fn delete_task(conn: &Connection, task_id: i64) -> SqliteResult<()> {
    conn.execute(
        "DELETE FROM task_tags WHERE task_id = ?1",
        rusqlite::params![task_id],
    )?;
    conn.execute(
        "DELETE FROM tasks WHERE id = ?1",
        rusqlite::params![task_id],
    )?;
    Ok(())
}

pub fn soft_delete_task(conn: &Connection, task_id: i64) -> SqliteResult<()> {
    let now = now_timestamp();
    conn.execute(
        "UPDATE tasks SET deleted = 1, deleted_at = ?1, updated_at = ?1 WHERE id = ?2",
        rusqlite::params![now, task_id],
    )?;
    Ok(())
}

pub fn soft_delete_tag(conn: &Connection, tag_id: i64) -> SqliteResult<()> {
    let now = now_timestamp();
    conn.execute(
        "UPDATE tags SET deleted = 1, deleted_at = ?1, updated_at = ?1 WHERE id = ?2",
        rusqlite::params![now, tag_id],
    )?;
    // Remove task_tags associations (logical cascade)
    conn.execute(
        "DELETE FROM task_tags WHERE tag_id = ?1",
        rusqlite::params![tag_id],
    )?;
    Ok(())
}

#[allow(dead_code)]
pub fn cleanup_old_soft_deletes(conn: &Connection, days: i64) -> SqliteResult<()> {
    let cutoff = (chrono::Local::now() - chrono::Duration::days(days))
        .naive_local()
        .format(TS_FMT)
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
    conn.execute(
        "DELETE FROM boards WHERE deleted = 1 AND deleted_at < ?1",
        rusqlite::params![cutoff],
    )?;
    Ok(())
}

pub fn load_boards(conn: &Connection) -> SqliteResult<Vec<Board>> {
    load_boards_filtered(conn, true)
}

pub fn load_all_boards(conn: &Connection) -> SqliteResult<Vec<Board>> {
    load_boards_filtered(conn, false)
}

fn load_boards_filtered(conn: &Connection, active_only: bool) -> SqliteResult<Vec<Board>> {
    let sql = if active_only {
        "SELECT id, uuid, name, position, created_at, updated_at, deleted, deleted_at
         FROM boards WHERE deleted = 0 ORDER BY position, id"
    } else {
        "SELECT id, uuid, name, position, created_at, updated_at, deleted, deleted_at
         FROM boards ORDER BY position, id"
    };
    let mut stmt = conn.prepare(sql)?;
    let boards = stmt
        .query_map([], |row| {
            let created_str: String = row.get(4)?;
            let updated_str: String = row.get(5)?;
            let deleted_int: i32 = row.get(6)?;
            let deleted_at_str: Option<String> = row.get(7)?;
            Ok(Board {
                id: row.get(0)?,
                uuid: row.get(1)?,
                name: row.get(2)?,
                position: row.get(3)?,
                created_at: chrono::NaiveDateTime::parse_from_str(&created_str, TS_FMT)
                    .unwrap_or_default(),
                updated_at: chrono::NaiveDateTime::parse_from_str(&updated_str, TS_FMT)
                    .unwrap_or_default(),
                deleted: deleted_int != 0,
                deleted_at: deleted_at_str
                    .and_then(|s| chrono::NaiveDateTime::parse_from_str(&s, TS_FMT).ok()),
            })
        })?
        .collect::<SqliteResult<Vec<_>>>()?;
    Ok(boards)
}

pub fn insert_board(conn: &Connection, name: &str) -> SqliteResult<i64> {
    let uuid = uuid::Uuid::new_v4().to_string();
    let now = now_timestamp();
    let position: i32 = conn.query_row(
        "SELECT COALESCE(MAX(position), -1) + 1 FROM boards WHERE deleted = 0",
        [],
        |row| row.get(0),
    )?;
    conn.execute(
        "INSERT INTO boards (uuid, name, position, created_at, updated_at) VALUES (?1, ?2, ?3, ?4, ?5)",
        rusqlite::params![uuid, name, position, now, now],
    )?;
    Ok(conn.last_insert_rowid())
}

pub fn insert_board_with_uuid(
    conn: &Connection,
    uuid: &str,
    name: &str,
    position: i32,
) -> SqliteResult<i64> {
    let now = now_timestamp();
    conn.execute(
        "INSERT INTO boards (uuid, name, position, created_at, updated_at) VALUES (?1, ?2, ?3, ?4, ?5)",
        rusqlite::params![uuid, name, position, now, now],
    )?;
    Ok(conn.last_insert_rowid())
}

pub fn update_board_name(conn: &Connection, board_id: i64, name: &str) -> SqliteResult<()> {
    let now = now_timestamp();
    conn.execute(
        "UPDATE boards SET name = ?1, updated_at = ?2 WHERE id = ?3",
        rusqlite::params![name, now, board_id],
    )?;
    Ok(())
}

pub fn soft_delete_board(conn: &Connection, board_id: i64) -> SqliteResult<()> {
    let now = now_timestamp();
    conn.execute(
        "UPDATE boards SET deleted = 1, deleted_at = ?1, updated_at = ?1 WHERE id = ?2",
        rusqlite::params![now, board_id],
    )?;
    Ok(())
}

pub fn soft_delete_board_cascade(conn: &Connection, board_id: i64) -> SqliteResult<()> {
    let board_uuid: String = conn.query_row(
        "SELECT uuid FROM boards WHERE id = ?1",
        rusqlite::params![board_id],
        |row| row.get(0),
    )?;
    let now = now_timestamp();
    conn.execute(
        "UPDATE tasks SET deleted = 1, deleted_at = ?1, updated_at = ?1 WHERE board_id = ?2 AND deleted = 0",
        rusqlite::params![now, board_uuid],
    )?;
    soft_delete_board(conn, board_id)?;
    Ok(())
}

pub fn board_count(conn: &Connection) -> SqliteResult<i32> {
    conn.query_row("SELECT COUNT(*) FROM boards WHERE deleted = 0", [], |row| {
        row.get(0)
    })
}

pub fn upsert_board_from_sync(conn: &Connection, board: &rk_shared::SyncBoard) -> SqliteResult<()> {
    let existing: Option<(i64, Option<String>)> = conn
        .query_row(
            "SELECT id, updated_at FROM boards WHERE uuid = ?1",
            rusqlite::params![board.uuid],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )
        .ok();

    let deleted_at_val: Option<&str> = if board.deleted {
        Some(board.updated_at.as_str())
    } else {
        None
    };

    match existing {
        Some((_id, local_updated)) => {
            let dominated = local_updated
                .as_deref()
                .is_some_and(|lu| lu >= board.updated_at.as_str());
            if !dominated {
                conn.execute(
                    "UPDATE boards SET name=?1, position=?2, updated_at=?3, deleted=?4, deleted_at=?5 WHERE uuid=?6",
                    rusqlite::params![board.name, board.position, board.updated_at, board.deleted as i32, deleted_at_val, board.uuid],
                )?;
            }
        }
        None => {
            conn.execute(
                "INSERT INTO boards (uuid, name, position, created_at, updated_at, deleted, deleted_at) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
                rusqlite::params![board.uuid, board.name, board.position, board.updated_at, board.updated_at, board.deleted as i32, deleted_at_val],
            )?;
        }
    }
    Ok(())
}

pub fn reset_db(conn: &Connection) -> SqliteResult<()> {
    conn.execute_batch(
        "DELETE FROM task_tags; DELETE FROM tags; DELETE FROM tasks; DELETE FROM boards;",
    )?;

    let uuid = uuid::Uuid::new_v4().to_string();
    let now = now_timestamp();
    conn.execute(
        "INSERT INTO boards (uuid, name, position, created_at, updated_at) VALUES (?1, 'Personal', 0, ?2, ?3)",
        rusqlite::params![uuid, now, now],
    )?;
    conn.execute(
        "INSERT INTO preferences (key, value) VALUES ('active_board', ?1)
         ON CONFLICT(key) DO UPDATE SET value = ?1",
        rusqlite::params![uuid],
    )?;
    Ok(())
}

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

pub fn db_path() -> std::path::PathBuf {
    let data_dir = dirs::data_dir().expect("Could not determine data directory");
    data_dir.join("rustkanban").join("kanban.db")
}

/// Load all task->tag UUID mappings in a single query.
pub fn get_all_task_tag_uuids(
    conn: &Connection,
) -> SqliteResult<std::collections::HashMap<i64, Vec<String>>> {
    let mut stmt = conn
        .prepare("SELECT tt.task_id, t.uuid FROM tags t JOIN task_tags tt ON t.id = tt.tag_id")?;
    let mut map: std::collections::HashMap<i64, Vec<String>> = std::collections::HashMap::new();
    stmt.query_map([], |row| {
        Ok((row.get::<_, i64>(0)?, row.get::<_, String>(1)?))
    })?
    .for_each(|r| {
        if let Ok((task_id, tag_uuid)) = r {
            map.entry(task_id).or_default().push(tag_uuid);
        }
    });
    Ok(map)
}

#[allow(dead_code)]
pub fn get_task_tag_uuids(conn: &Connection, task_id: i64) -> SqliteResult<Vec<String>> {
    let mut stmt = conn.prepare(
        "SELECT t.uuid FROM tags t JOIN task_tags tt ON t.id = tt.tag_id WHERE tt.task_id = ?1",
    )?;
    let result = stmt
        .query_map(rusqlite::params![task_id], |row| row.get(0))?
        .collect::<SqliteResult<Vec<_>>>();
    result
}

#[allow(dead_code)]
pub fn upsert_task_from_sync(conn: &Connection, task: &rk_shared::SyncTask) -> SqliteResult<()> {
    let tag_map = tag_uuid_to_id_map(conn)?;
    upsert_task_from_sync_with_map(conn, task, &tag_map)
}

/// Batch-upsert tasks from sync using a shared tag map (avoids N+1 queries).
pub fn upsert_tasks_from_sync(
    conn: &Connection,
    tasks: &[rk_shared::SyncTask],
) -> SqliteResult<()> {
    let tag_map = tag_uuid_to_id_map(conn)?;
    for task in tasks {
        upsert_task_from_sync_with_map(conn, task, &tag_map)?;
    }
    Ok(())
}

fn upsert_task_from_sync_with_map(
    conn: &Connection,
    task: &rk_shared::SyncTask,
    tag_map: &std::collections::HashMap<String, i64>,
) -> SqliteResult<()> {
    let existing: Option<(i64, String)> = conn
        .query_row(
            "SELECT id, updated_at FROM tasks WHERE uuid = ?1",
            rusqlite::params![task.uuid],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )
        .ok();

    let board_id = task
        .board_uuid
        .as_deref()
        .filter(|s| !s.is_empty())
        .map(|s| s.to_string())
        .unwrap_or_else(|| default_board_uuid(conn).unwrap_or_default());

    match existing {
        Some((local_id, local_updated)) => {
            if task.updated_at > local_updated {
                conn.execute(
                    "UPDATE tasks SET title=?1, description=?2, priority=?3, column_name=?4, due_date=?5, updated_at=?6, deleted=?7, deleted_at=?8, board_id=?9 WHERE uuid=?10",
                    rusqlite::params![
                        task.title,
                        task.description,
                        task.priority,
                        task.column,
                        task.due_date,
                        task.updated_at,
                        task.deleted as i32,
                        if task.deleted { Some(&task.updated_at) } else { None },
                        board_id,
                        task.uuid
                    ],
                )?;
                update_task_tags_by_uuid(conn, local_id, &task.tags, tag_map)?;
            }
        }
        None => {
            conn.execute(
                "INSERT INTO tasks (uuid, title, description, priority, column_name, due_date, created_at, updated_at, deleted, deleted_at, board_id)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11)",
                rusqlite::params![
                    task.uuid,
                    task.title,
                    task.description,
                    task.priority,
                    task.column,
                    task.due_date,
                    task.created_at,
                    task.updated_at,
                    task.deleted as i32,
                    if task.deleted {
                        Some(&task.updated_at)
                    } else {
                        None
                    },
                    board_id
                ],
            )?;
            let local_id = conn.last_insert_rowid();
            update_task_tags_by_uuid(conn, local_id, &task.tags, tag_map)?;
        }
    }
    Ok(())
}

/// Build a uuid→id map for all tags (used by sync to avoid N+1 queries).
fn tag_uuid_to_id_map(conn: &Connection) -> SqliteResult<std::collections::HashMap<String, i64>> {
    let mut stmt = conn.prepare("SELECT uuid, id FROM tags")?;
    let map = stmt
        .query_map([], |row| {
            Ok((row.get::<_, String>(0)?, row.get::<_, i64>(1)?))
        })?
        .filter_map(|r| r.ok())
        .collect();
    Ok(map)
}

fn update_task_tags_by_uuid(
    conn: &Connection,
    task_id: i64,
    tag_uuids: &[String],
    tag_map: &std::collections::HashMap<String, i64>,
) -> SqliteResult<()> {
    conn.execute(
        "DELETE FROM task_tags WHERE task_id = ?1",
        rusqlite::params![task_id],
    )?;
    for tag_uuid in tag_uuids {
        if let Some(&tag_id) = tag_map.get(tag_uuid) {
            conn.execute(
                "INSERT OR IGNORE INTO task_tags (task_id, tag_id) VALUES (?1, ?2)",
                rusqlite::params![task_id, tag_id],
            )?;
        }
    }
    Ok(())
}

pub fn upsert_tag_from_sync(conn: &Connection, tag: &rk_shared::SyncTag) -> SqliteResult<()> {
    let existing: Option<(i64, Option<String>)> = conn
        .query_row(
            "SELECT id, updated_at FROM tags WHERE uuid = ?1",
            rusqlite::params![tag.uuid],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )
        .ok();

    let deleted_at_val: Option<&str> = if tag.deleted {
        Some(tag.updated_at.as_str())
    } else {
        None
    };

    match existing {
        Some((_id, local_updated)) => {
            // LWW: only update if remote is newer
            let dominated = local_updated
                .as_deref()
                .is_some_and(|lu| lu >= tag.updated_at.as_str());
            if !dominated {
                conn.execute(
                    "UPDATE tags SET name=?1, updated_at=?2, deleted=?3, deleted_at=?4 WHERE uuid=?5",
                    rusqlite::params![tag.name, tag.updated_at, tag.deleted as i32, deleted_at_val, tag.uuid],
                )?;
            }
        }
        None => {
            conn.execute(
                "INSERT INTO tags (uuid, name, updated_at, deleted, deleted_at) VALUES (?1, ?2, ?3, ?4, ?5)",
                rusqlite::params![tag.uuid, tag.name, tag.updated_at, tag.deleted as i32, deleted_at_val],
            )?;
        }
    }
    Ok(())
}

pub fn default_board_uuid(conn: &Connection) -> SqliteResult<String> {
    conn.query_row(
        "SELECT uuid FROM boards WHERE deleted = 0 ORDER BY position, id LIMIT 1",
        [],
        |row| row.get(0),
    )
}

#[allow(dead_code)]
pub fn remap_tag_uuid(conn: &Connection, old_uuid: &str, new_uuid: &str) -> SqliteResult<()> {
    conn.execute(
        "UPDATE tags SET uuid = ?1 WHERE uuid = ?2",
        rusqlite::params![new_uuid, old_uuid],
    )?;
    Ok(())
}

#[cfg(test)]
pub fn init_db_memory() -> Connection {
    let conn = Connection::open_in_memory().unwrap();
    conn.execute_batch("PRAGMA foreign_keys = ON;").unwrap();
    run_migrations(&conn).unwrap();
    conn
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::{Column, Priority};

    fn setup() -> Connection {
        init_db_memory()
    }

    fn test_board_uuid(conn: &Connection) -> String {
        default_board_uuid(conn).unwrap()
    }

    #[test]
    fn test_insert_and_load_task() {
        let conn = setup();
        let board = test_board_uuid(&conn);
        let id = insert_task(
            &conn,
            "Test",
            "Desc",
            Priority::High,
            Column::Todo,
            None,
            &board,
        )
        .unwrap();
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
        let board = test_board_uuid(&conn);
        let id = insert_task(&conn, "Old", "", Priority::Low, Column::Todo, None, &board).unwrap();
        update_task(&conn, id, "New", "desc", Priority::High, None).unwrap();
        let tasks = load_tasks(&conn).unwrap();
        assert_eq!(tasks[0].title, "New");
        assert_eq!(tasks[0].priority, Priority::High);
    }

    #[test]
    fn test_update_task_column() {
        let conn = setup();
        let board = test_board_uuid(&conn);
        let id = insert_task(&conn, "T", "", Priority::Medium, Column::Todo, None, &board).unwrap();
        update_task_column(&conn, id, Column::Done).unwrap();
        let tasks = load_tasks(&conn).unwrap();
        assert_eq!(tasks[0].column, Column::Done);
    }

    #[test]
    fn test_update_task_priority() {
        let conn = setup();
        let board = test_board_uuid(&conn);
        let id = insert_task(&conn, "T", "", Priority::Low, Column::Todo, None, &board).unwrap();
        update_task_priority(&conn, id, Priority::High).unwrap();
        let tasks = load_tasks(&conn).unwrap();
        assert_eq!(tasks[0].priority, Priority::High);
    }

    #[test]
    fn test_delete_task() {
        let conn = setup();
        let board = test_board_uuid(&conn);
        let id = insert_task(&conn, "T", "", Priority::Medium, Column::Todo, None, &board).unwrap();
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
        let board = test_board_uuid(&conn);
        let task_id =
            insert_task(&conn, "T", "", Priority::Medium, Column::Todo, None, &board).unwrap();
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
        let board = test_board_uuid(&conn);
        insert_task(&conn, "T", "", Priority::Medium, Column::Todo, None, &board).unwrap();
        insert_tag(&conn, "x").unwrap();
        reset_db(&conn).unwrap();
        assert!(load_tasks(&conn).unwrap().is_empty());
        assert!(load_tags(&conn).unwrap().is_empty());
    }

    #[test]
    fn test_preferences() {
        let conn = setup();
        assert_eq!(get_preference(&conn, "sort_mode"), None);
        set_preference(&conn, "sort_mode", "Priority").unwrap();
        assert_eq!(
            get_preference(&conn, "sort_mode"),
            Some("Priority".to_string())
        );
        set_preference(&conn, "sort_mode", "DueDate").unwrap();
        assert_eq!(
            get_preference(&conn, "sort_mode"),
            Some("DueDate".to_string())
        );
    }

    #[test]
    fn test_due_date_roundtrip() {
        let conn = setup();
        let board = test_board_uuid(&conn);
        let date = chrono::NaiveDate::from_ymd_opt(2026, 6, 15);
        insert_task(&conn, "T", "", Priority::Medium, Column::Todo, date, &board).unwrap();
        let tasks = load_tasks(&conn).unwrap();
        assert_eq!(tasks[0].due_date, date);
    }

    #[test]
    fn test_schema_version_tracking() {
        let conn = init_db_memory();
        let v = get_preference(&conn, "schema_version");
        assert_eq!(v, Some("3".to_string()));
    }

    #[test]
    fn test_tasks_have_uuids() {
        let conn = init_db_memory();
        let board = test_board_uuid(&conn);
        insert_task(&conn, "T", "", Priority::Medium, Column::Todo, None, &board).unwrap();
        let tasks = load_tasks(&conn).unwrap();
        assert!(!tasks[0].uuid.is_empty());
        assert_eq!(tasks[0].uuid.len(), 36);
    }

    #[test]
    fn test_tags_have_uuids() {
        let conn = init_db_memory();
        insert_tag(&conn, "bug").unwrap();
        let tags = load_tags(&conn).unwrap();
        assert!(!tags[0].uuid.is_empty());
    }

    #[test]
    fn test_load_tasks_excludes_deleted() {
        let conn = init_db_memory();
        let board = test_board_uuid(&conn);
        insert_task(
            &conn,
            "Active",
            "",
            Priority::Medium,
            Column::Todo,
            None,
            &board,
        )
        .unwrap();
        let id2 = insert_task(
            &conn,
            "Deleted",
            "",
            Priority::Low,
            Column::Todo,
            None,
            &board,
        )
        .unwrap();
        conn.execute(
            "UPDATE tasks SET deleted = 1, deleted_at = '2026-01-01T00:00:00' WHERE id = ?1",
            rusqlite::params![id2],
        )
        .unwrap();
        let tasks = load_tasks(&conn).unwrap();
        assert_eq!(tasks.len(), 1);
        assert_eq!(tasks[0].title, "Active");
    }

    #[test]
    fn test_load_all_tasks_includes_deleted() {
        let conn = init_db_memory();
        let board = test_board_uuid(&conn);
        insert_task(
            &conn,
            "Active",
            "",
            Priority::Medium,
            Column::Todo,
            None,
            &board,
        )
        .unwrap();
        let id2 = insert_task(
            &conn,
            "Deleted",
            "",
            Priority::Low,
            Column::Todo,
            None,
            &board,
        )
        .unwrap();
        conn.execute(
            "UPDATE tasks SET deleted = 1, deleted_at = '2026-01-01T00:00:00' WHERE id = ?1",
            rusqlite::params![id2],
        )
        .unwrap();
        let tasks = load_all_tasks(&conn).unwrap();
        assert_eq!(tasks.len(), 2);
    }

    #[test]
    fn test_soft_delete_task() {
        let conn = init_db_memory();
        let board = test_board_uuid(&conn);
        let id = insert_task(&conn, "T", "", Priority::Medium, Column::Todo, None, &board).unwrap();
        soft_delete_task(&conn, id).unwrap();
        assert!(load_tasks(&conn).unwrap().is_empty());
        assert_eq!(load_all_tasks(&conn).unwrap().len(), 1);
    }

    #[test]
    fn test_soft_delete_tag() {
        let conn = init_db_memory();
        let board = test_board_uuid(&conn);
        let tag_id = insert_tag(&conn, "bug").unwrap();
        let task_id =
            insert_task(&conn, "T", "", Priority::Medium, Column::Todo, None, &board).unwrap();
        set_task_tags(&conn, task_id, &[tag_id]).unwrap();
        soft_delete_tag(&conn, tag_id).unwrap();
        assert!(load_tags(&conn).unwrap().is_empty());
        assert!(get_task_tag_ids(&conn, task_id).unwrap().is_empty());
    }

    #[test]
    fn test_cleanup_old_soft_deletes() {
        let conn = init_db_memory();
        let board = test_board_uuid(&conn);
        let id = insert_task(&conn, "T", "", Priority::Medium, Column::Todo, None, &board).unwrap();
        let old_date = (chrono::Local::now() - chrono::Duration::days(31))
            .naive_local()
            .format(TS_FMT)
            .to_string();
        conn.execute(
            "UPDATE tasks SET deleted = 1, deleted_at = ?1 WHERE id = ?2",
            rusqlite::params![old_date, id],
        )
        .unwrap();
        cleanup_old_soft_deletes(&conn, 30).unwrap();
        assert!(load_all_tasks(&conn).unwrap().is_empty());
    }

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
            board_uuid: None,
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
        let board = test_board_uuid(&conn);
        insert_task(
            &conn,
            "Local",
            "",
            Priority::Medium,
            Column::Todo,
            None,
            &board,
        )
        .unwrap();
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
            updated_at: "2099-01-01T00:00:00".into(),
            deleted: false,
            board_uuid: None,
        };
        upsert_task_from_sync(&conn, &remote).unwrap();
        let tasks = load_tasks(&conn).unwrap();
        assert_eq!(tasks[0].title, "Server Version");
    }

    #[test]
    fn test_upsert_task_lww_local_wins() {
        let conn = init_db_memory();
        let board = test_board_uuid(&conn);
        insert_task(
            &conn,
            "Local",
            "",
            Priority::Medium,
            Column::Todo,
            None,
            &board,
        )
        .unwrap();
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
            updated_at: "2020-01-01T00:00:00".into(),
            deleted: false,
            board_uuid: None,
        };
        upsert_task_from_sync(&conn, &remote).unwrap();
        let tasks = load_tasks(&conn).unwrap();
        assert_eq!(tasks[0].title, "Local");
    }

    #[test]
    fn test_remap_tag_uuid() {
        let conn = init_db_memory();
        insert_tag(&conn, "bug").unwrap();
        let tags = load_tags(&conn).unwrap();
        let old_uuid = tags[0].uuid.clone();
        remap_tag_uuid(&conn, &old_uuid, "new-server-uuid").unwrap();
        let tags = load_tags(&conn).unwrap();
        assert_eq!(tags[0].uuid, "new-server-uuid");
    }

    #[test]
    fn test_load_boards() {
        let conn = setup();
        let boards = load_boards(&conn).unwrap();
        assert_eq!(boards.len(), 1);
        assert_eq!(boards[0].name, "Personal");
        assert_eq!(boards[0].position, 0);
    }

    #[test]
    fn test_insert_board() {
        let conn = setup();
        let id = insert_board(&conn, "Work").unwrap();
        assert!(id > 0);
        let boards = load_boards(&conn).unwrap();
        assert_eq!(boards.len(), 2);
        assert_eq!(boards[1].name, "Work");
        assert_eq!(boards[1].position, 1);
    }

    #[test]
    fn test_update_board_name() {
        let conn = setup();
        let boards = load_boards(&conn).unwrap();
        update_board_name(&conn, boards[0].id, "My Board").unwrap();
        let boards = load_boards(&conn).unwrap();
        assert_eq!(boards[0].name, "My Board");
    }

    #[test]
    fn test_soft_delete_board() {
        let conn = setup();
        insert_board(&conn, "Temp").unwrap();
        let boards = load_boards(&conn).unwrap();
        assert_eq!(boards.len(), 2);
        let temp = boards.iter().find(|b| b.name == "Temp").unwrap();
        soft_delete_board(&conn, temp.id).unwrap();
        let boards = load_boards(&conn).unwrap();
        assert_eq!(boards.len(), 1);
        assert_eq!(boards[0].name, "Personal");
    }

    #[test]
    fn test_board_count() {
        let conn = setup();
        assert_eq!(board_count(&conn).unwrap(), 1);
        insert_board(&conn, "Work").unwrap();
        assert_eq!(board_count(&conn).unwrap(), 2);
    }

    #[test]
    fn test_delete_board_cascades_tasks() {
        let conn = setup();
        let boards = load_boards(&conn).unwrap();
        let board_uuid = &boards[0].uuid;
        insert_task(
            &conn,
            "Task 1",
            "",
            Priority::Medium,
            Column::Todo,
            None,
            board_uuid,
        )
        .unwrap();
        assert_eq!(load_tasks(&conn).unwrap().len(), 1);
        insert_board(&conn, "Other").unwrap();
        soft_delete_board_cascade(&conn, boards[0].id).unwrap();
        let tasks = load_tasks(&conn).unwrap();
        assert_eq!(tasks.len(), 0);
    }

    #[test]
    fn test_load_all_boards_includes_deleted() {
        let conn = setup();
        insert_board(&conn, "Temp").unwrap();
        let boards = load_boards(&conn).unwrap();
        let temp = boards.iter().find(|b| b.name == "Temp").unwrap();
        soft_delete_board(&conn, temp.id).unwrap();
        let active = load_boards(&conn).unwrap();
        assert_eq!(active.len(), 1);
        let all = load_all_boards(&conn).unwrap();
        assert_eq!(all.len(), 2);
    }

    #[test]
    fn test_default_board_uuid() {
        let conn = setup();
        let uuid = default_board_uuid(&conn).unwrap();
        let boards = load_boards(&conn).unwrap();
        assert_eq!(uuid, boards[0].uuid);
    }
}
