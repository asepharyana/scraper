//! Application state shared across all handlers.

use std::sync::Arc;

use deadpool_redis::Pool;
use sea_orm::DatabaseConnection;

use crate::events::bus::EventBus;

/// Shared application state injected into every handler via Axum State.
///
/// Contains the infrastructure dependencies that handlers and use cases
/// need to serve requests.
#[derive(Clone)]
pub struct AppState {
    pub redis_pool: Pool,
    pub db: Arc<DatabaseConnection>,
    pub event_bus: Arc<EventBus>,
}

impl AppState {
    pub fn sea_orm(&self) -> &DatabaseConnection {
        &self.db
    }
}
