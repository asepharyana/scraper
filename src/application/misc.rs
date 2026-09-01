//! Application use-cases for misc utilities.

use crate::domain::error::ScrapingError;
use crate::infrastructure::repository::misc as Repo;
use serde_json::Value;

/// Currency converter.
pub async fn currency_converter(amount: f64, from: &str, to: &str) -> Result<Value, ScrapingError> {
    Repo::fetch_currency_converter(amount, from, to)
        .await
        .map_err(ScrapingError::Http)
}

/// Harga emas Antam.
pub async fn harga_emas() -> Result<Value, ScrapingError> {
    Repo::fetch_harga_emas().await.map_err(ScrapingError::Http)
}

/// Kurs BCA (jual/beli).
pub async fn kurs_bca() -> Result<Value, ScrapingError> {
    Repo::fetch_kurs_bca().await.map_err(ScrapingError::Http)
}

/// Server info (OS/CPU/RAM/disk).
pub async fn server_info() -> Result<Value, ScrapingError> {
    Repo::fetch_server_info().await.map_err(ScrapingError::Http)
}
