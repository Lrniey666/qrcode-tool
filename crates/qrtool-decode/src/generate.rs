//! QR code generation (PC → phone): render text as a QR code bitmap or SVG.

use image::{Rgba, RgbaImage};
use qrcode::{Color, QrCode};

/// Error correction level.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum EcLevel {
    /// ~7% recovery.
    L,
    /// ~15% recovery (default).
    #[default]
    M,
    /// ~25% recovery.
    Q,
    /// ~30% recovery.
    H,
}

impl EcLevel {
    pub fn from_index(i: i32) -> Self {
        match i {
            0 => EcLevel::L,
            2 => EcLevel::Q,
            3 => EcLevel::H,
            _ => EcLevel::M,
        }
    }
    fn to_qrcode(self) -> qrcode::EcLevel {
        match self {
            EcLevel::L => qrcode::EcLevel::L,
            EcLevel::M => qrcode::EcLevel::M,
            EcLevel::Q => qrcode::EcLevel::Q,
            EcLevel::H => qrcode::EcLevel::H,
        }
    }
}

/// A generated QR code as a square matrix of modules.
#[derive(Debug, Clone)]
pub struct QrMatrix {
    /// Modules per side (without quiet zone).
    pub size: u32,
    modules: Vec<bool>,
    text: String,
    level: EcLevel,
}

/// Generation errors.
#[derive(Debug, thiserror::Error)]
pub enum GenerateError {
    #[error("the text is empty")]
    Empty,
    #[error("the text is too long for a QR code")]
    TooLong,
}

/// Encodes `text` as a QR code.
pub fn generate_qr(text: &str, level: EcLevel) -> Result<QrMatrix, GenerateError> {
    if text.is_empty() {
        return Err(GenerateError::Empty);
    }
    let code = QrCode::with_error_correction_level(text.as_bytes(), level.to_qrcode())
        .map_err(|_| GenerateError::TooLong)?;
    let size = code.width() as u32;
    let modules = code.to_colors().into_iter().map(|c| c == Color::Dark).collect();
    Ok(QrMatrix {
        size,
        modules,
        text: text.to_string(),
        level,
    })
}

impl QrMatrix {
    /// `true` if module `(x, y)` is dark.
    pub fn is_dark(&self, x: u32, y: u32) -> bool {
        self.modules[(y * self.size + x) as usize]
    }

    /// Renders a bitmap: `scale` pixels per module and a 4-module quiet zone.
    pub fn to_image(&self, scale: u32, dark: [u8; 4], light: [u8; 4]) -> RgbaImage {
        let scale = scale.max(1);
        let border = 4;
        let px = (self.size + 2 * border) * scale;
        RgbaImage::from_fn(px, px, |x, y| {
            let mx = (x / scale) as i64 - border as i64;
            let my = (y / scale) as i64 - border as i64;
            let dark_here = mx >= 0
                && my >= 0
                && (mx as u32) < self.size
                && (my as u32) < self.size
                && self.is_dark(mx as u32, my as u32);
            Rgba(if dark_here { dark } else { light })
        })
    }

    /// Renders an SVG document.
    pub fn to_svg(&self) -> String {
        // Re-encode with the same parameters to use qrcode's SVG renderer.
        let code = QrCode::with_error_correction_level(self.text.as_bytes(), self.level.to_qrcode())
            .expect("already validated");
        code.render::<qrcode::render::svg::Color>()
            .min_dimensions(256, 256)
            .quiet_zone(true)
            .build()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn generates_and_renders() {
        let m = generate_qr("https://example.com", EcLevel::M).unwrap();
        assert!(m.size >= 21);
        let img = m.to_image(4, [0, 0, 0, 255], [255, 255, 255, 255]);
        assert_eq!(img.width(), (m.size + 8) * 4);
        // top-left finder pattern corner is dark
        assert_eq!(img.get_pixel(16, 16).0, [0, 0, 0, 255]);
        // quiet zone is light
        assert_eq!(img.get_pixel(0, 0).0, [255, 255, 255, 255]);
        assert!(m.to_svg().starts_with("<?xml"));
    }

    #[test]
    fn rejects_empty_and_too_long() {
        assert!(matches!(generate_qr("", EcLevel::M), Err(GenerateError::Empty)));
        let long = "x".repeat(8000);
        assert!(matches!(
            generate_qr(&long, EcLevel::H),
            Err(GenerateError::TooLong)
        ));
    }

    #[test]
    fn generated_code_roundtrips_through_decoder() {
        let m = generate_qr("繁體中文 roundtrip", EcLevel::Q).unwrap();
        let img = m.to_image(5, [0, 0, 0, 255], [255, 255, 255, 255]);
        let found = crate::scan_image(&img, crate::DecodeOptions::default());
        assert_eq!(found[0].text, "繁體中文 roundtrip");
    }
}
