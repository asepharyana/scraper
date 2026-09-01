//! Infrastructure — Uploader utilities.
//!
//! The Shirokami-API source uploads to Ryzumi S3 (s3.ryzumi.vip) which is
//! DNS-dead from this VPS. This implementation stores uploaded files to a
//! local directory and serves them back, keeping the source's response shape:
//! `{ success, url, fileName, size }`.

use std::path::{Path, PathBuf};

use fastrand;
use serde_json::{json, Value};

/// Directory where uploaded files are stored.
const UPLOAD_DIR: &str = "/var/lib/scraper/uploads";

/// Base URL prefix used in the returned `url` field.
fn public_base() -> String {
    "/uploader/file".to_string()
}

/// Save an uploaded buffer to disk with a random filename + detected extension.
pub fn save_file(buffer: &[u8], file_name: &str) -> Result<Value, String> {
    if buffer.is_empty() {
        return Err("Invalid content: buffer is required".into());
    }

    let dir = PathBuf::from(UPLOAD_DIR);
    std::fs::create_dir_all(&dir).map_err(|e| format!("mkdir: {e}"))?;

    // Detect extension: prefer the provided filename, else infer from magic.
    let extension = if file_name.contains('.') {
        file_name.split('.').last().unwrap_or("").to_lowercase()
    } else {
        infer_extension(buffer).to_string()
    };

    if extension.is_empty() {
        return Err("Unable to determine file extension".into());
    }

    let random = hex::encode(&fastrand::u64(..).to_le_bytes());
    let key = format!("{random}.{extension}");
    let path = dir.join(&key);

    std::fs::write(&path, buffer).map_err(|e| format!("write: {e}"))?;

    let url = format!("{}/{}", public_base(), key);
    Ok(json!({
        "success": true,
        "url": url,
        "fileName": key,
        "size": buffer.len(),
    }))
}

/// Serve the file bytes for a given filename.
pub fn read_file(file_name: &str) -> Result<(Vec<u8>, String), String> {
    // Prevent path traversal.
    let safe = Path::new(file_name)
        .file_name()
        .and_then(|s| s.to_str())
        .ok_or("invalid filename")?;
    let path = PathBuf::from(UPLOAD_DIR).join(safe);
    let bytes = std::fs::read(&path).map_err(|e| format!("read: {e}"))?;
    let mime = infer_mime(safe).to_string();
    Ok((bytes, mime))
}

fn infer_extension(buf: &[u8]) -> &'static str {
    if buf.starts_with(b"\x89PNG\r\n\x1a\n") {
        "png"
    } else if buf.starts_with(b"\xff\xd8\xff") {
        "jpg"
    } else if buf.starts_with(b"GIF87a") || buf.starts_with(b"GIF89a") {
        "gif"
    } else if buf.starts_with(b"RIFF") && &buf[8..12] == b"WEBP" {
        "webp"
    } else if buf.starts_with(b"%PDF") {
        "pdf"
    } else if buf.starts_with(b"PK\x03\x04") {
        "zip"
    } else {
        "bin"
    }
}

fn infer_mime(name: &str) -> &'static str {
    let ext = name.rsplit('.').next().unwrap_or("");
    match ext {
        "png" => "image/png",
        "jpg" | "jpeg" => "image/jpeg",
        "gif" => "image/gif",
        "webp" => "image/webp",
        "pdf" => "application/pdf",
        "zip" => "application/zip",
        "txt" => "text/plain",
        "mp4" => "video/mp4",
        "webm" => "video/webm",
        "mp3" => "audio/mpeg",
        _ => "application/octet-stream",
    }
}
