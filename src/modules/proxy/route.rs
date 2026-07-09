use crate::modules::proxy::controller;
use crate::shared::state::AppState;
use axum::Router;
use std::sync::Arc;

pub fn routes(router: Router<Arc<AppState>>) -> Router<Arc<AppState>> {
    router
        .route(
            "/api/proxy/croxy",
            axum::routing::get(controller::fetch_with_proxy_only),
        )
        .route(
            "/api/proxy/image-cache",
            axum::routing::post(controller::image_cache),
        )
}
