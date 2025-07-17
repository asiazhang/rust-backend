//! 项目数据库模型
//!
//! 定义项目相关的数据库模型结构体

/// 项目信息结构体
#[derive(Debug, Clone)]
pub struct ProjectInfo {
    pub id: i32,
    pub project_name: String,
    pub comment: String,
}

/// 项目搜索结果
#[derive(Debug, Clone)]
pub struct ProjectSearchResult {
    pub projects: Vec<ProjectInfo>,
    pub total: u32,
}

/// 项目创建参数
#[derive(Debug, Clone)]
pub struct ProjectCreate {
    pub project_name: String,
    pub comment: String,
}

/// 项目更新参数
#[derive(Debug, Clone)]
pub struct ProjectUpdate {
    pub project_name: Option<String>,
    pub comment: Option<String>,
}
