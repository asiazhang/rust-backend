use crate::models::common::PageQuery;
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

#[derive(Deserialize, Debug, ToSchema)]
pub struct ProjectSearch {
    #[schema(example="foo")]
    /// 查询的项目名称（模糊搜索）
    pub project_name: Option<String>,

    /// 查询分页信息
    pub page_query: PageQuery,
}

#[derive(Deserialize, Debug, ToSchema)]
pub struct ProjectCreate {
    #[schema(example="foo")]
    /// 新建项目名称
    pub project_name: String
}

#[derive(Deserialize, Debug, ToSchema, Serialize)]
pub struct ProjectInfo {
    #[schema(example=15)]
    /// 项目ID
    pub id: u32,

    #[schema(example="bar")]
    /// 项目名称
    pub project_name: String,
}
