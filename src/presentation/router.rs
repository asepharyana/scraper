//! Axum router assembly.

use std::sync::Arc;

use axum::Router;
use tower_http::compression::{CompressionLayer, CompressionLevel};
use tower_http::cors::CorsLayer;
use utoipa::OpenApi;
use utoipa_swagger_ui::SwaggerUi;

use crate::observability::openapi::ApiDoc;
use crate::observability::openapi_modules::ModuleApiDoc;
use crate::presentation::state::AppState;

/// Build the main application router with all routes, middleware, and Swagger UI.
pub fn build_router(app_state: Arc<AppState>) -> anyhow::Result<Router> {
    let mut openapi = ApiDoc::openapi();
    openapi.merge(ModuleApiDoc::openapi());

    let app = Router::new()
        // Anime routes
        .route(
            "/api/anime",
            axum::routing::get(crate::presentation::handler::anime::anime_index),
        )
        .route(
            "/api/anime/genre_list",
            axum::routing::get(crate::presentation::handler::anime::genres),
        )
        .route(
            "/api/anime/detail/{slug}",
            axum::routing::get(crate::presentation::handler::anime::detail_slug),
        )
        .route(
            "/api/anime/complete_anime/{slug}",
            axum::routing::get(crate::presentation::handler::anime::complete_anime_slug),
        )
        .route(
            "/api/anime/full/{slug}",
            axum::routing::get(crate::presentation::handler::anime::full_slug),
        )
        .route(
            "/api/anime/ongoing_anime/{slug}",
            axum::routing::get(crate::presentation::handler::anime::ongoing_anime_slug),
        )
        .route(
            "/api/anime/latest/{slug}",
            axum::routing::get(crate::presentation::handler::anime::latest_slug),
        )
        .route(
            "/api/anime/search/{slug}",
            axum::routing::get(crate::presentation::handler::anime::search_slug_index),
        )
        .route(
            "/api/anime/search/{slug}/{page}",
            axum::routing::get(crate::presentation::handler::anime::search_slug_page),
        )
        .route(
            "/api/anime/genre/{slug}",
            axum::routing::get(crate::presentation::handler::anime::genre_slug_index),
        )
        .route(
            "/api/anime/genre/{slug}/{page}",
            axum::routing::get(crate::presentation::handler::anime::genre_slug_page),
        )
        // Anime2 routes
        .route(
            "/api/anime2",
            axum::routing::get(crate::presentation::handler::anime2::index),
        )
        .route(
            "/api/anime2/complete_anime/{slug}",
            axum::routing::get(crate::presentation::handler::anime2::complete_anime_slug),
        )
        .route(
            "/api/anime2/detail/{slug}",
            axum::routing::get(crate::presentation::handler::anime2::detail_slug),
        )
        .route(
            "/api/anime2/filter",
            axum::routing::get(crate::presentation::handler::anime2::filter),
        )
        .route(
            "/api/anime2/genre_list",
            axum::routing::get(crate::presentation::handler::anime2::genre_list),
        )
        .route(
            "/api/anime2/genre/{slug}",
            axum::routing::get(crate::presentation::handler::anime2::genre_slug_index),
        )
        .route(
            "/api/anime2/genre/{slug}/{page}",
            axum::routing::get(crate::presentation::handler::anime2::genre_slug_page),
        )
        .route(
            "/api/anime2/latest/{slug}",
            axum::routing::get(crate::presentation::handler::anime2::latest_slug),
        )
        .route(
            "/api/anime2/ongoing_anime/{slug}",
            axum::routing::get(crate::presentation::handler::anime2::ongoing_anime_slug),
        )
        .route(
            "/api/anime2/search/{slug}",
            axum::routing::get(crate::presentation::handler::anime2::search_slug_index),
        )
        .route(
            "/api/anime2/search/{slug}/{page}",
            axum::routing::get(crate::presentation::handler::anime2::search_slug_page),
        )
        // Komik routes
        .route(
            "/api/komik/genre_list",
            axum::routing::get(crate::presentation::handler::komik::genre_list),
        )
        .route(
            "/api/komik/chapter/{slug}",
            axum::routing::get(crate::presentation::handler::komik::chapter_slug),
        )
        .route(
            "/api/komik/detail/{slug}",
            axum::routing::get(crate::presentation::handler::komik::detail_slug),
        )
        .route(
            "/api/komik/genre/{slug}",
            axum::routing::get(crate::presentation::handler::komik::genre_slug),
        )
        .route(
            "/api/komik/genre/{slug}/{page}",
            axum::routing::get(crate::presentation::handler::komik::genre_slug_page),
        )
        .route(
            "/api/komik/manga/{slug}",
            axum::routing::get(crate::presentation::handler::komik::manga_slug),
        )
        .route(
            "/api/komik/manhua/{slug}",
            axum::routing::get(crate::presentation::handler::komik::manhua_slug),
        )
        .route(
            "/api/komik/manhwa/{slug}",
            axum::routing::get(crate::presentation::handler::komik::manhwa_slug),
        )
        .route(
            "/api/komik/popular/{slug}",
            axum::routing::get(crate::presentation::handler::komik::popular_slug),
        )
        .route(
            "/api/komik/search/{slug}",
            axum::routing::get(crate::presentation::handler::komik::search_slug),
        )
        .route(
            "/api/komik/search/{slug}/{page}",
            axum::routing::get(crate::presentation::handler::komik::search_slug_page),
        )
        // Proxy routes
        .route(
            "/api/proxy/croxy",
            axum::routing::get(crate::presentation::handler::proxy::fetch_with_proxy_only),
        )
        .route(
            "/api/proxy/image-cache",
            axum::routing::post(crate::presentation::handler::proxy::image_cache),
        )
        .route(
            "/api/proxy/image-cache/audit",
            axum::routing::post(crate::presentation::handler::proxy::audit_image_cache),
        )
        // Health
        .route(
            "/health",
            axum::routing::get(crate::presentation::handler::health::health_check),
        )
        // Swagger UI
        .merge(SwaggerUi::new("/docs").url("/api-docs/openapi.json", openapi))
        .with_state(app_state)
        .layer(axum::middleware::from_fn(
            crate::observability::metrics::otel_metrics_middleware,
        ))
        .layer(CompressionLayer::new().quality(CompressionLevel::Fastest))
        .layer(CorsLayer::permissive());

    Ok(app)
}
