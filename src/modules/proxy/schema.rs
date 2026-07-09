use serde::Deserialize;
use utoipa::IntoParams;
use utoipa::ToSchema;

/// Query parameters for proxy fetch (GET)
#[derive(Debug, Deserialize, ToSchema, IntoParams)]
pub struct ProxyParams {
    /// URL to fetch via proxy
    pub url: String,
}

/// Request body for image cache (POST)
#[derive(Debug, Deserialize, ToSchema)]
pub struct ImageCacheRequest {
    /// Original image URL to cache
    pub url: String,
    /// If true, returns original URL immediately and caches in background
    #[serde(default)]
    pub lazy: bool,
}

/// Request body for auditing image cache (POST)
#[derive(Debug, Deserialize, ToSchema)]
pub struct AuditImageCacheRequest {
    /// Original image URL to audit
    pub url: String,
}
