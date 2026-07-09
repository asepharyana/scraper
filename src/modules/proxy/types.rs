use serde::Serialize;
use utoipa::ToSchema;

/// Response for image cache POST
#[derive(Debug, Serialize, ToSchema)]
pub struct ImageCacheResponse {
    pub success: bool,
    pub original_url: String,
    pub cdn_url: String,
    pub from_cache: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub pending: Option<bool>,
}

/// Response for image cache audit POST
#[derive(Debug, Serialize, ToSchema)]
pub struct AuditImageCacheResponse {
    pub success: bool,
    pub original_url: String,
    pub cdn_url: Option<String>,
    pub was_accessible: bool,
    pub re_uploaded: bool,
    pub message: String,
}
