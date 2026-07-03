use axum::{
    Json, Router,
    body::Body,
    extract::{Path, State},
    http::{HeaderMap, HeaderValue, StatusCode, header},
    response::{IntoResponse, Response},
    routing::{get, post},
};
use serde::{Deserialize, Serialize};
use tracing::info;
use utoipa::ToSchema;
use uuid::Uuid;

use crate::error::{ApiError, ErrorBody};
use crate::models::{JobResponse, JobStatus, UniversalProduct};
use crate::services::{JobService, NormalizationEngine, normalized_output_key};
use crate::state::AppState;
use crate::validation::{ValidationIssue, ValidationResult, ValidationSummary};

pub fn protected_router(state: AppState) -> Router {
    Router::new()
        .route("/jobs", post(create_job))
        .route("/jobs/{id}", get(get_job))
        .route("/jobs/{id}/report", get(get_job_report))
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
    summary = "Queue a normalization job",
    description = "Creates an async normalization job. Optionally pass a `preview` record to see how supplier field names map to the universal schema before uploading a full feed. Returns `202 Accepted` with a `job_id` for status polling.",
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

#[derive(Debug, Serialize, ToSchema)]
pub struct JobReportResponse {
    pub job_id: Uuid,
    pub status: JobStatus,
    pub summary: ValidationSummary,
    pub issues: Vec<ValidationIssue>,
}

/// Get validation report for a completed job.
#[utoipa::path(
    get,
    path = "/jobs/{id}/report",
    summary = "Get job validation report",
    description = "Returns field-level validation results produced after normalization. Available when job `status` is `finished` or `completed_with_errors`. Jobs completed before validation was enabled return an empty report.",
    params(
        ("id" = Uuid, Path, description = "Job identifier"),
    ),
    security(("ApiKeyAuth" = [])),
    responses(
        (status = 200, description = "Validation report", body = JobReportResponse),
        (status = 400, description = "Job not complete", body = ErrorBody),
        (status = 401, description = "Missing or invalid API key", body = ErrorBody),
        (status = 404, description = "Job not found", body = ErrorBody),
        (status = 429, description = "Rate limit exceeded", body = ErrorBody),
        (status = 500, description = "Internal server error", body = ErrorBody),
    ),
    tag = "Jobs"
)]
pub async fn get_job_report(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
) -> Result<Json<JobReportResponse>, ApiError> {
    info!(job_id = %id, "job report requested");

    let (status, job_report) = JobService::find_report(&state.db, id).await?;

    match status {
        JobStatus::Finished | JobStatus::CompletedWithErrors => {}
        JobStatus::Failed => {
            return Err(ApiError::NotFound(format!(
                "validation report for job {id} not found"
            )));
        }
        _ => {
            return Err(ApiError::BadRequest(format!(
                "job {id} is not complete (status: {status:?})"
            )));
        }
    }

    let report = job_report
        .map(|value| serde_json::from_value::<ValidationResult>(value))
        .transpose()
        .map_err(|error| ApiError::BadRequest(format!("invalid stored job report: {error}")))?
        .unwrap_or_else(ValidationResult::empty);

    info!(
        job_id = %id,
        status = ?status,
        errors = report.summary.errors,
        warnings = report.summary.warnings,
        "job report fetched"
    );

    Ok(Json(JobReportResponse {
        job_id: id,
        status,
        summary: report.summary,
        issues: report.issues,
    }))
}

/// Get the current status of a normalization job.
#[utoipa::path(
    get,
    path = "/jobs/{id}",
    summary = "Get job status and metadata",
    description = "Poll this endpoint after upload or job creation. When `status` is `finished` or `completed_with_errors`, call `GET /jobs/{id}/download` to retrieve normalized JSON output and `GET /jobs/{id}/report` for validation details.",
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

    info!(job_id = %id, status = ?job.status, "job status fetched");

    Ok(Json(job.into()))
}

/// Download normalized JSON output for a finished job.
#[utoipa::path(
    get,
    path = "/jobs/{id}/download",
    summary = "Download normalized JSON output",
    description = "Returns the normalized product catalog as a JSON file attachment. Available when job `status` is `finished` or `completed_with_errors`. Each item follows the `UniversalProduct` schema with automatically mapped fields (sku, title, price, currency, ean).",
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

    if !matches!(
        job.status,
        JobStatus::Finished | JobStatus::CompletedWithErrors
    ) {
        return Err(ApiError::BadRequest(format!(
            "job {id} is not ready for download (status: {:?})",
            job.status
        )));
    }

    let output_key = normalized_output_key(id);

    if !state.storage.object_exists(&output_key).await? {
        return Err(ApiError::NotFound(format!(
            "normalized output for job {id} not found"
        )));
    }

    let bytes = state.storage.get_object(&output_key).await?;
    let size_bytes = bytes.len();
    let body = Body::from(bytes);

    info!(job_id = %id, size_bytes, "job download");

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
    summary = "Check API health",
    description = "Lightweight liveness check. No authentication required. Use for uptime monitoring and RapidAPI health probes.",
    responses(
        (status = 200, description = "Service is healthy", body = HealthResponse),
    ),
    tag = "Health"
)]
pub async fn health() -> Json<HealthResponse> {
    Json(HealthResponse { status: "ok" })
}
