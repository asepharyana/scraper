//! Axum handlers for tools.

use axum::extract::Query;
use axum::http::StatusCode;
use axum::Json;
use serde::Deserialize;
use serde_json::Value;

use crate::application::tools as use_cases;
use crate::presentation::error::AppError;

#[derive(Deserialize)]
pub struct DomainParams {
    pub domain: Option<String>,
}

#[derive(Deserialize)]
pub struct IpParams {
    pub ip: Option<String>,
}

#[derive(Deserialize)]
pub struct UrlParams {
    pub url: Option<String>,
}

#[derive(Deserialize)]
pub struct ResiParams {
    pub resi: Option<String>,
    pub ekspedisi: Option<String>,
}

pub async fn whois_handler(
    Query(params): Query<DomainParams>,
) -> Result<(StatusCode, Json<Value>), AppError> {
    let domain = params
        .domain
        .ok_or_else(|| AppError::BadRequest("domain parameter is required".into()))?;
    Ok((StatusCode::OK, Json(use_cases::whois(&domain).await?)))
}

pub async fn ip_location_handler(
    Query(params): Query<IpParams>,
) -> Result<(StatusCode, Json<Value>), AppError> {
    let ip = params
        .ip
        .ok_or_else(|| AppError::BadRequest("ip parameter is required".into()))?;
    Ok((StatusCode::OK, Json(use_cases::ip_location(&ip).await?)))
}

pub async fn tinyurl_handler(
    Query(params): Query<UrlParams>,
) -> Result<(StatusCode, Json<Value>), AppError> {
    let url = params
        .url
        .ok_or_else(|| AppError::BadRequest("url parameter is required".into()))?;
    Ok((StatusCode::OK, Json(use_cases::tinyurl(&url).await?)))
}

pub async fn check_hosting_handler(
    Query(params): Query<DomainParams>,
) -> Result<(StatusCode, Json<Value>), AppError> {
    let domain = params
        .domain
        .ok_or_else(|| AppError::BadRequest("domain parameter is required".into()))?;
    Ok((
        StatusCode::OK,
        Json(use_cases::check_hosting(&domain).await?),
    ))
}

pub async fn hargapangan_handler() -> Result<(StatusCode, Json<Value>), AppError> {
    Ok((StatusCode::OK, Json(use_cases::hargapangan().await?)))
}

pub async fn cek_resi_handler(
    Query(params): Query<ResiParams>,
) -> Result<(StatusCode, Json<Value>), AppError> {
    let ResiParams { resi, ekspedisi } = params;
    let resi = resi.ok_or_else(|| AppError::BadRequest("resi parameter is required".into()))?;
    Ok((
        StatusCode::OK,
        Json(use_cases::cek_resi(resi, ekspedisi).await?),
    ))
}
