//! Minimal parsers for contact QR codes: vCard (`BEGIN:VCARD`) and MECARD (`MECARD:`).
//!
//! Only the fields we display are extracted; the raw text is kept so the user can save
//! the original vCard unchanged.

use serde::{Deserialize, Serialize};

/// A contact card.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Contact {
    pub name: String,
    pub organization: Option<String>,
    pub phones: Vec<String>,
    pub emails: Vec<String>,
    pub urls: Vec<String>,
    pub address: Option<String>,
    pub note: Option<String>,
}

impl Contact {
    /// Serialises the contact as a vCard 3.0 document (used when the source was MECARD).
    pub fn to_vcard(&self) -> String {
        let mut out = String::from("BEGIN:VCARD\r\nVERSION:3.0\r\n");
        out.push_str(&format!("FN:{}\r\n", vcard_escape(&self.name)));
        out.push_str(&format!("N:{};;;;\r\n", vcard_escape(&self.name)));
        if let Some(org) = &self.organization {
            out.push_str(&format!("ORG:{}\r\n", vcard_escape(org)));
        }
        for p in &self.phones {
            out.push_str(&format!("TEL:{}\r\n", vcard_escape(p)));
        }
        for e in &self.emails {
            out.push_str(&format!("EMAIL:{}\r\n", vcard_escape(e)));
        }
        for u in &self.urls {
            out.push_str(&format!("URL:{}\r\n", vcard_escape(u)));
        }
        if let Some(a) = &self.address {
            out.push_str(&format!("ADR:;;{};;;;\r\n", vcard_escape(a)));
        }
        if let Some(n) = &self.note {
            out.push_str(&format!("NOTE:{}\r\n", vcard_escape(n)));
        }
        out.push_str("END:VCARD\r\n");
        out
    }
}

fn vcard_escape(s: &str) -> String {
    s.replace('\\', "\\\\")
        .replace(',', "\\,")
        .replace(';', "\\;")
        .replace('\n', "\\n")
}

fn vcard_unescape(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    let mut chars = s.chars();
    while let Some(c) = chars.next() {
        if c == '\\' {
            match chars.next() {
                Some('n') | Some('N') => out.push('\n'),
                Some(n) => out.push(n),
                None => {}
            }
        } else {
            out.push(c);
        }
    }
    out
}

/// Parses a vCard. Returns `None` if `input` is not a vCard.
pub fn parse_vcard(input: &str) -> Option<Contact> {
    let trimmed = input.trim_start();
    if !trimmed
        .get(..11)
        .is_some_and(|p| p.eq_ignore_ascii_case("BEGIN:VCARD"))
    {
        return None;
    }
    // Unfold continuation lines (RFC 6350 §3.2).
    let unfolded = trimmed
        .replace("\r\n ", "")
        .replace("\r\n\t", "")
        .replace("\n ", "")
        .replace("\n\t", "");
    let mut c = Contact::default();
    let mut structured_name = None;
    for line in unfolded.lines() {
        let Some((head, value)) = line.split_once(':') else {
            continue;
        };
        // property name without parameters / group prefix
        let prop = head
            .split(';')
            .next()
            .unwrap_or("")
            .rsplit('.')
            .next()
            .unwrap_or("")
            .to_ascii_uppercase();
        let value = value.trim();
        match prop.as_str() {
            "FN" => c.name = vcard_unescape(value),
            "N" => {
                let parts: Vec<String> = value.split(';').map(vcard_unescape).collect();
                let given = parts.get(1).cloned().unwrap_or_default();
                let family = parts.first().cloned().unwrap_or_default();
                let joined = format!("{given} {family}").trim().to_string();
                if !joined.is_empty() {
                    structured_name = Some(joined);
                }
            }
            "ORG" => c.organization = Some(vcard_unescape(&value.replace(';', " "))),
            "TEL" => c.phones.push(vcard_unescape(value)),
            "EMAIL" => c.emails.push(vcard_unescape(value)),
            "URL" => c.urls.push(vcard_unescape(value)),
            "ADR" => {
                let addr: Vec<String> = value
                    .split(';')
                    .map(vcard_unescape)
                    .filter(|p| !p.trim().is_empty())
                    .collect();
                if !addr.is_empty() {
                    c.address = Some(addr.join(", "));
                }
            }
            "NOTE" => c.note = Some(vcard_unescape(value)),
            _ => {}
        }
    }
    if c.name.is_empty() {
        c.name = structured_name.unwrap_or_default();
    }
    Some(c)
}

/// Parses a MECARD (`MECARD:N:Doe,John;TEL:123;EMAIL:a@b.c;;`).
pub fn parse_mecard(input: &str) -> Option<Contact> {
    let trimmed = input.trim();
    let body = match trimmed.get(..7) {
        Some(prefix) if prefix.eq_ignore_ascii_case("MECARD:") => &trimmed[7..],
        _ => return None,
    };
    let mut c = Contact::default();
    for field in split_unescaped(body, ';') {
        let Some((key, value)) = field.split_once(':') else {
            continue;
        };
        let value = mecard_unescape(value);
        match key.to_ascii_uppercase().as_str() {
            "N" => {
                // "Family,Given" → "Given Family"
                c.name = match value.split_once(',') {
                    Some((family, given)) => format!("{} {}", given.trim(), family.trim()).trim().to_string(),
                    None => value,
                }
            }
            "ORG" => c.organization = Some(value),
            "TEL" => c.phones.push(value),
            "EMAIL" => c.emails.push(value),
            "URL" => c.urls.push(value),
            "ADR" => c.address = Some(value),
            "NOTE" => c.note = Some(value),
            _ => {}
        }
    }
    Some(c)
}

fn split_unescaped(s: &str, sep: char) -> Vec<String> {
    let mut out = Vec::new();
    let mut cur = String::new();
    let mut chars = s.chars();
    while let Some(c) = chars.next() {
        if c == '\\' {
            cur.push(c);
            if let Some(n) = chars.next() {
                cur.push(n);
            }
        } else if c == sep {
            if !cur.is_empty() {
                out.push(std::mem::take(&mut cur));
            }
        } else {
            cur.push(c);
        }
    }
    if !cur.is_empty() {
        out.push(cur);
    }
    out
}

fn mecard_unescape(s: &str) -> String {
    let mut out = String::new();
    let mut chars = s.chars();
    while let Some(c) = chars.next() {
        if c == '\\' {
            if let Some(n) = chars.next() {
                out.push(n);
            }
        } else {
            out.push(c);
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn vcard_basic() {
        let v = "BEGIN:VCARD\nVERSION:3.0\nN:Chang;Jen-Yi;;;\nFN:Jen-Yi Chang\nORG:NKUST\nTEL;TYPE=cell:+886-912-345-678\nEMAIL:test@example.com\nURL:https://example.com\nEND:VCARD";
        let c = parse_vcard(v).unwrap();
        assert_eq!(c.name, "Jen-Yi Chang");
        assert_eq!(c.organization.as_deref(), Some("NKUST"));
        assert_eq!(c.phones, vec!["+886-912-345-678"]);
        assert_eq!(c.emails, vec!["test@example.com"]);
        assert_eq!(c.urls, vec!["https://example.com"]);
    }

    #[test]
    fn vcard_without_fn_uses_n() {
        let v = "BEGIN:VCARD\r\nVERSION:3.0\r\nN:Doe;John;;;\r\nEND:VCARD\r\n";
        assert_eq!(parse_vcard(v).unwrap().name, "John Doe");
    }

    #[test]
    fn mecard_basic_and_roundtrip() {
        let m = r"MECARD:N:Doe,John;TEL:+1234;EMAIL:john@example.com;NOTE:a\;b;;";
        let c = parse_mecard(m).unwrap();
        assert_eq!(c.name, "John Doe");
        assert_eq!(c.phones, vec!["+1234"]);
        assert_eq!(c.note.as_deref(), Some("a;b"));
        let vcard = c.to_vcard();
        assert!(vcard.starts_with("BEGIN:VCARD"));
        let back = parse_vcard(&vcard).unwrap();
        assert_eq!(back.name, "John Doe");
        assert_eq!(back.note.as_deref(), Some("a;b"));
    }

    #[test]
    fn rejects_other_text() {
        assert!(parse_vcard("hello").is_none());
        assert!(parse_mecard("hello").is_none());
    }
}
