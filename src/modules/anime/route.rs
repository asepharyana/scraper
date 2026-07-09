use crate::modules::anime::controller;
use crate::shared::state::AppState;
use axum::Router;
use std::sync::Arc;

pub fn routes(router: Router<Arc<AppState>>) -> Router<Arc<AppState>> {
    router
        .route("/api/anime", axum::routing::get(controller::anime_index))
        .route(
            "/api/anime/genre_list",
            axum::routing::get(controller::genres),
        )
        .route(
            "/api/anime/detail/{slug}",
            axum::routing::get(controller::detail_slug),
        )
        .route(
            "/api/anime/complete_anime/{slug}",
            axum::routing::get(controller::complete_anime_slug),
        )
        .route(
            "/api/anime/full/{slug}",
            axum::routing::get(controller::full_slug),
        )
        .route(
            "/api/anime/ongoing_anime/{slug}",
            axum::routing::get(controller::ongoing_anime_slug),
        )
        .route(
            "/api/anime/latest/{slug}",
            axum::routing::get(controller::latest_slug),
        )
        .route(
            "/api/anime/search/{slug}",
            axum::routing::get(controller::search_slug_index),
        )
        .route(
            "/api/anime/search/{slug}/{page}",
            axum::routing::get(controller::search_slug_page),
        )
        .route(
            "/api/anime/genre/{slug}",
            axum::routing::get(controller::genre_slug_index),
        )
        .route(
            "/api/anime/genre/{slug}/{page}",
            axum::routing::get(controller::genre_slug_page),
        )
}
