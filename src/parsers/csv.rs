use crate::models::UniversalProduct;
use crate::parsers::{FeedParser, ParserError};

/// CSV feed parser (delegates to `NormalizationEngine` in the worker path).
#[derive(Debug, Default)]
pub struct CsvFeedParser;

impl FeedParser for CsvFeedParser {
    fn parse(&self, _input: &[u8]) -> Result<Vec<UniversalProduct>, ParserError> {
        Err(ParserError::NotImplemented(
            "CSV parsing is handled by NormalizationEngine::process_feed",
        ))
    }
}
