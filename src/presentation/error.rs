//! Application-level HTTP error handling.
//!
//! Maps domain errors and infrastructure errors into HTTP responses.

use axum::response::IntoResponse;
use thiserror::Error;

use crate::domain::error::{DomainError, RepositoryError, ScrapingError};

/// Top-level HTTP error returned by all API handlers.
#[derive(Error, Debug)]
pub enum AppError {
    #[error("Bad request: {0}")]
    BadRequest(String),
    #[error("Not found: {0}")]
    NotFound(String),
    #[error("Scraping error: {0}")]
    ScraperError(String),
    #[error("Database error: {0}")]
    DatabaseError(String),
    #[error("Internal error: {0}")]
    Internal(String),
    #[error("Http error: {0}")]
    HttpError(#[from] http::Error),
    #[error("Url parse error: {0}")]
    UrlParseError(#[from] url::ParseError),
    #[error("Redis error: {0}")]
    RedisError(#[from] redis::RedisError),
    #[error("Json error: {0}")]
    SerdeJsonError(#[from] serde_json::Error),
    #[error("Reqwest error: {0}")]
    ReqwestError(#[from] reqwest::Error),
    #[error("IO error: {0}")]
    IoError(#[from] std::io::Error),
}

// ============================================================================
// From impls — convert domain/infra errors to AppError
// ============================================================================

impl From<DomainError> for AppError {
    fn from(err: DomainError) -> Self {
        match err {
            DomainError::NotFound(msg) => AppError::NotFound(msg),
            DomainError::Validation(msg) => AppError::BadRequest(msg),
            DomainError::Repository(repo_err) => match repo_err {
                RepositoryError::NotFound => AppError::NotFound("Resource not found".into()),
                RepositoryError::Conflict(msg) => {
                    AppError::BadRequest(format!("Conflict: {}", msg))
                }
                RepositoryError::Database(msg) => AppError::DatabaseError(msg),
                RepositoryError::Network(msg) => AppError::ScraperError(msg),
            },
            DomainError::Scraping(scrape_err) => match scrape_err {
                ScrapingError::Http(msg) => AppError::ScraperError(msg),
                ScrapingError::Parse(msg) => AppError::BadRequest(format!("Parse error: {}", msg)),
                ScrapingError::EmptyResponse => {
                    AppError::NotFound("Empty response from source".into())
                }
            },
        }
    }
}

impl From<String> for AppError {
    fn from(s: String) -> Self {
        AppError::Internal(s)
    }
}

impl From<anyhow::Error> for AppError {
    fn from(err: anyhow::Error) -> Self {
        AppError::Internal(err.to_string())
    }
}

impl From<deadpool_redis::PoolError> for AppError {
    fn from(err: deadpool_redis::PoolError) -> Self {
        AppError::Internal(err.to_string())
    }
}

impl From<tokio::task::JoinError> for AppError {
    fn from(err: tokio::task::JoinError) -> Self {
        AppError::Internal(err.to_string())
    }
}

impl From<&str> for AppError {
    fn from(s: &str) -> Self {
        AppError::Internal(s.to_string())
    }
}

// ============================================================================
// IntoResponse — render AppError as HTTP response
// ============================================================================

impl IntoResponse for AppError {
    fn into_response(self) -> axum::response::Response {
        use crate::presentation::dto::common::ApiResponse;
        use http::StatusCode;

        let (status, error_message) = match &self {
            AppError::NotFound(_) => (StatusCode::NOT_FOUND, self.to_string()),
            AppError::BadRequest(_) => (StatusCode::BAD_REQUEST, self.to_string()),
            AppError::ScraperError(_) => (StatusCode::BAD_GATEWAY, self.to_string()),
            AppError::DatabaseError(_) => {
                tracing::error!(%self, "Database error");
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    "Internal server error".into(),
                )
            }
            AppError::Internal(_) => {
                tracing::error!(%self, "Internal error");
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    "Internal server error".into(),
                )
            }
            _ => {
                tracing::error!(%self, "Unhandled error");
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    "Internal server error".into(),
                )
            }
        };

        let body = axum::Json(ApiResponse::<()>::error(error_message));
        (status, body).into_response()
    }
}

impl From<ScrapingError> for AppError {
    fn from(e: ScrapingError) -> Self {
        // Map ScrapingError::Http to BAD_GATEWAY (502) — the scraper code
        // correctly returns DownloadResult::error for client-side issues
        // (invalid URL, bad format), so any ScraperError that escapes is
        // an upstream provider failure, not a local bug.
        AppError::ScraperError(e.to_string())
    }
}
