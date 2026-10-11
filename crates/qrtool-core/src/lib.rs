//! Platform-independent core logic for **QRcode tool**.
//!
//! Nothing in this crate touches the screen, camera, network or UI. Everything here is
//! deterministic and unit-tested, which is why the security-relevant rules (what may be
//! opened, what may be stored) live here.
//!
//! * [`payload`] — classify decoded text (URL, Wi-Fi, contact, OTP secret, e-invoice, …)
//! * [`url_safety`] — analyse URLs for phishing tricks before anything is opened
//! * [`einvoice`] — Taiwan electronic invoice (電子發票) QR code parser
//! * [`settings`] — user settings persisted as TOML
//! * [`history`] — optional, privacy-aware scan history (JSON Lines)
//! * [`locale`] — map system locales to the supported UI languages
//! * [`stabilizer`] — debounce for live camera scanning

pub mod contact;
pub mod einvoice;
pub mod history;
pub mod locale;
pub mod payload;
pub mod settings;
pub mod stabilizer;
pub mod text;
pub mod url_safety;
pub mod wifi;

pub use payload::{classify, Payload, PayloadKind};
pub use url_safety::{analyze_url, Finding, Severity, UrlReport};

/// Application identifiers shared by all crates.
pub mod app_info {
    /// Human readable product name.
    pub const NAME: &str = "QRcode tool";
    /// Binary / package name.
    pub const BIN_NAME: &str = "qrcode-tool";
    /// Reverse-DNS application id (used for config dirs, IPC names, installers).
    pub const APP_ID: &str = "io.github.lrniey666.qrcode-tool";
    /// GitHub repository used for update checks and links.
    pub const REPO_URL: &str = "https://github.com/Lrniey666/qrcode-tool";
    /// Trademark notice that must accompany the words "QR Code".
    pub const QR_TRADEMARK_NOTICE: &str = "QR Code is a registered trademark of DENSO WAVE INCORPORATED.";
    /// Crate version.
    pub const VERSION: &str = env!("CARGO_PKG_VERSION");

    /// The PDF user manual in the repository for a UI language code ("zh-TW", "en", …).
    /// Chinese UI languages get the Traditional Chinese manual, every other one English.
    pub fn manual_url(language: &str) -> String {
        let file = if language.starts_with("zh") { "zh-TW" } else { "en" };
        format!("{REPO_URL}/blob/main/docs/manual/QRcode-tool-User-Manual-{file}.pdf")
    }

    #[cfg(test)]
    mod tests {
        use super::*;

        #[test]
        fn manual_follows_the_ui_language() {
            assert_eq!(
                manual_url("zh-TW"),
                "https://github.com/Lrniey666/qrcode-tool/blob/main/docs/manual/QRcode-tool-User-Manual-zh-TW.pdf"
            );
            assert!(manual_url("zh-CN").ends_with("-zh-TW.pdf"));
            assert!(manual_url("en").ends_with("-en.pdf"));
            assert!(manual_url("ja").ends_with("-en.pdf"));
            assert!(crate::url_safety::analyze_url(&manual_url("de")).is_ok());
        }
    }
}
