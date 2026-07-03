use serde_json::Value;
use sqlx::{PgPool, Row};
use uuid::Uuid;

use crate::error::ApiError;
use crate::models::{Job, JobStatus};
use crate::validation::ValidationResult;

const JOB_COLUMNS: &str =
    "id, status, format, filename, size_bytes, created_at, updated_at";

pub struct JobService;

impl JobService {
    pub async fn create(pool: &PgPool, format: Option<String>) -> Result<Job, ApiError> {
        let id = Uuid::new_v4();

        let query = format!(
            r#"
            INSERT INTO jobs (id, status, format)
            VALUES ($1, $2, $3)
            RETURNING {JOB_COLUMNS}
            "#
        );

        let job = sqlx::query_as::<_, Job>(&query)
            .bind(id)
            .bind(JobStatus::Queued)
            .bind(format)
            .fetch_one(pool)
            .await?;

        Ok(job)
    }

    pub async fn create_processing(
        pool: &PgPool,
        format: Option<String>,
    ) -> Result<Job, ApiError> {
        let id = Uuid::new_v4();

        let query = format!(
            r#"
            INSERT INTO jobs (id, status, format)
            VALUES ($1, $2, $3)
            RETURNING {JOB_COLUMNS}
            "#
        );

        let job = sqlx::query_as::<_, Job>(&query)
            .bind(id)
            .bind(JobStatus::Processing)
            .bind(format)
            .fetch_one(pool)
            .await?;

        Ok(job)
    }

    pub async fn find_by_id(pool: &PgPool, id: Uuid) -> Result<Job, ApiError> {
        let query = format!(
            r#"
            SELECT {JOB_COLUMNS}
            FROM jobs
            WHERE id = $1
            "#
        );

        let job = sqlx::query_as::<_, Job>(&query)
            .bind(id)
            .fetch_optional(pool)
            .await?
            .ok_or_else(|| ApiError::NotFound(format!("job {id} not found")))?;

        Ok(job)
    }

    pub async fn mark_failed(pool: &PgPool, id: Uuid) -> Result<(), sqlx::Error> {
        sqlx::query(
            r#"
            UPDATE jobs
            SET status = $2, updated_at = NOW()
            WHERE id = $1
            "#,
        )
        .bind(id)
        .bind(JobStatus::Failed)
        .execute(pool)
        .await?;

        Ok(())
    }

    pub async fn mark_finished(pool: &PgPool, id: Uuid) -> Result<(), sqlx::Error> {
        Self::complete_with_report(pool, id, JobStatus::Finished, None).await
    }

    pub async fn complete_with_report(
        pool: &PgPool,
        id: Uuid,
        status: JobStatus,
        report: Option<&ValidationResult>,
    ) -> Result<(), sqlx::Error> {
        let report_json = report
            .map(serde_json::to_value)
            .transpose()
            .map_err(|error| sqlx::Error::Decode(Box::new(error)))?;

        sqlx::query(
            r#"
            UPDATE jobs
            SET status = $2, job_report = $3, updated_at = NOW()
            WHERE id = $1
            "#,
        )
        .bind(id)
        .bind(status)
        .bind(report_json)
        .execute(pool)
        .await?;

        Ok(())
    }

    pub async fn find_report(
        pool: &PgPool,
        id: Uuid,
    ) -> Result<(JobStatus, Option<Value>), ApiError> {
        let row = sqlx::query(
            r#"
            SELECT status, job_report
            FROM jobs
            WHERE id = $1
            "#,
        )
        .bind(id)
        .fetch_optional(pool)
        .await?
        .ok_or_else(|| ApiError::NotFound(format!("job {id} not found")))?;

        let status: JobStatus = row.try_get("status")?;
        let job_report: Option<Value> = row.try_get("job_report")?;

        Ok((status, job_report))
    }

    /// Atomically claims the next job that has an uploaded file ready to parse.
    pub async fn claim_next(pool: &PgPool) -> Result<Option<Job>, sqlx::Error> {
        let query = format!(
            r#"
            UPDATE jobs
            SET status = $1, updated_at = NOW()
            WHERE id = (
                SELECT id
                FROM jobs
                WHERE status IN ($2, $1)
                  AND filename IS NOT NULL
                ORDER BY created_at ASC
                LIMIT 1
                FOR UPDATE SKIP LOCKED
            )
            RETURNING {JOB_COLUMNS}
            "#
        );

        let job = sqlx::query_as::<_, Job>(&query)
            .bind(JobStatus::Processing)
            .bind(JobStatus::Queued)
            .fetch_optional(pool)
            .await?;

        Ok(job)
    }

    pub async fn mark_processing(
        pool: &PgPool,
        id: Uuid,
        filename: &str,
        size_bytes: i64,
        format: Option<String>,
    ) -> Result<Job, ApiError> {
        let query = format!(
            r#"
            UPDATE jobs
            SET status = $2,
                filename = $3,
                size_bytes = $4,
                format = COALESCE($5, format),
                updated_at = NOW()
            WHERE id = $1
            RETURNING {JOB_COLUMNS}
            "#
        );

        let job = sqlx::query_as::<_, Job>(&query)
            .bind(id)
            .bind(JobStatus::Processing)
            .bind(filename)
            .bind(size_bytes)
            .bind(format)
            .fetch_optional(pool)
            .await?
            .ok_or_else(|| ApiError::NotFound(format!("job {id} not found")))?;

        Ok(job)
    }
}
