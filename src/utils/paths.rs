use std::path::{Path, PathBuf};

use uuid::Uuid;

use crate::error::ApiError;

const UPLOADS_DIR: &str = "uploads";
const ORIGINAL_FILE: &str = "original_file";
const NORMALIZED_OUTPUT: &str = "normalized_output.json";

pub fn job_dir(job_id: Uuid) -> PathBuf {
    PathBuf::from(UPLOADS_DIR).join(job_id.to_string())
}

pub fn original_file_path(job_id: Uuid) -> PathBuf {
    job_dir(job_id).join(ORIGINAL_FILE)
}

pub fn normalized_output_path(job_id: Uuid) -> PathBuf {
    job_dir(job_id).join(NORMALIZED_OUTPUT)
}

/// Ensures a resolved path stays inside the configured uploads directory.
pub fn ensure_within_uploads(path: &Path) -> Result<(), ApiError> {
    let uploads_root = PathBuf::from(UPLOADS_DIR);
    let canonical_root = uploads_root
        .canonicalize()
        .unwrap_or_else(|_| uploads_root.clone());

    let canonical_path = path
        .canonicalize()
        .map_err(|_| ApiError::NotFound("file not found".into()))?;

    if !canonical_path.starts_with(&canonical_root) {
        return Err(ApiError::BadRequest("invalid file path".into()));
    }

    Ok(())
}
