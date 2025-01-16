//! 路由入口
//!
//! 提供 [`routers`] 函数，导出当前App的所有路由。
//!
//! 用户可以在导出路由时传入共享数据 shared_state，这样所有路由函数都可以访问。

use crate::models::app::AppState;
use crate::routes::projects::__path_create_project;
use crate::routes::projects::__path_delete_project;
use crate::routes::projects::__path_find_projects;
use crate::routes::projects::__path_get_project;
use crate::routes::projects::__path_update_project;
use crate::routes::projects::{
    create_project, delete_project, find_projects, get_project, update_project,
};
use crate::routes::users::__path_create_user;
use crate::routes::users::__path_delete_user;
use crate::routes::users::__path_find_users;
use crate::routes::users::__path_get_user;
use crate::routes::users::__path_update_user;
use crate::routes::users::{create_user, delete_user, find_users, get_user, update_user};
use axum::Router;
use color_eyre::Result;
use sqlx::{Pool, Postgres};
use std::sync::Arc;
use tokio::signal;
use tokio::sync::watch::Sender;
use tracing::info;
use utoipa::OpenApi;
use utoipa_axum::router::OpenApiRouter;
use utoipa_axum::routes;
use utoipa_scalar::{Scalar, Servable};

pub mod projects;
pub mod users;

pub async fn start_axum_server(pool: Pool<Postgres>, shutdown_tx: Sender<bool>) -> Result<()> {
    // 使用官方推荐的[共享状态方式](https://docs.rs/axum/latest/axum/#sharing-state-with-handlers)来在
    // 不同的web处理器之间同步，主要是需要共享数据库连接池
    let shared_state = Arc::new(AppState { db_pool: pool });

    let router = create_app_router(shared_state);

    let bind_addr = "0.0.0.0:8080";
    info!("Starting server on {}", bind_addr);

    let listener = tokio::net::TcpListener::bind(bind_addr).await?;

    axum::serve(listener, router.into_make_service())
        .with_graceful_shutdown(shutdown_signal(shutdown_tx))
        .await?;

    Ok(())
}

/// 导出当前App的所有路由
///
/// ## 参数定义
/// - state: 共享数据，参考 [`AppState`] 定义。一般存放数据库连接池之类的全局共享数据。
///
/// ## **❗️注意事项：**
///
/// 由于 [`routes!`] 宏限制，在同一个宏里面不能同时定义多个相同类型的http接口。
/// 不能这样定义：
///
/// ```rust
/// routes!(get, get, post)
/// ```
///
/// 这样会导致Panic
///
/// 需要拆开定义
///
/// ```rust
/// routes!(get, post)
/// .routes!(get)
/// ```
///
fn routers(state: Arc<AppState>) -> OpenApiRouter {
    OpenApiRouter::new()
        .routes(routes!(find_projects))
        .routes(routes!(
            get_project,
            create_project,
            update_project,
            delete_project
        ))
        .routes(routes!(find_users))
        .routes(routes!(get_user, create_user, update_user, delete_user))
        .with_state(state)
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

async fn shutdown_signal(shutdown_tx: Sender<bool>) {
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
    shutdown_tx.send(true).expect("Failed to send signal");
}
