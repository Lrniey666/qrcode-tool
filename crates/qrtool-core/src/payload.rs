//! Classification of decoded barcode text into typed payloads.

use serde::{Deserialize, Serialize};
use url::Url;

use crate::contact::{parse_mecard, parse_vcard, Contact};
use crate::einvoice::{self, EInvoice};
use crate::url_safety::{analyze_url, UrlError, UrlReport};
use crate::wifi::{parse_wifi, WifiConfig};

/// What a decoded code contains.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum Payload {
    /// A web link (`http`/`https`) with its safety report.
    Url(UrlReport),
    /// A URI with another scheme. **Never opened** — shown and copyable only.
    OtherScheme {
        scheme: String,
    },
    Wifi(WifiConfig),
    Contact(Contact),
    Email {
        to: String,
        subject: Option<String>,
        body: Option<String>,
    },
    Phone {
        number: String,
    },
    Sms {
        number: String,
        body: Option<String>,
    },
    Geo {
        lat: f64,
        lon: f64,
        query: Option<String>,
    },
    /// An `otpauth://` two-factor secret. Sensitive: never stored, never auto-copied.
    Otp {
        issuer: Option<String>,
        account: Option<String>,
    },
    Crypto {
        scheme: String,
        address: String,
        amount: Option<String>,
    },
    /// Left QR code of a Taiwan e-invoice.
    EInvoice(Box<EInvoice>),
    /// Right QR code of a Taiwan e-invoice (`**…`), meaningful only with the left one.
    EInvoiceContinuation,
    Text,
}

/// Coarse kind, used for icons, history and translations.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum PayloadKind {
    Url,
    OtherScheme,
    Wifi,
    Contact,
    Email,
    Phone,
    Sms,
    Geo,
    Otp,
    Crypto,
    EInvoice,
    EInvoiceContinuation,
    Text,
}

impl PayloadKind {
    /// Stable identifier (matches the `serde` name) used by the UI for icons and labels.
    pub fn id(self) -> &'static str {
        match self {
            PayloadKind::Url => "url",
            PayloadKind::OtherScheme => "other-scheme",
            PayloadKind::Wifi => "wifi",
            PayloadKind::Contact => "contact",
            PayloadKind::Email => "email",
            PayloadKind::Phone => "phone",
            PayloadKind::Sms => "sms",
            PayloadKind::Geo => "geo",
            PayloadKind::Otp => "otp",
            PayloadKind::Crypto => "crypto",
            PayloadKind::EInvoice => "einvoice",
            PayloadKind::EInvoiceContinuation => "einvoice-cont",
            PayloadKind::Text => "text",
        }
    }
}

impl Payload {
    pub fn kind(&self) -> PayloadKind {
        match self {
            Payload::Url(_) => PayloadKind::Url,
            Payload::OtherScheme { .. } => PayloadKind::OtherScheme,
            Payload::Wifi(_) => PayloadKind::Wifi,
            Payload::Contact(_) => PayloadKind::Contact,
            Payload::Email { .. } => PayloadKind::Email,
            Payload::Phone { .. } => PayloadKind::Phone,
            Payload::Sms { .. } => PayloadKind::Sms,
            Payload::Geo { .. } => PayloadKind::Geo,
            Payload::Otp { .. } => PayloadKind::Otp,
            Payload::Crypto { .. } => PayloadKind::Crypto,
            Payload::EInvoice(_) => PayloadKind::EInvoice,
            Payload::EInvoiceContinuation => PayloadKind::EInvoiceContinuation,
            Payload::Text => PayloadKind::Text,
        }
    }

    /// Sensitive payloads must never be written to history, logs or the clipboard
    /// automatically.
    pub fn is_sensitive(&self) -> bool {
        match self {
            Payload::Otp { .. } => true,
            Payload::Wifi(w) => w.password.is_some(),
            _ => false,
        }
    }
}

/// Classifies decoded `text`. `raw_bytes` (the undecoded byte payload) improves Big5
/// e-invoice handling but is optional.
pub fn classify(text: &str, raw_bytes: Option<&[u8]>) -> Payload {
    let trimmed = text.trim();

    if einvoice::is_right_part(trimmed) {
        return Payload::EInvoiceContinuation;
    }
    if let Some(inv) = einvoice::parse_left(trimmed, raw_bytes) {
        return Payload::EInvoice(Box::new(inv));
    }
    if let Some(w) = parse_wifi(trimmed) {
        return Payload::Wifi(w);
    }
    if let Some(c) = parse_vcard(trimmed).or_else(|| parse_mecard(trimmed)) {
        return Payload::Contact(c);
    }

    let lower = trimmed.to_ascii_lowercase();

    // Bare domains like "www.example.com" are common on posters; treat them as https.
    if !lower.contains("://") && looks_like_bare_domain(trimmed) {
        if let Ok(report) = analyze_url(&format!("https://{trimmed}")) {
            return Payload::Url(report);
        }
    }

    if let Some(p) = parse_special_scheme(trimmed, &lower) {
        return p;
    }

    match analyze_url(trimmed) {
        Ok(report) => Payload::Url(report),
        Err(UrlError::NotWeb(scheme)) if looks_like_uri(trimmed, &scheme) => Payload::OtherScheme { scheme },
        _ => Payload::Text,
    }
}

fn looks_like_uri(text: &str, scheme: &str) -> bool {
    // "Note: remember milk" parses as scheme "note". A real URI has no space right after
    // the colon and fits on one line, so plain sentences stay text.
    if text.contains('\n') || scheme.len() < 2 {
        return false;
    }
    match text.get(scheme.len() + 1..) {
        Some(after) => !after.is_empty() && !after.starts_with(char::is_whitespace),
        None => false,
    }
}

fn looks_like_bare_domain(s: &str) -> bool {
    if s.contains(char::is_whitespace) || s.contains('@') || s.contains(':') {
        return false;
    }
    let host = s.split('/').next().unwrap_or("");
    let labels: Vec<&str> = host.split('.').collect();
    if labels.len() < 2 || labels.iter().any(|l| l.is_empty()) {
        return false;
    }
    let tld = labels.last().copied().unwrap_or("");
    let tld_ok = tld.len() >= 2 && tld.chars().all(|c| c.is_ascii_alphabetic());
    let starts_www = host.to_ascii_lowercase().starts_with("www.");
    // Accept "www.x.y" always; otherwise only common TLDs to avoid "file.txt" false positives.
    let common = [
        "com", "net", "org", "tw", "edu", "gov", "io", "dev", "app", "jp", "cn", "hk", "uk", "de", "fr",
        "kr", "info", "me", "co",
    ];
    tld_ok && (starts_www || common.contains(&tld.to_ascii_lowercase().as_str()))
}

fn parse_special_scheme(text: &str, lower: &str) -> Option<Payload> {
    if lower.starts_with("mailto:") {
        let url = Url::parse(text).ok()?;
        let to = percent_decode(url.path());
        let mut subject = None;
        let mut body = None;
        for (k, v) in url.query_pairs() {
            match k.to_ascii_lowercase().as_str() {
                "subject" => subject = Some(v.into_owned()),
                "body" => body = Some(v.into_owned()),
                _ => {}
            }
        }
        return Some(Payload::Email { to, subject, body });
    }
    if lower.starts_with("matmsg:") {
        // MATMSG:TO:a@b.c;SUB:subject;BODY:text;;
        let mut to = String::new();
        let mut subject = None;
        let mut body = None;
        for field in text[7..].split(';') {
            if let Some((k, v)) = field.split_once(':') {
                match k.to_ascii_uppercase().as_str() {
                    "TO" => to = v.to_string(),
                    "SUB" => subject = Some(v.to_string()),
                    "BODY" => body = Some(v.to_string()),
                    _ => {}
                }
            }
        }
        return Some(Payload::Email { to, subject, body });
    }
    if lower.starts_with("tel:") {
        return Some(Payload::Phone {
            number: text[4..].trim().to_string(),
        });
    }
    if lower.starts_with("smsto:") || lower.starts_with("sms:") || lower.starts_with("mmsto:") {
        let rest = &text[text.find(':')? + 1..];
        // SMSTO:number:body  or  sms:number?body=...
        if let Some((num, q)) = rest.split_once('?') {
            let body = q
                .split('&')
                .find_map(|kv| kv.strip_prefix("body="))
                .map(percent_decode);
            return Some(Payload::Sms {
                number: num.to_string(),
                body,
            });
        }
        let (number, body) = match rest.split_once(':') {
            Some((n, b)) => (n.to_string(), Some(b.to_string())),
            None => (rest.to_string(), None),
        };
        return Some(Payload::Sms { number, body });
    }
    if lower.starts_with("geo:") {
        let rest = &text[4..];
        let (coords, query) = match rest.split_once('?') {
            Some((c, q)) => (c, q.strip_prefix("q=").map(percent_decode)),
            None => (rest, None),
        };
        let coords = coords.split(';').next().unwrap_or(coords);
        let mut it = coords.split(',');
        let lat: f64 = it.next()?.trim().parse().ok()?;
        let lon: f64 = it.next()?.trim().parse().ok()?;
        if !(-90.0..=90.0).contains(&lat) || !(-180.0..=180.0).contains(&lon) {
            return None;
        }
        return Some(Payload::Geo { lat, lon, query });
    }
    if lower.starts_with("otpauth://") || lower.starts_with("otpauth-migration://") {
        let mut issuer = None;
        let mut account = None;
        if let Ok(url) = Url::parse(text) {
            let label = percent_decode(url.path().trim_start_matches('/'));
            match label.split_once(':') {
                Some((i, a)) => {
                    issuer = Some(i.to_string());
                    account = Some(a.trim().to_string());
                }
                None if !label.is_empty() => account = Some(label),
                None => {}
            }
            for (k, v) in url.query_pairs() {
                if k == "issuer" {
                    issuer = Some(v.into_owned());
                }
            }
        }
        return Some(Payload::Otp { issuer, account });
    }
    for scheme in [
        "bitcoin",
        "ethereum",
        "litecoin",
        "bitcoincash",
        "dogecoin",
        "monero",
        "solana",
        "tron",
    ] {
        if lower.starts_with(&format!("{scheme}:")) {
            let original = &text[scheme.len() + 1..];
            let (addr, query) = match original.split_once('?') {
                Some((a, q)) => (a, Some(q)),
                None => (original, None),
            };
            let amount = query.and_then(|q| {
                q.split('&')
                    .find_map(|kv| kv.strip_prefix("amount=").or(kv.strip_prefix("value=")))
                    .map(str::to_string)
            });
            return Some(Payload::Crypto {
                scheme: scheme.to_string(),
                address: addr.trim_start_matches("//").to_string(),
                amount,
            });
        }
    }
    None
}

fn percent_decode(s: &str) -> String {
    let bytes = s.as_bytes();
    let mut out = Vec::with_capacity(bytes.len());
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'%' && i + 2 < bytes.len() {
            let hex = std::str::from_utf8(&bytes[i + 1..i + 3]).unwrap_or("");
            if let Ok(v) = u8::from_str_radix(hex, 16) {
                out.push(v);
                i += 3;
                continue;
            }
        }
        if bytes[i] == b'+' {
            out.push(b' ');
        } else {
            out.push(bytes[i]);
        }
        i += 1;
    }
    String::from_utf8_lossy(&out).into_owned()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::url_safety::Severity;

    #[test]
    fn web_urls() {
        match classify("https://oosaf.nkust.edu.tw/form", None) {
            Payload::Url(r) => assert_eq!(r.host_display, "oosaf.nkust.edu.tw"),
            other => panic!("unexpected {other:?}"),
        }
        match classify("www.example.com/path", None) {
            Payload::Url(r) => assert_eq!(r.url, "https://www.example.com/path"),
            other => panic!("unexpected {other:?}"),
        }
    }

    #[test]
    fn unsafe_schemes_are_never_urls() {
        for s in ["ms-settings:display", "file:///etc/passwd", "javascript:alert(1)"] {
            assert_eq!(classify(s, None).kind(), PayloadKind::OtherScheme, "{s}");
        }
    }

    #[test]
    fn plain_text_stays_text() {
        assert_eq!(
            classify("Hello 繁體中文測試 提取碼: ab12", None).kind(),
            PayloadKind::Text
        );
        assert_eq!(classify("Note: buy milk", None).kind(), PayloadKind::Text);
        assert_eq!(classify("readme.txt", None).kind(), PayloadKind::Text);
        assert_eq!(classify("1234567890", None).kind(), PayloadKind::Text);
    }

    #[test]
    fn multibyte_text_never_panics() {
        for s in [
            "Hello 繁體中文測試",
            "繁",
            "€uro",
            "W繁IFI:",
            "MECAR繁D",
            "otpauth繁",
            "\u{FEFF}x",
        ] {
            let _ = classify(s, None);
        }
    }

    #[test]
    fn special_schemes() {
        assert_eq!(
            classify("mailto:a@b.com?subject=Hi%20there", None),
            Payload::Email {
                to: "a@b.com".into(),
                subject: Some("Hi there".into()),
                body: None
            }
        );
        assert_eq!(
            classify("tel:+886912345678", None),
            Payload::Phone {
                number: "+886912345678".into()
            }
        );
        assert_eq!(
            classify("SMSTO:0912345678:hello", None),
            Payload::Sms {
                number: "0912345678".into(),
                body: Some("hello".into())
            }
        );
        match classify("geo:22.6273,120.3014?q=Kaohsiung", None) {
            Payload::Geo { lat, lon, query } => {
                assert!((lat - 22.6273).abs() < 1e-6);
                assert!((lon - 120.3014).abs() < 1e-6);
                assert_eq!(query.as_deref(), Some("Kaohsiung"));
            }
            other => panic!("unexpected {other:?}"),
        }
    }

    #[test]
    fn otp_is_sensitive() {
        let p = classify(
            "otpauth://totp/GitHub:octocat?secret=JBSWY3DPEHPK3PXP&issuer=GitHub",
            None,
        );
        assert_eq!(
            p,
            Payload::Otp {
                issuer: Some("GitHub".into()),
                account: Some("octocat".into())
            }
        );
        assert!(p.is_sensitive());
    }

    #[test]
    fn wifi_and_contacts() {
        let w = classify("WIFI:T:WPA;S:Lab;P:pw;;", None);
        assert_eq!(w.kind(), PayloadKind::Wifi);
        assert!(w.is_sensitive());
        assert_eq!(classify("MECARD:N:Doe,John;;", None).kind(), PayloadKind::Contact);
    }

    #[test]
    fn crypto() {
        match classify("bitcoin:bc1qxyz?amount=0.01", None) {
            Payload::Crypto {
                scheme,
                address,
                amount,
            } => {
                assert_eq!(scheme, "bitcoin");
                assert_eq!(address, "bc1qxyz");
                assert_eq!(amount.as_deref(), Some("0.01"));
            }
            other => panic!("unexpected {other:?}"),
        }
    }

    #[test]
    fn danger_url_keeps_level() {
        match classify("https://google.com@example.com/login", None) {
            Payload::Url(r) => assert_eq!(r.level, Severity::Danger),
            other => panic!("unexpected {other:?}"),
        }
    }

    #[test]
    fn einvoice_detection() {
        // Too short to contain the 77-character fixed part.
        assert_ne!(
            classify("AB12345678:**********:1:1:1:茶:1:30", None).kind(),
            PayloadKind::EInvoice
        );
        let good = format!(
            "AB1234567811510051234000000640000006900000000{}{}:**********:1:1:1:茶:1:30",
            "12345678", "ABCDEFGHIJKLMNOPQRSTUVWX"
        );
        assert_eq!(classify(&good, None).kind(), PayloadKind::EInvoice);
        assert_eq!(
            classify("**咖啡:1:45", None).kind(),
            PayloadKind::EInvoiceContinuation
        );
    }
}
