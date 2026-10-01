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

/// Sniffs the format by inspecting the first non-whitespace character in the decompressed stream
pub fn sniff_format_from_bytes(bytes: &[u8]) -> FeedFormat {
    let mut i = 0;
    // Skip UTF-8 BOM if present (EF BB BF)
    if bytes.starts_with(&[0xEF, 0xBB, 0xBF]) {
        i = 3;
    }
    while i < bytes.len() {
        let b = bytes[i];
        if !b.is_ascii_whitespace() {
            if b == b'<' {
                return FeedFormat::Xml;
            } else if b == b'{' || b == b'[' {
                return FeedFormat::Json;
            } else {
                break;
            }
        }
        i += 1;
    }
    FeedFormat::Auto
}

pub fn detect_feed_format(filename: &str) -> FeedFormat {
    let lower = filename.to_lowercase();
    if lower.contains(".xml") {
        FeedFormat::Xml
    } else if lower.contains(".json") || lower.contains(".ndjson") || lower.contains(".jsonl") {
        FeedFormat::Json
    } else {
        FeedFormat::Auto
    }
}
