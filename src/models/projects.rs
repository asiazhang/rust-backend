use crate::models::common::PageQuery;
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use validator::Validate;

/// 搜索项目列表信息
///
/// - `project_name`为可选参数
#[derive(Deserialize, Debug, ToSchema, Validate)]
pub struct ProjectSearch {
    #[schema(example = "foo")]
    #[validate(length(min = 1, max = 100))]
    /// 查询的项目名称（模糊搜索）
    pub project_name: Option<String>,

    /// 查询分页信息
    #[validate(nested)]
    pub page_query: PageQuery,
}

#[derive(Deserialize, Debug, ToSchema)]
pub struct ProjectCreate {
    /// 新建项目名称
    #[schema(example = "foo")]
    pub project_name: String,

    /// 项目说明
    #[schema(example = "comment")]
    pub comment: String,
}

#[derive(Deserialize, Debug, ToSchema, Serialize)]
pub struct ProjectInfo {
    #[schema(example = 15)]
    /// 项目ID
    pub id: i32,

    #[schema(example = "bar")]
    /// 项目名称
    pub project_name: String,

    /// 项目说明
    #[schema(example = "foo_bar")]
    pub comment: String,
}

/// 更新项目的信息
#[derive(Deserialize, Debug, ToSchema, Serialize)]
pub struct ProjectUpdate {
    #[schema(example = "bar")]
    pub project_name: Option<String>,

    #[schema(example = "foo")]
    pub comment: Option<String>,
}
