//! Global keyboard shortcuts (RegisterHotKey on Windows, Carbon on macOS, X11 on Linux).
//!
//! Wayland has no global shortcut API that works for every desktop; there the user binds
//! `qrcode-tool scan-screen` in the system settings instead, and the UI explains this.

use global_hotkey::hotkey::HotKey;
use global_hotkey::{GlobalHotKeyEvent, GlobalHotKeyManager, HotKeyState};
use qrtool_core::settings::Hotkeys;

use crate::app::{app, dispatch, Action};

/// Shortcut ids used in the UI.
pub const IDS: [&str; 4] = ["screen", "region", "camera", "toolbar"];

/// Registration state per shortcut: 0 ok, 1 in use / failed, 2 invalid.
pub type States = [i32; IDS.len()];

pub struct HotkeyService {
    manager: Option<GlobalHotKeyManager>,
    registered: Vec<(HotKey, Action)>,
    pub states: States,
}

/// `false` on Wayland sessions, where global shortcuts cannot be grabbed.
pub fn supported() -> bool {
    #[cfg(target_os = "linux")]
    {
        let wayland = std::env::var("XDG_SESSION_TYPE").is_ok_and(|v| v.eq_ignore_ascii_case("wayland"))
            || std::env::var_os("WAYLAND_DISPLAY").is_some();
        let x11 = std::env::var_os("DISPLAY").is_some();
        !wayland && x11
    }
    #[cfg(not(target_os = "linux"))]
    {
        true
    }
}

impl HotkeyService {
    pub fn new() -> Self {
        let manager = if supported() {
            match GlobalHotKeyManager::new() {
                Ok(m) => Some(m),
                Err(e) => {
                    tracing::warn!("global shortcuts unavailable: {e}");
                    None
                }
            }
        } else {
            None
        };
        Self {
            manager,
            registered: Vec::new(),
            states: [0; IDS.len()],
        }
    }

    pub fn is_available(&self) -> bool {
        self.manager.is_some()
    }

    /// Registers the shortcuts from the settings. Returns the per-shortcut states.
    pub fn apply(&mut self, cfg: &Hotkeys) -> States {
        self.unregister_all();
        let Some(manager) = &self.manager else {
            self.states = [0; IDS.len()];
            return self.states;
        };
        let wanted = [
            (&cfg.scan_screen, Action::ScanScreen),
            (&cfg.scan_region, Action::ScanRegion),
            (&cfg.scan_camera, Action::ScanCamera),
            (&cfg.toolbar, Action::ToggleToolbar),
        ];
        let mut states = [0; IDS.len()];
        for (i, (text, action)) in wanted.into_iter().enumerate() {
            if text.trim().is_empty() {
                continue;
            }
            let Ok(hotkey) = text.parse::<HotKey>() else {
                states[i] = 2;
                continue;
            };
            match manager.register(hotkey) {
                Ok(()) => self.registered.push((hotkey, action)),
                Err(e) => {
                    tracing::warn!("cannot register {text}: {e}");
                    states[i] = 1;
                }
            }
        }
        self.states = states;
        states
    }

    pub fn unregister_all(&mut self) {
        if let Some(manager) = &self.manager {
            for (hk, _) in self.registered.drain(..) {
                let _ = manager.unregister(hk);
            }
        }
        self.registered.clear();
    }

    fn action_for(&self, id: u32) -> Option<Action> {
        self.registered
            .iter()
            .find(|(hk, _)| hk.id() == id)
            .map(|(_, a)| a.clone())
    }
}

/// Registers the configured shortcuts and starts listening.
pub fn init() {
    GlobalHotKeyEvent::set_event_handler(Some(|event: GlobalHotKeyEvent| {
        if event.state() != HotKeyState::Pressed {
            return;
        }
        let id = event.id();
        let _ = slint::invoke_from_event_loop(move || {
            let action = app().hotkeys.borrow().action_for(id);
            if let Some(action) = action {
                dispatch(action);
            }
        });
    }));
    reapply(true);
}

/// Registers the shortcuts again after a change. Reports conflicts once if `notify`.
pub fn reapply(notify: bool) {
    let a = app();
    let cfg = a.settings().hotkeys.clone();
    let states = a.hotkeys.borrow_mut().apply(&cfg);
    a.refresh_status();
    if notify {
        let values = [&cfg.scan_screen, &cfg.scan_region, &cfg.scan_camera, &cfg.toolbar];
        if let Some(i) = states.iter().position(|&s| s == 1) {
            crate::message::show_with("hotkey-failed", &display(values[i]), 2, |_| {
                crate::settings_ui::open(Some(3));
            });
        }
    }
}

/// A shortcut as shown to the user: `Super` is called `Win` on Windows and `Cmd` on macOS.
pub fn display(s: &str) -> String {
    let win = if cfg!(target_os = "macos") {
        "Cmd"
    } else if cfg!(windows) {
        "Win"
    } else {
        "Super"
    };
    s.split('+')
        .map(|part| {
            if part.eq_ignore_ascii_case("super") {
                win
            } else {
                part
            }
        })
        .collect::<Vec<_>>()
        .join("+")
}

/// Temporarily releases all shortcuts (while the user records a new one).
pub fn suspend() {
    app().hotkeys.borrow_mut().unregister_all();
}

/// Turns a key press recorded in Settings into a shortcut string such as `Ctrl+Alt+Q`.
/// Returns `None` if the combination cannot be used (no modifier, or a modifier alone).
pub fn format_recorded(ctrl: bool, alt: bool, shift: bool, meta: bool, text: &str) -> Option<String> {
    let key = key_name(text)?;
    let is_function_key =
        key.starts_with('F') && key.len() > 1 && key[1..].chars().all(|c| c.is_ascii_digit());
    if !(ctrl || alt || meta || is_function_key) {
        // Shift alone is not enough: it would steal ordinary typing.
        return None;
    }
    let mut parts = Vec::new();
    if ctrl {
        parts.push("Ctrl");
    }
    if alt {
        parts.push(if cfg!(target_os = "macos") {
            "Option"
        } else {
            "Alt"
        });
    }
    if shift {
        parts.push("Shift");
    }
    if meta {
        // Stored as "Super" (what global-hotkey parses); shown as Win/Cmd via `display`.
        parts.push("Super");
    }
    let mut s = parts.join("+");
    if !s.is_empty() {
        s.push('+');
    }
    s.push_str(&key);
    // Make sure global-hotkey understands it.
    s.parse::<HotKey>().ok().map(|_| s)
}

/// Maps Slint key text to a key name understood by global-hotkey.
fn key_name(text: &str) -> Option<String> {
    let mut chars = text.chars();
    let c = chars.next()?;
    if chars.next().is_some() {
        return None;
    }
    let name = match c {
        'a'..='z' | 'A'..='Z' => c.to_ascii_uppercase().to_string(),
        '0'..='9' => c.to_string(),
        ' ' => "Space".into(),
        '\u{0008}' => "Backspace".into(),
        '\u{0009}' => "Tab".into(),
        '\u{000a}' | '\u{000d}' => "Enter".into(),
        '\u{007f}' => "Delete".into(),
        '\u{F700}' => "Up".into(),
        '\u{F701}' => "Down".into(),
        '\u{F702}' => "Left".into(),
        '\u{F703}' => "Right".into(),
        '\u{F704}'..='\u{F71B}' => format!("F{}", c as u32 - 0xF704 + 1),
        '\u{F727}' => "Insert".into(),
        '\u{F729}' => "Home".into(),
        '\u{F72B}' => "End".into(),
        '\u{F72C}' => "PageUp".into(),
        '\u{F72D}' => "PageDown".into(),
        '\u{F730}' => "Pause".into(),
        '\u{F731}' => "PrintScreen".into(),
        '`' | '~' => "Backquote".into(),
        '-' | '_' => "Minus".into(),
        '=' | '+' => "Equal".into(),
        '[' | '{' => "BracketLeft".into(),
        ']' | '}' => "BracketRight".into(),
        '\\' | '|' => "Backslash".into(),
        ';' | ':' => "Semicolon".into(),
        '\'' | '"' => "Quote".into(),
        ',' | '<' => "Comma".into(),
        '.' | '>' => "Period".into(),
        '/' | '?' => "Slash".into(),
        // Shifted digits on US layouts.
        '!' => "1".into(),
        '@' => "2".into(),
        '#' => "3".into(),
        '$' => "4".into(),
        '%' => "5".into(),
        '^' => "6".into(),
        '&' => "7".into(),
        '*' => "8".into(),
        '(' => "9".into(),
        ')' => "0".into(),
        _ => return None,
    };
    Some(name)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_hotkeys_parse() {
        let d = Hotkeys::default();
        for s in [&d.scan_screen, &d.scan_region, &d.scan_camera, &d.toolbar] {
            assert!(s.parse::<HotKey>().is_ok(), "{s}");
        }
    }

    #[test]
    fn recorded_combinations() {
        assert_eq!(
            format_recorded(true, true, false, false, "q").as_deref(),
            Some("Ctrl+Alt+Q")
        );
        assert_eq!(format_recorded(false, false, true, false, "q"), None);
        assert_eq!(
            format_recorded(false, false, false, false, "\u{F70B}").as_deref(),
            Some("F8")
        );
        assert_eq!(
            format_recorded(true, false, true, false, "!").as_deref(),
            Some("Ctrl+Shift+1")
        );
        assert_eq!(format_recorded(true, false, false, false, "\u{0011}"), None);
    }
}
