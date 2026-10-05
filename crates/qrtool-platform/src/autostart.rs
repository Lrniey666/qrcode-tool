//! Start-with-the-OS support.
//!
//! * Windows: `HKCU\Software\Microsoft\Windows\CurrentVersion\Run` (no admin rights)
//! * macOS: Launch Agent
//! * Linux: XDG autostart `.desktop` file
//!
//! The app is started with `--autostart`, which keeps it hidden in the tray.

use auto_launch::{AutoLaunch, AutoLaunchBuilder, LinuxLaunchMode, MacOSLaunchMode, WindowsEnableMode};

fn launcher() -> Option<AutoLaunch> {
    let exe = std::env::current_exe().ok()?;
    let exe = exe.to_string_lossy().to_string();
    AutoLaunchBuilder::new()
        .set_app_name(qrtool_core::app_info::NAME)
        .set_app_path(&exe)
        .set_args(&["--autostart"])
        .set_macos_launch_mode(MacOSLaunchMode::LaunchAgent)
        .set_windows_enable_mode(WindowsEnableMode::CurrentUser)
        .set_linux_launch_mode(LinuxLaunchMode::XdgAutostart)
        .build()
        .ok()
}

/// Enables or disables starting with the OS. Returns the resulting state.
pub fn set_enabled(enabled: bool) -> bool {
    let Some(l) = launcher() else {
        return false;
    };
    let result = if enabled { l.enable() } else { l.disable() };
    if let Err(e) = result {
        tracing::warn!("autostart change failed: {e}");
    }
    is_enabled()
}

/// Whether starting with the OS is currently registered.
pub fn is_enabled() -> bool {
    launcher().and_then(|l| l.is_enabled().ok()).unwrap_or(false)
}
