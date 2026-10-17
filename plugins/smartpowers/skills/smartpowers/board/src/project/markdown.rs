//! Parsing for the Markdown conventions Smartpowers artifacts use:
//! an H1 title, `Key: value` header fields, requirement IDs, and links.

use std::collections::{BTreeMap, BTreeSet};

/// The H1 title and the `Key: value` fields that appear before the first H2.
/// Field names are lowercased; later duplicates win.
pub struct Header {
    pub title: String,
    fields: BTreeMap<String, String>,
}

impl Header {
    pub fn parse(text: &str) -> Self {
        let mut title = String::new();
        let mut fields = BTreeMap::new();
        for line in text.lines() {
            if line.starts_with("## ") {
                break;
            }
            if let Some(rest) = line.strip_prefix("# ") {
                if title.is_empty() {
                    title = rest.trim().to_string();
                    continue;
                }
            }
            if let Some((key, value)) = parse_field(line) {
                fields.insert(key, value);
            }
        }
        Header { title, fields }
    }

    /// Look up a field by its lowercase name, e.g. `"depends on"`.
    pub fn field(&self, name: &str) -> Option<&str> {
        self.fields.get(name).map(String::as_str)
    }

    /// The `Status:` field, or `"Unknown"` when absent.
    pub fn status(&self) -> String {
        self.field("status").unwrap_or("Unknown").to_string()
    }
}

/// `Key: value` where the key is ASCII letters and spaces, starting with a letter.
fn parse_field(line: &str) -> Option<(String, String)> {
    let (key, value) = line.split_once(':')?;
    let key = key.trim_end();
    let mut chars = key.chars();
    let starts_with_letter = chars.next()?.is_ascii_alphabetic();
    if !starts_with_letter || !chars.all(|c| c.is_ascii_alphabetic() || c == ' ') {
        return None;
    }
    Some((key.to_lowercase(), value.trim().to_string()))
}

fn is_word(byte: u8) -> bool {
    byte.is_ascii_alphanumeric() || byte == b'_'
}

/// `REQ-###` and `NFR-###` IDs as whole words, sorted and deduplicated.
pub fn requirement_ids(text: &str) -> Vec<String> {
    let bytes = text.as_bytes();
    let mut found = BTreeSet::new();
    for start in 0..bytes.len() {
        let rest = &bytes[start..];
        let is_prefix = rest.starts_with(b"REQ-") || rest.starts_with(b"NFR-");
        if !is_prefix || (start > 0 && is_word(bytes[start - 1])) {
            continue;
        }
        let end = start + 7;
        if end <= bytes.len()
            && bytes[start + 4..end].iter().all(u8::is_ascii_digit)
            && (end == bytes.len() || !is_word(bytes[end]))
        {
            found.insert(text[start..end].to_string());
        }
    }
    found.into_iter().collect()
}

/// Two-digit task references such as `01, 02`.
pub fn task_refs(text: &str) -> Vec<String> {
    text.split(|c: char| !(c.is_ascii_alphanumeric() || c == '_'))
        .filter(|token| token.len() == 2 && token.bytes().all(|b| b.is_ascii_digit()))
        .map(str::to_string)
        .collect()
}

/// The first http(s) URL, ending at whitespace or a closing bracket.
pub fn find_url(text: &str) -> Option<&str> {
    let start = [text.find("https://"), text.find("http://")]
        .into_iter()
        .flatten()
        .min()?;
    let rest = &text[start..];
    let end = rest
        .find(|c: char| c.is_whitespace() || matches!(c, ')' | '>' | ']'))
        .unwrap_or(rest.len());
    Some(&rest[..end])
}

/// The first issue key such as `ENG-42`: 2-10 uppercase letters/digits
/// starting with a letter, a hyphen, then digits.
pub fn find_issue_key(text: &str) -> Option<&str> {
    let bytes = text.as_bytes();
    for start in 0..bytes.len() {
        if !bytes[start].is_ascii_uppercase() || (start > 0 && is_word(bytes[start - 1])) {
            continue;
        }
        let mut hyphen = start + 1;
        while hyphen < bytes.len()
            && (bytes[hyphen].is_ascii_uppercase() || bytes[hyphen].is_ascii_digit())
        {
            hyphen += 1;
        }
        if !(2..=10).contains(&(hyphen - start)) || bytes.get(hyphen) != Some(&b'-') {
            continue;
        }
        let mut end = hyphen + 1;
        while end < bytes.len() && bytes[end].is_ascii_digit() {
            end += 1;
        }
        if end > hyphen + 1 && (end == bytes.len() || !is_word(bytes[end])) {
            return Some(&text[start..end]);
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_header_until_first_h2() {
        let header = Header::parse(
            "# 01: Title\n\nStatus: Done\nDepends on: 01, 02\nnot a field\n## Body\nStatus: Ignored\n",
        );
        assert_eq!(header.title, "01: Title");
        assert_eq!(header.status(), "Done");
        assert_eq!(header.field("depends on"), Some("01, 02"));
        assert_eq!(Header::parse("# Only title\n").status(), "Unknown");
    }

    #[test]
    fn extracts_ids_refs_urls_and_issue_keys() {
        assert_eq!(
            requirement_ids("NFR-001, REQ-002 REQ-002 XREQ-003 REQ-0044"),
            ["NFR-001", "REQ-002"]
        );
        assert_eq!(task_refs("01, 02 and 103"), ["01", "02"]);
        assert_eq!(find_url("ENG-1 (https://a.b/c) x"), Some("https://a.b/c"));
        assert_eq!(
            find_issue_key("see ENG-42 https://x/ENG-42"),
            Some("ENG-42")
        );
        assert_eq!(find_issue_key("eng-42 E-1"), None);
    }
}
