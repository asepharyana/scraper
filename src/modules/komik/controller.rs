use std::sync::Arc;

use axum::{extract::State, Json};

use crate::modules::komik::repository::KomikRepository;
use crate::modules::komik::service::KomikService;
use crate::shared::errors::AppError;
use crate::shared::state::AppState;

#[utoipa::path(
    get,
    path = "/api/komik/genre_list",
    tag = "komik",
    operation_id = "komik_genre_list",
    responses(
        (status = 200, description = "Handles GET requests for the /api/komik/genre_list endpoint.", body = serde_json::Value),
        (status = 500, description = "Internal Server Error", body = String)
    )
)]
pub async fn genre_list(
    State(app_state): State<Arc<AppState>>,
) -> Result<Json<crate::modules::komik::types::GenresResponse>, AppError> {
    let service = KomikService::new(KomikRepository::new());
    service.genre_list(app_state).await.map(Json)
}

#[utoipa::path(
    get,
    path = "/api/komik/chapter/{slug}",
    tag = "komik",
    operation_id = "komik_chapter_slug",
    responses(
        (status = 200, description = "Retrieves details for a specific chapter by slug.", body = serde_json::Value),
        (status = 500, description = "Internal Server Error", body = String)
    )
)]
pub async fn chapter_slug(
    State(app_state): State<Arc<AppState>>,
    axum::extract::Path(slug): axum::extract::Path<String>,
) -> Result<Json<crate::modules::komik::types::ChapterResponse>, AppError> {
    let service = KomikService::new(KomikRepository::new());
    service.chapter_slug(slug, app_state).await.map(Json)
}

#[utoipa::path(
    get,
    path = "/api/komik/detail/{slug}",
    tag = "komik",
    operation_id = "komik_detail_slug",
    responses(
        (status = 200, description = "Retrieves details for a specific detail by slug.", body = serde_json::Value),
        (status = 500, description = "Internal Server Error", body = String)
    )
)]
pub async fn detail_slug(
    State(app_state): State<Arc<AppState>>,
    axum::extract::Path(slug): axum::extract::Path<String>,
) -> Result<Json<crate::modules::komik::types::DetailResponse>, AppError> {
    let service = KomikService::new(KomikRepository::new());
    service.detail_slug(slug, app_state).await.map(Json)
}

#[utoipa::path(
    get,
    path = "/api/komik/genre/{slug}",
    tag = "komik",
    operation_id = "komik_genre_slug_index",
    responses(
        (status = 200, description = "Retrieves details for a specific genre by slug.", body = serde_json::Value),
        (status = 500, description = "Internal Server Error", body = String)
    )
)]
pub async fn genre_slug(
    State(app_state): State<Arc<AppState>>,
    axum::extract::Path(slug): axum::extract::Path<String>,
) -> Result<Json<crate::modules::komik::types::GenreKomikResponse>, AppError> {
    let service = KomikService::new(KomikRepository::new());
    service.genre_slug(slug, app_state).await.map(Json)
}

#[utoipa::path(
    get,
    path = "/api/komik/genre/{slug}/{page}",
    tag = "komik",
    operation_id = "komik_genre_slug_page",
    responses(
        (status = 200, description = "Retrieves paginated genre results by slug.", body = serde_json::Value),
        (status = 500, description = "Internal Server Error", body = String)
    )
)]
pub async fn genre_slug_page(
    State(app_state): State<Arc<AppState>>,
    axum::extract::Path((slug, page)): axum::extract::Path<(String, String)>,
) -> Result<Json<crate::modules::komik::types::GenreKomikResponse>, AppError> {
    let page_num = page
        .parse::<u32>()
        .map_err(|_| AppError::ScraperError("Invalid page number".to_string()))?;
    let service = KomikService::new(KomikRepository::new());
    service
        .genre_slug_page(slug, page_num, app_state)
        .await
        .map(Json)
}

#[utoipa::path(
    get,
    path = "/api/komik/manga/{slug}",
    tag = "komik",
    operation_id = "komik_manga_slug",
    responses(
        (status = 200, description = "Retrieves manga details by slug.", body = serde_json::Value),
        (status = 500, description = "Internal Server Error", body = String)
    )
)]
pub async fn manga_slug(
    State(app_state): State<Arc<AppState>>,
    axum::extract::Path(slug): axum::extract::Path<String>,
) -> Result<Json<crate::modules::komik::types::GenreKomikResponse>, AppError> {
    let service = KomikService::new(KomikRepository::new());
    service.manga_slug(slug, app_state).await.map(Json)
}

#[utoipa::path(
    get,
    path = "/api/komik/manhua/{slug}",
    tag = "komik",
    operation_id = "komik_manhua_slug",
    responses(
        (status = 200, description = "Retrieves manhua details by slug.", body = serde_json::Value),
        (status = 500, description = "Internal Server Error", body = String)
    )
)]
pub async fn manhua_slug(
    State(app_state): State<Arc<AppState>>,
    axum::extract::Path(slug): axum::extract::Path<String>,
) -> Result<Json<crate::modules::komik::types::GenreKomikResponse>, AppError> {
    let service = KomikService::new(KomikRepository::new());
    service.manhua_slug(slug, app_state).await.map(Json)
}

#[utoipa::path(
    get,
    path = "/api/komik/manhwa/{slug}",
    tag = "komik",
    operation_id = "komik_manhwa_slug",
    responses(
        (status = 200, description = "Retrieves manhwa details by slug.", body = serde_json::Value),
        (status = 500, description = "Internal Server Error", body = String)
    )
)]
pub async fn manhwa_slug(
    State(app_state): State<Arc<AppState>>,
    axum::extract::Path(slug): axum::extract::Path<String>,
) -> Result<Json<crate::modules::komik::types::GenreKomikResponse>, AppError> {
    let service = KomikService::new(KomikRepository::new());
    service.manhwa_slug(slug, app_state).await.map(Json)
}

#[utoipa::path(
    get,
    path = "/api/komik/popular/{slug}",
    tag = "komik",
    operation_id = "komik_popular_slug",
    responses(
        (status = 200, description = "Retrieves popular komik details by slug.", body = serde_json::Value),
        (status = 500, description = "Internal Server Error", body = String)
    )
)]
pub async fn popular_slug(
    State(app_state): State<Arc<AppState>>,
    axum::extract::Path(slug): axum::extract::Path<String>,
) -> Result<Json<crate::modules::komik::types::GenreKomikResponse>, AppError> {
    let service = KomikService::new(KomikRepository::new());
    service.popular_slug(slug, app_state).await.map(Json)
}

#[utoipa::path(
    get,
    path = "/api/komik/search/{slug}",
    tag = "komik",
    operation_id = "komik_search_slug_index",
    responses(
        (status = 200, description = "Retrieves search results by query slug.", body = serde_json::Value),
        (status = 500, description = "Internal Server Error", body = String)
    )
)]
pub async fn search_slug(
    State(app_state): State<Arc<AppState>>,
    axum::extract::Path(slug): axum::extract::Path<String>,
) -> Result<Json<crate::modules::komik::types::SearchKomikResponse>, AppError> {
    let service = KomikService::new(KomikRepository::new());
    service.search_slug(slug, app_state).await.map(Json)
}

#[utoipa::path(
    get,
    path = "/api/komik/search/{slug}/{page}",
    tag = "komik",
    operation_id = "komik_search_slug_page",
    responses(
        (status = 200, description = "Retrieves paginated search results by query slug.", body = serde_json::Value),
        (status = 500, description = "Internal Server Error", body = String)
    )
)]
pub async fn search_slug_page(
    State(app_state): State<Arc<AppState>>,
    axum::extract::Path((slug, page)): axum::extract::Path<(String, String)>,
) -> Result<Json<crate::modules::komik::types::SearchKomikResponse>, AppError> {
    let page_num = page
        .parse::<u32>()
        .map_err(|_| AppError::ScraperError("Invalid page number".to_string()))?;
    let service = KomikService::new(KomikRepository::new());
    service
        .search_slug_page(slug, page_num, app_state)
        .await
        .map(Json)
}
