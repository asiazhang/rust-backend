//! 项目相关接口
//!

use crate::models::common::{Reply, ReplyList};
use crate::models::err::AppError;
use crate::models::projects::{ProjectCreate, ProjectInfo, ProjectSearch, ProjectUpdate};
use crate::AppState;
use axum::extract::{Path, State};
use axum::Json;
use color_eyre::Result;
use database::ProjectRepositoryTrait;
use tracing::debug;
use validator::Validate;

/// 根据查询参数搜索项目
///
/// 根据查询参数搜索符合要求的项目列表，支持分页.
///
/// 查询参数由 [`ProjectSearch`] 参数决定，部分参数为可选参数。
///
/// 注意：**强烈建议**在handler上开启 [`axum::debug_handler`] 宏，否则错误提示信息可能不是很明确。
///
/// ## 参数
///
/// - state: 从路由函数传递给来的共享数据
/// - search: ProjectSearch类型数据
///
/// ## Json化
///
/// 通过`Json(search): Json<ProjectSearch>`这种语法，框架能自动将body数据反序列化为[`ProjectSearch`]对象，如果
/// 反序列化失败会直接返回400错误。
///
/// ## 返回值
///
/// 返回值的类型是 [`Result<Json<ReplyList<ProjectInfo>>, AppError>`]。
/// 在1.0.124其内部封装了以下几个关键：
///
/// 1. [`Result`] 使用 [`anyhow::Result`] 对返回结果进行封装，方便使用 `?` 进行错误传播
/// 2. [`Json`] 会对内部类型进行json序列化，保证返回的数据是一个合法的json字符串
/// 3. [`ReplyList`] 是我们封装的一个类型，表明结果是一个通用的`api-json`格式列表对象
/// 4. [`ProjectInfo`] 是实际的业务返回对象
/// 5. [`AppError`] 是错误时返回的Error类型，会自动转换为500错误信息
///
/// 使用case:
///
/// - 使用 `routes!(get, get, post)`
/// - 其中使用 r#""## 查看 quote原因，后续不会详细写
#[utoipa::path(post,
    path = "/search-projects",
    tag = "projects",
    request_body = ProjectSearch,
    responses(
        (status = 200, description = "Search results", body = ReplyList<ProjectInfo>)
    ),
)]
pub async fn find_projects<PR: ProjectRepositoryTrait>(
    State(state): State<AppState<PR>>,
    Json(search): Json<ProjectSearch>,
) -> Result<Json<ReplyList<ProjectInfo>>, AppError> {
    debug!("🔍 搜索项目 {:#?}", search);

    // 验证输入参数，确保有效性
    search.validate()?;

    // saturating_sub(1)会保证结果>=0，不会出现溢出
    let offset = (search.page_query.page_index.saturating_sub(1)) * search.page_query.page_size;

    // 获取项目仓库实例
    let project_repo = state.project_repository.clone();

    // 调用仓库方法执行搜索
    let result = project_repo
        .find_projects(search.project_name.clone(), search.page_query.page_size as i64, offset as i64)
        .await?;

    // 使用OK返回成功的结果
    Ok(Json(ReplyList {
        total: result.total,
        data: result.projects.into_iter().map(Into::into).collect(),
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
pub async fn create_project<PR: ProjectRepositoryTrait>(
    State(state): State<AppState<PR>>,
    Json(project): Json<ProjectCreate>,
) -> Result<Json<Reply<ProjectInfo>>, AppError> {
    debug!("Creating project {:#?}", project);

    // 获取项目仓库实例
    let project_repo = state.project_repository.clone();
    let db_project = database::models::ProjectCreate {
        project_name: project.project_name,
        comment: project.comment,
    };
    let project = project_repo.create_project(db_project).await?;

    Ok(Json(Reply { data: project.into() }))
}

/// 查询指定项目信息
#[utoipa::path(get, path = "/projects/{id}", tag = "projects")]
pub async fn get_project<PR: ProjectRepositoryTrait>(
    State(state): State<AppState<PR>>,
    Path(project_id): Path<i32>,
) -> Result<Json<ProjectInfo>, AppError> {
    debug!("Getting project id {:#?}", project_id);

    let project_repo = state.project_repository.clone();
    let project = project_repo.get_project_by_id(project_id).await?;

    Ok(Json(project.into()))
}

/// 更新项目信息
///
/// 根据用户指定的 `id` 和 修改信息 [`ProjectUpdate`] 来更新项目信息。
///
#[utoipa::path(patch, path = "/projects/{id}", tag = "projects")]
pub async fn update_project<PR: ProjectRepositoryTrait>(
    State(state): State<AppState<PR>>,
    Path(project_id): Path<i32>,
    Json(info): Json<ProjectUpdate>,
) -> Result<Json<ProjectInfo>, AppError> {
    debug!("Updating project {} with {:#?}", project_id, info);

    let project_repo = state.project_repository.clone();
    let db_update = database::models::ProjectUpdate {
        project_name: info.project_name,
        comment: info.comment,
    };
    let project = project_repo.update_project(project_id, db_update).await?;

    Ok(Json(project.into()))
}

/// 删除指定的项目
#[utoipa::path(delete, path = "/projects/{id}", tag = "projects")]
pub async fn delete_project<PR: ProjectRepositoryTrait>(
    State(state): State<AppState<PR>>,
    Path(project_id): Path<i32>,
) -> Result<Json<ProjectInfo>, AppError> {
    debug!("delete project {:#?}", project_id);

    let project_repo = state.project_repository.clone();
    let project = project_repo.delete_project(project_id).await?;

    Ok(Json(project.into()))
}
