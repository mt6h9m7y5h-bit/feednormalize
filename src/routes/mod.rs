pub mod feeds;
pub mod jobs;

use axum::{Router, middleware, routing::get};
use tower::ServiceBuilder;
use utoipa::OpenApi;
use utoipa_swagger_ui::SwaggerUi;

use crate::middleware::{auth, rate_limit};
use crate::openapi::ApiDoc;
use crate::routes::jobs::health;
use crate::state::AppState;

pub fn create_router(state: AppState) -> Router {
    let public = Router::new()
        .route("/health", get(health))
        .merge(
            SwaggerUi::new("/swagger-ui")
                .url("/api-docs/openapi.json", ApiDoc::openapi()),
        );

    let protected = Router::new()
        .merge(jobs::protected_router(state.clone()))
        .merge(feeds::router(state.clone()))
        .layer(
            ServiceBuilder::new()
                .layer(middleware::from_fn_with_state(
                    state.clone(),
                    auth::middleware,
                ))
                .layer(middleware::from_fn_with_state(
                    state,
                    rate_limit::middleware,
                )),
        );

    public.merge(protected)
}
