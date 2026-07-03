use axum::{Json, Router, routing::post};
use serde::Serialize;
use tracing::info;
use utoipa::ToSchema;

use crate::error::ErrorBody;

pub fn router() -> Router {
    Router::new().route("/webhooks/test", post(webhook_test))
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
    description = "Demonstrates readiness for async notifications and callback URLs (RapidAPI webhooks). Accepts any JSON object, logs the payload, and returns an acknowledgment. No authentication required.",
    request_body(
        content = inline(serde_json::Value),
        description = "JSON callback payload (any shape)",
    ),
    responses(
        (status = 200, description = "Payload received", body = WebhookTestResponse),
        (status = 400, description = "Invalid JSON body", body = ErrorBody),
        (status = 500, description = "Internal server error", body = ErrorBody),
    ),
    tag = "Webhooks"
)]
pub async fn webhook_test(Json(body): Json<serde_json::Value>) -> Json<WebhookTestResponse> {
    info!(payload = %body, "webhook test received");
    Json(WebhookTestResponse { received: true })
}
