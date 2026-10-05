//! Parser for Wi-Fi network QR codes (`WIFI:T:WPA;S:MyNet;P:secret;;`).
//!
//! Format reference: the de-facto ZXing "Wi-Fi Network config" format, including the
//! backslash escapes for `\ ; , : "`.

use serde::{Deserialize, Serialize};

/// Security type of a Wi-Fi network.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum WifiSecurity {
    Open,
    Wep,
    Wpa,
    Wpa3,
    Other(String),
}

impl WifiSecurity {
    fn from_code(code: &str) -> Self {
        match code.to_ascii_uppercase().as_str() {
            "" | "NOPASS" | "NONE" => WifiSecurity::Open,
            "WEP" => WifiSecurity::Wep,
            "WPA" | "WPA2" | "WPA/WPA2" => WifiSecurity::Wpa,
            "SAE" | "WPA3" => WifiSecurity::Wpa3,
            other => WifiSecurity::Other(other.to_string()),
        }
    }

    /// Short label for display.
    pub fn label(&self) -> String {
        match self {
            WifiSecurity::Open => "Open".into(),
            WifiSecurity::Wep => "WEP".into(),
            WifiSecurity::Wpa => "WPA/WPA2".into(),
            WifiSecurity::Wpa3 => "WPA3".into(),
            WifiSecurity::Other(s) => s.clone(),
        }
    }
}

/// A Wi-Fi network configuration from a QR code.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct WifiConfig {
    pub ssid: String,
    pub password: Option<String>,
    pub security: WifiSecurity,
    pub hidden: bool,
}

/// Parses a `WIFI:` payload. Returns `None` if it is not one or has no SSID.
pub fn parse_wifi(input: &str) -> Option<WifiConfig> {
    let body = strip_prefix_ci(input.trim(), "WIFI:")?;
    let fields = split_escaped(body, ';');
    let mut ssid = None;
    let mut password = None;
    let mut security = String::new();
    let mut hidden = false;
    for field in fields {
        let Some((key, value)) = split_key(&field) else {
            continue;
        };
        let value = unescape(value);
        match key.to_ascii_uppercase().as_str() {
            "S" => ssid = Some(value),
            "P" => password = Some(value),
            "T" => security = value,
            "H" => hidden = value.eq_ignore_ascii_case("true"),
            _ => {}
        }
    }
    let ssid = ssid.filter(|s| !s.is_empty())?;
    let security = WifiSecurity::from_code(&security);
    let password = match security {
        WifiSecurity::Open => None,
        _ => password.filter(|p| !p.is_empty()),
    };
    Some(WifiConfig {
        ssid,
        password,
        security,
        hidden,
    })
}

fn strip_prefix_ci<'a>(s: &'a str, prefix: &str) -> Option<&'a str> {
    match s.get(..prefix.len()) {
        Some(head) if head.eq_ignore_ascii_case(prefix) => Some(&s[prefix.len()..]),
        _ => None,
    }
}

/// Splits on `sep` while honouring backslash escapes (escapes are kept).
fn split_escaped(s: &str, sep: char) -> Vec<String> {
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
            out.push(std::mem::take(&mut cur));
        } else {
            cur.push(c);
        }
    }
    if !cur.is_empty() {
        out.push(cur);
    }
    out
}

/// Splits `K:value` at the first unescaped `:`.
fn split_key(field: &str) -> Option<(&str, &str)> {
    let idx = field.find(':')?;
    Some((&field[..idx], &field[idx + 1..]))
}

fn unescape(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
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
    // Some generators wrap values in double quotes.
    if out.len() >= 2 && out.starts_with('"') && out.ends_with('"') {
        out = out[1..out.len() - 1].to_string();
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_basic_wpa() {
        let w = parse_wifi("WIFI:T:WPA;S:MyNet;P:secret123;;").unwrap();
        assert_eq!(w.ssid, "MyNet");
        assert_eq!(w.password.as_deref(), Some("secret123"));
        assert_eq!(w.security, WifiSecurity::Wpa);
        assert!(!w.hidden);
    }

    #[test]
    fn handles_escapes_and_order() {
        let w = parse_wifi(r#"wifi:S:Caf\;e \"Net\";P:pa\:ss\\word;T:SAE;H:true;;"#).unwrap();
        assert_eq!(w.ssid, r#"Caf;e "Net""#);
        assert_eq!(w.password.as_deref(), Some(r"pa:ss\word"));
        assert_eq!(w.security, WifiSecurity::Wpa3);
        assert!(w.hidden);
    }

    #[test]
    fn open_network_has_no_password() {
        let w = parse_wifi("WIFI:T:nopass;S:Guest;P:;;").unwrap();
        assert_eq!(w.security, WifiSecurity::Open);
        assert_eq!(w.password, None);
    }

    #[test]
    fn utf8_ssid() {
        let w = parse_wifi("WIFI:T:WPA;S:高科大-訪客;P:12345678;;").unwrap();
        assert_eq!(w.ssid, "高科大-訪客");
    }

    #[test]
    fn rejects_non_wifi() {
        assert!(parse_wifi("https://example.com").is_none());
        assert!(parse_wifi("WIFI:T:WPA;P:x;;").is_none(), "missing SSID");
    }
}
