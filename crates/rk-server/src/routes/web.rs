use std::collections::HashMap;

use axum::{extract::Path, Extension, Json};
use chrono::NaiveDate;
use serde::{Deserialize, Serialize};
use sqlx::PgPool;
use uuid::Uuid;

use crate::auth::WebUser;
use crate::error::AppError;
use crate::{TS_FMT, VALID_COLUMNS, VALID_PRIORITIES};

// ───────────────────────────── Response types ─────────────────────────────────

#[derive(Serialize)]
pub struct UserResponse {
    pub id: String,
    pub username: String,
}

#[derive(Serialize)]
pub struct BoardSummary {
    pub uuid: String,
    pub name: String,
    pub position: i32,
}

#[derive(Serialize, Clone)]
pub struct TagResponse {
    pub uuid: String,
    pub name: String,
}

#[derive(Serialize)]
pub struct TaskResponse {
    pub uuid: String,
    pub title: String,
    pub description: String,
    pub priority: String,
    pub column: String,
    pub due_date: Option<String>,
    pub tags: Vec<TagResponse>,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Serialize)]
pub struct BoardDetailResponse {
    pub board: BoardSummary,
    pub tasks: Vec<TaskResponse>,
    pub tags: Vec<TagResponse>,
}

// ───────────────────────────── Request types ──────────────────────────────────

#[derive(Deserialize)]
pub struct CreateTaskRequest {
    pub board_uuid: String,
    pub title: String,
    pub priority: Option<String>,
    pub column: Option<String>,
    pub description: Option<String>,
    pub due_date: Option<String>,
    pub tag_uuids: Option<Vec<String>>,
}

#[derive(Deserialize)]
pub struct UpdateTaskRequest {
    pub title: Option<String>,
    pub description: Option<String>,
    pub priority: Option<String>,
    pub column: Option<String>,
    pub due_date: Option<Option<String>>,
    pub tag_uuids: Option<Vec<String>>,
}

#[derive(Deserialize)]
pub struct CreateTagRequest {
    pub name: String,
}

#[derive(Deserialize)]
pub struct UpdateTagRequest {
    pub name: String,
}

// ───────────────────────────── Shared helpers ────────────────────────────────

/// Verify that a board exists, belongs to the user, and is not deleted.
pub(crate) async fn verify_board_ownership(
    pool: &PgPool,
    board_uuid: Uuid,
    user_id: Uuid,
) -> Result<(), AppError> {
    let exists: bool = sqlx::query_scalar(
        "SELECT EXISTS(SELECT 1 FROM boards WHERE uuid = $1 AND user_id = $2 AND deleted = FALSE)",
    )
    .bind(board_uuid)
    .bind(user_id)
    .fetch_one(pool)
    .await?;
    if !exists {
        return Err(AppError::NotFound("Board not found".into()));
    }
    Ok(())
}

/// Batch-fetch tags for multiple tasks in a single query. Returns a map of task_uuid -> tags.
pub(crate) async fn fetch_tags_for_tasks(
    pool: &PgPool,
    task_uuids: &[Uuid],
) -> Result<HashMap<Uuid, Vec<TagResponse>>, AppError> {
    if task_uuids.is_empty() {
        return Ok(HashMap::new());
    }

    let rows = sqlx::query_as::<_, (Uuid, Uuid, String)>(
        "SELECT tt.task_uuid, tg.uuid, tg.name FROM tags tg \
         JOIN task_tags tt ON tt.tag_uuid = tg.uuid \
         WHERE tt.task_uuid = ANY($1) AND tg.deleted = FALSE \
         ORDER BY tg.name",
    )
    .bind(task_uuids)
    .fetch_all(pool)
    .await?;

    let mut map: HashMap<Uuid, Vec<TagResponse>> = HashMap::new();
    for (task_uuid, tag_uuid, tag_name) in rows {
        map.entry(task_uuid).or_default().push(TagResponse {
            uuid: tag_uuid.to_string(),
            name: tag_name,
        });
    }
    Ok(map)
}

/// Batch-validate and associate tags with a task. Only associates tags owned by `owner_id`.
pub(crate) async fn associate_tags(
    pool: &PgPool,
    task_uuid: Uuid,
    owner_id: Uuid,
    tag_uuid_strs: &[String],
) -> Result<(), AppError> {
    let tag_uuids: Vec<Uuid> = tag_uuid_strs
        .iter()
        .filter_map(|s| s.parse::<Uuid>().ok())
        .collect();

    if tag_uuids.is_empty() {
        return Ok(());
    }

    // Batch-verify ownership in a single query
    let valid_uuids: Vec<Uuid> = sqlx::query_scalar(
        "SELECT uuid FROM tags WHERE uuid = ANY($1) AND user_id = $2 AND deleted = FALSE",
    )
    .bind(&tag_uuids)
    .bind(owner_id)
    .fetch_all(pool)
    .await?;

    for tag_uuid in valid_uuids {
        sqlx::query(
            "INSERT INTO task_tags (task_uuid, tag_uuid) VALUES ($1, $2) ON CONFLICT DO NOTHING",
        )
        .bind(task_uuid)
        .bind(tag_uuid)
        .execute(pool)
        .await?;
    }

    Ok(())
}

/// Replace all tag associations for a task: delete existing, then associate new ones.
pub(crate) async fn replace_tags(
    pool: &PgPool,
    task_uuid: Uuid,
    owner_id: Uuid,
    tag_uuid_strs: &[String],
) -> Result<(), AppError> {
    sqlx::query("DELETE FROM task_tags WHERE task_uuid = $1")
        .bind(task_uuid)
        .execute(pool)
        .await?;
    associate_tags(pool, task_uuid, owner_id, tag_uuid_strs).await?;
    sqlx::query("UPDATE tasks SET updated_at = NOW() WHERE uuid = $1")
        .bind(task_uuid)
        .execute(pool)
        .await?;
    Ok(())
}

/// Validate fields common to task update requests. Returns errors for invalid values.
pub(crate) fn validate_update_fields(req: &UpdateTaskRequest) -> Result<(), AppError> {
    if let Some(ref title) = req.title {
        if title.trim().is_empty() || title.len() > 500 {
            return Err(AppError::Validation(
                "Title must be 1-500 characters".into(),
            ));
        }
    }
    if let Some(ref col) = req.column {
        if !VALID_COLUMNS.contains(&col.as_str()) {
            return Err(AppError::Validation("Invalid column".into()));
        }
    }
    if let Some(ref pri) = req.priority {
        if !VALID_PRIORITIES.contains(&pri.as_str()) {
            return Err(AppError::Validation("Invalid priority".into()));
        }
    }
    if let Some(ref desc) = req.description {
        if desc.len() > 5000 {
            return Err(AppError::Validation(
                "Description must be under 5000 characters".into(),
            ));
        }
    }
    Ok(())
}

/// Apply field-level updates to a task. Caller must verify ownership first.
pub(crate) async fn apply_task_updates(
    pool: &PgPool,
    task_uuid: Uuid,
    owner_id: Uuid,
    req: &UpdateTaskRequest,
) -> Result<(), AppError> {
    if let Some(ref title) = req.title {
        sqlx::query("UPDATE tasks SET title = $1, updated_at = NOW() WHERE uuid = $2")
            .bind(title.trim())
            .bind(task_uuid)
            .execute(pool)
            .await?;
    }
    if let Some(ref description) = req.description {
        sqlx::query("UPDATE tasks SET description = $1, updated_at = NOW() WHERE uuid = $2")
            .bind(description)
            .bind(task_uuid)
            .execute(pool)
            .await?;
    }
    if let Some(ref priority) = req.priority {
        sqlx::query("UPDATE tasks SET priority = $1, updated_at = NOW() WHERE uuid = $2")
            .bind(priority)
            .bind(task_uuid)
            .execute(pool)
            .await?;
    }
    if let Some(ref column) = req.column {
        sqlx::query("UPDATE tasks SET column_name = $1, updated_at = NOW() WHERE uuid = $2")
            .bind(column)
            .bind(task_uuid)
            .execute(pool)
            .await?;
    }
    if let Some(ref due_date_opt) = req.due_date {
        let due_date: Option<NaiveDate> = due_date_opt
            .as_ref()
            .map(|d| d.parse::<NaiveDate>())
            .transpose()
            .map_err(|_| AppError::Validation("Invalid due_date format".into()))?;
        sqlx::query("UPDATE tasks SET due_date = $1, updated_at = NOW() WHERE uuid = $2")
            .bind(due_date)
            .bind(task_uuid)
            .execute(pool)
            .await?;
    }
    if let Some(ref tag_uuids) = req.tag_uuids {
        replace_tags(pool, task_uuid, owner_id, tag_uuids).await?;
    }
    Ok(())
}

// ───────────────────────────── GET /api/v1/me ────────────────────────────────

pub async fn get_me(
    auth: WebUser,
    Extension(pool): Extension<PgPool>,
) -> Result<Json<UserResponse>, AppError> {
    let row = sqlx::query_as::<_, (Uuid, String)>("SELECT id, username FROM users WHERE id = $1")
        .bind(auth.user_id)
        .fetch_optional(&pool)
        .await?
        .ok_or(AppError::NotFound("User not found".into()))?;

    Ok(Json(UserResponse {
        id: row.0.to_string(),
        username: row.1,
    }))
}

// ───────────────────────────── GET /api/v1/boards ────────────────────────────

pub async fn list_boards(
    auth: WebUser,
    Extension(pool): Extension<PgPool>,
) -> Result<Json<Vec<BoardSummary>>, AppError> {
    let rows = sqlx::query_as::<_, (Uuid, String, i32)>(
        "SELECT uuid, name, position FROM boards \
         WHERE user_id = $1 AND deleted = FALSE \
         ORDER BY position",
    )
    .bind(auth.user_id)
    .fetch_all(&pool)
    .await?;

    Ok(Json(
        rows.into_iter()
            .map(|r| BoardSummary {
                uuid: r.0.to_string(),
                name: r.1,
                position: r.2,
            })
            .collect(),
    ))
}

// ───────────────────────────── GET /api/v1/boards/:uuid ──────────────────────

pub async fn get_board(
    auth: WebUser,
    Path(uuid): Path<Uuid>,
    Extension(pool): Extension<PgPool>,
) -> Result<Json<BoardDetailResponse>, AppError> {
    // Verify board ownership
    let board = sqlx::query_as::<_, (Uuid, String, i32)>(
        "SELECT uuid, name, position FROM boards \
         WHERE uuid = $1 AND user_id = $2 AND deleted = FALSE",
    )
    .bind(uuid)
    .bind(auth.user_id)
    .fetch_optional(&pool)
    .await?
    .ok_or(AppError::NotFound("Board not found".into()))?;

    // Fetch non-deleted tasks
    let task_rows = sqlx::query_as::<
        _,
        (
            Uuid,
            String,
            String,
            String,
            String,
            Option<NaiveDate>,
            String,
            String,
        ),
    >(&format!(
        "SELECT t.uuid, t.title, t.description, t.priority, t.column_name, t.due_date, \
             to_char(t.created_at, '{TS_FMT}'), to_char(t.updated_at, '{TS_FMT}') \
             FROM tasks t \
             WHERE t.board_uuid = $1 AND t.user_id = $2 AND t.deleted = FALSE \
             ORDER BY t.due_date ASC NULLS LAST, t.created_at ASC"
    ))
    .bind(uuid)
    .bind(auth.user_id)
    .fetch_all(&pool)
    .await?;

    // Batch-fetch tags for all tasks (single query instead of N+1)
    let task_uuids: Vec<Uuid> = task_rows.iter().map(|r| r.0).collect();
    let tags_by_task = fetch_tags_for_tasks(&pool, &task_uuids).await?;

    let tasks: Vec<TaskResponse> = task_rows
        .into_iter()
        .map(|row| TaskResponse {
            uuid: row.0.to_string(),
            title: row.1,
            description: row.2,
            priority: row.3,
            column: row.4,
            due_date: row.5.map(|d| d.format("%Y-%m-%d").to_string()),
            tags: tags_by_task.get(&row.0).cloned().unwrap_or_default(),
            created_at: row.6,
            updated_at: row.7,
        })
        .collect();

    // Fetch all user's non-deleted tags
    let all_tags = sqlx::query_as::<_, (Uuid, String)>(
        "SELECT uuid, name FROM tags WHERE user_id = $1 AND deleted = FALSE ORDER BY name",
    )
    .bind(auth.user_id)
    .fetch_all(&pool)
    .await?;

    Ok(Json(BoardDetailResponse {
        board: BoardSummary {
            uuid: board.0.to_string(),
            name: board.1,
            position: board.2,
        },
        tasks,
        tags: all_tags
            .into_iter()
            .map(|t| TagResponse {
                uuid: t.0.to_string(),
                name: t.1,
            })
            .collect(),
    }))
}

// ───────────────────────────── Task CRUD ──────────────────────────────────────

pub async fn create_task(
    auth: WebUser,
    Extension(pool): Extension<PgPool>,
    Json(req): Json<CreateTaskRequest>,
) -> Result<Json<TaskResponse>, AppError> {
    // Validate
    if req.title.trim().is_empty() || req.title.len() > 500 {
        return Err(AppError::Validation(
            "Title must be 1-500 characters".into(),
        ));
    }
    let column = req.column.as_deref().unwrap_or("todo");
    if !VALID_COLUMNS.contains(&column) {
        return Err(AppError::Validation("Invalid column".into()));
    }
    let priority = req.priority.as_deref().unwrap_or("Medium");
    if !VALID_PRIORITIES.contains(&priority) {
        return Err(AppError::Validation("Invalid priority".into()));
    }
    let description = req.description.as_deref().unwrap_or("");
    if description.len() > 5000 {
        return Err(AppError::Validation(
            "Description must be under 5000 characters".into(),
        ));
    }

    // Verify board ownership
    let board_uuid: Uuid = req
        .board_uuid
        .parse()
        .map_err(|_| AppError::Validation("Invalid board UUID".into()))?;
    verify_board_ownership(&pool, board_uuid, auth.user_id).await?;

    // Parse due_date
    let due_date: Option<NaiveDate> = req
        .due_date
        .as_ref()
        .map(|d| d.parse::<NaiveDate>())
        .transpose()
        .map_err(|_| AppError::Validation("Invalid due_date format (YYYY-MM-DD)".into()))?;

    let task_uuid = Uuid::new_v4();
    let now = chrono::Utc::now().naive_utc();

    sqlx::query(
        "INSERT INTO tasks (uuid, user_id, title, description, priority, column_name, \
         due_date, created_at, updated_at, board_uuid) \
         VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $8, $9)",
    )
    .bind(task_uuid)
    .bind(auth.user_id)
    .bind(req.title.trim())
    .bind(description)
    .bind(priority)
    .bind(column)
    .bind(due_date)
    .bind(now)
    .bind(board_uuid)
    .execute(&pool)
    .await?;

    // Associate tags (batch-validated)
    associate_tags(
        &pool,
        task_uuid,
        auth.user_id,
        &req.tag_uuids.unwrap_or_default(),
    )
    .await?;

    fetch_task_response(&pool, task_uuid).await
}

pub async fn update_task(
    auth: WebUser,
    Path(uuid): Path<Uuid>,
    Extension(pool): Extension<PgPool>,
    Json(req): Json<UpdateTaskRequest>,
) -> Result<Json<TaskResponse>, AppError> {
    // Verify task ownership
    let exists: bool = sqlx::query_scalar(
        "SELECT EXISTS(SELECT 1 FROM tasks WHERE uuid = $1 AND user_id = $2 AND deleted = FALSE)",
    )
    .bind(uuid)
    .bind(auth.user_id)
    .fetch_one(&pool)
    .await?;
    if !exists {
        return Err(AppError::NotFound("Task not found".into()));
    }

    validate_update_fields(&req)?;
    apply_task_updates(&pool, uuid, auth.user_id, &req).await?;

    fetch_task_response(&pool, uuid).await
}

pub async fn delete_task(
    auth: WebUser,
    Path(uuid): Path<Uuid>,
    Extension(pool): Extension<PgPool>,
) -> Result<axum::http::StatusCode, AppError> {
    let result = sqlx::query(
        "UPDATE tasks SET deleted = TRUE, deleted_at = NOW(), updated_at = NOW() \
         WHERE uuid = $1 AND user_id = $2 AND deleted = FALSE",
    )
    .bind(uuid)
    .bind(auth.user_id)
    .execute(&pool)
    .await?;

    if result.rows_affected() == 0 {
        return Err(AppError::NotFound("Task not found".into()));
    }

    Ok(axum::http::StatusCode::NO_CONTENT)
}

// ───────────────────────────── Tag CRUD ───────────────────────────────────────

pub async fn create_tag(
    auth: WebUser,
    Extension(pool): Extension<PgPool>,
    Json(req): Json<CreateTagRequest>,
) -> Result<Json<TagResponse>, AppError> {
    let name = req.name.trim();
    if name.is_empty() || name.len() > 50 {
        return Err(AppError::Validation(
            "Tag name must be 1-50 characters".into(),
        ));
    }

    // Check tag limit
    let count: i64 =
        sqlx::query_scalar("SELECT COUNT(*) FROM tags WHERE user_id = $1 AND deleted = FALSE")
            .bind(auth.user_id)
            .fetch_one(&pool)
            .await?;
    if count >= 15 {
        return Err(AppError::Validation("Tag limit reached (max 15)".into()));
    }

    let tag_uuid = Uuid::new_v4();
    sqlx::query("INSERT INTO tags (uuid, user_id, name, updated_at) VALUES ($1, $2, $3, NOW())")
        .bind(tag_uuid)
        .bind(auth.user_id)
        .bind(name)
        .execute(&pool)
        .await
        .map_err(|e| {
            if let sqlx::Error::Database(ref db_err) = e {
                if db_err.constraint() == Some("idx_tags_user_name_active") {
                    return AppError::Validation("Tag name already exists".into());
                }
            }
            AppError::from(e)
        })?;

    Ok(Json(TagResponse {
        uuid: tag_uuid.to_string(),
        name: name.to_string(),
    }))
}

pub async fn update_tag(
    auth: WebUser,
    Path(uuid): Path<Uuid>,
    Extension(pool): Extension<PgPool>,
    Json(req): Json<UpdateTagRequest>,
) -> Result<Json<TagResponse>, AppError> {
    let name = req.name.trim();
    if name.is_empty() || name.len() > 50 {
        return Err(AppError::Validation(
            "Tag name must be 1-50 characters".into(),
        ));
    }

    let result = sqlx::query(
        "UPDATE tags SET name = $1, updated_at = NOW() \
         WHERE uuid = $2 AND user_id = $3 AND deleted = FALSE",
    )
    .bind(name)
    .bind(uuid)
    .bind(auth.user_id)
    .execute(&pool)
    .await
    .map_err(|e| {
        if let sqlx::Error::Database(ref db_err) = e {
            if db_err.constraint() == Some("idx_tags_user_name_active") {
                return AppError::Validation("Tag name already exists".into());
            }
        }
        AppError::from(e)
    })?;

    if result.rows_affected() == 0 {
        return Err(AppError::NotFound("Tag not found".into()));
    }

    Ok(Json(TagResponse {
        uuid: uuid.to_string(),
        name: name.to_string(),
    }))
}

pub async fn delete_tag(
    auth: WebUser,
    Path(uuid): Path<Uuid>,
    Extension(pool): Extension<PgPool>,
) -> Result<axum::http::StatusCode, AppError> {
    // Soft-delete the tag
    let result = sqlx::query(
        "UPDATE tags SET deleted = TRUE, deleted_at = NOW(), updated_at = NOW() \
         WHERE uuid = $1 AND user_id = $2 AND deleted = FALSE",
    )
    .bind(uuid)
    .bind(auth.user_id)
    .execute(&pool)
    .await?;

    if result.rows_affected() == 0 {
        return Err(AppError::NotFound("Tag not found".into()));
    }

    // Remove from all task associations
    sqlx::query("DELETE FROM task_tags WHERE tag_uuid = $1")
        .bind(uuid)
        .execute(&pool)
        .await?;

    Ok(axum::http::StatusCode::NO_CONTENT)
}

// ───────────────────────────── Helper ─────────────────────────────────────────

pub(crate) async fn fetch_task_response(
    pool: &PgPool,
    task_uuid: Uuid,
) -> Result<Json<TaskResponse>, AppError> {
    let row = sqlx::query_as::<
        _,
        (
            Uuid,
            String,
            String,
            String,
            String,
            Option<NaiveDate>,
            String,
            String,
        ),
    >(&format!(
        "SELECT uuid, title, description, priority, column_name, due_date, \
             to_char(created_at, '{TS_FMT}'), to_char(updated_at, '{TS_FMT}') \
             FROM tasks WHERE uuid = $1"
    ))
    .bind(task_uuid)
    .fetch_optional(pool)
    .await?
    .ok_or(AppError::NotFound("Task not found".into()))?;

    let tag_rows = sqlx::query_as::<_, (Uuid, String)>(
        "SELECT tg.uuid, tg.name FROM tags tg \
         JOIN task_tags tt ON tt.tag_uuid = tg.uuid \
         WHERE tt.task_uuid = $1 AND tg.deleted = FALSE",
    )
    .bind(task_uuid)
    .fetch_all(pool)
    .await?;

    Ok(Json(TaskResponse {
        uuid: row.0.to_string(),
        title: row.1,
        description: row.2,
        priority: row.3,
        column: row.4,
        due_date: row.5.map(|d| d.format("%Y-%m-%d").to_string()),
        tags: tag_rows
            .into_iter()
            .map(|t| TagResponse {
                uuid: t.0.to_string(),
                name: t.1,
            })
            .collect(),
        created_at: row.6,
        updated_at: row.7,
    }))
}
