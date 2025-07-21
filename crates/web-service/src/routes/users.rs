use crate::{AppState, services::ProjectServiceTrait};
use axum::extract::{Path, State};

#[utoipa::path(post, path = "/search-users", tag = "users")]
pub async fn find_users<PS: ProjectServiceTrait>(State(_state): State<AppState<PS>>) {}

#[utoipa::path(get, path = "/users/{id}", tag = "users")]
pub async fn get_user<PS: ProjectServiceTrait>(State(_state): State<AppState<PS>>, Path(_id): Path<i32>) {}

#[utoipa::path(post, path = "/users", tag = "users")]
pub async fn create_user<PS: ProjectServiceTrait>(State(_state): State<AppState<PS>>) {}

#[utoipa::path(patch, path = "/users/{id}", tag = "users")]
pub async fn update_user<PS: ProjectServiceTrait>(State(_state): State<AppState<PS>>, Path(_id): Path<i32>) {}

#[utoipa::path(delete, path = "/users/{id}", tag = "users")]
pub async fn delete_user<PS: ProjectServiceTrait>(State(_state): State<AppState<PS>>, Path(_id): Path<i32>) {}
