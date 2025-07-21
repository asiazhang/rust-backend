//! 项目仓库
//!
//! 负责项目相关的数据库操作

use crate::DatabaseResult;
use crate::models::project::{ProjectCreate, ProjectInfo, ProjectSearchResult, ProjectUpdate};
use crate::repositories::traits::ProjectRepositoryTrait;
use sqlx::PgPool;
use tracing::debug;

/// 项目仓库结构体
#[derive(Debug, Clone)]
pub struct ProjectRepository {
    pool: PgPool,
}

impl ProjectRepository {
    /// 创建新的项目仓库实例
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

#[async_trait::async_trait]
impl ProjectRepositoryTrait for ProjectRepository {
    /// 根据查询参数搜索项目
    ///
    /// 根据查询参数搜索符合要求的项目列表，支持分页。
    ///
    /// # 参数
    /// - `project_name`: 项目名称（模糊搜索）
    /// - `page_size`: 页面大小
    /// - `offset`: 偏移量
    ///
    /// # 返回值
    /// 返回包含项目列表和总数的结果 [`ProjectSearchResult`]
    ///
    /// # SQL 查询说明
    ///
    /// 使用 CTE（Common Table Expression）来优化查询性能：
    /// 1. 首先在 `filtered_projects` 中进行过滤和计数
    /// 2. 使用 `COUNT(*) OVER ()` 窗口函数获取总记录数
    /// 3. 使用 `COALESCE` 函数处理可选的搜索参数
    /// 4. 支持项目名称的模糊搜索（LIKE 操作）
    ///
    /// # 错误处理
    ///
    /// 如果数据库操作失败，会返回 [`DatabaseError`]
    async fn find_projects(&self, project_name: Option<String>, page_size: i64, offset: i64) -> DatabaseResult<ProjectSearchResult> {
        debug!(
            "🔍 搜索项目 - 名称: {:?}, 页面大小: {}, 偏移量: {}",
            project_name, page_size, offset
        );

        // 准备搜索参数
        // 这里name需要clone一次，因为后面会使用两次name，导致重复消费
        let name_param = project_name.clone().unwrap_or_default();
        let like_param = project_name.map(|n| format!("%{n}%")).unwrap_or_default();

        // 具体sqlx的好处：
        // 1. 编译时SQL验证 - 确保SQL语法正确
        // 2. 类型安全 - 自动推导参数和返回值类型
        // 3. 防止SQL注入 - 使用预处理语句
        // 4. 性能优化 - 查询计划缓存
        let rows = sqlx::query!(
            r#"
            WITH filtered_projects AS (
                SELECT id,
                       project_name,
                       comment,
                       COUNT(*) OVER () as total_count
                FROM hm.projects
                WHERE (COALESCE($1, '') = '' OR project_name LIKE $2)
                LIMIT $3 OFFSET $4
            )
            SELECT id,
                   project_name,
                   comment,
                   total_count
            FROM filtered_projects;
            "#,
            name_param,
            like_param,
            page_size,
            offset,
        )
        .fetch_all(&self.pool)
        .await?;

        // 获取总数
        let total = rows.first().and_then(|r| r.total_count).unwrap_or(0) as u32;

        // 转换为 ProjectInfo 结构体
        let projects: Vec<ProjectInfo> = rows
            .into_iter()
            .map(|r| ProjectInfo {
                id: r.id,
                project_name: r.project_name,
                comment: r.comment,
            })
            .collect();

        debug!("✅ 搜索完成 - 找到 {} 个项目，总计 {} 个", projects.len(), total);

        Ok(ProjectSearchResult { projects, total })
    }

    /// 创建新项目
    ///
    /// 根据用户输入参数创建项目信息
    ///
    /// # 参数
    /// - `project`: 项目创建信息
    ///
    /// # 返回值
    /// 返回创建的项目信息
    async fn create_project(&self, project: ProjectCreate) -> DatabaseResult<ProjectInfo> {
        debug!("📝 创建项目: {:#?}", project);

        let project_info = sqlx::query_as!(
            ProjectInfo,
            r#"
            INSERT INTO hm.projects (project_name, comment, created_at, updated_at)
            VALUES ($1, $2, now(), now())
            RETURNING id, project_name, comment;
            "#,
            project.project_name,
            project.comment
        )
        .fetch_one(&self.pool)
        .await?;

        debug!("✅ 项目创建成功: {:#?}", project_info);
        Ok(project_info)
    }

    /// 根据 ID 获取项目信息
    ///
    /// 查询指定项目信息
    ///
    /// # 参数
    /// - `id`: 项目 ID
    ///
    /// # 返回值
    /// 返回项目信息
    async fn get_project_by_id(&self, id: i32) -> DatabaseResult<ProjectInfo> {
        debug!("🔍 根据 ID 获取项目: {}", id);

        let project = sqlx::query_as!(
            ProjectInfo,
            r#"
            SELECT id, project_name, comment
            FROM hm.projects
            WHERE id = $1
            LIMIT 1
            "#,
            id
        )
        .fetch_one(&self.pool)
        .await?;

        debug!("✅ 项目获取成功: {:#?}", project);
        Ok(project)
    }

    /// 更新项目信息
    ///
    /// 根据用户指定的 `id` 和 修改信息 [`ProjectUpdate`] 来更新项目信息。
    ///
    /// ## SQL
    ///
    /// 由于更新数据中的字段大部分都是[`Option`]，因此我们使用了`postgresql`中的`coalesce`函数，如果用户输入的值
    /// 为None，那么会被转换为数据库的null，最终被转换为之前值。
    ///
    /// 两个好处：
    /// - 防止前端输入了空数据，导致数据被误清除
    /// - 不用`if`拼接的方式，代码可维护性更好
    ///
    /// # 参数
    /// - `id`: 项目 ID
    /// - `update`: 更新信息
    ///
    /// # 返回值
    /// 返回更新后的项目信息
    async fn update_project(&self, id: i32, update: ProjectUpdate) -> DatabaseResult<ProjectInfo> {
        debug!("🔄 更新项目 {} 信息: {:#?}", id, update);

        let project = sqlx::query_as!(
            ProjectInfo,
            r#"
            UPDATE hm.projects
            SET project_name = coalesce($2, project_name),
                comment = coalesce($3, comment),
                updated_at = now()
            WHERE id = $1
            RETURNING id, project_name, comment;
            "#,
            id,
            update.project_name,
            update.comment,
        )
        .fetch_one(&self.pool)
        .await?;

        debug!("✅ 项目更新成功: {:#?}", project);
        Ok(project)
    }

    /// 删除项目
    ///
    /// 删除指定的项目
    ///
    /// # 参数
    /// - `id`: 项目 ID
    ///
    /// # 返回值
    /// 返回被删除的项目信息
    async fn delete_project(&self, id: i32) -> DatabaseResult<ProjectInfo> {
        debug!("🗑️ 删除项目: {}", id);

        let project = sqlx::query_as!(
            ProjectInfo,
            r#"
            DELETE FROM hm.projects
            WHERE id = $1
            RETURNING id, project_name, comment;
            "#,
            id
        )
        .fetch_one(&self.pool)
        .await?;

        debug!("✅ 项目删除成功: {:#?}", project);
        Ok(project)
    }
}
