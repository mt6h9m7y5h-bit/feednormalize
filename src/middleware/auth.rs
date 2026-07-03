use axum::{
    extract::{Request, State},
    middleware::Next,
    response::Response,
};

use crate::middleware::unauthorized_response;
use crate::services::ApiKeyService;
use crate::state::AppState;

const API_KEY_HEADER: &str = "x-api-key";
const RAPIDAPI_KEY_HEADER: &str = "x-rapidapi-key";

pub async fn middleware(
    State(state): State<AppState>,
    mut request: Request,
    next: Next,
) -> Response {
    let raw_key = match extract_api_key(request.headers()) {
        Some(key) => key,
        None => return unauthorized_response("missing API key"),
    };

    let api_key = match ApiKeyService::validate(&state.db, &raw_key).await {
        Ok(Some(key)) => key,
        Ok(None) => return unauthorized_response("invalid API key"),
        Err(error) => {
            tracing::error!(%error, "failed to validate API key");
            return unauthorized_response("invalid API key");
        }
    };

    request.extensions_mut().insert(api_key);
    next.run(request).await
}

fn extract_api_key(headers: &axum::http::HeaderMap) -> Option<String> {
    headers
        .get(API_KEY_HEADER)
        .or_else(|| headers.get(RAPIDAPI_KEY_HEADER))
        .and_then(|value| value.to_str().ok())
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(str::to_owned)
}
