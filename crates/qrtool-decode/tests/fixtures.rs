//! End-to-end decoding of the images in `tests/fixtures/`, including classification.

use std::path::PathBuf;

use qrtool_core::payload::{classify, Payload};
use qrtool_core::url_safety::Severity;
use qrtool_decode::{scan_image, DecodeOptions};

fn decode(name: &str) -> Vec<String> {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../tests/fixtures")
        .join(name);
    let img = image::open(&path)
        .unwrap_or_else(|e| panic!("{}: {e}", path.display()))
        .to_rgba8();
    scan_image(&img, DecodeOptions::default())
        .into_iter()
        .map(|d| d.text)
        .collect()
}

#[test]
fn url() {
    assert_eq!(decode("url.png"), ["https://github.com/Lrniey666/qrcode-tool"]);
}

#[test]
fn wifi_is_classified() {
    let texts = decode("wifi.png");
    assert_eq!(texts.len(), 1);
    match classify(&texts[0], None) {
        Payload::Wifi(w) => {
            assert_eq!(w.ssid, "Cafe Guest");
            assert_eq!(w.password.as_deref(), Some("hunter22"));
        }
        other => panic!("not Wi-Fi: {other:?}"),
    }
}

#[test]
fn vcard_is_classified() {
    let texts = decode("vcard.png");
    assert!(matches!(classify(&texts[0], None), Payload::Contact(c) if c.name == "Example Person"));
}

#[test]
fn disguised_link_is_dangerous() {
    let texts = decode("disguised-link.png");
    match classify(&texts[0], None) {
        Payload::Url(r) => {
            assert_eq!(r.level, Severity::Danger);
            assert_eq!(r.host_ascii, "evil.example");
        }
        other => panic!("not a URL: {other:?}"),
    }
}

#[test]
fn inverted_and_small_codes() {
    assert_eq!(decode("inverted.png"), ["https://example.com/inverted"]);
    assert_eq!(decode("small.png"), ["https://example.com/small"]);
}

#[test]
fn demo_screen_has_three_codes() {
    let mut texts = decode("demo-screen.png");
    texts.sort();
    assert_eq!(
        texts,
        [
            "WIFI:T:WPA;S:Cafe Guest;P:hunter22;;",
            "http://bit.ly/abc123",
            "https://github.com/Lrniey666/qrcode-tool",
        ]
    );
}
