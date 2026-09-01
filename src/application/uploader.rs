//! Application use-cases for the uploader.

use crate::domain::error::ScrapingError;
use crate::infrastructure::repository::uploader as Repo;
use serde_json::Value;

pub fn upload(buffer: &[u8], file_name: &str) -> Result<Value, ScrapingError> {
    Repo::save_file(buffer, file_name).map_err(ScrapingError::Http)
}

pub fn serve(file_name: &str) -> Result<(Vec<u8>, String), ScrapingError> {
    Repo::read_file(file_name).map_err(ScrapingError::Http)
}
