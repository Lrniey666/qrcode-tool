//! Platform-specific application directories.
//!
//! * Windows: `%APPDATA%\Lrniey666\qrcode-tool\config` and `%LOCALAPPDATA%\…\data`
//! * macOS: `~/Library/Application Support/io.github.Lrniey666.qrcode-tool`
//! * Linux: `~/.config/qrcode-tool`, `~/.local/share/qrcode-tool`

use std::path::PathBuf;

use directories::ProjectDirs;

fn dirs() -> Option<ProjectDirs> {
    ProjectDirs::from("io.github", "Lrniey666", "qrcode-tool")
}

fn fallback() -> PathBuf {
    std::env::temp_dir().join("qrcode-tool")
}

/// Directory for `settings.toml`.
pub fn config_dir() -> PathBuf {
    dirs()
        .map(|d| d.config_dir().to_path_buf())
        .unwrap_or_else(fallback)
}

/// Directory for history and logs.
pub fn data_dir() -> PathBuf {
    dirs()
        .map(|d| d.data_local_dir().to_path_buf())
        .unwrap_or_else(fallback)
}

/// Path of the settings file.
pub fn settings_file() -> PathBuf {
    config_dir().join("settings.toml")
}

/// Path of the history file.
pub fn history_file() -> PathBuf {
    data_dir().join("history.jsonl")
}

/// Directory for log files.
pub fn log_dir() -> PathBuf {
    data_dir().join("logs")
}
