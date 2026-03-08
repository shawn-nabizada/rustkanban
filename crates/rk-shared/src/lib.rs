use serde::{Deserialize, Serialize};
use std::collections::HashMap;

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
