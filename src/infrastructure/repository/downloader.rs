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
    // Handle short URLs like vm.tiktok.com/ZM8s5qJ6t — resolve redirect first
    if url.contains("vm.tiktok.com") || url.contains("vt.tiktok.com") {
        let re = regex::Regex::new(r"/([A-Za-z0-9_-]+)$").ok()?;
        let short_code = re
            .captures(url)
            .and_then(|c| c.get(1))?
            .as_str()
            .to_string();
        return Some(short_code);
    }
    // TikTok URLs contain an 18-20 digit video ID in the path
    let re = regex::Regex::new(r"/video/(\d{15,25})").ok()?;
    let caps = re.captures(url)?;
    Some(caps.get(1)?.as_str().to_string())
}

/// Locate the yt-dlp binary on the system.
/// Checks: PATH → /home/code/hermes-agent/.venv/bin/yt-dlp → common locations
fn find_ytdlp() -> Option<String> {
    // Check known locations first
    let candidates = [
        "/home/code/hermes-agent/.venv/bin/yt-dlp",
        "/usr/local/bin/yt-dlp",
        "/usr/bin/yt-dlp",
        "/snap/bin/yt-dlp",
        "/home/code/.local/bin/yt-dlp",
    ];
    for c in &candidates {
        if std::path::Path::new(c).exists() {
            return Some(c.to_string());
        }
    }
    // Check PATH
    let paths: Vec<_> = std::env::var_os("PATH")
        .into_iter()
        .flat_map(|p| std::env::split_paths(&p).collect::<Vec<_>>())
        .collect();
    for path in &paths {
        let candidate = path.join("yt-dlp");
        if candidate.exists() {
            return Some(candidate.to_string_lossy().into_owned());
        }
    }
    None
}

/// Run yt-dlp --dump-json and return the parsed JSON value.
/// Uses spawn + manual stdout reading to avoid pipe buffer truncation
/// on large outputs (>64KB on Linux default pipe buffer).
async fn run_ytdlp_json(
    url: &str,
    extra_args: &[&str],
) -> Result<serde_json::Value, ScrapingError> {
    let ytdlp =
        find_ytdlp().ok_or_else(|| ScrapingError::Http("yt-dlp binary not found".to_string()))?;

    let extra_args_owned: Vec<String> = extra_args.iter().map(|s| s.to_string()).collect();

    let (stdout_str, stderr_str, exit_code) = tokio::task::spawn_blocking({
        let url_owned = url.to_string();
        let ytdlp_owned = ytdlp.clone();
        move || {
            let mut cmd_args: Vec<String> = vec![
                "--dump-json".to_string(),
                "--no-warnings".to_string(),
                "--no-check-certificates".to_string(),
                "--user-agent".to_string(),
                "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/120.0.0.0 Safari/537.36".to_string(),
            ];
            cmd_args.extend(extra_args_owned.iter().cloned());
            cmd_args.push(url_owned);

            // Use Stdio::piped() + read_to_string to handle large stdout (64KB+).
            // Command::output() truncates at pipe buffer size.
            let mut child = std::process::Command::new(&ytdlp_owned)
                .args(&cmd_args)
                .stdout(std::process::Stdio::piped())
                .stderr(std::process::Stdio::piped())
                .spawn()
                .map_err(|e| ScrapingError::Http(format!("yt-dlp spawn failed: {}", e)))?;

            let stdout_file = child.stdout.take().unwrap();
            let stderr_file = child.stderr.take().unwrap();
            let mut stdout_str = String::new();
            let mut stderr_str = String::new();
            use std::io::Read;
            let mut stdout_handle = stdout_file;
            stdout_handle.read_to_string(&mut stdout_str).unwrap_or_default();
            let mut stderr_handle = stderr_file;
            stderr_handle.read_to_string(&mut stderr_str).unwrap_or_default();
            let status = child.wait().map_err(|e| ScrapingError::Http(format!("yt-dlp wait failed: {}", e)))?;

            Ok((stdout_str, stderr_str, status.code()))
        }
    })
    .await
    .map_err(|e| ScrapingError::Http(format!("yt-dlp execution failed: {}", e)))??;

    if exit_code != Some(0) {
        return Err(ScrapingError::Http(format!(
            "yt-dlp failed: {}",
            stderr_str.trim().lines().last().unwrap_or("unknown error")
        )));
    }

    // yt-dlp --dump-json outputs one JSON per line per format
    let json_line = stdout_str
        .lines()
        .next()
        .ok_or_else(|| ScrapingError::Http("yt-dlp produced no output".to_string()))?;

    serde_json::from_str(json_line)
        .map_err(|e| ScrapingError::Http(format!("yt-dlp JSON parse failed: {}", e)))
}

/// Run Playwright-based browser scraper as fallback when yt-dlp is blocked.
/// Uses headless Chromium to scrape video URLs from anti-bot-protected sites.
async fn run_playwright_scraper(
    url: &str,
    platform: &str,
) -> Result<serde_json::Value, ScrapingError> {
    let script = env!("CARGO_MANIFEST_DIR");
    let scraper_script = format!("{}/scrape_media.py", script);

    // Find a Python interpreter that has playwright installed.
    // The system `python3` may resolve to a different interpreter for the
    // service user, so probe known venv interpreters first.
    let python_candidates = [
        "/home/code/hermes-agent/.venv/bin/python3",
        "/usr/bin/python3",
        "python3",
    ];
    let python_bin = python_candidates
        .iter()
        .find(|p| {
            std::process::Command::new(p)
                .args(["-c", "import playwright"])
                .stdout(std::process::Stdio::null())
                .stderr(std::process::Stdio::null())
                .status()
                .map(|s| s.success())
                .unwrap_or(false)
        })
        .map(|s| s.to_string())
        .unwrap_or_else(|| "python3".to_string());

    let (stdout_str, stderr_str, exit_code) = tokio::task::spawn_blocking({
        let url_owned = url.to_string();
        let platform_owned = platform.to_string();
        let script_owned = scraper_script.clone();
        let python_owned = python_bin.clone();
        move || -> Result<(String, String, i32), ScrapingError> {
            let output = std::process::Command::new(&python_owned)
                .arg(&script_owned)
                .arg(&url_owned)
                .arg(&platform_owned)
                .stdout(std::process::Stdio::piped())
                .stderr(std::process::Stdio::piped())
                .output()
                .map_err(|e| ScrapingError::Http(format!("playwright spawn failed: {}", e)))?;
            let stdout = String::from_utf8_lossy(&output.stdout).to_string();
            let stderr = String::from_utf8_lossy(&output.stderr).to_string();
            Ok((stdout, stderr, output.status.code().unwrap_or(-1)))
        }
    })
    .await
    .map_err(|e| ScrapingError::Http(format!("playwright task error: {}", e)))??;

    if exit_code != 0 {
        return Err(ScrapingError::Http(format!(
            "playwright scraper failed: {}",
            stderr_str.trim().lines().last().unwrap_or("unknown error")
        )));
    }

    serde_json::from_str(&stdout_str)
        .map_err(|e| ScrapingError::Http(format!("playwright JSON parse failed: {}", e)))
}

/// Convert a Playwright scraper JSON result (from scrape_media.py) into DownloadResult.
fn playwright_to_download_result(data: &serde_json::Value) -> DownloadResult {
    let title = data
        .get("title")
        .and_then(|v| v.as_str())
        .map(|s| s.to_string());
    let mut result = DownloadResult::success(title);
    result.provider = data
        .get("provider")
        .and_then(|v| v.as_str())
        .map(|s| s.to_string());

    if let Some(medias) = data.get("media").and_then(|v| v.as_array()) {
        for m in medias {
            let item = MediaItem {
                url: m
                    .get("url")
                    .and_then(|v| v.as_str())
                    .unwrap_or("")
                    .to_string(),
                quality: None,
                file_type: m.get("ext").and_then(|v| v.as_str()).and_then(|e| match e {
                    "mp4" | "m3u8" => Some(MediaType::Video),
                    "mp3" | "m4a" => Some(MediaType::Audio),
                    _ => Some(MediaType::Video),
                }),
                extension: m.get("ext").and_then(|v| v.as_str()).map(|s| s.to_string()),
                thumbnail: None,
                file_size: None,
                size_bytes: None,
                frame_width: None,
                frame_height: None,
                note: m
                    .get("content_type")
                    .and_then(|v| v.as_str())
                    .map(|s| s.to_string()),
            };
            result.media.push(item);
        }
    }

    if result.media.is_empty() {
        result.message = Some("No download URLs found".to_string());
    }
    result
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
/// Uses downr.org as fallback for robustness, then Playwright browser scraping.
pub(crate) async fn fetch_snapsave(url: &str) -> Result<DownloadResult, ScrapingError> {
    // Validate Instagram/Facebook URL
    let valid_fb = regex::Regex::new(r"https?://(web\.|www\.|m\.)(facebook|fb)\.(com|watch)\S+")
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

    // Try downr.org first
    match fetch_all_in_one(url).await {
        Ok(result) if !result.media.is_empty() => return Ok(result),
        Ok(_) => {}
        Err(e) => {
            eprintln!(
                "downr.org failed for {}: {}, trying Playwright fallback",
                url, e
            );
        }
    }

    // Fallback: use Playwright browser scraping to extract video URLs
    let platform = if valid_ig { "instagram" } else { "facebook" };
    let data = run_playwright_scraper(url, platform).await?;

    // Build result from Playwright scraper output
    let title = data
        .get("title")
        .and_then(|v| v.as_str())
        .map(|s| s.to_string());
    let mut result = DownloadResult::success(title);
    result.provider = data
        .get("provider")
        .and_then(|v| v.as_str())
        .map(|s| s.to_string());

    let media_arr = data.get("media").and_then(|v| v.as_array());
    if let Some(medias) = media_arr {
        for m in medias {
            let item = MediaItem {
                url: m
                    .get("url")
                    .and_then(|v| v.as_str())
                    .unwrap_or("")
                    .to_string(),
                quality: None,
                file_type: m.get("ext").and_then(|v| v.as_str()).and_then(|e| match e {
                    "mp4" | "m3u8" => Some(MediaType::Video),
                    "mp3" | "m4a" => Some(MediaType::Audio),
                    _ => Some(MediaType::Video),
                }),
                extension: m.get("ext").and_then(|v| v.as_str()).map(|s| s.to_string()),
                thumbnail: None,
                file_size: None,
                size_bytes: None,
                frame_width: None,
                frame_height: None,
                note: None,
            };
            result.media.push(item);
        }
    }

    if result.media.is_empty() {
        return Ok(DownloadResult::error(format!(
            "Failed to extract media from {} — server IP may be blocked by anti-bot protection",
            platform
        )));
    }

    Ok(result)
}
pub async fn fetch_tiktok(url: &str) -> Result<DownloadResult, ScrapingError> {
    if extract_tiktok_id(url).is_none() {
        return Ok(DownloadResult::error("Invalid URL"));
    }

    // Try tikwm API first; fall back to yt-dlp if blocked
    let resp = http_client()
        .client()
        .get("https://www.tikwm.com/api/")
        .query(&[("url", url), ("hd", "1")])
        .header(USER_AGENT, "Mozilla/5.0")
        .timeout(Duration::from_secs(15))
        .send()
        .await
        .map_err(|e| ScrapingError::Http(format!("TikTok fetch failed: {}", e)))?;

    let resp_json: serde_json::Value = resp
        .json()
        .await
        .map_err(|e| ScrapingError::Http(format!("TikTok JSON parse failed: {}", e)))?;

    // Check if tikwm returned an error (e.g. Cloudflare blocked)
    let tikwm_code = resp_json.get("code");
    let tikwm_msg = resp_json.get("msg").and_then(|v| v.as_str());

    if tikwm_code == Some(&serde_json::Value::Number(serde_json::Number::from(0)))
        && tikwm_msg != Some("Url parsing is failed! Please check url.")
    {
        let mut result = DownloadResult::success(
            resp_json
                .get("data")
                .and_then(|d| d.get("title"))
                .and_then(|v| v.as_str())
                .map(|s| s.to_string()),
        );
        result.author = resp_json
            .get("data")
            .and_then(|d| d.get("author"))
            .and_then(|v| v.as_str())
            .map(|s| s.to_string());
        result.provider = Some("tikwm".to_string());
        result.thumbnail = resp_json
            .get("data")
            .and_then(|d| d.get("cover"))
            .and_then(|v| v.as_str())
            .map(|s| s.to_string());

        if let Some(plays) = resp_json
            .get("data")
            .and_then(|d| d.get("plays").and_then(|v| v.as_array()))
        {
            for play in plays {
                if let Some(play_url) = play.get("url").and_then(|v| v.as_str()) {
                    result.media.push(MediaItem {
                        url: play_url.to_string(),
                        quality: play
                            .get("quality")
                            .and_then(|v| v.as_str())
                            .map(|s| s.to_string()),
                        file_type: Some(MediaType::Video),
                        extension: Some("mp4".to_string()),
                        thumbnail: None,
                        file_size: play
                            .get("size")
                            .and_then(|v| v.as_str())
                            .map(|s| s.to_string()),
                        size_bytes: play
                            .get("size")
                            .and_then(|v| v.as_str())
                            .and_then(|s| s.parse().ok()),
                        frame_width: None,
                        frame_height: None,
                        note: None,
                    });
                }
            }
        }

        if result.media.is_empty() {
            result.message = Some("No download URLs found".to_string());
            return Ok(result);
        }
        return Ok(result);
    }

    // Fallback: use yt-dlp
    match run_ytdlp_json(url, &[]).await {
        Ok(data) => {
            let title = data
                .get("title")
                .and_then(|v| v.as_str())
                .map(|s| s.to_string());
            let author = data
                .get("uploader")
                .and_then(|v| v.as_str())
                .map(|s| s.to_string());
            let thumbnail = data
                .get("thumbnail")
                .and_then(|v| v.as_str())
                .map(|s| s.to_string());

            let mut result = DownloadResult::success(title);
            result.author = author;
            result.thumbnail = thumbnail;
            result.provider = Some("yt-dlp".to_string());

            if let Some(formats) = data.get("formats").and_then(|v| v.as_array()) {
                for fmt in formats {
                    if let Some(fmt_url) = fmt.get("url").and_then(|v| v.as_str()) {
                        result.media.push(MediaItem {
                            url: fmt_url.to_string(),
                            quality: fmt
                                .get("format_note")
                                .and_then(|v| v.as_str())
                                .map(|s| s.to_string())
                                .or_else(|| {
                                    fmt.get("height")
                                        .and_then(|v| v.as_str())
                                        .map(|s| s.to_string())
                                }),
                            file_type: fmt
                                .get("vcodec")
                                .and_then(|v| v.as_str())
                                .filter(|s| !s.is_empty() && *s != "none")
                                .map(|_| MediaType::Video),
                            extension: fmt
                                .get("ext")
                                .and_then(|v| v.as_str())
                                .map(|s| s.to_string()),
                            thumbnail: None,
                            file_size: fmt
                                .get("filesize")
                                .and_then(|v| v.as_u64())
                                .map(format_filesize),
                            size_bytes: fmt.get("filesize").and_then(|v| v.as_u64()),
                            frame_width: fmt
                                .get("width")
                                .and_then(|v| v.as_str())
                                .map(|s| s.to_string()),
                            frame_height: fmt
                                .get("height")
                                .and_then(|v| v.as_str())
                                .map(|s| s.to_string()),
                            note: None,
                        });
                    }
                }
            }

            if result.media.is_empty() {
                result.message = Some("No download URLs found".to_string());
            }
            return Ok(result);
        }
        Err(e) => {
            eprintln!("yt-dlp failed for TikTok, trying Playwright: {}", e);

            // Fallback: use Playwright browser scraping
            match run_playwright_scraper(url, "tiktok").await {
                Ok(data) => {
                    let title = data
                        .get("title")
                        .and_then(|v| v.as_str())
                        .map(|s| s.to_string());
                    let mut result = DownloadResult::success(title);
                    result.provider = data
                        .get("provider")
                        .and_then(|v| v.as_str())
                        .map(|s| s.to_string());

                    if let Some(medias) = data.get("media").and_then(|v| v.as_array()) {
                        for m in medias {
                            let item = MediaItem {
                                url: m
                                    .get("url")
                                    .and_then(|v| v.as_str())
                                    .unwrap_or("")
                                    .to_string(),
                                quality: None,
                                file_type: m.get("ext").and_then(|v| v.as_str()).and_then(|e| {
                                    match e {
                                        "mp4" | "m3u8" => Some(MediaType::Video),
                                        "mp3" | "m4a" => Some(MediaType::Audio),
                                        _ => Some(MediaType::Video),
                                    }
                                }),
                                extension: m
                                    .get("ext")
                                    .and_then(|v| v.as_str())
                                    .map(|s| s.to_string()),
                                thumbnail: None,
                                file_size: None,
                                size_bytes: None,
                                frame_width: None,
                                frame_height: None,
                                note: None,
                            };
                            result.media.push(item);
                        }
                    }

                    if result.media.is_empty() {
                        result.message = Some("No download URLs found".to_string());
                    }
                    return Ok(result);
                }
                Err(_) => {
                    // All methods failed
                    return Err(ScrapingError::Http(
                        "All TikTok download methods failed (tikwm API blocked, yt-dlp blocked, Playwright blocked)".to_string()
                    ));
                }
            }
        }
    }
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
    extract_youtube_id(url)
        .ok_or_else(|| ScrapingError::Http("Invalid YouTube URL".to_string()))?;

    let data = run_ytdlp_json(url, &["-f", "bestaudio/best"]).await?;

    let title = data
        .get("title")
        .and_then(|v| v.as_str())
        .map(|s| s.to_string());
    let author = data
        .get("uploader")
        .and_then(|v| v.as_str())
        .map(|s| s.to_string());
    let thumbnail = data
        .get("thumbnail")
        .and_then(|v| v.as_str())
        .map(|s| s.to_string());
    let duration = data
        .get("duration")
        .and_then(|v| v.as_u64())
        .map(|d| format!("{}s", d));

    let mut result = DownloadResult::success(title);
    result.author = author;
    result.thumbnail = thumbnail;
    result.duration = duration;
    result.provider = Some("yt-dlp".to_string());

    let download_url = data
        .get("url")
        .and_then(|v| v.as_str())
        .or_else(|| {
            data.get("formats")
                .and_then(|v| v.as_array())
                .and_then(|f| f.first())
                .and_then(|f| f.get("url").and_then(|v| v.as_str()))
        })
        .ok_or_else(|| ScrapingError::Http("No audio format URL from yt-dlp".to_string()))?;

    result.media.push(MediaItem {
        url: download_url.to_string(),
        quality: Some(format!(
            "{}kbps",
            data.get("abr").and_then(|v| v.as_u64()).unwrap_or(128)
        )),
        file_type: Some(MediaType::Audio),
        extension: Some("mp3".to_string()),
        thumbnail: data
            .get("thumbnail")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string()),
        file_size: data
            .get("filesize")
            .and_then(|v| v.as_u64())
            .map(format_filesize),
        size_bytes: data.get("filesize").and_then(|v| v.as_u64()),
        frame_width: None,
        frame_height: None,
        note: None,
    });

    Ok(result)
}

/// YouTube video via yt-dlp
pub async fn fetch_youtube_mp4(url: &str, quality: &str) -> Result<DownloadResult, ScrapingError> {
    extract_youtube_id(url)
        .ok_or_else(|| ScrapingError::Http("Invalid YouTube URL".to_string()))?;

    let fmt = format!(
        "bestvideo[height<={}]/best+bestaudio/best",
        quality.trim_end_matches('p')
    );
    let data = run_ytdlp_json(url, &["-f", &fmt]).await?;

    let title = data
        .get("title")
        .and_then(|v| v.as_str())
        .map(|s| s.to_string());
    let author = data
        .get("uploader")
        .and_then(|v| v.as_str())
        .map(|s| s.to_string());
    let thumbnail = data
        .get("thumbnail")
        .and_then(|v| v.as_str())
        .map(|s| s.to_string());
    let duration = data
        .get("duration")
        .and_then(|v| v.as_u64())
        .map(|d| format!("{}s", d));

    let mut result = DownloadResult::success(title);
    result.author = author;
    result.thumbnail = thumbnail;
    result.duration = duration;
    result.provider = Some("yt-dlp".to_string());

    let formats = data.get("formats").and_then(|v| v.as_array());
    let mut seen = std::collections::HashSet::new();

    if let Some(fmts) = formats {
        for f in fmts {
            if let (Some(furl), Some(ext_val)) = (
                f.get("url").and_then(|v| v.as_str()),
                f.get("ext").and_then(|v| v.as_str()),
            ) {
                if ext_val == "mhtml" {
                    continue;
                }
                let url_string = furl.to_string();
                if seen.insert(url_string.clone()) {
                    let fmt_type = if ext_val == "mp4" || ext_val == "webm" || ext_val == "mkv" {
                        MediaType::Video
                    } else if ext_val == "mp3" || ext_val == "m4a" || ext_val == "webm" {
                        MediaType::Audio
                    } else {
                        MediaType::File
                    };
                    let q = format!("{}p", f.get("height").and_then(|v| v.as_u64()).unwrap_or(0));
                    result.media.push(MediaItem {
                        url: url_string,
                        quality: Some(q),
                        file_type: Some(fmt_type),
                        extension: Some(ext_val.to_string()),
                        thumbnail: data
                            .get("thumbnail")
                            .and_then(|v| v.as_str())
                            .map(|s| s.to_string()),
                        file_size: f
                            .get("filesize")
                            .and_then(|v| v.as_u64())
                            .map(format_filesize),
                        size_bytes: f.get("filesize").and_then(|v| v.as_u64()),
                        frame_width: f
                            .get("width")
                            .and_then(|v| v.as_u64())
                            .map(|w| w.to_string()),
                        frame_height: f
                            .get("height")
                            .and_then(|v| v.as_u64())
                            .map(|h| h.to_string()),
                        note: None,
                    });
                }
            }
        }
    }

    if result.media.is_empty() {
        if let Some(url) = data.get("url").and_then(|v| v.as_str()) {
            result.media.push(MediaItem {
                url: url.to_string(),
                quality: Some(format!("{}p", quality.trim_end_matches('p'))),
                file_type: Some(MediaType::Video),
                extension: Some(
                    data.get("ext")
                        .and_then(|v| v.as_str())
                        .unwrap_or("mp4")
                        .to_string(),
                ),
                thumbnail: data
                    .get("thumbnail")
                    .and_then(|v| v.as_str())
                    .map(|s| s.to_string()),
                file_size: data
                    .get("filesize")
                    .and_then(|v| v.as_u64())
                    .map(format_filesize),
                size_bytes: data.get("filesize").and_then(|v| v.as_u64()),
                frame_width: None,
                frame_height: None,
                note: None,
            });
        }
    }

    Ok(result)
}

// Spotify downloaders
// ============================================================================

pub async fn fetch_spotify(url: &str) -> Result<DownloadResult, ScrapingError> {
    // Validate Spotify URL
    let re = regex::Regex::new(
        r"https?://open\.spotify\.com/(?:intl-[a-zA-Z0-9-]+/)?(track|album|playlist)/([a-zA-Z0-9]+)",
    )
    .unwrap();
    let captures = re
        .captures(url)
        .ok_or_else(|| ScrapingError::Http("Invalid Spotify URL".to_string()))?;
    let resource_type = captures.get(1).map(|m| m.as_str()).unwrap_or("track");
    let resource_id = captures.get(2).map(|m| m.as_str()).unwrap_or("");

    // Use yt-dlp to extract track info and download URL
    let data = run_ytdlp_json(url, &["--extract-audio", "--audio-format", "mp3"]).await?;

    let title = data
        .get("title")
        .and_then(|v| v.as_str())
        .map(|s| s.to_string());
    let author = data
        .get("artist")
        .and_then(|v| v.as_str())
        .map(|s| s.to_string());
    let thumbnail = data
        .get("thumbnail")
        .and_then(|v| v.as_str())
        .map(|s| s.to_string());
    let duration = data
        .get("duration")
        .and_then(|v| v.as_u64())
        .map(|d| format!("{}s", d));

    let mut result = DownloadResult::success(title);
    result.author = author;
    result.thumbnail = thumbnail;
    result.duration = duration;
    result.provider = Some(format!("spotify-yt-dlp ({})", resource_type));

    let download_url = data.get("url").and_then(|v| v.as_str()).or_else(|| {
        data.get("formats")
            .and_then(|v| v.as_array())
            .and_then(|f| f.first())
            .and_then(|f| f.get("url").and_then(|v| v.as_str()))
    });

    if let Some(dl_url) = download_url {
        result.media.push(MediaItem {
            url: dl_url.to_string(),
            quality: Some("320kbps".to_string()),
            file_type: Some(MediaType::Audio),
            extension: Some("mp3".to_string()),
            thumbnail: data
                .get("thumbnail")
                .and_then(|v| v.as_str())
                .map(|s| s.to_string()),
            file_size: data
                .get("filesize")
                .and_then(|v| v.as_u64())
                .map(format_filesize),
            size_bytes: data.get("filesize").and_then(|v| v.as_u64()),
            frame_width: None,
            frame_height: None,
            note: None,
        });
        Ok(result)
    } else {
        result.media.push(MediaItem {
            url: format!("https://open.spotify.com/track/{}", resource_id),
            quality: Some("metadata".to_string()),
            file_type: Some(MediaType::File),
            extension: Some("json".to_string()),
            thumbnail: data.get("thumbnail").and_then(|v| v.as_str()).map(|s| s.to_string()),
            file_size: None,
            size_bytes: None,
            frame_width: None,
            frame_height: None,
            note: Some("Spotify track metadata extracted via yt-dlp. Direct audio download requires Spotify Premium.".to_string()),
        });
        Ok(result)
    }
}

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

    let html = resp.get("data").and_then(|v| v.as_str()).ok_or_else(|| {
        let msg = resp
            .get("msg")
            .and_then(|v| v.as_str())
            .unwrap_or("No data in Twitter response");
        ScrapingError::Http(msg.to_string())
    })?;

    // Parse savetwitter HTML inside a block so all scraper types
    // (Html/Selector/ElementRef are NOT Send) go out of scope before the
    // Playwright fallback .await below keeps the future Send.
    let mut parsed_media: Vec<(String, Option<String>, Option<MediaType>, Option<String>)> =
        Vec::new();
    {
        let document = scraper::Html::parse_document(html);
        let tw_video_sel = scraper::Selector::parse("div.tw-video").unwrap();

        if document.select(&tw_video_sel).next().is_some() {
            if let Ok(item_sel) =
                scraper::Selector::parse("div.tw-right > div > p:nth-child(1) > a")
            {
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
                    parsed_media.push((
                        href,
                        Some(quality),
                        Some(MediaType::Video),
                        Some("mp4".to_string()),
                    ));
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
                        parsed_media.push((
                            href,
                            None,
                            Some(MediaType::Image),
                            Some("jpg".to_string()),
                        ));
                    }
                }
            }
        }
    } // document + selectors dropped here

    let mut result = DownloadResult::success(None);
    result.provider = Some("savetwitter".to_string());
    for (href, quality, file_type, extension) in parsed_media {
        result.media.push(MediaItem {
            url: href,
            quality,
            file_type,
            extension,
            thumbnail: None,
            file_size: None,
            size_bytes: None,
            frame_width: None,
            frame_height: None,
            note: None,
        });
    }

    if result.media.is_empty() {
        // Fallback: Playwright browser scraping for Twitter video URLs
        match run_playwright_scraper(url, "twitter").await {
            Ok(data) => {
                let pw_result = playwright_to_download_result(&data);
                if !pw_result.media.is_empty() {
                    return Ok(pw_result);
                }
            }
            Err(_e) => {}
        }
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
    // yt-dlp supports both AV (/video/av123) and BV (/video/BV1xxx) IDs
    let data = run_ytdlp_json(url, &["-f", "bv*+ba/b"]).await?;

    let title = data
        .get("title")
        .and_then(|v| v.as_str())
        .map(|s| s.to_string());
    let author = data
        .get("uploader")
        .and_then(|v| v.as_str())
        .map(|s| s.to_string());
    let thumbnail = data
        .get("thumbnail")
        .and_then(|v| v.as_str())
        .map(|s| s.to_string());
    let duration = data
        .get("duration")
        .and_then(|v| v.as_u64())
        .map(|d| format!("{}s", d));

    let mut result = DownloadResult::success(title);
    result.author = author;
    result.thumbnail = thumbnail;
    result.duration = duration;
    result.provider = Some("yt-dlp".to_string());

    // Extract media from formats array
    if let Some(formats) = data.get("formats").and_then(|v| v.as_array()) {
        for fmt in formats {
            let url = fmt.get("url").and_then(|v| v.as_str());
            if url.is_some() {
                result.media.push(MediaItem {
                    url: url.unwrap().to_string(),
                    quality: fmt
                        .get("format_note")
                        .and_then(|v| v.as_str())
                        .map(|s| s.to_string())
                        .or_else(|| {
                            fmt.get("height")
                                .and_then(|v| v.as_str())
                                .map(|s| s.to_string())
                        }),
                    file_type: fmt
                        .get("vcodec")
                        .and_then(|v| v.as_str())
                        .filter(|s| !s.is_empty() && *s != "none")
                        .map(|_| MediaType::Video),
                    extension: fmt
                        .get("ext")
                        .and_then(|v| v.as_str())
                        .map(|s| s.to_string()),
                    thumbnail: None,
                    file_size: fmt
                        .get("filesize")
                        .and_then(|v| v.as_u64())
                        .map(format_filesize),
                    size_bytes: fmt.get("filesize").and_then(|v| v.as_u64()),
                    frame_width: fmt
                        .get("width")
                        .and_then(|v| v.as_str())
                        .map(|s| s.to_string()),
                    frame_height: fmt
                        .get("height")
                        .and_then(|v| v.as_str())
                        .map(|s| s.to_string()),
                    note: None,
                });
            }
        }
    }

    Ok(result)
}

// ============================================================================
// SoundCloud downloader
// ============================================================================

pub async fn fetch_soundcloud(url: &str) -> Result<DownloadResult, ScrapingError> {
    // yt-dlp --extract-audio requires a download; use --dump-json for direct URL
    let data = run_ytdlp_json(url, &[]).await?;

    let title = data
        .get("title")
        .and_then(|v| v.as_str())
        .map(|s| s.to_string());
    let author = data
        .get("uploader")
        .and_then(|v| v.as_str())
        .map(|s| s.to_string());
    let thumbnail = data
        .get("thumbnail")
        .and_then(|v| v.as_str())
        .map(|s| s.to_string());
    let duration = data
        .get("duration")
        .and_then(|v| v.as_u64())
        .map(|d| format!("{}s", d));

    let mut result = DownloadResult::success(title);
    result.author = author;
    result.thumbnail = thumbnail;
    result.duration = duration;
    result.provider = Some("yt-dlp".to_string());

    let download_url = data.get("url").and_then(|v| v.as_str()).or_else(|| {
        data.get("formats")
            .and_then(|v| v.as_array())
            .and_then(|f| f.first())
            .and_then(|f| f.get("url").and_then(|v| v.as_str()))
    });

    if let Some(dl_url) = download_url {
        result.media.push(MediaItem {
            url: dl_url.to_string(),
            quality: Some(format!(
                "{}kbps",
                data.get("abr").and_then(|v| v.as_u64()).unwrap_or(128)
            )),
            file_type: Some(MediaType::Audio),
            extension: Some("mp3".to_string()),
            thumbnail: data
                .get("thumbnail")
                .and_then(|v| v.as_str())
                .map(|s| s.to_string()),
            file_size: data
                .get("filesize")
                .and_then(|v| v.as_u64())
                .map(format_filesize),
            size_bytes: data.get("filesize").and_then(|v| v.as_u64()),
            frame_width: None,
            frame_height: None,
            note: None,
        });
    }

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
        .header("accept", "text/html,application/xhtml+xml")
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
