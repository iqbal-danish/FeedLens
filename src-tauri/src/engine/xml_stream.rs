use std::collections::HashMap;
use std::io::BufRead;
use anyhow::Result;
use quick_xml::events::{BytesStart, Event};
use quick_xml::reader::Reader;

pub struct XmlRecord {
    pub fields: HashMap<String, String>,
}

pub struct XmlStreamParser<R: BufRead> {
    reader: Reader<R>,
    buf: Vec<u8>,
    record_tag: Option<String>,
    depth: usize,
}

impl<R: BufRead> XmlStreamParser<R> {
    pub fn new(read: R, record_tag: Option<String>) -> Self {
        let mut reader = Reader::from_reader(read);
        reader.config_mut().trim_text(true);
        reader.config_mut().expand_empty_elements = true;

        Self {
            reader,
            buf: Vec::with_capacity(8192),
            record_tag,
            depth: 0,
        }
    }

    /// Reads the next record from the stream
    pub fn next_record(&mut self) -> Result<Option<XmlRecord>> {
        let candidates = ["job", "item", "product", "record", "entry", "offer", "row", "article", "listing", "entity", "doc"];

        loop {
            self.buf.clear();
            let event = self.reader.read_event_into(&mut self.buf);
            match event {
                Ok(Event::Start(ref e)) => {
                    self.depth += 1;
                    let name = String::from_utf8_lossy(e.name().as_ref()).to_string();
                    let lower = name.to_lowercase();

                    let is_target = match &self.record_tag {
                        Some(target) => lower == target.to_lowercase(),
                        None => {
                            // Check candidates first
                            if candidates.iter().any(|&c| c == lower) {
                                self.record_tag = Some(lower);
                                true
                            } else if self.depth >= 2 && lower != "channel" && lower != "source" && lower != "header" && lower != "metadata" {
                                // Default to repeating child of root
                                self.record_tag = Some(lower);
                                true
                            } else {
                                false
                            }
                        }
                    };

                    if is_target {
                        let owned_start = e.to_owned();
                        let record = self.parse_record_body(&owned_start)?;
                        // End of record tag was consumed in parse_record_body, so decrement depth
                        if self.depth > 0 {
                            self.depth -= 1;
                        }
                        return Ok(Some(record));
                    }
                }
                Ok(Event::End(_)) => {
                    if self.depth > 0 {
                        self.depth -= 1;
                    }
                }
                Ok(Event::Eof) => return Ok(None),
                Err(e) => {
                    // Robust error recovery: skip corrupted bytes
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
        let target_tag = String::from_utf8_lossy(start.name().as_ref()).to_string();

        // Extract attributes from the root record tag itself
        for attr in start.attributes().flatten() {
            let key = String::from_utf8_lossy(attr.key.as_ref()).to_string();
            let val = String::from_utf8_lossy(&attr.value).to_string();
            fields.insert(key, val);
        }

        let mut body_buf = Vec::with_capacity(4096);
        let mut current_text = String::new();

        loop {
            body_buf.clear();
            match self.reader.read_event_into(&mut body_buf) {
                Ok(Event::Start(ref e)) => {
                    let sub_name = String::from_utf8_lossy(e.name().as_ref()).to_string();
                    tag_stack.push(sub_name.clone());
                    current_text.clear();

                    // Parse attributes on inner tags
                    for attr in e.attributes().flatten() {
                        let attr_key = String::from_utf8_lossy(attr.key.as_ref()).to_string();
                        let attr_val = String::from_utf8_lossy(&attr.value).to_string();
                        let full_key = if tag_stack.is_empty() {
                            attr_key
                        } else {
                            format!("{}.{}", tag_stack.join("."), attr_key)
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
                    let end_name = String::from_utf8_lossy(e.name().as_ref()).to_string();
                    if end_name.eq_ignore_ascii_case(&target_tag) && tag_stack.is_empty() {
                        // End of this record!
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
                Err(_) => {
                    // Skip invalid tokens inside record
                    continue;
                }
                _ => {}
            }
        }

        Ok(XmlRecord { fields })
    }
}
