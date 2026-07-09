use std::sync::Arc;

use crate::modules::anime2::parser;
use crate::modules::anime2::repository::Anime2Repository;
use crate::modules::anime2::types::{DetailResponse, GenresResponse};
use crate::shared::database::traits::scraping_repository::ScrapingRepository;
use crate::shared::errors::AppError;
use crate::shared::services::images::cache::{
    apply_cached_posters, cache_image_urls_batch_lazy, get_cached_or_original,
};
use crate::shared::state::AppState;
use crate::shared::types::ApiResponse;
use crate::shared::utils::Cache;

const INDEX_CACHE_TTL: u64 = 300;
const GENRE_LIST_CACHE_TTL: u64 = 3600;
const FILTER_CACHE_TTL: u64 = 300;
const DETAIL_CACHE_TTL: u64 = 300;
const GENRE_CACHE_TTL: u64 = 300;
const SEARCH_CACHE_TTL: u64 = 300;
const LATEST_CACHE_TTL: u64 = 120;
const ONGOING_CACHE_TTL: u64 = 300;
const COMPLETE_CACHE_TTL: u64 = 300;

pub struct Anime2Service {
    repository: Anime2Repository,
}

impl Anime2Service {
    pub fn new(repository: Anime2Repository) -> Self {
        Self { repository }
    }

    pub async fn index(
        &self,
        app_state: Arc<AppState>,
    ) -> Result<crate::modules::anime2::types::Anime2Response, AppError> {
        let cache = Cache::new(&app_state.redis_pool);

        cache
            .get_or_set("anime2:index", INDEX_CACHE_TTL, || async {
                let ongoing_html = self
                    .repository
                    .fetch_html(&self.repository.index_ongoing_url())
                    .await
                    .map_err(|e| e.to_string())?;
                let complete_html = self
                    .repository
                    .fetch_html(&self.repository.index_complete_url())
                    .await
                    .map_err(|e| e.to_string())?;

                let mut data = tokio::task::spawn_blocking(move || {
                    Ok::<_, String>((
                        parser::parse_ongoing_anime(&ongoing_html).map_err(|e| e.to_string())?,
                        parser::parse_complete_anime(&complete_html).map_err(|e| e.to_string())?,
                    ))
                })
                .await
                .map_err(|e| e.to_string())??;

                let mut posters: Vec<String> =
                    data.0.iter().map(|item| item.poster.clone()).collect();
                posters.extend(data.1.iter().map(|item| item.poster.clone()));

                let cached_posters = cache_image_urls_batch_lazy(
                    app_state.db.clone(),
                    &app_state.redis_pool,
                    posters,
                    Some(app_state.image_processing_semaphore.clone()),
                )
                .await;

                let ongoing_len = data.0.len();
                for (i, item) in data.0.iter_mut().enumerate() {
                    if let Some(url) = cached_posters.get(i) {
                        item.poster = url.clone();
                    }
                }
                for (i, item) in data.1.iter_mut().enumerate() {
                    if let Some(url) = cached_posters.get(ongoing_len + i) {
                        item.poster = url.clone();
                    }
                }

                Ok(crate::modules::anime2::types::Anime2Response {
                    status: "Ok".to_string(),
                    data: crate::modules::anime2::types::Anime2Data {
                        ongoing_anime: data.0,
                        complete_anime: data.1,
                    },
                })
            })
            .await
            .map_err(AppError::ScraperError)
    }

    pub async fn genre_list(&self, app_state: Arc<AppState>) -> Result<GenresResponse, AppError> {
        let cache = Cache::new(&app_state.redis_pool);

        cache
            .get_or_set("anime2:genres:list:v3", GENRE_LIST_CACHE_TTL, || async {
                let html = self
                    .repository
                    .fetch_html(&self.repository.genre_list_url())
                    .await
                    .map_err(|e| e.to_string())?;

                let genres = tokio::task::spawn_blocking(move || {
                    parser::parse_genres(&html).map_err(|e| e.to_string())
                })
                .await
                .map_err(|e| e.to_string())??;

                Ok(GenresResponse {
                    status: "Ok".to_string(),
                    data: genres,
                })
            })
            .await
            .map_err(AppError::ScraperError)
    }

    pub async fn filter(
        &self,
        app_state: Arc<AppState>,
        page: u32,
        genre: Option<String>,
        status: Option<String>,
        anime_type: Option<String>,
        order: String,
    ) -> Result<crate::modules::anime2::types::FilterResponse, AppError> {
        let cache = Cache::new(&app_state.redis_pool);
        let cache_key = format!(
            "anime2:filter:{}:{:?}:{:?}:{:?}:{}",
            page, genre, status, anime_type, order
        );
        let genre_clone = genre.clone();
        let status_clone = status.clone();
        let anime_type_clone = anime_type.clone();

        cache
            .get_or_set(&cache_key, FILTER_CACHE_TTL, || async {
                let mut url = self.repository.filter_url(page, &order);

                if let Some(g) = &genre {
                    for genre_item in g.split(',') {
                        url.push_str(&format!("&genre[]={}", genre_item.trim()));
                    }
                }
                if let Some(s) = &status {
                    url.push_str(&format!("&status={}", s));
                }
                if let Some(t) = &anime_type {
                    url.push_str(&format!("&type={}", t));
                }

                let html = self
                    .repository
                    .fetch_html(&url)
                    .await
                    .map_err(|e| e.to_string())?;
                let (data, pagination) = tokio::task::spawn_blocking(move || {
                    parser::parse_filter_page(&html, page).map_err(|e| e.to_string())
                })
                .await
                .map_err(|e| e.to_string())??;

                let mut final_data = data;
                apply_cached_posters(
                    &mut final_data,
                    app_state.db.clone(),
                    &app_state.redis_pool,
                    Some(app_state.image_processing_semaphore.clone()),
                )
                .await;

                Ok(crate::modules::anime2::types::FilterResponse {
                    success: true,
                    data: final_data,
                    pagination,
                    filters_applied: crate::modules::anime2::types::FiltersApplied {
                        genre: genre_clone,
                        status: status_clone,
                        r#type: anime_type_clone,
                        order: order.clone(),
                    },
                    status: "Ok".to_string(),
                })
            })
            .await
            .map_err(AppError::ScraperError)
    }

    pub async fn detail(
        &self,
        app_state: Arc<AppState>,
        slug: String,
    ) -> Result<DetailResponse, AppError> {
        let cache = Cache::new(&app_state.redis_pool);
        let cache_key = format!("anime2:detail:{}", slug);

        cache
            .get_or_set(&cache_key, DETAIL_CACHE_TTL, || async {
                let detail_url = self.repository.detail_url(&slug);
                let image_url = self.repository.detail_image_url(&slug);
                let (detail_html, image_html) = tokio::join!(
                    self.repository.fetch_html(&detail_url),
                    self.repository.fetch_html(&image_url)
                );
                let detail_html = detail_html.map_err(|e| e.to_string())?;
                let image_html = image_html.ok();

                let mut data = tokio::task::spawn_blocking(move || {
                    let mut data =
                        parser::parse_anime_detail(&detail_html).map_err(|e| e.to_string())?;
                    if let Some(image_html) = image_html {
                        if let Ok(image_data) = parser::parse_anime_detail(&image_html) {
                            if !image_data.poster.is_empty() {
                                data.poster = image_data.poster;
                            }
                            if !image_data.poster2.is_empty() {
                                data.poster2 = image_data.poster2;
                            }
                            data.recommendations = image_data.recommendations;
                        }
                    }
                    Ok::<_, String>(data)
                })
                .await
                .map_err(|e| e.to_string())??;

                data.poster = get_cached_or_original(
                    app_state.db.clone(),
                    &app_state.redis_pool,
                    &data.poster,
                    Some(app_state.image_processing_semaphore.clone()),
                )
                .await;
                data.poster2 = get_cached_or_original(
                    app_state.db.clone(),
                    &app_state.redis_pool,
                    &data.poster2,
                    Some(app_state.image_processing_semaphore.clone()),
                )
                .await;

                apply_cached_posters(
                    &mut data.recommendations,
                    app_state.db.clone(),
                    &app_state.redis_pool,
                    Some(app_state.image_processing_semaphore.clone()),
                )
                .await;

                Ok(DetailResponse {
                    status: "Ok".to_string(),
                    data,
                })
            })
            .await
            .map_err(AppError::ScraperError)
    }

    pub async fn genre_slug(
        &self,
        app_state: Arc<AppState>,
        genre_slug: String,
        page: u32,
    ) -> Result<ApiResponse<Vec<crate::shared::types::entities::anime::GenreAnimeItem>>, AppError>
    {
        let cache = Cache::new(&app_state.redis_pool);
        let cache_key = format!("anime2:genre:{}:{}", genre_slug, page);

        cache
            .get_or_set(&cache_key, GENRE_CACHE_TTL, || async {
                let html = self
                    .repository
                    .fetch_html(&self.repository.genre_page_url(&genre_slug, page))
                    .await
                    .map_err(|e| e.to_string())?;
                let (data, _pagination) = tokio::task::spawn_blocking(move || {
                    parser::parse_genre_page(&html, page).map_err(|e| e.to_string())
                })
                .await
                .map_err(|e| e.to_string())??;

                let mut final_data = data;
                apply_cached_posters(
                    &mut final_data,
                    app_state.db.clone(),
                    &app_state.redis_pool,
                    Some(app_state.image_processing_semaphore.clone()),
                )
                .await;

                Ok(ApiResponse::success(final_data))
            })
            .await
            .map_err(AppError::ScraperError)
    }

    pub async fn search(
        &self,
        app_state: Arc<AppState>,
        query: String,
        page: u32,
    ) -> Result<ApiResponse<Vec<crate::shared::types::entities::anime::SearchAnimeItem>>, AppError>
    {
        let cache = Cache::new(&app_state.redis_pool);
        let cache_key = format!("anime2:search:{}:{}", query, page);

        cache
            .get_or_set(&cache_key, SEARCH_CACHE_TTL, || async {
                let html = self
                    .repository
                    .fetch_html(&self.repository.search_url(&query, page))
                    .await
                    .map_err(|e| e.to_string())?;
                let (data, _pagination) = tokio::task::spawn_blocking(move || {
                    parser::parse_search_page(&html, page).map_err(|e| e.to_string())
                })
                .await
                .map_err(|e| e.to_string())??;

                let mut final_data = data;
                apply_cached_posters(
                    &mut final_data,
                    app_state.db.clone(),
                    &app_state.redis_pool,
                    Some(app_state.image_processing_semaphore.clone()),
                )
                .await;

                Ok(ApiResponse::success(final_data))
            })
            .await
            .map_err(AppError::ScraperError)
    }

    pub async fn latest(
        &self,
        app_state: Arc<AppState>,
        page: u32,
    ) -> Result<ApiResponse<Vec<crate::shared::types::entities::anime::LatestAnimeItem>>, AppError>
    {
        let cache = Cache::new(&app_state.redis_pool);
        let cache_key = format!("anime2:latest:{}", page);

        cache
            .get_or_set(&cache_key, LATEST_CACHE_TTL, || async {
                let html = self
                    .repository
                    .fetch_html(&self.repository.latest_url(page))
                    .await
                    .map_err(|e| e.to_string())?;
                let (data, _pagination) = tokio::task::spawn_blocking(move || {
                    parser::parse_latest_page(&html, page).map_err(|e| e.to_string())
                })
                .await
                .map_err(|e| e.to_string())??;

                let mut final_data = data;
                apply_cached_posters(
                    &mut final_data,
                    app_state.db.clone(),
                    &app_state.redis_pool,
                    Some(app_state.image_processing_semaphore.clone()),
                )
                .await;

                Ok(ApiResponse::success(final_data))
            })
            .await
            .map_err(AppError::ScraperError)
    }

    pub async fn ongoing_anime(
        &self,
        app_state: Arc<AppState>,
        page: u32,
    ) -> Result<
        ApiResponse<Vec<crate::shared::types::entities::anime::OngoingAnimeItemWithScore>>,
        AppError,
    > {
        let cache = Cache::new(&app_state.redis_pool);
        let cache_key = format!("anime2:ongoing:{}", page);

        cache
            .get_or_set(&cache_key, ONGOING_CACHE_TTL, || async {
                let html = self
                    .repository
                    .fetch_html(&self.repository.ongoing_url(page))
                    .await
                    .map_err(|e| e.to_string())?;
                let (data, _pagination) = tokio::task::spawn_blocking(move || {
                    parser::parse_ongoing_page(&html, page).map_err(|e| e.to_string())
                })
                .await
                .map_err(|e| e.to_string())??;

                let mut final_data = data;
                apply_cached_posters(
                    &mut final_data,
                    app_state.db.clone(),
                    &app_state.redis_pool,
                    Some(app_state.image_processing_semaphore.clone()),
                )
                .await;

                Ok(ApiResponse::success(final_data))
            })
            .await
            .map_err(AppError::ScraperError)
    }

    pub async fn complete_anime(
        &self,
        app_state: Arc<AppState>,
        page: u32,
    ) -> Result<ApiResponse<Vec<crate::shared::types::entities::anime::CompleteAnimeItem>>, AppError>
    {
        let cache = Cache::new(&app_state.redis_pool);
        let cache_key = format!("anime2:complete:{}", page);

        cache
            .get_or_set(&cache_key, COMPLETE_CACHE_TTL, || async {
                let html = self
                    .repository
                    .fetch_html(&self.repository.complete_url(page))
                    .await
                    .map_err(|e| e.to_string())?;
                let (data, _pagination) = tokio::task::spawn_blocking(move || {
                    parser::parse_complete_page(&html, page).map_err(|e| e.to_string())
                })
                .await
                .map_err(|e| e.to_string())??;

                let mut final_data = data;
                apply_cached_posters(
                    &mut final_data,
                    app_state.db.clone(),
                    &app_state.redis_pool,
                    Some(app_state.image_processing_semaphore.clone()),
                )
                .await;

                Ok(ApiResponse::success(final_data))
            })
            .await
            .map_err(AppError::ScraperError)
    }
}
