use crate::modules::proxy::repository::ProxyRepository;
use crate::modules::proxy::schema::{AuditImageCacheRequest, ImageCacheRequest, ProxyParams};
use crate::modules::proxy::types::{AuditImageCacheResponse, ImageCacheResponse};
use crate::shared::config::CONFIG;
use crate::shared::database::traits::image_cache::ImageCacheRepository;
use crate::shared::errors::AppError;
use crate::shared::events::bus::ImageRepaired;
use crate::shared::services::images::cache::ImageCache;
use crate::shared::state::AppState;
use axum::http::StatusCode;
use axum::response::Response;
use std::sync::Arc;
use tracing::{error, info, warn};

pub struct ProxyService {
    repository: ProxyRepository,
    image_cache_repo: Arc<dyn ImageCacheRepository>,
}

impl ProxyService {
    pub fn new(
        repository: ProxyRepository,
        image_cache_repo: Arc<dyn ImageCacheRepository>,
    ) -> Self {
        Self {
            repository,
            image_cache_repo,
        }
    }

    fn build_image_cache(&self) -> ImageCache {
        ImageCache::new(self.image_cache_repo.clone())
    }

    pub async fn fetch_with_proxy_only(&self, params: ProxyParams) -> Result<Response, AppError> {
        let url = params.url;
        match self.repository.fetch_with_proxy_url(&url).await {
            Ok(fetch_result) => {
                let mut builder = Response::builder().status(StatusCode::OK);
                if let Some(content_type) = fetch_result.content_type {
                    builder = builder.header("Content-Type", content_type);
                }
                Ok(builder.body(fetch_result.data.into())?)
            }
            Err(e) => {
                error!("Proxy fetch error: {:?}", e);
                Err(AppError::Other(format!(
                    "Failed to fetch URL via proxy: {}",
                    e
                )))
            }
        }
    }

    pub async fn image_cache(
        &self,
        state: Arc<AppState>,
        req: ImageCacheRequest,
    ) -> Result<ImageCacheResponse, AppError> {
        let cache = self
            .build_image_cache()
            .with_semaphore(state.image_processing_semaphore.clone());

        if let Some(cdn_url) = cache.get_cdn_url(&req.url).await {
            return Ok(ImageCacheResponse {
                success: true,
                original_url: req.url,
                cdn_url,
                from_cache: true,
                pending: None,
            });
        }

        if req.lazy {
            let url = req.url.clone();
            let repo = self.image_cache_repo.clone();
            let semaphore = state.image_processing_semaphore.clone();
            tokio::spawn(async move {
                let cache = ImageCache::new(repo).with_semaphore(semaphore);
                match cache.get_or_cache(&url).await {
                    Ok(cdn) => info!("[LazyCache] Cached {} -> {}", url, cdn),
                    Err(e) => warn!("[LazyCache] Failed {}: {}", url, e),
                }
            });
            return Ok(ImageCacheResponse {
                success: true,
                original_url: req.url.clone(),
                cdn_url: req.url,
                from_cache: false,
                pending: Some(true),
            });
        }

        match cache.get_or_cache(&req.url).await {
            Ok(cdn_url) => Ok(ImageCacheResponse {
                success: true,
                original_url: req.url,
                cdn_url,
                from_cache: false,
                pending: None,
            }),
            Err(e) => {
                error!("ImageCache error: {}", e);
                Ok(ImageCacheResponse {
                    success: false,
                    original_url: req.url.clone(),
                    cdn_url: req.url,
                    from_cache: false,
                    pending: None,
                })
            }
        }
    }

    pub async fn audit_image_cache(
        &self,
        state: Arc<AppState>,
        req: AuditImageCacheRequest,
    ) -> Result<AuditImageCacheResponse, AppError> {
        let cache = self.build_image_cache();

        let mut cdn_opt = cache.get_cdn_url(&req.url).await;
        let mut original = req.url.clone();
        if cdn_opt.is_none() {
            if let Some(orig) = cache.find_original_from_cdn(&req.url).await {
                info!(
                    "SmartAudit: {} recognized as CDN, original {}",
                    req.url, orig
                );
                original = orig;
                cdn_opt = Some(req.url.clone());
            }
        }

        if let Some(cdn_url) = cdn_opt {
            let client = crate::shared::utils::web::http_client::http_client().client();
            let mut accessible = false;
            match client.get(cdn_url.clone()).send().await {
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
                return Ok(AuditImageCacheResponse {
                    success: true,
                    original_url: original,
                    cdn_url: Some(cdn_url),
                    was_accessible: true,
                    re_uploaded: false,
                    message: "CDN URL is accessible and the image is valid".to_string(),
                });
            }
            info!("CDN {} inaccessible, purging and reuploading", cdn_url);
            let picser_delete_url = &CONFIG.urls.picser_api_url;
            if let Some(filename) = cdn_url.split('/').last() {
                let payload = serde_json::json!({ "filename": filename });
                match client.delete(picser_delete_url).json(&payload).send().await {
                    Ok(r) if r.status().is_success() => info!("Deleted {} via Picser", filename),
                    Ok(r) => warn!("Picser delete {} status {}", filename, r.status()),
                    Err(e) => warn!("Picser delete error {}: {}", filename, e),
                }
            }
            let _ = cache.invalidate(&original).await;
            match cache.get_or_cache(&original).await {
                Ok(new_cdn) => {
                    state
                        .event_bus
                        .publish(ImageRepaired {
                            original_url: original.clone(),
                            cdn_url: new_cdn.clone(),
                        })
                        .await;
                    Ok(AuditImageCacheResponse {
                        success: true,
                        original_url: original,
                        cdn_url: Some(new_cdn),
                        was_accessible: false,
                        re_uploaded: true,
                        message: "CDN URL was inaccessible, re-uploaded".to_string(),
                    })
                }
                Err(e) => Ok(AuditImageCacheResponse {
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
                Ok(new_cdn) => {
                    state
                        .event_bus
                        .publish(ImageRepaired {
                            original_url: original.clone(),
                            cdn_url: new_cdn.clone(),
                        })
                        .await;
                    Ok(AuditImageCacheResponse {
                        success: true,
                        original_url: original,
                        cdn_url: Some(new_cdn),
                        was_accessible: false,
                        re_uploaded: true,
                        message: "Cached newly".to_string(),
                    })
                }
                Err(e) => Ok(AuditImageCacheResponse {
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
