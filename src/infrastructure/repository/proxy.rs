//! Proxy fetching repository.

use async_trait::async_trait;

use crate::domain::error::ScrapingError;
use crate::domain::repository::ScrapingRepository;
use crate::infrastructure::scraping::proxy_fetch::{self, FetchResult};

pub struct ProxyRepository;

impl ProxyRepository {
    pub fn new() -> Self {
        Self
    }

    pub async fn fetch_with_proxy_url(&self, url: &str) -> Result<FetchResult, ScrapingError> {
        proxy_fetch::fetch_with_proxy(url)
            .await
            .map_err(|e| ScrapingError::Http(format!("Proxy fetch failed: {}", e)))
    }
}

#[async_trait]
impl ScrapingRepository for ProxyRepository {
    async fn fetch_html(&self, url: &str) -> Result<String, ScrapingError> {
        self.fetch_with_proxy_url(url).await.map(|r| r.data)
    }
}
