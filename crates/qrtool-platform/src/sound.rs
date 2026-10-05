//! Short "found" sound (off by default). Uses the OS notification sound, so no audio
//! library or bundled sound file is needed.

/// Plays a short system sound without blocking.
pub fn play_found() {
    #[cfg(windows)]
    {
        use windows_sys::Win32::System::Diagnostics::Debug::MessageBeep;
        use windows_sys::Win32::UI::WindowsAndMessaging::MB_OK;
        // SAFETY: plain Win32 call without pointers.
        unsafe {
            MessageBeep(MB_OK);
        }
    }
    #[cfg(target_os = "macos")]
    {
        let _ = std::process::Command::new("afplay")
            .arg("/System/Library/Sounds/Tink.aiff")
            .spawn();
    }
    #[cfg(all(unix, not(target_os = "macos")))]
    {
        // freedesktop sound theme via libcanberra, if installed; silently ignored otherwise.
        let _ = std::process::Command::new("canberra-gtk-play")
            .args(["-i", "complete"])
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null())
            .spawn();
    }
}
