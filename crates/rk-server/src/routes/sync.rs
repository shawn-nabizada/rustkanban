use std::collections::HashMap;

use axum::{Extension, Json};
use chrono::NaiveDate;
use rk_shared::{SyncBoard, SyncPayload, SyncResponse, SyncTag, SyncTask};
use sqlx::postgres::PgConnection;
use sqlx::PgPool;
use uuid::Uuid;

use crate::auth::AuthUser;
use crate::error::AppError;

// Timestamp format used for all to_char operations.
const TS_FMT: &str = "YYYY-MM-DD\"T\"HH24:MI:SS";

// ───────────────────────────── Result type for tag dedup ─────────────────────

enum TagResult {
    Accepted,
    Deduped {
        client_uuid: String,
        server_uuid: String,
    },
}

// ───────────────────────────── Public handlers ───────────────────────────────

/// `POST /api/v1/sync/pull` — return server-side changes since `last_synced_at`.
pub async fn pull(
    auth: AuthUser,
    Extension(pool): Extension<PgPool>,
    Json(payload): Json<SyncPayload>,
) -> Result<Json<SyncResponse>, AppError> {
    let device_id = auth.device_id.ok_or(AppError::Validation(
        "Sync requires a device token, not an API token".into(),
    ))?;

    let full_pull = should_full_pull(&pool, device_id, &payload).await?;

    let tasks = if full_pull {
        fetch_all_tasks(&pool, auth.user_id).await?
    } else {
        #[allow(clippy::unnecessary_unwrap)]
        fetch_tasks_since(
            &pool,
            auth.user_id,
            payload.last_synced_at.as_ref().unwrap(),
        )
        .await?
    };

    let tags = if full_pull {
        fetch_all_tags(&pool, auth.user_id).await?
    } else {
        #[allow(clippy::unnecessary_unwrap)]
        fetch_tags_since(
            &pool,
            auth.user_id,
            payload.last_synced_at.as_ref().unwrap(),
        )
        .await?
    };

    let boards = if full_pull {
        fetch_all_boards(&pool, auth.user_id).await?
    } else {
        #[allow(clippy::unnecessary_unwrap)]
        fetch_boards_since(
            &pool,
            auth.user_id,
            payload.last_synced_at.as_ref().unwrap(),
        )
        .await?
    };

    // Update device
    sqlx::query("UPDATE devices SET last_synced_at = NOW(), stale = FALSE WHERE id = $1")
        .bind(device_id)
        .execute(&pool)
        .await?;

    let synced_at = chrono::Utc::now().format("%Y-%m-%dT%H:%M:%S").to_string();

    Ok(Json(SyncResponse {
        tasks,
        tags,
        boards,
        tag_uuid_mappings: HashMap::new(),
        synced_at,
    }))
}

/// `POST /api/v1/sync/push` — accept client changes, return conflicts.
#[allow(clippy::explicit_auto_deref)]
pub async fn push(
    auth: AuthUser,
    Extension(pool): Extension<PgPool>,
    Json(payload): Json<SyncPayload>,
) -> Result<Json<SyncResponse>, AppError> {
    let device_id = auth.device_id.ok_or(AppError::Validation(
        "Sync requires a device token, not an API token".into(),
    ))?;

    validate_limits(&pool, auth.user_id, &payload).await?;

    let mut tx = pool.begin().await?;
    let mut tag_uuid_mappings: HashMap<String, String> = HashMap::new();
    let mut rejected_boards: Vec<SyncBoard> = Vec::new();
    let mut rejected_tags: Vec<SyncTag> = Vec::new();
    let mut rejected_tasks: Vec<SyncTask> = Vec::new();

    // 1. Process boards first (tasks reference boards)
    let boards_rejected = process_boards(&mut *tx, auth.user_id, &payload.boards).await?;
    rejected_boards.extend(boards_rejected);

    // 2. Process tags (name dedup may produce mappings)
    for tag in &payload.tags {
        match process_push_tag(&mut *tx, auth.user_id, tag).await? {
            TagResult::Accepted => {}
            TagResult::Deduped {
                client_uuid,
                server_uuid,
            } => {
                // Return the server tag the client should adopt
                if let Some(st) = fetch_tag_by_uuid(&mut *tx, auth.user_id, &server_uuid).await? {
                    if !rejected_tags.iter().any(|t| t.uuid == st.uuid) {
                        rejected_tags.push(st);
                    }
                }
                tag_uuid_mappings.insert(client_uuid, server_uuid);
            }
        }
    }

    // 3. Process tasks (remapping any deduped tag UUIDs)
    for task in &payload.tasks {
        let remapped_tags: Vec<String> = task
            .tags
            .iter()
            .map(|t| {
                tag_uuid_mappings
                    .get(t)
                    .cloned()
                    .unwrap_or_else(|| t.clone())
            })
            .collect();
        if let Some(server_task) =
            process_push_task(&mut *tx, auth.user_id, task, &remapped_tags).await?
        {
            rejected_tasks.push(server_task);
        }
    }

    // Update device
    sqlx::query("UPDATE devices SET last_synced_at = NOW(), stale = FALSE WHERE id = $1")
        .bind(device_id)
        .execute(&mut *tx)
        .await?;

    tx.commit().await?;

    let synced_at = chrono::Utc::now().format("%Y-%m-%dT%H:%M:%S").to_string();

    Ok(Json(SyncResponse {
        tasks: rejected_tasks,
        tags: rejected_tags,
        boards: rejected_boards,
        tag_uuid_mappings,
        synced_at,
    }))
}

/// `POST /api/v1/sync` — combined pull-then-push in a single round-trip.
#[allow(clippy::explicit_auto_deref)]
pub async fn combined(
    auth: AuthUser,
    Extension(pool): Extension<PgPool>,
    Json(payload): Json<SyncPayload>,
) -> Result<Json<SyncResponse>, AppError> {
    let device_id = auth.device_id.ok_or(AppError::Validation(
        "Sync requires a device token, not an API token".into(),
    ))?;

    validate_limits(&pool, auth.user_id, &payload).await?;

    let mut tx = pool.begin().await?;

    // ── Pull phase ──────────────────────────────────────────────────────

    let full_pull = should_full_pull_conn(&mut *tx, device_id, &payload).await?;

    let mut response_tasks = if full_pull {
        fetch_all_tasks_conn(&mut *tx, auth.user_id).await?
    } else {
        #[allow(clippy::unnecessary_unwrap)]
        fetch_tasks_since_conn(
            &mut *tx,
            auth.user_id,
            payload.last_synced_at.as_ref().unwrap(),
        )
        .await?
    };

    let mut response_tags = if full_pull {
        fetch_all_tags_conn(&mut *tx, auth.user_id).await?
    } else {
        #[allow(clippy::unnecessary_unwrap)]
        fetch_tags_since_conn(
            &mut *tx,
            auth.user_id,
            payload.last_synced_at.as_ref().unwrap(),
        )
        .await?
    };

    let mut response_boards = if full_pull {
        fetch_all_boards_conn(&mut *tx, auth.user_id).await?
    } else {
        #[allow(clippy::unnecessary_unwrap)]
        fetch_boards_since_conn(
            &mut *tx,
            auth.user_id,
            payload.last_synced_at.as_ref().unwrap(),
        )
        .await?
    };

    // ── Push phase ──────────────────────────────────────────────────────

    let mut tag_uuid_mappings: HashMap<String, String> = HashMap::new();

    // Process boards first (tasks reference boards)
    let boards_rejected = process_boards(&mut *tx, auth.user_id, &payload.boards).await?;
    for board in boards_rejected {
        if !response_boards.iter().any(|b| b.uuid == board.uuid) {
            response_boards.push(board);
        }
    }

    for tag in &payload.tags {
        match process_push_tag(&mut *tx, auth.user_id, tag).await? {
            TagResult::Accepted => {}
            TagResult::Deduped {
                client_uuid,
                server_uuid,
            } => {
                if let Some(st) = fetch_tag_by_uuid(&mut *tx, auth.user_id, &server_uuid).await? {
                    if !response_tags.iter().any(|t| t.uuid == st.uuid) {
                        response_tags.push(st);
                    }
                }
                tag_uuid_mappings.insert(client_uuid, server_uuid);
            }
        }
    }

    for task in &payload.tasks {
        let remapped_tags: Vec<String> = task
            .tags
            .iter()
            .map(|t| {
                tag_uuid_mappings
                    .get(t)
                    .cloned()
                    .unwrap_or_else(|| t.clone())
            })
            .collect();
        if let Some(server_task) =
            process_push_task(&mut *tx, auth.user_id, task, &remapped_tags).await?
        {
            if !response_tasks.iter().any(|t| t.uuid == server_task.uuid) {
                response_tasks.push(server_task);
            }
        }
    }

    // Update device
    sqlx::query("UPDATE devices SET last_synced_at = NOW(), stale = FALSE WHERE id = $1")
        .bind(device_id)
        .execute(&mut *tx)
        .await?;

    tx.commit().await?;

    let synced_at = chrono::Utc::now().format("%Y-%m-%dT%H:%M:%S").to_string();

    Ok(Json(SyncResponse {
        tasks: response_tasks,
        tags: response_tags,
        boards: response_boards,
        tag_uuid_mappings,
        synced_at,
    }))
}

// ───────────────────────────── Stale-device check ────────────────────────────

async fn should_full_pull(
    pool: &PgPool,
    device_id: Uuid,
    payload: &SyncPayload,
) -> Result<bool, AppError> {
    if payload.last_synced_at.is_none() {
        return Ok(true);
    }
    let stale: bool = sqlx::query_scalar("SELECT stale FROM devices WHERE id = $1")
        .bind(device_id)
        .fetch_one(pool)
        .await
        .unwrap_or(false);
    Ok(stale)
}

async fn should_full_pull_conn(
    conn: &mut PgConnection,
    device_id: Uuid,
    payload: &SyncPayload,
) -> Result<bool, AppError> {
    if payload.last_synced_at.is_none() {
        return Ok(true);
    }
    let stale: bool = sqlx::query_scalar("SELECT stale FROM devices WHERE id = $1")
        .bind(device_id)
        .fetch_one(&mut *conn)
        .await
        .unwrap_or(false);
    Ok(stale)
}

// ───────────────────────────── Validation ─────────────────────────────────────

async fn validate_limits(
    pool: &PgPool,
    user_id: Uuid,
    _payload: &SyncPayload,
) -> Result<(), AppError> {
    let task_count: i64 =
        sqlx::query_scalar("SELECT COUNT(*) FROM tasks WHERE user_id = $1 AND deleted = FALSE")
            .bind(user_id)
            .fetch_one(pool)
            .await?;
    if task_count > 200 {
        return Err(AppError::Validation("Task limit exceeded (max 200)".into()));
    }

    let tag_count: i64 =
        sqlx::query_scalar("SELECT COUNT(*) FROM tags WHERE user_id = $1 AND deleted = FALSE")
            .bind(user_id)
            .fetch_one(pool)
            .await?;
    if tag_count > 15 {
        return Err(AppError::Validation("Tag limit exceeded (max 15)".into()));
    }

    let board_count: i64 =
        sqlx::query_scalar("SELECT COUNT(*) FROM boards WHERE user_id = $1 AND deleted = FALSE")
            .bind(user_id)
            .fetch_one(pool)
            .await?;
    if board_count > 5 {
        return Err(AppError::Validation("Maximum of 5 boards per user".into()));
    }

    let device_count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM devices WHERE user_id = $1")
        .bind(user_id)
        .fetch_one(pool)
        .await?;
    if device_count > 5 {
        return Err(AppError::Validation("Device limit exceeded (max 5)".into()));
    }

    Ok(())
}

// ───────────────────────────── Tag push logic ────────────────────────────────

async fn process_push_tag(
    tx: &mut PgConnection,
    user_id: Uuid,
    tag: &SyncTag,
) -> Result<TagResult, AppError> {
    let tag_uuid: Uuid = tag
        .uuid
        .parse()
        .map_err(|_| AppError::Validation("Invalid tag UUID".into()))?;

    // Check if this UUID already exists for this user
    let existing_updated: Option<String> = sqlx::query_scalar(&format!(
        "SELECT to_char(updated_at, '{TS_FMT}') FROM tags WHERE uuid = $1 AND user_id = $2"
    ))
    .bind(tag_uuid)
    .bind(user_id)
    .fetch_optional(&mut *tx)
    .await?;

    if let Some(server_updated) = existing_updated {
        // Last-write-wins
        if tag.updated_at > server_updated {
            sqlx::query(
                "UPDATE tags SET name = $1, deleted = $2, \
                 deleted_at = CASE WHEN $2 THEN NOW() ELSE NULL END, \
                 updated_at = $3::timestamp \
                 WHERE uuid = $4 AND user_id = $5",
            )
            .bind(&tag.name)
            .bind(tag.deleted)
            .bind(&tag.updated_at)
            .bind(tag_uuid)
            .bind(user_id)
            .execute(&mut *tx)
            .await?;
        }
        // Either way, the tag exists — accepted
        return Ok(TagResult::Accepted);
    }

    // Check name dedup: same name, different UUID, not deleted
    let name_match: Option<Uuid> = sqlx::query_scalar(
        "SELECT uuid FROM tags \
         WHERE user_id = $1 AND name = $2 AND deleted = FALSE AND uuid != $3",
    )
    .bind(user_id)
    .bind(&tag.name)
    .bind(tag_uuid)
    .fetch_optional(&mut *tx)
    .await?;

    if let Some(server_uuid) = name_match {
        return Ok(TagResult::Deduped {
            client_uuid: tag.uuid.clone(),
            server_uuid: server_uuid.to_string(),
        });
    }

    // New tag — insert
    sqlx::query(
        "INSERT INTO tags (uuid, user_id, name, updated_at, deleted, deleted_at) \
         VALUES ($1, $2, $3, $4::timestamp, $5, \
         CASE WHEN $5 THEN $4::timestamp ELSE NULL END)",
    )
    .bind(tag_uuid)
    .bind(user_id)
    .bind(&tag.name)
    .bind(&tag.updated_at)
    .bind(tag.deleted)
    .execute(&mut *tx)
    .await?;

    Ok(TagResult::Accepted)
}

// ───────────────────────────── Task push logic ───────────────────────────────

async fn process_push_task(
    tx: &mut PgConnection,
    user_id: Uuid,
    task: &SyncTask,
    remapped_tags: &[String],
) -> Result<Option<SyncTask>, AppError> {
    let task_uuid: Uuid = task
        .uuid
        .parse()
        .map_err(|_| AppError::Validation("Invalid task UUID".into()))?;

    let existing_updated: Option<String> = sqlx::query_scalar(&format!(
        "SELECT to_char(updated_at, '{TS_FMT}') FROM tasks WHERE uuid = $1 AND user_id = $2"
    ))
    .bind(task_uuid)
    .bind(user_id)
    .fetch_optional(&mut *tx)
    .await?;

    // Resolve board_uuid: use provided value or fall back to user's first board
    let board_uuid = resolve_board_uuid(&mut *tx, user_id, task.board_uuid.as_deref()).await?;

    if let Some(server_updated) = existing_updated {
        if task.updated_at > server_updated {
            // Client wins — update
            let due_date: Option<NaiveDate> = task.due_date.as_ref().and_then(|d| d.parse().ok());
            sqlx::query(
                "UPDATE tasks SET title = $1, description = $2, priority = $3, \
                 column_name = $4, due_date = $5, updated_at = $6::timestamp, \
                 deleted = $7, \
                 deleted_at = CASE WHEN $7 THEN NOW() ELSE NULL END, \
                 board_uuid = $8 \
                 WHERE uuid = $9 AND user_id = $10",
            )
            .bind(&task.title)
            .bind(&task.description)
            .bind(&task.priority)
            .bind(&task.column)
            .bind(due_date)
            .bind(&task.updated_at)
            .bind(task.deleted)
            .bind(board_uuid)
            .bind(task_uuid)
            .bind(user_id)
            .execute(&mut *tx)
            .await?;

            update_task_tags(&mut *tx, task_uuid, remapped_tags).await?;
            return Ok(None);
        }
        // Server wins — return server version so client can reconcile
        let server_task = fetch_single_task(&mut *tx, task_uuid).await?;
        return Ok(server_task);
    }

    // New task — insert
    let due_date: Option<NaiveDate> = task.due_date.as_ref().and_then(|d| d.parse().ok());
    sqlx::query(
        "INSERT INTO tasks \
         (uuid, user_id, title, description, priority, column_name, \
          due_date, created_at, updated_at, deleted, deleted_at, board_uuid) \
         VALUES ($1, $2, $3, $4, $5, $6, $7, $8::timestamp, $9::timestamp, $10, \
         CASE WHEN $10 THEN $9::timestamp ELSE NULL END, $11)",
    )
    .bind(task_uuid)
    .bind(user_id)
    .bind(&task.title)
    .bind(&task.description)
    .bind(&task.priority)
    .bind(&task.column)
    .bind(due_date)
    .bind(&task.created_at)
    .bind(&task.updated_at)
    .bind(task.deleted)
    .bind(board_uuid)
    .execute(&mut *tx)
    .await?;

    update_task_tags(&mut *tx, task_uuid, remapped_tags).await?;
    Ok(None)
}

// ───────────────────────────── Task-tag association ───────────────────────────

async fn update_task_tags(
    conn: &mut PgConnection,
    task_uuid: Uuid,
    tag_uuids: &[String],
) -> Result<(), AppError> {
    sqlx::query("DELETE FROM task_tags WHERE task_uuid = $1")
        .bind(task_uuid)
        .execute(&mut *conn)
        .await?;

    for tag_uuid_str in tag_uuids {
        if let Ok(tag_uuid) = tag_uuid_str.parse::<Uuid>() {
            sqlx::query(
                "INSERT INTO task_tags (task_uuid, tag_uuid) \
                 VALUES ($1, $2) ON CONFLICT DO NOTHING",
            )
            .bind(task_uuid)
            .bind(tag_uuid)
            .execute(&mut *conn)
            .await?;
        }
    }
    Ok(())
}

// ───────────────────────────── Fetch helpers (PgPool) ────────────────────────
//
// &PgPool is Copy, so the generic executor approach works fine here.
// We use a LEFT JOIN + COALESCE(array_agg(...)) to fetch tasks with their
// tags in a single query, avoiding the N+1 problem.

async fn fetch_all_tasks(pool: &PgPool, user_id: Uuid) -> Result<Vec<SyncTask>, AppError> {
    fetch_all_tasks_conn(pool, user_id).await
}

async fn fetch_tasks_since(
    pool: &PgPool,
    user_id: Uuid,
    since: &str,
) -> Result<Vec<SyncTask>, AppError> {
    fetch_tasks_since_conn(pool, user_id, since).await
}

async fn fetch_all_tags(pool: &PgPool, user_id: Uuid) -> Result<Vec<SyncTag>, AppError> {
    fetch_all_tags_conn(pool, user_id).await
}

async fn fetch_tags_since(
    pool: &PgPool,
    user_id: Uuid,
    since: &str,
) -> Result<Vec<SyncTag>, AppError> {
    fetch_tags_since_conn(pool, user_id, since).await
}

// ───────────────────────────── Fetch implementations ─────────────────────────
//
// All fetch functions use `sqlx::Executor` which is implemented for both
// `&PgPool` and `&mut PgConnection`. Since tasks need tags too, we use a
// LEFT JOIN + array_agg to get everything in a single query.

type TaskRow = (
    Uuid,
    String,
    String,
    String,
    String,
    Option<NaiveDate>,
    String,
    String,
    bool,
    Option<Vec<String>>,
    Option<String>,
);

fn task_row_to_sync(r: TaskRow) -> SyncTask {
    let tag_uuids = r.9.unwrap_or_default();
    // Filter out NULL entries produced by the LEFT JOIN when there are no tags
    let tag_uuids: Vec<String> = tag_uuids.into_iter().filter(|s| !s.is_empty()).collect();
    SyncTask {
        uuid: r.0.to_string(),
        title: r.1,
        description: r.2,
        priority: r.3,
        column: r.4,
        due_date: r.5.map(|d| d.format("%Y-%m-%d").to_string()),
        tags: tag_uuids,
        created_at: r.6,
        updated_at: r.7,
        deleted: r.8,
        board_uuid: r.10,
    }
}

async fn fetch_all_tasks_conn<'e, E>(executor: E, user_id: Uuid) -> Result<Vec<SyncTask>, AppError>
where
    E: sqlx::Executor<'e, Database = sqlx::Postgres>,
{
    let rows: Vec<TaskRow> = sqlx::query_as(&format!(
        "SELECT t.uuid, t.title, t.description, t.priority, t.column_name, t.due_date, \
         to_char(t.created_at, '{TS_FMT}'), to_char(t.updated_at, '{TS_FMT}'), t.deleted, \
         COALESCE(array_agg(tt.tag_uuid::text) FILTER (WHERE tt.tag_uuid IS NOT NULL), ARRAY[]::text[]), \
         t.board_uuid::text \
         FROM tasks t \
         LEFT JOIN task_tags tt ON tt.task_uuid = t.uuid \
         WHERE t.user_id = $1 \
         GROUP BY t.uuid"
    ))
    .bind(user_id)
    .fetch_all(executor)
    .await?;

    Ok(rows.into_iter().map(task_row_to_sync).collect())
}

async fn fetch_tasks_since_conn<'e, E>(
    executor: E,
    user_id: Uuid,
    since: &str,
) -> Result<Vec<SyncTask>, AppError>
where
    E: sqlx::Executor<'e, Database = sqlx::Postgres>,
{
    let rows: Vec<TaskRow> = sqlx::query_as(&format!(
        "SELECT t.uuid, t.title, t.description, t.priority, t.column_name, t.due_date, \
         to_char(t.created_at, '{TS_FMT}'), to_char(t.updated_at, '{TS_FMT}'), t.deleted, \
         COALESCE(array_agg(tt.tag_uuid::text) FILTER (WHERE tt.tag_uuid IS NOT NULL), ARRAY[]::text[]), \
         t.board_uuid::text \
         FROM tasks t \
         LEFT JOIN task_tags tt ON tt.task_uuid = t.uuid \
         WHERE t.user_id = $1 AND t.updated_at > $2::timestamp \
         GROUP BY t.uuid"
    ))
    .bind(user_id)
    .bind(since)
    .fetch_all(executor)
    .await?;

    Ok(rows.into_iter().map(task_row_to_sync).collect())
}

async fn fetch_all_tags_conn<'e, E>(executor: E, user_id: Uuid) -> Result<Vec<SyncTag>, AppError>
where
    E: sqlx::Executor<'e, Database = sqlx::Postgres>,
{
    let rows = sqlx::query_as::<_, (Uuid, String, String, bool)>(&format!(
        "SELECT uuid, name, to_char(updated_at, '{TS_FMT}'), deleted \
         FROM tags WHERE user_id = $1"
    ))
    .bind(user_id)
    .fetch_all(executor)
    .await?;

    Ok(rows
        .into_iter()
        .map(|r| SyncTag {
            uuid: r.0.to_string(),
            name: r.1,
            updated_at: r.2,
            deleted: r.3,
        })
        .collect())
}

async fn fetch_tags_since_conn<'e, E>(
    executor: E,
    user_id: Uuid,
    since: &str,
) -> Result<Vec<SyncTag>, AppError>
where
    E: sqlx::Executor<'e, Database = sqlx::Postgres>,
{
    let rows = sqlx::query_as::<_, (Uuid, String, String, bool)>(&format!(
        "SELECT uuid, name, to_char(updated_at, '{TS_FMT}'), deleted \
         FROM tags WHERE user_id = $1 AND updated_at > $2::timestamp"
    ))
    .bind(user_id)
    .bind(since)
    .fetch_all(executor)
    .await?;

    Ok(rows
        .into_iter()
        .map(|r| SyncTag {
            uuid: r.0.to_string(),
            name: r.1,
            updated_at: r.2,
            deleted: r.3,
        })
        .collect())
}

// ───────────────────────────── Single-record helpers ─────────────────────────

/// Fetch a single task with its tags from within a transaction.
async fn fetch_single_task(
    conn: &mut PgConnection,
    task_uuid: Uuid,
) -> Result<Option<SyncTask>, AppError> {
    let row: Option<TaskRow> = sqlx::query_as(&format!(
        "SELECT t.uuid, t.title, t.description, t.priority, t.column_name, t.due_date, \
         to_char(t.created_at, '{TS_FMT}'), to_char(t.updated_at, '{TS_FMT}'), t.deleted, \
         COALESCE(array_agg(tt.tag_uuid::text) FILTER (WHERE tt.tag_uuid IS NOT NULL), ARRAY[]::text[]), \
         t.board_uuid::text \
         FROM tasks t \
         LEFT JOIN task_tags tt ON tt.task_uuid = t.uuid \
         WHERE t.uuid = $1 \
         GROUP BY t.uuid"
    ))
    .bind(task_uuid)
    .fetch_optional(&mut *conn)
    .await?;

    Ok(row.map(task_row_to_sync))
}

/// Fetch a single tag by UUID from within a transaction.
async fn fetch_tag_by_uuid(
    conn: &mut PgConnection,
    user_id: Uuid,
    uuid_str: &str,
) -> Result<Option<SyncTag>, AppError> {
    let tag_uuid: Uuid = uuid_str
        .parse()
        .map_err(|_| AppError::Validation("Invalid tag UUID".into()))?;

    let row = sqlx::query_as::<_, (Uuid, String, String, bool)>(&format!(
        "SELECT uuid, name, to_char(updated_at, '{TS_FMT}'), deleted \
         FROM tags WHERE uuid = $1 AND user_id = $2"
    ))
    .bind(tag_uuid)
    .bind(user_id)
    .fetch_optional(&mut *conn)
    .await?;

    Ok(row.map(|r| SyncTag {
        uuid: r.0.to_string(),
        name: r.1,
        updated_at: r.2,
        deleted: r.3,
    }))
}

// ───────────────────────────── Board push logic ──────────────────────────────

/// Process boards from a push payload. Returns rejected boards (server wins).
async fn process_boards(
    tx: &mut PgConnection,
    user_id: Uuid,
    boards: &[SyncBoard],
) -> Result<Vec<SyncBoard>, AppError> {
    let mut rejected = Vec::new();

    for board in boards {
        let board_uuid: Uuid = board
            .uuid
            .parse()
            .map_err(|_| AppError::Validation("Invalid board UUID".into()))?;

        // Check if this UUID already exists for this user
        let existing_updated: Option<String> = sqlx::query_scalar(&format!(
            "SELECT to_char(updated_at, '{TS_FMT}') FROM boards WHERE uuid = $1 AND user_id = $2"
        ))
        .bind(board_uuid)
        .bind(user_id)
        .fetch_optional(&mut *tx)
        .await?;

        if let Some(server_updated) = existing_updated {
            // Last-write-wins
            if board.updated_at > server_updated {
                sqlx::query(
                    "UPDATE boards SET name = $1, position = $2, deleted = $3, \
                     deleted_at = CASE WHEN $3 THEN NOW() ELSE NULL END, \
                     updated_at = $4::timestamp \
                     WHERE uuid = $5 AND user_id = $6",
                )
                .bind(&board.name)
                .bind(board.position)
                .bind(board.deleted)
                .bind(&board.updated_at)
                .bind(board_uuid)
                .bind(user_id)
                .execute(&mut *tx)
                .await?;
            } else {
                // Server wins — return server version
                if let Some(server_board) =
                    fetch_single_board(&mut *tx, user_id, board_uuid).await?
                {
                    rejected.push(server_board);
                }
            }
        } else {
            // New board — insert
            sqlx::query(
                "INSERT INTO boards (uuid, user_id, name, position, updated_at, deleted, deleted_at) \
                 VALUES ($1, $2, $3, $4, $5::timestamp, $6, \
                 CASE WHEN $6 THEN $5::timestamp ELSE NULL END)",
            )
            .bind(board_uuid)
            .bind(user_id)
            .bind(&board.name)
            .bind(board.position)
            .bind(&board.updated_at)
            .bind(board.deleted)
            .execute(&mut *tx)
            .await?;
        }
    }

    Ok(rejected)
}

/// Fetch a single board by UUID from within a transaction.
async fn fetch_single_board(
    conn: &mut PgConnection,
    user_id: Uuid,
    board_uuid: Uuid,
) -> Result<Option<SyncBoard>, AppError> {
    let row = sqlx::query_as::<_, (Uuid, String, i32, String, bool)>(&format!(
        "SELECT uuid, name, position, to_char(updated_at, '{TS_FMT}'), deleted \
         FROM boards WHERE uuid = $1 AND user_id = $2"
    ))
    .bind(board_uuid)
    .bind(user_id)
    .fetch_optional(&mut *conn)
    .await?;

    Ok(row.map(|r| SyncBoard {
        uuid: r.0.to_string(),
        name: r.1,
        position: r.2,
        updated_at: r.3,
        deleted: r.4,
    }))
}

/// Resolve a board UUID for a task. If the client provides one, parse and use it.
/// If not, fall back to the user's first board.
async fn resolve_board_uuid(
    conn: &mut PgConnection,
    user_id: Uuid,
    board_uuid_str: Option<&str>,
) -> Result<Uuid, AppError> {
    if let Some(s) = board_uuid_str {
        if let Ok(uuid) = s.parse::<Uuid>() {
            return Ok(uuid);
        }
    }
    // Fall back to user's first board (by position)
    let uuid: Option<Uuid> = sqlx::query_scalar(
        "SELECT uuid FROM boards WHERE user_id = $1 AND deleted = FALSE ORDER BY position LIMIT 1",
    )
    .bind(user_id)
    .fetch_optional(&mut *conn)
    .await?;

    uuid.ok_or_else(|| AppError::Validation("No board found for user".into()))
}

// ───────────────────────────── Board fetch helpers ────────────────────────────

async fn fetch_all_boards(pool: &PgPool, user_id: Uuid) -> Result<Vec<SyncBoard>, AppError> {
    fetch_all_boards_conn(pool, user_id).await
}

async fn fetch_boards_since(
    pool: &PgPool,
    user_id: Uuid,
    since: &str,
) -> Result<Vec<SyncBoard>, AppError> {
    fetch_boards_since_conn(pool, user_id, since).await
}

async fn fetch_all_boards_conn<'e, E>(
    executor: E,
    user_id: Uuid,
) -> Result<Vec<SyncBoard>, AppError>
where
    E: sqlx::Executor<'e, Database = sqlx::Postgres>,
{
    let rows = sqlx::query_as::<_, (Uuid, String, i32, String, bool)>(&format!(
        "SELECT uuid, name, position, to_char(updated_at, '{TS_FMT}'), deleted \
         FROM boards WHERE user_id = $1"
    ))
    .bind(user_id)
    .fetch_all(executor)
    .await?;

    Ok(rows
        .into_iter()
        .map(|r| SyncBoard {
            uuid: r.0.to_string(),
            name: r.1,
            position: r.2,
            updated_at: r.3,
            deleted: r.4,
        })
        .collect())
}

async fn fetch_boards_since_conn<'e, E>(
    executor: E,
    user_id: Uuid,
    since: &str,
) -> Result<Vec<SyncBoard>, AppError>
where
    E: sqlx::Executor<'e, Database = sqlx::Postgres>,
{
    let rows = sqlx::query_as::<_, (Uuid, String, i32, String, bool)>(&format!(
        "SELECT uuid, name, position, to_char(updated_at, '{TS_FMT}'), deleted \
         FROM boards WHERE user_id = $1 AND updated_at > $2::timestamp"
    ))
    .bind(user_id)
    .bind(since)
    .fetch_all(executor)
    .await?;

    Ok(rows
        .into_iter()
        .map(|r| SyncBoard {
            uuid: r.0.to_string(),
            name: r.1,
            position: r.2,
            updated_at: r.3,
            deleted: r.4,
        })
        .collect())
}
