use chrono::{DateTime, Utc};
use serde::Serialize;
use utoipa::ToSchema;
use uuid::Uuid;

#[derive(Debug, Clone, Copy, PartialEq, Eq, sqlx::Type, Serialize, ToSchema)]
#[sqlx(type_name = "job_status", rename_all = "lowercase")]
#[serde(rename_all = "lowercase")]
pub enum JobStatus {
    Queued,
    Processing,
    Finished,
    Failed,
}

#[derive(Debug, Clone, sqlx::FromRow, ToSchema)]
pub struct Job {
    pub id: Uuid,
    pub status: JobStatus,
    pub format: Option<String>,
    pub filename: Option<String>,
    pub size_bytes: Option<i64>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct JobResponse {
    pub id: Uuid,
    pub status: JobStatus,
    pub format: Option<String>,
    pub filename: Option<String>,
    pub size_bytes: Option<i64>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct UploadResponse {
    pub job_id: Uuid,
    pub status: JobStatus,
    pub filename: String,
    pub size_bytes: u64,
}

impl From<Job> for JobResponse {
    fn from(job: Job) -> Self {
        Self {
            id: job.id,
            status: job.status,
            format: job.format,
            filename: job.filename,
            size_bytes: job.size_bytes,
            created_at: job.created_at,
            updated_at: job.updated_at,
        }
    }
}
