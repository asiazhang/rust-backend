//! Web服务模块
//!
//! 提供 HTTP API 接口和文档服务

use color_eyre::Result;
use database::{ProjectRepository, ProjectRepositoryTrait};
use sqlx::{Pool, Postgres};
use std::sync::Arc;
use tokio::sync::watch::Receiver;
use tracing::info;

pub mod models;
pub mod routes;

/// 应用共享状态
pub struct AppState<PR: ProjectRepositoryTrait> {
    pub project_repository: Arc<PR>,
}

/// 具体的 AppState 类型别名
pub type ConcreteAppState = AppState<ProjectRepository>;

/// 启动 Web 服务
pub async fn start_web_service(pool: Pool<Postgres>, mut shutdown_rx: Receiver<bool>) -> Result<()> {
    let shared_state = AppState {
        project_repository: Arc::new(database::ProjectRepository::new(pool.clone())),
    };

    let router = routes::create_app_router(shared_state);

    let bind_addr = "0.0.0.0:8080";
    info!("🚀 启动 Web Service 在 {}", bind_addr);

    let listener = tokio::net::TcpListener::bind(bind_addr).await?;

    axum::serve(listener, router.into_make_service())
        .with_graceful_shutdown(async move {
            shutdown_rx.changed().await.expect("Failed to receive shutdown signal");
            info!("🛑 Web Service 正在关闭...");
        })
        .await?;

    Ok(())
}
