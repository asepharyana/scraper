use std::sync::Arc;

use axum::{
    extract::{Path, Query, State},
    Json,
};

use crate::modules::anime2::repository::Anime2Repository;
use crate::modules::anime2::schema::FilterQuery;
use crate::modules::anime2::service::Anime2Service;
use crate::shared::errors::AppError;
use crate::shared::state::AppState;

#[utoipa::path(
    get,
    path = "/api/anime2",
    tag = "anime2",
    operation_id = "anime2_index",
    responses(
        (status = 200, description = "Handles GET requests for the /api/anime2 endpoint.", body = serde_json::Value),
        (status = 500, description = "Internal Server Error", body = String)
    )
)]
pub async fn index(
    State(app_state): State<Arc<AppState>>,
) -> Result<Json<crate::modules::anime2::types::Anime2Response>, AppError> {
    let service = Anime2Service::new(Anime2Repository::new());
    Ok(Json(service.index(app_state).await?))
}

#[utoipa::path(
    get,
    path = "/api/anime2/genre_list",
    tag = "anime2",
    operation_id = "anime2_genre_list",
    responses(
        (status = 200, description = "Handles GET requests for the /api/anime2/genre_list endpoint.", body = serde_json::Value),
        (status = 500, description = "Internal Server Error", body = String)
    )
)]
pub async fn genre_list(
    State(app_state): State<Arc<AppState>>,
) -> Result<Json<crate::modules::anime2::types::GenresResponse>, AppError> {
    let service = Anime2Service::new(Anime2Repository::new());
    Ok(Json(service.genre_list(app_state).await?))
}

#[utoipa::path(
    get,
    path = "/api/anime2/filter",
    tag = "anime2",
    operation_id = "anime2_filter",
    responses(
        (status = 200, description = "Handles GET requests for the /api/anime2/filter endpoint.", body = serde_json::Value),
        (status = 500, description = "Internal Server Error", body = String)
    )
)]
pub async fn filter(
    State(app_state): State<Arc<AppState>>,
    Query(params): Query<FilterQuery>,
) -> Result<Json<crate::modules::anime2::types::FilterResponse>, AppError> {
    let page = params.page.unwrap_or(1);
    let genre = params.genre.clone();
    let status = params.status.clone();
    let anime_type = params.r#type.clone();
    let order = params.order.clone().unwrap_or("update".to_string());

    let service = Anime2Service::new(Anime2Repository::new());
    Ok(Json(
        service
            .filter(app_state, page, genre, status, anime_type, order)
            .await?,
    ))
}

#[utoipa::path(
    get,
    path = "/api/anime2/detail/{slug}",
    tag = "anime2",
    operation_id = "anime2_detail_slug",
    responses(
        (status = 200, description = "Retrieves details for a specific detail by slug.", body = serde_json::Value),
        (status = 500, description = "Internal Server Error", body = String)
    )
)]
pub async fn detail_slug(
    State(app_state): State<Arc<AppState>>,
    Path(slug): Path<String>,
) -> Result<Json<crate::modules::anime2::types::DetailResponse>, AppError> {
    let service = Anime2Service::new(Anime2Repository::new());
    Ok(Json(service.detail(app_state, slug).await?))
}

#[utoipa::path(
    get,
    path = "/api/anime2/genre/{slug}",
    tag = "anime2",
    operation_id = "anime2_genre_slug_index",
    responses(
        (status = 200, description = "Retrieves details for a specific genre by slug.", body = serde_json::Value),
        (status = 500, description = "Internal Server Error", body = String)
    )
)]
pub async fn genre_slug_index(
    State(app_state): State<Arc<AppState>>,
    Path(slug): Path<String>,
) -> Result<
    Json<
        crate::shared::types::ApiResponse<
            Vec<crate::shared::types::entities::anime::GenreAnimeItem>,
        >,
    >,
    AppError,
> {
    let service = Anime2Service::new(Anime2Repository::new());
    Ok(Json(service.genre_slug(app_state, slug, 1).await?))
}

#[utoipa::path(
    get,
    path = "/api/anime2/genre/{slug}/{page}",
    tag = "anime2",
    operation_id = "anime2_genre_slug_page",
    responses(
        (status = 200, description = "Handles GET requests for the /api/anime2/genre/{slug}/{page} endpoint.", body = serde_json::Value),
        (status = 500, description = "Internal Server Error", body = String)
    )
)]
pub async fn genre_slug_page(
    State(app_state): State<Arc<AppState>>,
    Path((slug, page)): Path<(String, u32)>,
) -> Result<
    Json<
        crate::shared::types::ApiResponse<
            Vec<crate::shared::types::entities::anime::GenreAnimeItem>,
        >,
    >,
    AppError,
> {
    let service = Anime2Service::new(Anime2Repository::new());
    Ok(Json(service.genre_slug(app_state, slug, page).await?))
}

#[utoipa::path(
    get,
    path = "/api/anime2/search/{slug}",
    tag = "anime2",
    operation_id = "anime2_search_slug_index",
    responses(
        (status = 200, description = "Retrieves details for a specific search by slug.", body = serde_json::Value),
        (status = 500, description = "Internal Server Error", body = String)
    )
)]
pub async fn search_slug_index(
    State(app_state): State<Arc<AppState>>,
    Path(slug): Path<String>,
) -> Result<
    Json<
        crate::shared::types::ApiResponse<
            Vec<crate::shared::types::entities::anime::SearchAnimeItem>,
        >,
    >,
    AppError,
> {
    let service = Anime2Service::new(Anime2Repository::new());
    Ok(Json(service.search(app_state, slug, 1).await?))
}

#[utoipa::path(
    get,
    path = "/api/anime2/search/{slug}/{page}",
    tag = "anime2",
    operation_id = "anime2_search_slug_page",
    responses(
        (status = 200, description = "Handles GET requests for the /api/anime2/search/{slug}/{page} endpoint.", body = serde_json::Value),
        (status = 500, description = "Internal Server Error", body = String)
    )
)]
pub async fn search_slug_page(
    State(app_state): State<Arc<AppState>>,
    Path((slug, page)): Path<(String, u32)>,
) -> Result<
    Json<
        crate::shared::types::ApiResponse<
            Vec<crate::shared::types::entities::anime::SearchAnimeItem>,
        >,
    >,
    AppError,
> {
    let service = Anime2Service::new(Anime2Repository::new());
    Ok(Json(service.search(app_state, slug, page).await?))
}

#[utoipa::path(
    get,
    path = "/api/anime2/latest/{slug}",
    tag = "anime2",
    operation_id = "anime2_latest_slug",
    responses(
        (status = 200, description = "Retrieves details for a specific latest by slug.", body = serde_json::Value),
        (status = 500, description = "Internal Server Error", body = String)
    )
)]
pub async fn latest_slug(
    State(app_state): State<Arc<AppState>>,
    Path(slug): Path<String>,
) -> Result<
    Json<
        crate::shared::types::ApiResponse<
            Vec<crate::shared::types::entities::anime::LatestAnimeItem>,
        >,
    >,
    AppError,
> {
    let page = slug
        .parse::<u32>()
        .map_err(|_| AppError::ScraperError(format!("Invalid page number: {}", slug)))?;
    let service = Anime2Service::new(Anime2Repository::new());
    Ok(Json(service.latest(app_state, page).await?))
}

#[utoipa::path(
    get,
    path = "/api/anime2/ongoing_anime/{slug}",
    tag = "anime2",
    operation_id = "anime2_ongoing_anime_slug",
    responses(
        (status = 200, description = "Retrieves details for a specific ongoing_anime by slug.", body = serde_json::Value),
        (status = 500, description = "Internal Server Error", body = String)
    )
)]
pub async fn ongoing_anime_slug(
    State(app_state): State<Arc<AppState>>,
    Path(slug): Path<String>,
) -> Result<
    Json<
        crate::shared::types::ApiResponse<
            Vec<crate::shared::types::entities::anime::OngoingAnimeItemWithScore>,
        >,
    >,
    AppError,
> {
    let page = slug
        .parse::<u32>()
        .map_err(|_| AppError::ScraperError(format!("Invalid page number: {}", slug)))?;
    let service = Anime2Service::new(Anime2Repository::new());
    Ok(Json(service.ongoing_anime(app_state, page).await?))
}

#[utoipa::path(
    get,
    path = "/api/anime2/complete_anime/{slug}",
    tag = "anime2",
    operation_id = "anime2_complete_anime_slug",
    responses(
        (status = 200, description = "Retrieves details for a specific complete_anime by slug.", body = serde_json::Value),
        (status = 500, description = "Internal Server Error", body = String)
    )
)]
pub async fn complete_anime_slug(
    State(app_state): State<Arc<AppState>>,
    Path(slug): Path<String>,
) -> Result<
    Json<
        crate::shared::types::ApiResponse<
            Vec<crate::shared::types::entities::anime::CompleteAnimeItem>,
        >,
    >,
    AppError,
> {
    let page = slug
        .parse::<u32>()
        .map_err(|_| AppError::ScraperError(format!("Invalid page number: {}", slug)))?;
    let service = Anime2Service::new(Anime2Repository::new());
    Ok(Json(service.complete_anime(app_state, page).await?))
}
