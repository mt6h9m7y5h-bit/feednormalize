mod db;
mod error;
mod middleware;
mod models;
mod openapi;
#[allow(dead_code)]
mod parsers;
mod rate_limit;
mod routes;
mod services;
mod state;
mod utils;
mod validation;
mod worker;

use std::net::SocketAddr;

use tracing::{error, info, warn};
use tracing_subscriber::{EnvFilter, fmt};

fn is_production() -> bool {
    std::env::var("RAILWAY_ENVIRONMENT")
        .map(|value| value == "production")
        .unwrap_or_else(|_| std::env::var("RAILWAY_PROJECT_ID").is_ok())
}

fn require_database_url() -> String {
    match std::env::var("DATABASE_URL") {
        Ok(url) if !url.is_empty() => url,
        _ => {
            error!(
                "DATABASE_URL is required. Set it to your PostgreSQL connection string. \
                 On Railway: add the PostgreSQL plugin, then reference its DATABASE_URL on this service."
            );
            std::process::exit(1);
        }
    }
}

fn warn_if_insecure_api_key_seed() {
    if !is_production() {
        return;
    }

    let seed = std::env::var("API_KEY_SEED").unwrap_or_default();
    if seed.is_empty() || seed == "dev-test-api-key" {
        warn!(
            "API_KEY_SEED is unset or still the dev default in production. \
             Set a strong random value in Railway Variables before first boot; \
             ApiKeyService::ensure_seed inserts it only when api_keys is empty."
        );
    }
}

#[tokio::main]
async fn main() {
    dotenvy::dotenv().ok();

    fmt()
        .with_env_filter(
            EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info")),
        )
        .init();

    let rust_log = std::env::var("RUST_LOG").unwrap_or_else(|_| "info".to_owned());
    info!(%rust_log, "tracing initialized (RUST_LOG defaults to info when unset)");

    warn_if_insecure_api_key_seed();

    let database_url = require_database_url();

    let pool = match db::init_pool(&database_url).await {
        Ok(pool) => pool,
        Err(error) => {
            error!(%error, "failed to connect to PostgreSQL or run migrations");
            if !is_production() {
                error!(
                    "local dev: start Postgres with `docker compose up -d`, \
                     or install PostgreSQL and run `createdb feednormalize`"
                );
            }
            std::process::exit(1);
        }
    };

    let storage = match services::StorageService::from_env().await {
        Ok(storage) => storage,
        Err(error) => {
            error!(%error, "failed to initialize object storage");
            std::process::exit(1);
        }
    };

    worker::spawn(pool.clone(), storage.clone());

    let rate_limiter = match rate_limit::RateLimiter::new().await {
        Ok(limiter) => std::sync::Arc::new(limiter),
        Err(error) => {
            error!(%error, "failed to initialize rate limiter");
            std::process::exit(1);
        }
    };

    let state = state::AppState::new(pool, rate_limiter, storage);

    let host = std::env::var("HOST").unwrap_or_else(|_| "0.0.0.0".to_owned());
    let port = std::env::var("PORT")
        .ok()
        .and_then(|value| value.parse().ok())
        .unwrap_or(3000);

    let addr: SocketAddr = format!("{host}:{port}")
        .parse()
        .expect("invalid HOST or PORT");

    let app = routes::create_router(state);

    info!(%addr, "starting FeedNormalize API");

    let listener = tokio::net::TcpListener::bind(addr)
        .await
        .expect("failed to bind TCP listener");

    info!(%addr, "FeedNormalize API ready");

    axum::serve(listener, app)
        .await
        .expect("server error");
}
