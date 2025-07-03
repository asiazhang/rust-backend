use axum::{
    http::StatusCode,
    response::{IntoResponse, Response},
};
use color_eyre::eyre::Error;
use redis::RedisError;
use thiserror::Error;
use validator::ValidationErrors;

/// 使用 [`thiserror`] 定义错误类型
/// 方便根据类型转换为相应的http错误码
#[derive(Error, Debug)]
pub enum AppError {
    /// 数据验证错误，这种错误通常都是用户参数不正确导致的，所以需要转换为403
    #[error(transparent)]
    ValidationFailed(#[from] ValidationErrors),

    /// 数据库错误
    #[error(transparent)]
    DatabaseError(#[from] sqlx::Error),

    #[error(transparent)]
    RedisError(#[from] RedisError),

    /// 其他类型错误
    #[error(transparent)]
    InternalError(#[from] Error),
}

/// Tell axum how to convert `AppError` into a response.
impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        match self {
            AppError::ValidationFailed(err) => {
                (StatusCode::BAD_REQUEST, format!("Validate failed: {err}")).into_response()
            }
            AppError::DatabaseError(err) => match err {
                sqlx::Error::RowNotFound => (
                    StatusCode::NOT_FOUND,
                    format!("Can not found resource: {err}"),
                )
                    .into_response(),
                _ => (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    format!("Database error: {err}"),
                )
                    .into_response(),
            },
            AppError::RedisError(err) => (
                StatusCode::INTERNAL_SERVER_ERROR,
                format!("Redis error: {err}"),
            )
                .into_response(),
            AppError::InternalError(err) => (
                StatusCode::INTERNAL_SERVER_ERROR,
                format!("Something went wrong: {err}"),
            )
                .into_response(),
        }
    }
}
