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

use crate::routes::routers;
use anyhow::{Context, Result};
use std::sync::Arc;

use crate::models::app::AppState;
use tracing::info;
use utoipa::OpenApi;
use utoipa_axum::router::OpenApiRouter;
use utoipa_scalar::{Scalar, Servable};

mod routes;

mod models;

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
    let pool = sqlx::PgPool::connect(&db_url)
        .await
        .context("Connect to postgresql database")?;

    let shared_state = Arc::new(AppState {
        db_pool: pool,
    });

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

    let (router, api) = OpenApiRouter::with_openapi(ApiDoc::openapi())
        .nest("/api/v1", routers(shared_state))
        .split_for_parts();

    let router = router
        .merge(Scalar::with_url("/docs", api));

    let bind_addr = "0.0.0.0:8080";
    info!("Starting server on {}", bind_addr);

    let listener = tokio::net::TcpListener::bind(bind_addr).await?;

    axum::serve(listener, router.into_make_service()).await?;

    Ok(())
}
