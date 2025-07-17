//! 项目仓库 trait 定义
//!
//! 定义项目数据库操作的抽象接口

use crate::models::project::{ProjectCreate, ProjectInfo, ProjectSearchResult, ProjectUpdate};
use crate::DatabaseResult;

/// 项目仓库trait定义
///
/// 定义了项目相关的数据库操作接口，支持：
/// - 项目搜索（分页）
/// - 项目创建
/// - 项目查询
/// - 项目更新
/// - 项目删除
#[async_trait::async_trait]
pub trait ProjectRepositoryTrait: Send + Sync + 'static {
    /// 根据查询参数搜索项目
    ///
    /// # 参数
    /// - `project_name`: 项目名称（模糊搜索）
    /// - `page_size`: 页面大小
    /// - `offset`: 偏移量
    ///
    /// # 返回值
    /// 返回包含项目列表和总数的结果 [`ProjectSearchResult`]
    async fn find_projects(&self, project_name: Option<String>, page_size: i64, offset: i64) -> DatabaseResult<ProjectSearchResult>;

    /// 创建新项目
    ///
    /// # 参数
    /// - `project`: 项目创建信息
    ///
    /// # 返回值
    /// 返回创建的项目信息
    async fn create_project(&self, project: ProjectCreate) -> DatabaseResult<ProjectInfo>;

    /// 根据 ID 获取项目信息
    ///
    /// # 参数
    /// - `id`: 项目 ID
    ///
    /// # 返回值
    /// 返回项目信息
    async fn get_project_by_id(&self, id: i32) -> DatabaseResult<ProjectInfo>;

    /// 更新项目信息
    ///
    /// # 参数
    /// - `id`: 项目 ID
    /// - `update`: 更新信息
    ///
    /// # 返回值
    /// 返回更新后的项目信息
    async fn update_project(&self, id: i32, update: ProjectUpdate) -> DatabaseResult<ProjectInfo>;

    /// 删除项目
    ///
    /// # 参数
    /// - `id`: 项目 ID
    ///
    /// # 返回值
    /// 返回被删除的项目信息
    async fn delete_project(&self, id: i32) -> DatabaseResult<ProjectInfo>;
}
