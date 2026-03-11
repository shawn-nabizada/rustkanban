use axum::extract::Path;
use axum::http::header;
use axum::response::IntoResponse;
use axum::{Extension, Json};
use chrono::NaiveDate;
use serde::{Deserialize, Serialize};
use sqlx::PgPool;
use uuid::Uuid;

use crate::auth::{generate_token, hash_token, WebUser};
use crate::error::AppError;
use crate::TS_FMT;

// ───────────────────────────── Response types ────────────────────────────────

#[derive(Serialize)]
pub struct DeviceResponse {
    pub id: String,
    pub name: String,
    pub last_synced_at: Option<String>,
    pub stale: bool,
    pub created_at: String,
}

#[derive(Serialize)]
pub struct ApiTokenResponse {
    pub id: String,
    pub label: String,
    pub last_used_at: Option<String>,
    pub expires_at: Option<String>,
    pub created_at: String,
}

#[derive(Serialize)]
pub struct ApiTokenCreated {
    pub id: String,
    pub label: String,
    pub token: String,
    pub expires_at: Option<String>,
}

// ───────────────────────────── Request types ─────────────────────────────────

#[derive(Deserialize)]
pub struct RenameDeviceRequest {
    pub name: String,
}

#[derive(Deserialize)]
pub struct CreateTokenRequest {
    pub label: String,
    pub expires: String, // "30", "90", or "never"
}

// ───────────────────────────── Devices ───────────────────────────────────────

/// GET /api/v1/account/devices
pub async fn list_devices(
    auth: WebUser,
    Extension(pool): Extension<PgPool>,
) -> Result<Json<Vec<DeviceResponse>>, AppError> {
    let rows = sqlx::query_as::<_, (Uuid, String, Option<String>, bool, String)>(&format!(
        "SELECT id, name, to_char(last_synced_at, '{TS_FMT}'), stale, to_char(created_at, '{TS_FMT}') \
         FROM devices WHERE user_id = $1 ORDER BY created_at"
    ))
    .bind(auth.user_id)
    .fetch_all(&pool)
    .await?;

    Ok(Json(
        rows.into_iter()
            .map(|r| DeviceResponse {
                id: r.0.to_string(),
                name: r.1,
                last_synced_at: r.2,
                stale: r.3,
                created_at: r.4,
            })
            .collect(),
    ))
}

/// PATCH /api/v1/account/devices/:id
pub async fn rename_device(
    auth: WebUser,
    Path(device_id): Path<Uuid>,
    Extension(pool): Extension<PgPool>,
    Json(req): Json<RenameDeviceRequest>,
) -> Result<axum::http::StatusCode, AppError> {
    let name = req.name.trim();
    if name.is_empty() || name.len() > 100 {
        return Err(AppError::Validation(
            "Device name must be 1-100 characters".into(),
        ));
    }

    let result = sqlx::query("UPDATE devices SET name = $1 WHERE id = $2 AND user_id = $3")
        .bind(name)
        .bind(device_id)
        .bind(auth.user_id)
        .execute(&pool)
        .await?;

    if result.rows_affected() == 0 {
        return Err(AppError::NotFound("Device not found".into()));
    }

    Ok(axum::http::StatusCode::NO_CONTENT)
}

/// DELETE /api/v1/account/devices/:id
pub async fn revoke_device(
    auth: WebUser,
    Path(device_id): Path<Uuid>,
    Extension(pool): Extension<PgPool>,
) -> Result<axum::http::StatusCode, AppError> {
    // Delete auth tokens for this device (ownership enforced via subquery)
    sqlx::query(
        "DELETE FROM auth_tokens WHERE device_id = $1 \
         AND device_id IN (SELECT id FROM devices WHERE id = $1 AND user_id = $2)",
    )
    .bind(device_id)
    .bind(auth.user_id)
    .execute(&pool)
    .await?;

    let result = sqlx::query("DELETE FROM devices WHERE id = $1 AND user_id = $2")
        .bind(device_id)
        .bind(auth.user_id)
        .execute(&pool)
        .await?;

    if result.rows_affected() == 0 {
        return Err(AppError::NotFound("Device not found".into()));
    }

    Ok(axum::http::StatusCode::NO_CONTENT)
}

// ───────────────────────────── API Tokens ────────────────────────────────────

/// GET /api/v1/account/tokens
pub async fn list_tokens(
    auth: WebUser,
    Extension(pool): Extension<PgPool>,
) -> Result<Json<Vec<ApiTokenResponse>>, AppError> {
    let rows =
        sqlx::query_as::<_, (Uuid, String, Option<String>, Option<String>, String)>(&format!(
            "SELECT id, label, to_char(last_used_at, '{TS_FMT}'), \
             to_char(expires_at, '{TS_FMT}'), to_char(created_at, '{TS_FMT}') \
             FROM api_tokens WHERE user_id = $1 ORDER BY created_at"
        ))
        .bind(auth.user_id)
        .fetch_all(&pool)
        .await?;

    Ok(Json(
        rows.into_iter()
            .map(|r| ApiTokenResponse {
                id: r.0.to_string(),
                label: r.1,
                last_used_at: r.2,
                expires_at: r.3,
                created_at: r.4,
            })
            .collect(),
    ))
}

/// POST /api/v1/account/tokens
pub async fn create_token(
    auth: WebUser,
    Extension(pool): Extension<PgPool>,
    Json(req): Json<CreateTokenRequest>,
) -> Result<Json<ApiTokenCreated>, AppError> {
    let label = req.label.trim().to_string();
    if label.is_empty() || label.len() > 100 {
        return Err(AppError::Validation(
            "Label must be 1-100 characters".into(),
        ));
    }

    let count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM api_tokens WHERE user_id = $1")
        .bind(auth.user_id)
        .fetch_one(&pool)
        .await?;
    if count >= 10 {
        return Err(AppError::Validation(
            "API token limit reached (max 10)".into(),
        ));
    }

    let expires_at: Option<chrono::NaiveDateTime> = match req.expires.as_str() {
        "30" => Some((chrono::Utc::now() + chrono::Duration::days(30)).naive_utc()),
        "90" => Some((chrono::Utc::now() + chrono::Duration::days(90)).naive_utc()),
        _ => None,
    };

    let raw_token = generate_token();
    let token_hash = hash_token(&raw_token);
    let token_id = Uuid::new_v4();

    sqlx::query(
        "INSERT INTO api_tokens (id, user_id, token_hash, label, expires_at) \
         VALUES ($1, $2, $3, $4, $5)",
    )
    .bind(token_id)
    .bind(auth.user_id)
    .bind(&token_hash)
    .bind(&label)
    .bind(expires_at)
    .execute(&pool)
    .await?;

    Ok(Json(ApiTokenCreated {
        id: token_id.to_string(),
        label,
        token: raw_token,
        expires_at: expires_at.map(|d| d.format("%Y-%m-%dT%H:%M:%S").to_string()),
    }))
}

/// DELETE /api/v1/account/tokens/:id
pub async fn revoke_token(
    auth: WebUser,
    Path(token_id): Path<Uuid>,
    Extension(pool): Extension<PgPool>,
) -> Result<axum::http::StatusCode, AppError> {
    let result = sqlx::query("DELETE FROM api_tokens WHERE id = $1 AND user_id = $2")
        .bind(token_id)
        .bind(auth.user_id)
        .execute(&pool)
        .await?;

    if result.rows_affected() == 0 {
        return Err(AppError::NotFound("API token not found".into()));
    }

    Ok(axum::http::StatusCode::NO_CONTENT)
}

// ───────────────────────────── Export ─────────────────────────────────────────

/// GET /api/v1/account/export
pub async fn export_data(
    auth: WebUser,
    Extension(pool): Extension<PgPool>,
) -> Result<impl IntoResponse, AppError> {
    let (task_result, tag_result, board_result) = tokio::join!(
        sqlx::query_as::<
            _,
            (
                Uuid,
                String,
                String,
                String,
                String,
                Option<NaiveDate>,
                Option<Vec<String>>,
            ),
        >(
            "SELECT t.uuid, t.title, t.description, t.priority, t.column_name, t.due_date, \
             COALESCE(array_agg(tg.name) FILTER (WHERE tg.name IS NOT NULL), ARRAY[]::text[]) \
             FROM tasks t \
             LEFT JOIN task_tags tt ON tt.task_uuid = t.uuid \
             LEFT JOIN tags tg ON tg.uuid = tt.tag_uuid AND tg.deleted = FALSE \
             WHERE t.user_id = $1 AND t.deleted = FALSE \
             GROUP BY t.uuid",
        )
        .bind(auth.user_id)
        .fetch_all(&pool),
        sqlx::query_as::<_, (Uuid, String)>(
            "SELECT uuid, name FROM tags WHERE user_id = $1 AND deleted = FALSE",
        )
        .bind(auth.user_id)
        .fetch_all(&pool),
        sqlx::query_as::<_, (Uuid, String, i32)>(
            "SELECT uuid, name, position FROM boards WHERE user_id = $1 AND deleted = FALSE ORDER BY position",
        )
        .bind(auth.user_id)
        .fetch_all(&pool),
    );

    let tasks = task_result.map_err(|e| AppError::Internal(format!("Export failed: {e}")))?;
    let tag_rows = tag_result.map_err(|e| AppError::Internal(format!("Export failed: {e}")))?;
    let board_rows = board_result.map_err(|e| AppError::Internal(format!("Export failed: {e}")))?;

    let export = serde_json::json!({
        "version": 2,
        "tasks": tasks.iter().map(|t| {
            serde_json::json!({
                "uuid": t.0.to_string(),
                "title": t.1,
                "description": t.2,
                "priority": t.3,
                "column": t.4,
                "due_date": t.5.map(|d| d.format("%Y-%m-%d").to_string()),
                "tags": t.6.as_ref().map(|v| v.iter().filter(|s| !s.is_empty()).collect::<Vec<_>>()).unwrap_or_default(),
            })
        }).collect::<Vec<_>>(),
        "tags": tag_rows.iter().map(|t| {
            serde_json::json!({
                "uuid": t.0.to_string(),
                "name": t.1,
            })
        }).collect::<Vec<_>>(),
        "boards": board_rows.iter().map(|b| {
            serde_json::json!({
                "uuid": b.0.to_string(),
                "name": b.1,
                "position": b.2,
            })
        }).collect::<Vec<_>>(),
    });

    let json = serde_json::to_string_pretty(&export).unwrap_or_default();

    Ok((
        [
            (header::CONTENT_TYPE, "application/json"),
            (
                header::CONTENT_DISPOSITION,
                "attachment; filename=\"rustkanban-export.json\"",
            ),
        ],
        json,
    ))
}

// ───────────────────────────── Account Management ────────────────────────────

/// DELETE /api/v1/account
pub async fn delete_account(
    auth: WebUser,
    session: tower_sessions::Session,
    Extension(pool): Extension<PgPool>,
) -> Result<axum::http::StatusCode, AppError> {
    sqlx::query("DELETE FROM users WHERE id = $1")
        .bind(auth.user_id)
        .execute(&pool)
        .await?;

    let _ = session.delete().await;
    Ok(axum::http::StatusCode::NO_CONTENT)
}

/// POST /api/v1/auth/logout
pub async fn logout(session: tower_sessions::Session) -> Result<axum::http::StatusCode, AppError> {
    let _ = session.delete().await;
    Ok(axum::http::StatusCode::NO_CONTENT)
}
