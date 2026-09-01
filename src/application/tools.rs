//! Application use-cases for tools.

use crate::domain::error::ScrapingError;
use crate::infrastructure::repository::tools as Repo;
use serde_json::Value;

pub async fn whois(domain: &str) -> Result<Value, ScrapingError> {
    Repo::fetch_whois(domain).await.map_err(ScrapingError::Http)
}

pub async fn ip_location(ip: &str) -> Result<Value, ScrapingError> {
    Repo::fetch_ip_location(ip)
        .await
        .map_err(ScrapingError::Http)
}

pub async fn tinyurl(url: &str) -> Result<Value, ScrapingError> {
    Repo::fetch_tinyurl(url).await.map_err(ScrapingError::Http)
}

pub async fn check_hosting(domain: &str) -> Result<Value, ScrapingError> {
    Repo::fetch_check_hosting(domain)
        .await
        .map_err(ScrapingError::Http)
}

pub async fn hargapangan() -> Result<Value, ScrapingError> {
    Repo::fetch_hargapangan().await.map_err(ScrapingError::Http)
}

pub async fn cek_resi(resi: String, ekspedisi: Option<String>) -> Result<Value, ScrapingError> {
    Repo::fetch_cek_resi(&resi, ekspedisi.as_deref())
        .await
        .map_err(ScrapingError::Http)
}
