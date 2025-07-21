//! Web服务模块
//!
//! 提供 HTTP API 接口和文档服务

use color_eyre::Result;
use sqlx::{Pool, Postgres};
use std::sync::Arc;
use tokio::sync::watch::Receiver;
use tracing::info;

pub mod models;
pub mod routes;
pub mod services;

use services::{ProjectService, ProjectServiceTrait};

/// 应用共享状态
#[derive(Debug, Clone)]
pub struct AppState<PS: ProjectServiceTrait> {
    pub project_service: Arc<PS>,
}

/// 启动 Web 服务
pub async fn start_web_service(pool: Pool<Postgres>, mut shutdown_rx: Receiver<bool>) -> Result<()> {
    let project_repository = database::ProjectRepository::new(pool.clone());
    let project_service = ProjectService::new(project_repository);

    let shared_state = AppState {
        project_service: Arc::new(project_service),
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
