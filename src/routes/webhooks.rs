use axum::{Json, Router, routing::post};
use serde::Serialize;
use tracing::info;
use utoipa::ToSchema;

use crate::error::ErrorBody;
use crate::state::AppState;

pub fn router(state: AppState) -> Router {
    Router::new()
        .route("/webhooks/test", post(webhook_test))
        .with_state(state)
}

#[derive(Debug, Serialize, ToSchema)]
pub struct WebhookTestResponse {
    pub received: bool,
}

/// Accept a test webhook callback payload.
#[utoipa::path(
    post,
    path = "/webhooks/test",
    summary = "Receive a test webhook callback",
    description = "Demonstrates readiness for async notifications and callback URLs (RapidAPI webhooks). Accepts any JSON object, logs the payload, and returns an acknowledgment.",
    request_body(
        content = inline(serde_json::Value),
        description = "JSON callback payload (any shape)",
    ),
    security(("ApiKeyAuth" = [])),
    responses(
        (status = 200, description = "Payload received", body = WebhookTestResponse),
        (status = 400, description = "Invalid JSON body", body = ErrorBody),
        (status = 401, description = "Missing or invalid API key", body = ErrorBody),
        (status = 429, description = "Rate limit exceeded", body = ErrorBody),
        (status = 500, description = "Internal server error", body = ErrorBody),
    ),
    tag = "Webhooks"
)]
pub async fn webhook_test(Json(body): Json<serde_json::Value>) -> Json<WebhookTestResponse> {
    let job_id = body.get("job_id").and_then(|value| value.as_str());
    info!(job_id = ?job_id, payload = %body, "webhook test received");
    Json(WebhookTestResponse { received: true })
}
