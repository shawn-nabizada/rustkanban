use axum::extract::{Form, Path};
use axum::http::header;
use axum::response::{IntoResponse, Redirect, Response};
use axum::{Extension, Json};
use chrono::NaiveDate;
use serde::{Deserialize, Serialize};
use sqlx::PgPool;
use tower_sessions::Session;
use uuid::Uuid;

use crate::auth::{generate_token, hash_token, AuthUser};
use crate::error::AppError;
use crate::session::{session_user_id, set_flash, validate_csrf, CsrfForm};

#[derive(Serialize)]
pub struct DeviceInfo {
    pub id: String,
    pub name: String,
    pub last_synced_at: Option<String>,
    pub stale: bool,
    pub created_at: String,
}

/// `GET /account/devices` -- List the authenticated user's devices.
pub async fn list_devices(
    auth: AuthUser,
    Extension(pool): Extension<PgPool>,
) -> Result<Json<Vec<DeviceInfo>>, AppError> {
    let rows = sqlx::query_as::<_, (Uuid, String, Option<String>, bool, String)>(
        "SELECT id, name, \
         to_char(last_synced_at, 'YYYY-MM-DD\"T\"HH24:MI:SS'), \
         stale, \
         to_char(created_at, 'YYYY-MM-DD\"T\"HH24:MI:SS') \
         FROM devices WHERE user_id = $1 ORDER BY created_at",
    )
    .bind(auth.user_id)
    .fetch_all(&pool)
    .await?;

    let devices = rows
        .into_iter()
        .map(|r| DeviceInfo {
            id: r.0.to_string(),
            name: r.1,
            last_synced_at: r.2,
            stale: r.3,
            created_at: r.4,
        })
        .collect();

    Ok(Json(devices))
}

/// `POST /account/devices/:id/revoke` -- Revoke a device and its auth token.
///
/// Browser form submission — always redirects back to /account (or /login on auth failure).
pub async fn revoke_device(
    session: Session,
    Path(device_id): Path<Uuid>,
    Extension(pool): Extension<PgPool>,
    Form(form): Form<CsrfForm>,
) -> Response {
    let user_id = match session_user_id(&session).await {
        Some(id) => id,
        None => return Redirect::temporary("/login").into_response(),
    };
    if !validate_csrf(&session, &form.csrf_token).await {
        return Redirect::to("/account").into_response();
    }

    // Delete auth tokens for this device (ownership enforced via subquery)
    if let Err(e) = sqlx::query(
        "DELETE FROM auth_tokens WHERE device_id = $1 \
         AND device_id IN (SELECT id FROM devices WHERE id = $1 AND user_id = $2)",
    )
    .bind(device_id)
    .bind(user_id)
    .execute(&pool)
    .await
    {
        tracing::error!("Failed to delete auth tokens for device {device_id}: {e}");
    }

    // Delete device (user_id check ensures ownership)
    if let Err(e) = sqlx::query("DELETE FROM devices WHERE id = $1 AND user_id = $2")
        .bind(device_id)
        .bind(user_id)
        .execute(&pool)
        .await
    {
        tracing::error!("Failed to delete device {device_id}: {e}");
    }

    set_flash(&session, "success", "Device revoked").await;
    Redirect::to("/account").into_response()
}

#[derive(Deserialize)]
pub struct RenameDeviceForm {
    pub csrf_token: String,
    pub name: String,
}

/// `POST /account/devices/:id/rename` -- Rename a device.
pub async fn rename_device(
    session: Session,
    Path(device_id): Path<Uuid>,
    Extension(pool): Extension<PgPool>,
    Form(form): Form<RenameDeviceForm>,
) -> Response {
    let user_id = match session_user_id(&session).await {
        Some(id) => id,
        None => return Redirect::temporary("/login").into_response(),
    };
    if !validate_csrf(&session, &form.csrf_token).await {
        return Redirect::to("/account").into_response();
    }

    let name = form.name.trim();
    if name.is_empty() || name.len() > 100 {
        set_flash(&session, "error", "Device name must be 1-100 characters").await;
        return Redirect::to("/account").into_response();
    }

    if let Err(e) = sqlx::query("UPDATE devices SET name = $1 WHERE id = $2 AND user_id = $3")
        .bind(name)
        .bind(device_id)
        .bind(user_id)
        .execute(&pool)
        .await
    {
        tracing::error!("Failed to rename device {device_id}: {e}");
        set_flash(&session, "error", "Failed to rename device").await;
        return Redirect::to("/account").into_response();
    }

    set_flash(&session, "success", "Device renamed").await;
    Redirect::to("/account").into_response()
}

/// `POST /account/delete` -- Delete the authenticated user's account and all data.
///
/// Browser form submission — always redirects (to / on success, /login on auth failure).
pub async fn delete_account(
    session: Session,
    Extension(pool): Extension<PgPool>,
    Form(form): Form<CsrfForm>,
) -> Response {
    let user_id = match session_user_id(&session).await {
        Some(id) => id,
        None => return Redirect::temporary("/login").into_response(),
    };
    if !validate_csrf(&session, &form.csrf_token).await {
        return Redirect::to("/account").into_response();
    }

    // Cascade delete handles everything (devices, auth_tokens, tasks, tags, task_tags)
    if let Err(e) = sqlx::query("DELETE FROM users WHERE id = $1")
        .bind(user_id)
        .execute(&pool)
        .await
    {
        tracing::error!("Failed to delete account {user_id}: {e}");
        set_flash(&session, "error", "Failed to delete account").await;
        return Redirect::to("/account").into_response();
    }

    let _ = session.delete().await;
    Redirect::to("/").into_response()
}

#[derive(Deserialize)]
pub struct CreateTokenForm {
    pub csrf_token: String,
    pub label: String,
    pub expires: String,
}

/// `POST /account/tokens/create` -- Create a new API token.
pub async fn create_api_token(
    session: Session,
    Extension(pool): Extension<PgPool>,
    Form(form): Form<CreateTokenForm>,
) -> Response {
    let user_id = match session_user_id(&session).await {
        Some(id) => id,
        None => return Redirect::temporary("/login").into_response(),
    };
    if !validate_csrf(&session, &form.csrf_token).await {
        return Redirect::to("/account").into_response();
    }

    let label = form.label.trim();
    if label.is_empty() || label.len() > 100 {
        set_flash(&session, "error", "Label must be 1-100 characters").await;
        return Redirect::to("/account").into_response();
    }

    let count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM api_tokens WHERE user_id = $1")
        .bind(user_id)
        .fetch_one(&pool)
        .await
        .unwrap_or(0);
    if count >= 10 {
        set_flash(&session, "error", "API token limit reached (max 10)").await;
        return Redirect::to("/account").into_response();
    }

    let expires_at: Option<chrono::NaiveDateTime> = match form.expires.as_str() {
        "30" => Some((chrono::Utc::now() + chrono::Duration::days(30)).naive_utc()),
        "90" => Some((chrono::Utc::now() + chrono::Duration::days(90)).naive_utc()),
        _ => None,
    };

    let raw_token = generate_token();
    let token_hash = hash_token(&raw_token);
    let token_id = Uuid::new_v4();

    let result = sqlx::query(
        "INSERT INTO api_tokens (id, user_id, token_hash, label, expires_at) \
         VALUES ($1, $2, $3, $4, $5)",
    )
    .bind(token_id)
    .bind(user_id)
    .bind(&token_hash)
    .bind(label)
    .bind(expires_at)
    .execute(&pool)
    .await;

    if let Err(e) = result {
        tracing::error!("API token creation failed: {e}");
        set_flash(&session, "error", "Failed to create token").await;
        return Redirect::to("/account").into_response();
    }

    let _ = session.insert("new_api_token", &raw_token).await;
    set_flash(&session, "success", "API token created").await;
    Redirect::to("/account").into_response()
}

/// `POST /account/tokens/:id/revoke` -- Revoke an API token.
pub async fn revoke_api_token(
    session: Session,
    Path(token_id): Path<Uuid>,
    Extension(pool): Extension<PgPool>,
    Form(form): Form<CsrfForm>,
) -> Response {
    let user_id = match session_user_id(&session).await {
        Some(id) => id,
        None => return Redirect::temporary("/login").into_response(),
    };
    if !validate_csrf(&session, &form.csrf_token).await {
        return Redirect::to("/account").into_response();
    }

    if let Err(e) = sqlx::query("DELETE FROM api_tokens WHERE id = $1 AND user_id = $2")
        .bind(token_id)
        .bind(user_id)
        .execute(&pool)
        .await
    {
        tracing::error!("Failed to revoke API token {token_id}: {e}");
        set_flash(&session, "error", "Failed to revoke token").await;
        return Redirect::to("/account").into_response();
    }

    set_flash(&session, "success", "API token revoked").await;
    Redirect::to("/account").into_response()
}

/// `GET /account/export` -- Download all tasks as JSON.
pub async fn export_data(session: Session, Extension(pool): Extension<PgPool>) -> Response {
    let user_id = match session_user_id(&session).await {
        Some(id) => id,
        None => return Redirect::temporary("/login").into_response(),
    };

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
        .bind(user_id)
        .fetch_all(&pool),
        sqlx::query_as::<_, (Uuid, String)>(
            "SELECT uuid, name FROM tags WHERE user_id = $1 AND deleted = FALSE",
        )
        .bind(user_id)
        .fetch_all(&pool),
        sqlx::query_as::<_, (Uuid, String, i32)>(
            "SELECT uuid, name, position FROM boards WHERE user_id = $1 AND deleted = FALSE ORDER BY position",
        )
        .bind(user_id)
        .fetch_all(&pool),
    );

    let tasks = match task_result {
        Ok(rows) => rows,
        Err(e) => {
            tracing::error!("Export tasks query failed: {e}");
            set_flash(&session, "error", "Export failed").await;
            return Redirect::to("/account").into_response();
        }
    };

    let tag_rows = match tag_result {
        Ok(rows) => rows,
        Err(e) => {
            tracing::error!("Export tags query failed: {e}");
            set_flash(&session, "error", "Export failed").await;
            return Redirect::to("/account").into_response();
        }
    };

    let board_rows = match board_result {
        Ok(rows) => rows,
        Err(e) => {
            tracing::error!("Export boards query failed: {e}");
            set_flash(&session, "error", "Export failed").await;
            return Redirect::to("/account").into_response();
        }
    };

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

    (
        [
            (header::CONTENT_TYPE, "application/json"),
            (
                header::CONTENT_DISPOSITION,
                "attachment; filename=\"rustkanban-export.json\"",
            ),
        ],
        json,
    )
        .into_response()
}
