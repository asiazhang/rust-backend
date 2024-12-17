use crate::routes::projects::{
    create_project, delete_project, find_projects, get_project, update_project,
};
use crate::routes::routers;
use crate::routes::users::{create_user, delete_user, find_users, get_user, update_user};
use anyhow::Result;
use tracing::info;
use utoipa::OpenApi;
use utoipa_axum::router::OpenApiRouter;
use utoipa_scalar::{Scalar, Servable};

mod routes;

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::INFO)
        .init();

    #[derive(OpenApi)]
    #[openapi(
        modifiers(),
        tags(
            (name = "rust-backend", description = "Rust backend sample")
        )
    )]
    struct ApiDoc;
    let (router, api) = OpenApiRouter::with_openapi(ApiDoc::openapi())
        .nest("api/v1", routers())
        .split_for_parts();

    let router = router.merge(Scalar::with_url("/docs", api));

    let bind_addr = "0.0.0.0:8080";
    info!("Starting server on {}", bind_addr);

    let listener = tokio::net::TcpListener::bind(bind_addr).await?;

    axum::serve(listener, router.into_make_service()).await?;

    Ok(())
}
