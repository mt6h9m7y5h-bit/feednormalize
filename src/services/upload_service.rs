use axum::extract::multipart::Field;
use bytes::Bytes;
use uuid::Uuid;

use crate::error::ApiError;
use crate::services::{StorageService, original_file_key};

pub struct UploadService;

impl UploadService {
    pub async fn store_original_file(
        storage: &StorageService,
        job_id: Uuid,
        mut field: Field<'_>,
    ) -> Result<(String, u64), ApiError> {
        let filename = field
            .file_name()
            .map(ToString::to_string)
            .unwrap_or_else(|| "upload".to_string());

        let mut data = Vec::new();
        let mut size_bytes = 0_u64;

        while let Some(chunk) = field.chunk().await.map_err(|error| {
            ApiError::BadRequest(format!("failed to read upload stream: {error}"))
        })? {
            size_bytes += chunk.len() as u64;
            data.extend_from_slice(&chunk);
        }

        storage
            .put_object(&original_file_key(job_id), Bytes::from(data))
            .await?;

        Ok((filename, size_bytes))
    }
}

pub fn infer_format_from_filename(filename: &str) -> Option<String> {
    let extension = filename.rsplit('.').next()?.to_ascii_lowercase();

    match extension.as_str() {
        "csv" => Some("csv".to_owned()),
        "json" => Some("json".to_owned()),
        "ndjson" => Some("ndjson".to_owned()),
        "xml" => Some("xml".to_owned()),
        "tsv" => Some("tsv".to_owned()),
        _ => None,
    }
}

pub fn infer_format(filename: Option<&str>, content_type: Option<&str>) -> Option<String> {
    if let Some(content_type) = content_type {
        let content_type = content_type.to_ascii_lowercase();

        if content_type.contains("application/json") || content_type.contains("text/json") {
            return Some("json".to_owned());
        }

        if content_type.contains("text/csv") || content_type.contains("application/csv") {
            return Some("csv".to_owned());
        }

        if content_type.contains("tab-separated") {
            return Some("tsv".to_owned());
        }
    }

    filename.and_then(infer_format_from_filename)
}
