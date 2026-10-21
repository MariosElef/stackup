use axum::{Json, Router, routing::get};
use serde::Serialize;
use tracing_subscriber::EnvFilter;
use utoipa::{OpenApi, ToSchema};

#[derive(Serialize, ToSchema)]
struct Health {
    status: String,
}

/// The API's OpenAPI document, built from the `#[utoipa::path]` annotations. List each new
/// handler under `paths(...)` and each request or response type under `components(schemas(...))`.
#[derive(OpenApi)]
#[openapi(paths(health), components(schemas(Health)))]
struct ApiDoc;

#[utoipa::path(
    get,
    path = "/health",
    responses((status = 200, description = "The service is up", body = Health))
)]
async fn health() -> Json<Health> {
    Json(Health {
        status: "ok".into(),
    })
}

async fn openapi() -> Json<utoipa::openapi::OpenApi> {
    Json(ApiDoc::openapi())
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt()
        .json()
        .with_env_filter(EnvFilter::try_from_default_env().unwrap_or_else(|_| "info".into()))
        .init();

    let addr = std::env::var("BIND_ADDR").unwrap_or_else(|_| "0.0.0.0:8000".into());
    let listener = tokio::net::TcpListener::bind(&addr).await?;
    tracing::info!(%addr, "listening");

    let app = Router::new()
        .route("/health", get(health))
        .route("/openapi.json", get(openapi));
    axum::serve(listener, app)
        .with_graceful_shutdown(async {
            let _ = tokio::signal::ctrl_c().await;
        })
        .await?;
    Ok(())
}
