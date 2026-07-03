use std::time::Duration;

use sqlx::PgPool;
use tracing::{error, info, warn};

use crate::services::{JobService, NormalizationEngine, StorageService};

pub fn spawn(pool: PgPool, storage: StorageService) {
    tokio::spawn(async move {
        let engine = NormalizationEngine::new();
        info!("background worker started");

        loop {
            match JobService::claim_next(&pool).await {
                Ok(Some(job)) => {
                    info!(job_id = %job.id, format = ?job.format, "processing job");

                    match engine.process_feed(&storage, &job).await {
                        Ok(()) => {
                            if let Err(db_error) = JobService::mark_finished(&pool, job.id).await {
                                error!(job_id = %job.id, %db_error, "failed to mark job finished");
                            } else {
                                info!(job_id = %job.id, "job finished");
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
