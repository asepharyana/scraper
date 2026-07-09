use std::sync::Arc;

use crate::modules::anime::repository::AnimeRepository;
use crate::modules::anime::types::*;
use crate::shared::errors::AppError;
use crate::shared::state::AppState;
use crate::shared::utils::Cache;

const INDEX_CACHE_TTL: u64 = 10;
const GENRE_LIST_CACHE_TTL: u64 = 3600;
const DEFAULT_CACHE_TTL: u64 = 300;

pub struct AnimeService {
    repository: AnimeRepository,
}

impl AnimeService {
    pub fn new(repository: AnimeRepository) -> Self {
        Self { repository }
    }

    pub async fn get_anime_index(&self, app_state: Arc<AppState>) -> Result<AnimeData, AppError> {
        let cache = Cache::new(&app_state.redis_pool);

        cache
            .get_or_set("anime:index:v2", INDEX_CACHE_TTL, || async {
                let mut data = self
                    .repository
                    .fetch_anime_index()
                    .await
                    .map_err(|e| e.to_string())?;

                if data.ongoing_anime.is_empty() && data.complete_anime.is_empty() {
                    return Err("Empty anime index — refusing to cache".to_string());
                }

                let mut posters: Vec<String> = data
                    .ongoing_anime
                    .iter()
                    .map(|item| item.poster.clone())
                    .collect();
                posters.extend(data.complete_anime.iter().map(|item| item.poster.clone()));

                let cached_posters =
                    crate::shared::services::images::cache::cache_image_urls_batch_lazy(
                        app_state.db.clone(),
                        &app_state.redis_pool,
                        posters,
                        Some(app_state.image_processing_semaphore.clone()),
                    )
                    .await;

                let ongoing_len = data.ongoing_anime.len();
                for (i, item) in data.ongoing_anime.iter_mut().enumerate() {
                    if let Some(url) = cached_posters.get(i) {
                        item.poster = url.clone();
                    }
                }
                for (i, item) in data.complete_anime.iter_mut().enumerate() {
                    if let Some(url) = cached_posters.get(ongoing_len + i) {
                        item.poster = url.clone();
                    }
                }

                Ok(data)
            })
            .await
            .map_err(|e| AppError::ScraperError(e))
    }

    pub async fn get_genres(&self, app_state: Arc<AppState>) -> Result<GenresResponse, AppError> {
        let cache = Cache::new(&app_state.redis_pool);

        cache
            .get_or_set("anime:genres:list", GENRE_LIST_CACHE_TTL, || async {
                let genres = self
                    .repository
                    .fetch_genres()
                    .await
                    .map_err(|e| e.to_string())?;
                Ok(GenresResponse {
                    status: "Ok".to_string(),
                    data: genres,
                })
            })
            .await
            .map_err(|e| AppError::ScraperError(e))
    }

    pub async fn get_anime_detail(
        &self,
        app_state: Arc<AppState>,
        slug: String,
    ) -> Result<DetailResponse, AppError> {
        let cache_key = format!("anime:detail:{}", slug);
        let cache = Cache::new(&app_state.redis_pool);

        cache
            .get_or_set(&cache_key, DEFAULT_CACHE_TTL, || async {
                let mut data = self
                    .repository
                    .fetch_anime_detail(&slug)
                    .await
                    .map_err(|e| e.to_string())?;

                data.poster = crate::shared::services::images::cache::get_cached_or_original(
                    app_state.db.clone(),
                    &app_state.redis_pool,
                    &data.poster,
                    Some(app_state.image_processing_semaphore.clone()),
                )
                .await;

                let rec_posters: Vec<String> = data
                    .recommendations
                    .iter()
                    .map(|r| r.poster.clone())
                    .collect();
                let cached_rec_posters =
                    crate::shared::services::images::cache::cache_image_urls_batch_lazy(
                        app_state.db.clone(),
                        &app_state.redis_pool,
                        rec_posters,
                        Some(app_state.image_processing_semaphore.clone()),
                    )
                    .await;

                for (i, rec) in data.recommendations.iter_mut().enumerate() {
                    if let Some(url) = cached_rec_posters.get(i) {
                        rec.poster = url.clone();
                    }
                }

                Ok(DetailResponse {
                    status: Some("Ok".to_string()),
                    data,
                })
            })
            .await
            .map_err(|e| AppError::ScraperError(e))
    }

    pub async fn get_complete_anime_page(
        &self,
        app_state: Arc<AppState>,
        slug: String,
    ) -> Result<ListResponse, AppError> {
        let cache_key = format!("anime:complete:{}", slug);
        let cache = Cache::new(&app_state.redis_pool);

        cache
            .get_or_set(&cache_key, DEFAULT_CACHE_TTL, || async {
                let (anime_list, pagination) = self
                    .repository
                    .fetch_complete_anime_page(&slug)
                    .await
                    .map_err(|e| e.to_string())?;
                let total = anime_list.len() as i64;
                Ok(ListResponse {
                    message: "Success".to_string(),
                    data: anime_list,
                    total: Some(total),
                    pagination: Some(pagination),
                })
            })
            .await
            .map_err(|e| AppError::ScraperError(e))
    }

    pub async fn get_ongoing_anime_page(
        &self,
        app_state: Arc<AppState>,
        slug: String,
    ) -> Result<OngoingAnimeResponse, AppError> {
        let cache_key = format!("anime:ongoing:{}", slug);
        let cache = Cache::new(&app_state.redis_pool);

        cache
            .get_or_set(&cache_key, DEFAULT_CACHE_TTL, || async {
                let (anime_list, pagination) = self
                    .repository
                    .fetch_ongoing_anime_page(&slug)
                    .await
                    .map_err(|e| e.to_string())?;
                Ok(OngoingAnimeResponse {
                    status: "Ok".to_string(),
                    data: anime_list,
                    pagination,
                })
            })
            .await
            .map_err(|e| AppError::ScraperError(e))
    }

    pub async fn get_latest_anime_page(
        &self,
        app_state: Arc<AppState>,
        slug: String,
    ) -> Result<LatestAnimeResponse, AppError> {
        let cache_key = format!("anime:latest:{}", slug);
        let cache = Cache::new(&app_state.redis_pool);

        cache
            .get_or_set(&cache_key, DEFAULT_CACHE_TTL, || async {
                let (anime_list, pagination) = self
                    .repository
                    .fetch_latest_anime_page(&slug)
                    .await
                    .map_err(|e| e.to_string())?;
                Ok(LatestAnimeResponse {
                    status: "Ok".to_string(),
                    data: anime_list,
                    pagination,
                })
            })
            .await
            .map_err(|e| AppError::ScraperError(e))
    }

    pub async fn get_search_anime_page(
        &self,
        app_state: Arc<AppState>,
        slug: String,
        page: String,
    ) -> Result<SearchResponse, AppError> {
        let cache_key = format!("anime:search:{}:{}", slug, page);
        let cache = Cache::new(&app_state.redis_pool);

        cache
            .get_or_set(&cache_key, DEFAULT_CACHE_TTL, || async {
                let (anime_list, pagination) = self
                    .repository
                    .fetch_search_anime_page(&slug, &page)
                    .await
                    .map_err(|e| e.to_string())?;
                Ok(SearchResponse {
                    status: "Ok".to_string(),
                    data: anime_list,
                    pagination,
                })
            })
            .await
            .map_err(|e| AppError::ScraperError(e))
    }

    pub async fn get_genre_anime_page(
        &self,
        app_state: Arc<AppState>,
        genre_slug: String,
        page: String,
    ) -> Result<GenreListResponse, AppError> {
        let cache_key = format!("anime:genre:{}:{}", genre_slug, page);
        let cache = Cache::new(&app_state.redis_pool);

        cache
            .get_or_set(&cache_key, DEFAULT_CACHE_TTL, || async {
                let (anime_list, pagination) = self
                    .repository
                    .fetch_genre_anime_page(&genre_slug, &page)
                    .await
                    .map_err(|e| e.to_string())?;
                Ok(GenreListResponse {
                    status: "Ok".to_string(),
                    data: anime_list,
                    pagination,
                })
            })
            .await
            .map_err(|e| AppError::ScraperError(e))
    }

    pub async fn get_anime_full(
        &self,
        app_state: Arc<AppState>,
        slug: String,
    ) -> Result<FullResponse, AppError> {
        let cache_key = format!("anime:full:{}", slug);
        let cache = Cache::new(&app_state.redis_pool);

        cache
            .get_or_set(&cache_key, DEFAULT_CACHE_TTL, || async {
                let data = self
                    .repository
                    .fetch_anime_full(&slug)
                    .await
                    .map_err(|e| e.to_string())?;
                Ok(FullResponse {
                    status: "Ok".to_string(),
                    data,
                })
            })
            .await
            .map_err(|e| AppError::ScraperError(e))
    }
}
