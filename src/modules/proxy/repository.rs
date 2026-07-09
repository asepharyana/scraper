use crate::shared::database::traits::scraping_repository::ScrapingRepository;
use crate::shared::errors::AppError;
use crate::shared::utils::web::proxy_fetch::{self, FetchResult};
use async_trait::async_trait;

pub struct ProxyRepository;

impl Default for ProxyRepository {
    fn default() -> Self {
        Self::new()
    }
}

impl ProxyRepository {
    pub fn new() -> Self {
        Self
    }

    pub async fn fetch_with_proxy_url(&self, url: &str) -> Result<FetchResult, AppError> {
        proxy_fetch::fetch_with_proxy(url).await
    }
}

#[async_trait]
impl ScrapingRepository for ProxyRepository {
    async fn fetch_html(&self, url: &str) -> Result<String, AppError> {
        self.fetch_with_proxy_url(url).await.map(|r| r.data)
    }
}
