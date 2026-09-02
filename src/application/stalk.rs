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

/// Twitter/X user stalk.
pub async fn twitter(username: &str) -> Result<Value, ScrapingError> {
    Repo::fetch_twitter_stalk(username)
        .await
        .map_err(ScrapingError::Http)
}
