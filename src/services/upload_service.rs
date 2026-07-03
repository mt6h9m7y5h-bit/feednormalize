use axum::extract::multipart::Field;
use tokio::io::AsyncWriteExt;
use uuid::Uuid;

use crate::error::ApiError;
use crate::utils::original_file_path;

pub struct UploadService;

impl UploadService {
    pub async fn store_original_file(
        job_id: Uuid,
        mut field: Field<'_>,
    ) -> Result<(String, u64), ApiError> {
        let filename = field
            .file_name()
            .map(ToString::to_string)
            .unwrap_or_else(|| "upload".to_string());

        let path = original_file_path(job_id);
        if let Some(parent) = path.parent() {
            tokio::fs::create_dir_all(parent).await?;
        }

        let mut file = tokio::fs::File::create(&path).await?;
        let mut size_bytes = 0_u64;

        while let Some(chunk) = field.chunk().await.map_err(|error| {
            ApiError::BadRequest(format!("failed to read upload stream: {error}"))
        })? {
            size_bytes += chunk.len() as u64;
            file.write_all(&chunk).await?;
        }

        file.flush().await?;

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
