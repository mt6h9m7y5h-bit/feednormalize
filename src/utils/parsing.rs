/// Returns `None` for empty or whitespace-only strings.
pub fn trim_or_none(value: &str) -> Option<String> {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        None
    } else {
        Some(trimmed.to_owned())
    }
}

/// Parses a decimal price string, accepting both `12.99` and `12,99`.
pub fn parse_price(value: &str) -> Option<f64> {
    let normalized = value.trim().replace(',', ".");
    normalized.parse().ok()
}
