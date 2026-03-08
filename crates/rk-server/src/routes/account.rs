use axum::{extract::Path, Extension, Json};
use serde::Serialize;
use sqlx::PgPool;
use uuid::Uuid;

use crate::auth::AuthUser;
use crate::error::AppError;

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
pub async fn revoke_device(
    auth: AuthUser,
    Path(device_id): Path<Uuid>,
    Extension(pool): Extension<PgPool>,
) -> Result<Json<serde_json::Value>, AppError> {
    // Verify device belongs to user
    let exists: bool =
        sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM devices WHERE id = $1 AND user_id = $2)")
            .bind(device_id)
            .bind(auth.user_id)
            .fetch_one(&pool)
            .await?;

    if !exists {
        return Err(AppError::Validation("Device not found".into()));
    }

    // Delete auth token for this device
    sqlx::query("DELETE FROM auth_tokens WHERE device_id = $1")
        .bind(device_id)
        .execute(&pool)
        .await?;

    // Delete device
    sqlx::query("DELETE FROM devices WHERE id = $1 AND user_id = $2")
        .bind(device_id)
        .bind(auth.user_id)
        .execute(&pool)
        .await?;

    Ok(Json(serde_json::json!({"status": "ok"})))
}

/// `POST /account/delete` -- Delete the authenticated user's account and all data.
pub async fn delete_account(
    auth: AuthUser,
    Extension(pool): Extension<PgPool>,
) -> Result<Json<serde_json::Value>, AppError> {
    // Cascade delete handles everything (devices, auth_tokens, tasks, tags, task_tags)
    sqlx::query("DELETE FROM users WHERE id = $1")
        .bind(auth.user_id)
        .execute(&pool)
        .await?;

    Ok(Json(serde_json::json!({"status": "deleted"})))
}
