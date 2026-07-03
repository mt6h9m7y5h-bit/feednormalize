use axum::{
    Json, Router,
    body::Body,
    extract::{Path, State},
    http::{HeaderMap, HeaderValue, StatusCode, header},
    response::{IntoResponse, Response},
    routing::{get, post},
};
use serde::{Deserialize, Serialize};
use tokio_util::io::ReaderStream;
use tracing::info;
use utoipa::ToSchema;
use uuid::Uuid;

use crate::error::{ApiError, ErrorBody};
use crate::models::{JobResponse, JobStatus, UniversalProduct};
use crate::services::{JobService, NormalizationEngine};
use crate::state::AppState;
use crate::utils::{ensure_within_uploads, normalized_output_path};

pub fn protected_router(state: AppState) -> Router {
    Router::new()
        .route("/jobs", post(create_job))
        .route("/jobs/{id}", get(get_job))
        .route("/jobs/{id}/download", get(download_job))
        .with_state(state)
}

#[derive(Debug, Deserialize, ToSchema)]
pub struct CreateJobRequest {
    /// Source feed format, e.g. "csv" or "json".
    pub format: Option<String>,
    /// Optional preview record for early normalization checks.
    #[schema(value_type = Option<Object>)]
    pub preview: Option<serde_json::Value>,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct CreateJobResponse {
    pub job_id: Uuid,
    pub status: JobStatus,
    pub format: Option<String>,
    pub preview_product: Option<UniversalProduct>,
    pub created_at: chrono::DateTime<chrono::Utc>,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct HealthResponse {
    pub status: &'static str,
}

/// Queue a new normalization job.
#[utoipa::path(
    post,
    path = "/jobs",
    request_body = CreateJobRequest,
    security(("ApiKeyAuth" = [])),
    responses(
        (status = 202, description = "Job accepted and queued", body = CreateJobResponse),
        (status = 400, description = "Invalid request", body = ErrorBody),
        (status = 401, description = "Missing or invalid API key", body = ErrorBody),
        (status = 429, description = "Rate limit exceeded", body = ErrorBody),
        (status = 500, description = "Internal server error", body = ErrorBody),
    ),
    tag = "Jobs"
)]
pub async fn create_job(
    State(state): State<AppState>,
    Json(payload): Json<CreateJobRequest>,
) -> Result<(StatusCode, Json<CreateJobResponse>), ApiError> {
    let job = JobService::create(&state.db, payload.format.clone()).await?;

    let preview_product = payload
        .preview
        .as_ref()
        .map(|raw| NormalizationEngine::new().normalize(raw));

    info!(job_id = %job.id, format = ?payload.format, "job queued");

    Ok((
        StatusCode::ACCEPTED,
        Json(CreateJobResponse {
            job_id: job.id,
            status: job.status,
            format: job.format,
            preview_product,
            created_at: job.created_at,
        }),
    ))
}

/// Get the current status of a normalization job.
#[utoipa::path(
    get,
    path = "/jobs/{id}",
    params(
        ("id" = Uuid, Path, description = "Job identifier"),
    ),
    security(("ApiKeyAuth" = [])),
    responses(
        (status = 200, description = "Job details", body = JobResponse),
        (status = 401, description = "Missing or invalid API key", body = ErrorBody),
        (status = 404, description = "Job not found", body = ErrorBody),
        (status = 429, description = "Rate limit exceeded", body = ErrorBody),
        (status = 500, description = "Internal server error", body = ErrorBody),
    ),
    tag = "Jobs"
)]
pub async fn get_job(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
) -> Result<Json<JobResponse>, ApiError> {
    let job = JobService::find_by_id(&state.db, id).await?;
    Ok(Json(job.into()))
}

/// Download normalized JSON output for a finished job.
#[utoipa::path(
    get,
    path = "/jobs/{id}/download",
    params(
        ("id" = Uuid, Path, description = "Job identifier"),
    ),
    security(("ApiKeyAuth" = [])),
    responses(
        (status = 200, description = "Normalized JSON file", content_type = "application/json"),
        (status = 400, description = "Job not ready for download", body = ErrorBody),
        (status = 401, description = "Missing or invalid API key", body = ErrorBody),
        (status = 404, description = "Job or output not found", body = ErrorBody),
        (status = 429, description = "Rate limit exceeded", body = ErrorBody),
        (status = 500, description = "Internal server error", body = ErrorBody),
    ),
    tag = "Jobs"
)]
pub async fn download_job(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
) -> Result<Response, ApiError> {
    let job = JobService::find_by_id(&state.db, id).await?;

    if job.status != JobStatus::Finished {
        return Err(ApiError::BadRequest(format!(
            "job {id} is not ready for download (status: {:?})",
            job.status
        )));
    }

    let path = normalized_output_path(id);

    if !path.exists() {
        return Err(ApiError::NotFound(format!(
            "normalized output for job {id} not found"
        )));
    }

    ensure_within_uploads(&path)?;

    let file = tokio::fs::File::open(&path).await?;
    let stream = ReaderStream::new(file);
    let body = Body::from_stream(stream);

    let download_name = job
        .filename
        .as_deref()
        .map(|name| format!("normalized-{name}"))
        .unwrap_or_else(|| format!("normalized-{id}.json"));

    let mut headers = HeaderMap::new();
    headers.insert(
        header::CONTENT_TYPE,
        HeaderValue::from_static("application/json"),
    );
    headers.insert(
        header::CONTENT_DISPOSITION,
        HeaderValue::from_str(&format!("attachment; filename=\"{download_name}\""))
            .map_err(|error| ApiError::BadRequest(format!("invalid download filename: {error}")))?,
    );

    Ok((StatusCode::OK, headers, body).into_response())
}

/// Check API health.
#[utoipa::path(
    get,
    path = "/health",
    responses(
        (status = 200, description = "Service is healthy", body = HealthResponse),
    ),
    tag = "Health"
)]
pub async fn health() -> Json<HealthResponse> {
    Json(HealthResponse { status: "ok" })
}
