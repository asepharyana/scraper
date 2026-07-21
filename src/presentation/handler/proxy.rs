//! Proxy and image cache API handlers.

use std::sync::Arc;

use axum::extract::{Json, Query, State};
use axum::response::Response;
use serde::Deserialize;
use tracing::info;
use utoipa::{IntoParams, ToSchema};

use crate::application::proxy::use_cases::{
    AuditImageCacheResult, ImageCacheResult, ProxyUseCases,
};
use crate::infrastructure::repository::image_cache_seaorm::SeaOrmImageCacheRepository;
use crate::infrastructure::repository::ProxyRepository;
use crate::presentation::error::AppError;
use crate::presentation::state::AppState;

// ============================================================================
// Request DTOs
// ============================================================================

/// Query parameters for proxy fetch (GET).
#[derive(Debug, Deserialize, IntoParams, ToSchema)]
pub struct ProxyParams {
    /// URL to fetch via proxy.
    pub url: String,
}

/// Request body for image cache (POST).
#[derive(Debug, Deserialize, ToSchema)]
pub struct ImageCacheRequest {
    /// Original image URL to cache.
    pub url: String,
    /// If true, returns original URL immediately and caches in background.
    #[serde(default)]
    pub lazy: bool,
}

/// Request body for auditing image cache (POST).
#[derive(Debug, Deserialize, ToSchema)]
pub struct AuditImageCacheRequest {
    /// Original image URL to audit.
    pub url: String,
}

// ============================================================================
// Helper
// ============================================================================

fn make_use_cases(state: &Arc<AppState>) -> ProxyUseCases {
    let repo = Arc::new(SeaOrmImageCacheRepository::new(
        state.db.clone(),
        state.redis_pool.clone(),
    ));
    ProxyUseCases::new(ProxyRepository::new(), repo)
}

// ============================================================================
// Handlers
// ============================================================================

/// GET /api/proxy/croxy — Fetch a URL through the proxy and return raw bytes.
#[utoipa::path(
    get,
    path = "/api/proxy/croxy",
    tag = "proxy",
    operation_id = "proxy_croxy",
    params(ProxyParams),
    responses(
        (status = 200, description = "Proxied response", body = Vec::<u8>, content_type = "application/octet-stream"),
        (status = 500, description = "Internal Server Error"),
    )
)]
pub async fn fetch_with_proxy_only(
    State(state): State<Arc<AppState>>,
    Query(params): Query<ProxyParams>,
) -> Result<Response, AppError> {
    info!("Handling proxy fetch for URL: {}", params.url);
    let response = make_use_cases(&state)
        .fetch_with_proxy_only(params.url)
        .await?;
    Ok(response)
}

/// POST /api/proxy/image-cache — Cache an image URL to CDN.
#[utoipa::path(
    post,
    path = "/api/proxy/image-cache",
    tag = "proxy",
    operation_id = "proxy_image_cache",
    request_body = ImageCacheRequest,
    responses(
        (status = 200, description = "Image cache result", body = ImageCacheResult),
        (status = 500, description = "Internal Server Error"),
    )
)]
pub async fn image_cache(
    State(state): State<Arc<AppState>>,
    Json(req): Json<ImageCacheRequest>,
) -> Result<Json<ImageCacheResult>, AppError> {
    info!(
        "Handling image cache for URL: {} (lazy: {})",
        req.url, req.lazy
    );
    let result = make_use_cases(&state)
        .image_cache(req.url, req.lazy)
        .await?;
    Ok(Json(result))
}

/// POST /api/proxy/image-cache/audit — Audit and repair a cached image.
#[utoipa::path(
    post,
    path = "/api/proxy/image-cache/audit",
    tag = "proxy",
    operation_id = "proxy_image_cache_audit",
    request_body = AuditImageCacheRequest,
    responses(
        (status = 200, description = "Audit result", body = AuditImageCacheResult),
        (status = 500, description = "Internal Server Error"),
    )
)]
pub async fn audit_image_cache(
    State(state): State<Arc<AppState>>,
    Json(req): Json<AuditImageCacheRequest>,
) -> Result<Json<AuditImageCacheResult>, AppError> {
    info!("Handling audit cache for URL: {}", req.url);
    let result = make_use_cases(&state).audit_image_cache(req.url).await?;
    Ok(Json(result))
}
