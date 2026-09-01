//! Axum handlers for the uploader.
//!
//! POST /uploader/ryzencdn — multipart file upload, saved to local disk.
//! GET  /uploader/file/:name — serve an uploaded file.

use axum::body::Body;
use axum::extract::{Multipart, Path};
use axum::http::{header, StatusCode};
use axum::response::Response;
use axum::Json;
use serde_json::Value;

use crate::application::uploader as use_cases;
use crate::presentation::error::AppError;

/// POST /uploader/ryzencdn — accept a multipart file upload.
pub async fn ryzencdn_handler(mut multipart: Multipart) -> Result<Json<Value>, AppError> {
    // Read the first file field.
    let mut file_name = String::from("");
    let mut data: Vec<u8> = Vec::new();

    while let Some(field) = multipart
        .next_field()
        .await
        .map_err(|e| AppError::BadRequest(format!("multipart: {e}")))?
    {
        let name = field.name().unwrap_or("file").to_string();
        if name == "file" {
            file_name = field.file_name().unwrap_or("").to_string();
            let content = field
                .bytes()
                .await
                .map_err(|e| AppError::BadRequest(format!("bytes: {e}")))?;
            data = content.to_vec();
            break;
        }
    }

    if data.is_empty() {
        return Err(AppError::BadRequest(
            "No file field named 'file' in multipart body".into(),
        ));
    }

    let result = use_cases::upload(&data, &file_name)?;
    Ok(Json(result))
}

/// GET /uploader/file/:name — serve an uploaded file.
pub async fn serve_file_handler(Path(name): Path<String>) -> Result<Response<Body>, AppError> {
    let (bytes, mime) = use_cases::serve(&name)?;
    Ok(Response::builder()
        .status(StatusCode::OK)
        .header(header::CONTENT_TYPE, mime)
        .header(header::CACHE_CONTROL, "public, max-age=86400")
        .body(Body::from(bytes))
        .unwrap())
}
