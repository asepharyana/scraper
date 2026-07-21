//! Rate limiting middleware.

use axum::{
    extract::Request,
    http::StatusCode,
    middleware::Next,
    response::{IntoResponse, Response},
    Json,
};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Instant;

use crate::presentation::dto::common::ApiResponse;

/// Simple in-memory rate limiter.
pub struct RateLimiter {
    max_requests: u64,
    window_secs: u64,
    counter: AtomicU64,
    window_start: Mutex<Instant>,
}

impl RateLimiter {
    pub fn new(max_requests: u64, window_secs: u64) -> Arc<Self> {
        Arc::new(Self {
            max_requests,
            window_secs,
            counter: AtomicU64::new(0),
            window_start: Mutex::new(Instant::now()),
        })
    }

    pub fn check(&self) -> bool {
        let Ok(mut window_guard) = self.window_start.lock() else {
            return false;
        };
        let window = &mut *window_guard;
        if window.elapsed().as_secs() >= self.window_secs {
            *window = Instant::now();
            self.counter.store(0, Ordering::SeqCst);
        }
        let count = self.counter.fetch_add(1, Ordering::SeqCst);
        count < self.max_requests
    }
}

pub async fn rate_limit_middleware(
    state: axum::extract::State<Arc<RateLimiter>>,
    request: Request,
    next: Next,
) -> Response {
    if state.check() {
        next.run(request).await
    } else {
        (
            StatusCode::TOO_MANY_REQUESTS,
            Json(ApiResponse::<()>::error("Rate limit exceeded".to_string())),
        )
            .into_response()
    }
}
