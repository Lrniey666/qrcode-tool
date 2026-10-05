//! Detects whether a full-screen application (game, video, presentation) is in front,
//! so the floating bubble can get out of the way.

/// `true` while a full-screen app, game or presentation is in the foreground.
pub fn foreground_is_fullscreen() -> bool {
    #[cfg(windows)]
    {
        use windows_sys::Win32::UI::Shell::{
            SHQueryUserNotificationState, QUNS_BUSY, QUNS_PRESENTATION_MODE, QUNS_RUNNING_D3D_FULL_SCREEN,
        };
        let mut state = 0;
        // SAFETY: valid out-pointer.
        let hr = unsafe { SHQueryUserNotificationState(&mut state) };
        hr == 0
            && matches!(
                state,
                QUNS_BUSY | QUNS_RUNNING_D3D_FULL_SCREEN | QUNS_PRESENTATION_MODE
            )
    }
    #[cfg(not(windows))]
    {
        false
    }
}
