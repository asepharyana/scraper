//! Infrastructure — Stalk utilities (github, youtube, instagram, tiktok, twitter).
//!
//! Ported from Shirokami-API `scraper/stalk/*.js`.

use crate::infrastructure::utils::http_client::http_client;
use regex::Regex;
use reqwest::header::USER_AGENT;
use serde_json::{json, Value};

/// GitHub user stalk via public API.
pub async fn fetch_github_stalk(username: &str) -> Result<Value, String> {
    let url = format!("https://api.github.com/users/{}", username);

    let resp = http_client()
        .client()
        .get(&url)
        .header(USER_AGENT, "Scraper/1.0")
        .send()
        .await
        .map_err(|e| format!("HTTP error: {}", e))?;

    if !resp.status().is_success() {
        let status = resp.status().as_u16();
        return Err(format!("GitHub user not found (HTTP {})", status));
    }

    let data: Value = resp
        .json()
        .await
        .map_err(|e| format!("JSON parse: {}", e))?;

    let pick = |k: &str| -> Value { data.get(k).cloned().unwrap_or(Value::Null) };

    Ok(json!({
        "username": username,
        "login": pick("login"),
        "id": pick("id"),
        "node_id": pick("node_id"),
        "avatar_url": pick("avatar_url"),
        "html_url": pick("html_url"),
        "type": pick("type"),
        "site_admin": pick("site_admin"),
        "name": pick("name"),
        "company": pick("company"),
        "blog": pick("blog"),
        "location": pick("location"),
        "email": pick("email"),
        "hireable": pick("hireable"),
        "bio": pick("bio"),
        "twitter_username": pick("twitter_username"),
        "public_repos": pick("public_repos"),
        "public_gists": pick("public_gists"),
        "followers": pick("followers"),
        "following": pick("following"),
        "created_at": pick("created_at"),
        "updated_at": pick("updated_at"),
    }))
}

/// YouTube channel stalk: parse ytInitialData JSON from channel page.
/// Fetches the `/<handle>/videos` subpage (richGridRenderer with lockupViewModel).
pub async fn fetch_youtube_stalk(username: &str) -> Result<Value, String> {
    let url = format!("https://youtube.com/@{username}/videos?hl=en&gl=US");

    let resp = http_client()
        .client()
        .get(&url)
        .header(USER_AGENT, "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/120.0 Safari/537.36")
        .header("Accept-Language", "en-US")
        .send()
        .await
        .map_err(|e| format!("HTTP error: {}", e))?;

    if !resp.status().is_success() {
        return Err(format!("HTTP {} from youtube", resp.status()));
    }

    let body = resp
        .text()
        .await
        .map_err(|e| format!("Failed to read body: {}", e))?;

    // Extract var ytInitialData = {...} via brace matching (JSON may contain ";").
    let start = body
        .find("var ytInitialData = ")
        .ok_or_else(|| "ytInitialData not found".to_string())?;
    let brace = body[start..]
        .find('{')
        .ok_or_else(|| "ytInitialData parse".to_string())?
        + start;
    let json_str = extract_balanced_json(&body, brace)?;
    let parsed: Value =
        serde_json::from_str(&json_str).map_err(|e| format!("JSON parse: {}", e))?;

    // Channel metadata from metadata.channelMetadataRenderer
    let mut channel = json!({
        "username": username,
        "subscriberCount": Value::Null,
        "videoCount": Value::Null,
        "avatarUrl": Value::Null,
        "channelUrl": Value::Null,
        "externalId": Value::Null,
        "description": Value::Null,
        "isFamilySafe": Value::Null,
    });

    if let Some(meta) = parsed
        .get("metadata")
        .and_then(|m| m.get("channelMetadataRenderer"))
    {
        let mut set = |dst: &str, src: &str| {
            if let Some(v) = meta.get(src) {
                channel[dst] = v.clone();
            }
        };
        set("description", "description");
        set("externalId", "externalId");
        set("channelUrl", "channelUrl");
        set("isFamilySafe", "isFamilySafe");
    }

    // Subscriber/video count from header.pageHeaderRenderer
    if let Some(header) = parsed
        .get("header")
        .and_then(|h| h.get("pageHeaderRenderer"))
    {
        if let Some(rows) = header
            .pointer("/content/pageHeaderViewModel/metadata/contentMetadataViewModel/metadataRows")
        {
            if let Some(arr) = rows.as_array() {
                for row in arr {
                    if let Some(parts) = row.get("metadataParts").and_then(|p| p.as_array()) {
                        for part in parts {
                            let text = part
                                .pointer("/text/content")
                                .and_then(|t| t.as_str())
                                .unwrap_or("");
                            if text.contains("subscriber") {
                                channel["subscriberCount"] = json!(text.trim());
                            } else if text.contains(" video") || text.ends_with(" videos") {
                                channel["videoCount"] = json!(text.trim());
                            }
                        }
                    }
                }
            }
        }
        // Avatar
        if let Some(sources) = header
            .pointer("/content/pageHeaderViewModel/image/decoratedAvatarViewModel/avatar/avatarViewModel/image/sources")
            .and_then(|s| s.as_array())
        {
            if !sources.is_empty() {
                channel["avatarUrl"] = sources[0].get("url").cloned().unwrap_or(Value::Null);
            }
        }
    }

    // Videos list (up to 5). The `/videos` subpage uses richGridRenderer with
    // lockupViewModel (new) or videoRenderer (legacy); we handle both.
    let mut videos: Vec<Value> = Vec::new();

    if let Some(tabs) = parsed
        .pointer("/contents/twoColumnBrowseResultsRenderer/tabs")
        .and_then(|t| t.as_array())
    {
        for tab in tabs {
            walk_video_grid(tab, &mut videos);
            if videos.len() >= 5 {
                break;
            }
        }
    }

    // Fallback: source's shelfRenderer/horizontalListRenderer approach.
    if videos.is_empty() {
        if let Some(tabs) = parsed
            .pointer("/contents/twoColumnBrowseResultsRenderer/tabs")
            .and_then(|t| t.as_array())
        {
            if let Some(contents) = tabs
                .get(0)
                .and_then(|t| t.pointer("/tabRenderer/content/sectionListRenderer/contents"))
                .and_then(|c| c.as_array())
            {
                for item in contents {
                    if let Some(section) = item.get("itemSectionRenderer") {
                        if let Some(inner) = section.get("contents").and_then(|c| c.as_array()) {
                            for content in inner {
                                if let Some(shelf) = content.get("shelfRenderer") {
                                    if let Some(items) = shelf
                                        .pointer("/content/horizontalListRenderer/items")
                                        .and_then(|i| i.as_array())
                                    {
                                        for video in items {
                                            if let Some(gv) = video.get("gridVideoRenderer") {
                                                videos.push(video_from_grid_video(gv));
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                    if videos.len() >= 5 {
                        break;
                    }
                }
            }
        }
    }

    Ok(json!({
        "channelMetadata": channel,
        "videoDataList": videos.iter().take(5).cloned().collect::<Vec<_>>(),
    }))
}

/// Extract a balanced JSON object string starting at `open_brace` (index of '{').
fn extract_balanced_json(body: &str, open_brace: usize) -> Result<String, String> {
    let bytes = body.as_bytes();
    let mut depth = 0i32;
    let mut in_str = false;
    let mut escaped = false;
    let mut close = None;
    for i in open_brace..bytes.len() {
        let c = bytes[i] as char;
        if in_str {
            if escaped {
                escaped = false;
            } else if c == '\\' {
                escaped = true;
            } else if c == '"' {
                in_str = false;
            }
        } else {
            match c {
                '"' => in_str = true,
                '{' => depth += 1,
                '}' => {
                    depth -= 1;
                    if depth == 0 {
                        close = Some(i);
                        break;
                    }
                }
                _ => {}
            }
        }
    }
    let close = close.ok_or_else(|| "unbalanced JSON in ytInitialData".to_string())?;
    Ok(body[open_brace..=close].to_string())
}

/// Recursively find `richItemRenderer` entries and convert their content to video objects.
fn walk_video_grid(node: &Value, out: &mut Vec<Value>) {
    match node {
        Value::Object(map) => {
            if let Some(item) = map.get("richItemRenderer") {
                if let Some(content) = item.get("content") {
                    if let Some(v) = content.get("videoRenderer") {
                        out.push(video_from_video_renderer(v));
                    } else if let Some(lv) = content.get("lockupViewModel") {
                        out.push(video_from_lockup(lv));
                    } else if let Some(sh) = content.get("shortsLockupViewModel") {
                        out.push(video_from_shorts(sh));
                    }
                }
            }
            for v in map.values() {
                if out.len() >= 5 {
                    return;
                }
                walk_video_grid(v, out);
            }
        }
        Value::Array(arr) => {
            for v in arr {
                if out.len() >= 5 {
                    return;
                }
                walk_video_grid(v, out);
            }
        }
        _ => {}
    }
}

/// Parse a legacy `videoRenderer` into our video shape.
fn video_from_video_renderer(vd: &Value) -> Value {
    let mut v = json!({
        "videoId": vd.get("videoId").cloned().unwrap_or(Value::Null),
        "title": vd.pointer("/title/runs/0/text").cloned().unwrap_or(Value::Null),
        "thumbnail": vd.pointer("/thumbnail/thumbnails/0/url").cloned().unwrap_or(Value::Null),
        "publishedTime": vd.pointer("/publishedTimeText/simpleText").cloned().unwrap_or(Value::Null),
        "viewCount": vd.pointer("/viewCountText/simpleText").cloned().unwrap_or(Value::Null),
        "navigationUrl": Value::Null,
        "duration": Value::Null,
    });
    if let Some(url) = vd.pointer("/navigationEndpoint/commandMetadata/webCommandMetadata/url") {
        v["navigationUrl"] = url.clone();
    }
    if let Some(dur) = vd
        .pointer("/thumbnailOverlays")
        .and_then(|o| o.as_array())
        .and_then(|arr| {
            arr.iter().find_map(|ov| {
                ov.pointer("/thumbnailOverlayTimeStatusRenderer/text/simpleText")
                    .cloned()
            })
        })
    {
        v["duration"] = dur;
    }
    v
}

/// Parse a `lockupViewModel` (newer YouTube renderer) into our video shape.
fn video_from_lockup(lv: &Value) -> Value {
    // videoId from thumbnail URL `i.ytimg.com/vi/<ID>/...`
    let thumb = lv
        .pointer("/contentImage/thumbnailViewModel/image/sources/0/url")
        .and_then(|u| u.as_str())
        .unwrap_or("");
    let video_id = Regex::new(r"vi/([A-Za-z0-9_-]{11})/")
        .ok()
        .and_then(|re| re.captures(thumb))
        .and_then(|c| c.get(1))
        .map(|m| m.as_str().to_string())
        .unwrap_or_default();

    let title = lv
        .pointer("/metadata/lockupMetadataViewModel/title/content")
        .and_then(|t| t.as_str())
        .unwrap_or("")
        .to_string();

    let mut view_count = Value::Null;
    let mut published = Value::Null;
    if let Some(parts) = lv
        .pointer("/metadata/lockupMetadataViewModel/metadata/contentMetadataViewModel/metadataRows/0/metadataParts")
        .and_then(|p| p.as_array())
    {
        if let Some(p0) = parts.get(0) {
            view_count = p0
                .pointer("/text/content")
                .cloned()
                .unwrap_or(Value::Null);
        }
        if let Some(p1) = parts.get(1) {
            published = p1
                .pointer("/text/content")
                .cloned()
                .unwrap_or(Value::Null);
        }
    }

    let duration = lv
        .pointer("/contentImage/thumbnailViewModel/image/overlays/0/thumbnailBottomOverlayViewModel/badges/0/thumbnailBadgeViewModel/text")
        .cloned()
        .unwrap_or(Value::Null);

    json!({
        "videoId": json!(video_id),
        "title": json!(title),
        "thumbnail": json!(thumb),
        "publishedTime": published,
        "viewCount": view_count,
        "navigationUrl": Value::Null,
        "duration": duration,
    })
}

/// Parse a `shortsLockupViewModel` (Shorts) into our video shape (best-effort).
fn video_from_shorts(sh: &Value) -> Value {
    let video_id = sh
        .pointer("/overlayMetadata/primaryText/content")
        .and_then(|c| c.as_str())
        .unwrap_or("");
    json!({
        "videoId": json!(video_id),
        "title": Value::Null,
        "thumbnail": Value::Null,
        "publishedTime": Value::Null,
        "viewCount": Value::Null,
        "navigationUrl": Value::Null,
        "duration": Value::Null,
    })
}

/// Parse a legacy `gridVideoRenderer` into our video shape.
fn video_from_grid_video(gv: &Value) -> Value {
    let mut v = json!({
        "videoId": gv.get("videoId").cloned().unwrap_or(Value::Null),
        "title": gv.pointer("/title/simpleText").cloned().unwrap_or(Value::Null),
        "thumbnail": gv.pointer("/thumbnail/thumbnails/0/url").cloned().unwrap_or(Value::Null),
        "publishedTime": gv.pointer("/publishedTimeText/simpleText").cloned().unwrap_or(Value::Null),
        "viewCount": gv.pointer("/viewCountText/simpleText").cloned().unwrap_or(Value::Null),
        "navigationUrl": Value::Null,
        "duration": Value::Null,
    });
    if let Some(nav) = gv.pointer("/navigationEndpoint/commandMetadata/webCommandMetadata/url") {
        v["navigationUrl"] = nav.clone();
    }
    if let Some(dur) = gv
        .pointer("/thumbnailOverlays")
        .and_then(|o| o.as_array())
        .and_then(|arr| {
            arr.iter().find_map(|ov| {
                ov.pointer("/thumbnailOverlayTimeStatusRenderer/text/simpleText")
                    .cloned()
            })
        })
    {
        v["duration"] = dur;
    }
    v
}

// ---------------------------------------------------------------------------
// Mobile Legends — gempaytopup.com stalk (CSRF token + POST)
// ---------------------------------------------------------------------------

pub async fn fetch_ml_stalk(user_id: &str, zone_id: &str) -> Result<Value, String> {
    // 1. GET to obtain CSRF token + cookies
    let resp = http_client()
        .client()
        .get("https://www.gempaytopup.com")
        .header(
            USER_AGENT,
            "Mozilla/5.0 (Windows NT 10.0; Win64; x64) Chrome/120.0 Safari/537.36",
        )
        .send()
        .await
        .map_err(|e| format!("HTTP: {}", e))?;
    let cookies = resp
        .headers()
        .get_all("set-cookie")
        .iter()
        .map(|v| {
            v.to_str()
                .unwrap_or("")
                .split(';')
                .next()
                .unwrap_or("")
                .to_string()
        })
        .collect::<Vec<_>>()
        .join("; ");
    let body = resp.text().await.map_err(|e| format!("Body: {}", e))?;

    let csrf_re = Regex::new(r#"<meta name="csrf-token" content="(.*?)">"#).unwrap();
    let csrf = csrf_re
        .captures(&body)
        .and_then(|c| c.get(1))
        .map(|m| m.as_str().to_string());

    let (csrf, cookies) = match (csrf, cookies.is_empty()) {
        (Some(c), false) => (c, cookies),
        _ => return Ok(json!({"error": "Gagal mendapatkan CSRF token atau cookie."})),
    };

    // 2. POST stalk-ml
    let payload = json!({"uid": user_id, "zone": zone_id});
    let resp = http_client()
        .client()
        .post("https://www.gempaytopup.com/stalk-ml")
        .header("X-CSRF-Token", &csrf)
        .header("Content-Type", "application/json")
        .header("Cookie", &cookies)
        .json(&payload)
        .send()
        .await
        .map_err(|e| format!("HTTP: {}", e))?;
    let data: Value = resp.json().await.map_err(|e| format!("JSON: {}", e))?;
    Ok(data)
}

// ---------------------------------------------------------------------------
// Free Fire — freefirecommunity API
// ---------------------------------------------------------------------------

pub async fn fetch_ff_stalk(user_id: &str) -> Result<Value, String> {
    let url = format!(
        "https://discordbot.freefirecommunity.com/player_info_api?uid={}&region=id",
        user_id
    );
    let resp = http_client()
        .client()
        .get(&url)
        .header("Origin", "https://www.freefirecommunity.com")
        .header(
            "Referer",
            "https://www.freefirecommunity.com/ff-account-info/",
        )
        .header(USER_AGENT, "Mozilla/5.0 (Linux; Android 10; K)")
        .header("Accept", "/")
        .send()
        .await
        .map_err(|e| format!("HTTP: {}", e))?;
    if !resp.status().is_success() {
        return Err(format!("HTTP {}", resp.status()));
    }
    let data: Value = resp.json().await.map_err(|e| format!("JSON: {}", e))?;

    let safe = |v: &Value| -> Value {
        if v.is_null() {
            Value::String("N/A".into())
        } else {
            v.clone()
        }
    };
    let arr_join = |v: &Value| -> Value {
        v.as_array()
            .map(|a| {
                Value::String(
                    a.iter()
                        .filter_map(|x| x.as_str())
                        .collect::<Vec<_>>()
                        .join(", "),
                )
            })
            .unwrap_or(Value::String("-".into()))
    };
    let fmt_time = |v: &Value| -> Value {
        // unix timestamp -> YYYY-MM-DD (approx via chrono)
        v.as_i64()
            .map(|ts| {
                use chrono::{TimeZone, Utc};
                match Utc.timestamp_opt(ts, 0) {
                    chrono::LocalResult::Single(dt) => {
                        Value::String(dt.format("%Y-%m-%d %H:%M:%S").to_string())
                    }
                    _ => Value::String("N/A".into()),
                }
            })
            .unwrap_or(Value::String("N/A".into()))
    };

    let d = &data["player_info"];
    let b = &d["basicInfo"];
    let c = &d["creditScoreInfo"];
    let p = &d["petInfo"];
    let prof = &d["profileInfo"];
    let s = &d["socialInfo"];

    // battle tags
    let tags = s["battleTag"].as_array().cloned().unwrap_or_default();
    let tag_counts = s["battleTagCount"].as_array().cloned().unwrap_or_default();
    let battle_tags = if tags.is_empty() {
        Value::String("N/A".into())
    } else {
        Value::String(
            tags.iter()
                .enumerate()
                .map(|(i, t)| {
                    let count = tag_counts.get(i).and_then(|v| v.as_i64()).unwrap_or(0);
                    format!("{} ({}x)", t.as_str().unwrap_or(""), count)
                })
                .collect::<Vec<_>>()
                .join("\n"),
        )
    };

    Ok(json!({
        "nickname": safe(&b["nickname"]),
        "accountId": safe(&b["accountId"]),
        "region": safe(&b["region"]),
        "level": safe(&b["level"]),
        "liked": safe(&b["liked"]),
        "rank": safe(&b["rank"]),
        "maxRank": safe(&b["maxRank"]),
        "csRank": safe(&b["csRank"]),
        "exp": safe(&b["exp"]),
        "createAt": fmt_time(&b["createAt"]),
        "lastLoginAt": fmt_time(&b["lastLoginAt"]),
        "rankingPoints": safe(&b["rankingPoints"]),
        "releaseVersion": safe(&b["releaseVersion"]),
        "seasonId": safe(&b["seasonId"]),
        "primeLevel": b["primeLevel"]["level"].is_null().then(|| Value::String("-".into())).unwrap_or_else(|| safe(&b["primeLevel"]["level"])),
        "diamondCost": safe(&d["diamondCostRes"]["diamondCost"]),
        "petName": safe(&p["name"]),
        "petLevel": safe(&p["level"]),
        "petExp": safe(&p["exp"]),
        "petSkinId": safe(&p["skinId"]),
        "petSkillId": safe(&p["selectedSkillId"]),
        "avatarId": safe(&prof["avatarId"]),
        "clothes": arr_join(&prof["clothes"]),
        "equipedSkills": arr_join(&prof["equipedSkills"]),
        "battleTags": battle_tags,
        "language": safe(&s["language"]),
        "rankShow": safe(&s["rankShow"]),
        "signature": safe(&s["signature"]),
        "creditScore": safe(&c["creditScore"]),
        "rewardState": safe(&c["rewardState"]),
        "bannerImage": format!("https://discordbot.freefirecommunity.com/banner_image_api?uid={}&region=id", user_id),
        "outfitImage": format!("https://discordbot.freefirecommunity.com/outfit_image_api?uid={}&region=id", user_id),
    }))
}

// ---------------------------------------------------------------------------
// Genshin Impact — enka.network public API
// ---------------------------------------------------------------------------

pub async fn fetch_genshin_stalk(user_id: &str) -> Result<Value, String> {
    let url = format!("https://enka.network/u/{}/__data.json", user_id);
    let resp = http_client()
        .client()
        .get(&url)
        .header(
            USER_AGENT,
            "Mozilla/5.0 (Windows NT 10.0; Win64; x64) Chrome/120.0 Safari/537.36",
        )
        .send()
        .await
        .map_err(|e| format!("HTTP: {}", e))?;
    if resp.status() == reqwest::StatusCode::NOT_FOUND {
        return Err("User tidak ditemukan".into());
    }
    if !resp.status().is_success() {
        return Err(format!("HTTP {}", resp.status()));
    }
    let data: Value = resp.json().await.map_err(|e| format!("JSON: {}", e))?;
    // Pass through the raw JSON (Enka __data.json is already structured).
    Ok(data)
}

// ---------------------------------------------------------------------------
// Twitter — fxtwitter API
// ---------------------------------------------------------------------------

pub async fn fetch_twitter_stalk(username: &str) -> Result<Value, String> {
    let url = format!("https://api.fxtwitter.com/{}", username);
    let resp = http_client()
        .client()
        .get(&url)
        .header(
            USER_AGENT,
            "Mozilla/5.0 (Windows NT 10.0; Win64; x64) Chrome/130.0 Safari/537.36",
        )
        .send()
        .await
        .map_err(|e| format!("HTTP: {}", e))?;
    let data: Value = resp.json().await.map_err(|e| format!("JSON: {}", e))?;

    // Transform to the source's shape
    let u = &data["user"];
    Ok(json!({
        "message": data.get("message").cloned().unwrap_or(Value::Null),
        "user": {
            "id": u.get("id").cloned().unwrap_or(Value::Null),
            "url": u.get("url").cloned().unwrap_or(Value::Null),
            "screen_name": u.get("screen_name").cloned().unwrap_or(Value::Null),
            "name": u.get("name").cloned().unwrap_or(Value::Null),
            "location": u.get("location").cloned().unwrap_or(Value::Null),
            "description": u.get("description").cloned().unwrap_or(Value::Null),
            "followers": u.get("followers").cloned().unwrap_or(Value::Null),
            "following": u.get("following").cloned().unwrap_or(Value::Null),
            "likes": u.get("likes").cloned().unwrap_or(Value::Null),
            "banner_url": u.get("banner_url").cloned().unwrap_or(Value::Null),
            "avatar_url": u.get("avatar_url").cloned().unwrap_or(Value::Null),
            "joined_at": u.get("joined").cloned().unwrap_or(Value::Null),
            "website": u.get("website").cloned().unwrap_or(Value::Null),
        }
    }))
}

// ---------------------------------------------------------------------------
// TikTok — __UNIVERSAL_DATA_FOR_REHYDRATION__ JSON (V1 direct, no browser)
// ---------------------------------------------------------------------------

pub async fn fetch_tiktok_stalk(username: &str) -> Result<Value, String> {
    let url = format!("https://www.tiktok.com/@{username}?_t=ZS-8tHANz7ieoS&_r=1");
    let resp = http_client()
        .client()
        .get(&url)
        .header(USER_AGENT, "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/117.0.0.0 Safari/537.36")
        .send()
        .await
        .map_err(|e| format!("HTTP: {}", e))?;
    let html = resp.text().await.map_err(|e| format!("Body: {}", e))?;

    // Extract __UNIVERSAL_DATA_FOR_REHYDRATION__ JSON
    let marker = r#"__UNIVERSAL_DATA_FOR_REHYDRATION__"#;
    let start = html.find(marker).ok_or("User tidak ditemukan")?;
    let json_start = html[start..]
        .find('>')
        .map(|i| start + i + 1)
        .ok_or("User tidak ditemukan")?;
    let json_end = html[json_start..]
        .find("</script>")
        .map(|i| json_start + i)
        .ok_or("User tidak ditemukan")?;
    let json_str = &html[json_start..json_end];

    let parsed: Value = serde_json::from_str(json_str).map_err(|e| format!("JSON: {}", e))?;
    let user_info = parsed
        .pointer("/__DEFAULT_SCOPE__/webapp.user-detail/userInfo/user")
        .cloned()
        .ok_or("User tidak ditemukan")?;
    let stats = parsed
        .pointer("/__DEFAULT_SCOPE__/webapp.user-detail/userInfo/stats")
        .cloned()
        .unwrap_or(Value::Null);

    Ok(json!({
        "userInfo": {
            "id": user_info.get("id").cloned().unwrap_or(Value::Null),
            "username": user_info.get("uniqueId").cloned().unwrap_or(Value::Null),
            "name": user_info.get("nickname").cloned().unwrap_or(Value::Null),
            "avatar": user_info.get("avatarLarger").cloned().unwrap_or(Value::Null),
            "bio": user_info.get("signature").cloned().unwrap_or(Value::Null),
            "verified": user_info.get("verified").cloned().unwrap_or(Value::Bool(false)),
            "totalFollowers": stats.get("followerCount").cloned().unwrap_or(Value::from(0)),
            "totalFollowing": stats.get("followingCount").cloned().unwrap_or(Value::from(0)),
            "totalLikes": stats.get("heart").cloned().unwrap_or(Value::from(0)),
            "totalVideos": stats.get("videoCount").cloned().unwrap_or(Value::from(0)),
            "totalFriends": stats.get("friendCount").cloned().unwrap_or(Value::from(0)),
        }
    }))
}

// ---------------------------------------------------------------------------
// Instagram — media.mollygram.com HTML scrape
// ---------------------------------------------------------------------------

pub async fn fetch_instagram_stalk(username: &str) -> Result<Value, String> {
    let url = format!("https://media.mollygram.com/?url={}", urlencode(username));
    let resp = http_client()
        .client()
        .get(&url)
        .header("accept", "*/*")
        .header("origin", "https://mollygram.com")
        .header("referer", "https://mollygram.com/")
        .header(USER_AGENT, "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/117.0.0.0 Safari/537.36")
        .send()
        .await
        .map_err(|e| format!("HTTP: {}", e))?;
    let body = resp.text().await.map_err(|e| format!("Body: {}", e))?;

    let re = |pat: &str| -> Option<String> {
        let rx = Regex::new(pat).ok()?;
        rx.captures(&body)
            .and_then(|c| c.get(1))
            .map(|m| m.as_str().trim().to_string())
    };

    let avatar = re(r#"<img[^>]*class="[^"]*rounded-circle[^"]*"[^>]*src="([^"]+)""#)
        .or_else(|| re(r#"<img[^>]*src="([^"]+)"[^>]*class="[^"]*rounded-circle[^"]*""#));
    let uname = re(r#"<h4 class="mb-0">([^<]+)</h4>"#);
    let fullname = re(r#"<p class="text-muted">([^<]+)</p>"#);
    let bio = re(r#"<p class="text-dark"[^>]*>([^<]*)</p>"#);
    let posts = re(r#"<span class="d-block h5 mb-0">([^<]+)</span>\s*<div[^>]*>\s*posts\s*</div>"#);
    let followers =
        re(r#"<span class="d-block h5 mb-0">([^<]+)</span>\s*<div[^>]*>\s*followers\s*</div>"#);
    let following =
        re(r#"<span class="d-block h5 mb-0">([^<]+)</span>\s*<div[^>]*>\s*following\s*</div>"#);

    Ok(json!({
        "avatar": avatar,
        "name": fullname.unwrap_or_default(),
        "username": uname.unwrap_or_else(|| username.to_string()),
        "posts": posts.unwrap_or_default(),
        "followers": followers.unwrap_or_default(),
        "following": following.unwrap_or_default(),
        "bio": bio.unwrap_or_default(),
    }))
}

fn urlencode(s: &str) -> String {
    s.replace('%', "%25")
        .replace('&', "%26")
        .replace('?', "%3F")
}
