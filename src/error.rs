use axum::{
    Json,
    http::StatusCode,
    response::{IntoResponse, Response},
};
use serde::Serialize;
use utoipa::ToSchema;

use crate::services::StorageError;

#[derive(Debug, Serialize, ToSchema)]
pub struct ErrorBody {
    pub error: String,
}

#[derive(Debug)]
pub enum ApiError {
    BadRequest(String),
    NotFound(String),
    Database(sqlx::Error),
    Io(std::io::Error),
    Storage(StorageError),
}

impl From<sqlx::Error> for ApiError {
    fn from(error: sqlx::Error) -> Self {
        Self::Database(error)
    }
}

impl From<std::io::Error> for ApiError {
    fn from(error: std::io::Error) -> Self {
        Self::Io(error)
    }
}

impl From<StorageError> for ApiError {
    fn from(error: StorageError) -> Self {
        Self::Storage(error)
    }
}

impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        let (status, message) = match self {
            Self::BadRequest(message) => (StatusCode::BAD_REQUEST, message),
            Self::NotFound(message) => (StatusCode::NOT_FOUND, message),
            Self::Database(error) => {
                tracing::error!(%error, "database error");
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    "internal database error".to_owned(),
                )
            }
            Self::Io(error) => {
                tracing::error!(%error, "io error");
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    "internal storage error".to_owned(),
                )
            }
            Self::Storage(error) => {
                tracing::error!(%error, "storage error");
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    "internal storage error".to_owned(),
                )
            }
        };

        (status, Json(ErrorBody { error: message })).into_response()
    }
}
