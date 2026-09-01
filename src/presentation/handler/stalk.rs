//! Axum handlers for stalk (profile/user lookups).
//!
//! Ported from Shirokami-API `scraper/stalk/*.js`.

use axum::extract::Query;
use axum::http::StatusCode;
use axum::Json;
use serde::Deserialize;
use serde_json::Value;
use utoipa::IntoParams;

use crate::application::stalk as use_cases;
use crate::presentation::error::AppError;

/// Query params for stalk endpoints.
#[derive(Deserialize, IntoParams)]
pub struct StalkParams {
    pub username: Option<String>,
    pub user_id: Option<String>,
    pub userid: Option<String>,
    pub zone_id: Option<String>,
    pub zoneid: Option<String>,
}

/// GET /stalk/github?username=asepharyana
#[utoipa::path(
    get,
    path = "/stalk/github",
    tag = "stalk",
    operation_id = "stalk_github",
    params(StalkParams),
    responses(
        (status = 200, description = "GitHub user profile", body = Value),
        (status = 400, description = "Missing username"),
        (status = 502, description = "Upstream error"),
    )
)]
pub async fn github_handler(
    Query(params): Query<StalkParams>,
) -> Result<(StatusCode, Json<Value>), AppError> {
    let username = params
        .username
        .ok_or_else(|| AppError::BadRequest("Missing 'username' parameter".to_string()))?;
    let result = use_cases::github(&username).await?;
    Ok((StatusCode::OK, Json(result)))
}

/// GET /stalk/youtube?username=<handle>
#[utoipa::path(
    get,
    path = "/stalk/youtube",
    tag = "stalk",
    operation_id = "stalk_youtube",
    params(StalkParams),
    responses(
        (status = 200, description = "YouTube channel profile", body = Value),
        (status = 400, description = "Missing username"),
        (status = 502, description = "Upstream error"),
    )
)]
pub async fn youtube_handler(
    Query(params): Query<StalkParams>,
) -> Result<(StatusCode, Json<Value>), AppError> {
    let username = params
        .username
        .ok_or_else(|| AppError::BadRequest("Missing 'username' parameter".to_string()))?;
    let result = use_cases::youtube(&username).await?;
    Ok((StatusCode::OK, Json(result)))
}

/// GET /stalk/mobile-legends?user_id=...&zone_id=... (aliases: userid, zoneid)
#[utoipa::path(
    get,
    path = "/stalk/mobile-legends",
    tag = "stalk",
    operation_id = "stalk_mobile_legends",
    params(StalkParams),
    responses(
        (status = 200, description = "Mobile Legends player info", body = Value),
        (status = 400, description = "Missing user_id/zone_id"),
        (status = 502, description = "Upstream error"),
    )
)]
pub async fn mobile_legends_handler(
    Query(params): Query<StalkParams>,
) -> Result<(StatusCode, Json<Value>), AppError> {
    let user_id = params
        .user_id
        .or(params.userid)
        .ok_or_else(|| AppError::BadRequest("Missing 'user_id' parameter".to_string()))?;
    let zone_id = params
        .zone_id
        .or(params.zoneid)
        .ok_or_else(|| AppError::BadRequest("Missing 'zone_id' parameter".to_string()))?;
    let result = use_cases::mobile_legends(&user_id, &zone_id).await?;
    Ok((StatusCode::OK, Json(result)))
}

/// GET /stalk/free-fire?user_id=... (alias: userid)
#[utoipa::path(
    get,
    path = "/stalk/free-fire",
    tag = "stalk",
    operation_id = "stalk_free_fire",
    params(StalkParams),
    responses(
        (status = 200, description = "Free Fire player info", body = Value),
        (status = 400, description = "Missing user_id"),
        (status = 502, description = "Upstream error"),
    )
)]
pub async fn free_fire_handler(
    Query(params): Query<StalkParams>,
) -> Result<(StatusCode, Json<Value>), AppError> {
    let user_id = params
        .user_id
        .or(params.userid)
        .ok_or_else(|| AppError::BadRequest("Missing 'user_id' parameter".to_string()))?;
    let result = use_cases::free_fire(&user_id).await?;
    Ok((StatusCode::OK, Json(result)))
}

/// GET /stalk/genshin-impact?user_id=... (alias: userid)
#[utoipa::path(
    get,
    path = "/stalk/genshin-impact",
    tag = "stalk",
    operation_id = "stalk_genshin",
    params(StalkParams),
    responses(
        (status = 200, description = "Genshin Impact profile", body = Value),
        (status = 400, description = "Missing user_id"),
        (status = 502, description = "Upstream error"),
    )
)]
pub async fn genshin_handler(
    Query(params): Query<StalkParams>,
) -> Result<(StatusCode, Json<Value>), AppError> {
    let user_id = params
        .user_id
        .or(params.userid)
        .ok_or_else(|| AppError::BadRequest("Missing 'user_id' parameter".to_string()))?;
    let result = use_cases::genshin(&user_id).await?;
    Ok((StatusCode::OK, Json(result)))
}

/// GET /stalk/twitter?username=...
#[utoipa::path(
    get,
    path = "/stalk/twitter",
    tag = "stalk",
    operation_id = "stalk_twitter",
    params(StalkParams),
    responses(
        (status = 200, description = "Twitter/X user profile", body = Value),
        (status = 400, description = "Missing username"),
        (status = 502, description = "Upstream error"),
    )
)]
pub async fn twitter_handler(
    Query(params): Query<StalkParams>,
) -> Result<(StatusCode, Json<Value>), AppError> {
    let username = params
        .username
        .ok_or_else(|| AppError::BadRequest("Missing 'username' parameter".to_string()))?;
    let result = use_cases::twitter(&username).await?;
    Ok((StatusCode::OK, Json(result)))
}

/// GET /stalk/tiktok?username=...
#[utoipa::path(
    get,
    path = "/stalk/tiktok",
    tag = "stalk",
    operation_id = "stalk_tiktok",
    params(StalkParams),
    responses(
        (status = 200, description = "TikTok user profile", body = Value),
        (status = 400, description = "Missing username"),
        (status = 502, description = "Upstream error"),
    )
)]
pub async fn tiktok_handler(
    Query(params): Query<StalkParams>,
) -> Result<(StatusCode, Json<Value>), AppError> {
    let username = params
        .username
        .ok_or_else(|| AppError::BadRequest("Missing 'username' parameter".to_string()))?;
    let result = use_cases::tiktok(&username).await?;
    Ok((StatusCode::OK, Json(result)))
}

/// GET /stalk/instagram?username=...
#[utoipa::path(
    get,
    path = "/stalk/instagram",
    tag = "stalk",
    operation_id = "stalk_instagram",
    params(StalkParams),
    responses(
        (status = 200, description = "Instagram user profile", body = Value),
        (status = 400, description = "Missing username"),
        (status = 502, description = "Upstream error"),
    )
)]
pub async fn instagram_handler(
    Query(params): Query<StalkParams>,
) -> Result<(StatusCode, Json<Value>), AppError> {
    let username = params
        .username
        .ok_or_else(|| AppError::BadRequest("Missing 'username' parameter".to_string()))?;
    let result = use_cases::instagram(&username).await?;
    Ok((StatusCode::OK, Json(result)))
}
