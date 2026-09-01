//! Axum handlers for stalk (profile/user lookups).
//!
//! Ported from Shirokami-API `scraper/stalk/*.js`.

use axum::extract::Query;
use axum::http::StatusCode;
use axum::Json;
use serde::Deserialize;
use serde_json::Value;

use crate::application::stalk as use_cases;
use crate::presentation::error::AppError;

/// Query params for stalk endpoints.
#[derive(Deserialize)]
pub struct StalkParams {
    pub username: Option<String>,
    pub user_id: Option<String>,
    pub userid: Option<String>,
    pub zone_id: Option<String>,
    pub zoneid: Option<String>,
}

/// GET /stalk/github?username=asepharyana
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
pub async fn instagram_handler(
    Query(params): Query<StalkParams>,
) -> Result<(StatusCode, Json<Value>), AppError> {
    let username = params
        .username
        .ok_or_else(|| AppError::BadRequest("Missing 'username' parameter".to_string()))?;
    let result = use_cases::instagram(&username).await?;
    Ok((StatusCode::OK, Json(result)))
}
