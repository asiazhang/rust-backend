//! 项目服务
//!
//! 提供项目相关的业务逻辑操作

use crate::services::traits::ProjectServiceTrait;
use database::{DatabaseResult, ProjectCreate, ProjectInfo, ProjectRepositoryTrait, ProjectSearchResult, ProjectUpdate};

#[derive(Debug, Clone)]
pub struct ProjectService<PR>
where
    PR: ProjectRepositoryTrait,
{
    project_repository: PR,
}

impl<PR> ProjectService<PR>
where
    PR: ProjectRepositoryTrait,
{
    pub fn new(project_repository: PR) -> Self {
        Self { project_repository }
    }
}

#[async_trait::async_trait]
impl<PR> ProjectServiceTrait for ProjectService<PR>
where
    PR: ProjectRepositoryTrait,
{
    async fn find_projects(&self, name: Option<String>, page_size: i64, offset: i64) -> DatabaseResult<ProjectSearchResult> {
        self.project_repository.find_projects(name, page_size, offset).await
    }

    async fn create_project(&self, project: ProjectCreate) -> DatabaseResult<ProjectInfo> {
        self.project_repository.create_project(project).await
    }

    async fn get_project_by_id(&self, id: i32) -> DatabaseResult<ProjectInfo> {
        self.project_repository.get_project_by_id(id).await
    }

    async fn update_project(&self, id: i32, update: ProjectUpdate) -> DatabaseResult<ProjectInfo> {
        self.project_repository.update_project(id, update).await
    }

    async fn delete_project(&self, id: i32) -> DatabaseResult<ProjectInfo> {
        self.project_repository.delete_project(id).await
    }
}
