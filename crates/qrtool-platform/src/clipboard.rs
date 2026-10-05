//! Clipboard access.
//!
//! A single long-lived [`arboard::Clipboard`] is kept because on Linux/X11 the clipboard
//! content disappears when the owning object is dropped.

use std::sync::Mutex;

use image::RgbaImage;

static CLIPBOARD: Mutex<Option<arboard::Clipboard>> = Mutex::new(None);

fn with<T>(f: impl FnOnce(&mut arboard::Clipboard) -> Result<T, arboard::Error>) -> Option<T> {
    let mut guard = CLIPBOARD.lock().ok()?;
    if guard.is_none() {
        *guard = arboard::Clipboard::new().ok();
    }
    let cb = guard.as_mut()?;
    match f(cb) {
        Ok(v) => Some(v),
        Err(e) => {
            tracing::debug!("clipboard: {e}");
            None
        }
    }
}

/// Copies text. Returns `true` on success.
pub fn set_text(text: &str) -> bool {
    with(|c| c.set_text(text.to_string())).is_some()
}

/// Reads text from the clipboard.
pub fn get_text() -> Option<String> {
    with(|c| c.get_text())
}

/// Reads an image from the clipboard (e.g. a screenshot copied with Win+Shift+S).
pub fn get_image() -> Option<RgbaImage> {
    let data = with(|c| c.get_image())?;
    RgbaImage::from_raw(data.width as u32, data.height as u32, data.bytes.into_owned())
}

/// Copies an image to the clipboard.
pub fn set_image(img: &RgbaImage) -> bool {
    with(|c| {
        c.set_image(arboard::ImageData {
            width: img.width() as usize,
            height: img.height() as usize,
            bytes: std::borrow::Cow::Borrowed(img.as_raw()),
        })
    })
    .is_some()
}
