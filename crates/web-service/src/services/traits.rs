//! 服务层 trait 定义
//!
//! 定义服务层的抽象接口，遵循六边形架构的端口适配器模式

use database::{DatabaseResult, ProjectInfo, ProjectCreate, ProjectUpdate, ProjectSearchResult};

/// 项目服务 trait 定义
///
/// 定义了项目相关的业务逻辑接口，作为应用层的端口(Port)
/// 
/// 该 trait 作为业务逻辑的抽象接口，具体实现由 [`ProjectService`] 提供
#[async_trait::async_trait]
pub trait ProjectServiceTrait: Send + Sync + Clone + 'static {
    /// 根据查询参数搜索项目
    ///
    /// # 参数
    /// - `name`: 项目名称（模糊搜索）
    /// - `page_size`: 页面大小
    /// - `offset`: 偏移量
    ///
    /// # 返回值
    /// 返回包含项目列表和总数的结果
    async fn find_projects(&self, name: Option<String>, page_size: i64, offset: i64) -> DatabaseResult<ProjectSearchResult>;

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
