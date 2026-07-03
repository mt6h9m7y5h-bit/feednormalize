use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "lowercase")]
pub enum ValidationLevel {
    Success,
    Warning,
    Error,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct ValidationIssue {
    pub row: usize,
    pub field: String,
    pub level: ValidationLevel,
    pub message: String,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, ToSchema)]
pub struct ValidationSummary {
    pub errors: usize,
    pub warnings: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct ValidationResult {
    pub summary: ValidationSummary,
    pub issues: Vec<ValidationIssue>,
}

impl ValidationResult {
    pub fn empty() -> Self {
        Self {
            summary: ValidationSummary::default(),
            issues: Vec::new(),
        }
    }

    pub fn overall_level(&self) -> ValidationLevel {
        if self.summary.errors > 0 {
            ValidationLevel::Error
        } else if self.summary.warnings > 0 {
            ValidationLevel::Warning
        } else {
            ValidationLevel::Success
        }
    }

    pub fn from_issues(issues: Vec<ValidationIssue>) -> Self {
        let mut summary = ValidationSummary::default();

        for issue in &issues {
            match issue.level {
                ValidationLevel::Error => summary.errors += 1,
                ValidationLevel::Warning => summary.warnings += 1,
                ValidationLevel::Success => {}
            }
        }

        Self { summary, issues }
    }
}
