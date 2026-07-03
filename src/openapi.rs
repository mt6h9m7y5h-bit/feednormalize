use utoipa::OpenApi;
use utoipa::openapi::security::{ApiKey, ApiKeyValue, SecurityScheme};
use utoipa::Modify;

use crate::error::ErrorBody;
use crate::models::{Job, JobResponse, JobStatus, UniversalProduct, UploadResponse};
use crate::routes::feeds::UploadForm;
use crate::routes::jobs::{CreateJobRequest, CreateJobResponse, HealthResponse};

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

#[derive(OpenApi)]
#[openapi(
    info(
        title = "FeedNormalize API",
        description = "Universal Product Feed Normalization API",
        version = "0.1.0",
    ),
    modifiers(&SecurityAddon),
    paths(
        crate::routes::jobs::create_job,
        crate::routes::jobs::get_job,
        crate::routes::jobs::download_job,
        crate::routes::feeds::upload_feed,
        crate::routes::jobs::health,
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
            ErrorBody,
        )
    ),
    tags(
        (name = "Jobs", description = "Create and monitor normalization jobs"),
        (name = "Feeds", description = "Upload product feed files"),
        (name = "Health", description = "Service health checks"),
    ),
)]
pub struct ApiDoc;
