//! Light/dark mode and accent colour, following the operating system unless the user
//! picked a theme in Settings.

use qrtool_core::settings::ThemeMode;

/// The brand blue, used when the system has no accent colour (macOS, Linux).
const BRAND_ACCENT: (u8, u8, u8) = (0x1D, 0x4F, 0xFF);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ThemeState {
    pub dark: bool,
    pub accent: (u8, u8, u8),
    pub high_contrast: bool,
}

impl ThemeState {
    pub fn detect(mode: ThemeMode) -> Self {
        let dark = match mode {
            ThemeMode::System => qrtool_platform::theme::system_is_dark(),
            ThemeMode::Light => false,
            ThemeMode::Dark => true,
        };
        let accent = qrtool_platform::theme::accent_color()
            .filter(|&(r, g, b)| usable_accent(r, g, b))
            .unwrap_or(BRAND_ACCENT);
        Self {
            dark,
            accent,
            high_contrast: qrtool_platform::theme::high_contrast(),
        }
    }

    pub fn accent_color(&self) -> slint::Color {
        slint::Color::from_rgb_u8(self.accent.0, self.accent.1, self.accent.2)
    }
}

/// Very light or very dark accents (e.g. white) would make selected pills invisible.
fn usable_accent(r: u8, g: u8, b: u8) -> bool {
    let luma = 0.2126 * f32::from(r) + 0.7152 * f32::from(g) + 0.0722 * f32::from(b);
    (24.0..=232.0).contains(&luma)
}

/// Pushes the theme into a component's `Theme` global (each window has its own copy).
macro_rules! apply_theme {
    ($component:expr, $state:expr) => {{
        #[allow(unused_imports)]
        use slint::ComponentHandle as _;
        let state: $crate::theme::ThemeState = $state;
        let g = $component.global::<$crate::ui::Theme>();
        g.set_dark(state.dark);
        g.set_accent(state.accent_color());
        g.set_high_contrast(state.high_contrast);
    }};
}
pub(crate) use apply_theme;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rejects_white_and_black_accents() {
        assert!(!usable_accent(255, 255, 255));
        assert!(!usable_accent(0, 0, 0));
        assert!(usable_accent(0x1D, 0x4F, 0xFF));
    }
}
