//! URL safety analysis.
//!
//! QR code content is attacker-controlled and humans cannot read it with their eyes.
//! Before a link is shown or opened we parse it with the WHATWG URL parser (the same
//! algorithm browsers use) and look for common phishing tricks.
//!
//! **Security invariant:** only `http` and `https` URLs can produce a [`UrlReport`];
//! everything else is rejected with [`UrlError::NotWeb`] and must never be handed to the
//! operating system's "open" machinery.

use std::net::IpAddr;

use serde::{Deserialize, Serialize};
use url::{Host, Url};

/// How worrying a finding (or a whole report) is.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub enum Severity {
    /// Nothing noteworthy.
    Ok = 0,
    /// Informational; does not block automatic opening.
    Info = 1,
    /// Shown in yellow; the user must confirm before opening.
    Warning = 2,
    /// Shown in red; the user must explicitly confirm a second time.
    Danger = 3,
}

/// A single observation about a URL.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum Finding {
    /// Plain `http://` — the connection is not encrypted.
    NotEncrypted,
    /// `https://trusted.com@evil.com` — everything before `@` is ignored by browsers.
    UserInfo { real_host: String },
    /// Internationalized domain name. `ascii` is the punycode (`xn--`) form.
    InternationalDomain { unicode: String, ascii: String },
    /// A single label mixes scripts (e.g. Latin + Cyrillic) — classic homograph attack.
    MixedScript { label: String },
    /// The host is a raw IP address instead of a domain name.
    IpAddress { ip: String, private: bool },
    /// A non-default port is used.
    NonStandardPort { port: u16 },
    /// A known URL shortener — the real destination is unknown until it is resolved.
    Shortener { host: String },
    /// The URL is unusually long (often used to hide the real domain off-screen).
    VeryLong { length: usize },
    /// Many percent-encoded bytes in the URL.
    HeavilyEncoded,
    /// The host has an unusually deep subdomain chain (e.g. `login.bank.com.evil.io`).
    DeepSubdomain { labels: usize },
}

impl Finding {
    /// Severity of this finding.
    pub fn severity(&self) -> Severity {
        match self {
            Finding::UserInfo { .. } | Finding::MixedScript { .. } => Severity::Danger,
            Finding::NotEncrypted
            | Finding::InternationalDomain { .. }
            | Finding::IpAddress { .. }
            | Finding::NonStandardPort { .. }
            | Finding::Shortener { .. } => Severity::Warning,
            Finding::VeryLong { .. } | Finding::HeavilyEncoded | Finding::DeepSubdomain { .. } => {
                Severity::Info
            }
        }
    }

    /// Stable identifier used to look up the translated description in the UI.
    pub fn code(&self) -> &'static str {
        match self {
            Finding::NotEncrypted => "not-encrypted",
            Finding::UserInfo { .. } => "user-info",
            Finding::InternationalDomain { .. } => "idn",
            Finding::MixedScript { .. } => "mixed-script",
            Finding::IpAddress { .. } => "ip-address",
            Finding::NonStandardPort { .. } => "port",
            Finding::Shortener { .. } => "shortener",
            Finding::VeryLong { .. } => "very-long",
            Finding::HeavilyEncoded => "encoded",
            Finding::DeepSubdomain { .. } => "deep-subdomain",
        }
    }

    /// The detail argument shown next to the translated description (may be empty).
    pub fn detail(&self) -> String {
        match self {
            Finding::UserInfo { real_host } => real_host.clone(),
            Finding::InternationalDomain { ascii, .. } => ascii.clone(),
            Finding::MixedScript { label } => label.clone(),
            Finding::IpAddress { ip, .. } => ip.clone(),
            Finding::NonStandardPort { port } => port.to_string(),
            Finding::Shortener { host } => host.clone(),
            Finding::VeryLong { length } => length.to_string(),
            Finding::DeepSubdomain { labels } => labels.to_string(),
            Finding::NotEncrypted | Finding::HeavilyEncoded => String::new(),
        }
    }
}

/// Why a string could not be analysed as a web URL.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum UrlError {
    #[error("not a valid URL")]
    Invalid,
    #[error("scheme `{0}` is not a web link")]
    NotWeb(String),
    #[error("URL has no host")]
    NoHost,
}

/// Result of analysing a web URL.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct UrlReport {
    /// The normalised URL (punycode host). This is what gets opened.
    pub url: String,
    /// Host as it will actually be contacted (ASCII / punycode).
    pub host_ascii: String,
    /// Host for display (Unicode if it is an IDN).
    pub host_display: String,
    /// Registrable-ish domain used for trust decisions (host without leading `www.`).
    pub host_key: String,
    /// All findings, most severe first.
    pub findings: Vec<Finding>,
    /// Maximum severity of all findings.
    pub level: Severity,
}

impl UrlReport {
    /// `true` when nothing at `Warning` level or above was found.
    pub fn is_clean(&self) -> bool {
        self.level < Severity::Warning
    }
}

/// Known URL-shortener hosts. The destination of these links is unknown until resolved,
/// and resolving them would leak the scan to a third party, so we only warn.
pub const SHORTENERS: &[&str] = &[
    "bit.ly",
    "bitly.com",
    "j.mp",
    "tinyurl.com",
    "t.co",
    "goo.gl",
    "ow.ly",
    "buff.ly",
    "is.gd",
    "v.gd",
    "x.gd",
    "rb.gy",
    "cutt.ly",
    "shorturl.at",
    "tiny.cc",
    "t.ly",
    "s.id",
    "rebrand.ly",
    "bl.ink",
    "gg.gg",
    "lnkd.in",
    "amzn.to",
    "reurl.cc",
    "ppt.cc",
    "lihi.cc",
    "lihi1.cc",
    "lihi2.cc",
    "lihi3.cc",
    "pse.is",
    "0rz.tw",
    "tinyurl.tw",
    "shorturl.asia",
    "qrco.de",
    "u.to",
    "clck.ru",
    "dwz.cn",
    "url.cn",
    "t.cn",
    "sohu.gg",
];

const LONG_URL_THRESHOLD: usize = 200;
const ENCODED_THRESHOLD: usize = 12;
const DEEP_SUBDOMAIN_LABELS: usize = 5;

/// Analyse `input` as a web URL.
///
/// Returns [`UrlError::NotWeb`] for any scheme other than `http`/`https`.
pub fn analyze_url(input: &str) -> Result<UrlReport, UrlError> {
    let trimmed = input.trim();
    let url = Url::parse(trimmed).map_err(|_| UrlError::Invalid)?;
    let scheme = url.scheme().to_ascii_lowercase();
    if scheme != "http" && scheme != "https" {
        return Err(UrlError::NotWeb(scheme));
    }

    let host = url.host().ok_or(UrlError::NoHost)?;
    let mut findings = Vec::new();

    if scheme == "http" {
        findings.push(Finding::NotEncrypted);
    }

    let (host_ascii, host_display) = match &host {
        Host::Domain(d) => {
            let ascii = d.to_string();
            let (unicode, _errors) = idna::domain_to_unicode(&ascii);
            (ascii, unicode)
        }
        Host::Ipv4(ip) => (ip.to_string(), ip.to_string()),
        Host::Ipv6(ip) => (format!("[{ip}]"), format!("[{ip}]")),
    };

    // user-info trick: https://google.com@evil.com
    if !url.username().is_empty() || url.password().is_some() {
        findings.push(Finding::UserInfo {
            real_host: host_display.clone(),
        });
    }

    match host {
        Host::Ipv4(ip) => findings.push(Finding::IpAddress {
            ip: ip.to_string(),
            private: is_private(IpAddr::V4(ip)),
        }),
        Host::Ipv6(ip) => findings.push(Finding::IpAddress {
            ip: ip.to_string(),
            private: is_private(IpAddr::V6(ip)),
        }),
        Host::Domain(_) => {
            if host_ascii.split('.').any(|l| l.starts_with("xn--")) {
                findings.push(Finding::InternationalDomain {
                    unicode: host_display.clone(),
                    ascii: host_ascii.clone(),
                });
                for label in host_display.split('.') {
                    if is_mixed_script(label) {
                        findings.push(Finding::MixedScript {
                            label: label.to_string(),
                        });
                    }
                }
            }
            let labels = host_ascii.split('.').filter(|l| !l.is_empty()).count();
            if labels >= DEEP_SUBDOMAIN_LABELS {
                findings.push(Finding::DeepSubdomain { labels });
            }
            let bare = strip_www(&host_ascii);
            if SHORTENERS.contains(&bare) {
                findings.push(Finding::Shortener {
                    host: bare.to_string(),
                });
            }
        }
    }

    if let Some(port) = url.port() {
        // `url` already drops default ports, so any port here is non-standard.
        findings.push(Finding::NonStandardPort { port });
    }

    let serialized = url.as_str().to_string();
    if serialized.chars().count() > LONG_URL_THRESHOLD {
        findings.push(Finding::VeryLong {
            length: serialized.chars().count(),
        });
    }
    if serialized.matches('%').count() >= ENCODED_THRESHOLD {
        findings.push(Finding::HeavilyEncoded);
    }

    findings.sort_by_key(|f| std::cmp::Reverse(f.severity()));
    let level = findings
        .iter()
        .map(Finding::severity)
        .max()
        .unwrap_or(Severity::Ok);

    Ok(UrlReport {
        url: serialized,
        host_key: strip_www(&host_ascii).to_string(),
        host_ascii,
        host_display,
        findings,
        level,
    })
}

/// Returns `true` if `host` equals one of the `trusted` domains or is a subdomain of one.
///
/// Matching is done on ASCII (punycode) hosts, case-insensitively. Entries may be written
/// with or without a leading `*.` or `www.`.
pub fn is_trusted(host_ascii: &str, trusted: &[String]) -> bool {
    let host = host_ascii.trim_end_matches('.').to_ascii_lowercase();
    trusted.iter().any(|t| {
        let t = normalize_trusted_entry(t);
        if t.is_empty() {
            return false;
        }
        host == t || host.ends_with(&format!(".{t}"))
    })
}

/// Normalises a user-entered trusted-domain entry (`*.Example.COM/` → `example.com`).
pub fn normalize_trusted_entry(entry: &str) -> String {
    let mut e = entry.trim().to_ascii_lowercase();
    for prefix in ["https://", "http://"] {
        if let Some(rest) = e.strip_prefix(prefix) {
            e = rest.to_string();
        }
    }
    if let Some(idx) = e.find('/') {
        e.truncate(idx);
    }
    let e = e.trim_start_matches("*.").trim_end_matches('.');
    let e = strip_www(e);
    // Convert IDN entries to punycode so they compare with `host_ascii`.
    match idna::domain_to_ascii(e) {
        Ok(ascii) => ascii,
        Err(_) => e.to_string(),
    }
}

/// Decides whether a URL may be opened without asking.
///
/// Requires: the user enabled auto-open, the report is clean (no warning or danger
/// findings) and the host is on the trusted list.
pub fn may_auto_open(report: &UrlReport, auto_open_enabled: bool, trusted: &[String]) -> bool {
    auto_open_enabled && report.is_clean() && is_trusted(&report.host_ascii, trusted)
}

fn strip_www(host: &str) -> &str {
    host.strip_prefix("www.").unwrap_or(host)
}

fn is_private(ip: IpAddr) -> bool {
    match ip {
        IpAddr::V4(v4) => v4.is_private() || v4.is_loopback() || v4.is_link_local(),
        IpAddr::V6(v6) => v6.is_loopback() || (v6.segments()[0] & 0xfe00) == 0xfc00,
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Script {
    Latin,
    Cyrillic,
    Greek,
    Other,
}

fn script_of(c: char) -> Option<Script> {
    match c {
        'a'..='z' | 'A'..='Z' => Some(Script::Latin),
        '\u{00C0}'..='\u{024F}' => Some(Script::Latin),
        '\u{0370}'..='\u{03FF}' => Some(Script::Greek),
        '\u{0400}'..='\u{052F}' => Some(Script::Cyrillic),
        '0'..='9' | '-' => None,
        _ => Some(Script::Other),
    }
}

/// `true` if a label mixes Latin with Cyrillic or Greek letters (homograph attack).
fn is_mixed_script(label: &str) -> bool {
    let mut seen = Vec::new();
    for c in label.chars() {
        if let Some(s) = script_of(c) {
            if !seen.contains(&s) {
                seen.push(s);
            }
        }
    }
    let confusable = [Script::Latin, Script::Cyrillic, Script::Greek];
    seen.iter().filter(|s| confusable.contains(s)).count() > 1
}

#[cfg(test)]
mod tests {
    use super::*;

    fn codes(r: &UrlReport) -> Vec<&'static str> {
        r.findings.iter().map(Finding::code).collect()
    }

    #[test]
    fn clean_https_url() {
        let r = analyze_url("https://www.nkust.edu.tw/p/index.php").unwrap();
        assert_eq!(r.level, Severity::Ok);
        assert!(r.findings.is_empty());
        assert_eq!(r.host_display, "www.nkust.edu.tw");
        assert_eq!(r.host_key, "nkust.edu.tw");
        assert!(r.is_clean());
    }

    #[test]
    fn rejects_non_web_schemes() {
        for s in [
            "file:///C:/Windows/System32/calc.exe",
            "javascript:alert(1)",
            "ms-settings:display",
            "search-ms:query=x",
            "ms-msdt:/id PCWDiagnostic",
            "data:text/html,<script>alert(1)</script>",
            "steam://run/123",
            "vbscript:msgbox",
        ] {
            assert!(
                matches!(analyze_url(s), Err(UrlError::NotWeb(_))),
                "{s} must be rejected"
            );
        }
        assert_eq!(analyze_url("not a url"), Err(UrlError::Invalid));
        assert_eq!(
            analyze_url(r"\\attacker\share\x"),
            Err(UrlError::Invalid),
            "UNC paths are not URLs"
        );
    }

    #[test]
    fn detects_userinfo_trick() {
        let r = analyze_url("https://google.com@example.com/login").unwrap();
        assert_eq!(r.level, Severity::Danger);
        assert_eq!(r.host_ascii, "example.com");
        assert!(codes(&r).contains(&"user-info"));
        assert_eq!(r.findings[0].detail(), "example.com");
    }

    #[test]
    fn detects_homograph() {
        // Cyrillic 'а' (U+0430) in "аpple.com"
        let r = analyze_url("https://\u{0430}pple.com/").unwrap();
        assert!(r.host_ascii.starts_with("xn--"));
        assert!(codes(&r).contains(&"idn"));
        assert!(codes(&r).contains(&"mixed-script"));
        assert_eq!(r.level, Severity::Danger);
    }

    #[test]
    fn single_script_idn_is_warning_only() {
        let r = analyze_url("https://台灣.tw/").unwrap();
        assert!(codes(&r).contains(&"idn"));
        assert!(!codes(&r).contains(&"mixed-script"));
        assert_eq!(r.level, Severity::Warning);
        assert_eq!(r.host_display, "台灣.tw");
    }

    #[test]
    fn http_ip_port_shortener() {
        let r = analyze_url("http://192.168.1.10:8080/x").unwrap();
        let c = codes(&r);
        assert!(c.contains(&"not-encrypted"));
        assert!(c.contains(&"ip-address"));
        assert!(c.contains(&"port"));
        assert_eq!(r.level, Severity::Warning);

        let s = analyze_url("https://bit.ly/abc").unwrap();
        assert!(codes(&s).contains(&"shortener"));
        let s2 = analyze_url("https://reurl.cc/abc").unwrap();
        assert!(codes(&s2).contains(&"shortener"));
    }

    #[test]
    fn default_port_is_not_flagged() {
        let r = analyze_url("https://example.com:443/").unwrap();
        assert!(r.findings.is_empty());
    }

    #[test]
    fn long_and_encoded_are_info() {
        let long = format!("https://example.com/{}", "a".repeat(300));
        let r = analyze_url(&long).unwrap();
        assert_eq!(r.level, Severity::Info);
        assert!(r.is_clean());
        let enc = format!("https://example.com/{}", "%41".repeat(20));
        assert!(codes(&analyze_url(&enc).unwrap()).contains(&"encoded"));
    }

    #[test]
    fn trusted_domain_matching() {
        let trusted = vec!["*.NKUST.edu.tw".to_string(), "https://github.com/".to_string()];
        assert!(is_trusted("oosaf.nkust.edu.tw", &trusted));
        assert!(is_trusted("nkust.edu.tw", &trusted));
        assert!(is_trusted("github.com", &trusted));
        assert!(!is_trusted("evilgithub.com", &trusted));
        assert!(!is_trusted("github.com.evil.io", &trusted));
        assert!(!is_trusted("example.com", &[String::new()]));
    }

    #[test]
    fn auto_open_requires_clean_and_trusted() {
        let trusted = vec!["example.com".to_string()];
        let ok = analyze_url("https://example.com/a").unwrap();
        assert!(may_auto_open(&ok, true, &trusted));
        assert!(!may_auto_open(&ok, false, &trusted));
        let http = analyze_url("http://example.com/a").unwrap();
        assert!(!may_auto_open(&http, true, &trusted), "warnings block auto-open");
        let other = analyze_url("https://other.com/a").unwrap();
        assert!(!may_auto_open(&other, true, &trusted));
    }
}
