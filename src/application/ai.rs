//! Application use-cases for AI endpoints (Pollinations).

use crate::domain::error::ScrapingError;
use crate::infrastructure::repository::ai as Repo;
use serde_json::Value;

pub async fn chat(
    text: String,
    prompt: String,
    model: Option<String>,
) -> Result<Value, ScrapingError> {
    Repo::chat(text, prompt, model)
        .await
        .map_err(ScrapingError::Http)
}

pub async fn gemini(
    text: String,
    prompt: String,
    image_url: Option<String>,
) -> Result<Value, ScrapingError> {
    Repo::gemini(text, prompt, image_url)
        .await
        .map_err(ScrapingError::Http)
}

pub async fn image(
    prompt: String,
    model: String,
    width: u32,
    height: u32,
) -> Result<Vec<u8>, ScrapingError> {
    Repo::image(prompt, model, width, height)
        .await
        .map_err(ScrapingError::Http)
}
