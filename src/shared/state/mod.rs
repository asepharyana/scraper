use std::sync::Arc;

use deadpool_redis::Pool;
use sea_orm::DatabaseConnection;

pub struct AppState {
    pub redis_pool: Pool,
    pub db: Arc<DatabaseConnection>,
    pub image_processing_semaphore: Arc<tokio::sync::Semaphore>,
    pub event_bus: Arc<crate::shared::events::bus::EventBus>,
}

impl AppState {
    pub fn sea_orm(&self) -> &DatabaseConnection {
        &self.db
    }
}
