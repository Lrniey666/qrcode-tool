//! First-run welcome window: language, the autostart notice and how to scan.

use std::time::Duration;

use slint::ComponentHandle;

use crate::app::{app, dispatch, Action};
use crate::theme::apply_theme;
use crate::{i18n, ui};

pub fn open() {
    let a = app();
    if let Some(w) = a.welcome.borrow().as_ref() {
        let _ = w.show();
        return;
    }
    let Ok(w) = ui::WelcomeWindow::new() else {
        tracing::error!("cannot create welcome window");
        return;
    };
    apply_theme!(w, a.theme.get());
    {
        let s = a.settings();
        w.set_languages(i18n::picker_model(&a.tr("system-language")));
        w.set_language_index(i18n::picker_index(&s.language));
        w.set_autostart(s.autostart);
        w.set_hotkey_screen(crate::hotkeys::display(&s.hotkeys.scan_screen).into());
        w.set_hotkey_region(crate::hotkeys::display(&s.hotkeys.scan_region).into());
        w.set_hotkey_camera(crate::hotkeys::display(&s.hotkeys.scan_camera).into());
    }
    w.set_hotkeys_supported(crate::hotkeys::supported() && a.hotkeys.borrow().is_available());
    w.set_camera_available(a.camera_available());

    let weak = w.as_weak();
    w.on_language_changed(move |i| {
        let lang = i18n::setting_for_index(i);
        app().update_settings(|s| s.language = lang.clone());
        i18n::apply(&lang);
        if let Some(w) = weak.upgrade() {
            // The first entry ("Use system language") is translated too.
            w.set_languages(i18n::picker_model(&app().tr("system-language")));
            w.set_language_index(i);
        }
    });
    w.on_autostart_toggled(|on| {
        let ok = qrtool_platform::autostart::set_enabled(on);
        if !ok {
            tracing::warn!("could not change autostart");
        }
        app().update_settings(|s| s.autostart = on);
    });
    w.on_finish(finish);
    w.window().on_close_requested(|| {
        slint::Timer::single_shot(Duration::ZERO, || finish(false));
        slint::CloseRequestResponse::HideWindow
    });
    if let Err(e) = w.show() {
        tracing::error!("cannot show welcome window: {e}");
        return;
    }
    crate::winutil::center_when_created(&w);
    *a.welcome.borrow_mut() = Some(w);
}

fn finish(try_now: bool) {
    let a = app();
    a.update_settings(|s| s.first_run_completed = true);
    if let Some(w) = a.welcome.borrow_mut().take() {
        let _ = w.hide();
        slint::Timer::single_shot(Duration::ZERO, move || drop(w));
    }
    if try_now {
        // Let the window disappear before the screen is captured.
        slint::Timer::single_shot(Duration::from_millis(300), || dispatch(Action::ScanScreen));
    } else {
        a.floating.show_toolbar();
    }
}
