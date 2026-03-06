use rusqlite::{Connection, Result as SqliteResult};

use crate::model::{Column, Priority, Tag, Task};

pub fn init_db(path: &std::path::Path) -> SqliteResult<Connection> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).expect("Failed to create database directory");
    }

    let conn = Connection::open(path)?;
    conn.execute_batch("PRAGMA foreign_keys = ON;")?;
    run_migrations(&conn)?;
    Ok(conn)
}

fn run_migrations(conn: &Connection) -> SqliteResult<()> {
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
        ",
    )?;
    Ok(())
}

pub fn load_tasks(conn: &Connection) -> SqliteResult<Vec<Task>> {
    let mut stmt = conn.prepare(
        "SELECT id, title, description, priority, column_name, due_date, created_at, updated_at
         FROM tasks",
    )?;

    let mut tasks = stmt
        .query_map([], |row| {
            let priority_str: String = row.get(3)?;
            let column_str: String = row.get(4)?;
            let due_date_str: Option<String> = row.get(5)?;
            let created_str: String = row.get(6)?;
            let updated_str: String = row.get(7)?;

            Ok(Task {
                id: row.get(0)?,
                title: row.get(1)?,
                description: row.get(2)?,
                priority: priority_str
                    .parse::<Priority>()
                    .unwrap_or(Priority::Medium),
                column: column_str.parse::<Column>().unwrap_or(Column::Todo),
                due_date: due_date_str
                    .and_then(|s| chrono::NaiveDate::parse_from_str(&s, "%Y-%m-%d").ok()),
                tags: Vec::new(),
                created_at: chrono::NaiveDateTime::parse_from_str(&created_str, "%Y-%m-%dT%H:%M:%S")
                    .unwrap_or_default(),
                updated_at: chrono::NaiveDateTime::parse_from_str(&updated_str, "%Y-%m-%dT%H:%M:%S")
                    .unwrap_or_default(),
            })
        })?
        .collect::<SqliteResult<Vec<_>>>()?;

    // Load tags for each task
    let mut tag_stmt = conn.prepare(
        "SELECT t.name FROM tags t
         JOIN task_tags tt ON t.id = tt.tag_id
         WHERE tt.task_id = ?1
         ORDER BY t.name",
    )?;

    for task in &mut tasks {
        let tag_names: Vec<String> = tag_stmt
            .query_map(rusqlite::params![task.id], |row| row.get(0))?
            .collect::<SqliteResult<Vec<_>>>()?;
        task.tags = tag_names;
    }

    Ok(tasks)
}

pub fn load_tags(conn: &Connection) -> SqliteResult<Vec<Tag>> {
    let mut stmt = conn.prepare("SELECT id, name FROM tags ORDER BY name")?;
    let tags = stmt
        .query_map([], |row| {
            Ok(Tag {
                id: row.get(0)?,
                name: row.get(1)?,
            })
        })?
        .collect::<SqliteResult<Vec<_>>>()?;
    Ok(tags)
}

pub fn insert_tag(conn: &Connection, name: &str) -> SqliteResult<i64> {
    conn.execute(
        "INSERT INTO tags (name) VALUES (?1)",
        rusqlite::params![name],
    )?;
    Ok(conn.last_insert_rowid())
}

pub fn delete_tag(conn: &Connection, tag_id: i64) -> SqliteResult<()> {
    conn.execute("DELETE FROM tags WHERE id = ?1", rusqlite::params![tag_id])?;
    Ok(())
}

pub fn rename_tag(conn: &Connection, tag_id: i64, new_name: &str) -> SqliteResult<()> {
    conn.execute(
        "UPDATE tags SET name = ?1 WHERE id = ?2",
        rusqlite::params![new_name, tag_id],
    )?;
    Ok(())
}

pub fn set_task_tags(conn: &Connection, task_id: i64, tag_ids: &[i64]) -> SqliteResult<()> {
    conn.execute(
        "DELETE FROM task_tags WHERE task_id = ?1",
        rusqlite::params![task_id],
    )?;
    let mut stmt = conn.prepare(
        "INSERT INTO task_tags (task_id, tag_id) VALUES (?1, ?2)",
    )?;
    for &tag_id in tag_ids {
        stmt.execute(rusqlite::params![task_id, tag_id])?;
    }
    Ok(())
}

pub fn get_task_tag_ids(conn: &Connection, task_id: i64) -> SqliteResult<Vec<i64>> {
    let mut stmt = conn.prepare(
        "SELECT tag_id FROM task_tags WHERE task_id = ?1",
    )?;
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
) -> SqliteResult<i64> {
    let now = chrono::Local::now().naive_local().format("%Y-%m-%dT%H:%M:%S").to_string();
    let due_date_str = due_date.map(|d| d.format("%Y-%m-%d").to_string());

    conn.execute(
        "INSERT INTO tasks (title, description, priority, column_name, due_date, created_at, updated_at)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
        rusqlite::params![
            title,
            description,
            priority.as_str(),
            column.as_str(),
            due_date_str,
            now,
            now,
        ],
    )?;

    Ok(conn.last_insert_rowid())
}

pub fn update_task_column(conn: &Connection, task_id: i64, column: Column) -> SqliteResult<()> {
    let now = chrono::Local::now().naive_local().format("%Y-%m-%dT%H:%M:%S").to_string();
    conn.execute(
        "UPDATE tasks SET column_name = ?1, updated_at = ?2 WHERE id = ?3",
        rusqlite::params![column.as_str(), now, task_id],
    )?;
    Ok(())
}

pub fn update_task_priority(conn: &Connection, task_id: i64, priority: Priority) -> SqliteResult<()> {
    let now = chrono::Local::now().naive_local().format("%Y-%m-%dT%H:%M:%S").to_string();
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
    let now = chrono::Local::now().naive_local().format("%Y-%m-%dT%H:%M:%S").to_string();
    let due_date_str = due_date.map(|d| d.format("%Y-%m-%d").to_string());
    conn.execute(
        "UPDATE tasks SET title = ?1, description = ?2, priority = ?3, due_date = ?4, updated_at = ?5 WHERE id = ?6",
        rusqlite::params![title, description, priority.as_str(), due_date_str, now, task_id],
    )?;
    Ok(())
}

pub fn delete_task(conn: &Connection, task_id: i64) -> SqliteResult<()> {
    conn.execute("DELETE FROM task_tags WHERE task_id = ?1", rusqlite::params![task_id])?;
    conn.execute("DELETE FROM tasks WHERE id = ?1", rusqlite::params![task_id])?;
    Ok(())
}

pub fn reset_db(conn: &Connection) -> SqliteResult<()> {
    conn.execute_batch(
        "DELETE FROM task_tags; DELETE FROM tags; DELETE FROM tasks;",
    )?;
    Ok(())
}

pub fn db_path() -> std::path::PathBuf {
    let data_dir = dirs::data_dir().expect("Could not determine data directory");
    data_dir.join("rustkanban").join("kanban.db")
}
