use axum::{extract::Path, Extension, Json};
use chrono::NaiveDate;
use rand::Rng;
use serde::{Deserialize, Serialize};
use sqlx::PgPool;
use uuid::Uuid;

use super::web::{
    apply_task_updates, associate_tags, fetch_tags_for_tasks, fetch_task_response,
    validate_update_fields, verify_board_ownership, TagResponse, TaskResponse,
};
use crate::auth::WebUser;
use crate::error::AppError;
use crate::{TS_FMT, VALID_COLUMNS, VALID_PRIORITIES};

// ───────────────────────────── Constants ─────────────────────────────────────

const PERM_VIEW: &str = "view";
const PERM_EDIT: &str = "edit";
const VALID_PERMISSIONS: &[&str] = &[PERM_VIEW, PERM_EDIT];

// ───────────────────────────── Types ──────────────────────────────────────────

#[derive(Deserialize)]
pub struct CreateShareRequest {
    pub permission: String,
}

#[derive(Serialize)]
pub struct ShareResponse {
    pub id: String,
    pub token: String,
    pub permission: String,
    pub url: String,
}

#[derive(Serialize)]
pub struct SharedBoardResponse {
    pub board: SharedBoardInfo,
    pub tasks: Vec<TaskResponse>,
    pub tags: Vec<TagResponse>,
    pub permission: String,
    pub owner_username: String,
}

#[derive(Serialize)]
pub struct SharedBoardInfo {
    pub uuid: String,
    pub name: String,
}

// ───────────────────────────── Share Management ──────────────────────────────

/// Generate a 22-char URL-safe random token.
fn generate_share_token() -> String {
    use rand::distributions::Alphanumeric;
    rand::thread_rng()
        .sample_iter(&Alphanumeric)
        .take(22)
        .map(char::from)
        .collect()
}

/// POST /api/v1/boards/:uuid/shares
pub async fn create_share(
    auth: WebUser,
    Path(board_uuid): Path<Uuid>,
    Extension(pool): Extension<PgPool>,
    Json(req): Json<CreateShareRequest>,
) -> Result<Json<ShareResponse>, AppError> {
    if !VALID_PERMISSIONS.contains(&req.permission.as_str()) {
        return Err(AppError::Validation(
            "Permission must be 'view' or 'edit'".into(),
        ));
    }

    verify_board_ownership(&pool, board_uuid, auth.user_id).await?;

    // Check share limit (max 10 per board)
    let share_count: i64 =
        sqlx::query_scalar("SELECT COUNT(*) FROM board_shares WHERE board_uuid = $1")
            .bind(board_uuid)
            .fetch_one(&pool)
            .await?;
    if share_count >= 10 {
        return Err(AppError::Validation(
            "Share limit reached (max 10 per board)".into(),
        ));
    }

    let token = generate_share_token();
    let share_id = Uuid::new_v4();

    sqlx::query(
        "INSERT INTO board_shares (id, board_uuid, token, permission, created_by) \
         VALUES ($1, $2, $3, $4, $5)",
    )
    .bind(share_id)
    .bind(board_uuid)
    .bind(&token)
    .bind(&req.permission)
    .bind(auth.user_id)
    .execute(&pool)
    .await?;

    Ok(Json(ShareResponse {
        id: share_id.to_string(),
        permission: req.permission,
        url: format!("/app#/shared/{token}"),
        token,
    }))
}

/// GET /api/v1/boards/:uuid/shares
pub async fn list_shares(
    auth: WebUser,
    Path(board_uuid): Path<Uuid>,
    Extension(pool): Extension<PgPool>,
) -> Result<Json<Vec<ShareResponse>>, AppError> {
    verify_board_ownership(&pool, board_uuid, auth.user_id).await?;

    let rows = sqlx::query_as::<_, (Uuid, String, String)>(
        "SELECT id, token, permission FROM board_shares WHERE board_uuid = $1 ORDER BY created_at",
    )
    .bind(board_uuid)
    .fetch_all(&pool)
    .await?;

    Ok(Json(
        rows.into_iter()
            .map(|r| ShareResponse {
                id: r.0.to_string(),
                token: r.1.clone(),
                permission: r.2,
                url: format!("/app#/shared/{}", r.1),
            })
            .collect(),
    ))
}

/// DELETE /api/v1/shares/:id
pub async fn delete_share(
    auth: WebUser,
    Path(id): Path<Uuid>,
    Extension(pool): Extension<PgPool>,
) -> Result<axum::http::StatusCode, AppError> {
    let result = sqlx::query("DELETE FROM board_shares WHERE id = $1 AND created_by = $2")
        .bind(id)
        .bind(auth.user_id)
        .execute(&pool)
        .await?;

    if result.rows_affected() == 0 {
        return Err(AppError::NotFound("Share not found".into()));
    }

    Ok(axum::http::StatusCode::NO_CONTENT)
}

// ───────────────────────────── Shared Board Access ───────────────────────────

/// Look up a share by token, returning (board_uuid, owner_id, permission).
async fn lookup_share(pool: &PgPool, token: &str) -> Result<(Uuid, Uuid, String), AppError> {
    sqlx::query_as::<_, (Uuid, Uuid, String)>(
        "SELECT bs.board_uuid, bs.created_by, bs.permission \
         FROM board_shares bs \
         JOIN boards b ON b.uuid = bs.board_uuid \
         WHERE bs.token = $1 AND b.deleted = FALSE",
    )
    .bind(token)
    .fetch_optional(pool)
    .await?
    .ok_or(AppError::NotFound(
        "Share not found or board deleted".into(),
    ))
}

/// Validate share token has edit permission. Returns (board_uuid, owner_id).
async fn validate_edit_share(pool: &PgPool, token: &str) -> Result<(Uuid, Uuid), AppError> {
    let (board_uuid, owner_id, permission) = lookup_share(pool, token).await?;
    if permission != PERM_EDIT {
        return Err(AppError::Forbidden);
    }
    Ok((board_uuid, owner_id))
}

/// GET /api/v1/shared/:token -- public, no auth required
pub async fn get_shared_board(
    Path(token): Path<String>,
    Extension(pool): Extension<PgPool>,
) -> Result<Json<SharedBoardResponse>, AppError> {
    let (board_uuid, owner_id, permission) = lookup_share(&pool, &token).await?;

    // Fetch board info
    let board = sqlx::query_as::<_, (String,)>("SELECT name FROM boards WHERE uuid = $1")
        .bind(board_uuid)
        .fetch_one(&pool)
        .await?;

    // Fetch owner username
    let owner_username: String = sqlx::query_scalar("SELECT username FROM users WHERE id = $1")
        .bind(owner_id)
        .fetch_one(&pool)
        .await?;

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
        "SELECT uuid, title, description, priority, column_name, due_date, \
             to_char(created_at, '{TS_FMT}'), to_char(updated_at, '{TS_FMT}') \
             FROM tasks WHERE board_uuid = $1 AND user_id = $2 AND deleted = FALSE \
             ORDER BY due_date ASC NULLS LAST, created_at ASC"
    ))
    .bind(board_uuid)
    .bind(owner_id)
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

    // Fetch tags referenced by board tasks only (for shared view)
    let tags = sqlx::query_as::<_, (Uuid, String)>(
        "SELECT DISTINCT tg.uuid, tg.name FROM tags tg \
         JOIN task_tags tt ON tt.tag_uuid = tg.uuid \
         JOIN tasks t ON t.uuid = tt.task_uuid \
         WHERE t.board_uuid = $1 AND t.user_id = $2 AND t.deleted = FALSE AND tg.deleted = FALSE \
         ORDER BY tg.name",
    )
    .bind(board_uuid)
    .bind(owner_id)
    .fetch_all(&pool)
    .await?;

    Ok(Json(SharedBoardResponse {
        board: SharedBoardInfo {
            uuid: board_uuid.to_string(),
            name: board.0,
        },
        tasks,
        tags: tags
            .into_iter()
            .map(|t| TagResponse {
                uuid: t.0.to_string(),
                name: t.1,
            })
            .collect(),
        permission,
        owner_username,
    }))
}

/// POST /api/v1/shared/:token/tasks
pub async fn create_shared_task(
    _auth: WebUser, // Must be authenticated, but task owned by board owner
    Path(token): Path<String>,
    Extension(pool): Extension<PgPool>,
    Json(req): Json<super::web::CreateTaskRequest>,
) -> Result<Json<TaskResponse>, AppError> {
    let (board_uuid, owner_id) = validate_edit_share(&pool, &token).await?;

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

    let due_date: Option<NaiveDate> = req
        .due_date
        .as_ref()
        .map(|d| d.parse::<NaiveDate>())
        .transpose()
        .map_err(|_| AppError::Validation("Invalid due_date format".into()))?;

    let task_uuid = Uuid::new_v4();
    let now = chrono::Utc::now().naive_utc();

    // Task owned by board owner, not the editing user
    sqlx::query(
        "INSERT INTO tasks (uuid, user_id, title, description, priority, column_name, \
         due_date, created_at, updated_at, board_uuid) \
         VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $8, $9)",
    )
    .bind(task_uuid)
    .bind(owner_id)
    .bind(req.title.trim())
    .bind(description)
    .bind(priority)
    .bind(column)
    .bind(due_date)
    .bind(now)
    .bind(board_uuid)
    .execute(&pool)
    .await?;

    // Associate tags (batch-validated against owner's tags)
    associate_tags(
        &pool,
        task_uuid,
        owner_id,
        &req.tag_uuids.unwrap_or_default(),
    )
    .await?;

    fetch_task_response(&pool, task_uuid).await
}

/// PATCH /api/v1/shared/:token/tasks/:uuid
pub async fn update_shared_task(
    _auth: WebUser,
    Path((token, task_uuid)): Path<(String, Uuid)>,
    Extension(pool): Extension<PgPool>,
    Json(req): Json<super::web::UpdateTaskRequest>,
) -> Result<Json<TaskResponse>, AppError> {
    let (board_uuid, owner_id) = validate_edit_share(&pool, &token).await?;

    // Verify task belongs to this board
    let task_exists: bool = sqlx::query_scalar(
        "SELECT EXISTS(SELECT 1 FROM tasks WHERE uuid = $1 AND board_uuid = $2 AND user_id = $3 AND deleted = FALSE)",
    )
    .bind(task_uuid)
    .bind(board_uuid)
    .bind(owner_id)
    .fetch_one(&pool)
    .await?;
    if !task_exists {
        return Err(AppError::NotFound("Task not found on this board".into()));
    }

    validate_update_fields(&req)?;
    apply_task_updates(&pool, task_uuid, owner_id, &req).await?;

    fetch_task_response(&pool, task_uuid).await
}

/// DELETE /api/v1/shared/:token/tasks/:uuid
pub async fn delete_shared_task(
    _auth: WebUser,
    Path((token, task_uuid)): Path<(String, Uuid)>,
    Extension(pool): Extension<PgPool>,
) -> Result<axum::http::StatusCode, AppError> {
    let (board_uuid, owner_id) = validate_edit_share(&pool, &token).await?;

    let result = sqlx::query(
        "UPDATE tasks SET deleted = TRUE, deleted_at = NOW(), updated_at = NOW() \
         WHERE uuid = $1 AND board_uuid = $2 AND user_id = $3 AND deleted = FALSE",
    )
    .bind(task_uuid)
    .bind(board_uuid)
    .bind(owner_id)
    .execute(&pool)
    .await?;

    if result.rows_affected() == 0 {
        return Err(AppError::NotFound("Task not found on this board".into()));
    }

    Ok(axum::http::StatusCode::NO_CONTENT)
}
