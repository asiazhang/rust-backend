//! 项目相关接口
//!

use crate::models::app::AppState;
use crate::models::common::Reply;
use crate::models::common::ReplyList;
use crate::models::err::AppError;
use crate::models::projects::{ProjectCreate, ProjectInfo, ProjectSearch, ProjectUpdate};
use axum::extract::{Path, State};
use axum::Json;
use color_eyre::Result;
use std::sync::Arc;
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
/// 返回值的类型是 [`Result<Json<ReplyList<ProjectInfo>>, AppError>`] 初次接触时可能会比较复杂，我们一层层解释下：
///
/// 1. [`Result`] 使用 [`anyhow::Result`] 对返回结果进行封装，方便使用 `?` 进行错误传播
/// 2. [`Json`] 会对内部类型进行json序列化，保证返回的数据是一个合法的json字符串
/// 3. [`ReplyList`] 是我们封装的一个类型，表明结果是一个通用的`api-json`格式列表对象
/// 4. [`ProjectInfo`] 是实际的业务返回对象
/// 5. [`AppError`] 是错误时返回的Error类型，会自动转换为500错误信息
///
/// ## SQLx
///
/// 这里通过sqlx来组装sql查询语句，特点如下：
///
/// ### 静态宏检查
/// [`sqlx::query`]宏会在编译期会对sql语句进行检查（发送到数据库），确保sql的有效性。
///
/// ### 分页查询
/// 通过`COUNT(*) OVER () as total_count` + `WITH` 语法，可以一次性将数据和总数都查询出来，很方便。
///
/// ### 条件查询
/// 在以往我们经常会遇到这种场景：用户如果传入A，则进行过滤，否则不过滤，比如模糊搜索：
///
/// 在`postgres`中可以这样实现:
///
/// ```
/// WHERE (COALESCE($1, '') = '' OR project_name LIKE $2)
/// ```
///
/// $1和$2基本上是一个值。生成的查询语句如下：
///
/// - 如果是None，那么会展开为：`WHERE '' = '' OR project_name LIKE %%`，这个会被简化为真值，直接被查询引擎优化掉
/// - 如果是字符串A，那么会展开为: `WHERE 'A' = '' OR project_name LIKE %A%`，这个会被简化为`WHERE project_name LIKE %A%`
///
/// 所以我们不用再像其他框架里面使用`if`来拼接了！
#[utoipa::path(post,
    path = "/search-projects",
    tag = "projects",
    request_body = ProjectSearch,
    responses(
        (status = 200, description = "Search results", body = ReplyList<ProjectInfo>)
    ),
)]
#[axum::debug_handler]
pub async fn find_projects(
    State(state): State<Arc<AppState>>,
    Json(search): Json<ProjectSearch>,
) -> Result<Json<ReplyList<ProjectInfo>>, AppError> {
    debug!("Searching projects {:#?}", search);

    // 验证输入参数，确保有效性
    search.validate()?;

    // 这里name需要clone一次，因为后面会使用两次name，导致重复消费
    let name = search.project_name.clone();

    // saturating_sub(1)会保证结果>=0，不会出现溢出
    let offset = (search.page_query.page_index.saturating_sub(1)) * search.page_query.page_size;

    // 具体sqlx的好处可以参考上面的注释
    let rows = sqlx::query!(
        r#"
WITH filtered_projects AS (SELECT id,
                                  project_name,
                                  comment,
                                  COUNT(*) OVER () as total_count
                           FROM hm.projects
                           WHERE (COALESCE($1, '') = '' OR project_name LIKE $2)
                           LIMIT $3 OFFSET $4)
SELECT id,
       project_name,
       comment,
       total_count
FROM filtered_projects;
    "#,
        name.unwrap_or("".to_string()),
        search
            .project_name
            .map(|n| format!("%{}%", n))
            .unwrap_or_default(),
        search.page_query.page_size as i64,
        offset as i64,
    )
    .fetch_all(&state.db_pool)
    .await?; // 异步操作需要使用.await，?会将错误直接传播到上层，转换为http错误

    // 获取总数和分页数据
    let total = rows.first().and_then(|r| r.total_count).unwrap_or(0) as u32;
    let projects = rows
        .into_iter() // Vec<Record>转换为迭代器
        .map(|r| ProjectInfo {
            id: r.id,
            project_name: r.project_name,
            comment: r.comment,
        }) // 使用map将数据库Record对象转换为ProjectInfo对象
        .collect(); // 转换回Vec

    // 使用OK返回成功的结果
    Ok(Json(ReplyList {
        total,
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
#[axum::debug_handler]
pub async fn create_project(
    State(state): State<Arc<AppState>>,
    Json(project): Json<ProjectCreate>,
) -> Result<Json<Reply<ProjectInfo>>, AppError> {
    debug!("Creating project {:#?}", project);

    // query_as!可以直接将Record结果对象转换为类型对象
    let project = sqlx::query_as!(
        ProjectInfo,
        r#"
insert into hm.projects (project_name, comment, created_at, updated_at)
values ($1, $2, now(), now())
returning id, project_name, comment;
    "#,
        project.project_name,
        project.comment
    )
    .fetch_one(&state.db_pool)
    .await?;

    Ok(Json(Reply { data: project }))
}

/// 查询指定项目信息
#[utoipa::path(get, path = "/projects/{id}", tag = "projects")]
#[axum::debug_handler]
pub async fn get_project(
    State(_state): State<Arc<AppState>>,
    Path(project_id): Path<i32>,
) -> Result<Json<ProjectInfo>, AppError> {
    debug!("Creating project id {:#?}", project_id);

    let project = sqlx::query_as!(
        ProjectInfo,
        r#"
select id, project_name, comment from hm.projects
where id = $1
limit 1
    "#,
        project_id
    )
    .fetch_one(&_state.db_pool)
    .await?;

    Ok(Json(project))
}

/// 更新项目信息
///
/// 根据用户指定的 `id` 和 修改信息 [`ProjectUpdate`] 来更新项目信息。
///
/// ## Sql
///
/// 由于更新数据中的字段大部分都是[`Option`]，因此我们使用了`postgresql`中的`coalesce`函数，如果用户输入的值
/// 为None，那么会被转换为数据库的null，最终被转换为之前值。
///
/// 两个好处：
/// - 防止前端输入了空数据，导致数据被误清除
/// - 不用`if`拼接的方式，代码可维护性更好
///
#[utoipa::path(patch, path = "/projects/{id}", tag = "projects")]
#[axum::debug_handler]
pub async fn update_project(
    State(state): State<Arc<AppState>>,
    Path(project_id): Path<i32>,
    Json(info): Json<ProjectUpdate>,
) -> Result<Json<ProjectInfo>, AppError> {
    debug!("Updating project {} with {:#?}", project_id, info);

    let project = sqlx::query_as!(
        ProjectInfo,
        r#"
update hm.projects
set project_name = coalesce($2, project_name),
    comment = coalesce($3, comment),
    updated_at=now()
where id = $1
returning id, project_name, comment;
        "#,
        project_id,
        info.project_name,
        info.comment,
    )
    .fetch_one(&state.db_pool)
    .await?;

    Ok(Json(project))
}

/// 删除指定的项目
#[utoipa::path(delete, path = "/projects/{id}", tag = "projects")]
#[axum::debug_handler]
pub async fn delete_project(
    State(state): State<Arc<AppState>>,
    Path(project_id): Path<i32>,
) -> Result<Json<ProjectInfo>, AppError> {
    debug!("delete project {:#?}", project_id);

    let project = sqlx::query_as!(
        ProjectInfo,
        r#"
delete
from hm.projects
where id = $1
returning id, project_name, comment;
    "#,
        project_id
    )
    .fetch_one(&state.db_pool)
    .await?;

    Ok(Json(project))
}
