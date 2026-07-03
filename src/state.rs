use std::sync::Arc;

use sqlx::PgPool;

use crate::rate_limit::RateLimiter;
use crate::services::StorageService;

#[derive(Clone)]
pub struct AppState {
    pub db: PgPool,
    pub rate_limiter: Arc<RateLimiter>,
    pub storage: StorageService,
}

impl AppState {
    pub fn new(db: PgPool, rate_limiter: Arc<RateLimiter>, storage: StorageService) -> Self {
        Self {
            db,
            rate_limiter,
            storage,
        }
    }
}
