use std::collections::HashMap;
use std::io::BufRead;
use anyhow::Result;
use serde_json::Value;

pub struct JsonRecord {
    pub fields: HashMap<String, String>,
}

pub struct JsonStreamParser<R: BufRead> {
    reader: R,
    is_ndjson: Option<bool>,
    buffer_line: String,
}

impl<R: BufRead + 'static> JsonStreamParser<R> {
    pub fn new(reader: R) -> Self {
        Self {
            reader,
            is_ndjson: None,
            buffer_line: String::with_capacity(4096),
        }
    }

    /// Read next record from JSON or NDJSON stream
    pub fn next_record(&mut self) -> Result<Option<JsonRecord>> {
        // First check if NDJSON or Array format
        if self.is_ndjson.is_none() {
            let first_chars: String = {
                let buf = self.reader.fill_buf()?;
                let sample = String::from_utf8_lossy(buf);
                sample.trim_start().chars().take(20).collect()
            };

            if first_chars.starts_with('{') {
                // Could be NDJSON (lines of { ... }) or envelope { "jobs": [ ... ] }
                // Let's test reading one line
                self.is_ndjson = Some(true);
            } else if first_chars.starts_with('[') {
                self.is_ndjson = Some(false);
            } else {
                self.is_ndjson = Some(true);
            }
        }

        if self.is_ndjson == Some(true) {
            loop {
                self.buffer_line.clear();
                let bytes_read = self.reader.read_line(&mut self.buffer_line)?;
                if bytes_read == 0 {
                    return Ok(None);
                }

                let line = self.buffer_line.trim();
                if line.is_empty() || line == "[" || line == "]" || line == "]," {
                    continue;
                }

                // If line ends with a trailing comma from an array, strip it
                let clean_line = if line.ends_with(',') {
                    &line[..line.len() - 1]
                } else {
                    line
                };

                if let Ok(val) = serde_json::from_str::<Value>(clean_line) {
                    if let Value::Object(map) = val {
                        let mut fields = HashMap::new();
                        flatten_json_object("", &map, &mut fields);
                        return Ok(Some(JsonRecord { fields }));
                    }
                }
            }
        } else {
            // For standard JSON array or fallback
            loop {
                self.buffer_line.clear();
                let bytes_read = self.reader.read_line(&mut self.buffer_line)?;
                if bytes_read == 0 {
                    return Ok(None);
                }

                let line = self.buffer_line.trim();
                if line.is_empty() || line == "[" || line == "]" {
                    continue;
                }

                let clean_line = if line.ends_with(',') {
                    &line[..line.len() - 1]
                } else {
                    line
                };

                if let Ok(val) = serde_json::from_str::<Value>(clean_line) {
                    if let Value::Object(map) = val {
                        let mut fields = HashMap::new();
                        flatten_json_object("", &map, &mut fields);
                        return Ok(Some(JsonRecord { fields }));
                    }
                }
            }
        }
    }
}

pub fn flatten_json_object(prefix: &str, map: &serde_json::Map<String, Value>, out: &mut HashMap<String, String>) {
    for (k, v) in map {
        let key = if prefix.is_empty() {
            k.clone()
        } else {
            format!("{}.{}", prefix, k)
        };

        match v {
            Value::Null => {
                // leave as null/omitted
            }
            Value::Bool(b) => {
                out.insert(key, b.to_string());
            }
            Value::Number(n) => {
                out.insert(key, n.to_string());
            }
            Value::String(s) => {
                out.insert(key, s.clone());
            }
            Value::Array(arr) => {
                if arr.iter().all(|item| item.is_string() || item.is_number()) {
                    let joined = arr
                        .iter()
                        .map(|item| item.as_str().map(|s| s.to_string()).unwrap_or_else(|| item.to_string()))
                        .collect::<Vec<_>>()
                        .join(", ");
                    out.insert(key, joined);
                } else {
                    out.insert(key, serde_json::to_string(arr).unwrap_or_default());
                }
            }
            Value::Object(inner_map) => {
                flatten_json_object(&key, inner_map, out);
            }
        }
    }
}
