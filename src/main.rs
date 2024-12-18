use crate::routes::routers;
use anyhow::Result;
use tracing::info;
use utoipa::OpenApi;
use utoipa_axum::router::OpenApiRouter;
use utoipa_scalar::{Scalar, Servable};

mod routes;

mod models;

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::DEBUG)
        .init();

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
        .nest("/api/v1", routers())
        .split_for_parts();

    let router = router.merge(Scalar::with_url("/docs", api));

    let bind_addr = "0.0.0.0:8080";
    info!("Starting server on {}", bind_addr);

    let listener = tokio::net::TcpListener::bind(bind_addr).await?;

    axum::serve(listener, router.into_make_service()).await?;

    Ok(())
}
