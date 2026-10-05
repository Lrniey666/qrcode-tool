//! The system tray icon. Left-click scans the whole screen; right-click opens the menu.

use std::rc::Rc;

use crate::app::{dispatch, Action, App};

pub fn init(app: &Rc<App>) {
    let tray = &app.tray;
    // macOS menu bar icons are monochrome templates.
    tray.set_template_icon(cfg!(target_os = "macos"));
    tray.on_action(|id| {
        let action = match id.as_str() {
            "scan-screen" => Action::ScanScreen,
            "scan-region" => Action::ScanRegion,
            "scan-camera" => Action::ScanCamera,
            "scan-clipboard" => Action::ScanClipboard,
            "open-image" => Action::OpenImage,
            "generate" => Action::Generate(None),
            "toolbar" => Action::ToggleToolbar,
            "bubble" => Action::ToggleBubble,
            "history" => Action::History,
            "settings" => Action::Settings,
            "check-updates" => Action::CheckUpdates,
            "quit" => Action::Quit,
            other => {
                tracing::warn!("unknown tray action {other}");
                return;
            }
        };
        dispatch(action);
    });
    app.refresh_status();
}
