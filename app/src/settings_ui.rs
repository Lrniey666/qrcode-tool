//! The Settings window.

use std::time::Duration;

use qrtool_core::app_info;
use qrtool_core::history::{History, HistoryEntry};
use qrtool_core::payload::{classify, Payload};
use qrtool_core::settings::{Hotkeys, ThemeMode, ToolbarLayout};
use qrtool_core::url_safety::{self, Severity};
use qrtool_platform::{autostart, paths};
use slint::{ComponentHandle, ModelRc, SharedString, VecModel};

use crate::app::app;
use crate::hotkeys;
use crate::i18n;
use crate::scan::display;
use crate::theme::apply_theme;
use crate::ui;

pub struct SettingsUi {
    pub ui: ui::SettingsWindow,
    history: Vec<HistoryEntry>,
}

/// Opens Settings, optionally on a page (0 general … 5 about).
pub fn open(page: Option<i32>) {
    let a = app();
    if let Some(s) = a.settings_win.borrow_mut().as_mut() {
        if let Some(p) = page {
            s.ui.set_page(p);
        }
        if page == Some(4) {
            s.reload_history();
        }
        let _ = s.ui.show();
        return;
    }
    let Ok(w) = ui::SettingsWindow::new() else {
        tracing::error!("cannot create settings window");
        return;
    };
    apply_theme!(w, a.theme.get());
    wire(&w);
    let mut s = SettingsUi {
        ui: w,
        history: Vec::new(),
    };
    s.load();
    if let Some(p) = page {
        s.ui.set_page(p);
    }
    if let Err(e) = s.ui.show() {
        tracing::error!("cannot show settings: {e}");
        return;
    }
    crate::winutil::center_when_created(&s.ui);
    *a.settings_win.borrow_mut() = Some(s);
}

fn with<R>(f: impl FnOnce(&mut SettingsUi) -> R) -> Option<R> {
    let a = app();
    let mut guard = a.settings_win.borrow_mut();
    guard.as_mut().map(f)
}

fn close() {
    let a = app();
    let taken = a.settings_win.borrow_mut().take();
    if let Some(s) = taken {
        let recording = !s.ui.get_recording_id().is_empty();
        let _ = s.ui.hide();
        slint::Timer::single_shot(Duration::ZERO, move || drop(s));
        if recording {
            hotkeys::reapply(false);
        }
    }
}

impl SettingsUi {
    fn load(&mut self) {
        let a = app();
        let s = a.settings().clone();
        let w = &self.ui;
        w.set_languages(i18n::picker_model(&a.tr("system-language")));
        w.set_language_index(i18n::picker_index(&s.language));
        w.set_theme_index(s.theme.index());
        w.set_autostart(s.autostart);
        w.set_shell_menu(s.shell_menu);
        w.set_shell_menu_supported(cfg!(windows));
        w.set_bubble_at_startup(s.floating.bubble_at_startup);
        w.set_hide_in_fullscreen(s.floating.hide_in_fullscreen);
        w.set_toolbar_layout_index(s.floating.toolbar_layout.index());
        w.set_fullscreen_detection_supported(cfg!(windows));
        w.set_check_updates(s.updates.check_automatically);

        w.set_auto_open_trusted(s.scan.auto_open_trusted);
        w.set_auto_copy(s.scan.auto_copy);
        w.set_sound(s.scan.sound);
        w.set_all_formats(s.scan.all_formats);
        self.refresh_trusted();

        w.set_mirror(s.camera.mirror);
        w.set_close_after_scan(s.camera.close_after_scan);
        w.set_hold_seconds(s.camera.hold_seconds as i32);
        self.refresh_cameras();

        w.set_hotkeys_supported(hotkeys::supported() && a.hotkeys.borrow().is_available());
        self.refresh_hotkeys();

        w.set_history_enabled(s.history.enabled);
        w.set_history_max(s.history.max_entries as i32);

        w.set_version(app_info::VERSION.into());
        w.set_trademark_notice(app_info::QR_TRADEMARK_NOTICE.into());
        if w.get_update_status().is_empty() {
            w.set_update_status(a.tr("update-never").into());
        }
        self.reload_history();
    }

    fn refresh_trusted(&self) {
        let list: Vec<SharedString> = app()
            .settings()
            .scan
            .trusted_domains
            .iter()
            .map(|d| display(&idna_unicode(d), 120).into())
            .collect();
        self.ui.set_trusted_domains(ModelRc::new(VecModel::from(list)));
    }

    pub fn refresh_cameras(&self) {
        let a = app();
        let names = a.cameras.borrow().names.clone();
        let available = cfg!(feature = "camera") && !names.is_empty();
        let device = a.settings().camera.device.clone();
        let model: Vec<SharedString> = if names.is_empty() {
            vec![a.tr("no-camera").into()]
        } else {
            names.iter().map(|n| n.as_str().into()).collect()
        };
        let index = device
            .and_then(|d| names.iter().position(|n| *n == d))
            .unwrap_or(0);
        self.ui.set_camera_available(available);
        self.ui.set_cameras(ModelRc::new(VecModel::from(model)));
        self.ui.set_camera_index(index as i32);
    }

    fn refresh_hotkeys(&self) {
        let a = app();
        let cfg = a.settings().hotkeys.clone();
        let states = a.hotkeys.borrow().states;
        let values = [cfg.scan_screen, cfg.scan_region, cfg.scan_camera, cfg.toolbar];
        let rows: Vec<ui::HotkeyRow> = hotkeys::IDS
            .iter()
            .zip(values)
            .zip(states)
            .map(|((id, value), state)| ui::HotkeyRow {
                id: (*id).into(),
                value: hotkeys::display(&value).into(),
                state,
            })
            .collect();
        self.ui.set_hotkeys(ModelRc::new(VecModel::from(rows)));
    }

    fn reload_history(&mut self) {
        let s = app().settings().clone();
        let history = History::new(paths::history_file(), s.history.max_entries);
        let mut entries = history.load();
        entries.reverse();
        let offset = crate::app::local_offset();
        let fmt = time::macros::format_description!("[year]-[month]-[day] [hour]:[minute]");
        let rows: Vec<ui::HistoryRow> = entries
            .iter()
            .map(|e| {
                let time = time::OffsetDateTime::from_unix_timestamp(e.ts)
                    .map(|t| t.to_offset(offset))
                    .ok()
                    .and_then(|t| t.format(&fmt).ok())
                    .unwrap_or_default();
                ui::HistoryRow {
                    time: time.into(),
                    kind: e.kind.id().into(),
                    text: display(&e.text, 140).into(),
                    openable: history_openable(&e.text),
                }
            })
            .collect();
        self.ui.set_history(ModelRc::new(VecModel::from(rows)));
        self.history = entries;
    }
}

/// Shows IDN entries (stored as punycode) in Unicode.
fn idna_unicode(domain: &str) -> String {
    let (unicode, result) = idna::domain_to_unicode(domain);
    if result.is_ok() {
        unicode
    } else {
        domain.to_string()
    }
}

/// Only clean web links can be opened again from history; everything else is copied.
fn history_openable(text: &str) -> bool {
    matches!(classify(text, None), Payload::Url(r) if r.level < Severity::Warning)
}

fn wire(w: &ui::SettingsWindow) {
    w.on_changed(|key| changed(key.as_str()));
    w.on_add_trusted(|text| {
        let entry = url_safety::normalize_trusted_entry(text.as_str());
        let valid = !entry.is_empty()
            && entry.contains('.')
            && !entry.contains('/')
            && url_safety::analyze_url(&format!("https://{entry}/")).is_ok();
        if valid {
            app().update_settings(|s| {
                if !s.scan.trusted_domains.contains(&entry) {
                    s.scan.trusted_domains.push(entry.clone());
                }
            });
            with(|s| s.refresh_trusted());
        }
    });
    w.on_remove_trusted(|i| {
        app().update_settings(|s| {
            if (i as usize) < s.scan.trusted_domains.len() {
                s.scan.trusted_domains.remove(i as usize);
            }
        });
        with(|s| s.refresh_trusted());
    });
    w.on_start_recording(|id| {
        hotkeys::suspend();
        with(|s| s.ui.set_recording_id(id));
    });
    w.on_cancel_recording(|| {
        with(|s| s.ui.set_recording_id("".into()));
        hotkeys::reapply(false);
        with(|s| s.refresh_hotkeys());
    });
    w.on_key_recorded(|ctrl, alt, shift, meta, text| {
        // Modifier keys alone: keep waiting for the real key.
        if text
            .chars()
            .next()
            .is_some_and(|c| ('\u{0010}'..='\u{0018}').contains(&c))
        {
            return;
        }
        let Some(combo) = hotkeys::format_recorded(ctrl, alt, shift, meta, text.as_str()) else {
            return;
        };
        let Some(id) = with(|s| s.ui.get_recording_id().to_string()) else {
            return;
        };
        app().update_settings(|s| {
            let h = &mut s.hotkeys;
            // Moving a combination from one action to another clears the old one.
            for v in [
                &mut h.scan_screen,
                &mut h.scan_region,
                &mut h.scan_camera,
                &mut h.toolbar,
            ] {
                if v.eq_ignore_ascii_case(&combo) {
                    v.clear();
                }
            }
            match id.as_str() {
                "screen" => h.scan_screen = combo.clone(),
                "region" => h.scan_region = combo.clone(),
                "camera" => h.scan_camera = combo.clone(),
                "toolbar" => h.toolbar = combo.clone(),
                _ => {}
            }
        });
        with(|s| s.ui.set_recording_id("".into()));
        hotkeys::reapply(false);
        with(|s| s.refresh_hotkeys());
    });
    w.on_reset_hotkeys(|| {
        app().update_settings(|s| s.hotkeys = Hotkeys::default());
        hotkeys::reapply(false);
        with(|s| s.refresh_hotkeys());
    });
    w.on_history_open(|i| {
        let text = with(|s| s.history.get(i as usize).map(|e| e.text.clone())).flatten();
        if let Some(t) = text {
            if let Payload::Url(r) = classify(&t, None) {
                if r.level < Severity::Warning {
                    if let Err(e) = qrtool_platform::open::open_web(&r) {
                        crate::message::show("open-failed", &e.to_string(), 3);
                    }
                }
            }
        }
    });
    w.on_history_copy(|i| {
        let text = with(|s| s.history.get(i as usize).map(|e| e.text.clone())).flatten();
        if let Some(t) = text {
            qrtool_platform::clipboard::set_text(&t);
        }
    });
    w.on_history_clear(|| {
        let max = app().settings().history.max_entries;
        if let Err(e) = History::new(paths::history_file(), max).clear() {
            tracing::warn!("cannot clear history: {e}");
        }
        with(SettingsUi::reload_history);
    });
    w.on_check_for_updates(|| crate::updater::check(true));
    w.on_open_link(|which| open_link(which.as_str()));
    w.on_show_welcome(crate::welcome::open);
    w.window().on_close_requested(|| {
        slint::Timer::single_shot(Duration::ZERO, close);
        slint::CloseRequestResponse::HideWindow
    });
}

fn changed(key: &str) {
    let a = app();
    let Some(w) = a.settings_win.borrow().as_ref().map(|s| s.ui.clone_strong()) else {
        return;
    };
    match key {
        "language" => {
            let lang = i18n::setting_for_index(w.get_language_index());
            a.update_settings(|s| s.language = lang.clone());
            i18n::apply(&lang);
            crate::app::sync_shell_menu();
            let index = w.get_language_index();
            w.set_languages(i18n::picker_model(&a.tr("system-language")));
            w.set_language_index(index);
            with(|s| s.refresh_cameras());
        }
        "theme" => {
            a.update_settings(|s| s.theme = ThemeMode::from_index(w.get_theme_index()));
            a.refresh_theme();
        }
        "autostart" => {
            let on = w.get_autostart();
            if !autostart::set_enabled(on) {
                tracing::warn!("could not change autostart");
            }
            a.update_settings(|s| s.autostart = on);
        }
        "shell-menu" => {
            let on = w.get_shell_menu();
            a.update_settings(|s| s.shell_menu = on);
            crate::app::sync_shell_menu();
        }
        "bubble-at-startup" => {
            a.update_settings(|s| s.floating.bubble_at_startup = w.get_bubble_at_startup())
        }
        "hide-in-fullscreen" => {
            a.update_settings(|s| s.floating.hide_in_fullscreen = w.get_hide_in_fullscreen());
            a.floating.settings_changed();
        }
        "toolbar-layout" => {
            let layout = ToolbarLayout::from_index(w.get_toolbar_layout_index());
            if a.settings().floating.toolbar_layout != layout {
                a.update_settings(|s| s.floating.toolbar_layout = layout);
                a.floating.layout_changed();
            }
        }
        "check-updates" => a.update_settings(|s| s.updates.check_automatically = w.get_check_updates()),
        "auto-open-trusted" => a.update_settings(|s| s.scan.auto_open_trusted = w.get_auto_open_trusted()),
        "auto-copy" => a.update_settings(|s| s.scan.auto_copy = w.get_auto_copy()),
        "sound" => {
            let on = w.get_sound();
            a.update_settings(|s| s.scan.sound = on);
            if on {
                qrtool_platform::sound::play_found();
            }
        }
        "all-formats" => a.update_settings(|s| s.scan.all_formats = w.get_all_formats()),
        "camera" => {
            let name = a
                .cameras
                .borrow()
                .names
                .get(w.get_camera_index() as usize)
                .cloned();
            a.update_settings(|s| s.camera.device = name);
        }
        "hold-seconds" => {
            let secs = w.get_hold_seconds().clamp(0, 60) as u32;
            a.update_settings(|s| s.camera.hold_seconds = secs);
        }
        "mirror" => a.update_settings(|s| s.camera.mirror = w.get_mirror()),
        "close-after-scan" => a.update_settings(|s| s.camera.close_after_scan = w.get_close_after_scan()),
        "history-enabled" => {
            a.update_settings(|s| s.history.enabled = w.get_history_enabled());
            with(SettingsUi::reload_history);
        }
        "history-max" => {
            let max = w.get_history_max().clamp(5, 500) as usize;
            a.update_settings(|s| s.history.max_entries = max);
        }
        other => tracing::warn!("unknown setting {other}"),
    }
}

fn open_link(which: &str) {
    let repo = app_info::REPO_URL;
    let url = match which {
        "repo" => repo.to_string(),
        "issues" => format!("{repo}/issues"),
        "releases" => format!("{repo}/releases"),
        "privacy" => format!("{repo}/blob/main/docs/PRIVACY.md"),
        "licenses" => format!("{repo}/blob/main/THIRD_PARTY_NOTICES.md"),
        "sponsor" => "https://github.com/sponsors/naritaroad".to_string(),
        "config" => {
            let dir = paths::config_dir();
            let _ = std::fs::create_dir_all(&dir);
            if let Err(e) = qrtool_platform::open::reveal(&dir) {
                crate::message::show("open-failed", &e.to_string(), 3);
            }
            return;
        }
        "logs" => {
            let dir = paths::log_dir();
            let _ = std::fs::create_dir_all(&dir);
            if let Err(e) = qrtool_platform::open::reveal(&dir) {
                crate::message::show("open-failed", &e.to_string(), 3);
            }
            return;
        }
        _ => return,
    };
    if let Ok(report) = url_safety::analyze_url(&url) {
        if let Err(e) = qrtool_platform::open::open_web(&report) {
            crate::message::show("open-failed", &e.to_string(), 3);
        }
    }
}
