use axum::{response::Json, extract::Query};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use tracing::instrument;

#[derive(Debug, Deserialize)]
pub struct ListProjectsQuery {
    pub page: Option<u32>,
    pub limit: Option<u32>,
}

#[derive(Debug, Serialize)]
pub struct Project {
    pub id: u32,
    pub name: String,
    pub description: String,
    pub status: String,
}

#[instrument]
pub async fn list_projects(Query(query): Query<ListProjectsQuery>) -> Json<Value> {
    let page = query.page.unwrap_or(1);
    let limit = query.limit.unwrap_or(10);
    
    // 模拟项目数据
    let projects = vec![
        Project {
            id: 1,
            name: "项目A".to_string(),
            description: "这是项目A的描述".to_string(),
            status: "active".to_string(),
        },
        Project {
            id: 2,
            name: "项目B".to_string(),
            description: "这是项目B的描述".to_string(),
            status: "inactive".to_string(),
        },
    ];
    
    Json(json!({
        "data": projects,
        "pagination": {
            "page": page,
            "limit": limit,
            "total": projects.len()
        }
    }))
}
