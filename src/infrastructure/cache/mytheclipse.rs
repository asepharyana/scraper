//! Bridge between the deadpool Redis pool and `mytheclipse_cache::RedisCache`.
//!
//! The scraper keeps its deadpool `Pool` (connection lifecycle, recycling) and
//! hands each checked-out connection to mytheclipse's `RedisCache` (which
//! implements the library `Cache` trait). Because both sides now share the
//! same `redis` crate version, a `deadpool_redis::Connection` can be converted
//! directly into a `redis::aio::MultiplexedConnection` via `take()`.

use std::sync::LazyLock;

use mytheclipse_cache::{Cache, CacheError, RedisCache};
use tokio::sync::OnceCell;

use crate::infrastructure::cache::redis_pool::redis_pool;

/// Lazily-initialised shared `RedisCache` built from the deadpool pool.
///
/// The multiplexed connection is cheaply cloneable (Arc-backed), so the whole
/// process shares one logical connection while deadpool manages recycling.
static REDIS_CACHE: LazyLock<OnceCell<RedisCache>> = LazyLock::new(OnceCell::new);

/// Return a handle to the shared mytheclipse `RedisCache`, initialising it on
/// first use from the deadpool pool.
pub async fn redis_cache() -> Result<&'static RedisCache, CacheError> {
    let cell = &*REDIS_CACHE;
    cell.get_or_try_init(|| async {
        let pool = redis_pool().map_err(CacheError::Io)?;
        let conn = pool
            .get()
            .await
            .map_err(|e| CacheError::Io(e.to_string()))?;
        let mux = deadpool_redis::Connection::take(conn);
        Ok(RedisCache::new(mux))
    })
    .await
}

/// Convenience wrappers so callers can use the mytheclipse `Cache` methods
/// directly without importing the trait twice.
pub async fn get(key: &str) -> Result<Option<Vec<u8>>, CacheError> {
    redis_cache().await?.get(key).await
}

pub async fn set(
    key: &str,
    value: Vec<u8>,
    ttl: Option<std::time::Duration>,
) -> Result<(), CacheError> {
    redis_cache().await?.set(key, value, ttl).await
}

pub async fn invalidate(key: &str) -> Result<(), CacheError> {
    redis_cache().await?.invalidate(key).await
}
