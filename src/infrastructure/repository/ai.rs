//! Infrastructure — AI endpoints via Pollinations.ai (free, no API key).
//!
//! Ported from Shirokami-API `scraper/ai/pollinationsai-{text,image,gemini}.js`.
//! All three use the public Pollinations API:
//!   - text: POST https://text.pollinations.ai/openai (OpenAI-compatible)
//!   - gemini: POST https://text.pollinations.ai/openai with model=gemini
//!   - image: GET https://image.pollinations.ai/prompt/{prompt}
//! Retry logic follows the source: try Bearer token (hardcoded in source),
//! fall back to Referer when 401/403 (no key needed).

use crate::infrastructure::utils::http_client::http_client;
use serde_json::{json, Value};

const TEXT_URL: &str = "https://text.pollinations.ai/openai";
const IMAGE_URL: &str = "https://image.pollinations.ai/prompt";
// The source ships a hardcoded token; on 401/403 it falls back to Referer-only.
// We skip the token and use Referer directly (anonymous tier) — same result,
// no secret embedded in the binary.
const REFERER: &str = "https://api.ryzumi.vip";

fn ua() -> &'static str {
    "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/120.0 Safari/537.36"
}

fn build_messages(system: &str, text: &str, image_data_url: Option<String>) -> Vec<Value> {
    let mut messages = Vec::new();
    if !system.is_empty() {
        messages.push(json!({"role": "system", "content": system}));
    }
    if let Some(img) = image_data_url {
        let mut content = Vec::new();
        if !text.is_empty() {
            content.push(json!({"type": "text", "text": text}));
        }
        content.push(json!({"type": "image_url", "image_url": {"url": img}}));
        messages.push(json!({"role": "user", "content": content}));
    } else {
        messages.push(json!({"role": "user", "content": text}));
    }
    messages
}

/// Chat via Pollinations (OpenAI-compatible).
/// Default model "openai" (the source's "gpt-5-nano" no longer exists on
/// Pollinations and 404s; fallback to "openai" on any error, incl. 404).
pub async fn chat(text: String, prompt: String, model: Option<String>) -> Result<Value, String> {
    let model = model.unwrap_or_else(|| "openai".to_string());
    let system = if prompt.trim().is_empty() {
        String::new()
    } else {
        format!("{}\nConversation history:", prompt)
    };
    let messages = build_messages(&system, &text, None);

    let body = json!({
        "model": model,
        "messages": messages,
        "referrer": REFERER,
    });

    let resp = http_client()
        .client()
        .post(TEXT_URL)
        .header("Content-Type", "application/json")
        .header("Referer", REFERER)
        .header("User-Agent", ua())
        .json(&body)
        .send()
        .await
        .map_err(|e| format!("HTTP: {e}"))?;

    if !resp.status().is_success() {
        // Match source: on failure retry with model "openai"
        let status = resp.status();
        if status == reqwest::StatusCode::UNAUTHORIZED
            || status == reqwest::StatusCode::FORBIDDEN
            || status == reqwest::StatusCode::NOT_FOUND
            || status == reqwest::StatusCode::PAYMENT_REQUIRED
        {
            let body2 = json!({
                "model": "openai",
                "messages": messages,
                "referrer": REFERER,
            });
            let resp2 = http_client()
                .client()
                .post(TEXT_URL)
                .header("Content-Type", "application/json")
                .header("Referer", REFERER)
                .header("User-Agent", ua())
                .json(&body2)
                .send()
                .await
                .map_err(|e| format!("HTTP retry: {e}"))?;
            return parse_text_response(resp2).await;
        }
        return Err(format!("API error {status}"));
    }

    parse_text_response(resp).await
}

/// Gemini chat via Pollinations (model "gemini").
pub async fn gemini(
    text: String,
    prompt: String,
    image_url: Option<String>,
) -> Result<Value, String> {
    let system = if prompt.trim().is_empty() {
        String::new()
    } else {
        format!("{}\nConversation history:", prompt)
    };

    // If an image URL is given, fetch it and convert to a data URL (source behavior).
    let image_data_url = match &image_url {
        Some(url) if !url.trim().is_empty() => Some(fetch_data_url(url).await?),
        _ => None,
    };

    let messages = build_messages(&system, &text, image_data_url);

    let body = json!({
        "model": "gemini",
        "messages": messages,
        "referrer": REFERER,
    });

    let resp = http_client()
        .client()
        .post(TEXT_URL)
        .header("Content-Type", "application/json")
        .header("Referer", REFERER)
        .header("User-Agent", ua())
        .json(&body)
        .send()
        .await
        .map_err(|e| format!("HTTP: {e}"))?;

    if !resp.status().is_success() {
        let status = resp.status();
        if status == reqwest::StatusCode::UNAUTHORIZED
            || status == reqwest::StatusCode::FORBIDDEN
            || status == reqwest::StatusCode::NOT_FOUND
            || status == reqwest::StatusCode::PAYMENT_REQUIRED
        {
            let body2 = json!({
                "model": "openai",
                "messages": messages,
                "referrer": REFERER,
            });
            let resp2 = http_client()
                .client()
                .post(TEXT_URL)
                .header("Content-Type", "application/json")
                .header("Referer", REFERER)
                .header("User-Agent", ua())
                .json(&body2)
                .send()
                .await
                .map_err(|e| format!("HTTP retry: {e}"))?;
            return parse_text_response(resp2).await;
        }
        return Err(format!("API error {status}"));
    }

    parse_text_response(resp).await
}

/// Image generation via Pollinations. Returns raw image bytes.
pub async fn image(
    prompt: String,
    model: String,
    width: u32,
    height: u32,
) -> Result<Vec<u8>, String> {
    let model = if model.is_empty() {
        "flux".to_string()
    } else {
        model
    };
    let valid = ["flux", "kontext", "turbo"];
    if !valid.contains(&model.as_str()) {
        return Err(format!(
            "Invalid model. Available models: {}",
            valid.join(", ")
        ));
    }

    let url = format!(
        "{}/{}?model={}&width={}&height={}&nologo=true&private=false&enhance=false&safe=false&referrer={}",
        IMAGE_URL,
        urlencode(&prompt),
        model,
        width,
        height,
        urlencode(REFERER)
    );

    let resp = http_client()
        .client()
        .get(&url)
        .header("Referer", REFERER)
        .header("User-Agent", ua())
        .send()
        .await
        .map_err(|e| format!("HTTP: {e}"))?;

    if !resp.status().is_success() {
        return Err(format!("Image API error {}", resp.status()));
    }

    let bytes = resp.bytes().await.map_err(|e| format!("Body: {e}"))?;
    Ok(bytes.to_vec())
}

async fn parse_text_response(resp: reqwest::Response) -> Result<Value, String> {
    let data: Value = resp.json().await.map_err(|e| format!("JSON: {e}"))?;
    let content = data
        .pointer("/choices/0/message/content")
        .or_else(|| data.get("message"))
        .cloned()
        .unwrap_or_else(|| Value::String("No response from assistant.".into()));
    Ok(json!({"success": true, "result": content}))
}

async fn fetch_data_url(url: &str) -> Result<String, String> {
    if url.starts_with("data:") {
        return Ok(url.to_string());
    }
    let resp = http_client()
        .client()
        .get(url)
        .header("User-Agent", ua())
        .send()
        .await
        .map_err(|e| format!("Fetch image failed: {e}"))?;
    if !resp.status().is_success() {
        return Err(format!("Fetch image failed: {}", resp.status()));
    }
    let ct = resp
        .headers()
        .get(reqwest::header::CONTENT_TYPE)
        .and_then(|v| v.to_str().ok())
        .unwrap_or("image/jpeg")
        .to_string();
    let ext = ct
        .split('/')
        .nth(1)
        .map(|s| s.split('+').next().unwrap_or("jpeg").to_string())
        .unwrap_or_else(|| "jpeg".to_string());
    let bytes = resp.bytes().await.map_err(|e| format!("Body: {e}"))?;
    use base64::Engine;
    Ok(format!(
        "data:image/{};base64,{}",
        ext,
        base64::engine::general_purpose::STANDARD.encode(bytes)
    ))
}

fn urlencode(s: &str) -> String {
    let mut out = String::new();
    for b in s.bytes() {
        match b {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                out.push(b as char)
            }
            _ => out.push_str(&format!("%{:02X}", b)),
        }
    }
    out
}
