# Scraper Downloader API Fix Specification

## Goal
Fix all 15 broken downloader endpoints that return HTTP 500, replace dead third-party APIs with working alternatives, and re-test with real URLs from real platforms.

## Current State Analysis

### Working Endpoints (HTTP 200):
| Platform | Provider | Notes |
|----------|----------|-------|
| TikTok | tikwm.com | ✓ Working (tiktok.com API) |
| MediaFire | mediafire | ✓ Direct page scrape |
| Mega | mega.co.nz | ✓ Valid error messages for bad URLs |
| GDrive | gdrive | ✓ Valid error for missing file ID |
| Pinterest | pinterestdownloader.io | Returns 200 with `{success:false, error:"Unknown error occurred."}` |
| Threads | downr.org | Returns 500 (via fetch_snapsave → fetch_all_in_one) |

### Broken Endpoints (HTTP 500):
| Platform | Upstream API | Error |
|----------|-------------|-------|
| YouTube | media.savetube.me | DNS resolution failure |
| Instagram | downr.org | 403 action_forbidden (Cloudflare) |
| Facebook | downr.org | 403 action_forbidden (Cloudflare) |
| Twitter | savetwitter.net | API returns 404 "Video not found" but code crashes on missing data field |
| Spotify | ytdlpyton.nvlgroup.my.id | DNS resolution failure |
| SoundCloud | ytdlpyton.nvlgroup.my.id | DNS resolution failure |
| PixelDrain | pixeldrain.com | Code crashes parsing viewer_data (wrong regex?) |
| KrakenFiles | krakenfiles.com | API returns 403/empty |
| Danbooru | danbooru.donmai.us | Returns 404 |
| DoodStream | d000d.com | Proxy worker dead |
| TeraBox | tera2.sylyt93.workers.dev | DNS resolution failure |
| Bilibili | cobalt instances | Cobalt DNS failures |

## Root Cause
1. **Error Handling Bug**: `ScrapingError::Http` maps to `AppError::Internal` (HTTP 500) via the `From<ScrapingError>` impl in error.rs. This was already patched to `AppError::ScraperError` → HTTP 502, but **the running binary hasn't been rebuilt**.
2. **Dead Upstream APIs**: Many third-party APIs have moved/changed/blocked requests from VPS IPs.
3. **Error Propagation**: Fetch functions use `.unwrap()` on JSON paths (`resp["data"]`, `resp["download_url"]`) which panics when the upstream returns an unexpected response structure, causing HTTP 500.

## Fix Plan

### Fix 1: Robust error handling (no panics on missing JSON fields)
Replace all `.unwrap()` and direct indexing (`resp["key"]`) with proper `.get()` + error handling. When upstream returns an error response, return `DownloadResult::error("...")` instead of panicking.

### Fix 2: Replace dead upstream APIs

#### YouTube (media.savetube.me → multiple alternatives)
- Primary: Use yt-dlp subprocess via `std::process::Command` (industry standard, handles all platforms)
- Fallback: `https://co.wuk.sh/api/json` (Cobalt instance)
- Fallback: `https://api.tikmate.app/api/lookup` for TikTok

#### Instagram/Facebook (downr.org → alternatives)
- Primary: Use yt-dlp subprocess
- Fallback chain: `snapsave.app/action.php`, `ddownr.com`, `qurls.app`

#### Twitter (savetwitter.net → alternatives)
- Primary: Use twitsave.com (which returned 200 but no MP4 for that test URL)
- Add proper handling for 404 status code from savetwitter.net API
- Fallback: `https://twitsave.com/info?url=...`

#### Spotify (ytdlpyton → alternatives)
- Use Spotify API for metadata + alternative audio source
- Replace with direct Spotify metadata API (already has a fallback to spotify.com API for metadata)

#### Pinterest (pinterestdownloader.io → alternatives)
- Use yt-dlp subprocess
- Fallback: `https://savepin.app`, `https://pin-downloader.com`

#### PixelDrain (fix existing code)
- The regex for extracting viewer_data is wrong - test with real URLs

#### KrakenFiles
- Fix the POST request (the API likely changed)

#### Danbooru
- The API key may be required now

#### DoodStream
- The proxy worker (rv.lil-hacker.workers.dev) is dead - need new proxy

#### TeraBox
- The workers.dev endpoint is dead - find new endpoint

#### Bilibili
- Use working Cobalt instance or the direct API

### Fix 3: Add yt-dlp as universal fallback
Install yt-dlp binary on the VPS. Add a generic download function that shells out to yt-dlp for any platform that fails with its primary API. yt-dlp handles YouTube, IG, FB, Twitter, TikTok, SoundCloud, etc.

### Fix 4: Improve DownloadResult error messages
Ensure every error path returns a descriptive `DownloadResult::error("...")` with platform-specific failure info, not a generic 500.

## Files to Modify
1. `/home/code/scraper/src/presentation/error.rs` - Already patched (ScrapingError → 502)
2. `/home/code/scraper/src/infrastructure/repository/downloader.rs` - Main fix target
3. `/home/code/scraper/Cargo.toml` - Add tokio-process for yt-dlp subprocess
4. `tests/` - Add integration tests with real URLs (non-flaky)

## Verification
- Rebuild and restart the scraper binary
- Test each endpoint with REAL platform URLs
- Verify HTTP status codes are 200 (with success=false data) or 502 (Bad Gateway), never 500
