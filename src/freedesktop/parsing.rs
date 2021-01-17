use std::mem;

use super::Section;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
enum LineKind<'a> {
    SectionHeader(&'a str),
    KeyValue {
        key: &'a str,
        value: &'a str,
        attribute: Option<&'a str>,
    },
    Comment(&'a str),
    Whitespace,
}

fn parse_line(raw: &str) -> Result<LineKind<'_>, &str> {
    let raw = raw.trim();
    if raw.is_empty() {
        Ok(LineKind::Whitespace)
    } else if let Some(comment) = raw.strip_prefix('#') {
        Ok(LineKind::Comment(comment))
    } else if let Some(header) = raw.strip_prefix('[').and_then(|cur| cur.strip_suffix(']')) {
        Ok(LineKind::SectionHeader(header))
    } else if let Some((raw_key, val)) = split_at_first(raw, '=') {
        let raw_key = raw_key.trim();
        let value = val.trim();
        let attribute_parse_res = split_at_first(raw_key, '[').and_then(|(k, attr_with_suffix)| {
            let attr = attr_with_suffix.strip_suffix(']')?;
            Some((k, attr))
        });
        if let Some((key, attribute)) = attribute_parse_res {
            Ok(LineKind::KeyValue {
                key,
                value,
                attribute: Some(attribute),
            })
        } else {
            Ok(LineKind::KeyValue {
                key: raw_key,
                value,
                attribute: None,
            })
        }
    } else {
        Err(raw)
    }
}

fn split_at_first(raw: &str, sep: char) -> Option<(&str, &str)> {
    let mut itr = raw.splitn(2, sep);
    let first = itr.next()?;
    let second = itr.next()?;
    Some((first, second))
}

#[derive(Default)]
pub struct SectionReader {
    cur_section: Section,
}

impl SectionReader {
    pub fn new() -> Self {
        Self {
            cur_section: Section::default(),
        }
    }
    pub fn push<'a>(&mut self, raw_line: &'a str) -> Option<Section> {
        let next_line = parse_line(&raw_line);
        match next_line {
            Ok(LineKind::SectionHeader(header)) => {
                let old_section =
                    mem::replace(&mut self.cur_section, Section::new(header.to_owned()));
                if !old_section.is_blank() {
                    return Some(old_section);
                }
            }
            Ok(LineKind::KeyValue {
                key,
                value,
                attribute,
            }) => {
                let entmap = self.cur_section.fields.entry(key.to_owned()).or_default();
                let old = match attribute {
                    Some(atr) => entmap.attributes.insert(atr.to_owned(), value.to_owned()),
                    None => entmap.default.replace(value.to_owned()),
                };
                if let Some(old) = old {
                    todo!(
                        "Handle dup entries: {:?}/{:?} => {:?}, {:?}",
                        key,
                        attribute,
                        old,
                        value
                    );
                }
            }
            Ok(LineKind::Comment(..)) | Ok(LineKind::Whitespace) => {}
            Err(e) => {
                todo!("Found parser error for line : {:?}", e);
            }
        }
        None
    }
    pub fn finish(self) -> Option<Section> {
        if !self.cur_section.is_blank() {
            Some(self.cur_section)
        } else {
            None
        }
    }
}
