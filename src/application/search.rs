//! Application use-cases for search utilities.

use crate::domain::error::ScrapingError;
use crate::infrastructure::repository::search as Repo;
use serde_json::Value;

pub async fn bmkg() -> Result<Value, ScrapingError> {
    Repo::fetch_bmkg().await.map_err(ScrapingError::Http)
}

pub async fn jadwal_sholat(kota: &str) -> Result<Value, ScrapingError> {
    Repo::fetch_jadwal_sholat(kota)
        .await
        .map_err(ScrapingError::Http)
}

pub async fn weather(city: &str) -> Result<Value, ScrapingError> {
    Repo::fetch_weather(city).await.map_err(ScrapingError::Http)
}

pub async fn google(query: &str) -> Result<Value, ScrapingError> {
    Repo::fetch_google(query).await.map_err(ScrapingError::Http)
}

pub async fn yt_search(query: &str) -> Result<Value, ScrapingError> {
    Repo::fetch_yt_search(query)
        .await
        .map_err(ScrapingError::Http)
}
