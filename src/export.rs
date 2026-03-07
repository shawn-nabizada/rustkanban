use std::collections::HashMap;
use std::collections::HashSet;

use serde::{Deserialize, Serialize};

use crate::db;
use crate::model::{Column, Priority};

#[derive(Serialize, Deserialize)]
struct ExportData {
    version: u32,
    tasks: Vec<ExportTask>,
    tags: Vec<String>,
}

#[derive(Serialize, Deserialize)]
struct ExportTask {
    title: String,
    description: String,
    priority: String,
    column: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    due_date: Option<String>,
    tags: Vec<String>,
}

pub fn export_json(conn: &rusqlite::Connection) -> Result<String, Box<dyn std::error::Error>> {
    let tasks = db::load_tasks(conn)?;
    let tags = db::load_tags(conn)?;

    let data = ExportData {
        version: 1,
        tasks: tasks
            .iter()
            .map(|t| ExportTask {
                title: t.title.clone(),
                description: t.description.clone(),
                priority: t.priority.as_str().to_string(),
                column: t.column.as_str().to_string(),
                due_date: t.due_date.map(|d| d.format("%Y-%m-%d").to_string()),
                tags: t.tags.clone(),
            })
            .collect(),
        tags: tags.iter().map(|t| t.name.clone()).collect(),
    };

    Ok(serde_json::to_string_pretty(&data)?)
}

pub fn import_json(
    conn: &rusqlite::Connection,
    json: &str,
) -> Result<usize, Box<dyn std::error::Error>> {
    let data: ExportData = serde_json::from_str(json)?;

    // Collect all referenced tag names
    let mut all_tag_names: HashSet<String> = data.tags.iter().cloned().collect();
    for task in &data.tasks {
        for tag_name in &task.tags {
            all_tag_names.insert(tag_name.clone());
        }
    }

    // Create missing tags
    let existing_tags = db::load_tags(conn)?;
    let existing_names: HashSet<String> = existing_tags.iter().map(|t| t.name.clone()).collect();
    for name in &all_tag_names {
        if !existing_names.contains(name) {
            db::insert_tag(conn, name)?;
        }
    }

    // Build tag name to ID map
    let all_tags = db::load_tags(conn)?;
    let tag_map: HashMap<&str, i64> = all_tags.iter().map(|t| (t.name.as_str(), t.id)).collect();

    let mut count = 0;
    for task in &data.tasks {
        let priority: Priority = task.priority.parse().unwrap_or(Priority::Medium);
        let column: Column = task.column.parse().unwrap_or(Column::Todo);
        let due_date = task
            .due_date
            .as_ref()
            .and_then(|s| chrono::NaiveDate::parse_from_str(s, "%Y-%m-%d").ok());

        let task_id = db::insert_task(
            conn,
            &task.title,
            &task.description,
            priority,
            column,
            due_date,
        )?;

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
        assert!(json.contains("\"version\": 1"));
    }

    #[test]
    fn test_export_import_roundtrip() {
        let conn = crate::db::init_db_memory();
        crate::db::insert_tag(&conn, "bug").unwrap();
        let task_id = crate::db::insert_task(
            &conn,
            "Fix it",
            "desc",
            Priority::High,
            crate::model::Column::InProgress,
            None,
        )
        .unwrap();
        let tag_ids = crate::db::load_tags(&conn).unwrap();
        crate::db::set_task_tags(&conn, task_id, &[tag_ids[0].id]).unwrap();

        let json = export_json(&conn).unwrap();

        // Import into fresh DB
        let conn2 = crate::db::init_db_memory();
        let count = import_json(&conn2, &json).unwrap();
        assert_eq!(count, 1);

        let tasks = crate::db::load_tasks(&conn2).unwrap();
        assert_eq!(tasks[0].title, "Fix it");
        assert_eq!(tasks[0].priority, Priority::High);
        assert_eq!(tasks[0].tags, vec!["bug"]);
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
}
