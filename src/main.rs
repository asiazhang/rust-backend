use crate::routes::routers;
use anyhow::Result;
use tracing::{debug, info};
use utoipa::{Modify, OpenApi};
use utoipa_axum::router::OpenApiRouter;
use utoipa_scalar::{Scalar, Servable};

mod routes;

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::DEBUG)
        .init();

    #[derive(OpenApi)]
    #[openapi(
        modifiers(&AutoImportModify),
        tags(
            (name = "rust-backend", description = "Rust backend sample")
        ),
    )]
    struct ApiDoc;
    struct AutoImportModify;

    impl Modify for AutoImportModify {
        fn modify(&self, openapi: &mut utoipa::openapi::OpenApi) {
            // 这里可以添加自定义的 OpenAPI 配置
        }
    }

    let (router, api) = OpenApiRouter::with_openapi(ApiDoc::openapi())
        .nest("/api/v1", routers())
        .split_for_parts();

    debug!("openapi: {}", api.to_pretty_json()?);

    let router = router.merge(Scalar::with_url("/docs", api));

    let bind_addr = "0.0.0.0:8080";
    info!("Starting server on {}", bind_addr);

    let listener = tokio::net::TcpListener::bind(bind_addr).await?;

    axum::serve(listener, router.into_make_service()).await?;

    Ok(())
}
