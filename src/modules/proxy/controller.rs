use crate::modules::proxy::repository::ProxyRepository;
use crate::modules::proxy::schema::{AuditImageCacheRequest, ImageCacheRequest, ProxyParams};
use crate::modules::proxy::service::ProxyService;
use crate::modules::proxy::types::{AuditImageCacheResponse, ImageCacheResponse};
use crate::shared::database::repositories::image_cache::SeaOrmImageCacheRepository;
use crate::shared::errors::AppError;
use crate::shared::state::AppState;
use axum::extract::{Json, Query, State};
use axum::response::Response;
use std::sync::Arc;

fn make_service(state: &Arc<AppState>) -> ProxyService {
    let repo = Arc::new(SeaOrmImageCacheRepository::new(
        state.db.clone(),
        state.redis_pool.clone(),
    ));
    ProxyService::new(ProxyRepository::new(), repo)
}

#[utoipa::path(
    get,
    path = "/api/proxy/croxy",
    tag = "proxy",
    operation_id = "proxy_croxy",
    params(ProxyParams),
    responses(
        (status = 200, description = "Handles GET requests for the /api/proxy/croxy endpoint.", body = serde_json::Value),
        (status = 500, description = "Internal Server Error", body = String)
    )
)]
pub async fn fetch_with_proxy_only(
    State(state): State<Arc<AppState>>,
    Query(params): Query<ProxyParams>,
) -> Result<Response, AppError> {
    make_service(&state).fetch_with_proxy_only(params).await
}

#[utoipa::path(
    post,
    path = "/api/proxy/image-cache",
    tag = "proxy",
    operation_id = "proxy_image_cache",
    request_body = ImageCacheRequest,
    responses(
        (status = 200, description = "Cache an image to CDN and return the cached URL", body = ImageCacheResponse),
        (status = 500, description = "Internal Server Error", body = String)
    )
)]
pub async fn image_cache(
    State(state): State<Arc<AppState>>,
    Json(req): Json<ImageCacheRequest>,
) -> Result<Json<ImageCacheResponse>, AppError> {
    make_service(&state).image_cache(state, req).await.map(Json)
}

#[utoipa::path(
    post,
    path = "/api/proxy/image-cache/audit",
    tag = "proxy",
    operation_id = "proxy_image_cache_audit",
    request_body = AuditImageCacheRequest,
    responses(
        (status = 200, description = "Audit an image cache entry", body = AuditImageCacheResponse),
        (status = 500, description = "Internal Server Error", body = String)
    )
)]
pub async fn audit_image_cache(
    State(state): State<Arc<AppState>>,
    Json(req): Json<AuditImageCacheRequest>,
) -> Result<Json<AuditImageCacheResponse>, AppError> {
    make_service(&state)
        .audit_image_cache(state, req)
        .await
        .map(Json)
}
