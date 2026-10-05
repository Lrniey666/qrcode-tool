//! Screen scanning: freeze the screen, find codes, show Lens-style pills.
//!
//! Flow: hide our own floating windows → capture every monitor (background thread) →
//! one borderless full-screen overlay per monitor showing the frozen picture → decode in
//! the background → pills. Whole-screen scan and region selection are separate actions:
//! one searches every monitor immediately, the other waits for a dragged box.

use std::sync::Arc;
use std::time::Duration;

use image::RgbaImage;
use qrtool_core::history::ScanSource;
use qrtool_decode::{scan_image, scan_region, DecodeOptions, Rect};
use slint::{ComponentHandle, Model, SharedPixelBuffer};

use crate::app::{app, dispatch, Action};
use crate::detail::{self, Outcome};
use crate::scan::{self, Primary, ScanItem};
use crate::theme::{apply_theme, ThemeState};
use crate::ui;
use crate::winutil::{self, MonitorRect};

/// Status values understood by `OverlayWindow.status`.
mod status {
    pub const HINT: i32 = 0;
    pub const SCANNING: i32 = 1;
    pub const FOUND: i32 = 2;
    pub const NONE_ON_SCREEN: i32 = 3;
    pub const NONE_IN_REGION: i32 = 4;
}

struct Monitor {
    ui: ui::OverlayWindow,
    image: Arc<RgbaImage>,
}

/// An open screen-scan session.
pub struct Session {
    monitors: Vec<Monitor>,
    items: Vec<ScanItem>,
    selected: Option<usize>,
    /// Item whose detail card is open, and whether a dangerous link was confirmed once.
    detail: Option<(usize, bool)>,
    /// Incremented for every decode run so stale results are ignored.
    generation: u64,
    /// Floating windows that were visible before the scan and must come back.
    restore_toolbar: bool,
    restore_bubble: bool,
}

impl Session {
    pub fn apply_theme(&self, t: ThemeState) {
        for m in &self.monitors {
            apply_theme!(m.ui, t);
        }
    }

    fn strings(&self) -> Option<ui::Strings<'_>> {
        self.monitors.first().map(|m| m.ui.global::<ui::Strings>())
    }

    /// Pushes items/selection into every overlay.
    fn refresh_pills(&self) {
        let Some(s) = self.strings() else {
            return;
        };
        for (i, m) in self.monitors.iter().enumerate() {
            let pills = scan::pills_for(&self.items, i, self.selected, &s, None);
            if let Some(model) = scan::update_pills(m.ui.get_pills(), pills) {
                m.ui.set_pills(model);
            }
            m.ui.set_found_count(self.items.len() as i32);
        }
    }

    fn set_status(&self, st: i32) {
        for m in &self.monitors {
            m.ui.set_status(st);
        }
    }

    fn show_detail(&mut self, index: usize, confirm: bool) {
        let Some(item) = self.items.get(index) else {
            return;
        };
        let trusted = app().settings().scan.trusted_domains.clone();
        for m in &self.monitors {
            m.ui.set_detail_visible(false);
        }
        let m = &self.monitors[item.source.min(self.monitors.len() - 1)];
        let data = detail::build(item, confirm, &trusted, &m.ui.global::<ui::Strings>());
        m.ui.set_detail(data);
        m.ui.set_detail_visible(true);
        self.detail = Some((index, confirm));
        self.selected = Some(index);
    }

    fn hide_detail(&mut self) {
        for m in &self.monitors {
            m.ui.set_detail_visible(false);
        }
        self.detail = None;
    }
}

/// Freezes the screen and searches every monitor for codes.
pub fn start_screen() {
    start(false);
}

/// Freezes the screen and waits for a dragged region. Does not search the whole screen.
pub fn start_region() {
    start(true);
}

fn start(region: bool) {
    let a = app();
    if a.overlay.borrow().is_some() {
        // The screen is already frozen. Switch mode instead of capturing again.
        if region {
            enter_region_mode();
        } else {
            rescan();
        }
        return;
    }
    let (restore_toolbar, restore_bubble) = a.floating.hide_for_capture();
    // Give the compositor a moment to remove our windows from the screen.
    let delay = if restore_toolbar || restore_bubble { 120 } else { 0 };
    slint::Timer::single_shot(Duration::from_millis(delay), move || {
        capture(restore_toolbar, restore_bubble, region);
    });
}

fn capture(restore_toolbar: bool, restore_bubble: bool, region: bool) {
    std::thread::Builder::new()
        .name("capture".into())
        .spawn(move || {
            let started = std::time::Instant::now();
            let result = qrtool_capture::capture_all().map(|shots| {
                shots
                    .into_iter()
                    .map(|s| {
                        let buf = SharedPixelBuffer::<slint::Rgba8Pixel>::clone_from_slice(
                            s.image.as_raw(),
                            s.image.width(),
                            s.image.height(),
                        );
                        let rect = MonitorRect {
                            x: s.x,
                            y: s.y,
                            width: s.width,
                            height: s.height,
                        };
                        (rect, s.is_primary, Arc::new(s.image), buf)
                    })
                    .collect::<Vec<_>>()
            });
            tracing::debug!("capture took {:?}", started.elapsed());
            let _ = slint::invoke_from_event_loop(move || match result {
                Ok(shots) => open_overlays(shots, restore_toolbar, restore_bubble, region),
                Err(e) => {
                    app()
                        .floating
                        .restore_after_capture(restore_toolbar, restore_bubble);
                    if matches!(e, qrtool_capture::CaptureError::PermissionDenied) {
                        crate::message::show_with("capture-permission", "", 2, |_| {
                            qrtool_platform::open::open_screen_recording_settings();
                        });
                    } else {
                        crate::message::show("capture-failed", &e.to_string(), 3);
                    }
                }
            });
        })
        .expect("cannot spawn capture thread");
}

type Shot = (
    MonitorRect,
    bool,
    Arc<RgbaImage>,
    SharedPixelBuffer<slint::Rgba8Pixel>,
);

fn open_overlays(shots: Vec<Shot>, restore_toolbar: bool, restore_bubble: bool, region: bool) {
    tracing::debug!(monitors = shots.len(), region, "open overlays");
    let a = app();
    let theme = a.theme.get();
    let (hk_screen, hk_region, hk_camera) = {
        let s = a.settings();
        if crate::hotkeys::supported() {
            (
                crate::hotkeys::display(&s.hotkeys.scan_screen),
                crate::hotkeys::display(&s.hotkeys.scan_region),
                crate::hotkeys::display(&s.hotkeys.scan_camera),
            )
        } else {
            (String::new(), String::new(), String::new())
        }
    };
    let has_primary = shots.iter().any(|s| s.1);

    let mut monitors = Vec::new();
    for (i, (rect, is_primary, image, buf)) in shots.into_iter().enumerate() {
        let primary = is_primary || (!has_primary && i == 0);
        let Ok(w) = ui::OverlayWindow::new() else {
            tracing::error!("cannot create overlay window");
            continue;
        };
        apply_theme!(w, theme);
        w.set_screenshot(slint::Image::from_rgba8(buf));
        w.set_img_w(image.width() as f32);
        w.set_img_h(image.height() as f32);
        w.set_primary(primary);
        w.set_animate(qrtool_platform::theme::animations_enabled());
        w.set_region_mode(region);
        if !region {
            w.invoke_play_sweep();
        }
        w.set_status(if region { status::HINT } else { status::SCANNING });
        w.set_camera_available(a.camera_available());
        w.set_hotkey_screen(hk_screen.clone().into());
        w.set_hotkey_region(hk_region.clone().into());
        w.set_hotkey_camera(hk_camera.clone().into());
        wire(&w, i);
        winutil::place_on(w.window(), rect);
        if let Err(e) = w.show() {
            tracing::error!("cannot show overlay: {e}");
            continue;
        }
        winutil::when_created(&w, move |c, ww| {
            winutil::make_overlay(ww, rect, primary);
            winutil::trace_input_events(c.window());
        });
        monitors.push(Monitor { ui: w, image });
    }
    if monitors.is_empty() {
        a.floating.restore_after_capture(restore_toolbar, restore_bubble);
        return;
    }

    *a.overlay.borrow_mut() = Some(Session {
        monitors,
        items: Vec::new(),
        selected: None,
        detail: None,
        generation: 0,
        restore_toolbar,
        restore_bubble,
    });
    if !region {
        decode_all(true);
    }
}

/// Switches a frozen screen into region selection without searching it again.
fn enter_region_mode() {
    with_session(|s| {
        s.hide_detail();
        for m in &s.monitors {
            m.ui.set_region_mode(true);
            if s.items.is_empty() {
                m.ui.set_status(status::HINT);
            }
        }
    });
}

fn wire(w: &ui::OverlayWindow, monitor: usize) {
    w.on_region_selected(move |x, y, width, height| region_selected(monitor, Rect::new(x, y, width, height)));
    w.on_pill_clicked(move |local| {
        if let Some(i) = global_index(monitor, local as usize) {
            activate(i);
        }
    });
    w.on_pill_details(move |local| {
        if let Some(i) = global_index(monitor, local as usize) {
            with_session(|s| s.show_detail(i, false));
        }
    });
    w.on_empty_click(|| {
        with_session(|s| {
            if s.detail.is_some() {
                s.hide_detail();
            }
        });
    });
    w.on_key(|k| key(k.as_str()));
    w.on_toolbar(|id| {
        let action = match id.as_str() {
            "rescan" => {
                rescan();
                return;
            }
            "region" => {
                enter_region_mode();
                return;
            }
            "camera" => Action::ScanCamera,
            "clipboard" => Action::ScanClipboard,
            "image" => Action::OpenImage,
            "generate" => Action::Generate(None),
            "history" => Action::History,
            "settings" => Action::Settings,
            _ => return,
        };
        close();
        dispatch(action);
    });
    w.on_detail_action(|id| detail_action(id.as_str()));
    w.on_detail_copy_field(|idx| {
        with_session(|s| {
            if let Some((i, _)) = s.detail {
                let m = &s.monitors[s.items[i].source.min(s.monitors.len() - 1)];
                if let Some(f) = m.ui.get_detail().fields.row_data(idx as usize) {
                    qrtool_platform::clipboard::set_text(f.value.as_str());
                }
            }
        });
    });
    w.on_detail_close(|| {
        with_session(Session::hide_detail);
    });
    w.window().on_close_requested(|| {
        // Alt+F4 or similar: end the whole session.
        slint::Timer::single_shot(Duration::ZERO, close);
        slint::CloseRequestResponse::KeepWindowShown
    });
}

fn with_session<R>(f: impl FnOnce(&mut Session) -> R) -> Option<R> {
    let a = app();
    let mut guard = a.overlay.borrow_mut();
    guard.as_mut().map(f)
}

fn global_index(monitor: usize, local: usize) -> Option<usize> {
    with_session(|s| {
        s.items
            .iter()
            .enumerate()
            .filter(|(_, it)| it.source == monitor)
            .nth(local)
            .map(|(i, _)| i)
    })
    .flatten()
}

/// Ends the session and brings back the floating windows.
pub fn close() {
    tracing::debug!("overlay close");
    let a = app();
    let session = a.overlay.borrow_mut().take();
    if let Some(s) = session {
        for m in &s.monitors {
            let _ = m.ui.hide();
        }
        a.floating
            .restore_after_capture(s.restore_toolbar, s.restore_bubble);
        // Drop the windows (and the large screenshots) once the current callback, which
        // may belong to one of these windows, has returned.
        slint::Timer::single_shot(Duration::ZERO, move || drop(s));
    }
}

fn decode_all(first: bool) {
    let Some((generation, images, all_formats)) = with_session(|s| {
        s.generation += 1;
        s.set_status(status::SCANNING);
        let images: Vec<Arc<RgbaImage>> = s.monitors.iter().map(|m| m.image.clone()).collect();
        (s.generation, images, app().settings().scan.all_formats)
    }) else {
        return;
    };
    let started = std::time::Instant::now();
    std::thread::Builder::new()
        .name("decode".into())
        .spawn(move || {
            let opts = DecodeOptions {
                all_formats,
                thorough: true,
            };
            // One thread per monitor.
            let handles: Vec<_> = images
                .into_iter()
                .enumerate()
                .map(|(i, img)| std::thread::spawn(move || (i, scan_image(&img, opts))))
                .collect();
            let mut items = Vec::new();
            for h in handles {
                if let Ok((i, dets)) = h.join() {
                    items.extend(scan::items_from(dets, i));
                }
            }
            tracing::debug!("decode took {:?}, {} codes", started.elapsed(), items.len());
            let _ = slint::invoke_from_event_loop(move || decoded(generation, items, first));
        })
        .expect("cannot spawn decode thread");
}

fn decoded(generation: u64, items: Vec<ScanItem>, first: bool) {
    tracing::debug!(generation, count = items.len(), "overlay decoded");
    let a = app();
    let settings = a.settings().clone();
    let auto_open = {
        let mut guard = a.overlay.borrow_mut();
        let Some(s) = guard.as_mut() else {
            return;
        };
        if s.generation != generation {
            return;
        }
        let added = scan::add_unique(&mut s.items, items);
        if s.items.is_empty() {
            s.set_status(status::NONE_ON_SCREEN);
            return;
        }
        s.set_status(status::FOUND);
        if s.selected.is_none() {
            s.selected = Some(0);
        }
        s.refresh_pills();
        if added > 0 {
            scan::on_found(&s.items, ScanSource::Screen, &settings);
        }
        if first {
            scan::auto_open_candidate(&s.items, &settings).map(|i| s.items[i].clone())
        } else {
            None
        }
    };
    if let Some(item) = auto_open {
        tracing::info!("opening trusted link automatically");
        if scan::open_item(&item).is_ok() {
            close();
        }
    }
}

fn region_selected(monitor: usize, rect: Rect) {
    let Some((image, generation, all_formats)) = with_session(|s| {
        s.hide_detail();
        s.generation += 1;
        s.set_status(status::SCANNING);
        let image = s.monitors.get(monitor).map(|m| m.image.clone());
        (image, s.generation, app().settings().scan.all_formats)
    }) else {
        return;
    };
    let Some(image) = image else {
        return;
    };
    std::thread::spawn(move || {
        let opts = DecodeOptions {
            all_formats,
            thorough: true,
        };
        let rect = rect.clamp_to(image.width(), image.height());
        let items = scan::items_from(scan_region(&image, rect, opts), monitor);
        let _ = slint::invoke_from_event_loop(move || {
            let a = app();
            let settings = a.settings().clone();
            let mut guard = a.overlay.borrow_mut();
            let Some(s) = guard.as_mut() else {
                return;
            };
            if s.generation != generation {
                return;
            }
            let first_new = s.items.len();
            let added = scan::add_unique(&mut s.items, items.clone());
            if added == 0 && items.is_empty() {
                s.set_status(if s.items.is_empty() {
                    status::NONE_IN_REGION
                } else {
                    status::FOUND
                });
                return;
            }
            // Select the code that was just found (or the existing one in the region).
            let sel = if added > 0 {
                first_new.min(s.items.len() - 1)
            } else {
                s.items
                    .iter()
                    .position(|it| it.source == monitor && items.iter().any(|n| n.text == it.text))
                    .unwrap_or(0)
            };
            s.selected = Some(sel);
            s.set_status(status::FOUND);
            s.refresh_pills();
            if added > 0 {
                scan::on_found(&s.items[first_new..], ScanSource::Screen, &settings);
            }
        });
    });
}

fn rescan() {
    with_session(|s| {
        s.hide_detail();
        s.items.clear();
        s.selected = None;
        s.refresh_pills();
        for m in &s.monitors {
            m.ui.set_region_mode(false);
            m.ui.invoke_play_sweep();
        }
    });
    decode_all(false);
}

/// Enter / click on a pill.
fn activate(index: usize) {
    tracing::debug!(index, "overlay activate");
    let Some(item) = with_session(|s| {
        s.selected = Some(index);
        s.refresh_pills();
        s.items.get(index).cloned()
    })
    .flatten() else {
        return;
    };
    match scan::primary_action(&item) {
        Primary::Open => match scan::open_item(&item) {
            Ok(()) => close(),
            Err(e) => crate::message::show("open-failed", &e, 3),
        },
        Primary::Details => {
            with_session(|s| s.show_detail(index, false));
        }
    }
}

fn key(k: &str) {
    tracing::debug!(key = k, "overlay key");
    match k {
        "escape" => {
            let had_detail = with_session(|s| {
                let had = s.detail.is_some();
                if had {
                    s.hide_detail();
                }
                had
            })
            .unwrap_or(false);
            if !had_detail {
                close();
            }
        }
        "enter" => {
            let target = with_session(|s| match s.detail {
                Some((i, _)) => Err(i),
                None => Ok(s.selected.or(if s.items.len() == 1 { Some(0) } else { None })),
            });
            match target {
                Some(Ok(Some(i))) => activate(i),
                // Enter on an open card runs its primary action.
                Some(Err(i)) => {
                    let primary = with_session(|s| {
                        let m = &s.monitors[s.items[i].source.min(s.monitors.len() - 1)];
                        let actions = m.ui.get_detail().actions;
                        actions.iter().find(|a| a.primary).map(|a| a.id.to_string())
                    })
                    .flatten();
                    if let Some(id) = primary {
                        detail_action(&id);
                    }
                }
                _ => {}
            }
        }
        "tab" | "backtab" => {
            with_session(|s| {
                if s.items.is_empty() || s.detail.is_some() {
                    return;
                }
                let n = s.items.len();
                let next = match (s.selected, k == "tab") {
                    (None, _) => 0,
                    (Some(i), true) => (i + 1) % n,
                    (Some(i), false) => (i + n - 1) % n,
                };
                s.selected = Some(next);
                s.refresh_pills();
            });
        }
        "copy" => {
            let text = with_session(|s| {
                let i = s.detail.map(|d| d.0).or(s.selected)?;
                s.items.get(i).map(|it| it.text.clone())
            })
            .flatten();
            if let Some(t) = text {
                qrtool_platform::clipboard::set_text(&t);
                close();
            }
        }
        "rescan" => rescan(),
        _ => {}
    }
}

fn detail_action(id: &str) {
    let Some((item, confirm, index)) = with_session(|s| {
        let (i, confirm) = s.detail?;
        Some((s.items.get(i)?.clone(), confirm, i))
    })
    .flatten() else {
        return;
    };
    if id == "close" {
        with_session(Session::hide_detail);
        return;
    }
    let outcome = {
        let a = app();
        let strings = a.tray.global::<ui::Strings>();
        detail::perform(id, &item, confirm, &strings)
    };
    match outcome {
        Outcome::Done | Outcome::Copied => close(),
        Outcome::Rebuild { confirm_danger } => {
            with_session(|s| s.show_detail(index, confirm_danger));
        }
        Outcome::CloseCard => {
            with_session(Session::hide_detail);
        }
        Outcome::Error(kind, arg) => crate::message::show(kind, &arg, 3),
        Outcome::Pending => {
            // A save dialog opens; the overlay would cover it.
            close();
        }
    }
}
