//! Small message dialogs (errors, permission hints, update prompts).

use slint::ComponentHandle;

use crate::app::app;
use crate::theme::apply_theme;
use crate::ui;

/// Shows a message of `kind` (see `Strings.msg-title` in `strings.slint`).
/// `level`: 0 info, 2 warning, 3 error.
pub fn show(kind: &str, arg: &str, level: i32) {
    show_with(kind, arg, level, |_| {});
}

/// Like [`show`], but `on_primary` runs when the primary button is pressed (the dialog
/// closes afterwards unless `on_primary` sets it busy and closes it later itself).
pub fn show_with(kind: &str, arg: &str, level: i32, on_primary: impl Fn(&ui::MessageWindow) + 'static) {
    let a = app();
    // Do not stack the same message twice.
    if let Some(existing) = a
        .messages
        .borrow()
        .iter()
        .find(|m| m.get_kind() == kind && m.get_arg() == arg)
    {
        let _ = existing.show();
        return;
    }
    let Ok(win) = ui::MessageWindow::new() else {
        tracing::error!("cannot create message window");
        return;
    };
    apply_theme!(win, a.theme.get());
    win.set_kind(kind.into());
    win.set_arg(arg.into());
    win.set_level(level);

    let weak = win.as_weak();
    win.on_primary(move || {
        if let Some(w) = weak.upgrade() {
            on_primary(&w);
            if !w.get_busy() {
                close(&w);
            }
        }
    });
    let weak = win.as_weak();
    win.on_secondary(move || {
        if let Some(w) = weak.upgrade() {
            close(&w);
        }
    });
    let weak = win.as_weak();
    win.window().on_close_requested(move || {
        if let Some(w) = weak.upgrade() {
            forget(&w);
        }
        slint::CloseRequestResponse::HideWindow
    });
    if let Err(e) = win.show() {
        tracing::error!("cannot show message: {e}");
        return;
    }
    crate::winutil::center_when_created(&win);
    a.messages.borrow_mut().push(win);
}

/// Closes a message window and releases it.
pub fn close(win: &ui::MessageWindow) {
    let _ = win.hide();
    forget(win);
}

fn forget(win: &ui::MessageWindow) {
    let a = app();
    let target = win.as_weak();
    // Drop it after the current callback has returned.
    slint::Timer::single_shot(std::time::Duration::ZERO, move || {
        if let Some(w) = target.upgrade() {
            a.messages
                .borrow_mut()
                .retain(|m| !std::ptr::eq(m.window(), w.window()));
        }
    });
}
