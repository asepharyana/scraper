//! Repository trait for scraping HTML from remote sources.

use async_trait::async_trait;

use crate::domain::error::ScrapingError;

/// Trait for repositories that fetch and scrape HTML content.
#[async_trait]
pub trait ScrapingRepository: Send + Sync {
    /// Fetch raw HTML from a URL.
    async fn fetch_html(&self, url: &str) -> Result<String, ScrapingError>;
}
