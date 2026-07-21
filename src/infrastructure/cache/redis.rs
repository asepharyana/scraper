//! Redis caching helpers.

use deadpool_redis::redis::AsyncCommands;
use deadpool_redis::Pool;
use serde::{de::DeserializeOwned, Serialize};
use tracing::{debug, error};

/// Default cache TTL in seconds (5 minutes).
pub const DEFAULT_CACHE_TTL: u64 = 300;

/// Cache helper for Redis operations.
pub struct Cache<'a> {
    pool: &'a Pool,
}

impl<'a> Cache<'a> {
    pub fn new(pool: &'a Pool) -> Self {
        Self { pool }
    }

    pub async fn get<T: DeserializeOwned>(&self, key: &str) -> Option<T> {
        let mut conn = match self.pool.get().await {
            Ok(c) => c,
            Err(e) => {
                error!("Cache: failed to get connection: {}", e);
                return None;
            }
        };

        let cached: Option<String> = conn.get(key).await.ok()?;

        if cached.is_some() {
            debug!("Cache hit: {}", key);
        } else {
            debug!("Cache miss: {}", key);
        }

        cached.and_then(|json| serde_json::from_str(&json).ok())
    }

    pub async fn mget<T: DeserializeOwned>(&self, keys: &[String]) -> Vec<Option<T>> {
        if keys.is_empty() {
            return Vec::new();
        }

        let mut conn = match self.pool.get().await {
            Ok(c) => c,
            Err(e) => {
                error!("Cache: failed to get connection: {}", e);
                return std::iter::repeat_with(|| None).take(keys.len()).collect();
            }
        };

        use deadpool_redis::redis::cmd;
        let cached_values: Vec<Option<String>> =
            match cmd("MGET").arg(keys).query_async(&mut conn).await {
                Ok(v) => v,
                Err(e) => {
                    error!("Cache: failed to mget values: {}", e);
                    return std::iter::repeat_with(|| None).take(keys.len()).collect();
                }
            };

        cached_values
            .into_iter()
            .map(|opt_s| opt_s.and_then(|json| serde_json::from_str(&json).ok()))
            .collect()
    }

    pub async fn set<T: Serialize>(&self, key: &str, value: &T) -> Result<(), String> {
        self.set_with_ttl(key, value, DEFAULT_CACHE_TTL).await
    }

    pub async fn set_with_ttl<T: Serialize>(
        &self,
        key: &str,
        value: &T,
        ttl_secs: u64,
    ) -> Result<(), String> {
        let mut conn = self.pool.get().await.map_err(|e| e.to_string())?;
        let json = serde_json::to_string(value).map_err(|e| e.to_string())?;
        conn.set_ex::<_, _, ()>(key, json, ttl_secs)
            .await
            .map_err(|e| e.to_string())?;
        debug!("Cache: set key {} with TTL {}s", key, ttl_secs);
        Ok(())
    }

    pub async fn delete(&self, key: &str) -> Result<(), String> {
        let mut conn = self.pool.get().await.map_err(|e| e.to_string())?;
        conn.del::<_, ()>(key).await.map_err(|e| e.to_string())?;
        debug!("Cache: deleted key {}", key);
        Ok(())
    }

    pub async fn exists(&self, key: &str) -> bool {
        let mut conn = match self.pool.get().await {
            Ok(c) => c,
            Err(_) => return false,
        };
        conn.exists::<_, bool>(key).await.unwrap_or(false)
    }

    /// Get or set: returns cached value or computes and caches new value.
    pub async fn get_or_set<T, F, Fut>(
        &self,
        key: &str,
        ttl_secs: u64,
        compute: F,
    ) -> Result<T, String>
    where
        T: Serialize + DeserializeOwned,
        F: FnOnce() -> Fut,
        Fut: std::future::Future<Output = Result<T, String>>,
    {
        if let Some(cached) = self.get::<T>(key).await {
            debug!("Cache hit: {}", key);
            return Ok(cached);
        }

        debug!("Cache miss: {}", key);
        let value = compute().await?;
        self.set_with_ttl(key, &value, ttl_secs).await?;
        Ok(value)
    }
}

/// Create a cache key with prefix.
pub fn cache_key(prefix: &str, id: &str) -> String {
    format!("{}:{}", prefix, id)
}
