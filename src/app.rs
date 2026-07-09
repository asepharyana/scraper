use std::sync::Arc;

use axum::Router;
use sea_orm::DatabaseConnection;
use tower_http::compression::{CompressionLayer, CompressionLevel};
use tower_http::cors::CorsLayer;
use utoipa::OpenApi;
use utoipa_swagger_ui::SwaggerUi;

use crate::shared::observability::openapi::ApiDoc;
use crate::shared::state::AppState;

pub async fn build_router(
    app_state: Arc<AppState>,
    db: Arc<DatabaseConnection>,
) -> anyhow::Result<Router> {
    init_scheduler(db).await?;

    let mut openapi = ApiDoc::openapi();
    openapi.merge(crate::shared::observability::openapi_modules::ModuleApiDoc::openapi());

    let app = crate::modules::routes(Router::new())
        .merge(SwaggerUi::new("/docs").url("/api-docs/openapi.json", openapi))
        .with_state(app_state)
        .layer(axum::middleware::from_fn(
            crate::shared::observability::metrics::otel_metrics_middleware,
        ))
        .layer(CompressionLayer::new().quality(CompressionLevel::Fastest))
        .layer(CorsLayer::permissive());

    Ok(app)
}

async fn init_scheduler(db: Arc<DatabaseConnection>) -> anyhow::Result<()> {
    let scheduler = crate::shared::scheduler::Scheduler::new()
        .await
        .map_err(|e| anyhow::anyhow!("Failed to create scheduler: {}", e))?;

    let cache_cleanup = crate::shared::scheduler::CleanupOldCache::new(db);
    scheduler
        .add(cache_cleanup)
        .await
        .map_err(|e| anyhow::anyhow!("Failed to add cache cleanup: {}", e))?;

    scheduler
        .start()
        .await
        .map_err(|e| anyhow::anyhow!("Failed to start scheduler: {}", e))?;
    tracing::info!("✓ Scheduler started");
    Ok(())
}
