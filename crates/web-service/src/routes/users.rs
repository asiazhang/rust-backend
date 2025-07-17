use crate::AppState;
use axum::extract::{Path, State};
use database::ProjectRepositoryTrait;

#[utoipa::path(post, path = "/search-users", tag = "users")]
pub async fn find_users<PR: ProjectRepositoryTrait>(State(_state): State<AppState<PR>>) {}

#[utoipa::path(get, path = "/users/{id}", tag = "users")]
pub async fn get_user<PR: ProjectRepositoryTrait>(State(_state): State<AppState<PR>>, Path(_id): Path<i32>) {}

#[utoipa::path(post, path = "/users", tag = "users")]
pub async fn create_user<PR: ProjectRepositoryTrait>(State(_state): State<AppState<PR>>) {}

#[utoipa::path(patch, path = "/users/{id}", tag = "users")]
pub async fn update_user<PR: ProjectRepositoryTrait>(State(_state): State<AppState<PR>>, Path(_id): Path<i32>) {}

#[utoipa::path(delete, path = "/users/{id}", tag = "users")]
pub async fn delete_user<PR: ProjectRepositoryTrait>(State(_state): State<AppState<PR>>, Path(_id): Path<i32>) {}
