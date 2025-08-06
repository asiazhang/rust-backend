use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use validator::Validate;

/// 分页查询信息
#[derive(Deserialize, Debug, ToSchema, Validate)]
pub struct PageQuery {
    #[schema(example = 1)]
    #[validate(range(min = 1))]
    /// 分页查询的开始页数
    pub page_index: u32,

    #[schema(example = 20)]
    #[validate(range(min = 1, max = 100))]
    /// 分页查询的每页大小
    pub page_size: u32,
}

/// 封装符合json-api的单个返回对象
///
/// 具体参考：<https://jsonapi.org>
#[derive(Deserialize, Debug, ToSchema, Serialize)]
pub struct Reply<T> {
    pub data: T,
}

/// 封装符合json-api的列表对象
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
