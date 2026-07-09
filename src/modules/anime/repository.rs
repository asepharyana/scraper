use crate::modules::anime::parser;
use crate::modules::anime::types::*;
use crate::shared::database::traits::scraping_repository::ScrapingRepository;
use crate::shared::errors::AppError;
use crate::shared::utils::web::proxy_fetch::fetch_with_proxy;
use crate::shared::utils::web::scraping_urls::{get_otakudesu_url, OTAKUDESU_BASE_URL};
use crate::shared::utils::{default_backoff, fetch_html_with_retry, transient};
use async_trait::async_trait;
use backoff::future::retry;
use tracing::{info, warn};

pub struct AnimeRepository;

impl Default for AnimeRepository {
    fn default() -> Self {
        Self::new()
    }
}

impl AnimeRepository {
    pub fn new() -> Self {
        Self
    }
}

#[async_trait]
impl ScrapingRepository for AnimeRepository {
    async fn fetch_html(&self, url: &str) -> Result<String, AppError> {
        fetch_html_with_retry(url).await
    }
}

impl AnimeRepository {
    pub fn base_url(&self) -> String {
        get_otakudesu_url()
    }

    pub fn index_urls(&self) -> (String, String) {
        let base = self.base_url();
        (
            format!("{}/ongoing-anime/", base),
            format!("{}/complete-anime/", base),
        )
    }

    pub fn genres_url(&self) -> String {
        format!("{}/genre-list/", self.base_url())
    }

    pub fn detail_url(&self, slug: &str) -> String {
        format!("{}/anime/{}", OTAKUDESU_BASE_URL, slug)
    }

    pub fn page_url(&self, category: &str, page: &str) -> String {
        format!("{}/{}/page/{}/", OTAKUDESU_BASE_URL, category, page)
    }

    pub fn search_url(&self, query: &str, page: &str) -> String {
        if page == "1" {
            format!("{}/search/{}/", self.base_url(), query)
        } else {
            format!("{}/search/{}/page/{}/", self.base_url(), query, page)
        }
    }

    pub fn genre_page_url(&self, genre_slug: &str, page: &str) -> String {
        format!("{}/genre/{}/page/{}/", self.base_url(), genre_slug, page)
    }

    pub fn full_episode_url(&self, slug: &str) -> String {
        format!("{}/episode/{}", OTAKUDESU_BASE_URL, slug)
    }

    pub async fn fetch_anime_index(&self) -> Result<AnimeData, AppError> {
        let (ongoing_url, complete_url) = self.index_urls();

        let (ongoing_html, complete_html) = tokio::join!(
            self.fetch_html(&ongoing_url),
            self.fetch_html(&complete_url)
        );

        let ongoing_html = ongoing_html?;
        let complete_html = complete_html?;

        let ongoing_anime =
            tokio::task::spawn_blocking(move || parser::parse_ongoing_anime(&ongoing_html))
                .await??;
        let complete_anime =
            tokio::task::spawn_blocking(move || parser::parse_complete_anime(&complete_html))
                .await??;

        Ok(AnimeData {
            ongoing_anime,
            complete_anime,
        })
    }

    pub async fn fetch_genres(&self) -> Result<Vec<Genre>, AppError> {
        let html = self.fetch_html(&self.genres_url()).await?;
        tokio::task::spawn_blocking(move || parser::parse_genres(&html)).await?
    }

    pub async fn fetch_anime_detail(&self, slug: &str) -> Result<AnimeDetailData, AppError> {
        let url = self.detail_url(slug);
        let html = self
            .fetch_with_proxy_retry(&url)
            .await
            .map_err(|e| AppError::ScraperError(e.to_string()))?;

        tokio::task::spawn_blocking(move || parser::parse_anime_detail_document(&html)).await?
    }

    pub async fn fetch_complete_anime_page(
        &self,
        slug: &str,
    ) -> Result<(Vec<CompleteAnimeListItem>, Pagination), AppError> {
        let url = self.page_url("complete-anime", slug);
        let html = self.fetch_html(&url).await?;
        let slug_owned = slug.to_string();
        tokio::task::spawn_blocking(move || parser::parse_anime_page(&html, &slug_owned)).await?
    }

    pub async fn fetch_ongoing_anime_page(
        &self,
        slug: &str,
    ) -> Result<(Vec<OngoingAnimeListItem>, Pagination), AppError> {
        let url = self.page_url("ongoing-anime", slug);
        let html = self.fetch_html(&url).await?;
        let slug_owned = slug.to_string();
        tokio::task::spawn_blocking(move || {
            parser::parse_ongoing_anime_document(&html, &slug_owned)
        })
        .await?
    }

    pub async fn fetch_latest_anime_page(
        &self,
        slug: &str,
    ) -> Result<(Vec<LatestAnimeItem>, Pagination), AppError> {
        let url = self.page_url("latest-anime", slug);
        let html = self.fetch_html(&url).await?;
        let slug_owned = slug.to_string();
        tokio::task::spawn_blocking(move || parser::parse_latest_anime_document(&html, &slug_owned))
            .await?
    }

    pub async fn fetch_search_anime_page(
        &self,
        slug: &str,
        page: &str,
    ) -> Result<(Vec<SearchAnimeItem>, Pagination), AppError> {
        let url = self.search_url(slug, page);
        let html = self.fetch_html(&url).await?;
        let page_owned = page.to_string();
        tokio::task::spawn_blocking(move || parser::parse_search_anime_document(&html, &page_owned))
            .await?
    }

    pub async fn fetch_genre_anime_page(
        &self,
        genre_slug: &str,
        page: &str,
    ) -> Result<(Vec<GenreAnimeItem>, Pagination), AppError> {
        let url = self.genre_page_url(genre_slug, page);
        let html = self.fetch_html(&url).await?;
        let page_owned = page.to_string();
        tokio::task::spawn_blocking(move || parser::parse_genre_anime_document(&html, &page_owned))
            .await?
    }

    pub async fn fetch_anime_full(&self, slug: &str) -> Result<AnimeFullData, AppError> {
        let url = self.full_episode_url(slug);
        let html = self.fetch_html(&url).await?;
        let slug_owned = slug.to_string();
        tokio::task::spawn_blocking(move || parser::parse_anime_full_document(&html, &slug_owned))
            .await?
    }

    async fn fetch_with_proxy_retry(&self, url: &str) -> Result<String, AppError> {
        let backoff = default_backoff();
        let url_owned = url.to_string();
        let fetch_op = || async {
            info!("Fetching URL: {}", url_owned);
            match fetch_with_proxy(&url_owned).await {
                Ok(response) => {
                    info!("Successfully fetched URL: {}", url_owned);
                    Ok(response.data)
                }
                Err(e) => {
                    warn!("Failed to fetch URL: {}, error: {:?}", url_owned, e);
                    Err(transient(e))
                }
            }
        };
        retry(backoff, fetch_op)
            .await
            .map_err(|e| AppError::ScraperError(e.to_string()))
    }
}
