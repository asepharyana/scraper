use std::sync::Arc;

use axum::{routing::get, Router};

use crate::modules::komik::controller;
use crate::shared::state::AppState;

pub fn routes(router: Router<Arc<AppState>>) -> Router<Arc<AppState>> {
    router
        .route("/api/komik/genre_list", get(controller::genre_list))
        .route("/api/komik/chapter/{slug}", get(controller::chapter_slug))
        .route("/api/komik/detail/{slug}", get(controller::detail_slug))
        .route("/api/komik/genre/{slug}", get(controller::genre_slug))
        .route(
            "/api/komik/genre/{slug}/{page}",
            get(controller::genre_slug_page),
        )
        .route("/api/komik/manga/{slug}", get(controller::manga_slug))
        .route("/api/komik/manhua/{slug}", get(controller::manhua_slug))
        .route("/api/komik/manhwa/{slug}", get(controller::manhwa_slug))
        .route("/api/komik/popular/{slug}", get(controller::popular_slug))
        .route("/api/komik/search/{slug}", get(controller::search_slug))
        .route(
            "/api/komik/search/{slug}/{page}",
            get(controller::search_slug_page),
        )
}
