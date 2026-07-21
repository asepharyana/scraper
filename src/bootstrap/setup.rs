//! Database schema initialization.

use sea_orm::{ConnectionTrait, DatabaseConnection, Schema, Statement};
use tracing::info;

use crate::infrastructure::persistence::entities::image_cache;

pub async fn init(db: &DatabaseConnection) -> Result<(), sea_orm::DbErr> {
    info!("🚀 Initializing database schema...");
    let backend = db.get_database_backend();
    let schema = Schema::new(backend);

    let tables = vec![(
        "ImageCache",
        schema
            .create_table_from_entity(image_cache::Entity)
            .if_not_exists()
            .to_owned(),
    )];

    for (name, stmt) in tables {
        match db.execute(backend.build(&stmt)).await {
            Ok(_) => info!("   ✓ Table '{}' checked/created", name),
            Err(e) => {
                tracing::error!("   [!] Failed to create table '{}': {}", name, e);
                return Err(e);
            }
        }
    }

    let index_sql =
        "CREATE INDEX IF NOT EXISTS idx_image_cache_cdn_url ON \"ImageCache\" (cdn_url)";
    match db.execute(Statement::from_string(backend, index_sql)).await {
        Ok(_) => info!("   ✓ Index 'idx_image_cache_cdn_url' ensured"),
        Err(e) => {
            let err_str = e.to_string();
            if err_str.contains("already exists") || err_str.contains("duplicate") {
                info!("   ✓ Index 'idx_image_cache_cdn_url' already exists");
            } else {
                tracing::error!("   [!] Failed to create index on ImageCache: {}", e);
            }
        }
    }

    info!("✅ Database schema initialization complete.");
    Ok(())
}
