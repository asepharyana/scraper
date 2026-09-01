//! Application use-cases for stalk (profile/user lookups).

use crate::domain::error::ScrapingError;
use crate::infrastructure::repository::stalk as Repo;
use serde_json::Value;

/// GitHub user profile stalk.
pub async fn github(username: &str) -> Result<Value, ScrapingError> {
    Repo::fetch_github_stalk(username)
        .await
        .map_err(ScrapingError::Http)
}

/// YouTube channel/profile stalk.
pub async fn youtube(username: &str) -> Result<Value, ScrapingError> {
    Repo::fetch_youtube_stalk(username)
        .await
        .map_err(ScrapingError::Http)
}

/// Mobile Legends player stalk.
pub async fn mobile_legends(user_id: &str, zone_id: &str) -> Result<Value, ScrapingError> {
    Repo::fetch_ml_stalk(user_id, zone_id)
        .await
        .map_err(ScrapingError::Http)
}

/// Free Fire player stalk.
pub async fn free_fire(user_id: &str) -> Result<Value, ScrapingError> {
    Repo::fetch_ff_stalk(user_id)
        .await
        .map_err(ScrapingError::Http)
}

/// Genshin Impact player stalk.
pub async fn genshin(user_id: &str) -> Result<Value, ScrapingError> {
    Repo::fetch_genshin_stalk(user_id)
        .await
        .map_err(ScrapingError::Http)
}

/// Twitter/X user stalk.
pub async fn twitter(username: &str) -> Result<Value, ScrapingError> {
    Repo::fetch_twitter_stalk(username)
        .await
        .map_err(ScrapingError::Http)
}

/// TikTok user stalk.
pub async fn tiktok(username: &str) -> Result<Value, ScrapingError> {
    Repo::fetch_tiktok_stalk(username)
        .await
        .map_err(ScrapingError::Http)
}

/// Instagram user stalk.
pub async fn instagram(username: &str) -> Result<Value, ScrapingError> {
    Repo::fetch_instagram_stalk(username)
        .await
        .map_err(ScrapingError::Http)
}
