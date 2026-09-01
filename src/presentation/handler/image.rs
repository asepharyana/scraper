//! Axum handlers for image generation endpoints.
//!
//! Returns raw image bytes with the proper Content-Type.

use axum::body::Body;
use axum::extract::Query;
use axum::http::{header, StatusCode};
use axum::response::Response;
use serde::Deserialize;

use crate::application::image as use_cases;
use crate::presentation::error::AppError;

#[derive(Deserialize)]
pub struct BratParams {
    pub text: Option<String>,
}

pub async fn brat_handler(Query(params): Query<BratParams>) -> Result<Response, AppError> {
    let text = params
        .text
        .ok_or_else(|| AppError::BadRequest("text parameter is required".into()))?;
    let bytes = use_cases::brat(&text).await?;
    Ok(Response::builder()
        .status(StatusCode::OK)
        .header(header::CONTENT_TYPE, "image/png")
        .body(Body::from(bytes))
        .expect("valid response"))
}

pub async fn brat_animated_handler(Query(params): Query<BratParams>) -> Result<Response, AppError> {
    let text = params
        .text
        .ok_or_else(|| AppError::BadRequest("text parameter is required".into()))?;
    let bytes = use_cases::brat_animated(&text).await?;
    Ok(Response::builder()
        .status(StatusCode::OK)
        .header(header::CONTENT_TYPE, "image/gif")
        .body(Body::from(bytes))
        .expect("valid response"))
}
