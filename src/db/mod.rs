use std::time::Duration;

use sqlx::postgres::PgPoolOptions;
use sqlx::PgPool;
use tracing::info;

use crate::services::ApiKeyService;

pub async fn init_pool(database_url: &str) -> Result<PgPool, sqlx::Error> {
    let pool = PgPoolOptions::new()
        .max_connections(10)
        .acquire_timeout(Duration::from_secs(5))
        .connect(database_url)
        .await?;

    sqlx::migrate!().run(&pool).await?;
    ApiKeyService::ensure_seed(&pool).await?;

    info!("database connected and migrations applied");

    Ok(pool)
}
