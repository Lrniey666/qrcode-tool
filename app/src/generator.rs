//! "Create QR code": text or link → QR code to scan with a phone, copy or save.

use std::time::Duration;

use qrtool_decode::{generate_qr, EcLevel, QrMatrix};
use slint::{ComponentHandle, SharedPixelBuffer};

use crate::app::app;
use crate::theme::apply_theme;
use crate::ui;

const DARK: [u8; 4] = [0, 0, 0, 255];
const LIGHT: [u8; 4] = [255, 255, 255, 255];

pub struct Generator {
    pub ui: ui::GeneratorWindow,
    matrix: Option<QrMatrix>,
    feedback_timer: slint::Timer,
}

/// Opens the generator, optionally pre-filled.
pub fn open(text: Option<String>) {
    let a = app();
    if let Some(g) = a.generator.borrow_mut().as_mut() {
        if let Some(t) = text {
            g.ui.set_text(t.as_str().into());
            g.update();
        }
        let _ = g.ui.show();
        return;
    }
    let Ok(w) = ui::GeneratorWindow::new() else {
        tracing::error!("cannot create generator window");
        return;
    };
    apply_theme!(w, a.theme.get());
    w.set_text(text.unwrap_or_default().into());
    w.on_text_edited(|_| {
        with(Generator::update);
    });
    w.on_ec_changed(|_| {
        with(Generator::update);
    });
    w.on_copy_image(|| {
        with(|g| {
            if let Some(m) = &g.matrix {
                let img = m.to_image(10, DARK, LIGHT);
                if qrtool_platform::clipboard::set_image(&img) {
                    let msg = app().tr("copied");
                    g.flash(&msg);
                }
            }
        });
    });
    w.on_save_png(|| save(false));
    w.on_save_svg(|| save(true));
    w.on_close_requested(close);
    w.window().on_close_requested(|| {
        slint::Timer::single_shot(Duration::ZERO, close);
        slint::CloseRequestResponse::HideWindow
    });
    if let Err(e) = w.show() {
        tracing::error!("cannot show generator: {e}");
        return;
    }
    crate::winutil::center_when_created(&w);
    let mut g = Generator {
        ui: w,
        matrix: None,
        feedback_timer: slint::Timer::default(),
    };
    g.update();
    *a.generator.borrow_mut() = Some(g);
}

fn with<R>(f: impl FnOnce(&mut Generator) -> R) -> Option<R> {
    let a = app();
    let mut guard = a.generator.borrow_mut();
    guard.as_mut().map(f)
}

pub fn close() {
    let a = app();
    let taken = a.generator.borrow_mut().take();
    if let Some(g) = taken {
        let _ = g.ui.hide();
        slint::Timer::single_shot(Duration::ZERO, move || drop(g));
    }
}

impl Generator {
    fn update(&mut self) {
        let text = self.ui.get_text().to_string();
        let level = EcLevel::from_index(self.ui.get_ec_level());
        self.ui.set_feedback("".into());
        if text.is_empty() {
            self.matrix = None;
            self.ui.set_state(0);
            return;
        }
        match generate_qr(&text, level) {
            Ok(m) => {
                // Render at an integer scale close to 2× the 276 px preview for sharp edges.
                let modules = m.size + 8;
                let scale = (552 / modules).max(1);
                let img = m.to_image(scale, DARK, LIGHT);
                let buf = SharedPixelBuffer::<slint::Rgba8Pixel>::clone_from_slice(
                    img.as_raw(),
                    img.width(),
                    img.height(),
                );
                self.ui.set_preview(slint::Image::from_rgba8(buf));
                self.ui.set_modules(m.size as i32);
                self.ui.set_state(1);
                self.matrix = Some(m);
            }
            Err(_) => {
                self.matrix = None;
                self.ui.set_state(2);
            }
        }
    }

    fn flash(&self, msg: &str) {
        self.ui.set_feedback(msg.into());
        let weak = self.ui.as_weak();
        self.feedback_timer
            .start(slint::TimerMode::SingleShot, Duration::from_secs(3), move || {
                if let Some(w) = weak.upgrade() {
                    w.set_feedback("".into());
                }
            });
    }
}

fn save(svg: bool) {
    let Some(matrix) = with(|g| g.matrix.clone()).flatten() else {
        return;
    };
    let a = app();
    let title = a.tr(if svg { "dialog-save-svg" } else { "dialog-save-png" });
    let (ext, name) = if svg { ("svg", "SVG") } else { ("png", "PNG") };
    let _ = slint::spawn_local(async move {
        let Some(file) = rfd::AsyncFileDialog::new()
            .set_title(title)
            .add_filter(name, &[ext])
            .set_file_name(format!("qrcode.{ext}"))
            .save_file()
            .await
        else {
            return;
        };
        let path = file.path().to_path_buf();
        let result = if svg {
            std::fs::write(&path, matrix.to_svg()).map_err(|e| e.to_string())
        } else {
            matrix
                .to_image(16, DARK, LIGHT)
                .save_with_format(&path, image::ImageFormat::Png)
                .map_err(|e| e.to_string())
        };
        match result {
            Ok(()) => {
                let msg = app().tr("saved");
                with(|g| g.flash(&msg));
            }
            Err(e) => crate::message::show("open-failed", &e, 3),
        }
    });
}
