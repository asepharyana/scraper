use std::sync::Arc;

use crate::modules::komik::parser;
use crate::modules::komik::repository::KomikRepository;
use crate::modules::komik::types::{
    ChapterResponse, DetailResponse, GenreKomikResponse, GenresResponse, SearchKomikResponse,
};
use crate::shared::database::traits::scraping_repository::ScrapingRepository;
use crate::shared::errors::AppError;
use crate::shared::services::images::cache::{
    apply_cached_posters, cache_image_urls_batch_lazy, get_cached_or_original,
};
use crate::shared::state::AppState;
use crate::shared::utils::Cache;

const GENRE_LIST_CACHE_TTL: u64 = 3600;
const GENRE_CACHE_TTL: u64 = 300;
const DETAIL_CACHE_TTL: u64 = 300;
const CHAPTER_CACHE_TTL: u64 = 300;
const SEARCH_CACHE_TTL: u64 = 300;

pub struct KomikService {
    repository: KomikRepository,
}

impl KomikService {
    pub fn new(repository: KomikRepository) -> Self {
        Self { repository }
    }

    pub async fn genre_list(&self, app_state: Arc<AppState>) -> Result<GenresResponse, AppError> {
        let cache = Cache::new(&app_state.redis_pool);
        let cache_key = "komik:genres:list:v3";

        cache
            .get_or_set(cache_key, GENRE_LIST_CACHE_TTL, || async {
                let html = self
                    .repository
                    .fetch_html(&self.repository.api_url())
                    .await
                    .map_err(|e| e.to_string())?;
                let genres = tokio::task::spawn_blocking(move || parser::parse_genres(&html))
                    .await
                    .map_err(|e| e.to_string())?
                    .map_err(|e| e.to_string())?;

                Ok(GenresResponse {
                    status: "Ok".to_string(),
                    data: genres,
                })
            })
            .await
            .map_err(AppError::ScraperError)
    }

    pub async fn genre_slug(
        &self,
        genre_slug: String,
        app_state: Arc<AppState>,
    ) -> Result<GenreKomikResponse, AppError> {
        let page = 1;
        let cache = Cache::new(&app_state.redis_pool);
        let cache_key = format!("komik:genre:{}:{}:v2", genre_slug, page);

        cache
            .get_or_set(&cache_key, GENRE_CACHE_TTL, || async {
                let url = self.repository.genre_url(&genre_slug, page);
                let html = self
                    .repository
                    .fetch_html(&url)
                    .await
                    .map_err(|e| e.to_string())?;

                let (mut komik_list, pagination) =
                    tokio::task::spawn_blocking(move || parser::parse_genre_page(&html, page))
                        .await
                        .map_err(|e| e.to_string())?
                        .map_err(|e| e.to_string())?;

                apply_cached_posters(
                    &mut komik_list,
                    app_state.db.clone(),
                    &app_state.redis_pool,
                    Some(app_state.image_processing_semaphore.clone()),
                )
                .await;

                Ok(GenreKomikResponse {
                    status: "Ok".to_string(),
                    genre: genre_slug.clone(),
                    data: komik_list,
                    pagination,
                })
            })
            .await
            .map_err(AppError::ScraperError)
    }

    pub async fn genre_slug_page(
        &self,
        genre_slug: String,
        page: u32,
        app_state: Arc<AppState>,
    ) -> Result<GenreKomikResponse, AppError> {
        let cache = Cache::new(&app_state.redis_pool);
        let cache_key = format!("komik:genre:{}:{}:v2", genre_slug, page);

        cache
            .get_or_set(&cache_key, GENRE_CACHE_TTL, || async {
                let url = self.repository.genre_url(&genre_slug, page);
                let html = self
                    .repository
                    .fetch_html(&url)
                    .await
                    .map_err(|e| e.to_string())?;

                let (mut komik_list, pagination) =
                    tokio::task::spawn_blocking(move || parser::parse_genre_page(&html, page))
                        .await
                        .map_err(|e| e.to_string())?
                        .map_err(|e| e.to_string())?;

                apply_cached_posters(
                    &mut komik_list,
                    app_state.db.clone(),
                    &app_state.redis_pool,
                    Some(app_state.image_processing_semaphore.clone()),
                )
                .await;

                Ok(GenreKomikResponse {
                    status: "Ok".to_string(),
                    genre: genre_slug.clone(),
                    data: komik_list,
                    pagination,
                })
            })
            .await
            .map_err(AppError::ScraperError)
    }

    pub async fn detail_slug(
        &self,
        komik_id: String,
        app_state: Arc<AppState>,
    ) -> Result<DetailResponse, AppError> {
        let cache = Cache::new(&app_state.redis_pool);
        let cache_key = format!("komik:detail:{}", komik_id);

        cache
            .get_or_set(&cache_key, DETAIL_CACHE_TTL, || async {
                let url = self.repository.detail_url(&komik_id);
                let html = self
                    .repository
                    .fetch_html(&url)
                    .await
                    .map_err(|e| e.to_string())?;

                let mut data =
                    tokio::task::spawn_blocking(move || parser::parse_komik_detail_document(&html))
                        .await
                        .map_err(|e| e.to_string())?
                        .map_err(|e| e.to_string())?;

                if !data.poster.is_empty() {
                    data.poster = get_cached_or_original(
                        app_state.db.clone(),
                        &app_state.redis_pool,
                        &data.poster,
                        Some(app_state.image_processing_semaphore.clone()),
                    )
                    .await;
                }

                Ok(DetailResponse { status: true, data })
            })
            .await
            .map_err(AppError::ScraperError)
    }

    pub async fn chapter_slug(
        &self,
        chapter_url: String,
        app_state: Arc<AppState>,
    ) -> Result<ChapterResponse, AppError> {
        let cache = Cache::new(&app_state.redis_pool);
        let cache_key = format!("komik:chapter:{}", chapter_url);

        cache
            .get_or_set(&cache_key, CHAPTER_CACHE_TTL, || async {
                let url = self.repository.chapter_url(&chapter_url);
                let html = self
                    .repository
                    .fetch_html(&url)
                    .await
                    .map_err(|e| e.to_string())?;

                let mut data = tokio::task::spawn_blocking({
                    let chapter_url = chapter_url.clone();
                    move || parser::parse_komik_chapter_document(&html, &chapter_url)
                })
                .await
                .map_err(|e| e.to_string())?
                .map_err(|e| e.to_string())?;

                data.images = cache_image_urls_batch_lazy(
                    app_state.db.clone(),
                    &app_state.redis_pool,
                    data.images,
                    Some(app_state.image_processing_semaphore.clone()),
                )
                .await;

                Ok(ChapterResponse {
                    message: "Ok".to_string(),
                    data,
                })
            })
            .await
            .map_err(AppError::ScraperError)
    }

    pub async fn manga_slug(
        &self,
        page_slug: String,
        app_state: Arc<AppState>,
    ) -> Result<GenreKomikResponse, AppError> {
        let page = page_slug
            .parse::<u32>()
            .map_err(|_| AppError::ScraperError("Invalid page number".to_string()))?;
        self.list_by_url(
            "manga",
            page,
            self.repository.manga_list_url(page),
            app_state,
        )
        .await
    }

    pub async fn manhua_slug(
        &self,
        page_slug: String,
        app_state: Arc<AppState>,
    ) -> Result<GenreKomikResponse, AppError> {
        let page = page_slug
            .parse::<u32>()
            .map_err(|_| AppError::ScraperError("Invalid page number".to_string()))?;
        self.list_by_url(
            "manhua",
            page,
            self.repository.manhua_list_url(page),
            app_state,
        )
        .await
    }

    pub async fn manhwa_slug(
        &self,
        page_slug: String,
        app_state: Arc<AppState>,
    ) -> Result<GenreKomikResponse, AppError> {
        let page = page_slug
            .parse::<u32>()
            .map_err(|_| AppError::ScraperError("Invalid page number".to_string()))?;
        self.list_by_url(
            "manhwa",
            page,
            self.repository.manhwa_list_url(page),
            app_state,
        )
        .await
    }

    pub async fn popular_slug(
        &self,
        page_slug: String,
        app_state: Arc<AppState>,
    ) -> Result<GenreKomikResponse, AppError> {
        let page = page_slug
            .parse::<u32>()
            .map_err(|_| AppError::ScraperError("Invalid page number".to_string()))?;
        self.list_by_url(
            "popular",
            page,
            self.repository.popular_list_url(page),
            app_state,
        )
        .await
    }

    async fn list_by_url(
        &self,
        list_name: &str,
        page: u32,
        url: String,
        app_state: Arc<AppState>,
    ) -> Result<GenreKomikResponse, AppError> {
        let cache = Cache::new(&app_state.redis_pool);
        let cache_key = format!("komik:list:{}:{}:v2", list_name, page);

        cache
            .get_or_set(&cache_key, GENRE_CACHE_TTL, || async {
                let html = self
                    .repository
                    .fetch_html(&url)
                    .await
                    .map_err(|e| e.to_string())?;

                let (mut komik_list, pagination) =
                    tokio::task::spawn_blocking(move || parser::parse_genre_page(&html, page))
                        .await
                        .map_err(|e| e.to_string())?
                        .map_err(|e| e.to_string())?;

                if komik_list.is_empty() {
                    return Err(format!("Empty komik {} page {}", list_name, page));
                }

                apply_cached_posters(
                    &mut komik_list,
                    app_state.db.clone(),
                    &app_state.redis_pool,
                    Some(app_state.image_processing_semaphore.clone()),
                )
                .await;

                Ok(GenreKomikResponse {
                    status: "Ok".to_string(),
                    genre: list_name.to_string(),
                    data: komik_list,
                    pagination,
                })
            })
            .await
            .map_err(AppError::ScraperError)
    }

    pub async fn search_slug(
        &self,
        query: String,
        app_state: Arc<AppState>,
    ) -> Result<SearchKomikResponse, AppError> {
        let page = 1;
        let cache = Cache::new(&app_state.redis_pool);
        let cache_key = format!("komik:search:{}:{}", query, page);

        cache
            .get_or_set(&cache_key, SEARCH_CACHE_TTL, || async {
                let url = self.repository.search_url(&query, page);
                let html = self
                    .repository
                    .fetch_html(&url)
                    .await
                    .map_err(|e| e.to_string())?;

                let (mut komik_list, pagination) =
                    tokio::task::spawn_blocking(move || parser::parse_genre_page(&html, page))
                        .await
                        .map_err(|e| e.to_string())?
                        .map_err(|e| e.to_string())?;

                apply_cached_posters(
                    &mut komik_list,
                    app_state.db.clone(),
                    &app_state.redis_pool,
                    Some(app_state.image_processing_semaphore.clone()),
                )
                .await;

                Ok(SearchKomikResponse {
                    status: "Ok".to_string(),
                    data: komik_list,
                    pagination,
                })
            })
            .await
            .map_err(AppError::ScraperError)
    }

    pub async fn search_slug_page(
        &self,
        query: String,
        page: u32,
        app_state: Arc<AppState>,
    ) -> Result<SearchKomikResponse, AppError> {
        let cache = Cache::new(&app_state.redis_pool);
        let cache_key = format!("komik:search:{}:{}", query, page);

        cache
            .get_or_set(&cache_key, SEARCH_CACHE_TTL, || async {
                let url = self.repository.search_url(&query, page);
                let html = self
                    .repository
                    .fetch_html(&url)
                    .await
                    .map_err(|e| e.to_string())?;

                let (mut komik_list, pagination) =
                    tokio::task::spawn_blocking(move || parser::parse_genre_page(&html, page))
                        .await
                        .map_err(|e| e.to_string())?
                        .map_err(|e| e.to_string())?;

                apply_cached_posters(
                    &mut komik_list,
                    app_state.db.clone(),
                    &app_state.redis_pool,
                    Some(app_state.image_processing_semaphore.clone()),
                )
                .await;

                Ok(SearchKomikResponse {
                    status: "Ok".to_string(),
                    data: komik_list,
                    pagination,
                })
            })
            .await
            .map_err(AppError::ScraperError)
    }
}
