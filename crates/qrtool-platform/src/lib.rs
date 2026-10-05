//! Operating-system integration for **QRcode tool**.
//!
//! Everything that talks to the OS lives here, behind small functions with no UI
//! dependencies, so the app crate stays portable and this crate stays reviewable.

pub mod autostart;
pub mod clipboard;
pub mod fullscreen;
pub mod ipc;
pub mod open;
pub mod paths;
pub mod shell_menu;
pub mod sound;
pub mod theme;
pub mod wifi;

/// The system locale as reported by the OS (e.g. `zh-TW`).
pub fn system_locale() -> Option<String> {
    sys_locale::get_locale()
}
