use axum::{
    http::StatusCode,
    response::{IntoResponse, Response},
};
use thiserror::Error;
use validator::ValidationErrors;

/// 使用 [`thiserror`] 定义错误类型
/// 方便根据类型转换为相应的http错误码
#[derive(Error, Debug)]
pub enum AppError {
    #[error(transparent)]
    ValidationFailed(#[from] ValidationErrors),

    #[error(transparent)]
    DatabaseError(#[from] sqlx::Error),

    #[error(transparent)]
    InternalError(#[from] anyhow::Error),
}

/// Tell axum how to convert `AppError` into a response.
impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        match self {
            AppError::ValidationFailed(err) => {
                (StatusCode::BAD_REQUEST, format!("Validate failed: {}", err)).into_response()
            }
            AppError::DatabaseError(err) => {
                (StatusCode::BAD_REQUEST, format!("Database error: {}", err)).into_response()
            }
            AppError::InternalError(err) => (
                StatusCode::INTERNAL_SERVER_ERROR,
                format!("Something went wrong: {}", err),
            )
                .into_response(),
        }
    }
}
