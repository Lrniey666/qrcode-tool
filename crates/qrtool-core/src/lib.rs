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
}
