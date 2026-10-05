//! Opening links and URIs — the single place where content leaves the app.
//!
//! **Security invariants**
//! * A web link is opened only from a [`UrlReport`], which can only exist for `http`/`https`.
//! * The URL is re-parsed and the scheme re-checked right before opening (defence in depth).
//! * Other actions (`mailto:`, `tel:`, `sms:`) are rebuilt from parsed fields, never passed
//!   through from the raw QR text.
//! * Nothing from a QR code is ever passed to a shell.

use qrtool_core::url_safety::UrlReport;

/// Errors when opening something.
#[derive(Debug, thiserror::Error)]
pub enum OpenError {
    #[error("refusing to open a non-web URL")]
    NotWeb,
    #[error("could not open: {0}")]
    Io(#[from] std::io::Error),
}

/// Opens a web URL in the user's default browser.
pub fn open_web(report: &UrlReport) -> Result<(), OpenError> {
    let parsed = url::Url::parse(&report.url).map_err(|_| OpenError::NotWeb)?;
    if !matches!(parsed.scheme(), "http" | "https") {
        return Err(OpenError::NotWeb);
    }
    tracing::info!(host = %report.host_ascii, "opening link");
    webbrowser::open(parsed.as_str()).map_err(OpenError::Io)
}

/// Opens an e-mail draft in the default mail client.
pub fn open_mail(to: &str, subject: Option<&str>, body: Option<&str>) -> Result<(), OpenError> {
    let mut url = url::Url::parse("mailto:").map_err(|_| OpenError::NotWeb)?;
    url.set_path(to);
    {
        let mut q = url.query_pairs_mut();
        if let Some(s) = subject {
            q.append_pair("subject", s);
        }
        if let Some(b) = body {
            q.append_pair("body", b);
        }
    }
    let s = url.as_str().trim_end_matches('?').to_string();
    open::that_detached(s).map_err(OpenError::Io)
}

/// Opens the system dialer / phone app (`tel:`).
pub fn open_phone(number: &str) -> Result<(), OpenError> {
    let clean: String = number
        .chars()
        .filter(|c| c.is_ascii_digit() || matches!(c, '+' | '*' | '#'))
        .collect();
    if clean.is_empty() {
        return Err(OpenError::NotWeb);
    }
    open::that_detached(format!("tel:{clean}")).map_err(OpenError::Io)
}

/// Opens the system messaging app (`sms:`).
pub fn open_sms(number: &str, body: Option<&str>) -> Result<(), OpenError> {
    let clean: String = number
        .chars()
        .filter(|c| c.is_ascii_digit() || *c == '+')
        .collect();
    let mut s = format!("sms:{clean}");
    if let Some(b) = body {
        let mut tmp = url::Url::parse("sms:x").map_err(|_| OpenError::NotWeb)?;
        tmp.query_pairs_mut().append_pair("body", b);
        if let Some(q) = tmp.query() {
            s.push('?');
            s.push_str(q);
        }
    }
    open::that_detached(s).map_err(OpenError::Io)
}

/// Opens a map at the given coordinates (OpenStreetMap, in the browser).
pub fn open_map(lat: f64, lon: f64) -> Result<(), OpenError> {
    let url = format!("https://www.openstreetmap.org/?mlat={lat:.6}&mlon={lon:.6}#map=17/{lat:.6}/{lon:.6}");
    webbrowser::open(&url).map_err(OpenError::Io)
}

/// Opens the OS privacy settings page for screen recording (macOS) so the user can grant
/// permission. No-op elsewhere.
pub fn open_screen_recording_settings() {
    #[cfg(target_os = "macos")]
    {
        let _ = open::that_detached(
            "x-apple.systempreferences:com.apple.preference.security?Privacy_ScreenCapture",
        );
    }
}

/// Opens the OS privacy settings page for the camera.
pub fn open_camera_settings() {
    #[cfg(target_os = "macos")]
    {
        let _ = open::that_detached("x-apple.systempreferences:com.apple.preference.security?Privacy_Camera");
    }
    #[cfg(windows)]
    {
        let _ = open::that_detached("ms-settings:privacy-webcam");
    }
}

/// Opens a folder or file with the system file manager (used for "show logs").
pub fn reveal(path: &std::path::Path) -> Result<(), OpenError> {
    open::that_detached(path).map_err(OpenError::Io)
}
