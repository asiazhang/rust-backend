pub mod handlers;
pub mod middleware;
pub mod routes;

use axum::{response::Json, routing::get, Router};
use serde_json::{json, Value};
use tracing::{info, instrument};

/// 创建Web服务应用
pub fn create_app() -> Router {
    Router::new()
        .route("/", get(root))
        .route("/health", get(health_check))
        .route("/api/v1/users", get(handlers::users::list_users))
        .route("/api/v1/projects", get(handlers::projects::list_projects))
}

/// 启动Web服务
pub async fn start_server(port: u16) -> anyhow::Result<()> {
    info!("🚀 启动 Web Service...");

    let app = create_app();

    let listener = tokio::net::TcpListener::bind(format!("0.0.0.0:{}", port))
        .await
        .expect("Failed to bind to address");

    info!("📡 Web Service 正在监听 http://0.0.0.0:{}", port);

    axum::serve(listener, app).await?;
    Ok(())
}

#[instrument]
async fn root() -> Json<Value> {
    Json(json!({
        "service": "web-service",
        "status": "running",
        "message": "🌐 Web Service 运行中"
    }))
}

#[instrument]
async fn health_check() -> Json<Value> {
    Json(json!({
        "status": "healthy",
        "timestamp": chrono::Utc::now().to_rfc3339(),
        "service": "web-service"
    }))
}
