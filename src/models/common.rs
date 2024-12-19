use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

#[derive(Deserialize, Debug, ToSchema)]
pub struct PageQuery {
    #[schema(example = 1)]
    /// 分页查询的开始页数
    pub page_index: u32,

    #[schema(example = 20)]
    /// 分页查询的每页大小
    pub page_size: u32,
}

#[derive(Deserialize, Debug, ToSchema, Serialize)]
pub struct Reply<T> {
    pub data: T,
}

#[derive(Deserialize, Debug, ToSchema, Serialize)]
pub struct ReplyList<T> {
    pub data: Vec<T>,
    #[schema(example = 146)]
    /// 分页查询总数
    pub total: u32,

    #[schema(example = 1)]
    /// 分页查询的开始页数
    pub page_size: u32,

    #[schema(example = 20)]
    /// 分页查询的每页大小
    pub page_index: u32,
}
