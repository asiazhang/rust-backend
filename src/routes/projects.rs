use crate::models::app::AppState;
use crate::models::common::Reply;
use crate::models::common::ReplyList;
use crate::models::projects::{ProjectCreate, ProjectInfo, ProjectSearch};
use anyhow::Result;
use axum::extract::State;
use axum::Json;
use std::sync::Arc;
use tracing::debug;
use crate::models::err::AppError;

/// 根据查询参数搜索项目
///
/// 根据查询参数搜索符合要求的项目列表，支持分页.
#[utoipa::path(post,
    path = "/search-projects",
    tag = "projects",
    request_body = ProjectSearch,
    responses(
        (status = 200, description = "Search results", body = ReplyList<ProjectInfo>)
    ),
)]
pub async fn find_projects(
    State(state): State<Arc<AppState>>,
    Json(search): Json<ProjectSearch>,
) -> Result<Json<ReplyList<ProjectInfo>>, AppError> {
    debug!("Searching projects {:#?}", search);

    let projects = sqlx::query_as!(
        ProjectInfo,
        r#"
    select id, project_name from hm.projects
    "#
    )
    .fetch_all(&state.db_pool)
    .await?;

    Ok(Json(ReplyList {
        total: projects.len() as u32,
        data: projects,
        page_size: search.page_query.page_size,
        page_index: search.page_query.page_index,
    }))
}

/// 创建项目
///
/// 根据用户输入参数创建项目信息
#[utoipa::path(post,
    path = "/projects",
    tag = "projects",
    responses(
        (status = 200, description = "Create project result", body = Reply<ProjectInfo>)
    )
)]
pub async fn create_project(
    State(state): State<Arc<AppState>>,
    Json(project): Json<ProjectCreate>,
) -> Result<Json<Reply<ProjectInfo>>, AppError> {
    debug!("Creating project {:#?}", project);

    let project = sqlx::query_as!(
        ProjectInfo,
        r#"
insert into hm.projects (project_name)
values ($1)
returning id, project_name;
    "#,
        project.project_name
    )
    .fetch_one(&state.db_pool)
    .await?;

    Ok(Json(Reply { data: project }))
}

#[utoipa::path(get, path = "/projects/{id}", tag = "projects")]
pub async fn get_project(State(_state): State<Arc<AppState>>) {}

#[utoipa::path(patch, path = "/projects/{id}", tag = "projects")]
pub async fn update_project(State(_state): State<Arc<AppState>>) {}

#[utoipa::path(delete, path = "/projects/{id}", tag = "projects")]
pub async fn delete_project(State(_state): State<Arc<AppState>>) {}
