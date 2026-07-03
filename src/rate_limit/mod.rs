use std::collections::HashMap;
use std::time::{Duration, Instant};

use tokio::sync::Mutex;
use uuid::Uuid;

#[derive(Debug, thiserror::Error)]
pub enum RateLimitError {
    #[error("redis error: {0}")]
    Redis(#[from] redis::RedisError),
}

pub struct RateLimiter {
    backend: RateLimitBackend,
}

enum RateLimitBackend {
    Redis(redis::aio::ConnectionManager),
    Memory(MemoryRateLimiter),
}

struct MemoryRateLimiter {
    windows: Mutex<HashMap<Uuid, Vec<Instant>>>,
}

impl RateLimiter {
    pub async fn new() -> Result<Self, RateLimitError> {
        let backend = if let Ok(redis_url) = std::env::var("REDIS_URL") {
            if redis_url.is_empty() {
                Self::memory_backend()
            } else {
                let client = redis::Client::open(redis_url)?;
                let connection = client.get_connection_manager().await?;
                tracing::info!("rate limiter using Redis");
                RateLimitBackend::Redis(connection)
            }
        } else {
            Self::memory_backend()
        };

        Ok(Self { backend })
    }

    fn memory_backend() -> RateLimitBackend {
        tracing::info!("rate limiter using in-memory sliding window");
        RateLimitBackend::Memory(MemoryRateLimiter {
            windows: Mutex::new(HashMap::new()),
        })
    }

    pub async fn check(&self, api_key_id: Uuid, limit_per_minute: i32) -> Result<bool, RateLimitError> {
        if limit_per_minute <= 0 {
            return Ok(true);
        }

        match &self.backend {
            RateLimitBackend::Redis(connection) => {
                check_redis(connection, api_key_id, limit_per_minute).await
            }
            RateLimitBackend::Memory(limiter) => Ok(check_memory(limiter, api_key_id, limit_per_minute).await),
        }
    }
}

async fn check_redis(
    connection: &redis::aio::ConnectionManager,
    api_key_id: Uuid,
    limit_per_minute: i32,
) -> Result<bool, RateLimitError> {
    let key = format!("rate_limit:{api_key_id}");
    let now_ms = chrono::Utc::now().timestamp_millis();
    let window_start_ms = now_ms - 60_000;
    let member = format!("{now_ms}");

    let mut connection = connection.clone();

    let _: () = redis::cmd("ZREMRANGEBYSCORE")
        .arg(&key)
        .arg(0)
        .arg(window_start_ms)
        .query_async(&mut connection)
        .await?;

    let count: i32 = redis::cmd("ZCARD")
        .arg(&key)
        .query_async(&mut connection)
        .await?;

    if count >= limit_per_minute {
        return Ok(false);
    }

    let _: () = redis::cmd("ZADD")
        .arg(&key)
        .arg(now_ms)
        .arg(&member)
        .query_async(&mut connection)
        .await?;

    let _: () = redis::cmd("EXPIRE")
        .arg(&key)
        .arg(120)
        .query_async(&mut connection)
        .await?;

    Ok(true)
}

async fn check_memory(
    limiter: &MemoryRateLimiter,
    api_key_id: Uuid,
    limit_per_minute: i32,
) -> bool {
    let now = Instant::now();
    let window = Duration::from_secs(60);
    let limit = usize::try_from(limit_per_minute).unwrap_or(usize::MAX);

    let mut windows = limiter.windows.lock().await;
    let entries = windows.entry(api_key_id).or_default();
    entries.retain(|timestamp| now.duration_since(*timestamp) < window);

    if entries.len() >= limit {
        return false;
    }

    entries.push(now);
    true
}
