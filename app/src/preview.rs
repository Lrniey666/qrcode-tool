//! The preview window: live webcam scanning, and scanning image files or the clipboard.
#![cfg_attr(not(feature = "camera"), allow(dead_code, unused_imports))]

use std::sync::Arc;
use std::time::{Duration, Instant};

use image::RgbaImage;
use qrtool_core::history::ScanSource;
use qrtool_core::stabilizer::Stabilizer;
use qrtool_decode::{scan_image, DecodeOptions};
use slint::{ComponentHandle, Model, ModelRc, SharedPixelBuffer, VecModel};

use crate::app::app;
use crate::detail::{self, Outcome};
use crate::scan::{self, Primary, ScanItem};
use crate::theme::apply_theme;
use crate::ui;

/// `PreviewWindow.status` values.
mod status {
    pub const STARTING: i32 = 0;
    pub const SCANNING: i32 = 1;
    pub const FOUND: i32 = 2;
    pub const NOTHING: i32 = 3;
    pub const ERROR: i32 = 4;
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Mode {
    Camera,
    Image(ScanSource),
}

pub struct Preview {
    pub ui: ui::PreviewWindow,
    mode: Mode,
    items: Vec<ScanItem>,
    selected: Option<usize>,
    detail: Option<(usize, bool)>,
    frame_w: u32,
    #[cfg(feature = "camera")]
    camera: Option<qrtool_camera::CameraSession>,
    timer: slint::Timer,
    last_frame: u64,
    last_result: u64,
    stabilizer: Stabilizer,
    mirror: bool,
    /// Camera: recently seen codes (see `hold`); `items` mirrors their order.
    held: Vec<crate::hold::Held>,
}

impl Preview {
    fn new(ui: ui::PreviewWindow, mode: Mode) -> Self {
        Self {
            ui,
            mode,
            items: Vec::new(),
            selected: None,
            detail: None,
            frame_w: 1,
            #[cfg(feature = "camera")]
            camera: None,
            timer: slint::Timer::default(),
            last_frame: 0,
            last_result: 0,
            stabilizer: Stabilizer::new(3, Duration::from_secs(4)),
            mirror: false,
            held: Vec::new(),
        }
    }

    fn refresh_pills(&self) {
        let s = self.ui.global::<ui::Strings>();
        let mirror = (self.mode == Mode::Camera && self.mirror).then_some(self.frame_w as f32);
        let mut pills = scan::pills_for(&self.items, 0, self.selected, &s, mirror);
        if self.mode == Mode::Camera {
            for (p, h) in pills.iter_mut().zip(&self.held) {
                p.live = h.live;
            }
        }
        if let Some(model) = scan::update_pills(self.ui.get_pills(), pills) {
            self.ui.set_pills(model);
        }
    }

    fn show_detail(&mut self, index: usize, confirm: bool) {
        let Some(item) = self.items.get(index) else {
            return;
        };
        let trusted = app().settings().scan.trusted_domains.clone();
        let data = detail::build(item, confirm, &trusted, &self.ui.global::<ui::Strings>());
        self.ui.set_detail(data);
        self.ui.set_detail_visible(true);
        self.detail = Some((index, confirm));
        self.selected = Some(index);
        self.refresh_pills();
    }

    fn hide_detail(&mut self) {
        self.ui.set_detail_visible(false);
        self.detail = None;
    }

    fn stop_camera(&mut self) {
        self.timer.stop();
        #[cfg(feature = "camera")]
        if let Some(cam) = self.camera.take() {
            // Joining the capture thread can take a frame; do it off the UI thread.
            std::thread::spawn(move || cam.stop());
        }
    }

    /// Closes the window without side effects (used when quitting).
    pub fn close_silently(mut self) {
        self.stop_camera();
        let _ = self.ui.hide();
    }
}

fn with_preview<R>(f: impl FnOnce(&mut Preview) -> R) -> Option<R> {
    let a = app();
    let mut guard = a.preview.borrow_mut();
    guard.as_mut().map(f)
}

/// Returns the open preview window switched to `mode`, creating it if needed.
fn ensure_window(mode: Mode) -> bool {
    let a = app();
    if let Some(p) = a.preview.borrow_mut().as_mut() {
        p.stop_camera();
        p.mode = mode;
        p.items.clear();
        p.selected = None;
        p.hide_detail();
        p.refresh_pills();
        p.ui.set_camera_mode(mode == Mode::Camera);
        p.ui.set_status(status::STARTING);
        let _ = p.ui.show();
        return true;
    }
    let Ok(w) = ui::PreviewWindow::new() else {
        tracing::error!("cannot create preview window");
        return false;
    };
    apply_theme!(w, a.theme.get());
    w.set_camera_mode(mode == Mode::Camera);
    w.set_status(status::STARTING);
    wire(&w);
    if let Err(e) = w.show() {
        tracing::error!("cannot show preview: {e}");
        return false;
    }
    crate::winutil::center_when_created(&w);
    *a.preview.borrow_mut() = Some(Preview::new(w, mode));
    true
}

fn wire(w: &ui::PreviewWindow) {
    w.on_camera_selected(|i| {
        let a = app();
        let name = a.cameras.borrow().names.get(i as usize).cloned();
        a.update_settings(|s| s.camera.device = name);
        start_camera_session(i as usize);
    });
    w.on_mirror_toggled(|on| {
        app().update_settings(|s| s.camera.mirror = on);
        with_preview(|p| {
            p.mirror = on;
            p.last_frame = 0;
            p.refresh_pills();
        });
    });
    w.on_pill_clicked(|i| activate(i as usize));
    w.on_pill_details(|i| {
        with_preview(|p| p.show_detail(i as usize, false));
    });
    w.on_open_image(pick_image);
    w.on_scan_clipboard(scan_clipboard);
    w.on_key(|k| key(k.as_str()));
    w.on_detail_action(|id| detail_action(id.as_str()));
    w.on_detail_copy_field(|idx| {
        with_preview(|p| {
            if let Some(f) = p.ui.get_detail().fields.row_data(idx as usize) {
                qrtool_platform::clipboard::set_text(f.value.as_str());
            }
        });
    });
    w.on_detail_close(|| {
        with_preview(Preview::hide_detail);
    });
    w.window().on_close_requested(|| {
        slint::Timer::single_shot(Duration::ZERO, close);
        slint::CloseRequestResponse::HideWindow
    });
}

/// Closes the preview window and stops the camera.
pub fn close() {
    let a = app();
    let taken = a.preview.borrow_mut().take();
    if let Some(mut p) = taken {
        p.stop_camera();
        let _ = p.ui.hide();
        slint::Timer::single_shot(Duration::ZERO, move || drop(p));
    }
}

// ───────────────────────────── Camera ─────────────────────────────

/// Opens the webcam preview.
pub fn open_camera() {
    #[cfg(not(feature = "camera"))]
    {
        crate::message::show("camera-none", "", 2);
    }
    #[cfg(feature = "camera")]
    {
        let a = app();
        let (probed, names) = {
            let c = a.cameras.borrow();
            (c.probed, c.names.clone())
        };
        if probed && names.is_empty() {
            // Maybe a camera was plugged in since start-up.
            crate::app::probe_cameras();
            crate::message::show("camera-none", "", 2);
            return;
        }
        crate::overlay::close();
        if !ensure_window(Mode::Camera) {
            return;
        }
        let (device, mirror) = {
            let s = a.settings();
            (s.camera.device.clone(), s.camera.mirror)
        };
        let index = device
            .and_then(|d| names.iter().position(|n| *n == d))
            .unwrap_or(0);
        with_preview(|p| {
            p.mirror = mirror;
            p.ui.set_mirror(mirror);
            let model: Vec<slint::SharedString> = names.iter().map(|n| n.as_str().into()).collect();
            p.ui.set_cameras(ModelRc::new(VecModel::from(model)));
            p.ui.set_camera_index(index as i32);
        });
        qrtool_camera::request_permission(move |granted| {
            let _ = slint::invoke_from_event_loop(move || {
                if granted {
                    start_camera_session(index);
                } else {
                    with_preview(|p| p.ui.set_status(status::ERROR));
                    crate::message::show_with("camera-permission", "", 2, |_| {
                        qrtool_platform::open::open_camera_settings();
                    });
                }
            });
        });
    }
}

fn start_camera_session(index: usize) {
    #[cfg(feature = "camera")]
    {
        let all_formats = app().settings().scan.all_formats;
        with_preview(|p| {
            if p.mode != Mode::Camera {
                return;
            }
            p.stop_camera();
            p.items.clear();
            p.held.clear();
            p.selected = None;
            p.last_frame = 0;
            p.last_result = 0;
            p.refresh_pills();
            p.ui.set_status(status::STARTING);
            p.camera = Some(qrtool_camera::CameraSession::start(index as u32, all_formats));
            p.timer
                .start(slint::TimerMode::Repeated, Duration::from_millis(33), poll_camera);
        });
    }
    #[cfg(not(feature = "camera"))]
    let _ = index;
}

#[cfg(feature = "camera")]
fn poll_camera() {
    let a = app();
    let settings = a.settings().clone();
    let mut auto_open: Option<ScanItem> = None;
    let mut error: Option<qrtool_camera::CameraError> = None;
    {
        let mut guard = a.preview.borrow_mut();
        let Some(p) = guard.as_mut() else {
            return;
        };
        let Some(cam) = p.camera.as_ref() else {
            return;
        };
        if let Some(e) = cam.error() {
            error = Some(e);
        } else {
            if let Some((seq, frame)) = cam.latest_frame() {
                if seq != p.last_frame {
                    p.last_frame = seq;
                    p.frame_w = frame.width();
                    p.ui.set_frame(to_slint_image(&frame, p.mirror));
                    p.ui.set_frame_w(frame.width() as f32);
                    p.ui.set_frame_h(frame.height() as f32);
                    if p.ui.get_status() == status::STARTING {
                        p.ui.set_status(status::SCANNING);
                    }
                }
            }
            let hold = Duration::from_secs(u64::from(settings.camera.hold_seconds));
            let now = Instant::now();
            let result = cam.latest_result();
            if result.seq != p.last_result && p.detail.is_none() {
                p.last_result = result.seq;
                let previous = p.selected.and_then(|i| p.items.get(i)).map(|it| it.text.clone());
                let fresh = scan::items_from(result.detections, 0);
                // Codes in the newest frame drive the stabilizer (auto-open, history); the
                // list on screen also keeps codes seen during the last `hold` seconds.
                let visible: Vec<String> = fresh.iter().map(|it| it.text.clone()).collect();
                crate::hold::merge(&mut p.held, fresh, now, hold);
                p.items = p.held.iter().map(|h| h.item.clone()).collect();
                p.selected = previous
                    .and_then(|t| p.items.iter().position(|it| it.text == t))
                    .or(if p.items.is_empty() { None } else { Some(0) });
                p.refresh_pills();
                p.ui.set_status(if p.items.is_empty() {
                    status::SCANNING
                } else {
                    status::FOUND
                });

                let stable = p.stabilizer.feed(&visible, now);
                if !stable.is_empty() {
                    let new: Vec<ScanItem> = p
                        .items
                        .iter()
                        .filter(|it| stable.contains(&it.text))
                        .cloned()
                        .collect();
                    scan::on_found(&new, ScanSource::Camera, &settings);
                    if let Some(i) = scan::auto_open_candidate(&p.items, &settings) {
                        if stable.contains(&p.items[i].text) {
                            auto_open = Some(p.items[i].clone());
                        }
                    }
                }
            } else if p.detail.is_none() && crate::hold::expire(&mut p.held, now, hold) {
                // No new frame analysed, but held codes ran out.
                let previous = p.selected.and_then(|i| p.items.get(i)).map(|it| it.text.clone());
                p.items = p.held.iter().map(|h| h.item.clone()).collect();
                p.selected = previous
                    .and_then(|t| p.items.iter().position(|it| it.text == t))
                    .or(if p.items.is_empty() { None } else { Some(0) });
                p.refresh_pills();
                if p.items.is_empty() {
                    p.ui.set_status(status::SCANNING);
                }
            }
        }
    }
    if let Some(e) = error {
        with_preview(|p| {
            p.stop_camera();
            p.ui.set_status(status::ERROR);
        });
        match e {
            qrtool_camera::CameraError::PermissionDenied => {
                crate::message::show_with("camera-permission", "", 2, |_| {
                    qrtool_platform::open::open_camera_settings();
                });
            }
            qrtool_camera::CameraError::NoCamera => crate::message::show("camera-none", "", 2),
            other => crate::message::show("camera-failed", &other.to_string(), 3),
        }
        return;
    }
    if let Some(item) = auto_open {
        if scan::open_item(&item).is_ok() {
            after_used();
        }
    }
}

#[cfg(not(feature = "camera"))]
#[allow(dead_code)]
fn poll_camera() {}

/// Copies a frame into a Slint image, optionally mirrored horizontally.
fn to_slint_image(frame: &RgbaImage, mirror: bool) -> slint::Image {
    let (w, h) = frame.dimensions();
    if !mirror {
        return slint::Image::from_rgba8(SharedPixelBuffer::clone_from_slice(frame.as_raw(), w, h));
    }
    let mut buf = SharedPixelBuffer::<slint::Rgba8Pixel>::new(w, h);
    let row = w as usize * 4;
    let src = frame.as_raw();
    let dst = buf.make_mut_bytes();
    for y in 0..h as usize {
        let s = &src[y * row..(y + 1) * row];
        let d = &mut dst[y * row..(y + 1) * row];
        for x in 0..w as usize {
            let sx = (w as usize - 1 - x) * 4;
            d[x * 4..x * 4 + 4].copy_from_slice(&s[sx..sx + 4]);
        }
    }
    slint::Image::from_rgba8(buf)
}

/// After a code from the camera or an image was used.
fn after_used() {
    let close_window = with_preview(|p| match p.mode {
        Mode::Camera => app().settings().camera.close_after_scan,
        Mode::Image(_) => true,
    })
    .unwrap_or(false);
    if close_window {
        close();
    } else {
        with_preview(|p| {
            p.hide_detail();
            // Do not immediately re-open the same code.
            p.stabilizer.reset();
        });
    }
}

// ───────────────────────────── Images ─────────────────────────────

/// Asks for an image file and scans it.
pub fn pick_image() {
    let a = app();
    let title = a.tr("dialog-open-image");
    let filter = a.tr("filter-images");
    let _ = slint::spawn_local(async move {
        let file = rfd::AsyncFileDialog::new()
            .set_title(title)
            .add_filter(filter, &["png", "jpg", "jpeg", "bmp", "gif", "webp"])
            .pick_file()
            .await;
        let Some(file) = file else {
            return;
        };
        open_image_file(file.path().to_path_buf());
    });
}

/// Scans an image the user already chose (file dialog or the image right-click menu).
pub fn open_image_file(path: std::path::PathBuf) {
    // Windows MAX_PATH is far below this; reject anything that cannot be a real picture path.
    // A path that is not Unicode is refused so a replacement character cannot open another file.
    let Some(text) = path.to_str() else {
        crate::message::show("image-failed", &app().tr("shell-bad-path"), 0);
        return;
    };
    if text.is_empty() || text.len() > 32 * 1024 {
        crate::message::show("image-failed", &app().tr("shell-bad-path"), 0);
        return;
    }
    let all_formats = app().settings().scan.all_formats;
    std::thread::spawn(move || {
        let result = image::open(&path).map(|i| i.to_rgba8());
        match result {
            Ok(img) => decode_and_show(img, ScanSource::Image, all_formats),
            Err(e) => {
                let msg = e.to_string();
                let _ = slint::invoke_from_event_loop(move || {
                    crate::message::show("image-failed", &msg, 3);
                });
            }
        }
    });
}

/// Scans an image on the clipboard.
pub fn scan_clipboard() {
    match qrtool_platform::clipboard::get_image() {
        Some(img) => {
            let all_formats = app().settings().scan.all_formats;
            std::thread::spawn(move || decode_and_show(img, ScanSource::Clipboard, all_formats));
        }
        None => crate::message::show("clipboard-empty", "", 0),
    }
}

/// Runs on a worker thread: decodes `img` and shows it in the preview window.
fn decode_and_show(img: RgbaImage, source: ScanSource, all_formats: bool) {
    let img = Arc::new(img);
    let detections = scan_image(
        &img,
        DecodeOptions {
            all_formats,
            thorough: true,
        },
    );
    let items = scan::items_from(detections, 0);
    let (w, h) = img.dimensions();
    let buf = SharedPixelBuffer::<slint::Rgba8Pixel>::clone_from_slice(img.as_raw(), w, h);
    let _ = slint::invoke_from_event_loop(move || show_image(buf, w, h, items, source));
}

fn show_image(
    buf: SharedPixelBuffer<slint::Rgba8Pixel>,
    w: u32,
    h: u32,
    items: Vec<ScanItem>,
    source: ScanSource,
) {
    crate::overlay::close();
    if !ensure_window(Mode::Image(source)) {
        return;
    }
    let settings = app().settings().clone();
    let auto_open = with_preview(|p| {
        p.frame_w = w;
        p.ui.set_frame(slint::Image::from_rgba8(buf));
        p.ui.set_frame_w(w as f32);
        p.ui.set_frame_h(h as f32);
        p.items = items;
        p.selected = if p.items.is_empty() { None } else { Some(0) };
        p.refresh_pills();
        p.ui.set_status(if p.items.is_empty() {
            status::NOTHING
        } else {
            status::FOUND
        });
        scan::on_found(&p.items, source, &settings);
        scan::auto_open_candidate(&p.items, &settings).map(|i| p.items[i].clone())
    })
    .flatten();
    if let Some(item) = auto_open {
        if scan::open_item(&item).is_ok() {
            close();
        }
    }
}

// ───────────────────────────── Interaction ─────────────────────────────

fn activate(index: usize) {
    let Some(item) = with_preview(|p| {
        p.selected = Some(index);
        p.refresh_pills();
        p.items.get(index).cloned()
    })
    .flatten() else {
        return;
    };
    match scan::primary_action(&item) {
        Primary::Open => match scan::open_item(&item) {
            Ok(()) => after_used(),
            Err(e) => crate::message::show("open-failed", &e, 3),
        },
        Primary::Details => {
            with_preview(|p| p.show_detail(index, false));
        }
    }
}

fn key(k: &str) {
    match k {
        "escape" => {
            let had_detail = with_preview(|p| {
                let had = p.detail.is_some();
                if had {
                    p.hide_detail();
                }
                had
            })
            .unwrap_or(false);
            if !had_detail {
                close();
            }
        }
        "enter" => {
            let target = with_preview(|p| match p.detail {
                Some(_) => Err(p
                    .ui
                    .get_detail()
                    .actions
                    .iter()
                    .find(|a| a.primary)
                    .map(|a| a.id.to_string())),
                None => Ok(p.selected),
            });
            match target {
                Some(Ok(Some(i))) => activate(i),
                Some(Err(Some(id))) => detail_action(&id),
                _ => {}
            }
        }
        "tab" | "backtab" => {
            with_preview(|p| {
                if p.items.is_empty() || p.detail.is_some() {
                    return;
                }
                let n = p.items.len();
                let next = match (p.selected, k == "tab") {
                    (None, _) => 0,
                    (Some(i), true) => (i + 1) % n,
                    (Some(i), false) => (i + n - 1) % n,
                };
                p.selected = Some(next);
                p.refresh_pills();
            });
        }
        "copy" => {
            let text = with_preview(|p| {
                let i = p.detail.map(|d| d.0).or(p.selected)?;
                p.items.get(i).map(|it| it.text.clone())
            })
            .flatten();
            if let Some(t) = text {
                qrtool_platform::clipboard::set_text(&t);
            }
        }
        _ => {}
    }
}

fn detail_action(id: &str) {
    let Some((item, confirm, index)) = with_preview(|p| {
        let (i, confirm) = p.detail?;
        Some((p.items.get(i)?.clone(), confirm, i))
    })
    .flatten() else {
        return;
    };
    if id == "close" {
        with_preview(Preview::hide_detail);
        return;
    }
    let outcome = {
        let a = app();
        let strings = a.tray.global::<ui::Strings>();
        detail::perform(id, &item, confirm, &strings)
    };
    match outcome {
        Outcome::Done => after_used(),
        Outcome::Copied | Outcome::CloseCard | Outcome::Pending => {
            with_preview(Preview::hide_detail);
        }
        Outcome::Rebuild { confirm_danger } => {
            with_preview(|p| p.show_detail(index, confirm_danger));
        }
        Outcome::Error(kind, arg) => crate::message::show(kind, &arg, 3),
    }
}
