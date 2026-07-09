use std::sync::Arc;

use axum::{routing::get, Router};

use crate::modules::anime2::controller;
use crate::shared::state::AppState;

pub fn routes(router: Router<Arc<AppState>>) -> Router<Arc<AppState>> {
    router
        .route("/api/anime2", get(controller::index))
        .route(
            "/api/anime2/complete_anime/{slug}",
            get(controller::complete_anime_slug),
        )
        .route("/api/anime2/detail/{slug}", get(controller::detail_slug))
        .route("/api/anime2/filter", get(controller::filter))
        .route("/api/anime2/genre_list", get(controller::genre_list))
        .route(
            "/api/anime2/genre/{slug}",
            get(controller::genre_slug_index),
        )
        .route(
            "/api/anime2/genre/{slug}/{page}",
            get(controller::genre_slug_page),
        )
        .route("/api/anime2/latest/{slug}", get(controller::latest_slug))
        .route(
            "/api/anime2/ongoing_anime/{slug}",
            get(controller::ongoing_anime_slug),
        )
        .route(
            "/api/anime2/search/{slug}",
            get(controller::search_slug_index),
        )
        .route(
            "/api/anime2/search/{slug}/{page}",
            get(controller::search_slug_page),
        )
}
