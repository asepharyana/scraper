//! Axum handlers for weebs endpoints.
//!
//! Ported from Shirokami-API `scraper/weebs/*.js`.

use axum::extract::Query;
use axum::http::StatusCode;
use axum::Json;
use serde::Deserialize;
use serde_json::Value;

use crate::application::weebs as use_cases;
use crate::presentation::error::AppError;

#[derive(Deserialize)]
pub struct WeebsParams {
    pub query: Option<String>,
    pub q: Option<String>,
    pub url: Option<String>,
    pub tag: Option<String>,
}

fn require(v: Option<&String>, name: &str) -> Result<String, AppError> {
    v.cloned()
        .ok_or_else(|| AppError::BadRequest(format!("Missing '{}' parameter", name)))
}

/// GET /weebs/anime-info?query=naruto
#[utoipa::path(
    get,
    path = "/weebs/anime-info",
    tag = "weebs",
    operation_id = "weebs_anime_info_handler",
    responses(
        (status = 200, description = "OK", body = serde_json::Value),
        (status = 400, description = "Bad Request"),
        (status = 502, description = "Upstream error"),
    )
)]

pub async fn anime_info_handler(
    Query(p): Query<WeebsParams>,
) -> Result<(StatusCode, Json<Value>), AppError> {
    let query = require(p.query.as_ref().or(p.q.as_ref()), "query")?;
    Ok((StatusCode::OK, Json(use_cases::anime_info(&query).await?)))
}

/// GET /weebs/manga-info?query=one%20piece
#[utoipa::path(
    get,
    path = "/weebs/manga-info",
    tag = "weebs",
    operation_id = "weebs_manga_info_handler",
    responses(
        (status = 200, description = "OK", body = serde_json::Value),
        (status = 400, description = "Bad Request"),
        (status = 502, description = "Upstream error"),
    )
)]

pub async fn manga_info_handler(
    Query(p): Query<WeebsParams>,
) -> Result<(StatusCode, Json<Value>), AppError> {
    let query = require(p.query.as_ref().or(p.q.as_ref()), "query")?;
    Ok((StatusCode::OK, Json(use_cases::manga_info(&query).await?)))
}

/// GET /weebs/whatanime?url=<image-url>
#[utoipa::path(
    get,
    path = "/weebs/whatanime",
    tag = "weebs",
    operation_id = "weebs_whatanime_handler",
    responses(
        (status = 200, description = "OK", body = serde_json::Value),
        (status = 400, description = "Bad Request"),
        (status = 502, description = "Upstream error"),
    )
)]

pub async fn whatanime_handler(
    Query(p): Query<WeebsParams>,
) -> Result<(StatusCode, Json<Value>), AppError> {
    let url = require(p.url.as_ref(), "url")?;
    Ok((StatusCode::OK, Json(use_cases::whatanime(&url).await?)))
}

/// GET /weebs/sfw-waifu?tag=waifu
#[utoipa::path(
    get,
    path = "/weebs/sfw-waifu",
    tag = "weebs",
    operation_id = "weebs_sfw_waifu_handler",
    responses(
        (status = 200, description = "OK", body = serde_json::Value),
        (status = 400, description = "Bad Request"),
        (status = 502, description = "Upstream error"),
    )
)]

pub async fn sfw_waifu_handler(
    Query(p): Query<WeebsParams>,
) -> Result<(StatusCode, Json<Value>), AppError> {
    let tag = require(p.tag.as_ref(), "tag")?;
    Ok((StatusCode::OK, Json(use_cases::sfw_waifu(&tag).await?)))
}

/// GET /weebs/nsfw-waifu?tag=waifu
#[utoipa::path(
    get,
    path = "/weebs/nsfw-waifu",
    tag = "weebs",
    operation_id = "weebs_nsfw_waifu_handler",
    responses(
        (status = 200, description = "OK", body = serde_json::Value),
        (status = 400, description = "Bad Request"),
        (status = 502, description = "Upstream error"),
    )
)]

pub async fn nsfw_waifu_handler(
    Query(p): Query<WeebsParams>,
) -> Result<(StatusCode, Json<Value>), AppError> {
    let tag = require(p.tag.as_ref(), "tag")?;
    Ok((StatusCode::OK, Json(use_cases::nsfw_waifu(&tag).await?)))
}
