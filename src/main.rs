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

use crate::models::app::AppState;
use crate::routes::routers;
use anyhow::{Context, Result};
use axum::Router;
use std::sync::Arc;
use tracing::info;
use utoipa::OpenApi;
use utoipa_axum::router::OpenApiRouter;
use utoipa_scalar::{Scalar, Servable};

mod routes;
mod models;
mod tasks;
mod crons; 

/// 入口函数
///
/// - 使用tokio作为异步运行时，因此需要增加 `#[tokio::main]`
#[tokio::main]
async fn main() -> Result<()> {
    // 使用tracing作为日志记录器
    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::DEBUG)
        .init();

    // 加载.env文件中的数据注入到环境变量中，方便本地测试
    // 线上环境部署时会直接使用环境变量，不需要.env文件
    dotenvy::dotenv()?;

    // 读取数据库地址信息（仅支持postgresql）
    let db_url =
        std::env::var("DATABASE_URL").context("Can not load DATABASE_URL in environment")?;

    // 创建postgres数据库连接池
    // 使用默认配置，如果有调整需要可参考sqlx文档
    // 注意：pool已经是一个智能指针了，所以可以使用.clone()安全跨线程使用
    let pool = sqlx::PgPool::connect(&db_url)
        .await
        .context("Connect to postgresql database")?;

    // 使用官方推荐的[共享状态方式](https://docs.rs/axum/latest/axum/#sharing-state-with-handlers)来在
    // 不同的web处理器之间同步，主要是需要共享数据库连接池
    let shared_state = Arc::new(AppState { db_pool: pool });

    let router = create_app_router(shared_state);

    let bind_addr = "0.0.0.0:8080";
    info!("Starting server on {}", bind_addr);

    let listener = tokio::net::TcpListener::bind(bind_addr).await?;
    axum::serve(listener, router.into_make_service()).await?;

    Ok(())
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
