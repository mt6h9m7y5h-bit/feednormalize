use crate::models::UniversalProduct;
use crate::validation::result::{ValidationIssue, ValidationLevel, ValidationResult};

/// Validates normalized products against schema rules.
#[derive(Debug, Default)]
pub struct ValidationEngine;

impl ValidationEngine {
    pub fn new() -> Self {
        Self
    }

    pub fn validate_products(&self, products: &[UniversalProduct]) -> ValidationResult {
        let mut issues = Vec::new();

        for (index, product) in products.iter().enumerate() {
            let row = index + 1;
            self.validate_product(row, product, &mut issues);
        }

        ValidationResult::from_issues(issues)
    }

    fn validate_product(&self, row: usize, product: &UniversalProduct, issues: &mut Vec<ValidationIssue>) {
        match product.price {
            None => issues.push(ValidationIssue {
                row,
                field: "price".to_owned(),
                level: ValidationLevel::Error,
                message: "Price must be positive".to_owned(),
            }),
            Some(price) if price <= 0.0 => issues.push(ValidationIssue {
                row,
                field: "price".to_owned(),
                level: ValidationLevel::Error,
                message: "Price must be positive".to_owned(),
            }),
            _ => {}
        }

        let sku_empty = product
            .sku
            .as_ref()
            .map(|value| value.trim().is_empty())
            .unwrap_or(true);

        if sku_empty {
            issues.push(ValidationIssue {
                row,
                field: "sku".to_owned(),
                level: ValidationLevel::Error,
                message: "SKU must not be empty".to_owned(),
            });
        }

        let title_too_short = product
            .title
            .as_ref()
            .map(|value| value.trim().chars().count() < 10)
            .unwrap_or(true);

        if title_too_short {
            issues.push(ValidationIssue {
                row,
                field: "title".to_owned(),
                level: ValidationLevel::Error,
                message: "Title must be at least 10 characters".to_owned(),
            });
        }
    }
}
