use axum::{response::Json, extract::Query};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use tracing::instrument;

#[derive(Debug, Deserialize)]
pub struct ListUsersQuery {
    pub page: Option<u32>,
    pub limit: Option<u32>,
}

#[derive(Debug, Serialize)]
pub struct User {
    pub id: u32,
    pub name: String,
    pub email: String,
}

#[instrument]
pub async fn list_users(Query(query): Query<ListUsersQuery>) -> Json<Value> {
    let page = query.page.unwrap_or(1);
    let limit = query.limit.unwrap_or(10);
    
    // 模拟用户数据
    let users = vec![
        User {
            id: 1,
            name: "张三".to_string(),
            email: "zhangsan@example.com".to_string(),
        },
        User {
            id: 2,
            name: "李四".to_string(),
            email: "lisi@example.com".to_string(),
        },
    ];
    
    Json(json!({
        "data": users,
        "pagination": {
            "page": page,
            "limit": limit,
            "total": users.len()
        }
    }))
}
