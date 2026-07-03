use axum::{
    Json, Router,
    extract::{Multipart, State},
    http::StatusCode,
    routing::post,
};
use tracing::info;
use utoipa::ToSchema;
use uuid::Uuid;

use crate::error::{ApiError, ErrorBody};
use crate::models::UploadResponse;
use crate::services::{JobService, UploadService, infer_format};
use crate::state::AppState;

pub fn router(state: AppState) -> Router {
    Router::new()
        .route("/feeds/upload", post(upload_feed))
        .with_state(state)
}

/// Multipart upload payload for feed files.
#[derive(ToSchema)]
#[allow(dead_code)]
pub struct UploadForm {
    /// Optional existing job ID to attach the upload to.
    #[schema(value_type = Option<String>, format = Uuid)]
    job_id: Option<String>,
    /// Product feed file (CSV or JSON).
    #[schema(value_type = String, format = Binary)]
    file: String,
}

/// Upload a product feed file for normalization.
#[utoipa::path(
    post,
    path = "/feeds/upload",
    summary = "Upload a product feed for normalization",
    description = "Primary entry point for feed normalization. Send `multipart/form-data` with a required `file` field (CSV or JSON). Optionally include `job_id` to attach the upload to an existing job (must be sent before the file field). Processing runs asynchronously; poll `GET /jobs/{id}` until finished.",
    request_body(
        content = UploadForm,
        content_type = "multipart/form-data",
        description = "Multipart form with optional job_id and required file field",
    ),
    security(("ApiKeyAuth" = [])),
    responses(
        (status = 202, description = "Feed accepted for processing", body = UploadResponse),
        (status = 400, description = "Invalid multipart payload", body = ErrorBody),
        (status = 401, description = "Missing or invalid API key", body = ErrorBody),
        (status = 404, description = "Referenced job not found", body = ErrorBody),
        (status = 429, description = "Rate limit exceeded", body = ErrorBody),
        (status = 500, description = "Internal server error", body = ErrorBody),
    ),
    tag = "Feeds"
)]
pub async fn upload_feed(
    State(state): State<AppState>,
    mut multipart: Multipart,
) -> Result<(StatusCode, Json<UploadResponse>), ApiError> {
    let mut job_id: Option<Uuid> = None;
    let mut upload: Option<(Uuid, String, u64, Option<String>)> = None;

    while let Some(field) = multipart.next_field().await.map_err(|error| {
        ApiError::BadRequest(format!("invalid multipart payload: {error}"))
    })? {
        match field.name() {
            Some("job_id") if upload.is_none() => {
                let value = field.text().await.map_err(|error| {
                    ApiError::BadRequest(format!("invalid job_id field: {error}"))
                })?;
                job_id = Some(Uuid::parse_str(value.trim()).map_err(|error| {
                    ApiError::BadRequest(format!("invalid job_id uuid: {error}"))
                })?);
            }
            Some("job_id") => {
                return Err(ApiError::BadRequest(
                    "job_id must be sent before the file field".into(),
                ));
            }
            Some("file") if upload.is_none() => {
                let detected_format = infer_format(
                    field.file_name().as_deref(),
                    field.content_type().map(|mime| mime.as_ref()),
                );

                let job = match job_id {
                    Some(id) => JobService::find_by_id(&state.db, id).await?,
                    None => JobService::create(&state.db, detected_format.clone()).await?,
                };

                let (filename, size_bytes) =
                    match UploadService::store_original_file(&state.storage, job.id, field).await
                    {
                        Ok(uploaded) => uploaded,
                        Err(error) => {
                            let _ = JobService::mark_failed(&state.db, job.id).await;
                            return Err(error);
                        }
                    };

                upload = Some((job.id, filename, size_bytes, detected_format));
            }
            Some("file") => {
                return Err(ApiError::BadRequest(
                    "only one file field is allowed".to_owned(),
                ));
            }
            _ => {}
        }
    }

    let (job_id, filename, size_bytes, format) =
        upload.ok_or_else(|| ApiError::BadRequest("missing file field".into()))?;

    let job = JobService::mark_processing(
        &state.db,
        job_id,
        &filename,
        i64::try_from(size_bytes).map_err(|_| {
            ApiError::BadRequest("upload exceeds maximum supported size".into())
        })?,
        format,
    )
    .await?;

    info!(
        job_id = %job.id,
        %filename,
        size_bytes,
        "feed uploaded"
    );

    Ok((
        StatusCode::ACCEPTED,
        Json(UploadResponse {
            job_id: job.id,
            status: job.status,
            filename,
            size_bytes,
        }),
    ))
}
