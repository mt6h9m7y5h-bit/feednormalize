use utoipa::OpenApi;
use utoipa::openapi::security::{ApiKey, ApiKeyValue, SecurityScheme};
use utoipa::openapi::server::{ServerBuilder, ServerVariableBuilder};
use utoipa::Modify;

use crate::error::ErrorBody;
use crate::models::{Job, JobResponse, JobStatus, UniversalProduct, UploadResponse};
use crate::routes::feeds::UploadForm;
use crate::routes::jobs::{CreateJobRequest, CreateJobResponse, HealthResponse};
use crate::routes::webhooks::WebhookTestResponse;

struct SecurityAddon;

impl Modify for SecurityAddon {
    fn modify(&self, openapi: &mut utoipa::openapi::OpenApi) {
        if let Some(components) = openapi.components.as_mut() {
            components.add_security_scheme(
                "ApiKeyAuth",
                SecurityScheme::ApiKey(ApiKey::Header(ApiKeyValue::new("x-api-key"))),
            );
        }
    }
}

struct ServersAddon;

impl Modify for ServersAddon {
    fn modify(&self, openapi: &mut utoipa::openapi::OpenApi) {
        openapi.servers = Some(vec![
            ServerBuilder::new()
                .url("https://{domain}")
                .description(Some("Production deployment (Railway / RapidAPI)"))
                .parameter(
                    "domain",
                    ServerVariableBuilder::new()
                        .default_value("feednormalize-production.up.railway.app")
                        .description(Some(
                            "Your Railway public hostname (set as RapidAPI base URL when publishing)",
                        )),
                )
                .build(),
        ]);
    }
}

#[derive(OpenApi)]
#[openapi(
    info(
        title = "FeedNormalize API",
        description = "Universal Product Feed Normalization API. Upload supplier product feeds (CSV, JSON, TSV, NDJSON), track async normalization jobs, and download a standardized JSON catalog. Protected endpoints require the `x-api-key` header (or `x-rapidapi-key` when proxied through RapidAPI). Interactive docs: `/swagger-ui`. OpenAPI JSON: `/api-docs/openapi.json`.",
        version = "0.1.0",
    ),
    modifiers(&SecurityAddon, &ServersAddon),
    paths(
        crate::routes::jobs::create_job,
        crate::routes::jobs::get_job,
        crate::routes::jobs::download_job,
        crate::routes::feeds::upload_feed,
        crate::routes::jobs::health,
        crate::routes::webhooks::webhook_test,
    ),
    components(
        schemas(
            UniversalProduct,
            Job,
            JobStatus,
            JobResponse,
            CreateJobRequest,
            CreateJobResponse,
            UploadResponse,
            UploadForm,
            HealthResponse,
            WebhookTestResponse,
            ErrorBody,
        )
    ),
    tags(
        (name = "Jobs", description = "Create and monitor normalization jobs"),
        (name = "Feeds", description = "Upload product feed files"),
        (name = "Health", description = "Service health checks"),
        (name = "Webhooks", description = "Callback and notification endpoints"),
    ),
)]
pub struct ApiDoc;
