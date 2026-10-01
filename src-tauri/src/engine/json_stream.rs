use std::collections::HashMap;
use std::io::BufRead;
use anyhow::Result;
use serde_json::Value;

pub struct JsonRecord {
    pub fields: HashMap<String, String>,
}

pub struct JsonStreamParser<R: BufRead> {
    reader: R,
    skip_descriptions: bool,
    record_tag: Option<String>,
    buf: Vec<u8>,
    in_string: bool,
    escaped: bool,
    depth: usize,
    in_array: bool,
    started: bool,
    finished: bool,
}

impl<R: BufRead + 'static> JsonStreamParser<R> {
    pub fn new(reader: R, skip_descriptions: bool, record_tag: Option<String>) -> Self {
        Self {
            reader,
            skip_descriptions,
            record_tag,
            buf: Vec::with_capacity(16 * 1024),
            in_string: false,
            escaped: false,
            depth: 0,
            in_array: false,
            started: false,
            finished: false,
        }
    }

    fn initialize_stream(&mut self) -> Result<()> {
        let available = self.reader.fill_buf()?;
        if available.is_empty() {
            self.started = true;
            return Ok(());
        }

        let first_non_ws = available.iter().position(|b| !b.is_ascii_whitespace());
        let first_char = first_non_ws.map(|pos| available[pos]);

        if first_char == Some(b'[') {
            let pos = first_non_ws.unwrap();
            self.reader.consume(pos + 1);
            self.in_array = true;
            self.started = true;
            self.depth = 0;
            return Ok(());
        }

        if first_char == Some(b'{') {
            let target_tag_lower = self.record_tag.as_ref().map(|s| s.to_lowercase());
            if let Some(pos_after_bracket) = find_nested_array_start(available, target_tag_lower.as_deref()) {
                self.reader.consume(pos_after_bracket);
                self.in_array = true;
                self.started = true;
                self.depth = 0;
                return Ok(());
            }
        }

        self.started = true;
        Ok(())
    }

    /// Read next record from JSON, NDJSON, or Array stream
    pub fn next_record(&mut self) -> Result<Option<JsonRecord>> {
        if self.finished {
            return Ok(None);
        }

        if !self.started {
            self.initialize_stream()?;
        }

        loop {
            let (completed, consumed, hit_eof_or_end) = {
                let available = self.reader.fill_buf()?;
                if available.is_empty() {
                    return Ok(None);
                }

                let mut bytes_used = 0;
                let mut completed_object = false;
                let mut reached_end = false;

                for &b in available {
                    bytes_used += 1;

                    if self.in_string {
                        self.buf.push(b);
                        if self.escaped {
                            self.escaped = false;
                        } else if b == b'\\' {
                            self.escaped = true;
                        } else if b == b'"' {
                            self.in_string = false;
                        }
                    } else {
                        match b {
                            b'"' => {
                                self.in_string = true;
                                if self.depth > 0 {
                                    self.buf.push(b);
                                }
                            }
                            b'{' => {
                                if self.depth == 0 {
                                    self.buf.clear();
                                }
                                self.depth += 1;
                                self.buf.push(b);
                            }
                            b'}' => {
                                if self.depth > 0 {
                                    self.depth -= 1;
                                    self.buf.push(b);
                                    if self.depth == 0 {
                                        completed_object = true;
                                        break;
                                    }
                                }
                            }
                            b']' => {
                                if self.depth == 0 && self.in_array {
                                    reached_end = true;
                                    break;
                                }
                                if self.depth > 0 {
                                    self.buf.push(b);
                                }
                            }
                            _ => {
                                if self.depth > 0 {
                                    self.buf.push(b);
                                }
                            }
                        }
                    }
                }

                (completed_object, bytes_used, reached_end)
            };

            self.reader.consume(consumed);

            if hit_eof_or_end {
                self.finished = true;
                return Ok(None);
            }

            if completed {
                if let Ok(val) = serde_json::from_slice::<Value>(&self.buf) {
                    if let Value::Object(map) = val {
                        let mut fields = HashMap::new();

                        // If user specified record_tag, e.g. "job"
                        if let Some(ref tag) = self.record_tag {
                            if let Some(Value::Object(inner_map)) = map.get(tag) {
                                flatten_json_object("", inner_map, &mut fields, self.skip_descriptions);
                                return Ok(Some(JsonRecord { fields }));
                            }
                        }

                        flatten_json_object("", &map, &mut fields, self.skip_descriptions);
                        return Ok(Some(JsonRecord { fields }));
                    }
                }
            } else {
                let available = self.reader.fill_buf()?;
                if available.is_empty() {
                    self.finished = true;
                    return Ok(None);
                }
            }
        }
    }
}

pub fn flatten_json_object(
    prefix: &str,
    map: &serde_json::Map<String, Value>,
    out: &mut HashMap<String, String>,
    skip_descriptions: bool,
) {
    for (k, v) in map {
        let key = if prefix.is_empty() {
            k.clone()
        } else {
            format!("{}.{}", prefix, k)
        };

        let k_lower = k.to_lowercase();
        if skip_descriptions && (k_lower.contains("description") || k_lower == "body" || k_lower == "content") {
            continue;
        }

        match v {
            Value::Null => {}
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
                if arr.is_empty() {
                    continue;
                }
                if arr.iter().all(|item| item.is_string() || item.is_number()) {
                    let joined = arr
                        .iter()
                        .map(|item| item.as_str().map(|s| s.to_string()).unwrap_or_else(|| item.to_string()))
                        .collect::<Vec<_>>()
                        .join(", ");
                    out.insert(key, joined);
                } else if arr.iter().all(|item| item.is_object()) {
                    for (i, item) in arr.iter().enumerate() {
                        if let Value::Object(item_map) = item {
                            flatten_json_object(&format!("{}.{}", key, i), item_map, out, skip_descriptions);
                        }
                    }
                } else {
                    out.insert(key, serde_json::to_string(arr).unwrap_or_default());
                }
            }
            Value::Object(inner_map) => {
                flatten_json_object(&key, inner_map, out, skip_descriptions);
            }
        }
    }
}

pub fn find_nested_array_start(bytes: &[u8], user_target: Option<&str>) -> Option<usize> {
    let text = String::from_utf8_lossy(bytes);

    if let Some(target) = user_target {
        let pattern = format!("\"{}\"", target);
        if let Some(pos) = text.to_lowercase().find(&pattern.to_lowercase()) {
            let after_key = &text[pos + pattern.len()..];
            let mut saw_colon = false;
            for (idx, ch) in after_key.char_indices() {
                if ch.is_whitespace() {
                    continue;
                }
                if ch == ':' {
                    saw_colon = true;
                    continue;
                }
                if saw_colon && ch == '[' {
                    return Some(pos + pattern.len() + idx + 1);
                }
                if saw_colon {
                    break;
                }
            }
        }
    }

    let candidates = [
        "data", "jobs", "postings", "vacancies", "results", "items", "records",
        "positions", "listings", "offers", "jobfeed", "elements"
    ];

    let text_lower = text.to_lowercase();
    for candidate in &candidates {
        let pattern = format!("\"{}\"", candidate);
        if let Some(pos) = text_lower.find(&pattern) {
            let after_key = &text[pos + pattern.len()..];
            let mut saw_colon = false;
            for (idx, ch) in after_key.char_indices() {
                if ch.is_whitespace() {
                    continue;
                }
                if ch == ':' {
                    saw_colon = true;
                    continue;
                }
                if saw_colon && ch == '[' {
                    return Some(pos + pattern.len() + idx + 1);
                }
                if saw_colon {
                    break;
                }
            }
        }
    }

    // Generic fallback: match any key followed by colon and bracket within the first 64KB
    let mut in_str = false;
    let mut esc = false;
    let mut state = 0; // 0: init, 1: saw string, 2: saw colon

    for (i, &b) in bytes.iter().enumerate() {
        if i > 65536 {
            break;
        }
        if in_str {
            if esc {
                esc = false;
            } else if b == b'\\' {
                esc = true;
            } else if b == b'"' {
                in_str = false;
                state = 1;
            }
        } else {
            match b {
                b'"' => in_str = true,
                b':' if state == 1 => state = 2,
                b'[' if state == 2 => return Some(i + 1),
                b'{' if state == 2 => state = 0,
                b if !b.is_ascii_whitespace() => state = 0,
                _ => {}
            }
        }
    }

    None
}
