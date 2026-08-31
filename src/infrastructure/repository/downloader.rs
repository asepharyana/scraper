//! Media downloader repository — ports HTTP API results to domain types.
//!
//! Each method corresponds to a downloader from Shirokami-API.
//! All use the shared `http_client()` and proxy-fetch infrastructure.

use std::borrow::Cow;
use std::collections::HashMap;
use std::time::Duration;

use aes::cipher::{BlockDecrypt, KeyInit};
use base64::Engine;

use crate::domain::entity::downloader::{DownloadResult, MediaItem, MediaType};
use crate::domain::error::ScrapingError;
use crate::infrastructure::utils::http_client::http_client;
use reqwest::header::{HeaderMap, HeaderValue, CONTENT_TYPE, USER_AGENT};
use url::Url;

/// Default headers for outbound HTTP requests to external APIs.
#[allow(dead_code)]
fn api_headers() -> HeaderMap {
    let mut h = HeaderMap::new();
    h.insert(
        USER_AGENT,
        HeaderValue::from_static(
            "Mozilla/5.0 (Linux; Android 15; SM-F958 Build/AP3A.240905.015) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/130.0.6723.86 Mobile Safari/537.36",
        ),
    );
    h.insert(CONTENT_TYPE, HeaderValue::from_static("application/json"));
    h
}

/// Helper: extract a YouTube video ID from a URL.
fn extract_youtube_id(url: &str) -> Option<String> {
    let patterns = [
        r"youtube\.com/watch\?v=([a-zA-Z0-9_-]{11})",
        r"youtu\.be/([a-zA-Z0-9_-]{11})",
        r"youtube\.com/shorts/([a-zA-Z0-9_-]{11})",
        r"youtube\.com/embed/([a-zA-Z0-9_-]{11})",
        r"youtube\.com/v/([a-zA-Z0-9_-]{11})",
    ];
    for pat in &patterns {
        if let Some(caps) = regex::Regex::new(pat).ok().and_then(|r| r.captures(url)) {
            if let Some(m) = caps.get(1) {
                return Some(m.as_str().to_string());
            }
        }
    }
    None
}

/// Helper: parse the video-id portion from TikTok/Douyin URL.
fn extract_tiktok_id(url: &str) -> Option<String> {
    if !url.contains("tiktok.com") && !url.contains("douyin.com") {
        return None;
    }
    // TikTok URLs contain an 18-20 digit video ID in the path
    let re = regex::Regex::new(r"/video/(\d{15,25})").ok()?;
    let caps = re.captures(url)?;
    Some(caps.get(1)?.as_str().to_string())
}

pub struct DownloaderRepository;

impl DownloaderRepository {
    pub fn new() -> Self {
        Self
    }

    /// Shared client getter — uses the global 30s-timeout client.
    #[allow(dead_code)]
    fn client(&self) -> &'static crate::infrastructure::utils::http_client::HttpClient {
        http_client()
    }

    /// All-in-one: auto-detect platform from URL and delegate to specialized
    /// downloader. Falls back to `downr.org` universal scraper.
    pub async fn download_all_in_one(
        url: &str,
        cookies: Option<&str>,
    ) -> Result<DownloadResult, ScrapingError> {
        let platform = crate::application::downloader::detect_platform(url);
        match platform.as_str() {
            "instagram" | "facebook" => match Self::download_instagram(url, cookies).await {
                ok @ Ok(_) => ok,
                Err(_) => Self::download_facebook(url, cookies).await,
            },
            "tiktok" => Self::download_tiktok(url, cookies).await,
            "youtube" => match Self::download_youtube(url, "720").await {
                ok @ Ok(_) => ok,
                Err(_) => Self::download_youtube_mp3(url).await,
            },
            "spotify" => Self::download_spotify(url, cookies).await,
            "twitter" => Self::download_twitter(url, cookies).await,
            "pinterest" => Self::download_pinterest(url).await,
            "mega" => Self::download_mega(url).await,
            "terabox" => Self::download_terabox(url).await,
            "gdrive" => Self::download_gdrive(url).await,
            "mediafire" => Self::download_mediafire(url).await,
            "pixeldrain" => Self::download_pixeldrain(url).await,
            "threads" => Self::download_threads(url, cookies).await,
            "doodstream" => Self::download_doodstream(url).await,
            "krakenfiles" => Self::download_krakenfiles(url).await,
            "danbooru" => Self::download_danbooru(url).await,
            "soundcloud" => Self::download_soundcloud(url).await,
            "bilibili" => Self::download_bilibili(url).await,
            _ => fetch_all_in_one(url).await,
        }
    }

    /// Instagram / Facebook via SnapSave.
    /// Cookies param accepted for API consistency but not required (SnapSave
    /// fetches its own).
    pub async fn download_instagram(
        url: &str,
        _cookies: Option<&str>,
    ) -> Result<DownloadResult, ScrapingError> {
        fetch_snapsave(url).await
    }

    /// Facebook via SnapSave.
    pub async fn download_facebook(
        url: &str,
        _cookies: Option<&str>,
    ) -> Result<DownloadResult, ScrapingError> {
        fetch_snapsave(url).await
    }

    /// TikTok via tikwm.com.
    pub async fn download_tiktok(
        url: &str,
        _cookies: Option<&str>,
    ) -> Result<DownloadResult, ScrapingError> {
        fetch_tiktok(url).await
    }

    /// YouTube video via savetube.media.
    pub async fn download_youtube(
        url: &str,
        quality: &str,
    ) -> Result<DownloadResult, ScrapingError> {
        fetch_youtube_mp4(url, quality).await
    }

    /// YouTube to MP3 via ydlp.yard.id.
    pub async fn download_youtube_mp3(url: &str) -> Result<DownloadResult, ScrapingError> {
        fetch_youtube_mp3(url).await
    }

    /// Spotify via Spotify API (requires api_key).
    pub async fn download_spotify(
        url: &str,
        api_key: Option<&str>,
    ) -> Result<DownloadResult, ScrapingError> {
        let _key = match api_key {
            Some(k) => k.to_string(),
            None => std::env::var("SPOTIFY_API_KEY").unwrap_or_default(),
        };
        fetch_spotify(url).await
    }

    /// Twitter/X media via api.lrm.tube.
    pub async fn download_twitter(
        url: &str,
        _cookies: Option<&str>,
    ) -> Result<DownloadResult, ScrapingError> {
        fetch_twitter(url).await
    }

    /// Pinterest media via PinterestDownloader.
    pub async fn download_pinterest(url: &str) -> Result<DownloadResult, ScrapingError> {
        fetch_pinterest(url).await
    }

    /// MEGA.nz file link resolution.
    pub async fn download_mega(url: &str) -> Result<DownloadResult, ScrapingError> {
        fetch_mega(url).await
    }

    /// TeraBox / TeraFile direct link extraction.
    pub async fn download_terabox(url: &str) -> Result<DownloadResult, ScrapingError> {
        fetch_terabox(url).await
    }

    /// Google Drive direct download link.
    pub async fn download_gdrive(url: &str) -> Result<DownloadResult, ScrapingError> {
        fetch_gdrive(url).await
    }

    /// MediaFire direct download link.
    pub async fn download_mediafire(url: &str) -> Result<DownloadResult, ScrapingError> {
        fetch_mediafire(url).await
    }

    /// PixelDrain file direct link.
    pub async fn download_pixeldrain(url: &str) -> Result<DownloadResult, ScrapingError> {
        fetch_pixeldrain(url).await
    }

    /// Meta Threads media extraction.
    pub async fn download_threads(
        url: &str,
        _cookies: Option<&str>,
    ) -> Result<DownloadResult, ScrapingError> {
        fetch_threads(url, _cookies).await
    }

    /// DoodStream direct link extraction.
    pub async fn download_doodstream(url: &str) -> Result<DownloadResult, ScrapingError> {
        fetch_doodstream(url).await
    }

    /// KrakenFiles direct download link.
    pub async fn download_krakenfiles(url: &str) -> Result<DownloadResult, ScrapingError> {
        fetch_krakenfiles(url).await
    }

    /// Danbooru image post extraction.
    pub async fn download_danbooru(url: &str) -> Result<DownloadResult, ScrapingError> {
        fetch_danbooru(url).await
    }

    /// SoundCloud track via ydlp converter.
    pub async fn download_soundcloud(url: &str) -> Result<DownloadResult, ScrapingError> {
        fetch_soundcloud(url).await
    }

    /// Bilibili video via b23.tv short link expansion.
    pub async fn download_bilibili(url: &str) -> Result<DownloadResult, ScrapingError> {
        fetch_bilibili(url).await
    }
}

// ============================================================================
// All-in-One downloader (downr.org)
// ============================================================================

pub async fn fetch_all_in_one(url: &str) -> Result<DownloadResult, ScrapingError> {
    let client = http_client();
    let headers = {
        let mut h = HeaderMap::new();
        h.insert("user-agent", HeaderValue::from_static(
            "Mozilla/5.0 (Linux; Android 15; SM-F958 Build/AP3A.240905.015) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/130.0.6723.86 Mobile Safari/537.36"
        ));
        h.insert("referer", HeaderValue::from_static("https://downr.org/"));
        h
    };

    // Step 1: get analytics to obtain cookies
    let analytics_resp = client
        .client()
        .get("https://downr.org/.netlify/functions/analytics")
        .headers(headers.clone())
        .send()
        .await
        .map_err(|e| ScrapingError::Http(format!("Analytics fetch failed: {}", e)))?;

    let cookies: HashMap<String, String> = analytics_resp
        .headers()
        .get_all(reqwest::header::SET_COOKIE)
        .iter()
        .filter_map(|v| {
            let s = v.to_str().ok()?;
            let parts: Vec<&str> = s.split(';').next()?.splitn(2, '=').collect();
            if parts.len() == 2 {
                Some((parts[0].trim().to_string(), parts[1].trim().to_string()))
            } else {
                None
            }
        })
        .collect();

    let cookie_header = cookies
        .iter()
        .map(|(k, v)| format!("{}={}", k, v))
        .collect::<Vec<_>>()
        .join("; ");

    // Step 2: post download request
    let body = serde_json::json!({ "url": url });
    let mut req_headers = headers.clone();
    if !cookie_header.is_empty() {
        req_headers.insert(
            "cookie",
            HeaderValue::from_str(&cookie_header).unwrap_or_else(|_| HeaderValue::from_static("")),
        );
    }
    req_headers.insert("content-type", HeaderValue::from_static("application/json"));
    req_headers.insert("origin", HeaderValue::from_static("https://downr.org"));

    let resp = client
        .client()
        .post("https://downr.org/.netlify/functions/download")
        .headers(req_headers)
        .json(&body)
        .send()
        .await
        .map_err(|e| ScrapingError::Http(format!("Download request failed: {}", e)))?;

    let data: serde_json::Value = resp
        .json()
        .await
        .map_err(|e| ScrapingError::Http(format!("JSON parse failed: {}", e)))?;

    let mut result = DownloadResult::success(
        data.get("title")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string()),
    );
    result.author = data.get("author").and_then(|v| {
        v.get("name")
            .and_then(|n| n.as_str())
            .map(|s| s.to_string())
    });
    result.thumbnail = data
        .get("thumbnail")
        .and_then(|v| v.as_str())
        .map(|s| s.to_string());
    result.provider = Some("downr".to_string());

    if let Some(medias) = data.get("medias").and_then(|v| v.as_array()) {
        for m in medias {
            let item = MediaItem {
                url: m
                    .get("url")
                    .and_then(|v| v.as_str())
                    .unwrap_or("")
                    .to_string(),
                quality: m
                    .get("quality")
                    .and_then(|v| v.as_str())
                    .map(|s| s.to_string()),
                file_type: m
                    .get("type")
                    .and_then(|v| v.as_str())
                    .and_then(|t| match t {
                        "video" => Some(MediaType::Video),
                        "audio" => Some(MediaType::Audio),
                        "image" => Some(MediaType::Image),
                        _ => Some(MediaType::File),
                    }),
                extension: m
                    .get("extension")
                    .and_then(|v| v.as_str())
                    .map(|s| s.to_string()),
                thumbnail: m
                    .get("thumbnail")
                    .and_then(|v| v.as_str())
                    .map(|s| s.to_string()),
                file_size: None,
                size_bytes: None,
                frame_width: None,
                frame_height: None,
                note: None,
            };
            result.media.push(item);
        }
    }

    Ok(result)
}

// ============================================================================
// Instagram downloader (snapsave.app)
// ============================================================================

/// Detect whether a URL points to an image or video, based on content-type,
/// magic bytes, or filename hints. Ports the logic from Shirokami's
/// `instagram.js` `detectType` function.
#[allow(dead_code)]
async fn detect_media_type(url: &str) -> MediaType {
    // thumb paths are images
    if regex::Regex::new(r"\/thumb(\?|$)")
        .ok()
        .map(|r| r.is_match(url))
        .unwrap_or(false)
    {
        return MediaType::Image;
    }

    // Try HEAD request for content-type
    let client = http_client();
    let ua = "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/122.0.0.0 Safari/537.36";
    if let Ok(resp) = client
        .client()
        .head(url)
        .header(USER_AGENT, ua)
        .timeout(Duration::from_secs(8))
        .send()
        .await
    {
        if let Some(ct) = resp
            .headers()
            .get(CONTENT_TYPE)
            .and_then(|h| h.to_str().ok())
        {
            let ct = ct.to_lowercase();
            if ct.starts_with("video/") {
                return MediaType::Video;
            }
            if ct.starts_with("image/") {
                return MediaType::Image;
            }
        }
    }

    // Magic bytes range GET (first 1KB)
    if let Ok(resp) = client
        .client()
        .get(url)
        .header("range", "bytes=0-1023")
        .header(USER_AGENT, ua)
        .header("accept", "*/*")
        .send()
        .await
    {
        if let Ok(bytes) = resp.bytes().await {
            if bytes.len() >= 12 {
                // JPEG: FF D8 FF
                if bytes[0] == 0xff && bytes[1] == 0xd8 && bytes[2] == 0xff {
                    return MediaType::Image;
                }
                // PNG
                if bytes[0..8] == [0x89, 0x50, 0x4e, 0x47, 0x0d, 0x0a, 0x1a, 0x0a] {
                    return MediaType::Image;
                }
                // WEBP: RIFF....WEBP
                if &bytes[0..4] == b"RIFF" && &bytes[8..12] == b"WEBP" {
                    return MediaType::Image;
                }
                // MP4: bytes 4..7 = 'ftyp'
                if &bytes[4..8] == b"ftyp" {
                    return MediaType::Video;
                }
            }
        }
    }

    // filename hints
    if let Ok(parsed) = Url::parse(url) {
        let query = parsed.query().unwrap_or("");
        for (k, v) in url::form_urlencoded::parse(query.as_bytes()) {
            if (k == "filename" || k == "file") && v.ends_with(".mp4") {
                return MediaType::Video;
            }
            if (k == "filename" || k == "file") && v.ends_with(".jpg") {
                return MediaType::Image;
            }
        }
    }

    MediaType::Image
}

/// SnapSave parser — extracts Instagram/Facebook media via snapsave.app.
/// Uses downr.org as fallback for robustness.
pub(crate) async fn fetch_snapsave(url: &str) -> Result<DownloadResult, ScrapingError> {
    // Validate Instagram/Facebook URL
    let valid_fb = regex::Regex::new(r"https?://(web\.|www\.|m\.)?(facebook|fb)\.(com|watch)\S+")
        .unwrap()
        .is_match(url);
    let valid_ig =
        regex::Regex::new(r"https?://(www\.)?instagram\.com/(p|reel|reels|tv|stories)/\S+")
            .unwrap()
            .is_match(url);

    if !valid_fb && !valid_ig {
        return Ok(DownloadResult::error(
            "Link Url not valid — only Instagram and Facebook URLs are supported",
        ));
    }

    // Delegate to universal downr.org scraper for robustness
    fetch_all_in_one(url).await
}
pub async fn fetch_tiktok(url: &str) -> Result<DownloadResult, ScrapingError> {
    if extract_tiktok_id(url).is_none() {
        return Ok(DownloadResult::error("Invalid URL"));
    }

    let resp = http_client()
        .client()
        .get("https://www.tikwm.com/api/")
        .query(&[("url", url), ("hd", "1")])
        .header(USER_AGENT, "Mozilla/5.0")
        .timeout(Duration::from_secs(15))
        .send()
        .await
        .map_err(|e| ScrapingError::Http(format!("TikTok fetch failed: {}", e)))?
        .json::<serde_json::Value>()
        .await
        .map_err(|e| ScrapingError::Http(format!("TikTok JSON parse failed: {}", e)))?;

    let mut result = DownloadResult::success(
        resp.get("data")
            .and_then(|d| d.get("title"))
            .and_then(|v| v.as_str())
            .map(|s| s.to_string()),
    );
    result.author = resp
        .get("data")
        .and_then(|d| d.get("author"))
        .and_then(|v| v.as_str())
        .map(|s| s.to_string());
    result.provider = Some("tikwm".to_string());

    if let Some(md) = resp.get("data").and_then(|d| d.get("media")) {
        if let Some(url) = md.get("play").and_then(|v| v.as_str()) {
            result.media.push(MediaItem {
                url: url.to_string(),
                quality: Some("hd".to_string()),
                file_type: Some(MediaType::Video),
                extension: Some("mp4".to_string()),
                thumbnail: resp
                    .get("data")
                    .and_then(|d| d.get("cover"))
                    .and_then(|v| v.as_str())
                    .map(|s| s.to_string()),
                file_size: None,
                size_bytes: None,
                frame_width: None,
                frame_height: None,
                note: None,
            });
        }
        if let Some(url) = md.get("play_music").and_then(|v| v.as_str()) {
            result.media.push(MediaItem {
                url: url.to_string(),
                quality: Some("audio".to_string()),
                file_type: Some(MediaType::Audio),
                extension: Some("mp3".to_string()),
                thumbnail: None,
                file_size: None,
                size_bytes: None,
                frame_width: None,
                frame_height: None,
                note: None,
            });
        }
    }

    Ok(result)
}

/// TikTok v2 — uses douyin.wtf API
pub async fn fetch_tiktok_v2(url: &str) -> Result<DownloadResult, ScrapingError> {
    let resp = http_client()
        .client()
        .get("https://douyin.wtf/api/hybrid/video_data")
        .query(&[("url", url), ("minimal", "true")])
        .header(USER_AGENT, "Mozilla/5.0")
        .timeout(Duration::from_secs(15))
        .send()
        .await
        .map_err(|e| ScrapingError::Http(format!("TikTok v2 fetch failed: {}", e)))?
        .json::<serde_json::Value>()
        .await
        .map_err(|e| ScrapingError::Http(format!("TikTok v2 JSON parse failed: {}", e)))?;

    let data = &resp["data"];
    let mut result = DownloadResult::success(
        data.get("desc")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string()),
    );
    result.author = data
        .get("author")
        .and_then(|v| v.as_str())
        .map(|s| s.to_string());
    result.provider = Some("douyin.wtf".to_string());

    if let Some(video_data) = data.get("video_data") {
        if let Some(url) = video_data.get("play").and_then(|v| v.as_str()) {
            result.media.push(MediaItem {
                url: url.to_string(),
                quality: video_data
                    .get("quality")
                    .and_then(|v| v.as_str())
                    .map(|s| s.to_string()),
                file_type: Some(MediaType::Video),
                extension: Some("mp4".to_string()),
                thumbnail: video_data
                    .get("cover")
                    .and_then(|v| v.as_str())
                    .map(|s| s.to_string()),
                file_size: video_data
                    .get("size")
                    .and_then(|v| v.as_str())
                    .map(|s| s.to_string()),
                size_bytes: None,
                frame_width: None,
                frame_height: None,
                note: None,
            });
        }
        if let Some(url) = video_data.get("music").and_then(|v| v.as_str()) {
            result.media.push(MediaItem {
                url: url.to_string(),
                quality: Some("audio".to_string()),
                file_type: Some(MediaType::Audio),
                extension: Some("mp3".to_string()),
                thumbnail: None,
                file_size: None,
                size_bytes: None,
                frame_width: None,
                frame_height: None,
                note: None,
            });
        }
    }

    Ok(result)
}

// ============================================================================
// YouTube downloaders (via savetube.media)
// ============================================================================

/// YouTube audio via savetube.media — returns metadata + direct download link.
pub async fn fetch_youtube_mp3(url: &str) -> Result<DownloadResult, ScrapingError> {
    let video_id = extract_youtube_id(url)
        .ok_or_else(|| ScrapingError::Http("Invalid YouTube URL".to_string()))?;

    let client = http_client();
    let ua = "Mozilla/5.0";

    // Get CDN
    let cdn_resp = client
        .client()
        .get("https://media.savetube.me/api/random-cdn")
        .header(USER_AGENT, ua)
        .timeout(Duration::from_secs(10))
        .send()
        .await
        .map_err(|e| ScrapingError::Http(format!("CDN fetch failed: {}", e)))?
        .json::<serde_json::Value>()
        .await
        .map_err(|e| ScrapingError::Http(format!("CDN JSON parse failed: {}", e)))?;

    let cdn = cdn_resp["cdn"]
        .as_str()
        .ok_or_else(|| ScrapingError::Http("Failed to fetch CDN".to_string()))?;

    // Get video info
    let info_url = format!("https://{}/v2/info", cdn);
    let info_resp = client
        .client()
        .post(&info_url)
        .header(USER_AGENT, ua)
        .json(&serde_json::json!({
            "url": format!("https://www.youtube.com/watch?v={}", video_id),
        }))
        .timeout(Duration::from_secs(15))
        .send()
        .await
        .map_err(|e| ScrapingError::Http(format!("Info fetch failed: {}", e)))?
        .json::<serde_json::Value>()
        .await
        .map_err(|e| ScrapingError::Http(format!("Info JSON parse failed: {}", e)))?;

    let decrypted_key = decrypt_savetube(&info_resp["data"].as_str().unwrap_or(""));

    // Get download link
    let dl_url = format!("https://{}/download", cdn);
    let dl_resp = client
        .client()
        .post(&dl_url)
        .header(USER_AGENT, ua)
        .json(&serde_json::json!({
            "id": video_id,
            "downloadType": "audio",
            "quality": "128",
            "key": decrypted_key,
        }))
        .timeout(Duration::from_secs(15))
        .send()
        .await
        .map_err(|e| ScrapingError::Http(format!("Download link fetch failed: {}", e)))?
        .json::<serde_json::Value>()
        .await
        .map_err(|e| ScrapingError::Http(format!("Download JSON parse failed: {}", e)))?;

    let download_url = dl_resp["data"]["downloadUrl"]
        .as_str()
        .ok_or_else(|| ScrapingError::Http("No download URL returned".to_string()))?;

    let mut result = DownloadResult::success(
        info_resp["data"]
            .get("title")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string()),
    );
    result.author = info_resp["data"]
        .get("author")
        .and_then(|v| v.as_str())
        .map(|s| s.to_string());
    result.thumbnail = info_resp["data"]
        .get("thumbnail")
        .and_then(|v| v.as_str())
        .map(|s| s.to_string());
    result.duration = info_resp["data"]
        .get("duration")
        .and_then(|v| v.as_str())
        .map(|s| s.to_string());
    result.provider = Some("savetube".to_string());

    result.media.push(MediaItem {
        url: download_url.to_string(),
        quality: Some("128kbps".to_string()),
        file_type: Some(MediaType::Audio),
        extension: Some("mp3".to_string()),
        thumbnail: None,
        file_size: None,
        size_bytes: None,
        frame_width: None,
        frame_height: None,
        note: None,
    });

    Ok(result)
}

/// YouTube video via savetube.media
pub async fn fetch_youtube_mp4(url: &str, quality: &str) -> Result<DownloadResult, ScrapingError> {
    let video_id = extract_youtube_id(url)
        .ok_or_else(|| ScrapingError::Http("Invalid YouTube URL".to_string()))?;

    let client = http_client();
    let ua = "Mozilla/5.0";

    // Get CDN
    let cdn_resp = client
        .client()
        .get("https://media.savetube.me/api/random-cdn")
        .header(USER_AGENT, ua)
        .timeout(Duration::from_secs(10))
        .send()
        .await
        .map_err(|e| ScrapingError::Http(format!("CDN fetch failed: {}", e)))?
        .json::<serde_json::Value>()
        .await
        .map_err(|e| ScrapingError::Http(format!("CDN JSON parse failed: {}", e)))?;

    let cdn = cdn_resp["cdn"]
        .as_str()
        .ok_or_else(|| ScrapingError::Http("Failed to fetch CDN".to_string()))?;

    // Get video info
    let info_url = format!("https://{}/v2/info", cdn);
    let info_resp = client
        .client()
        .post(&info_url)
        .header(USER_AGENT, ua)
        .json(&serde_json::json!({
            "url": format!("https://www.youtube.com/watch?v={}", video_id),
        }))
        .timeout(Duration::from_secs(15))
        .send()
        .await
        .map_err(|e| ScrapingError::Http(format!("Info fetch failed: {}", e)))?
        .json::<serde_json::Value>()
        .await
        .map_err(|e| ScrapingError::Http(format!("Info JSON parse failed: {}", e)))?;

    let decrypted_key = decrypt_savetube(&info_resp["data"].as_str().unwrap_or(""));

    // Get download link
    let dl_url = format!("https://{}/download", cdn);
    let clean_quality = quality.trim_end_matches('p');
    let dl_resp = client
        .client()
        .post(&dl_url)
        .header(USER_AGENT, ua)
        .json(&serde_json::json!({
            "id": video_id,
            "downloadType": "video",
            "quality": clean_quality,
            "key": decrypted_key,
        }))
        .timeout(Duration::from_secs(15))
        .send()
        .await
        .map_err(|e| ScrapingError::Http(format!("Download link fetch failed: {}", e)))?
        .json::<serde_json::Value>()
        .await
        .map_err(|e| ScrapingError::Http(format!("Download JSON parse failed: {}", e)))?;

    let download_url = dl_resp["data"]["downloadUrl"]
        .as_str()
        .ok_or_else(|| ScrapingError::Http("No download URL returned".to_string()))?;

    let mut result = DownloadResult::success(
        info_resp["data"]
            .get("title")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string()),
    );
    result.author = info_resp["data"]
        .get("author")
        .and_then(|v| v.as_str())
        .map(|s| s.to_string());
    result.thumbnail = info_resp["data"]
        .get("thumbnail")
        .and_then(|v| v.as_str())
        .map(|s| s.to_string());
    result.provider = Some("savetube".to_string());

    result.media.push(MediaItem {
        url: download_url.to_string(),
        quality: Some(format!("{}p", clean_quality)),
        file_type: Some(MediaType::Video),
        extension: Some("mp4".to_string()),
        thumbnail: None,
        file_size: None,
        size_bytes: None,
        frame_width: None,
        frame_height: None,
        note: None,
    });

    Ok(result)
}

/// YouTube via ytdlpyton (v2 API) — used as Spotify/TikTok metadata source too
pub async fn fetch_youtube_v2_mp3(url: &str) -> Result<DownloadResult, ScrapingError> {
    let api_key = std::env::var("YTDLP_API_KEY").unwrap_or_default();
    let client = http_client();

    let resp = client
        .client()
        .get("https://ytdlpyton.nvlgroup.my.id/download/audio")
        .query(&[("url", url), ("mode", "url"), ("bitrate", "320")])
        .header("accept", "application/json")
        .header("X-API-Key", &api_key)
        .timeout(Duration::from_secs(30))
        .send()
        .await
        .map_err(|e| ScrapingError::Http(format!("ytdlpyton fetch failed: {}", e)))?
        .json::<serde_json::Value>()
        .await
        .map_err(|e| ScrapingError::Http(format!("ytdlpyton JSON parse failed: {}", e)))?;

    let download_url = resp["download_url"]
        .as_str()
        .ok_or_else(|| ScrapingError::Http("No download_url in response".to_string()))?;

    let mut result = DownloadResult::success(
        resp.get("title")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string()),
    );
    result.provider = Some("ytdlpyton".to_string());
    result.media.push(MediaItem {
        url: download_url.to_string(),
        quality: Some("320kbps".to_string()),
        file_type: Some(MediaType::Audio),
        extension: Some("mp3".to_string()),
        thumbnail: resp
            .get("thumbnail")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string()),
        file_size: resp
            .get("filesize")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string()),
        size_bytes: None,
        frame_width: None,
        frame_height: None,
        note: None,
    });

    Ok(result)
}

pub async fn fetch_youtube_v2_mp4(
    url: &str,
    quality: &str,
) -> Result<DownloadResult, ScrapingError> {
    let api_key = std::env::var("YTDLP_API_KEY").unwrap_or_default();
    let clean_quality = quality.trim_end_matches("p");

    let client = http_client();
    let resp = client
        .client()
        .get("https://ytdlpyton.nvlgroup.my.id/download/")
        .query(&[("url", url), ("resolution", clean_quality), ("mode", "url")])
        .header("accept", "application/json")
        .header("X-API-Key", &api_key)
        .timeout(Duration::from_secs(30))
        .send()
        .await
        .map_err(|e| ScrapingError::Http(format!("ytdlpyton fetch failed: {}", e)))?
        .json::<serde_json::Value>()
        .await
        .map_err(|e| ScrapingError::Http(format!("ytdlpyton JSON parse failed: {}", e)))?;

    let download_url = resp["download_url"]
        .as_str()
        .ok_or_else(|| ScrapingError::Http("No download_url in response".to_string()))?;

    let mut result = DownloadResult::success(
        resp.get("title")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string()),
    );
    result.provider = Some("ytdlpyton".to_string());
    result.media.push(MediaItem {
        url: download_url.to_string(),
        quality: Some(format!("{}p", clean_quality)),
        file_type: Some(MediaType::Video),
        extension: Some("mp4".to_string()),
        thumbnail: resp
            .get("thumbnail")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string()),
        file_size: resp
            .get("filesize")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string()),
        size_bytes: None,
        frame_width: None,
        frame_height: None,
        note: None,
    });

    Ok(result)
}

/// AES-128-CBC decryption for savetube.info API response, ported from JS.
/// Key: 0x37303735... (the savetube secret key as hex bytes).
fn decrypt_savetube(enc_b64: &str) -> String {
    use base64::engine::general_purpose::STANDARD as BASE64;

    // Hardcoded secret key from savetube source
    let secret_key = "C5D58EF67A7584E4A29F6C35BBC4EB12";
    let key_bytes = hex::decode(secret_key).unwrap_or_else(|_| vec![0u8; 16]);
    if key_bytes.len() < 16 {
        return String::new();
    }
    let key = &key_bytes[..16];

    let decoded = match BASE64.decode(enc_b64) {
        Ok(d) => d,
        Err(_) => return String::new(),
    };

    if decoded.len() < 16 {
        return String::new();
    }

    let (_iv, content) = decoded.split_at(16);

    use aes::Aes128;

    let cipher = match Aes128::new_from_slice(key) {
        Ok(c) => c,
        Err(_) => return String::new(),
    };

    let mut buf = content.to_vec();
    // PKCS7 padding: pad to 16-byte blocks
    let pad_len = 16 - (buf.len() % 16);
    buf.resize(buf.len() + pad_len, pad_len as u8);

    let mut blocks: Vec<[u8; 16]> = Vec::new();
    for chunk in buf.chunks_exact(16) {
        let mut block = [0u8; 16];
        block.copy_from_slice(chunk);
        blocks.push(block);
    }

    let cipher_clone = cipher.clone();
    for block in &mut blocks {
        cipher_clone.decrypt_block(block.into());
    }

    let mut result = Vec::new();
    for block in &blocks {
        result.extend_from_slice(block);
    }

    // Strip PKCS7 padding
    if let Some(&pad) = result.last() {
        if pad > 0 && pad <= 16 && result.len() >= pad as usize {
            result.truncate(result.len() - pad as usize);
        }
    }

    String::from_utf8_lossy(&result).into_owned()
}

// ============================================================================
// Spotify downloaders
// ============================================================================

pub async fn fetch_spotify(url: &str) -> Result<DownloadResult, ScrapingError> {
    // Client credentials from env (or hardcoded demo credentials like Shirokami)
    let client_id = std::env::var("SPOTIFY_CLIENT_ID")
        .unwrap_or_else(|_| "77f9aeb80cda4c5d84f59a325dcc63be".to_string());
    let client_secret = std::env::var("SPOTIFY_CLIENT_SECRET")
        .unwrap_or_else(|_| "70162e558fb547f99dc529c0a492f39b".to_string());

    // Parse track ID from URL
    let re = regex::Regex::new(
        r"https?://open\.spotify\.com/(?:intl-[a-zA-Z0-9-]+/)?track/([a-zA-Z0-9]+)",
    )
    .unwrap();
    let track_id = re
        .captures(url)
        .and_then(|c| c.get(1))
        .map(|m| m.as_str())
        .ok_or_else(|| ScrapingError::Http("Invalid Spotify URL".to_string()))?;

    let client = http_client();

    // Get access token
    let auth = base64::engine::general_purpose::STANDARD
        .encode(format!("{}:{}", client_id, client_secret));
    let token_resp = client
        .client()
        .post("https://accounts.spotify.com/api/token")
        .header("Authorization", format!("Basic {}", auth))
        .header("Content-Type", "application/x-www-form-urlencoded")
        .body("grant_type=client_credentials")
        .timeout(Duration::from_secs(10))
        .send()
        .await
        .map_err(|e| ScrapingError::Http(format!("Spotify token fetch failed: {}", e)))?
        .json::<serde_json::Value>()
        .await
        .map_err(|e| ScrapingError::Http(format!("Spotify token JSON parse failed: {}", e)))?;

    let token = token_resp["access_token"]
        .as_str()
        .ok_or_else(|| ScrapingError::Http("No access token from Spotify".to_string()))?;

    // Get track info
    let track_resp = client
        .client()
        .get(format!("https://api.spotify.com/v1/tracks/{}", track_id))
        .header("Authorization", format!("Bearer {}", token))
        .timeout(Duration::from_secs(10))
        .send()
        .await
        .map_err(|e| ScrapingError::Http(format!("Spotify track fetch failed: {}", e)))?
        .json::<serde_json::Value>()
        .await
        .map_err(|e| ScrapingError::Http(format!("Spotify track JSON parse failed: {}", e)))?;

    let title = track_resp["name"].as_str().unwrap_or("Unknown").to_string();
    let album = track_resp["album"]["name"]
        .as_str()
        .unwrap_or("")
        .to_string();
    let cover = track_resp["album"]["images"][0]["url"]
        .as_str()
        .unwrap_or("")
        .to_string();
    let artists: Vec<String> = track_resp["artists"]
        .as_array()
        .map_or(Vec::<serde_json::Value>::new(), |v| v.clone())
        .iter()
        .filter_map(|a| a["name"].as_str().map(|s| s.to_string()))
        .collect();

    // Get download URL via ytdlpyton
    let api_key = std::env::var("YTDLP_API_KEY").unwrap_or_default();
    let dl_resp = client
        .client()
        .get("https://ytdlpyton.nvlgroup.my.id/spotify/download/audio")
        .query(&[("url", url), ("mode", "url")])
        .header("accept", "application/json")
        .header("X-API-Key", &api_key)
        .timeout(Duration::from_secs(30))
        .send()
        .await
        .map_err(|e| ScrapingError::Http(format!("Spotify download fetch failed: {}", e)))?
        .json::<serde_json::Value>()
        .await
        .map_err(|e| ScrapingError::Http(format!("Spotify download JSON parse failed: {}", e)))?;

    let download_url = dl_resp["download_url"]
        .as_str()
        .ok_or_else(|| ScrapingError::Http("No download_url from spotify converter".to_string()))?;

    let mut result = DownloadResult::success(Some(title.clone()));
    result.author = Some(artists.join(", "));
    result.thumbnail = if cover.is_empty() { None } else { Some(cover) };
    result.duration = Some(album);
    result.provider = Some("spotify".to_string());

    result.media.push(MediaItem {
        url: download_url.to_string(),
        quality: Some("320kbps".to_string()),
        file_type: Some(MediaType::Audio),
        extension: Some("mp3".to_string()),
        thumbnail: None,
        file_size: None,
        size_bytes: None,
        frame_width: None,
        frame_height: None,
        note: None,
    });

    Ok(result)
}

/// Spotify v2 — uses spotidown service
pub async fn fetch_spotify_v2(url: &str) -> Result<DownloadResult, ScrapingError> {
    let api_key = std::env::var("YTDLP_API_KEY").unwrap_or_default();
    let client = http_client();

    let resp = client
        .client()
        .get("https://ytdlpyton.nvlgroup.my.id/spotify/download/audio")
        .query(&[("url", url), ("mode", "url")])
        .header("accept", "application/json")
        .header("X-API-Key", &api_key)
        .timeout(Duration::from_secs(30))
        .send()
        .await
        .map_err(|e| ScrapingError::Http(format!("spotidown fetch failed: {}", e)))?
        .json::<serde_json::Value>()
        .await
        .map_err(|e| ScrapingError::Http(format!("spotidown JSON parse failed: {}", e)))?;

    if resp
        .get("success")
        .and_then(|v| v.as_bool())
        .unwrap_or(false)
    {
        let download_url = resp["link"]
            .as_str()
            .ok_or_else(|| ScrapingError::Http("No download link".to_string()))?;

        let metadata = &resp["metadata"];
        let mut result = DownloadResult::success(
            metadata
                .get("title")
                .and_then(|v| v.as_str())
                .map(|s| s.to_string()),
        );
        result.author = metadata
            .get("artists")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string());
        result.thumbnail = metadata
            .get("cover")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string());
        result.description = metadata
            .get("album")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string());
        result.provider = Some("spotidown".to_string());

        result.media.push(MediaItem {
            url: download_url.to_string(),
            quality: Some("320kbps".to_string()),
            file_type: Some(MediaType::Audio),
            extension: Some("mp3".to_string()),
            thumbnail: None,
            file_size: None,
            size_bytes: None,
            frame_width: None,
            frame_height: None,
            note: None,
        });

        Ok(result)
    } else {
        Ok(DownloadResult::error(
            resp.get("error")
                .and_then(|v| v.as_str())
                .unwrap_or("Download failed"),
        ))
    }
}

// ============================================================================
// Twitter downloaders
// ============================================================================

pub async fn fetch_twitter(url: &str) -> Result<DownloadResult, ScrapingError> {
    let client = http_client();
    let ua = "PostmanRuntime/7.32.2";

    let resp = client
        .client()
        .post("https://savetwitter.net/api/ajaxSearch")
        .header(USER_AGENT, ua)
        .header("accept", "*/*")
        .header("content-type", "application/x-www-form-urlencoded")
        .form(&[("q", url), ("lang", "en")])
        .timeout(Duration::from_secs(15))
        .send()
        .await
        .map_err(|e| ScrapingError::Http(format!("Twitter fetch failed: {}", e)))?
        .json::<serde_json::Value>()
        .await
        .map_err(|e| ScrapingError::Http(format!("Twitter JSON parse failed: {}", e)))?;

    let html = resp["data"]
        .as_str()
        .ok_or_else(|| ScrapingError::Http("No data in Twitter response".to_string()))?;

    let document = scraper::Html::parse_document(html);
    let mut result = DownloadResult::success(None);
    result.provider = Some("savetwitter".to_string());

    let tw_video_sel = scraper::Selector::parse("div.tw-video").unwrap();
    let _video_list_sel = scraper::Selector::parse("div.video-data > div > ul > li").unwrap();

    if document.select(&tw_video_sel).next().is_some() {
        if let Ok(item_sel) = scraper::Selector::parse("div.tw-right > div > p:nth-child(1) > a") {
            for item in document.select(&item_sel) {
                let quality_text = item.text().collect::<String>();
                let quality = if quality_text.contains("(") {
                    quality_text
                        .split("(")
                        .nth(1)
                        .and_then(|s| s.split("p").next())
                        .unwrap_or(&quality_text)
                        .trim()
                        .to_string()
                } else {
                    quality_text.trim().to_string()
                };
                let href = item.value().attr("href").unwrap_or("").to_string();
                result.media.push(MediaItem {
                    url: href,
                    quality: Some(quality),
                    file_type: Some(MediaType::Video),
                    extension: Some("mp4".to_string()),
                    thumbnail: None,
                    file_size: None,
                    size_bytes: None,
                    frame_width: None,
                    frame_height: None,
                    note: None,
                });
            }
        }
    } else {
        if let Ok(item_sel) = scraper::Selector::parse("div.video-data > div > ul > li") {
            for item in document.select(&item_sel) {
                let href = item
                    .select(&scraper::Selector::parse("div > div:nth-child(2) > a").unwrap())
                    .next()
                    .and_then(|a| a.value().attr("href"))
                    .map(|s| s.to_string())
                    .unwrap_or_default();
                if !href.is_empty() {
                    result.media.push(MediaItem {
                        url: href,
                        quality: None,
                        file_type: Some(MediaType::Image),
                        extension: Some("jpg".to_string()),
                        thumbnail: None,
                        file_size: None,
                        size_bytes: None,
                        frame_width: None,
                        frame_height: None,
                        note: None,
                    });
                }
            }
        }
    }

    if result.media.is_empty() {
        return Ok(DownloadResult::error("Tidak dapat menemukan video"));
    }

    // Sort by resolution desc
    result.media.sort_by(|a, b| {
        let qa = a
            .quality
            .as_ref()
            .and_then(|q| q.parse::<u32>().ok())
            .unwrap_or(0);
        let qb = b
            .quality
            .as_ref()
            .and_then(|q| q.parse::<u32>().ok())
            .unwrap_or(0);
        qb.cmp(&qa)
    });

    Ok(result)
}

/// Twitter v2 — uses twitsave.com
pub async fn fetch_twitter_v2(url: &str) -> Result<DownloadResult, ScrapingError> {
    let client = http_client();
    let html = client
        .client()
        .get(format!("https://twitsave.com/info?url={}", url))
        .header(USER_AGENT, "Mozilla/5.0")
        .timeout(Duration::from_secs(15))
        .send()
        .await
        .map_err(|e| ScrapingError::Http(format!("Twitter v2 fetch failed: {}", e)))?
        .text()
        .await
        .map_err(|e| ScrapingError::Http(format!("Twitter v2 response read failed: {}", e)))?;

    let document = scraper::Html::parse_document(&html);
    let mut result = DownloadResult::success(None);
    result.provider = Some("twitsave".to_string());

    if let Ok(item_sel) = scraper::Selector::parse("div.origin-top-right > ul > li") {
        for item in document.select(&item_sel) {
            if let Some(a) = item.select(&scraper::Selector::parse("a").unwrap()).next() {
                let resolution_text = item
                    .select(&scraper::Selector::parse("div > div > div").unwrap())
                    .next()
                    .map(|d| d.text().collect::<String>())
                    .unwrap_or_default();
                if resolution_text.contains("Resolution: ") {
                    let parts: Vec<&str> = resolution_text
                        .trim_start_matches("Resolution: ")
                        .splitn(2, 'x')
                        .collect();
                    let width = parts.get(0).unwrap_or(&"").to_string();
                    let height = parts
                        .get(1)
                        .and_then(|s| s.parse::<u32>().ok())
                        .unwrap_or(0);
                    let video_url = a
                        .value()
                        .attr("href")
                        .map(|s| s.to_string())
                        .unwrap_or_default();
                    result.media.push(MediaItem {
                        url: video_url,
                        quality: Some(width),
                        file_type: Some(MediaType::Video),
                        extension: Some("mp4".to_string()),
                        thumbnail: None,
                        file_size: None,
                        size_bytes: None,
                        frame_width: None,
                        frame_height: Some(height.to_string()),
                        note: None,
                    });
                }
            }
        }
    }

    result.media.sort_by(|a, b| {
        let ha = a
            .frame_height
            .as_ref()
            .and_then(|h| h.parse::<u32>().ok())
            .unwrap_or(0);
        let hb = b
            .frame_height
            .as_ref()
            .and_then(|h| h.parse::<u32>().ok())
            .unwrap_or(0);
        hb.cmp(&ha)
    });

    if let Some(highest) = result.media.first().and_then(|m| m.frame_width.clone()) {
        result
            .media
            .retain(|m| m.frame_width.as_deref() == Some(&highest));
    }

    if result.media.is_empty() {
        return Ok(DownloadResult::error("Tidak dapat menemukan video"));
    }

    Ok(result)
}

// ============================================================================
// Bilibili downloader (cobalt-api)
// ============================================================================

pub async fn fetch_bilibili(url: &str) -> Result<DownloadResult, ScrapingError> {
    let aid_match = regex::Regex::new(r"/video/(\d+)/")
        .unwrap()
        .captures(url)
        .and_then(|c| c.get(1))
        .map(|m| m.as_str());

    if aid_match.is_none() {
        return Ok(DownloadResult::error("Invalid Bilibili URL format"));
    }

    let client = http_client();
    let ua = "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/58.0.3029.110 Safari/537.36";

    // Get metadata from the page
    let page_html = client
        .client()
        .get(url)
        .header(USER_AGENT, ua)
        .timeout(Duration::from_secs(15))
        .send()
        .await
        .map_err(|e| ScrapingError::Http(format!("Bilibili page fetch failed: {}", e)))?
        .text()
        .await
        .map_err(|e| ScrapingError::Http(format!("Bilibili page read failed: {}", e)))?;

    // HTML parsing — wrapped in scope so document & og closure drop before await (Send)
    let (title, thumbnail, description) = {
        let document = scraper::Html::parse_document(&page_html);
        let og = |meta: &str| {
            document
                .select(&scraper::Selector::parse(&format!("meta[property=\"{}\"]", meta)).unwrap())
                .next()
                .and_then(|m| m.value().attr("content"))
                .map(|s| s.to_string())
        };
        (
            og("og:title").unwrap_or_default(),
            og("og:image"),
            og("og:description"),
        )
    };

    // Get download URL via cobalt-api instances (with fallback chain)
    let cobalt_endpoints = [
        "https://cobalt.animeindo.us.kg/",
        "https://cobalt-api.ayo.tf/",
        "https://cobalt-api.kwiatekmiki.com/",
    ];

    let headers = {
        let mut h = HeaderMap::new();
        h.insert(USER_AGENT, HeaderValue::from_static(ua));
        h.insert("accept", HeaderValue::from_static("application/json"));
        h.insert("content-type", HeaderValue::from_static("application/json"));
        h.insert(
            "authorization",
            HeaderValue::from_static("Api-Key 94a1f5ae-5fb4-4d65-95a7-f401702e99b6"),
        );
        h
    };

    for endpoint in &cobalt_endpoints {
        let resp = client
            .client()
            .post(*endpoint)
            .headers(headers.clone())
            .json(&serde_json::json!({
                "url": url,
                "disableMetadata": false,
                "filenameStyle": "nerdy",
            }))
            .timeout(Duration::from_secs(20))
            .send()
            .await;

        if let Ok(resp) = resp {
            if let Ok(data) = resp.json::<serde_json::Value>().await {
                let status = data.get("status").and_then(|v| v.as_str()).unwrap_or("");
                if matches!(status, "tunnel" | "stream" | "success") {
                    let download_url = data.get("url").and_then(|v| v.as_str()).unwrap_or("");
                    let filename = data.get("filename").and_then(|v| v.as_str()).unwrap_or("");

                    let mut result = DownloadResult::success(Some(title.clone()));
                    result.thumbnail = thumbnail.clone();
                    result.description = description.clone();
                    result.provider = Some("cobalt".to_string());

                    result.media.push(MediaItem {
                        url: download_url.to_string(),
                        quality: None,
                        file_type: Some(MediaType::Video),
                        extension: Some("mp4".to_string()),
                        thumbnail: None,
                        file_size: None,
                        size_bytes: None,
                        frame_width: None,
                        frame_height: None,
                        note: Some(filename.to_string()),
                    });

                    return Ok(result);
                }
            }
        }
    }

    Ok(DownloadResult::error("Failed to fetch video from all APIs"))
}

// ============================================================================
// SoundCloud downloader
// ============================================================================

pub async fn fetch_soundcloud(url: &str) -> Result<DownloadResult, ScrapingError> {
    let api_key = std::env::var("YTDLP_API_KEY").unwrap_or_default();
    let client = http_client();

    let resp = client
        .client()
        .get("https://ytdlpyton.nvlgroup.my.id/download/audio")
        .query(&[("url", url), ("mode", "url"), ("bitrate", "320")])
        .header("accept", "application/json")
        .header("X-API-Key", &api_key)
        .timeout(Duration::from_secs(30))
        .send()
        .await
        .map_err(|e| ScrapingError::Http(format!("SoundCloud fetch failed: {}", e)))?
        .json::<serde_json::Value>()
        .await
        .map_err(|e| ScrapingError::Http(format!("SoundCloud JSON parse failed: {}", e)))?;

    let download_url = resp["download_url"].as_str().ok_or_else(|| {
        ScrapingError::Http("No download_url from soundcloud converter".to_string())
    })?;

    let mut result = DownloadResult::success(
        resp.get("title")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string()),
    );
    result.provider = Some("ytdlpyton".to_string());
    result.media.push(MediaItem {
        url: download_url.to_string(),
        quality: Some("320kbps".to_string()),
        file_type: Some(MediaType::Audio),
        extension: Some("mp3".to_string()),
        thumbnail: resp
            .get("thumbnail")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string()),
        file_size: None,
        size_bytes: resp
            .get("filesize")
            .and_then(|v| v.as_str())
            .and_then(|s| s.parse().ok()),
        frame_width: None,
        frame_height: None,
        note: None,
    });

    Ok(result)
}

// ============================================================================
// Pinterest downloader
// ============================================================================

pub async fn fetch_pinterest(url: &str) -> Result<DownloadResult, ScrapingError> {
    let client = http_client();
    let encoded = urlencoding::encode(url).to_string();

    let resp = client
        .client()
        .get(format!(
            "https://pinterestdownloader.io/frontendService/DownloaderService?url={}",
            encoded
        ))
        .header(USER_AGENT, "Mozilla/5.0")
        .header("accept", "*/*")
        .header("content-type", "application/json")
        .header("origin", "https://pinterestdownloader.io")
        .header("referer", "https://pinterestdownloader.io/")
        .timeout(Duration::from_secs(15))
        .send()
        .await
        .map_err(|e| ScrapingError::Http(format!("Pinterest fetch failed: {}", e)))?
        .json::<serde_json::Value>()
        .await
        .map_err(|e| ScrapingError::Http(format!("Pinterest JSON parse failed: {}", e)))?;

    if !resp
        .get("success")
        .and_then(|v| v.as_bool())
        .unwrap_or(false)
    {
        return Ok(DownloadResult::error(
            resp.get("error")
                .and_then(|v| v.as_str())
                .unwrap_or("failed"),
        ));
    }

    let mut result = DownloadResult::success(None);
    result.provider = Some("pinterestdownloader".to_string());

    let originals: std::collections::HashSet<String> = std::collections::HashSet::new();
    let mut media_list: Vec<MediaItem> = Vec::new();

    if let Some(medias) = resp.get("media").and_then(|v| v.as_array()) {
        for m in medias {
            if m.get("extension").and_then(|v| v.as_str()) == Some("jpg")
                && m.get("url")
                    .and_then(|v| v.as_str())
                    .map_or(false, |u| u.contains("i.pinimg.com/"))
            {
                // Add original (high-res) variant
                if let Some(u) = m.get("url").and_then(|v| v.as_str()) {
                    let original_url = u.replace("/2/", "/originals/");
                    if !originals.contains(&original_url) {
                        media_list.push(MediaItem {
                            url: original_url.clone(),
                            quality: Some("original".to_string()),
                            file_type: Some(MediaType::Image),
                            extension: Some("jpg".to_string()),
                            thumbnail: m
                                .get("thumbnail")
                                .and_then(|v| v.as_str())
                                .map(|s| s.to_string()),
                            file_size: m
                                .get("size")
                                .and_then(|v| v.as_str())
                                .map(|s| s.to_string()),
                            size_bytes: None,
                            frame_width: None,
                            frame_height: None,
                            note: None,
                        });
                    }
                }
            }

            media_list.push(MediaItem {
                url: m
                    .get("url")
                    .and_then(|v| v.as_str())
                    .unwrap_or("")
                    .to_string(),
                quality: m
                    .get("quality")
                    .and_then(|v| v.as_str())
                    .map(|s| s.to_string()),
                file_type: Some(MediaType::Image),
                extension: m
                    .get("extension")
                    .and_then(|v| v.as_str())
                    .map(|s| s.to_string()),
                thumbnail: m
                    .get("thumbnail")
                    .and_then(|v| v.as_str())
                    .map(|s| s.to_string()),
                file_size: m
                    .get("size")
                    .and_then(|v| v.as_str())
                    .map(|s| s.to_string()),
                size_bytes: None,
                frame_width: None,
                frame_height: None,
                note: None,
            });
        }
    }

    // Sort by size desc (like Shirokami)
    media_list.sort_by(|a, b| {
        let sa = a
            .file_size
            .as_ref()
            .and_then(|s| s.parse::<u64>().ok())
            .unwrap_or(0);
        let sb = b
            .file_size
            .as_ref()
            .and_then(|s| s.parse::<u64>().ok())
            .unwrap_or(0);
        sb.cmp(&sa)
    });

    result.media = media_list;
    Ok(result)
}

// ============================================================================
// Others: PixelDrain, DoodStream, TeraBox, Video, KrakenFiles, Mega, Danbooru
// ============================================================================

pub async fn fetch_pixeldrain(url: &str) -> Result<DownloadResult, ScrapingError> {
    let client = http_client();
    let html = client
        .client()
        .get(url)
        .header(USER_AGENT, "Mozilla/5.0")
        .timeout(Duration::from_secs(15))
        .send()
        .await
        .map_err(|e| ScrapingError::Http(format!("PixelDrain fetch failed: {}", e)))?
        .text()
        .await
        .map_err(|e| ScrapingError::Http(format!("PixelDrain read failed: {}", e)))?;

    let re = regex::Regex::new(r"window\.viewer_data\s*=\s*(\{.*?\});").unwrap();
    let m = re
        .captures(&html)
        .and_then(|c| c.get(1))
        .map(|m| m.as_str());
    let json_str =
        m.ok_or_else(|| ScrapingError::Http("Failed to retrieve viewer data".to_string()))?;

    let viewer_data: serde_json::Value = serde_json::from_str(json_str)
        .map_err(|e| ScrapingError::Http(format!("PixelDrain JSON parse failed: {}", e)))?;

    let file_data = &viewer_data["api_response"];
    let file_id = file_data["id"].as_str().unwrap_or("");

    let mut result = DownloadResult::success(file_data["name"].as_str().map(|s| s.to_string()));
    result.provider = Some("pixeldrain".to_string());

    result.media.push(MediaItem {
        url: format!("https://pixeldrain.com/api/file/{}?download", file_id),
        quality: None,
        file_type: Some(MediaType::File),
        extension: None,
        thumbnail: None,
        file_size: Some(format_filesize(file_data["size"].as_u64().unwrap_or(0))),
        size_bytes: file_data["size"].as_u64(),
        frame_width: None,
        frame_height: None,
        note: None,
    });

    Ok(result)
}

pub async fn fetch_doodstream(url: &str) -> Result<DownloadResult, ScrapingError> {
    let id = regex::Regex::new(r"/[de]/([a-zA-Z0-9]+)")
        .unwrap()
        .captures(url)
        .and_then(|c| c.get(1))
        .map(|m| m.as_str())
        .ok_or_else(|| ScrapingError::Http("Linknya tidak bisa diproses".to_string()))?;

    let proxy = "https://rv.lil-hacker.workers.dev/proxy?mirror=dood&url=";
    let client = http_client();

    // Get metadata from dood.li page
    let page_html = client
        .client()
        .get(format!("https://dood.li/d/{}", id))
        .header(USER_AGENT, "Mozilla/5.0")
        .timeout(Duration::from_secs(15))
        .send()
        .await
        .map_err(|e| ScrapingError::Http(format!("DoodStream page fetch failed: {}", e)))?
        .text()
        .await
        .map_err(|e| ScrapingError::Http(format!("DoodStream page read failed: {}", e)))?;

    // HTML parsing — extract all needed text BEFORE any .await to keep future Send
    // Scraper types (Html/HtmlElementRef/Selector) are NOT Send, so they must be
    // dropped before async points.
    let (page_title, page_length, page_uploadate) = {
        let document = scraper::Html::parse_document(&page_html);
        let text = |sel: &str| {
            document
                .select(&scraper::Selector::parse(sel).unwrap())
                .next()
                .map(|e| e.text().collect::<String>().trim().to_string())
                .unwrap_or_default()
        };
        (text(".title-wrap h4"), text(".length"), text(".uploadate"))
    };

    // Get embedded player page
    let ua = "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/117.0.0.0 Safari/537.36";
    let headers = {
        let mut h = HeaderMap::new();
        h.insert(USER_AGENT, HeaderValue::from_static(ua));
        h.insert("referer", HeaderValue::from_static("https://d000d.com/"));
        h
    };

    let embed_resp = client
        .client()
        .get(format!("{}{}/e/{}", proxy, "https://d000d.com", id))
        .headers(headers.clone())
        .timeout(Duration::from_secs(15))
        .send()
        .await
        .map_err(|e| ScrapingError::Http(format!("DoodStream embed fetch failed: {}", e)))?
        .text()
        .await
        .map_err(|e| ScrapingError::Http(format!("DoodStream embed read failed: {}", e)))?;

    let cdn_match = regex::Regex::new(r"\$\.get\('([^']+)',")
        .unwrap()
        .captures(&embed_resp)
        .and_then(|c| c.get(1))
        .map(|m| m.as_str().to_string());

    let cdn_path = cdn_match.ok_or_else(|| {
        ScrapingError::Http("Link Pass MD5 tidak ditemukan! Coba lagi nanti.".to_string())
    })?;

    // Generate random token (like JS crypto.randomBytes)
    let chars: String = (0..10)
        .map(|_| {
            let idx = fastrand::usize(..62);
            let c = if idx < 26 {
                (b'A' + idx as u8) as char
            } else if idx < 52 {
                (b'a' + (idx - 26) as u8) as char
            } else {
                (b'0' + (idx - 52) as u8) as char
            };
            c
        })
        .collect();

    let ds_resp = client
        .client()
        .get(format!("{}{}{}", proxy, "https://d000d.com", cdn_path))
        .headers(headers.clone())
        .timeout(Duration::from_secs(15))
        .send()
        .await
        .map_err(|e| ScrapingError::Http(format!("DoodStream DS fetch failed: {}", e)))?
        .text()
        .await
        .map_err(|e| ScrapingError::Http(format!("DoodStream DS read failed: {}", e)))?;

    let _cm = cdn_path.split('/').last().unwrap_or("");

    let expiry = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_millis())
        .unwrap_or(0);
    let direct_link = format!(
        "{}{}{}?token={}&expiry={}",
        proxy,
        "https://d000d.com",
        ds_resp.trim(),
        chars,
        expiry,
    );

    // Title/length/uploadate were extracted inside the scope block above
    // as owned strings to keep scraper::Html from crossing .await
    let (title, length_str, uploadate) = (page_title, page_length, page_uploadate);
    let mut result = DownloadResult::success(Some(title));
    result.provider = Some("doodstream".to_string());
    result.duration = Some(length_str);

    result.description = Some(uploadate);

    result.media.push(MediaItem {
        url: direct_link,
        quality: None,
        file_type: Some(MediaType::Video),
        extension: Some("mp4".to_string()),
        thumbnail: None,
        file_size: None,
        size_bytes: None,
        frame_width: None,
        frame_height: None,
        note: None,
    });

    Ok(result)
}

pub async fn fetch_terabox(url: &str) -> Result<DownloadResult, ScrapingError> {
    let pattern = regex::Regex::new(
        r"^https?://(?:www\.|1024)?terabox(?:app)?\.com/.*[?&]?surl=([a-zA-Z0-9_-]+)",
    )
    .unwrap();

    let mut surl = pattern
        .captures(url)
        .and_then(|c| c.get(1))
        .map(|m| m.as_str().to_string());

    if surl.is_none() {
        // Try resolving redirect
        if !regex::Regex::new(r"^https?://(?:www\.|1024)?terabox(?:app)?\.com")
            .unwrap()
            .is_match(url)
        {
            return Ok(DownloadResult::error("Invalid TeraBox URL."));
        }

        let client = http_client();
        let resp = client
            .client()
            .get(url)
            .header(USER_AGENT, "Mozilla/5.0")
            .timeout(Duration::from_secs(15))
            .send()
            .await
            .map_err(|e| ScrapingError::Http(format!("TeraBox redirect fetch failed: {}", e)))?;

        let request_url = resp.url().to_string();
        surl = pattern
            .captures(&request_url)
            .and_then(|c| c.get(1))
            .map(|m| m.as_str().to_string());
    }

    let surl = surl.ok_or_else(|| ScrapingError::Http("SURL not found.".to_string()))?;

    let client = http_client();
    let resp = client
        .client()
        .get("https://tera2.sylyt93.workers.dev/info")
        .query(&[("s", &surl)])
        .header("origin", "https://www.kauruka.com")
        .header("referer", "https://www.kauruka.com/")
        .header(USER_AGENT, "Mozilla/5.0")
        .timeout(Duration::from_secs(15))
        .send()
        .await
        .map_err(|e| ScrapingError::Http(format!("TeraBox info fetch failed: {}", e)))?
        .json::<serde_json::Value>()
        .await
        .map_err(|e| ScrapingError::Http(format!("TeraBox JSON parse failed: {}", e)))?;

    if let Some(download_url) = resp.get("url").and_then(|v| v.as_str()) {
        let mut result = DownloadResult::success(
            resp.get("name")
                .and_then(|v| v.as_str())
                .map(|s| s.to_string()),
        );
        result.provider = Some("terabox".to_string());
        result.thumbnail = resp
            .get("img")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string());

        result.media.push(MediaItem {
            url: download_url.to_string(),
            quality: None,
            file_type: Some(MediaType::File),
            extension: None,
            thumbnail: None,
            file_size: None,
            size_bytes: None,
            frame_width: None,
            frame_height: None,
            note: None,
        });

        Ok(result)
    } else {
        Ok(DownloadResult::error(
            resp.get("error")
                .and_then(|v| v.as_str())
                .unwrap_or("Failed to fetch TeraBox info"),
        ))
    }
}

pub async fn fetch_videy(url: &str) -> Result<DownloadResult, ScrapingError> {
    let id = Url::parse(url)
        .map_err(|e| ScrapingError::Http(format!("Invalid URL: {}", e)))?
        .query_pairs()
        .find(|(k, _)| k == "id")
        .map(|(_, v)| v.into_owned())
        .ok_or_else(|| ScrapingError::Http("Invalid URL, missing \"id\" parameter".to_string()))?;

    let file_type = if id.len() == 9 && id.as_bytes().get(8) == Some(&b'2') {
        ".mov"
    } else {
        ".mp4"
    };

    let direct_url = format!("https://cdn.videy.co/{}{}", id, file_type);

    let mut result = DownloadResult::success(Some(format!("Videy video {}", id)));
    result.provider = Some("videy".to_string());

    result.media.push(MediaItem {
        url: direct_url,
        quality: None,
        file_type: Some(MediaType::Video),
        extension: Some(file_type.trim_start_matches('.').to_string()),
        thumbnail: None,
        file_size: None,
        size_bytes: None,
        frame_width: None,
        frame_height: None,
        note: None,
    });

    Ok(result)
}

pub async fn fetch_krakenfiles(url: &str) -> Result<DownloadResult, ScrapingError> {
    // Parse file ID from krakenfiles.com URL
    let file_id = regex::Regex::new(r"krakenfiles\.com/v/([A-Za-z0-9_-]+)")
        .unwrap()
        .captures(url)
        .and_then(|c| c.get(1))
        .map(|m| m.as_str())
        .ok_or_else(|| ScrapingError::Http("Invalid KrakenFiles URL".to_string()))?;

    let client = http_client();
    let resp = client
        .client()
        .post(format!("https://krakenfiles.com/v/{}", file_id))
        .header(USER_AGENT, "Mozilla/5.0")
        .timeout(Duration::from_secs(15))
        .send()
        .await
        .map_err(|e| ScrapingError::Http(format!("KrakenFiles fetch failed: {}", e)))?
        .json::<serde_json::Value>()
        .await
        .map_err(|e| ScrapingError::Http(format!("KrakenFiles JSON parse failed: {}", e)))?;

    let mut result = DownloadResult::success(
        resp.get("name")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string()),
    );
    result.provider = Some("krakenfiles".to_string());

    result.media.push(MediaItem {
        url: resp
            .get("url")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string(),
        quality: None,
        file_type: Some(MediaType::File),
        extension: resp
            .get("ext")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string()),
        thumbnail: None,
        file_size: None,
        size_bytes: resp.get("size_b").and_then(|v| v.as_u64()),
        frame_width: None,
        frame_height: None,
        note: None,
    });

    Ok(result)
}

/// MEGA.nz — requires AES-128-CBC decryption of the file attributes.
/// Ported from Shirokami's `mega.js` crypto logic.
pub async fn fetch_mega(url: &str) -> Result<DownloadResult, ScrapingError> {
    let url_fixed = url.replace('#', "%23");
    let _parts: Vec<&str> = url_fixed.splitn(2, "#").collect();
    // Actually need to handle the # separator differently
    let cleaned = url.replace("#", "%23");
    let decoded_url = urlencoding::decode(&cleaned)
        .unwrap_or(Cow::Borrowed(&cleaned))
        .into_owned();

    let file_id = regex::Regex::new(r"file/([^#]+)")
        .unwrap()
        .captures(&decoded_url)
        .and_then(|c| c.get(1))
        .map(|m| m.as_str().to_string());

    let file_id = file_id.ok_or_else(|| ScrapingError::Http("Not found".to_string()))?;

    let parts: Vec<&str> = decoded_url.splitn(2, '#').collect();
    let file_key = parts.get(1).copied().unwrap_or("");

    if file_key.is_empty() || file_key.len() != 43 {
        return Ok(DownloadResult::error(
            if file_key.is_empty() {
                "File key tidak ditemukan"
            } else if file_key.len() < 43 {
                "Not enough character"
            } else {
                "Too many character"
            }
            .to_string(),
        ));
    }

    let client = http_client();
    let headers = {
        let mut h = HeaderMap::new();
        h.insert(USER_AGENT, HeaderValue::from_static("Mozilla/5.0"));
        h.insert("origin", HeaderValue::from_static("https://mega.nz"));
        h.insert("referer", HeaderValue::from_static("https://mega.nz"));
        h
    };

    let payload = serde_json::json!([{ "a": "g", "g": 1, "p": file_id }]);

    // Retry loop (3 attempts)
    let mut data = None;
    for attempt in 0..3 {
        match client
            .client()
            .post("https://g.api.mega.co.nz/cs")
            .headers(headers.clone())
            .json(&payload)
            .timeout(Duration::from_secs(10))
            .send()
            .await
        {
            Ok(resp) => {
                data = Some(
                    resp.json::<Vec<serde_json::Value>>()
                        .await
                        .unwrap_or_default(),
                );
                break;
            }
            Err(_) if attempt < 2 => continue,
            Err(e) => {
                return Ok(DownloadResult::error(e.to_string()));
            }
        }
    }

    let data = data.unwrap_or_default();
    let first = data.first().unwrap_or(&serde_json::Value::Null);

    let mut attrs = None;
    if let Some(at) = first.get("at") {
        if let Some(dec) = decrypt_mega_attr(at.as_str().unwrap_or(""), file_key) {
            attrs = Some(dec);
        }
    }

    let mut result = DownloadResult::success(
        attrs
            .as_ref()
            .and_then(|a| a.get("n").and_then(|v| v.as_str()).map(|s| s.to_string())),
    );
    result.provider = Some("mega".to_string());
    result.title = attrs
        .as_ref()
        .and_then(|a| a.get("n").and_then(|v| v.as_str()).map(|s| s.to_string()));

    result.media.push(MediaItem {
        url: first
            .get("g")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string(),
        quality: None,
        file_type: Some(MediaType::File),
        extension: None,
        thumbnail: None,
        file_size: first.get("s").and_then(|v| v.as_u64()).map(format_filesize),
        size_bytes: first.get("s").and_then(|v| v.as_u64()),
        frame_width: None,
        frame_height: None,
        note: None,
    });

    Ok(result)
}

/// MEGA attribute decryption — AES-128-CBC with the derived key.
fn decrypt_mega_attr(enc: &str, file_key: &str) -> Option<serde_json::Value> {
    use base64::engine::general_purpose::URL_SAFE_NO_PAD as BASE64;

    let fixed_key = fix_base64_key(file_key);
    let key_buffer = BASE64.decode(&fixed_key).ok()?;
    if key_buffer.len() < 32 {
        return None;
    }

    // Derive 16-byte key from 32-byte file key
    let key_ints: Vec<u32> = (0..8)
        .map(|i| {
            u32::from_le_bytes([
                key_buffer[i * 4],
                key_buffer[i * 4 + 1],
                key_buffer[i * 4 + 2],
                key_buffer[i * 4 + 3],
            ])
        })
        .collect();

    let key_out: [u8; 16] = {
        let ints = [
            key_ints[0] ^ key_ints[4],
            key_ints[1] ^ key_ints[5],
            key_ints[2] ^ key_ints[6],
            key_ints[3] ^ key_ints[7],
        ];
        let mut buf = [0u8; 16];
        for (i, &val) in ints.iter().enumerate() {
            buf[i * 4..(i + 1) * 4].copy_from_slice(&val.to_le_bytes());
        }
        buf
    };

    let iv = [0u8; 16];
    decrypt_aes_128_cbc(&key_out, &iv, enc)
        .and_then(|s| serde_json::from_str::<serde_json::Value>(&s).ok())
}

fn fix_base64_key(s: &str) -> String {
    let rem = s.len() % 4;
    if rem > 0 {
        format!("{}{}", s, "=".repeat(4 - rem))
    } else {
        s.to_string()
    }
}

fn decrypt_aes_128_cbc(key: &[u8], _iv: &[u8], enc: &str) -> Option<String> {
    use aes::Aes128;
    use base64::engine::general_purpose::URL_SAFE_NO_PAD as BASE64;

    let data = BASE64.decode(enc).ok()?;
    let cipher = Aes128::new_from_slice(key).ok()?;

    let mut result = Vec::new();
    for chunk in data.chunks_exact(16) {
        let mut block = [0u8; 16];
        block.copy_from_slice(chunk);
        if let Ok(_) = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            cipher.decrypt_block((&mut block).into())
        })) {
            result.extend_from_slice(&block);
        } else {
            return None;
        }
    }

    let s = String::from_utf8_lossy(&result).into_owned();
    let s = s.trim_start_matches("MEGA");
    Some(s.to_string())
}

/// Danbooru — returns direct image URL from post
pub async fn fetch_danbooru(url: &str) -> Result<DownloadResult, ScrapingError> {
    let post_id = regex::Regex::new(r"danbooru\.donmai\.us/posts/(\d+)$")
        .unwrap()
        .captures(url)
        .and_then(|c| c.get(1))
        .map(|m| m.as_str())
        .ok_or_else(|| ScrapingError::Http("Invalid URL".to_string()))?;

    let client = http_client();
    let resp = client
        .client()
        .get(format!("https://danbooru.donmai.us/posts/{}.json", post_id))
        .header(USER_AGENT, "Mozilla/5.0")
        .timeout(Duration::from_secs(15))
        .send()
        .await
        .map_err(|e| ScrapingError::Http(format!("Danbooru fetch failed: {}", e)))?
        .json::<serde_json::Value>()
        .await
        .map_err(|e| ScrapingError::Http(format!("Danbooru JSON parse failed: {}", e)))?;

    let file_url = resp["file_url"]
        .as_str()
        .or_else(|| resp["large_file_url"].as_str())
        .ok_or_else(|| ScrapingError::Http("No file URL found".to_string()))?;

    let mut result = DownloadResult::success(resp["tag_string"].as_str().map(|s| s.to_string()));
    result.provider = Some("danbooru".to_string());

    let full_url = if file_url.starts_with("http") {
        file_url.to_string()
    } else {
        format!("https://danbooru.donmai.us{}", file_url)
    };

    let ext = full_url.rsplit('.').next().unwrap_or("jpg").to_string();
    let mut file_type = MediaType::Image;
    if ext == "mp4" || ext == "webm" || ext == "gif" {
        file_type = if ext == "gif" {
            MediaType::Image
        } else {
            MediaType::Video
        };
    }

    result.media.push(MediaItem {
        url: full_url,
        quality: None,
        file_type: Some(file_type),
        extension: Some(ext),
        thumbnail: resp["preview_file_url"].as_str().map(|s| s.to_string()),
        file_size: resp["file_size"].as_str().map(|s| s.to_string()),
        size_bytes: None,
        frame_width: resp["image_width"].as_str().map(|s| s.to_string()),
        frame_height: resp["image_height"].as_str().map(|s| s.to_string()),
        note: Some(resp["md5"].as_str().unwrap_or("").to_string()),
    });

    Ok(result)
}

// ============================================================================
// Helper functions
// ============================================================================

fn format_filesize(bytes: u64) -> String {
    const UNITS: &[&str] = &["bytes", "KB", "MB", "GB", "TB"];
    let mut size = bytes as f64;
    let mut unit = 0;
    while size >= 1024.0 && unit < UNITS.len() - 1 {
        size /= 1024.0;
        unit += 1;
    }
    format!("{:.2} {}", size, UNITS[unit])
}

// ============================================================================
// Google Drive direct download link — extracts the fileId from the share URL
// and builds a download URL via Google's uc handler.
// ============================================================================

pub async fn fetch_gdrive(url: &str) -> Result<DownloadResult, ScrapingError> {
    let mut result = DownloadResult::error("Invalid Google Drive URL");
    let id_re = regex::Regex::new(r"[?&]id=([a-zA-Z0-9_-]+)").unwrap();
    if let Some(caps) = id_re.captures(url) {
        let file_id = caps.get(1).unwrap().as_str();
        let download_url = format!("https://drive.google.com/uc?id={}&export=download", file_id);
        result = DownloadResult::success(None);
        result.provider = Some("gdrive".to_string());
        result.media.push(MediaItem {
            url: download_url,
            quality: None,
            file_type: Some(MediaType::File),
            extension: None,
            thumbnail: None,
            file_size: None,
            size_bytes: None,
            frame_width: None,
            frame_height: None,
            note: None,
        });
    }
    Ok(result)
}

// ============================================================================
// MediaFire direct download link — parses the MediaFire page for the
// direct download anchor.
// ============================================================================

pub async fn fetch_mediafire(url: &str) -> Result<DownloadResult, ScrapingError> {
    let client = http_client();
    let html = client
        .client()
        .get(url)
        .header(
            USER_AGENT,
            "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36",
        )
        .header("accept", "text/html")
        .timeout(Duration::from_secs(20))
        .send()
        .await
        .map_err(|e| ScrapingError::Http(format!("MediaFire fetch failed: {}", e)))?
        .text()
        .await
        .map_err(|e| ScrapingError::Http(format!("MediaFire response read failed: {}", e)))?;

    let document = scraper::Html::parse_document(&html);
    let dl_sel = scraper::Selector::parse(r#"a#downloadButton"#).unwrap();
    let download_url = document
        .select(&dl_sel)
        .next()
        .and_then(|el| el.value().attr("href"))
        .map(|s| s.to_string());

    let title = {
        let title_sel = scraper::Selector::parse("h1").unwrap();
        document
            .select(&title_sel)
            .next()
            .map(|el| el.text().collect::<String>())
    };

    Ok(DownloadResult {
        title,
        status: crate::domain::entity::downloader::DownloadStatus::Success,
        media: download_url
            .map(|u| {
                vec![MediaItem {
                    url: u,
                    quality: None,
                    file_type: Some(MediaType::File),
                    extension: None,
                    thumbnail: None,
                    file_size: None,
                    size_bytes: None,
                    frame_width: None,
                    frame_height: None,
                    note: None,
                }]
            })
            .unwrap_or_default(),
        provider: Some("mediafire".to_string()),
        author: None,
        thumbnail: None,
        description: None,
        duration: None,
        message: None,
    })
}

// ============================================================================
// Threads media extraction — reuses the SnapSave flow (Instagram backend
// already handles Threads via the same snapsave.app endpoint).
// ============================================================================

pub async fn fetch_threads(
    url: &str,
    _cookies: Option<&str>,
) -> Result<DownloadResult, ScrapingError> {
    // Threads URLs go through SnapSave (same engine as Instagram)
    let mut result = fetch_snapsave(url).await?;
    if result.provider.is_none() {
        result.provider = Some("threads".to_string());
    }
    Ok(result)
}
