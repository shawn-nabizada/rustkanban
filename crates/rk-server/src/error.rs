use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use axum::Json;

#[derive(Debug)]
#[allow(dead_code)]
pub enum AppError {
    Internal(String),
    Validation(String),
    LimitExceeded {
        resource: &'static str,
        current: i64,
        max: i64,
    },
    Unauthorized,
    Forbidden,
    NotFound(String),
}

impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        let (status, error, message) = match self {
            AppError::Internal(msg) => (
                StatusCode::INTERNAL_SERVER_ERROR,
                "internal_error".into(),
                msg,
            ),
            AppError::Validation(msg) => (
                StatusCode::UNPROCESSABLE_ENTITY,
                "validation_error".into(),
                msg,
            ),
            AppError::LimitExceeded {
                resource,
                current,
                max,
            } => {
                let display_name = format!("{}{}", &resource[..1].to_uppercase(), &resource[1..]);
                (
                    StatusCode::TOO_MANY_REQUESTS,
                    format!("{}_limit_exceeded", resource),
                    format!("{} limit reached ({}/{})", display_name, current, max),
                )
            }
            AppError::Unauthorized => (
                StatusCode::UNAUTHORIZED,
                "unauthorized".into(),
                "Authentication required".into(),
            ),
            AppError::Forbidden => (
                StatusCode::FORBIDDEN,
                "forbidden".into(),
                "Access denied".into(),
            ),
            AppError::NotFound(msg) => (StatusCode::NOT_FOUND, "not_found".into(), msg),
        };

        let body = serde_json::json!({
            "error": error,
            "message": message,
        });

        (status, Json(body)).into_response()
    }
}

impl From<sqlx::Error> for AppError {
    fn from(e: sqlx::Error) -> Self {
        tracing::error!("Database error: {}", e);
        AppError::Internal("Database error".into())
    }
}
