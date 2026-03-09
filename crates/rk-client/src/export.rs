use std::collections::HashMap;
use std::collections::HashSet;

use serde::{Deserialize, Serialize};

use crate::db;
use crate::model::{Column, Priority};

#[derive(Serialize, Deserialize)]
struct ExportData {
    version: u32,
    tasks: Vec<ExportTask>,
    tags: Vec<ExportTagEntry>,
    #[serde(default)]
    boards: Vec<ExportBoard>,
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
    #[serde(default)]
    board_uuid: Option<String>,
}

#[derive(Serialize, Deserialize)]
struct ExportBoard {
    uuid: String,
    name: String,
    position: i32,
}

#[derive(Serialize, Deserialize, Clone)]
struct ExportTag {
    #[serde(default)]
    uuid: Option<String>,
    name: String,
}

#[derive(Serialize, Deserialize, Clone)]
#[serde(untagged)]
enum ExportTagEntry {
    Full(ExportTag),
    Plain(String),
}

impl ExportTagEntry {
    fn name(&self) -> &str {
        match self {
            ExportTagEntry::Full(tag) => &tag.name,
            ExportTagEntry::Plain(name) => name,
        }
    }

    fn uuid(&self) -> Option<&str> {
        match self {
            ExportTagEntry::Full(tag) => tag.uuid.as_deref(),
            ExportTagEntry::Plain(_) => None,
        }
    }
}

pub fn export_json(conn: &rusqlite::Connection) -> Result<String, Box<dyn std::error::Error>> {
    let tasks = db::load_tasks(conn)?;
    let tags = db::load_tags(conn)?;
    let boards = db::load_boards(conn)?;

    let data = ExportData {
        version: 2,
        tasks: tasks
            .iter()
            .map(|t| ExportTask {
                uuid: Some(t.uuid.clone()),
                title: t.title.clone(),
                description: t.description.clone(),
                priority: t.priority.as_str().to_string(),
                column: t.column.as_str().to_string(),
                due_date: t.due_date.map(|d| d.format("%Y-%m-%d").to_string()),
                tags: t.tags.clone(),
                board_uuid: Some(t.board_id.clone()),
            })
            .collect(),
        tags: tags
            .iter()
            .map(|t| {
                ExportTagEntry::Full(ExportTag {
                    uuid: Some(t.uuid.clone()),
                    name: t.name.clone(),
                })
            })
            .collect(),
        boards: boards
            .iter()
            .map(|b| ExportBoard {
                uuid: b.uuid.clone(),
                name: b.name.clone(),
                position: b.position,
            })
            .collect(),
    };

    Ok(serde_json::to_string_pretty(&data)?)
}

pub fn import_json(
    conn: &rusqlite::Connection,
    json: &str,
) -> Result<usize, Box<dyn std::error::Error>> {
    let data: ExportData = serde_json::from_str(json)?;

    // Import boards (create missing ones)
    {
        let existing_boards = db::load_boards(conn)?;
        for board in &data.boards {
            if existing_boards.iter().any(|b| b.uuid == board.uuid) {
                continue;
            }
            let mut name = board.name.clone();
            if existing_boards.iter().any(|b| b.name == name) {
                name = format!("{} (2)", name);
            }
            let _ = db::insert_board_with_uuid(conn, &board.uuid, &name, board.position);
        }
    }

    // Collect all referenced tag names (from the top-level tags list and from task tags)
    let mut all_tag_names: HashSet<String> =
        data.tags.iter().map(|t| t.name().to_string()).collect();
    for task in &data.tasks {
        for tag_name in &task.tags {
            all_tag_names.insert(tag_name.clone());
        }
    }

    // Create missing tags, respecting UUIDs where provided
    {
        let existing_tags = db::load_tags(conn)?;
        let existing_names: HashSet<String> =
            existing_tags.iter().map(|t| t.name.clone()).collect();

        // Process top-level tag entries (which may have UUIDs)
        for entry in &data.tags {
            let name = entry.name();
            if existing_names.contains(name) {
                continue;
            }
            if let Some(uuid) = entry.uuid() {
                if db::tag_uuid_exists(conn, uuid)? {
                    continue;
                }
                db::insert_tag_with_uuid(conn, uuid, name)?;
            } else {
                db::insert_tag(conn, name)?;
            }
        }
    }

    // Create any tags referenced by tasks but not in the top-level tags list
    let mut tags_after = db::load_tags(conn)?;
    {
        let names_after: HashSet<&str> = tags_after.iter().map(|t| t.name.as_str()).collect();
        let mut inserted = false;
        for name in &all_tag_names {
            if !names_after.contains(name.as_str()) {
                db::insert_tag(conn, name)?;
                inserted = true;
            }
        }
        if inserted {
            tags_after = db::load_tags(conn)?;
        }
    }

    // Build tag name->ID map
    let tag_map: HashMap<&str, i64> = tags_after.iter().map(|t| (t.name.as_str(), t.id)).collect();

    let default_board = db::default_board_uuid(conn).unwrap_or_default();

    let mut count = 0;
    for task in &data.tasks {
        // If the task has a UUID, check if it already exists locally
        if let Some(ref uuid) = task.uuid {
            if db::task_uuid_exists(conn, uuid)? {
                // Skip tasks with UUIDs that already exist
                continue;
            }
        }

        let priority: Priority = task.priority.parse().unwrap_or(Priority::Medium);
        let column: Column = task.column.parse().unwrap_or(Column::Todo);
        let due_date = task
            .due_date
            .as_ref()
            .and_then(|s| chrono::NaiveDate::parse_from_str(s, "%Y-%m-%d").ok());

        let board_id = task
            .board_uuid
            .as_deref()
            .filter(|s| !s.is_empty())
            .unwrap_or(&default_board);

        let task_id = if let Some(ref uuid) = task.uuid {
            db::insert_task_with_uuid(
                conn,
                uuid,
                &task.title,
                &task.description,
                priority,
                column,
                due_date,
                board_id,
            )?
        } else {
            db::insert_task(
                conn,
                &task.title,
                &task.description,
                priority,
                column,
                due_date,
                board_id,
            )?
        };

        let tag_ids: Vec<i64> = task
            .tags
            .iter()
            .filter_map(|name| tag_map.get(name.as_str()).copied())
            .collect();
        if !tag_ids.is_empty() {
            db::set_task_tags(conn, task_id, &tag_ids)?;
        }

        count += 1;
    }

    Ok(count)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::Priority;

    #[test]
    fn test_export_empty() {
        let conn = crate::db::init_db_memory();
        let json = export_json(&conn).unwrap();
        assert!(json.contains("\"tasks\": []"));
        assert!(json.contains("\"version\": 2"));
        // init_db_memory creates a default board, so boards should not be empty
        assert!(json.contains("\"boards\""));
    }

    #[test]
    fn test_export_import_roundtrip() {
        let conn = crate::db::init_db_memory();
        let board = crate::db::default_board_uuid(&conn).unwrap();
        crate::db::insert_tag(&conn, "bug").unwrap();
        let task_id = crate::db::insert_task(
            &conn,
            "Fix it",
            "desc",
            Priority::High,
            crate::model::Column::InProgress,
            None,
            &board,
        )
        .unwrap();
        let tag_ids = crate::db::load_tags(&conn).unwrap();
        crate::db::set_task_tags(&conn, task_id, &[tag_ids[0].id]).unwrap();

        let json = export_json(&conn).unwrap();
        assert!(json.contains("\"version\": 2"));
        assert!(json.contains("\"uuid\""));
        assert!(json.contains("\"board_uuid\""));
        assert!(json.contains("\"boards\""));

        // Import into fresh DB
        let conn2 = crate::db::init_db_memory();
        let count = import_json(&conn2, &json).unwrap();
        assert_eq!(count, 1);

        let tasks = crate::db::load_tasks(&conn2).unwrap();
        assert_eq!(tasks[0].title, "Fix it");
        assert_eq!(tasks[0].priority, Priority::High);
        assert_eq!(tasks[0].tags, vec!["bug"]);

        // Verify boards were imported
        let boards = crate::db::load_boards(&conn2).unwrap();
        assert!(!boards.is_empty());
    }

    #[test]
    fn test_import_deduplicates_tags() {
        let conn = crate::db::init_db_memory();
        crate::db::insert_tag(&conn, "existing").unwrap();

        let json = r#"{"version":1,"tasks":[],"tags":["existing","new"]}"#;
        import_json(&conn, json).unwrap();

        let tags = crate::db::load_tags(&conn).unwrap();
        assert_eq!(tags.len(), 2);
    }

    #[test]
    fn test_export_v2_has_uuids() {
        let conn = crate::db::init_db_memory();
        let board = crate::db::default_board_uuid(&conn).unwrap();
        crate::db::insert_task(&conn, "T", "", Priority::Medium, Column::Todo, None, &board)
            .unwrap();
        crate::db::insert_tag(&conn, "bug").unwrap();
        let json = export_json(&conn).unwrap();
        assert!(json.contains("\"version\": 2"));
        assert!(json.contains("\"uuid\""));
        assert!(json.contains("\"board_uuid\""));
    }

    #[test]
    fn test_import_v1_backward_compatible() {
        let conn = crate::db::init_db_memory();
        let json = r#"{"version":1,"tasks":[{"title":"T","description":"","priority":"Medium","column":"todo","tags":[]}],"tags":["bug"]}"#;
        let count = import_json(&conn, json).unwrap();
        assert_eq!(count, 1);
        let tasks = crate::db::load_tasks(&conn).unwrap();
        assert!(!tasks[0].uuid.is_empty());
    }

    #[test]
    fn test_import_v2_skips_existing_uuid() {
        let conn = crate::db::init_db_memory();
        let board = crate::db::default_board_uuid(&conn).unwrap();
        crate::db::insert_task(
            &conn,
            "Original",
            "",
            Priority::Medium,
            Column::Todo,
            None,
            &board,
        )
        .unwrap();
        let tasks = crate::db::load_tasks(&conn).unwrap();
        let uuid = tasks[0].uuid.clone();

        let json = format!(
            r#"{{"version":2,"tasks":[{{"uuid":"{}","title":"Duplicate","description":"","priority":"Medium","column":"todo","tags":[]}}],"tags":[]}}"#,
            uuid
        );
        let count = import_json(&conn, &json).unwrap();
        assert_eq!(count, 0);
        let tasks = crate::db::load_tasks(&conn).unwrap();
        assert_eq!(tasks.len(), 1);
        assert_eq!(tasks[0].title, "Original");
    }
}
