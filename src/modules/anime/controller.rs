use crate::modules::anime::repository::AnimeRepository;
use crate::modules::anime::service::AnimeService;
use crate::shared::errors::AppError;
use crate::shared::state::AppState;
use axum::extract::{Path, State};
use axum::Json;
use std::sync::Arc;
use tracing::info;

#[utoipa::path(
    get,
    path = "/api/anime",
    tag = "anime",
    operation_id = "anime_index",
    responses(
        (status = 200, description = "Handles GET requests for the /api/anime endpoint.", body = serde_json::Value),
        (status = 500, description = "Internal Server Error", body = String)
    )
)]
pub async fn anime_index(
    State(app_state): State<Arc<AppState>>,
) -> Result<Json<crate::modules::anime::types::AnimeData>, AppError> {
    info!("Handling request for anime index");
    let service = AnimeService::new(AnimeRepository::new());
    service.get_anime_index(app_state).await.map(Json)
}

#[utoipa::path(
    get,
    path = "/api/anime/genre_list",
    tag = "anime",
    operation_id = "anime_genre_list",
    responses(
        (status = 200, description = "Handles GET requests for the /api/anime/genre_list endpoint.", body = serde_json::Value),
        (status = 500, description = "Internal Server Error", body = String)
    )
)]
pub async fn genres(
    State(app_state): State<Arc<AppState>>,
) -> Result<Json<crate::modules::anime::types::GenresResponse>, AppError> {
    info!("Handling request for anime genres");
    let service = AnimeService::new(AnimeRepository::new());
    service.get_genres(app_state).await.map(Json)
}

#[utoipa::path(
    get,
    path = "/api/anime/detail/{slug}",
    tag = "anime",
    operation_id = "anime_detail_slug",
    responses(
        (status = 200, description = "Retrieves details for a specific detail by slug.", body = serde_json::Value),
        (status = 500, description = "Internal Server Error", body = String)
    )
)]
pub async fn detail_slug(
    State(app_state): State<Arc<AppState>>,
    Path(slug): Path<String>,
) -> Result<Json<crate::modules::anime::types::DetailResponse>, AppError> {
    info!("Starting request for detail slug: {}", slug);
    let service = AnimeService::new(AnimeRepository::new());
    service.get_anime_detail(app_state, slug).await.map(Json)
}

#[utoipa::path(
    get,
    path = "/api/anime/complete_anime/{slug}",
    tag = "anime",
    operation_id = "anime_complete_anime_slug",
    responses(
        (status = 200, description = "Retrieves details for a specific complete_anime by slug.", body = serde_json::Value),
        (status = 500, description = "Internal Server Error", body = String)
    )
)]
pub async fn complete_anime_slug(
    State(app_state): State<Arc<AppState>>,
    Path(slug): Path<String>,
) -> Result<Json<crate::modules::anime::types::ListResponse>, AppError> {
    info!("Starting request for complete_anime slug: {}", slug);
    let service = AnimeService::new(AnimeRepository::new());
    service
        .get_complete_anime_page(app_state, slug)
        .await
        .map(Json)
}

#[utoipa::path(
    get,
    path = "/api/anime/full/{slug}",
    tag = "anime",
    operation_id = "anime_full_slug",
    responses(
        (status = 200, description = "Retrieves full episode details for a specific episode by slug.", body = serde_json::Value),
        (status = 500, description = "Internal Server Error", body = String)
    )
)]
pub async fn full_slug(
    State(app_state): State<Arc<AppState>>,
    Path(slug): Path<String>,
) -> Result<Json<crate::modules::anime::types::FullResponse>, AppError> {
    info!("Starting request for full slug: {}", slug);
    let service = AnimeService::new(AnimeRepository::new());
    service.get_anime_full(app_state, slug).await.map(Json)
}

#[utoipa::path(
    get,
    path = "/api/anime/ongoing_anime/{slug}",
    tag = "anime",
    operation_id = "anime_ongoing_anime_slug",
    responses(
        (status = 200, description = "Retrieves details for a specific ongoing_anime by slug.", body = serde_json::Value),
        (status = 500, description = "Internal Server Error", body = String)
    )
)]
pub async fn ongoing_anime_slug(
    State(app_state): State<Arc<AppState>>,
    Path(slug): Path<String>,
) -> Result<Json<crate::modules::anime::types::OngoingAnimeResponse>, AppError> {
    info!("Starting request for ongoing_anime slug: {}", slug);
    let service = AnimeService::new(AnimeRepository::new());
    service
        .get_ongoing_anime_page(app_state, slug)
        .await
        .map(Json)
}

#[utoipa::path(
    get,
    path = "/api/anime/latest/{slug}",
    tag = "anime",
    operation_id = "anime_latest_slug",
    responses(
        (status = 200, description = "Retrieves details for a specific latest by slug.", body = serde_json::Value),
        (status = 500, description = "Internal Server Error", body = String)
    )
)]
pub async fn latest_slug(
    State(app_state): State<Arc<AppState>>,
    Path(slug): Path<String>,
) -> Result<Json<crate::modules::anime::types::LatestAnimeResponse>, AppError> {
    info!("Starting request for latest slug: {}", slug);
    let service = AnimeService::new(AnimeRepository::new());
    service
        .get_latest_anime_page(app_state, slug)
        .await
        .map(Json)
}

#[utoipa::path(
    get,
    path = "/api/anime/search/{slug}",
    tag = "anime",
    operation_id = "anime_search_slug_index",
    responses(
        (status = 200, description = "Retrieves details for a specific search by slug.", body = serde_json::Value),
        (status = 500, description = "Internal Server Error", body = String)
    )
)]
pub async fn search_slug_index(
    State(app_state): State<Arc<AppState>>,
    Path(slug): Path<String>,
) -> Result<Json<crate::modules::anime::types::SearchResponse>, AppError> {
    info!("Starting request for search slug: {}", slug);
    let service = AnimeService::new(AnimeRepository::new());
    service
        .get_search_anime_page(app_state, slug, "1".to_string())
        .await
        .map(Json)
}

#[utoipa::path(
    get,
    path = "/api/anime/search/{slug}/{page}",
    tag = "anime",
    operation_id = "anime_search_slug_page",
    responses(
        (status = 200, description = "Retrieves details for a specific search by slug and page.", body = serde_json::Value),
        (status = 500, description = "Internal Server Error", body = String)
    )
)]
pub async fn search_slug_page(
    State(app_state): State<Arc<AppState>>,
    Path((slug, page)): Path<(String, String)>,
) -> Result<Json<crate::modules::anime::types::SearchResponse>, AppError> {
    info!("Starting request for search slug: {} page: {}", slug, page);
    let service = AnimeService::new(AnimeRepository::new());
    service
        .get_search_anime_page(app_state, slug, page)
        .await
        .map(Json)
}

#[utoipa::path(
    get,
    path = "/api/anime/genre/{slug}",
    tag = "anime",
    operation_id = "anime_genre_slug_index",
    responses(
        (status = 200, description = "Retrieves details for a specific genre by slug.", body = serde_json::Value),
        (status = 500, description = "Internal Server Error", body = String)
    )
)]
pub async fn genre_slug_index(
    State(app_state): State<Arc<AppState>>,
    Path(slug): Path<String>,
) -> Result<Json<crate::modules::anime::types::GenreListResponse>, AppError> {
    info!("Starting request for genre slug: {}", slug);
    let service = AnimeService::new(AnimeRepository::new());
    service
        .get_genre_anime_page(app_state, slug, "1".to_string())
        .await
        .map(Json)
}

#[utoipa::path(
    get,
    path = "/api/anime/genre/{slug}/{page}",
    tag = "anime",
    operation_id = "anime_genre_slug_page",
    responses(
        (status = 200, description = "Retrieves details for a specific genre by slug and page.", body = serde_json::Value),
        (status = 500, description = "Internal Server Error", body = String)
    )
)]
pub async fn genre_slug_page(
    State(app_state): State<Arc<AppState>>,
    Path((slug, page)): Path<(String, String)>,
) -> Result<Json<crate::modules::anime::types::GenreListResponse>, AppError> {
    info!("Starting request for genre slug: {} page: {}", slug, page);
    let service = AnimeService::new(AnimeRepository::new());
    service
        .get_genre_anime_page(app_state, slug, page)
        .await
        .map(Json)
}
