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
