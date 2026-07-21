//! Proxy application use cases.
//!
//! Provides proxy fetch, image caching, and audit/repair operations.
//!
//! TODO: Move result types to `crate::presentation::dto::proxy`.
//! TODO: Add event bus integration for ImageRepaired events.
//! TODO: Replace `reqwest::Client::new()` with shared HTTP client from infrastructure.

use std::sync::Arc;

use axum::http::StatusCode;
use axum::response::Response;
use serde::Serialize;
use tracing::{error, info, warn};
use utoipa::ToSchema;

use crate::domain::error::*;
use crate::domain::repository::ImageCacheRepository;
use crate::infrastructure::repository::{ProxyRepository, SeaOrmImageCacheRepository};
use crate::infrastructure::services::images::cache::ImageCache;

// ============================================================================
// Result types — TEMPORARY: move to presentation layer
// ============================================================================

/// Result of caching an image URL.
/// TODO: Move to presentation::dto::proxy
#[derive(Debug, Serialize, ToSchema)]
pub struct ImageCacheResult {
    pub success: bool,
    pub original_url: String,
    pub cdn_url: String,
    pub from_cache: bool,
    pub pending: Option<bool>,
}

/// Result of auditing/repairing a cached image URL.
/// TODO: Move to presentation::dto::proxy
#[derive(Debug, Serialize, ToSchema)]
pub struct AuditImageCacheResult {
    pub success: bool,
    pub original_url: String,
    pub cdn_url: Option<String>,
    pub was_accessible: bool,
    pub re_uploaded: bool,
    pub message: String,
}

// ============================================================================
// Use case struct
// ============================================================================

pub struct ProxyUseCases {
    repository: ProxyRepository,
    image_cache_repo: Arc<dyn ImageCacheRepository>,
}

impl ProxyUseCases {
    pub fn new(
        repository: ProxyRepository,
        image_cache_repo: Arc<SeaOrmImageCacheRepository>,
    ) -> Self {
        Self {
            repository,
            image_cache_repo,
        }
    }

    fn build_image_cache(&self) -> ImageCache {
        ImageCache::new(self.image_cache_repo.clone())
    }

    pub async fn fetch_with_proxy_only(&self, url: String) -> Result<Response, DomainError> {
        let fetch_result = self
            .repository
            .fetch_with_proxy_url(&url)
            .await
            .map_err(|e| DomainError::Scraping(ScrapingError::Http(e.to_string())))?;

        let mut builder = Response::builder().status(StatusCode::OK);
        if let Some(content_type) = fetch_result.content_type {
            builder = builder.header("Content-Type", content_type);
        }

        builder
            .body(fetch_result.data.into())
            .map_err(|e| DomainError::Repository(RepositoryError::Network(e.to_string())))
    }

    pub async fn image_cache(
        &self,
        url: String,
        lazy: bool,
    ) -> Result<ImageCacheResult, DomainError> {
        let cache = self.build_image_cache();

        if let Some(cdn_url) = cache.get_cdn_url(&url).await {
            return Ok(ImageCacheResult {
                success: true,
                original_url: url,
                cdn_url,
                from_cache: true,
                pending: None,
            });
        }

        if lazy {
            let repo = self.image_cache_repo.clone();
            let url_clone = url.clone();
            tokio::spawn(async move {
                let cache = ImageCache::new(repo);
                match cache.get_or_cache(&url_clone).await {
                    Ok(cdn) => info!("[LazyCache] Cached {} -> {}", url_clone, cdn),
                    Err(e) => warn!("[LazyCache] Failed {}: {}", url_clone, e),
                }
            });
            return Ok(ImageCacheResult {
                success: true,
                original_url: url.clone(),
                cdn_url: url,
                from_cache: false,
                pending: Some(true),
            });
        }

        match cache.get_or_cache(&url).await {
            Ok(cdn_url) => Ok(ImageCacheResult {
                success: true,
                original_url: url,
                cdn_url,
                from_cache: false,
                pending: None,
            }),
            Err(e) => {
                error!("ImageCache error: {}", e);
                Ok(ImageCacheResult {
                    success: false,
                    original_url: url.clone(),
                    cdn_url: url,
                    from_cache: false,
                    pending: None,
                })
            }
        }
    }

    pub async fn audit_image_cache(
        &self,
        url: String,
    ) -> Result<AuditImageCacheResult, DomainError> {
        let cache = self.build_image_cache();
        let mut cdn_opt = cache.get_cdn_url(&url).await;
        let mut original = url.clone();

        if cdn_opt.is_none() {
            if let Some(orig) = cache.find_original_from_cdn(&url).await {
                info!("SmartAudit: {} recognized as CDN, original {}", url, orig);
                original = orig;
                cdn_opt = Some(url.clone());
            }
        }

        if let Some(cdn_url) = cdn_opt {
            let client = reqwest::Client::new();
            let mut accessible = false;

            match client.get(&cdn_url).send().await {
                Ok(resp) if resp.status().is_success() => {
                    if let Ok(bytes) = resp.bytes().await {
                        if infer::get(&bytes)
                            .map(|k| k.mime_type().starts_with("image/"))
                            .unwrap_or(false)
                        {
                            accessible = true;
                        } else {
                            warn!("CDN {} returned non-image content", cdn_url);
                        }
                    }
                }
                Ok(resp) => warn!("CDN {} status {}", cdn_url, resp.status()),
                Err(e) => warn!("CDN {} fetch error {}", cdn_url, e),
            }

            if accessible {
                return Ok(AuditImageCacheResult {
                    success: true,
                    original_url: original,
                    cdn_url: Some(cdn_url),
                    was_accessible: true,
                    re_uploaded: false,
                    message: "CDN URL is accessible and the image is valid".to_string(),
                });
            }

            info!("CDN {} inaccessible, purging and reuploading", cdn_url);
            let _ = cache.invalidate(&original).await;
            match cache.get_or_cache(&original).await {
                Ok(new_cdn) => Ok(AuditImageCacheResult {
                    success: true,
                    original_url: original,
                    cdn_url: Some(new_cdn),
                    was_accessible: false,
                    re_uploaded: true,
                    message: "CDN URL was inaccessible, re-uploaded".to_string(),
                }),
                Err(e) => Ok(AuditImageCacheResult {
                    success: false,
                    original_url: original,
                    cdn_url: None,
                    was_accessible: false,
                    re_uploaded: false,
                    message: format!("Re-upload failed: {}", e),
                }),
            }
        } else {
            match cache.get_or_cache(&original).await {
                Ok(new_cdn) => Ok(AuditImageCacheResult {
                    success: true,
                    original_url: original,
                    cdn_url: Some(new_cdn),
                    was_accessible: false,
                    re_uploaded: true,
                    message: "Cached newly".to_string(),
                }),
                Err(e) => Ok(AuditImageCacheResult {
                    success: false,
                    original_url: original,
                    cdn_url: None,
                    was_accessible: false,
                    re_uploaded: false,
                    message: format!("Cache failed: {}", e),
                }),
            }
        }
    }
}
