//! Infrastructure — Weebs endpoints (anime/manga info, waifu pics, whatanime).
//!
//! Ported from Shirokami-API `scraper/weebs/*.js`. These are public JSON
//! APIs that we proxy/forward (jikan.moe, waifu.pics, trace.moe).

use crate::infrastructure::utils::http_client::http_client;
use reqwest::header::USER_AGENT;
use serde_json::{json, Value};

/// Anime info from jikan.moe, ported from anime-search's `getAnimeInfo`.
pub async fn fetch_anime_info(query: &str) -> Result<Value, String> {
    let url = format!("https://api.jikan.moe/v4/anime?q={}", urlencode(query));
    let data = get_json(&url).await?;

    let arr = data
        .get("data")
        .and_then(|d| d.as_array())
        .cloned()
        .unwrap_or_default();
    if arr.is_empty() {
        return Ok(json!({ "error": "Anime tidak ditemukan" }));
    }
    let anime = &arr[0];

    let judul = anime
        .get("titles")
        .and_then(|t| t.as_array())
        .and_then(|t| t.first())
        .and_then(|t| t.get("title"))
        .and_then(|t| t.as_str())
        .unwrap_or("Title not available");
    let genrenya = join_names(anime.get("genres"));

    Ok(json!({
        "title": judul,
        "url": anime.get("url").and_then(|v| v.as_str()).unwrap_or(""),
        "type": anime.get("type").and_then(|v| v.as_str()).unwrap_or(""),
        "score": anime.get("score").cloned().unwrap_or(Value::Null),
        "members": anime.get("members").cloned().unwrap_or(Value::Null),
        "status": anime.get("status").and_then(|v| v.as_str()).unwrap_or(""),
        "synopsis": anime.get("synopsis").and_then(|v| v.as_str()).unwrap_or(""),
        "favorites": anime.get("favorites").cloned().unwrap_or(Value::Null),
        "images": anime.get("images").cloned().unwrap_or(Value::Null),
        "genres": genrenya,
    }))
}

/// Manga info from jikan.moe, ported from anime-search's `getMangaInfo`.
pub async fn fetch_manga_info(query: &str) -> Result<Value, String> {
    let url = format!("https://api.jikan.moe/v4/manga?q={}", urlencode(query));
    let data = get_json(&url).await?;

    let arr = data
        .get("data")
        .and_then(|d| d.as_array())
        .cloned()
        .unwrap_or_default();
    if arr.is_empty() {
        return Ok(json!({ "error": "Manga tidak ditemukan" }));
    }
    let manga = &arr[0];

    let judul = manga
        .get("titles")
        .and_then(|t| t.as_array())
        .map(|tt| {
            tt.iter()
                .filter_map(|j| {
                    let title = j.get("title").and_then(|v| v.as_str()).unwrap_or("");
                    let ty = j.get("type").and_then(|v| v.as_str()).unwrap_or("");
                    if title.is_empty() {
                        None
                    } else {
                        Some(format!("{} [{}]", title, ty))
                    }
                })
                .collect::<Vec<_>>()
                .join(", ")
        })
        .unwrap_or_default();
    let genrenya = join_names(manga.get("genres"));

    Ok(json!({
        "title": judul,
        "chapters": manga.get("chapters").cloned().unwrap_or(Value::Null),
        "type": manga.get("type").and_then(|v| v.as_str()).unwrap_or(""),
        "status": manga.get("status").and_then(|v| v.as_str()).unwrap_or(""),
        "genre": genrenya,
        "volumes": manga.get("volumes").cloned().unwrap_or(Value::Null),
        "favorites": manga.get("favorites").cloned().unwrap_or(Value::Null),
        "score": manga.get("score").cloned().unwrap_or(Value::Null),
        "scored": manga.get("scored").cloned().unwrap_or(Value::Null),
        "scored_by": manga.get("scored_by").cloned().unwrap_or(Value::Null),
        "rank": manga.get("rank").cloned().unwrap_or(Value::Null),
        "popularity": manga.get("popularity").cloned().unwrap_or(Value::Null),
        "members": manga.get("members").cloned().unwrap_or(Value::Null),
        "url": manga.get("url").and_then(|v| v.as_str()).unwrap_or(""),
        "background": manga.get("background").and_then(|v| v.as_str()).unwrap_or(""),
        "synopsis": manga.get("synopsis").and_then(|v| v.as_str()).unwrap_or(""),
    }))
}

/// What anime is in this screenshot? trace.moe reverse-image search.
pub async fn fetch_whatanime(url: &str) -> Result<Value, String> {
    let api = format!(
        "https://api.trace.moe/search?cutBorders&url={}",
        urlencode(url)
    );
    let data = get_json(&api).await?;

    let result = data
        .get("result")
        .and_then(|r| r.as_array())
        .and_then(|r| r.first())
        .cloned();
    let Some(r) = result else {
        return Ok(json!({ "error": "Anime tidak ditemukan" }));
    };

    let similarity = r
        .get("similarity")
        .and_then(|s| s.as_f64())
        .map(|s| (s * 100.0).round() as i64)
        .unwrap_or(0);

    Ok(json!({
        "judul": r.get("filename").and_then(|v| v.as_str()).unwrap_or(""),
        "episode": r.get("episode").and_then(|v| v.as_str()).unwrap_or(""),
        "similarity": similarity,
        "videoURL": r.get("video").and_then(|v| v.as_str()).unwrap_or(""),
        "videoIMG": r.get("image").and_then(|v| v.as_str()).unwrap_or(""),
    }))
}

/// SFW waifu image from waifu.pics.
pub async fn fetch_sfw_waifu(tag: &str) -> Result<Value, String> {
    let url = format!("https://api.waifu.pics/sfw/{}", urlencode(tag));
    get_json(&url).await
}

/// NSFW waifu image from waifu.pics.
pub async fn fetch_nsfw_waifu(tag: &str) -> Result<Value, String> {
    let url = format!("https://api.waifu.pics/nsfw/{}", urlencode(tag));
    get_json(&url).await
}

// ============================================================================
// Helpers
// ============================================================================

async fn get_json(url: &str) -> Result<Value, String> {
    let resp = http_client()
        .client()
        .get(url)
        .header(USER_AGENT, "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/120.0 Safari/537.36")
        .send()
        .await
        .map_err(|e| format!("HTTP error: {}", e))?;
    if !resp.status().is_success() {
        return Err(format!("HTTP {} from upstream", resp.status()));
    }
    resp.json::<Value>()
        .await
        .map_err(|e| format!("JSON parse: {}", e))
}

/// Join an array of { name: "X" } objects into a comma-separated string.
fn join_names(v: Option<&Value>) -> String {
    v.and_then(|g| g.as_array())
        .map(|arr| {
            arr.iter()
                .filter_map(|g| g.get("name").and_then(|n| n.as_str()).map(|s| s.to_string()))
                .collect::<Vec<_>>()
                .join(", ")
        })
        .unwrap_or_default()
}

/// Minimal URL-encode (space -> %20, keep alphanumerics and safe chars).
fn urlencode(s: &str) -> String {
    let mut out = String::new();
    for b in s.bytes() {
        match b {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                out.push(b as char)
            }
            b' ' => out.push_str("%20"),
            _ => out.push_str(&format!("%{:02X}", b)),
        }
    }
    out
}
