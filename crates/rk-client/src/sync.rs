use rusqlite::Connection;

use crate::auth::{self, Credentials};
use crate::db::{self, TS_FMT};
use rk_shared::{SyncBoard, SyncPayload, SyncResponse, SyncTag, SyncTask};

#[derive(Debug)]
pub enum SyncError {
    NotLoggedIn,
    Network(String),
    AuthExpired,
    AccountNotFound,
    ServerError(String),
    Other(String),
}

impl From<rusqlite::Error> for SyncError {
    fn from(e: rusqlite::Error) -> Self {
        SyncError::Other(e.to_string())
    }
}

impl std::fmt::Display for SyncError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SyncError::NotLoggedIn => write!(f, "Not logged in"),
            SyncError::Network(e) => write!(f, "Sync failed — working offline ({})", e),
            SyncError::AuthExpired => {
                write!(f, "Session expired — run `rk login` to re-authenticate")
            }
            SyncError::AccountNotFound => {
                write!(
                    f,
                    "Account not found — run `rk logout` to clear credentials"
                )
            }
            SyncError::ServerError(e) => write!(f, "Server error — try again later ({})", e),
            SyncError::Other(e) => write!(f, "Sync error: {}", e),
        }
    }
}

/// Pull changes from server
#[allow(dead_code)]
pub fn pull(conn: &Connection) -> Result<String, SyncError> {
    let creds = auth::load_credentials().ok_or(SyncError::NotLoggedIn)?;

    let payload = SyncPayload {
        tasks: vec![],
        tags: vec![],
        boards: vec![],
        last_synced_at: creds.last_synced_at.clone(),
    };

    let response = post_sync(&creds, "/api/v1/sync/pull", &payload)?;
    apply_pull_response(conn, &response)?;

    save_last_synced(&response.synced_at);
    Ok(response.synced_at)
}

/// Push local changes to server
#[allow(dead_code)]
pub fn push(conn: &Connection) -> Result<String, SyncError> {
    let creds = auth::load_credentials().ok_or(SyncError::NotLoggedIn)?;

    let payload = build_push_payload(conn, &creds)?;
    let response = post_sync(&creds, "/api/v1/sync/push", &payload)?;
    apply_pull_response(conn, &response)?;

    save_last_synced(&response.synced_at);
    Ok(response.synced_at)
}

/// Combined pull + push in one round trip
pub fn sync(conn: &Connection) -> Result<String, SyncError> {
    let creds = auth::load_credentials().ok_or(SyncError::NotLoggedIn)?;

    let payload = build_push_payload(conn, &creds)?;
    let response = post_sync(&creds, "/api/v1/sync", &payload)?;

    apply_pull_response(conn, &response)?;
    apply_tag_uuid_mappings(conn, &response.tag_uuid_mappings)?;

    save_last_synced(&response.synced_at);
    Ok(response.synced_at)
}

fn save_last_synced(synced_at: &str) {
    if let Err(e) = auth::update_last_synced(synced_at) {
        eprintln!("Warning: failed to save sync timestamp: {}", e);
    }
}

fn build_push_payload(conn: &Connection, creds: &Credentials) -> Result<SyncPayload, SyncError> {
    let all_tasks = db::load_all_tasks(conn)?;
    let all_tags = db::load_all_tags(conn)?;
    let all_boards = db::load_all_boards(conn)?;
    let all_tag_uuids = db::get_all_task_tag_uuids(conn)?;

    let tasks: Vec<SyncTask> = if let Some(ref last) = creds.last_synced_at {
        let cutoff = chrono::NaiveDateTime::parse_from_str(last, TS_FMT).unwrap_or_default();
        all_tasks
            .iter()
            .filter(|t| t.updated_at > cutoff)
            .map(|t| task_to_sync(t, &all_tag_uuids))
            .collect()
    } else {
        all_tasks
            .iter()
            .map(|t| task_to_sync(t, &all_tag_uuids))
            .collect()
    };

    let tags: Vec<SyncTag> = all_tags.iter().map(tag_to_sync).collect();
    let boards: Vec<SyncBoard> = all_boards.iter().map(board_to_sync).collect();

    Ok(SyncPayload {
        tasks,
        tags,
        boards,
        last_synced_at: creds.last_synced_at.clone(),
    })
}

fn task_to_sync(
    task: &crate::model::Task,
    all_tag_uuids: &std::collections::HashMap<i64, Vec<String>>,
) -> SyncTask {
    let tag_uuids = all_tag_uuids.get(&task.id).cloned().unwrap_or_default();
    SyncTask {
        uuid: task.uuid.clone(),
        title: task.title.clone(),
        description: task.description.clone(),
        priority: task.priority.as_str().to_string(),
        column: task.column.as_str().to_string(),
        due_date: task.due_date.map(|d| d.format("%Y-%m-%d").to_string()),
        tags: tag_uuids,
        created_at: task.created_at.format(TS_FMT).to_string(),
        updated_at: task.updated_at.format(TS_FMT).to_string(),
        deleted: task.deleted,
        board_uuid: if task.board_id.is_empty() {
            None
        } else {
            Some(task.board_id.clone())
        },
    }
}

fn tag_to_sync(tag: &crate::model::Tag) -> SyncTag {
    SyncTag {
        uuid: tag.uuid.clone(),
        name: tag.name.clone(),
        updated_at: tag.updated_at.format(TS_FMT).to_string(),
        deleted: tag.deleted,
    }
}

fn board_to_sync(board: &crate::model::Board) -> SyncBoard {
    SyncBoard {
        uuid: board.uuid.clone(),
        name: board.name.clone(),
        position: board.position,
        updated_at: board.updated_at.format(TS_FMT).to_string(),
        deleted: board.deleted,
    }
}

fn post_sync(
    creds: &Credentials,
    path: &str,
    payload: &SyncPayload,
) -> Result<SyncResponse, SyncError> {
    let url = format!("{}{}", creds.server_url, path);
    let body = serde_json::to_string(payload).map_err(|e| SyncError::Other(e.to_string()))?;

    let result = ureq::post(&url)
        .header("Authorization", &format!("Bearer {}", creds.token))
        .header("Content-Type", "application/json")
        .send(body.as_bytes());

    match result {
        Ok(mut resp) => {
            let text = resp
                .body_mut()
                .read_to_string()
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
    // Boards first (tasks reference them)
    for board in &response.boards {
        db::upsert_board_from_sync(conn, board)?;
    }

    // Tags second
    for tag in &response.tags {
        db::upsert_tag_from_sync(conn, tag)?;
    }

    // Then tasks
    db::upsert_tasks_from_sync(conn, &response.tasks)?;

    Ok(())
}

fn apply_tag_uuid_mappings(
    conn: &Connection,
    mappings: &std::collections::HashMap<String, String>,
) -> Result<(), SyncError> {
    for (old_uuid, new_uuid) in mappings {
        db::remap_tag_uuid(conn, old_uuid, new_uuid)?;
    }
    Ok(())
}
