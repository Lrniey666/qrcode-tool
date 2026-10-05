//! Webcam capture with background decoding.
//!
//! A [`CameraSession`] runs two threads:
//!
//! * **capture** — pulls frames from the camera as fast as it delivers them and keeps only
//!   the newest one (no queue, so the preview never lags behind reality);
//! * **decode** — repeatedly takes the newest frame and runs a fast decoder pass on it.
//!
//! Dropping the session stops both threads and releases the camera immediately, which also
//! turns off the camera's privacy LED.

use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{Arc, Mutex};
use std::thread::JoinHandle;
use std::time::Duration;

use image::RgbaImage;
use nokhwa::pixel_format::RgbAFormat;
use nokhwa::utils::{
    ApiBackend, CameraFormat, CameraIndex, FrameFormat, RequestedFormat, RequestedFormatType, Resolution,
};
use nokhwa::Camera;
use qrtool_decode::Detection;

/// A camera that can be opened.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CameraDevice {
    /// Backend index.
    pub index: u32,
    /// Human readable name (also stored in the settings to remember the choice).
    pub name: String,
}

/// Camera errors.
#[derive(Debug, Clone, thiserror::Error)]
pub enum CameraError {
    #[error("no camera found")]
    NoCamera,
    #[error("camera access was denied")]
    PermissionDenied,
    #[error("camera error: {0}")]
    Backend(String),
}

/// Requests camera permission where the OS needs it (macOS). Calls `done(granted)`.
/// On other platforms `done(true)` is called immediately.
pub fn request_permission(done: impl Fn(bool) + Send + Sync + 'static) {
    #[cfg(target_os = "macos")]
    {
        if nokhwa::nokhwa_check() {
            done(true);
        } else {
            nokhwa::nokhwa_initialize(done);
        }
    }
    #[cfg(not(target_os = "macos"))]
    done(true);
}

/// Lists available cameras.
pub fn list_cameras() -> Result<Vec<CameraDevice>, CameraError> {
    let infos = nokhwa::query(ApiBackend::Auto).map_err(|e| CameraError::Backend(e.to_string()))?;
    Ok(infos
        .into_iter()
        .enumerate()
        .map(|(i, info)| CameraDevice {
            index: info.index().as_index().unwrap_or(i as u32),
            name: info.human_name(),
        })
        .collect())
}

/// Result of decoding the newest frame.
#[derive(Debug, Clone, Default)]
pub struct LiveResult {
    /// Sequence number of the analysed frame.
    pub seq: u64,
    pub width: u32,
    pub height: u32,
    pub detections: Vec<Detection>,
}

#[derive(Default)]
struct Shared {
    frame: Mutex<Option<(u64, Arc<RgbaImage>)>>,
    result: Mutex<LiveResult>,
    error: Mutex<Option<CameraError>>,
    seq: AtomicU64,
    stop: AtomicBool,
    opened: AtomicBool,
}

/// A running camera with background decoding.
pub struct CameraSession {
    shared: Arc<Shared>,
    threads: Vec<JoinHandle<()>>,
}

impl CameraSession {
    /// Opens camera `index` and starts capturing and decoding.
    pub fn start(index: u32, all_formats: bool) -> Self {
        let shared = Arc::new(Shared::default());

        let s1 = shared.clone();
        let capture = std::thread::Builder::new()
            .name("camera-capture".into())
            .spawn(move || capture_loop(index, &s1))
            .expect("spawn camera thread");

        let s2 = shared.clone();
        let decode = std::thread::Builder::new()
            .name("camera-decode".into())
            .spawn(move || decode_loop(&s2, all_formats))
            .expect("spawn decode thread");

        Self {
            shared,
            threads: vec![capture, decode],
        }
    }

    /// The newest frame and its sequence number.
    pub fn latest_frame(&self) -> Option<(u64, Arc<RgbaImage>)> {
        self.shared.frame.lock().ok()?.clone()
    }

    /// The newest decoding result.
    pub fn latest_result(&self) -> LiveResult {
        self.shared.result.lock().map(|r| r.clone()).unwrap_or_default()
    }

    /// An error that stopped the camera, if any.
    pub fn error(&self) -> Option<CameraError> {
        self.shared.error.lock().ok()?.clone()
    }

    /// `true` once the camera stream is open.
    pub fn is_open(&self) -> bool {
        self.shared.opened.load(Ordering::Relaxed)
    }

    /// Stops the threads and releases the camera.
    pub fn stop(mut self) {
        self.shutdown();
    }

    fn shutdown(&mut self) {
        self.shared.stop.store(true, Ordering::Relaxed);
        for t in self.threads.drain(..) {
            let _ = t.join();
        }
    }
}

impl Drop for CameraSession {
    fn drop(&mut self) {
        self.shutdown();
    }
}

fn open_camera(index: u32) -> Result<Camera, CameraError> {
    let idx = CameraIndex::Index(index);
    // Prefer 1280×720 MJPEG @ 30 fps: sharp enough for QR codes, light on CPU.
    let preferred = RequestedFormat::new::<RgbAFormat>(RequestedFormatType::Closest(CameraFormat::new(
        Resolution::new(1280, 720),
        FrameFormat::MJPEG,
        30,
    )));
    let fallback = RequestedFormat::new::<RgbAFormat>(RequestedFormatType::AbsoluteHighestFrameRate);
    let map = |e: nokhwa::NokhwaError| {
        let msg = e.to_string();
        if msg.to_ascii_lowercase().contains("permission") || msg.contains("denied") {
            CameraError::PermissionDenied
        } else {
            CameraError::Backend(msg)
        }
    };
    match Camera::new(idx.clone(), preferred) {
        Ok(c) => Ok(c),
        Err(_) => Camera::new(idx, fallback).map_err(map),
    }
}

fn capture_loop(index: u32, shared: &Shared) {
    let mut camera = match open_camera(index) {
        Ok(c) => c,
        Err(e) => {
            set_error(shared, e);
            return;
        }
    };
    if let Err(e) = camera.open_stream() {
        set_error(shared, CameraError::Backend(e.to_string()));
        return;
    }
    shared.opened.store(true, Ordering::Relaxed);
    let mut failures = 0u32;
    while !shared.stop.load(Ordering::Relaxed) {
        match camera
            .frame()
            .map_err(|e| e.to_string())
            .and_then(|b| frame_to_rgba(&b))
        {
            Ok(img) => {
                failures = 0;
                let seq = shared.seq.fetch_add(1, Ordering::Relaxed) + 1;
                if let Ok(mut slot) = shared.frame.lock() {
                    *slot = Some((seq, Arc::new(img)));
                }
            }
            Err(e) => {
                failures += 1;
                tracing::debug!("camera frame error: {e}");
                if failures > 50 {
                    set_error(shared, CameraError::Backend(e));
                    break;
                }
                std::thread::sleep(Duration::from_millis(20));
            }
        }
    }
    let _ = camera.stop_stream();
    shared.opened.store(false, Ordering::Relaxed);
}

/// Converts a camera buffer to RGBA. MJPEG is decoded with the pure-Rust JPEG decoder of
/// the `image` crate (so no C JPEG library is needed); raw formats (YUYV, NV12, …) use
/// nokhwa's converters.
fn frame_to_rgba(buffer: &nokhwa::Buffer) -> Result<RgbaImage, String> {
    if buffer.source_frame_format() == FrameFormat::MJPEG {
        image::load_from_memory_with_format(buffer.buffer(), image::ImageFormat::Jpeg)
            .map(|i| i.to_rgba8())
            .map_err(|e| e.to_string())
    } else {
        buffer.decode_image::<RgbAFormat>().map_err(|e| e.to_string())
    }
}

fn decode_loop(shared: &Shared, all_formats: bool) {
    let mut last_seq = 0;
    while !shared.stop.load(Ordering::Relaxed) {
        let frame = shared.frame.lock().ok().and_then(|f| f.clone());
        match frame {
            Some((seq, img)) if seq != last_seq => {
                last_seq = seq;
                let detections = qrtool_decode::scan_frame(&img, all_formats);
                if let Ok(mut r) = shared.result.lock() {
                    *r = LiveResult {
                        seq,
                        width: img.width(),
                        height: img.height(),
                        detections,
                    };
                }
            }
            _ => std::thread::sleep(Duration::from_millis(15)),
        }
    }
}

fn set_error(shared: &Shared, e: CameraError) {
    tracing::warn!("camera stopped: {e}");
    if let Ok(mut slot) = shared.error.lock() {
        *slot = Some(e);
    }
}
