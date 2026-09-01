//! Application use-cases for weebs (anime/manga info, waifu, whatanime).

use crate::domain::error::ScrapingError;
use crate::infrastructure::repository::weebs as Repo;
use serde_json::Value;

pub async fn anime_info(query: &str) -> Result<Value, ScrapingError> {
    Repo::fetch_anime_info(query)
        .await
        .map_err(ScrapingError::Http)
}

pub async fn manga_info(query: &str) -> Result<Value, ScrapingError> {
    Repo::fetch_manga_info(query)
        .await
        .map_err(ScrapingError::Http)
}

pub async fn whatanime(url: &str) -> Result<Value, ScrapingError> {
    Repo::fetch_whatanime(url)
        .await
        .map_err(ScrapingError::Http)
}

pub async fn sfw_waifu(tag: &str) -> Result<Value, ScrapingError> {
    Repo::fetch_sfw_waifu(tag)
        .await
        .map_err(ScrapingError::Http)
}

pub async fn nsfw_waifu(tag: &str) -> Result<Value, ScrapingError> {
    Repo::fetch_nsfw_waifu(tag)
        .await
        .map_err(ScrapingError::Http)
}
