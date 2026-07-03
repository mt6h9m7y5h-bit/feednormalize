pub mod auth;
pub mod rate_limit;

use axum::{
    Json,
    http::StatusCode,
    response::{IntoResponse, Response},
};

use crate::error::ErrorBody;

pub(crate) fn unauthorized_response(message: &str) -> Response {
    (
        StatusCode::UNAUTHORIZED,
        Json(ErrorBody {
            error: message.to_owned(),
        }),
    )
        .into_response()
}

pub(crate) fn too_many_requests_response() -> Response {
    (
        StatusCode::TOO_MANY_REQUESTS,
        Json(ErrorBody {
            error: "rate limit exceeded".to_owned(),
        }),
    )
        .into_response()
}
