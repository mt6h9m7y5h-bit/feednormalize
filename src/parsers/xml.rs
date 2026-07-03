use quick_xml::Reader;
use quick_xml::events::Event;

use crate::models::UniversalProduct;
use crate::parsers::{FeedParser, ParserError};

/// XML feed parser stub — full product mapping will be added in a later phase.
#[derive(Debug, Default)]
pub struct XmlFeedParser;

impl FeedParser for XmlFeedParser {
    fn parse(&self, input: &[u8]) -> Result<Vec<UniversalProduct>, ParserError> {
        let mut reader = Reader::from_reader(input);
        reader.config_mut().trim_text(true);

        let mut buf = Vec::new();

        loop {
            match reader.read_event_into(&mut buf) {
                Ok(Event::Eof) => break,
                Ok(_) => {}
                Err(error) => {
                    return Err(ParserError::Xml(error.to_string()));
                }
            }
            buf.clear();
        }

        Ok(Vec::new())
    }
}
