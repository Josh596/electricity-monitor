use axum::{
    Json,
    http::StatusCode,
    response::{IntoResponse, Response},
};

pub enum AppError {
    NotFound(String),
    BadRequest(String),
    Internal(String),
    Unauthorized(String),
}

impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        let (status, message) = match self {
            AppError::NotFound(msg) => (StatusCode::NOT_FOUND, msg),
            AppError::BadRequest(msg) => (StatusCode::BAD_REQUEST, msg),
            AppError::Unauthorized(msg) => (StatusCode::UNAUTHORIZED, msg),
            AppError::Internal(msg) => {
                tracing::error!("Internal error: {msg}");
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    "Something went wrong".into(),
                )
            }
        };

        (status, Json(serde_json::json!({ "error": message }))).into_response()
    }
}

impl From<StatusCode> for AppError {
    fn from(status: StatusCode) -> Self {
        // e.g. "Unauthorized" for 401
        let message = status.canonical_reason().unwrap_or("Unknown error").to_string();

        match status {
            StatusCode::NOT_FOUND => AppError::NotFound(message),
            StatusCode::BAD_REQUEST => AppError::BadRequest(message),
            StatusCode::UNAUTHORIZED => AppError::Unauthorized(message),
            // No matching variant, so it can't keep its status; log it so it isn't lost
            _ => AppError::Internal(format!("Unmapped status code: {status}")),
        }
    }
}

impl From<sqlx::Error> for AppError {
    fn from(err: sqlx::Error) -> Self {
        match err {
            sqlx::Error::RowNotFound => AppError::NotFound("Resource not found".into()),
            _ => AppError::Internal(err.to_string()),
        }
    }
}
