use axum::{
    Json,
    http::StatusCode,
    response::{IntoResponse, Response},
};
use serde_json::json;

#[derive(Debug, thiserror::Error)]
pub enum AppError {
    #[error("Database error: {0}")]
    Database(#[from] sqlx::Error),

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("Object not found: {0}")]
    NotFound(String),

    #[error("Unauthorized")]
    Unauthorized,

    #[error("Payload exceeds maximum allowed size: {0} bytes")]
    PayloadTooLarge(usize),

    #[allow(dead_code)]
    #[error("Internal server error")]
    Internal,
}

impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        let (status, message) = match self {
            AppError::NotFound(key) => {
                (StatusCode::NOT_FOUND, format!("Object not found: {}", key))
            }
            AppError::Unauthorized => (StatusCode::UNAUTHORIZED, "Unauthorized".to_string()),
            AppError::Database(e) => (
                StatusCode::INTERNAL_SERVER_ERROR,
                format!("Database error: {}", e),
            ),
            AppError::Io(e) => (
                StatusCode::INTERNAL_SERVER_ERROR,
                format!("IO error: {}", e),
            ),
            AppError::PayloadTooLarge(limit) => (
                StatusCode::PAYLOAD_TOO_LARGE,
                format!("Payload exceeds maximum allowed size: {} bytes", limit),
            ),
            AppError::Internal => (
                StatusCode::INTERNAL_SERVER_ERROR,
                "Internal server error".to_string(),
            ),
        };

        (
            status,
            Json(json!({
                "error": message,
                "server": "lila",
                "author": "april"
            })),
        )
            .into_response()
    }
}

pub type Result<T> = std::result::Result<T, AppError>;
