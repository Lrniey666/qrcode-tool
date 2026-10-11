//! User settings, persisted as TOML in the platform configuration directory.
//!
//! Every field has a default (`#[serde(default)]`), so old or partial files keep working
//! and new settings can be added without migrations.

use std::path::Path;

use serde::{Deserialize, Serialize};

/// Theme preference.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ThemeMode {
    #[default]
    System,
    Light,
    Dark,
}

impl ThemeMode {
    pub fn index(self) -> i32 {
        match self {
            ThemeMode::System => 0,
            ThemeMode::Light => 1,
            ThemeMode::Dark => 2,
        }
    }
    pub fn from_index(i: i32) -> Self {
        match i {
            1 => ThemeMode::Light,
            2 => ThemeMode::Dark,
            _ => ThemeMode::System,
        }
    }
}

/// Global hotkeys, stored in a human readable form such as `Ctrl+Alt+Q`.
/// Default "scan the screen" shortcut on every platform. `Ctrl+Alt+Q` stays clear of
/// the Snipping Tool (`Win+Shift+S`) and of Super/Cmd+Shift+Q, which often quits or
/// logs out.
pub const DEFAULT_SCAN_SCREEN: &str = "Ctrl+Alt+Q";
/// Windows default in schema 2. Migrated back when the user never changed it.
#[cfg(windows)]
const PREVIOUS_SCAN_SCREEN: &str = "Super+Shift+Q";
/// Current settings schema; see [`Settings::sanitize`] for migrations.
pub const SCHEMA: u32 = 3;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct Hotkeys {
    pub scan_screen: String,
    /// Freeze the screen and drag a box around one code.
    pub scan_region: String,
    pub scan_camera: String,
    pub toolbar: String,
}

impl Default for Hotkeys {
    fn default() -> Self {
        Self {
            scan_screen: DEFAULT_SCAN_SCREEN.into(),
            scan_region: "Ctrl+Alt+A".into(),
            scan_camera: "Ctrl+Alt+W".into(),
            toolbar: "Ctrl+Alt+E".into(),
        }
    }
}

/// Scanning behaviour.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct ScanSettings {
    /// Open links automatically when they are clean *and* on the trusted list.
    pub auto_open_trusted: bool,
    /// Domains that may be opened automatically (subdomains included).
    pub trusted_domains: Vec<String>,
    /// Copy the first (non-sensitive) result to the clipboard automatically.
    pub auto_copy: bool,
    /// Play a short sound when something was found.
    pub sound: bool,
    /// Also read 1D barcodes, Data Matrix, Aztec, PDF417 … (not just QR codes).
    pub all_formats: bool,
}

impl Default for ScanSettings {
    fn default() -> Self {
        Self {
            auto_open_trusted: true,
            trusted_domains: Vec::new(),
            auto_copy: false,
            sound: false,
            all_formats: true,
        }
    }
}

/// Camera behaviour.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct CameraSettings {
    /// Human-readable name of the preferred camera (`None` = first available).
    pub device: Option<String>,
    /// Mirror the preview horizontally (like a selfie camera).
    pub mirror: bool,
    /// Close the camera window after a successful scan.
    pub close_after_scan: bool,
    /// Seconds a detected code stays listed after it leaves the camera, so links do not
    /// jump around or vanish while the user reaches for them. 0 = only while visible.
    pub hold_seconds: u32,
}

impl Default for CameraSettings {
    fn default() -> Self {
        Self {
            device: None,
            mirror: true,
            close_after_scan: true,
            hold_seconds: 8,
        }
    }
}

/// How the floating toolbar is laid out and where it appears by default.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum ToolbarLayout {
    /// A row of buttons, centred at the top of the screen.
    #[default]
    Horizontal,
    /// A column of buttons at the right edge of the screen.
    VerticalRight,
    /// A column of buttons at the left edge of the screen.
    VerticalLeft,
}

impl ToolbarLayout {
    pub fn index(self) -> i32 {
        match self {
            ToolbarLayout::Horizontal => 0,
            ToolbarLayout::VerticalRight => 1,
            ToolbarLayout::VerticalLeft => 2,
        }
    }
    pub fn from_index(i: i32) -> Self {
        match i {
            1 => ToolbarLayout::VerticalRight,
            2 => ToolbarLayout::VerticalLeft,
            _ => ToolbarLayout::Horizontal,
        }
    }
    pub fn is_vertical(self) -> bool {
        self != ToolbarLayout::Horizontal
    }
}

/// Floating bubble / toolbar placement.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct FloatingSettings {
    /// Show the floating bubble when the app starts (hidden by default).
    pub bubble_at_startup: bool,
    /// Hide the bubble while a full-screen app or game is in the foreground.
    pub hide_in_fullscreen: bool,
    /// Row at the top (default) or column at the right / left edge.
    pub toolbar_layout: ToolbarLayout,
    pub bubble_position: Option<(i32, i32)>,
    pub toolbar_position: Option<(i32, i32)>,
}

impl Default for FloatingSettings {
    fn default() -> Self {
        Self {
            bubble_at_startup: false,
            hide_in_fullscreen: true,
            toolbar_layout: ToolbarLayout::Horizontal,
            bubble_position: None,
            toolbar_position: None,
        }
    }
}

/// Scan history (off by default for privacy).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct HistorySettings {
    pub enabled: bool,
    pub max_entries: usize,
}

impl Default for HistorySettings {
    fn default() -> Self {
        Self {
            enabled: false,
            max_entries: 50,
        }
    }
}

/// Update checks.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct UpdateSettings {
    pub check_automatically: bool,
}

impl Default for UpdateSettings {
    fn default() -> Self {
        Self {
            check_automatically: true,
        }
    }
}

/// All persisted settings.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct Settings {
    /// Schema version for future migrations.
    pub schema: u32,
    /// UI language code (see [`crate::locale::LANGUAGES`]); empty = follow the system.
    pub language: String,
    pub theme: ThemeMode,
    /// Start with the operating system (enabled on first run, can be turned off).
    pub autostart: bool,
    /// Windows: desktop and image right-click menu entries.
    pub shell_menu: bool,
    /// The first-run welcome screen has been completed.
    pub first_run_completed: bool,
    pub hotkeys: Hotkeys,
    pub scan: ScanSettings,
    pub camera: CameraSettings,
    pub floating: FloatingSettings,
    pub history: HistorySettings,
    pub updates: UpdateSettings,
    /// Advanced (file only): UI renderer — "" automatic, "software" or "femtovg" (OpenGL).
    /// The software renderer avoids GPU driver problems.
    pub renderer: String,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            schema: SCHEMA,
            language: String::new(),
            theme: ThemeMode::System,
            autostart: true,
            shell_menu: true,
            first_run_completed: false,
            hotkeys: Hotkeys::default(),
            scan: ScanSettings::default(),
            camera: CameraSettings::default(),
            floating: FloatingSettings::default(),
            history: HistorySettings::default(),
            updates: UpdateSettings::default(),
            renderer: String::new(),
        }
    }
}

/// Errors while saving settings.
#[derive(Debug, thiserror::Error)]
pub enum SettingsError {
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),
    #[error("serialization error: {0}")]
    Serialize(#[from] toml::ser::Error),
}

impl Settings {
    /// Loads settings from `path`. Missing files yield defaults; corrupt files are renamed
    /// to `*.corrupt` (so nothing is silently lost) and defaults are returned.
    pub fn load(path: &Path) -> Self {
        match std::fs::read_to_string(path) {
            Ok(text) => match toml::from_str::<Settings>(&text) {
                Ok(mut s) => {
                    s.sanitize();
                    s
                }
                Err(err) => {
                    tracing::warn!("settings file is invalid ({err}); using defaults");
                    let _ = std::fs::rename(path, path.with_extension("toml.corrupt"));
                    Settings::default()
                }
            },
            Err(_) => Settings::default(),
        }
    }

    /// Atomically writes the settings to `path` (write to a temp file, then rename).
    pub fn save(&self, path: &Path) -> Result<(), SettingsError> {
        if let Some(dir) = path.parent() {
            std::fs::create_dir_all(dir)?;
        }
        let text = toml::to_string_pretty(self)?;
        let tmp = path.with_extension("toml.tmp");
        std::fs::write(&tmp, text)?;
        std::fs::rename(&tmp, path)?;
        Ok(())
    }

    /// Clamps values that could have been edited by hand into a valid range.
    pub fn sanitize(&mut self) {
        // Schema 2 used Win+Shift+Q on Windows. Move that untouched default back to
        // Ctrl+Alt+Q. A shortcut the user chose themselves is left alone.
        if self.schema < 3 {
            #[cfg(windows)]
            if self.hotkeys.scan_screen == PREVIOUS_SCAN_SCREEN {
                self.hotkeys.scan_screen = DEFAULT_SCAN_SCREEN.into();
            }
            self.schema = SCHEMA;
        }
        self.history.max_entries = self.history.max_entries.clamp(1, 1000);
        self.camera.hold_seconds = self.camera.hold_seconds.min(60);
        self.scan.trusted_domains = self
            .scan
            .trusted_domains
            .iter()
            .map(|d| crate::url_safety::normalize_trusted_entry(d))
            .filter(|d| !d.is_empty())
            .collect();
        self.scan.trusted_domains.sort();
        self.scan.trusted_domains.dedup();
        if !self.language.is_empty() && !crate::locale::is_supported(&self.language) {
            self.language.clear();
        }
        if !matches!(self.renderer.as_str(), "" | "software" | "femtovg") {
            self.renderer.clear();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn defaults_match_product_decisions() {
        let s = Settings::default();
        assert!(s.autostart, "autostart is on by default (shown on first run)");
        assert!(s.shell_menu, "the Windows right-click menu is on by default");
        assert!(!s.floating.bubble_at_startup, "bubble hidden by default");
        assert!(!s.history.enabled, "history off by default");
        assert!(!s.scan.auto_copy);
        assert!(!s.scan.sound);
        assert!(s.scan.auto_open_trusted);
        assert!(s.scan.trusted_domains.is_empty());
        assert_eq!(s.hotkeys.scan_screen, "Ctrl+Alt+Q");
        assert_eq!(s.hotkeys.scan_region, "Ctrl+Alt+A");
        assert_eq!(s.floating.toolbar_layout, ToolbarLayout::Horizontal);
    }

    #[test]
    fn toolbar_layout_round_trips() {
        for layout in [
            ToolbarLayout::Horizontal,
            ToolbarLayout::VerticalRight,
            ToolbarLayout::VerticalLeft,
        ] {
            assert_eq!(ToolbarLayout::from_index(layout.index()), layout);
        }
        assert_eq!(ToolbarLayout::from_index(99), ToolbarLayout::Horizontal);
        assert!(!ToolbarLayout::Horizontal.is_vertical());
        assert!(ToolbarLayout::VerticalLeft.is_vertical());

        let s: Settings = toml::from_str("[floating]\ntoolbar_layout = \"vertical-left\"\n").unwrap();
        assert_eq!(s.floating.toolbar_layout, ToolbarLayout::VerticalLeft);
        let text = toml::to_string_pretty(&s).unwrap();
        assert!(text.contains("toolbar_layout = \"vertical-left\""));
        // Files from older versions have no layout and keep the row at the top.
        let old: Settings = toml::from_str("[floating]\nbubble_at_startup = true\n").unwrap();
        assert_eq!(old.floating.toolbar_layout, ToolbarLayout::Horizontal);
    }

    #[test]
    fn roundtrip_and_partial_files() {
        let dir = std::env::temp_dir().join(format!("qrtool-settings-{}", std::process::id()));
        let path = dir.join("settings.toml");
        let mut s = Settings {
            language: "zh-TW".into(),
            ..Default::default()
        };
        s.scan.trusted_domains = vec!["*.NKUST.edu.tw".into(), "nkust.edu.tw".into()];
        s.save(&path).unwrap();
        let loaded = Settings::load(&path);
        assert_eq!(loaded.language, "zh-TW");
        assert_eq!(loaded.scan.trusted_domains, vec!["nkust.edu.tw".to_string()]);

        std::fs::write(&path, "language = \"ja\"\n[scan]\nsound = true\n").unwrap();
        let partial = Settings::load(&path);
        assert_eq!(partial.language, "ja");
        assert!(partial.scan.sound);
        assert!(partial.autostart, "missing fields fall back to defaults");
        assert!(partial.shell_menu, "missing fields fall back to defaults");

        std::fs::write(&path, "this is = = not toml").unwrap();
        let broken = Settings::load(&path);
        assert_eq!(broken, Settings::default());
        assert!(path.with_extension("toml.corrupt").exists());
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn untouched_windows_shortcut_returns_to_ctrl_alt_q() {
        let mut s = Settings {
            schema: 2,
            ..Default::default()
        };
        s.hotkeys.scan_screen = "Super+Shift+Q".into();
        s.sanitize();
        #[cfg(windows)]
        assert_eq!(s.hotkeys.scan_screen, "Ctrl+Alt+Q");
        #[cfg(not(windows))]
        assert_eq!(s.hotkeys.scan_screen, "Super+Shift+Q");
        assert_eq!(s.schema, SCHEMA);
    }

    #[test]
    fn customized_shortcut_is_kept() {
        let mut s = Settings {
            schema: 2,
            ..Default::default()
        };
        s.hotkeys.scan_screen = "Ctrl+Shift+Q".into();
        s.sanitize();
        assert_eq!(s.hotkeys.scan_screen, "Ctrl+Shift+Q");
        assert_eq!(s.schema, SCHEMA);
    }

    #[test]
    fn sanitize_rejects_unknown_language() {
        let mut s = Settings {
            language: "xx-YY".into(),
            ..Default::default()
        };
        s.sanitize();
        assert!(s.language.is_empty());
    }
}
