use sha2::{Digest, Sha256};
use sqlx::PgPool;
use uuid::Uuid;

use crate::models::AuthenticatedApiKey;

pub fn hash_api_key(key: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(key.as_bytes());
    hex::encode(hasher.finalize())
}

pub struct ApiKeyService;

impl ApiKeyService {
    pub async fn validate(pool: &PgPool, raw_key: &str) -> Result<Option<AuthenticatedApiKey>, sqlx::Error> {
        let key_hash = hash_api_key(raw_key);

        let row = sqlx::query_as::<_, (Uuid, i32)>(
            r#"
            SELECT id, rate_limit_per_minute
            FROM api_keys
            WHERE key_hash = $1 AND is_active = TRUE
            "#,
        )
        .bind(key_hash)
        .fetch_optional(pool)
        .await?;

        Ok(row.map(|(id, rate_limit_per_minute)| AuthenticatedApiKey {
            id,
            rate_limit_per_minute,
        }))
    }

    pub async fn ensure_seed(pool: &PgPool) -> Result<(), sqlx::Error> {
        let count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM api_keys")
            .fetch_one(pool)
            .await?;

        if count > 0 {
            return Ok(());
        }

        let seed = std::env::var("API_KEY_SEED").unwrap_or_else(|_| "dev-test-api-key".to_owned());
        let key_hash = hash_api_key(&seed);
        let name = std::env::var("API_KEY_SEED_NAME").unwrap_or_else(|_| "default-dev-key".to_owned());

        tracing::info!(name = %name, "seeded default API key (set API_KEY_SEED to override)");

        sqlx::query(
            r#"
            INSERT INTO api_keys (id, key_hash, name, rate_limit_per_minute, is_active)
            VALUES ($1, $2, $3, 60, TRUE)
            ON CONFLICT (key_hash) DO NOTHING
            "#,
        )
        .bind(Uuid::new_v4())
        .bind(key_hash)
        .bind(name)
        .execute(pool)
        .await?;

        Ok(())
    }
}
