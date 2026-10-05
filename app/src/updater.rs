//! Updates through Velopack and GitHub Releases.
//!
//! Only installed copies (made with the setup program) can update themselves; portable
//! and developer builds are told to download new versions from GitHub.

use std::cell::Cell;

thread_local! {
    static BUSY: Cell<bool> = const { Cell::new(false) };
}

/// Velopack's install/update/uninstall hooks. Must be the first thing in `main`.
pub fn startup_hook() {
    #[cfg(feature = "updater")]
    {
        let mut app = velopack::VelopackApp::build();
        #[cfg(windows)]
        {
            app = app.on_before_uninstall_fast_callback(|_| {
                // Remove entries that would point at the deleted program.
                qrtool_platform::autostart::set_enabled(false);
                qrtool_platform::shell_menu::remove();
            });
        }
        app.run();
    }
}

/// Status line for Settings › About.
#[cfg(feature = "updater")]
fn set_status(text: &str, busy: bool) {
    let a = crate::app::app();
    let guard = a.settings_win.borrow();
    if let Some(s) = guard.as_ref() {
        s.ui.set_update_status(text.into());
        s.ui.set_update_busy(busy);
    }
}

#[cfg(feature = "updater")]
fn status_text(kind: &str, arg: &str) -> String {
    let a = crate::app::app();
    a.tray
        .global::<crate::ui::Strings>()
        .invoke_msg_title(kind.into(), arg.into())
        .to_string()
}

/// Checks GitHub for a newer version. `manual`: the user asked, so also report
/// "up to date" and errors; automatic checks only speak up when there is an update.
pub fn check(manual: bool) {
    if BUSY.with(Cell::get) {
        return;
    }
    #[cfg(not(feature = "updater"))]
    {
        if manual {
            crate::message::show("update-not-installed", "", 0);
        }
    }
    #[cfg(feature = "updater")]
    {
        BUSY.with(|b| b.set(true));
        set_status(&crate::app::app().tr("update-checking"), true);
        std::thread::spawn(move || {
            let result = check_blocking();
            let _ = slint::invoke_from_event_loop(move || {
                BUSY.with(|b| b.set(false));
                handle_result(result, manual);
            });
        });
    }
}

#[cfg(feature = "updater")]
enum CheckResult {
    NotInstalled,
    UpToDate(String),
    Available(Box<velopack::UpdateInfo>),
    Error(String),
}

#[cfg(feature = "updater")]
fn manager() -> Result<velopack::UpdateManager, velopack::Error> {
    let source = velopack::sources::GithubSource::new(qrtool_core::app_info::REPO_URL, None, false);
    velopack::UpdateManager::new(source, None, None)
}

#[cfg(feature = "updater")]
fn check_blocking() -> CheckResult {
    let um = match manager() {
        Ok(um) => um,
        Err(velopack::Error::NotInstalled(_)) => return CheckResult::NotInstalled,
        Err(e) => return CheckResult::Error(e.to_string()),
    };
    if um.get_is_portable() {
        return CheckResult::NotInstalled;
    }
    match um.check_for_updates() {
        Ok(velopack::UpdateCheck::UpdateAvailable(info)) => CheckResult::Available(info),
        Ok(_) => CheckResult::UpToDate(um.get_current_version_as_string()),
        Err(e) => CheckResult::Error(e.to_string()),
    }
}

#[cfg(feature = "updater")]
fn handle_result(result: CheckResult, manual: bool) {
    use slint::ComponentHandle as _;
    match result {
        CheckResult::NotInstalled => {
            set_status(&status_text("update-not-installed", ""), false);
            if manual {
                crate::message::show("update-not-installed", "", 0);
            }
        }
        CheckResult::UpToDate(v) => {
            set_status(&status_text("update-none", &v), false);
            if manual {
                crate::message::show("update-none", &v, 0);
            }
        }
        CheckResult::Error(e) => {
            tracing::warn!("update check failed: {e}");
            set_status(&status_text("update-error", ""), false);
            if manual {
                crate::message::show("update-error", &e, 2);
            }
        }
        CheckResult::Available(info) => {
            let version = info.TargetFullRelease.Version.clone();
            tracing::info!("update available: {version}");
            set_status(&status_text("update-available", &version), false);
            let info = std::rc::Rc::new(*info);
            crate::message::show_with("update-available", &version, 0, move |win| {
                win.set_busy(true);
                win.set_kind("update-downloading".into());
                download_and_restart((*info).clone(), win.as_weak());
            });
        }
    }
}

#[cfg(feature = "updater")]
fn download_and_restart(info: velopack::UpdateInfo, win: slint::Weak<crate::ui::MessageWindow>) {
    BUSY.with(|b| b.set(true));
    // Save state now: the process is replaced right after the download.
    crate::app::app().floating.save_positions();
    std::thread::spawn(move || {
        let result = manager().and_then(|um| {
            um.download_updates(&info, None)?;
            um.apply_updates_and_restart(&info)
        });
        if let Err(e) = result {
            let msg = e.to_string();
            let _ = slint::invoke_from_event_loop(move || {
                BUSY.with(|b| b.set(false));
                if let Some(w) = win.upgrade() {
                    crate::message::close(&w);
                }
                crate::message::show("update-error", &msg, 3);
            });
        }
    });
}
