use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use axum::Json;

#[derive(Debug)]
#[allow(dead_code)]
pub enum AppError {
    Internal(String),
    Validation(String),
    Unauthorized,
    Forbidden,
}

impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        let (status, error, message) = match self {
            AppError::Internal(msg) => (StatusCode::INTERNAL_SERVER_ERROR, "internal_error", msg),
            AppError::Validation(msg) => {
                (StatusCode::UNPROCESSABLE_ENTITY, "validation_error", msg)
            }
            AppError::Unauthorized => (
                StatusCode::UNAUTHORIZED,
                "unauthorized",
                "Authentication required".into(),
            ),
            AppError::Forbidden => (StatusCode::FORBIDDEN, "forbidden", "Access denied".into()),
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
