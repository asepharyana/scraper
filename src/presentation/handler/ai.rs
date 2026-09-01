//! Axum handlers for AI endpoints (Pollinations).

use axum::body::Body;
use axum::extract::Query;
use axum::http::{header, StatusCode};
use axum::response::Response;
use axum::Json;
use serde::Deserialize;
use serde_json::Value;

use crate::application::ai as use_cases;
use crate::presentation::error::AppError;

#[derive(Deserialize)]
pub struct AiParams {
    pub text: Option<String>,
    pub prompt: Option<String>,
    pub model: Option<String>,
    pub image_url: Option<String>,
    pub url: Option<String>,
    pub width: Option<u32>,
    pub height: Option<u32>,
}

/// GET /ai/chat?text=...&prompt=...&model=...
#[utoipa::path(
    get,
    path = "/ai/chat",
    tag = "ai",
    operation_id = "ai_chat_handler",
    responses(
        (status = 200, description = "OK", body = serde_json::Value),
        (status = 400, description = "Bad Request"),
        (status = 502, description = "Upstream error"),
    )
)]

pub async fn chat_handler(Query(params): Query<AiParams>) -> Result<Json<Value>, AppError> {
    let text = params
        .text
        .ok_or_else(|| AppError::BadRequest("Missing 'text' parameter".to_string()))?;
    let result = use_cases::chat(text, params.prompt.unwrap_or_default(), params.model).await?;
    Ok(Json(result))
}

/// GET /ai/gemini?text=...&prompt=...&image_url=...
#[utoipa::path(
    get,
    path = "/ai/gemini",
    tag = "ai",
    operation_id = "ai_gemini_handler",
    responses(
        (status = 200, description = "OK", body = serde_json::Value),
        (status = 400, description = "Bad Request"),
        (status = 502, description = "Upstream error"),
    )
)]

pub async fn gemini_handler(Query(params): Query<AiParams>) -> Result<Json<Value>, AppError> {
    let text = params
        .text
        .ok_or_else(|| AppError::BadRequest("Missing 'text' parameter".to_string()))?;
    let result =
        use_cases::gemini(text, params.prompt.unwrap_or_default(), params.image_url).await?;
    Ok(Json(result))
}

/// GET /ai/image?prompt=...&model=...&width=...&height=... — returns image bytes.
#[utoipa::path(
    get,
    path = "/ai/image",
    tag = "ai",
    operation_id = "ai_image_handler",
    responses(
        (status = 200, description = "OK", body = serde_json::Value),
        (status = 400, description = "Bad Request"),
        (status = 502, description = "Upstream error"),
    )
)]

pub async fn image_handler(Query(params): Query<AiParams>) -> Result<Response<Body>, AppError> {
    let prompt = params
        .prompt
        .ok_or_else(|| AppError::BadRequest("Missing 'prompt' parameter".to_string()))?;
    let model = params.model.unwrap_or_else(|| "flux".to_string());
    let width = params.width.unwrap_or(1024);
    let height = params.height.unwrap_or(1024);

    let bytes = use_cases::image(prompt, model, width, height).await?;
    // Pollinations flux returns JPEG by default.
    let mime = if bytes.starts_with(b"\xff\xd8\xff") {
        "image/jpeg"
    } else if bytes.starts_with(b"\x89PNG\r\n\x1a\n") {
        "image/png"
    } else {
        "application/octet-stream"
    };
    Response::builder()
        .status(StatusCode::OK)
        .header(header::CONTENT_TYPE, mime)
        .body(Body::from(bytes))
        .map_err(|e| AppError::Internal(format!("build response: {e}")))
}
