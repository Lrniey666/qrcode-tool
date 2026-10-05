//! System theme (dark/light) and accent colour.

/// `true` if the OS is in dark mode.
pub fn system_is_dark() -> bool {
    matches!(dark_light::detect(), Ok(dark_light::Mode::Dark))
}

/// The system accent colour as `(r, g, b)`, when the OS exposes one.
pub fn accent_color() -> Option<(u8, u8, u8)> {
    #[cfg(windows)]
    {
        use winreg::enums::HKEY_CURRENT_USER;
        use winreg::RegKey;
        let key = RegKey::predef(HKEY_CURRENT_USER)
            .open_subkey(r"Software\Microsoft\Windows\DWM")
            .ok()?;
        // AccentColor is stored as 0xAABBGGRR.
        let v: u32 = key.get_value("AccentColor").ok()?;
        let r = (v & 0xFF) as u8;
        let g = ((v >> 8) & 0xFF) as u8;
        let b = ((v >> 16) & 0xFF) as u8;
        Some((r, g, b))
    }
    #[cfg(not(windows))]
    {
        None
    }
}

/// `true` if Windows high-contrast mode is active.
pub fn high_contrast() -> bool {
    #[cfg(windows)]
    {
        use windows_sys::Win32::UI::Accessibility::{HCF_HIGHCONTRASTON, HIGHCONTRASTW};
        use windows_sys::Win32::UI::WindowsAndMessaging::{SystemParametersInfoW, SPI_GETHIGHCONTRAST};
        let mut hc = HIGHCONTRASTW {
            cbSize: std::mem::size_of::<HIGHCONTRASTW>() as u32,
            dwFlags: 0,
            lpszDefaultScheme: std::ptr::null_mut(),
        };
        // SAFETY: valid pointer to a correctly sized struct.
        let ok =
            unsafe { SystemParametersInfoW(SPI_GETHIGHCONTRAST, hc.cbSize, &mut hc as *mut _ as *mut _, 0) };
        ok != 0 && (hc.dwFlags & HCF_HIGHCONTRASTON) != 0
    }
    #[cfg(not(windows))]
    {
        false
    }
}

/// `false` when the user turned animations off (Windows: "Animation effects" in
/// Settings › Accessibility › Visual effects). Other systems: `true`.
pub fn animations_enabled() -> bool {
    #[cfg(windows)]
    {
        use windows_sys::Win32::UI::WindowsAndMessaging::{
            SystemParametersInfoW, SPI_GETCLIENTAREAANIMATION,
        };
        let mut on: i32 = 1;
        // SAFETY: valid out-pointer to a BOOL.
        let ok =
            unsafe { SystemParametersInfoW(SPI_GETCLIENTAREAANIMATION, 0, (&mut on as *mut i32).cast(), 0) };
        ok == 0 || on != 0
    }
    #[cfg(not(windows))]
    {
        true
    }
}
