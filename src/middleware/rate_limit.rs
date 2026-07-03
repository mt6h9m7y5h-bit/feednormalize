use axum::{
    extract::{Request, State},
    middleware::Next,
    response::Response,
};

use crate::middleware::too_many_requests_response;
use crate::models::AuthenticatedApiKey;
use crate::state::AppState;

pub async fn middleware(
    State(state): State<AppState>,
    request: Request,
    next: Next,
) -> Response {
    let api_key = match request.extensions().get::<AuthenticatedApiKey>() {
        Some(key) => key.clone(),
        None => {
            tracing::error!("rate limit middleware reached without authenticated API key");
            return too_many_requests_response();
        }
    };

    match state
        .rate_limiter
        .check(api_key.id, api_key.rate_limit_per_minute)
        .await
    {
        Ok(allowed) if allowed => next.run(request).await,
        Ok(_) => too_many_requests_response(),
        Err(error) => {
            tracing::error!(%error, api_key_id = %api_key.id, "rate limit check failed");
            too_many_requests_response()
        }
    }
}
