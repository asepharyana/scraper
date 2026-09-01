//! Application use-cases for image generation.

use crate::domain::error::ScrapingError;
use crate::infrastructure::repository::image as Repo;

pub async fn brat(text: &str) -> Result<Vec<u8>, ScrapingError> {
    Repo::fetch_brat(text).await.map_err(ScrapingError::Http)
}

pub async fn brat_animated(text: &str) -> Result<Vec<u8>, ScrapingError> {
    Repo::fetch_brat_animated(text)
        .await
        .map_err(ScrapingError::Http)
}
