//! HTTP handlers for media download endpoints.
//!
//! Each handler wraps an application-layer use case, parses request params,
//! and returns a JSON response. Platform is auto-detected from the URL
//! via the all-in-one dispatcher.

use axum::extract::Query;
use axum::Json;
use serde::Deserialize;

use crate::application::downloader as use_cases;
use crate::presentation::dto::downloader::DownloadResponse;
use crate::presentation::error::AppError;

// ============================================================================
// DownloadParams + response
// ============================================================================

/// Request params for downloader endpoints.
#[derive(Debug, Deserialize)]
pub struct DownloadParams {
    pub url: String,
    pub cookies: Option<String>,
    pub api_key: Option<String>,
    pub quality: Option<String>,
}

// ============================================================================
// Handlers
// ============================================================================

/// All-in-one downloader with auto-detection.
pub async fn download(
    Query(params): Query<DownloadParams>,
) -> Result<Json<DownloadResponse>, AppError> {
    let result = use_cases::download_all_in_one(&params.url, params.cookies.as_deref()).await?;
    Ok(Json(DownloadResponse::ok(result)))
}

/// Platform detection endpoint.
pub async fn detect_platform_handler(
    Query(params): Query<DownloadParams>,
) -> Result<Json<serde_json::Value>, AppError> {
    let platform = use_cases::detect_platform(&params.url);
    Ok(Json(serde_json::json!({
        "platform": platform,
        "url": params.url,
    })))
}

/// Generate handler for each downloader platform.
/// Handler for _instagram.
pub async fn download_instagram(
    Query(params): Query<DownloadParams>,
) -> Result<Json<DownloadResponse>, AppError> {
    let result = use_cases::download_instagram(&params.url, params.cookies.as_deref()).await?;
    Ok(Json(DownloadResponse::ok(result)))
}

/// Handler for _facebook.
pub async fn download_facebook(
    Query(params): Query<DownloadParams>,
) -> Result<Json<DownloadResponse>, AppError> {
    let result = use_cases::download_facebook(&params.url, params.cookies.as_deref()).await?;
    Ok(Json(DownloadResponse::ok(result)))
}

/// Handler for _tiktok.
pub async fn download_tiktok(
    Query(params): Query<DownloadParams>,
) -> Result<Json<DownloadResponse>, AppError> {
    let result = use_cases::download_tiktok(&params.url, params.cookies.as_deref()).await?;
    Ok(Json(DownloadResponse::ok(result)))
}

/// Handler for _youtube.
pub async fn download_youtube(
    Query(params): Query<DownloadParams>,
) -> Result<Json<DownloadResponse>, AppError> {
    let result =
        use_cases::download_youtube(&params.url, params.quality.as_deref().unwrap_or("720"))
            .await?;
    Ok(Json(DownloadResponse::ok(result)))
}

/// Handler for _youtube_mp3.
pub async fn download_youtube_mp3(
    Query(params): Query<DownloadParams>,
) -> Result<Json<DownloadResponse>, AppError> {
    let result = use_cases::download_youtube_mp3(&params.url).await?;
    Ok(Json(DownloadResponse::ok(result)))
}

/// Handler for _spotify.
pub async fn download_spotify(
    Query(params): Query<DownloadParams>,
) -> Result<Json<DownloadResponse>, AppError> {
    let result = use_cases::download_spotify(&params.url, params.api_key.as_deref()).await?;
    Ok(Json(DownloadResponse::ok(result)))
}

/// Handler for _twitter.
pub async fn download_twitter(
    Query(params): Query<DownloadParams>,
) -> Result<Json<DownloadResponse>, AppError> {
    let result = use_cases::download_twitter(&params.url, params.cookies.as_deref()).await?;
    Ok(Json(DownloadResponse::ok(result)))
}

/// Handler for _pinterest.
pub async fn download_pinterest(
    Query(params): Query<DownloadParams>,
) -> Result<Json<DownloadResponse>, AppError> {
    let result = use_cases::download_pinterest(&params.url).await?;
    Ok(Json(DownloadResponse::ok(result)))
}

/// Handler for _mega.
pub async fn download_mega(
    Query(params): Query<DownloadParams>,
) -> Result<Json<DownloadResponse>, AppError> {
    let result = use_cases::download_mega(&params.url).await?;
    Ok(Json(DownloadResponse::ok(result)))
}

/// Handler for _terabox.
pub async fn download_terabox(
    Query(params): Query<DownloadParams>,
) -> Result<Json<DownloadResponse>, AppError> {
    let result = use_cases::download_terabox(&params.url).await?;
    Ok(Json(DownloadResponse::ok(result)))
}

/// Handler for _gdrive.
pub async fn download_gdrive(
    Query(params): Query<DownloadParams>,
) -> Result<Json<DownloadResponse>, AppError> {
    let result = use_cases::download_gdrive(&params.url).await?;
    Ok(Json(DownloadResponse::ok(result)))
}

/// Handler for _mediafire.
pub async fn download_mediafire(
    Query(params): Query<DownloadParams>,
) -> Result<Json<DownloadResponse>, AppError> {
    let result = use_cases::download_mediafire(&params.url).await?;
    Ok(Json(DownloadResponse::ok(result)))
}

/// Handler for _pixeldrain.
pub async fn download_pixeldrain(
    Query(params): Query<DownloadParams>,
) -> Result<Json<DownloadResponse>, AppError> {
    let result = use_cases::download_pixeldrain(&params.url).await?;
    Ok(Json(DownloadResponse::ok(result)))
}

/// Handler for _threads.
pub async fn download_threads(
    Query(params): Query<DownloadParams>,
) -> Result<Json<DownloadResponse>, AppError> {
    let result = use_cases::download_threads(&params.url, params.cookies.as_deref()).await?;
    Ok(Json(DownloadResponse::ok(result)))
}

/// Handler for _doodstream.
pub async fn download_doodstream(
    Query(params): Query<DownloadParams>,
) -> Result<Json<DownloadResponse>, AppError> {
    let result = use_cases::download_doodstream(&params.url).await?;
    Ok(Json(DownloadResponse::ok(result)))
}

/// Handler for _krakenfiles.
pub async fn download_krakenfiles(
    Query(params): Query<DownloadParams>,
) -> Result<Json<DownloadResponse>, AppError> {
    let result = use_cases::download_krakenfiles(&params.url).await?;
    Ok(Json(DownloadResponse::ok(result)))
}

/// Handler for _danbooru.
pub async fn download_danbooru(
    Query(params): Query<DownloadParams>,
) -> Result<Json<DownloadResponse>, AppError> {
    let result = use_cases::download_danbooru(&params.url).await?;
    Ok(Json(DownloadResponse::ok(result)))
}

/// Handler for _soundcloud.
pub async fn download_soundcloud(
    Query(params): Query<DownloadParams>,
) -> Result<Json<DownloadResponse>, AppError> {
    let result = use_cases::download_soundcloud(&params.url).await?;
    Ok(Json(DownloadResponse::ok(result)))
}

/// Handler for _bilibili.
pub async fn download_bilibili(
    Query(params): Query<DownloadParams>,
) -> Result<Json<DownloadResponse>, AppError> {
    let result = use_cases::download_bilibili(&params.url).await?;
    Ok(Json(DownloadResponse::ok(result)))
}
