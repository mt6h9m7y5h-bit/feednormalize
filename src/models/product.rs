use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use validator::Validate;

/// Canonical product representation used across all feed formats.
#[derive(Debug, Clone, Serialize, Deserialize, Validate, Default, ToSchema)]
pub struct UniversalProduct {
    pub sku: Option<String>,
    pub title: Option<String>,
    pub price: Option<f64>,
    pub currency: Option<String>,
    pub ean: Option<String>,
}
