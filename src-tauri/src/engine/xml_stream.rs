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

const HTML_FORMATTING_TAGS: &[&str] = &[
    "li", "ul", "ol", "p", "br", "h1", "h2", "h3", "h4", "h5", "h6",
    "div", "span", "strong", "b", "i", "em", "u", "s", "a", "img",
    "table", "tr", "td", "th", "tbody", "thead", "tfoot", "font", "hr",
    "pre", "code", "blockquote", "style", "script", "button", "input",
    "header", "footer", "section", "article", "nav", "aside", "form"
];

fn get_keyword_score(tag: &str) -> f64 {
    if HTML_FORMATTING_TAGS.contains(&tag) {
        return -99999.0;
    }
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

pub fn is_description_tag(tag: &str) -> bool {
    let lower = tag.to_lowercase();
    lower.contains("description")
        || lower == "body"
        || lower == "content"
        || lower == "summary"
        || lower == "details"
        || lower == "post"
        || lower == "job_post"
        || lower == "jobpost"
        || lower.contains("post_body")
        || lower == "desc"
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
    reader.config_mut().check_end_names = false;

    let mut buf = Vec::with_capacity(4096);
    let mut stack: Vec<String> = Vec::with_capacity(32);
    let mut tag_counts: HashMap<String, usize> = HashMap::new();
    let mut tag_start_counts: HashMap<String, usize> = HashMap::new();
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
                *tag_start_counts.entry(lower_name.clone()).or_insert(0) += 1;

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
                *tag_start_counts.entry(lower_name.clone()).or_insert(0) += 1;

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
            Err(_) => {
                // Do NOT abort on malformed HTML tags inside XML feeds; continue scanning
                continue;
            }
            _ => {}
        }
    }

    let mut best_tag = "job".to_string();
    let mut best_score = -999999.0;

    for tag in exact_casing.keys() {
        // Never consider HTML formatting tags as the feed record root
        if HTML_FORMATTING_TAGS.contains(&tag.as_str()) {
            continue;
        }

        let end_c = *tag_counts.get(tag).unwrap_or(&0);
        let start_c = *tag_start_counts.get(tag).unwrap_or(&0);
        let count = std::cmp::max(end_c, start_c);

        if count == 0 {
            continue;
        }

        let depth = *tag_depths.get(tag).unwrap_or(&1);
        let num_children = tag_child_tags.get(tag).map(|c| c.len()).unwrap_or(0);

        // Leaf tags cannot be the record container unless it's a known job tag with attributes
        if num_children == 0 && get_keyword_score(tag) <= 0.0 {
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
                    score += 60.0;
                }
                if parent == &format!("{tag}s") || parent == &format!("{tag}es") {
                    score += 90.0;
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
        reader.config_mut().check_end_names = false;

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

                    if self.skip_descriptions && is_description_tag(&sub_lower) {
                        let mut desc_depth: u32 = 1;
                        let mut skip_buf = Vec::with_capacity(512);
                        while desc_depth > 0 {
                            skip_buf.clear();
                            match self.reader.read_event_into(&mut skip_buf) {
                                Ok(Event::Start(ref se)) => {
                                    let sname = strip_namespace(&String::from_utf8_lossy(se.name().as_ref())).to_lowercase();
                                    if sname == target_tag {
                                        break;
                                    }
                                    if !HTML_FORMATTING_TAGS.contains(&sname.as_str()) {
                                        desc_depth += 1;
                                    }
                                }
                                Ok(Event::End(ref ee)) => {
                                    let ename = strip_namespace(&String::from_utf8_lossy(ee.name().as_ref())).to_lowercase();
                                    if ename == target_tag || ename == sub_lower {
                                        break;
                                    }
                                    if !HTML_FORMATTING_TAGS.contains(&ename.as_str()) {
                                        desc_depth = desc_depth.saturating_sub(1);
                                    }
                                }
                                Ok(Event::Eof) => break,
                                Err(_) => continue,
                                _ => {}
                            }
                        }
                        continue;
                    }

                    // Inside description or if it's an HTML tag, treat it as formatting instead of creating sub-columns
                    let inside_desc = tag_stack.iter().any(|t| is_description_tag(t));
                    if HTML_FORMATTING_TAGS.contains(&sub_lower.as_str()) || inside_desc {
                        if sub_lower == "p" || sub_lower == "br" || sub_lower == "tr" || sub_lower.starts_with('h') {
                            if !current_text.is_empty() && !current_text.ends_with('\n') {
                                current_text.push('\n');
                            }
                        } else if sub_lower == "li" {
                            if !current_text.is_empty() && !current_text.ends_with('\n') {
                                current_text.push('\n');
                            }
                            current_text.push_str("• ");
                        }
                        // If it's an HTML formatting tag, do not push to tag_stack
                        if HTML_FORMATTING_TAGS.contains(&sub_lower.as_str()) {
                            continue;
                        }
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
                    let sub_lower = sub_name.to_lowercase();

                    if HTML_FORMATTING_TAGS.contains(&sub_lower.as_str()) {
                        if sub_lower == "br" || sub_lower == "hr" {
                            if !current_text.is_empty() && !current_text.ends_with('\n') {
                                current_text.push('\n');
                            }
                        }
                        continue;
                    }

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
                        if !current_text.is_empty() && !current_text.ends_with(' ') && !current_text.ends_with('\n') {
                            current_text.push(' ');
                        }
                        current_text.push_str(text.trim());
                    }
                }
                Ok(Event::CData(ref e)) => {
                    let text = String::from_utf8_lossy(e.as_ref()).to_string();
                    if !text.trim().is_empty() {
                        if !current_text.is_empty() && !current_text.ends_with(' ') && !current_text.ends_with('\n') {
                            current_text.push(' ');
                        }
                        current_text.push_str(text.trim());
                    }
                }
                Ok(Event::End(ref e)) => {
                    let raw_name = String::from_utf8_lossy(e.name().as_ref()).to_string();
                    let end_name = strip_namespace(&raw_name).to_lowercase();

                    // If end of the target record tag, break unconditionally
                    if end_name == target_tag {
                        break;
                    }

                    // If it's an HTML tag, adjust formatting and don't pop tag_stack
                    if HTML_FORMATTING_TAGS.contains(&end_name.as_str()) {
                        if end_name == "p" || end_name == "li" || end_name == "tr" || end_name.starts_with('h') {
                            if !current_text.is_empty() && !current_text.ends_with('\n') {
                                current_text.push('\n');
                            }
                        }
                        continue;
                    }

                    if let Some(pos) = tag_stack.iter().rposition(|x| x.eq_ignore_ascii_case(&end_name)) {
                        let field_key = tag_stack[..=pos].join(".");
                        if let Some(existing) = fields.get_mut(&field_key) {
                            if !existing.is_empty() && !current_text.trim().is_empty() {
                                existing.push_str(", ");
                                existing.push_str(current_text.trim());
                            }
                        } else if !current_text.trim().is_empty() {
                            fields.insert(field_key.clone(), current_text.trim().to_string());
                        }

                        // Fast O(1) check: only inspect for key-value pair container when the tag is likely a customfield/item
                        if end_name.contains("custom")
                            || end_name == "field"
                            || end_name == "param"
                            || end_name == "property"
                            || end_name == "attribute"
                            || end_name == "item"
                            || end_name == "entry"
                        {
                            let base_prefix = tag_stack[..=pos].join(".");

                            // Direct O(1) probe for name and value keys
                            let name_suffixes = [".name", ".Name", ".key", ".Key", ".id", ".Id", ".param", ".field"];
                            let val_suffixes = [".value", ".Value", ".val", ".Val", ".content", ".text"];

                            let mut name_key_to_remove = None;
                            let mut name_val = None;
                            for s in &name_suffixes {
                                let test_key = format!("{}{}", base_prefix, s);
                                if let Some(v) = fields.get(&test_key) {
                                    name_key_to_remove = Some(test_key);
                                    name_val = Some(v.clone());
                                    break;
                                }
                            }

                            if let (Some(nk), Some(nv)) = (name_key_to_remove, name_val) {
                                let mut val_key_to_remove = None;
                                let mut value_val = None;
                                for s in &val_suffixes {
                                    let test_key = format!("{}{}", base_prefix, s);
                                    if let Some(v) = fields.get(&test_key) {
                                        val_key_to_remove = Some(test_key);
                                        value_val = Some(v.clone());
                                        break;
                                    }
                                }

                                fields.remove(&nk);
                                if let Some(vk) = val_key_to_remove {
                                    fields.remove(&vk);
                                }
                                let val = value_val.unwrap_or_default();
                                let n_clean = nv.trim();
                                if !n_clean.is_empty() {
                                    let parent_path = if pos > 0 {
                                        tag_stack[..pos].join(".")
                                    } else {
                                        String::new()
                                    };
                                    let pivoted_key = if parent_path.is_empty() {
                                        n_clean.to_string()
                                    } else {
                                        format!("{parent_path}.{n_clean}")
                                    };
                                    if let Some(existing) = fields.get_mut(&pivoted_key) {
                                        if !val.is_empty() {
                                            if !existing.is_empty() {
                                                existing.push_str(", ");
                                            }
                                            existing.push_str(&val);
                                        }
                                    } else {
                                        fields.insert(pivoted_key, val);
                                    }
                                }
                            }
                        }

                        tag_stack.truncate(pos);
                        current_text.clear();
                    }
                }
                Ok(Event::Eof) => break,
                Err(_) => continue,
                _ => {}
            }
        }

        Ok(XmlRecord { fields })
    }
}
