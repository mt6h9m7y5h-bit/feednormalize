mod csv;
mod xml;

use thiserror::Error;

use crate::models::UniversalProduct;

#[allow(unused_imports)]
pub use csv::CsvFeedParser;
#[allow(unused_imports)]
pub use xml::XmlFeedParser;

#[derive(Debug, Error)]
pub enum ParserError {
    #[error("parser not implemented: {0}")]
    NotImplemented(&'static str),
    #[error("xml parse error: {0}")]
    Xml(String),
    #[error("csv error: {0}")]
    Csv(#[from] csv_async::Error),
    #[error("io error: {0}")]
    Io(#[from] std::io::Error),
}

/// Abstraction over feed format parsers (CSV, XML, etc.).
pub trait FeedParser: Send + Sync {
    fn parse(&self, input: &[u8]) -> Result<Vec<UniversalProduct>, ParserError>;
}
