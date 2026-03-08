use axum::extract::FromRequestParts;
use axum::http::request::Parts;
use axum::http::StatusCode;
use sha2::{Digest, Sha256};
use sqlx::PgPool;
use uuid::Uuid;

/// Authenticated user extracted from Bearer token in the Authorization header.
/// Use as an Axum extractor on protected routes.
#[derive(Debug, Clone)]
#[allow(dead_code)]
pub struct AuthUser {
    pub user_id: Uuid,
    pub device_id: Uuid,
}

impl<S: Send + Sync> FromRequestParts<S> for AuthUser {
    type Rejection = StatusCode;

    async fn from_request_parts(parts: &mut Parts, _state: &S) -> Result<Self, Self::Rejection> {
        let pool = parts
            .extensions
            .get::<PgPool>()
            .ok_or(StatusCode::INTERNAL_SERVER_ERROR)?;

        let token = parts
            .headers
            .get("Authorization")
            .and_then(|v| v.to_str().ok())
            .and_then(|v| v.strip_prefix("Bearer "))
            .ok_or(StatusCode::UNAUTHORIZED)?;

        let hash = hash_token(token);

        let record = sqlx::query_as::<_, (Uuid, Uuid)>(
            "SELECT user_id, device_id FROM auth_tokens \
             WHERE token_hash = $1 AND (expires_at IS NULL OR expires_at > NOW())",
        )
        .bind(&hash)
        .fetch_optional(pool)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
        .ok_or(StatusCode::UNAUTHORIZED)?;

        // Refresh token expiry (90-day sliding window)
        let _ = sqlx::query(
            "UPDATE auth_tokens SET expires_at = NOW() + INTERVAL '90 days' WHERE token_hash = $1",
        )
        .bind(&hash)
        .execute(pool)
        .await;

        Ok(AuthUser {
            user_id: record.0,
            device_id: record.1,
        })
    }
}

/// SHA-256 hash a raw bearer token for storage/lookup.
pub fn hash_token(token: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(token.as_bytes());
    format!("{:x}", hasher.finalize())
}

/// Generate a cryptographically random bearer token with `rk_` prefix.
pub fn generate_token() -> String {
    use rand::Rng;
    let random_bytes: Vec<u8> = (0..32).map(|_| rand::thread_rng().gen()).collect();
    format!("rk_{}", hex::encode(random_bytes))
}
