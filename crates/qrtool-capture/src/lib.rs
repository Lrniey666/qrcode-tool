//! Multi-monitor screen capture.
//!
//! Every monitor is captured once, in **physical pixels**, together with its position on the
//! virtual desktop and its scale factor. The overlay then shows exactly this frozen image
//! and the user selects on the image itself, so selection coordinates never have to be
//! converted between DPI spaces (the root cause of region offsets in many tools).
//!
//! Captured images live in memory only and are never written to disk.

use image::RgbaImage;

/// One captured monitor.
#[derive(Debug, Clone)]
pub struct MonitorShot {
    /// Stable-ish identifier from the OS.
    pub id: u32,
    /// Display name (for logs only).
    pub name: String,
    /// Position of the monitor's top-left corner on the virtual desktop (physical px).
    pub x: i32,
    pub y: i32,
    /// Size in physical pixels.
    pub width: u32,
    pub height: u32,
    /// OS scale factor (1.0 = 100 %, 1.5 = 150 %).
    pub scale_factor: f32,
    pub is_primary: bool,
    /// The captured pixels (physical resolution).
    pub image: RgbaImage,
}

/// Capture errors.
#[derive(Debug, thiserror::Error)]
pub enum CaptureError {
    #[error("no monitors found")]
    NoMonitors,
    #[error("screen capture failed: {0}")]
    Backend(String),
    #[error("screen capture permission is missing")]
    PermissionDenied,
}

/// Captures every connected monitor.
///
/// Monitors that fail individually are skipped; an error is returned only if none could
/// be captured.
pub fn capture_all() -> Result<Vec<MonitorShot>, CaptureError> {
    let monitors = xcap::Monitor::all().map_err(|e| CaptureError::Backend(e.to_string()))?;
    if monitors.is_empty() {
        return Err(CaptureError::NoMonitors);
    }
    let mut shots = Vec::with_capacity(monitors.len());
    let mut last_err = None;
    for m in monitors {
        match capture_one(&m) {
            Ok(shot) => shots.push(shot),
            Err(e) => {
                tracing::warn!("capturing a monitor failed: {e}");
                last_err = Some(e);
            }
        }
    }
    if shots.is_empty() {
        return Err(last_err.unwrap_or(CaptureError::NoMonitors));
    }
    // A completely black frame on macOS usually means the screen-recording permission
    // has not been granted yet.
    #[cfg(target_os = "macos")]
    if shots.iter().all(|s| is_all_black(&s.image)) {
        return Err(CaptureError::PermissionDenied);
    }
    Ok(shots)
}

fn capture_one(m: &xcap::Monitor) -> Result<MonitorShot, CaptureError> {
    let be = |e: xcap::XCapError| CaptureError::Backend(e.to_string());
    let image = m.capture_image().map_err(be)?;
    let width = image.width();
    let height = image.height();
    Ok(MonitorShot {
        id: m.id().unwrap_or(0),
        name: m.name().unwrap_or_default(),
        x: m.x().unwrap_or(0),
        y: m.y().unwrap_or(0),
        width,
        height,
        scale_factor: m.scale_factor().unwrap_or(1.0).max(0.5),
        is_primary: m.is_primary().unwrap_or(false),
        image,
    })
}

/// Returns `true` if every sampled pixel is black (used to detect missing permissions).
pub fn is_all_black(img: &RgbaImage) -> bool {
    let (w, h) = img.dimensions();
    if w == 0 || h == 0 {
        return true;
    }
    let step_x = (w / 32).max(1);
    let step_y = (h / 32).max(1);
    (0..h).step_by(step_y as usize).all(|y| {
        (0..w).step_by(step_x as usize).all(|x| {
            let p = img.get_pixel(x, y).0;
            p[0] < 3 && p[1] < 3 && p[2] < 3
        })
    })
}

/// Returns the shot that contains the virtual-desktop point `(x, y)`, if any.
pub fn shot_at(shots: &[MonitorShot], x: i32, y: i32) -> Option<&MonitorShot> {
    shots
        .iter()
        .find(|s| x >= s.x && y >= s.y && x < s.x + s.width as i32 && y < s.y + s.height as i32)
}

#[cfg(test)]
mod tests {
    use super::*;
    use image::Rgba;

    fn shot(x: i32, y: i32, w: u32, h: u32) -> MonitorShot {
        MonitorShot {
            id: 0,
            name: String::new(),
            x,
            y,
            width: w,
            height: h,
            scale_factor: 1.0,
            is_primary: false,
            image: RgbaImage::new(1, 1),
        }
    }

    #[test]
    fn black_detection() {
        assert!(is_all_black(&RgbaImage::new(100, 100)));
        let mut img = RgbaImage::new(100, 100);
        img.put_pixel(0, 0, Rgba([255, 255, 255, 255]));
        assert!(!is_all_black(&img));
    }

    #[test]
    fn finds_monitor_with_negative_coordinates() {
        let shots = vec![shot(0, 0, 1920, 1080), shot(-1280, 0, 1280, 1024)];
        assert_eq!(shot_at(&shots, -10, 500).unwrap().x, -1280);
        assert_eq!(shot_at(&shots, 100, 100).unwrap().x, 0);
        assert!(shot_at(&shots, 5000, 5000).is_none());
    }
}
