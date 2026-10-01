pub mod progress;
pub mod decompressor;
pub mod xml_stream;
pub mod json_stream;

use std::collections::HashMap;

#[derive(Debug, Clone)]
pub struct FeedRecord {
    pub fields: HashMap<String, String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FeedFormat {
    Xml,
    Json,
    Auto,
}

pub fn detect_feed_format(filename: &str) -> FeedFormat {
    let lower = filename.to_lowercase();
    if lower.contains(".xml") {
        FeedFormat::Xml
    } else if lower.contains(".json") || lower.contains(".ndjson") || lower.contains(".jsonl") {
        FeedFormat::Json
    } else {
        // Fallback: auto
        FeedFormat::Auto
    }
}
