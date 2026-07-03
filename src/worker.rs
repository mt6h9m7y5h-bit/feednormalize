use std::time::Duration;

use sqlx::PgPool;
use tracing::{error, info, warn};

use crate::models::JobStatus;
use crate::services::{JobService, NormalizationEngine, StorageService};
use crate::validation::ValidationEngine;

pub fn spawn(pool: PgPool, storage: StorageService) {
    tokio::spawn(async move {
        let engine = NormalizationEngine::new();
        let validator = ValidationEngine::new();
        info!("background worker started");

        loop {
            match JobService::claim_next(&pool).await {
                Ok(Some(job)) => {
                    info!(job_id = %job.id, format = ?job.format, "processing job");

                    match engine.process_feed(&storage, &job).await {
                        Ok(products) => {
                            let report = validator.validate_products(&products);
                            let status = if report.summary.errors > 0 {
                                JobStatus::CompletedWithErrors
                            } else {
                                JobStatus::Finished
                            };

                            if let Err(db_error) =
                                JobService::complete_with_report(&pool, job.id, status, Some(&report))
                                    .await
                            {
                                error!(job_id = %job.id, %db_error, "failed to complete job");
                            } else {
                                info!(
                                    job_id = %job.id,
                                    status = ?status,
                                    errors = report.summary.errors,
                                    warnings = report.summary.warnings,
                                    "job completed"
                                );
                            }
                        }
                        Err(process_error) => {
                            error!(job_id = %job.id, %process_error, "job processing failed");

                            if let Err(db_error) = JobService::mark_failed(&pool, job.id).await {
                                error!(job_id = %job.id, %db_error, "failed to mark job failed");
                            }
                        }
                    }
                }
                Ok(None) => {}
                Err(db_error) => {
                    warn!(%db_error, "failed to claim next job");
                }
            }

            tokio::time::sleep(Duration::from_secs(5)).await;
        }
    });
}
