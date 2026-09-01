//! Axum handlers for misc utility endpoints.
//!
//! Ported from Shirokami-API `scraper/misc/*.js`.

use axum::extract::Query;
use axum::http::StatusCode;
use axum::Json;
use serde::Deserialize;
use serde_json::Value;

use crate::application::misc as use_cases;
use crate::presentation::error::AppError;

/// Query params for currency converter.
#[derive(Deserialize)]
pub struct CurrencyParams {
    pub amount: Option<f64>,
    pub from: Option<String>,
    pub to: Option<String>,
}

/// GET /misc/currency-converter?amount=100&from=USD&to=IDR
pub async fn currency_converter_handler(
    Query(params): Query<CurrencyParams>,
) -> Result<(StatusCode, Json<Value>), AppError> {
    let amount = params.amount.unwrap_or(1.0);
    let from = params.from.unwrap_or_else(|| "USD".to_string());
    let to = params.to.unwrap_or_else(|| "IDR".to_string());

    let result = use_cases::currency_converter(amount, &from, &to).await?;
    Ok((StatusCode::OK, Json(result)))
}

/// GET /misc/harga-emas
pub async fn harga_emas_handler() -> Result<(StatusCode, Json<Value>), AppError> {
    let result = use_cases::harga_emas().await?;
    Ok((StatusCode::OK, Json(result)))
}

/// GET /misc/kurs-bca
pub async fn kurs_bca_handler() -> Result<(StatusCode, Json<Value>), AppError> {
    let result = use_cases::kurs_bca().await?;
    Ok((StatusCode::OK, Json(result)))
}

/// GET /misc/server-info
pub async fn server_info_handler() -> Result<(StatusCode, Json<Value>), AppError> {
    let result = use_cases::server_info().await?;
    Ok((StatusCode::OK, Json(result)))
}
