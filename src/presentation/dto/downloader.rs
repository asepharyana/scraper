//! Presentation DTOs for media downloader endpoints.

use serde::Serialize;
use utoipa::ToSchema;

use crate::domain::entity::downloader::DownloadResult;

/// Standard response wrapper for all downloader endpoints.
///
/// Mirrors the Shirokami-API pattern of `{ success, ...data }` but
/// adds `status` and `message` for consistency with the anime/komik modules.
#[derive(Serialize, Debug, Clone, ToSchema)]
pub struct DownloadResponse {
    pub success: bool,
    pub status: i16,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub message: Option<String>,
    pub data: Option<DownloadResult>,
}

impl From<DownloadResult> for DownloadResponse {
    fn from(data: DownloadResult) -> Self {
        let success = data.status == crate::domain::entity::downloader::DownloadStatus::Success;
        Self {
            success,
            status: if success { 200 } else { 400 },
            message: data.message.clone(),
            data: Some(data),
        }
    }
}

impl DownloadResponse {
    pub fn ok(data: DownloadResult) -> Self {
        let success = data.status == crate::domain::entity::downloader::DownloadStatus::Success;
        let msg = data.message.clone();
        Self {
            success,
            status: 200,
            message: msg,
            data: Some(data),
        }
    }

    pub fn error(status: i16, message: impl Into<String>) -> Self {
        Self {
            success: false,
            status,
            message: Some(message.into()),
            data: None,
        }
    }
}
