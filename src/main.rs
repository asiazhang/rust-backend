//! 使用Rust来开发Web后端
//!
//! 包含以下功能：
//!
//! - 对外提供的`json-api`
//! - 可视化的文档
//! - 异步消息处理器(`redis`)
//! - `cron`任务处理器(定时任务)
//!
//! 所有代码都放在一个程序中，方便部署和维护(适用于小型系统)

use crate::crons::start_cron_tasks;
use crate::models::app::AppState;
use crate::models::config::AppConfig;
use crate::routes::routers;
use crate::tasks::start_job_consumers;
use axum::Router;
use color_eyre::eyre::Context;
use color_eyre::Result;
use std::sync::Arc;
use tokio::sync::watch::Sender;
use tokio::{signal, try_join};
use tracing::info;
use utoipa::OpenApi;
use utoipa_axum::router::OpenApiRouter;
use utoipa_scalar::{Scalar, Servable};

mod crons;
mod models;
mod routes;
mod tasks;

/// 入口函数
///
/// - 使用tokio作为异步运行时，因此需要增加 `#[tokio::main]`
#[tokio::main]
async fn main() -> Result<()> {
    color_eyre::install()?;
    // 使用tracing作为日志记录器
    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::DEBUG)
        .init();

    let conf = AppConfig::load()?;

    // 创建postgres数据库连接池
    // 使用默认配置，如果有调整需要可参考sqlx文档
    // 注意：pool已经是一个智能指针了，所以可以使用.clone()安全跨线程使用
    let pool = sqlx::PgPool::connect(&conf.postgresql_addr)
        .await
        .context("Connect to postgresql database")?;

    // 使用官方推荐的[共享状态方式](https://docs.rs/axum/latest/axum/#sharing-state-with-handlers)来在
    // 不同的web处理器之间同步，主要是需要共享数据库连接池
    let shared_state = Arc::new(AppState { db_pool: pool });

    let router = create_app_router(shared_state);

    let bind_addr = "0.0.0.0:8080";
    info!("Starting server on {}", bind_addr);

    let (tx, rx) = tokio::sync::watch::channel(false);

    let listener = tokio::net::TcpListener::bind(bind_addr).await?;
    let server_handle = tokio::spawn(async move {
        axum::serve(listener, router.into_make_service())
            .with_graceful_shutdown(shutdown_signal(tx))
            .await
    });
    let job_handle = tokio::spawn(start_job_consumers(conf.clone(), rx.clone()));
    let cron_handle = tokio::spawn(start_cron_tasks(rx.clone()));
    
    // TODO: 这里 job_handle 错误并没有提前返回
    _ = try_join!(server_handle, job_handle, cron_handle)?;

    Ok(())
}

async fn shutdown_signal(tx: Sender<bool>) {
    let ctrl_c = async {
        signal::ctrl_c()
            .await
            .expect("failed to install Ctrl+C handler");
    };

    #[cfg(unix)]
    let terminate = async {
        signal::unix::signal(signal::unix::SignalKind::terminate())
            .expect("failed to install signal handler")
            .recv()
            .await;
    };

    #[cfg(not(unix))]
    let terminate = std::future::pending::<()>();

    tokio::select! {
        _ = ctrl_c => {info!("Received Ctrl+C, initiating shutdown...");},
        _ = terminate => {info!("Received SIGTERM, initiating shutdown...");},
    }

    // 发送关闭信号
    tx.send(true).expect("Failed to send signal");
}

/// 创建当前App的路由
///
/// 完成以下功能：
/// - 生成OpenAPI文档
/// - 生成App路由
/// - 使用Scalar作为最终在线文档格式
///
/// 由于使用了 `utoipa` 库来自动化生成`openapi`文档，因此我们没有使用原生的 [`Router`]，而是使用了
/// [`OpenApiRouter`] 。
fn create_app_router(shared_state: Arc<AppState>) -> Router {
    // 当前项目的OpenAPI声明
    #[derive(OpenApi)]
    #[openapi(
        tags(
            (name = "rust-backend", description = r#"
Rust后端例子，覆盖场景：

- API后端
- 异步处理后端(Redis)
- OpenAPI文档
            "#)
        ),
    )]
    struct ApiDoc;

    // 使用`utoipa_axum`提供的OpenApiRouter来创建路由。
    // 同时传递共享状态数据到路由中供使用。
    // 最终拿到的变量：
    // - router: Axum的Router，实际的路由对象
    // - api: utoipa的OpenApi，生成的OpenAPI对象
    let (router, api) = OpenApiRouter::with_openapi(ApiDoc::openapi())
        .nest("/api/v1", routers(shared_state))
        .split_for_parts();

    // 合并文档路由，用户可通过 /docs 访问文档网页地址
    router.merge(Scalar::with_url("/docs", api))
}
