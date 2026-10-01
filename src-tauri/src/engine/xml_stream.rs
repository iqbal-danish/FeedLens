use std::collections::{HashMap, HashSet};
use std::io::{BufRead, BufReader, Read};
use anyhow::Result;
use quick_xml::events::{BytesStart, Event};
use quick_xml::reader::Reader;

const ROOT_CONTAINERS: &[&str] = &[
    "root", "root-node", "source", "sources", "jobs", "jobfeed", "feed", "feeds",
    "channel", "rss", "document", "results", "response", "data", "catalog", "items",
    "postings", "vacancies", "openings", "positions", "listings", "records", "array"
];

fn get_keyword_score(tag: &str) -> f64 {
    match tag {
        "job" => 100.0,
        "posting" | "position" | "vacancy" | "opening" => 90.0,
        "opportunity" | "listing" => 85.0,
        "career" | "offer" => 80.0,
        "item" | "entry" => 75.0,
        "record" | "requisition" => 70.0,
        "work" => 60.0,
        _ => 0.0,
    }
}

#[inline]
pub fn strip_namespace(name: &str) -> &str {
    if let Some(pos) = name.rfind(':') {
        &name[pos + 1..]
    } else {
        name
    }
}

pub struct XmlRecord {
    pub fields: HashMap<String, String>,
}

/// Scans the initial 5 MB of an XML reader to accurately detect the repeating record tag
pub fn detect_xml_record_tag<R: Read>(reader_source: R, limit_bytes: usize) -> Result<String> {
    struct LimitedReader<R> {
        inner: R,
        remaining: usize,
    }
    impl<R: Read> Read for LimitedReader<R> {
        fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
            if self.remaining == 0 {
                return Ok(0);
            }
            let max_to_read = std::cmp::min(buf.len(), self.remaining);
            let bytes_read = self.inner.read(&mut buf[..max_to_read])?;
            self.remaining -= bytes_read;
            Ok(bytes_read)
        }
    }

    let limited = LimitedReader {
        inner: reader_source,
        remaining: limit_bytes,
    };

    let mut reader = Reader::from_reader(BufReader::with_capacity(128 * 1024, limited));
    reader.config_mut().trim_text(true);

    let mut buf = Vec::with_capacity(4096);
    let mut stack: Vec<String> = Vec::with_capacity(32);
    let mut tag_counts: HashMap<String, usize> = HashMap::new();
    let mut tag_depths: HashMap<String, usize> = HashMap::new();
    let mut tag_parents: HashMap<String, HashSet<String>> = HashMap::new();
    let mut tag_child_tags: HashMap<String, HashSet<String>> = HashMap::new();
    let mut exact_casing: HashMap<String, String> = HashMap::new();

    let root_containers_set: HashSet<&str> = ROOT_CONTAINERS.iter().copied().collect();

    loop {
        buf.clear();
        match reader.read_event_into(&mut buf) {
            Ok(Event::Start(e)) => {
                let raw_name = String::from_utf8_lossy(e.name().as_ref()).to_string();
                let local_name = strip_namespace(&raw_name).to_string();
                let lower_name = local_name.to_lowercase();

                exact_casing.entry(lower_name.clone()).or_insert_with(|| local_name.clone());

                let depth = stack.len();
                if let Some(parent) = stack.last() {
                    tag_parents.entry(lower_name.clone()).or_default().insert(parent.clone());
                    tag_child_tags.entry(parent.clone()).or_default().insert(lower_name.clone());
                }

                tag_depths
                    .entry(lower_name.clone())
                    .and_modify(|d| *d = std::cmp::min(*d, depth))
                    .or_insert(depth);

                stack.push(lower_name);
            }
            Ok(Event::End(e)) => {
                let raw_name = String::from_utf8_lossy(e.name().as_ref()).to_string();
                let local_name = strip_namespace(&raw_name).to_string();
                let lower_name = local_name.to_lowercase();

                if let Some(pos) = stack.iter().rposition(|x| x == &lower_name) {
                    stack.truncate(pos);
                }
                *tag_counts.entry(lower_name).or_insert(0) += 1;
            }
            Ok(Event::Empty(e)) => {
                let raw_name = String::from_utf8_lossy(e.name().as_ref()).to_string();
                let local_name = strip_namespace(&raw_name).to_string();
                let lower_name = local_name.to_lowercase();

                exact_casing.entry(lower_name.clone()).or_insert_with(|| local_name.clone());

                let depth = stack.len();
                if let Some(parent) = stack.last() {
                    tag_parents.entry(lower_name.clone()).or_default().insert(parent.clone());
                    tag_child_tags.entry(parent.clone()).or_default().insert(lower_name.clone());
                }

                tag_depths
                    .entry(lower_name.clone())
                    .and_modify(|d| *d = std::cmp::min(*d, depth))
                    .or_insert(depth);

                *tag_counts.entry(lower_name).or_insert(0) += 1;
            }
            Ok(Event::Eof) => break,
            Err(_) => break,
            _ => {}
        }
    }

    let mut best_tag = "job".to_string();
    let mut best_score = -999999.0;

    for (tag, &count) in &tag_counts {
        let depth = *tag_depths.get(tag).unwrap_or(&1);
        let num_children = tag_child_tags.get(tag).map(|c| c.len()).unwrap_or(0);

        // Leaf tags cannot be the record container
        if num_children == 0 {
            continue;
        }
        // Root tag cannot be individual job
        if depth == 0 {
            continue;
        }

        let mut score = get_keyword_score(tag);

        if let Some(parents) = tag_parents.get(tag) {
            for parent in parents {
                if root_containers_set.contains(parent.as_str()) {
                    score += 50.0;
                }
                if parent == &format!("{tag}s") || parent == &format!("{tag}es") {
                    score += 80.0;
                }
                if get_keyword_score(parent) > 0.0 {
                    score -= 80.0;
                }
            }
        }

        match depth {
            1 => score += 40.0,
            2 => score += 35.0,
            3 => score -= 40.0,
            _ => score -= 80.0,
        }

        score += ((num_children as f64) * 5.0).min(50.0);
        score += ((count as f64) + 1.0).log2() * 5.0;

        if score > best_score {
            best_score = score;
            best_tag = tag.clone();
        }
    }

    Ok(exact_casing.get(&best_tag).cloned().unwrap_or(best_tag))
}

pub struct XmlStreamParser<R: BufRead> {
    reader: Reader<R>,
    buf: Vec<u8>,
    target_tag_lower: String,
    skip_descriptions: bool,
}

impl<R: BufRead> XmlStreamParser<R> {
    pub fn new(read: R, target_tag: &str, skip_descriptions: bool) -> Self {
        let mut reader = Reader::from_reader(read);
        reader.config_mut().trim_text(true);
        reader.config_mut().expand_empty_elements = false;

        Self {
            reader,
            buf: Vec::with_capacity(8192),
            target_tag_lower: target_tag.to_lowercase(),
            skip_descriptions,
        }
    }

    /// Reads the next record from the stream
    pub fn next_record(&mut self) -> Result<Option<XmlRecord>> {
        loop {
            self.buf.clear();
            match self.reader.read_event_into(&mut self.buf) {
                Ok(Event::Start(ref e)) => {
                    let raw_name = String::from_utf8_lossy(e.name().as_ref()).to_string();
                    let local_name = strip_namespace(&raw_name);
                    if local_name.eq_ignore_ascii_case(&self.target_tag_lower) {
                        let owned_start = e.to_owned();
                        let record = self.parse_record_body(&owned_start)?;
                        return Ok(Some(record));
                    }
                }
                Ok(Event::Empty(ref e)) => {
                    let raw_name = String::from_utf8_lossy(e.name().as_ref()).to_string();
                    let local_name = strip_namespace(&raw_name);
                    if local_name.eq_ignore_ascii_case(&self.target_tag_lower) {
                        let mut fields = HashMap::new();
                        for attr in e.attributes().flatten() {
                            let key = strip_namespace(&String::from_utf8_lossy(attr.key.as_ref())).to_string();
                            let val = String::from_utf8_lossy(&attr.value).to_string();
                            fields.insert(key, val);
                        }
                        return Ok(Some(XmlRecord { fields }));
                    }
                }
                Ok(Event::Eof) => return Ok(None),
                Err(e) => {
                    eprintln!("XML parsing warning: {:?}", e);
                    continue;
                }
                _ => {}
            }
        }
    }

    fn parse_record_body(&mut self, start: &BytesStart) -> Result<XmlRecord> {
        let mut fields: HashMap<String, String> = HashMap::new();
        let mut tag_stack: Vec<String> = Vec::new();
        let target_tag = strip_namespace(&String::from_utf8_lossy(start.name().as_ref())).to_lowercase();

        // Extract attributes from root record tag
        for attr in start.attributes().flatten() {
            let key = strip_namespace(&String::from_utf8_lossy(attr.key.as_ref())).to_string();
            let val = String::from_utf8_lossy(&attr.value).to_string();
            fields.insert(key, val);
        }

        let mut body_buf = Vec::with_capacity(4096);
        let mut current_text = String::new();

        loop {
            body_buf.clear();
            match self.reader.read_event_into(&mut body_buf) {
                Ok(Event::Start(ref e)) => {
                    let raw_name = String::from_utf8_lossy(e.name().as_ref()).to_string();
                    let sub_name = strip_namespace(&raw_name).to_string();
                    let sub_lower = sub_name.to_lowercase();

                    if self.skip_descriptions && (sub_lower.contains("description") || sub_lower == "body" || sub_lower == "content") {
                        let mut desc_depth = 1;
                        let mut skip_buf = Vec::with_capacity(512);
                        while desc_depth > 0 {
                            skip_buf.clear();
                            match self.reader.read_event_into(&mut skip_buf) {
                                Ok(Event::Start(_)) => desc_depth += 1,
                                Ok(Event::End(_)) => desc_depth -= 1,
                                Ok(Event::Eof) => break,
                                _ => {}
                            }
                        }
                        continue;
                    }

                    tag_stack.push(sub_name);
                    current_text.clear();

                    // Parse attributes on inner tags
                    for attr in e.attributes().flatten() {
                        let attr_key = strip_namespace(&String::from_utf8_lossy(attr.key.as_ref())).to_string();
                        let attr_val = String::from_utf8_lossy(&attr.value).to_string();
                        let full_key = if tag_stack.is_empty() {
                            attr_key
                        } else {
                            format!("{}.{}", tag_stack.join("."), attr_key)
                        };
                        fields.insert(full_key, attr_val);
                    }
                }
                Ok(Event::Empty(ref e)) => {
                    let raw_name = String::from_utf8_lossy(e.name().as_ref()).to_string();
                    let sub_name = strip_namespace(&raw_name).to_string();
                    for attr in e.attributes().flatten() {
                        let attr_key = strip_namespace(&String::from_utf8_lossy(attr.key.as_ref())).to_string();
                        let attr_val = String::from_utf8_lossy(&attr.value).to_string();
                        let full_key = if tag_stack.is_empty() {
                            format!("{sub_name}.{attr_key}")
                        } else {
                            format!("{}.{sub_name}.{attr_key}", tag_stack.join("."))
                        };
                        fields.insert(full_key, attr_val);
                    }
                }
                Ok(Event::Text(ref e)) => {
                    let text = e.unescape().unwrap_or_default().to_string();
                    if !text.trim().is_empty() {
                        current_text.push_str(&text);
                    }
                }
                Ok(Event::CData(ref e)) => {
                    let text = String::from_utf8_lossy(e.as_ref()).to_string();
                    if !text.trim().is_empty() {
                        current_text.push_str(&text);
                    }
                }
                Ok(Event::End(ref e)) => {
                    let raw_name = String::from_utf8_lossy(e.name().as_ref()).to_string();
                    let end_name = strip_namespace(&raw_name).to_lowercase();
                    if end_name == target_tag && tag_stack.is_empty() {
                        break;
                    }

                    if let Some(popped) = tag_stack.pop() {
                        if !current_text.trim().is_empty() {
                            let key = if tag_stack.is_empty() {
                                popped
                            } else {
                                format!("{}.{}", tag_stack.join("."), popped)
                            };
                            fields.insert(key, current_text.trim().to_string());
                        }
                    }
                    current_text.clear();
                }
                Ok(Event::Eof) => break,
                Err(_) => continue,
                _ => {}
            }
        }

        Ok(XmlRecord { fields })
    }
}
