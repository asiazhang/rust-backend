use crate::models::common::Reply;
use crate::models::common::ReplyList;
use crate::models::projects::{ProjectCreate, ProjectInfo, ProjectSearch};
use axum::Json;
use tracing::debug;

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
pub async fn find_projects(Json(search): Json<ProjectSearch>) -> Json<ReplyList<ProjectInfo>> {
    debug!("Searching projects {:#?}", search);

    Json(ReplyList {
        data: vec![],
        total: 100,
        page_size: 20,
        page_index: 1,
    })
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
pub async fn create_project(Json(project): Json<ProjectCreate>) -> Json<Reply<ProjectInfo>> {
    debug!("Creating project {:#?}", project);

    Json(Reply {
        data: ProjectInfo {
            id: 1,
            name: project.project_name,
        },
    })
}

#[utoipa::path(get, path = "/projects/{id}", tag = "projects")]
pub async fn get_project() {}

#[utoipa::path(patch, path = "/projects/{id}", tag = "projects")]
pub async fn update_project() {}

#[utoipa::path(delete, path = "/projects/{id}", tag = "projects")]
pub async fn delete_project() {}
