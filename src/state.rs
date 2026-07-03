use std::sync::Arc;

use sqlx::PgPool;

use crate::rate_limit::RateLimiter;

#[derive(Clone)]
pub struct AppState {
    pub db: PgPool,
    pub rate_limiter: Arc<RateLimiter>,
}

impl AppState {
    pub fn new(db: PgPool, rate_limiter: Arc<RateLimiter>) -> Self {
        Self { db, rate_limiter }
    }
}
