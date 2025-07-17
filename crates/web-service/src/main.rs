use axum::{
    response::Json,
    routing::get,
    Router,
};
use serde_json::{json, Value};
use tracing::{info, instrument};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // 初始化日志
    tracing_subscriber::fmt::init();
    
    info!("🚀 启动 Web Service...");
    
    // 构建应用路由
    let app = Router::new()
        .route("/", get(root))
        .route("/health", get(health_check));
    
    // 监听端口
    let listener = tokio::net::TcpListener::bind("0.0.0.0:3000")
        .await
        .expect("Failed to bind to address");
    
    info!("📡 Web Service 正在监听 http://0.0.0.0:3000");
    
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
