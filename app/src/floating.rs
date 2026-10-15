//! The floating toolbar and the floating bubble.
//!
//! Both are created on demand and dropped when hidden, so an idle app holds no windows.
//! They hide themselves while a full-screen app (video, game, presentation) is in front.

use std::cell::{Cell, RefCell};
use std::time::Duration;

use qrtool_core::settings::ToolbarLayout;
use slint::winit_030::{winit, EventResult, WinitWindowAccessor as _};
use slint::ComponentHandle;

use crate::app::{app, dispatch, Action};
use crate::theme::{apply_theme, ThemeState};
use crate::ui;
use crate::winutil;

#[derive(Default)]
pub struct Floating {
    toolbar: RefCell<Option<ui::ToolbarWindow>>,
    bubble: RefCell<Option<ui::BubbleWindow>>,
    /// The user wants the window shown (it may be hidden temporarily).
    toolbar_wanted: Cell<bool>,
    bubble_wanted: Cell<bool>,
    /// Hidden because a full-screen app is in front.
    fullscreen_hidden: Cell<bool>,
    toolbar_pos: Cell<Option<(i32, i32)>>,
    bubble_pos: Cell<Option<(i32, i32)>>,
    /// Set after the window has been shown at its saved position. Later reveals (leaving
    /// a full-screen app) must not jump the window back.
    toolbar_placed: Cell<bool>,
    bubble_placed: Cell<bool>,
    poll: RefCell<Option<slint::Timer>>,
    /// The floating windows are invisible to screen capture (Windows), so they need not
    /// be hidden before a scan.
    pub excluded_from_capture: Cell<bool>,
}

pub fn init() {
    let a = app();
    let s = a.settings();
    a.floating.toolbar_pos.set(s.floating.toolbar_position);
    a.floating.bubble_pos.set(s.floating.bubble_position);
}

impl Floating {
    pub fn apply_theme(&self, t: ThemeState) {
        if let Some(w) = self.toolbar.borrow().as_ref() {
            apply_theme!(w, t);
        }
        if let Some(w) = self.bubble.borrow().as_ref() {
            apply_theme!(w, t);
        }
    }

    pub fn refresh_status(&self, camera: bool, hk_screen: &str, hk_region: &str, hk_camera: &str) {
        if let Some(w) = self.toolbar.borrow().as_ref() {
            w.set_camera_available(camera);
            w.set_hotkey_screen(hk_screen.into());
            w.set_hotkey_region(hk_region.into());
            w.set_hotkey_camera(hk_camera.into());
        }
        if let Some(w) = self.bubble.borrow().as_ref() {
            w.set_camera_available(camera);
        }
    }

    // ── toolbar / bubble ──
    //
    // The bubble is the collapsed toolbar, so the two are never shown together.
    // Close dismisses both; only Collapse (or the tray's bubble command) opens the bubble.

    pub fn toggle_toolbar(&self) {
        self.apply_mode(transition(self.mode(), "toggle-toolbar"));
    }

    pub fn show_toolbar(&self) {
        self.apply_mode(transition(self.mode(), "show-toolbar"));
    }

    /// Hides the toolbar and does not reveal the bubble.
    pub fn hide_toolbar(&self) {
        self.apply_mode(transition(self.mode(), "close"));
    }

    pub fn toggle_bubble(&self) {
        self.apply_mode(transition(self.mode(), "toggle-bubble"));
    }

    pub fn show_bubble(&self) {
        self.apply_mode(transition(self.mode(), "show-bubble"));
    }

    /// Switches from the toolbar to the bubble.
    pub fn collapse_to_bubble(&self) {
        self.apply_mode(transition(self.mode(), "collapse"));
    }

    pub fn hide_bubble(&self) {
        self.apply_mode(transition(self.mode(), "hide-bubble"));
    }

    fn mode(&self) -> FloatMode {
        if self.toolbar_wanted.get() {
            FloatMode::Toolbar
        } else if self.bubble_wanted.get() {
            FloatMode::Bubble
        } else {
            FloatMode::Hidden
        }
    }

    fn apply_mode(&self, mode: FloatMode) {
        tracing::debug!(?mode, "floating mode");
        self.toolbar_wanted.set(mode == FloatMode::Toolbar);
        self.bubble_wanted.set(mode == FloatMode::Bubble);
        match mode {
            FloatMode::Toolbar => {
                self.unmount_bubble();
                self.mount_toolbar();
            }
            FloatMode::Bubble => {
                self.unmount_toolbar();
                self.mount_bubble();
            }
            FloatMode::Hidden => {
                self.unmount_toolbar();
                self.unmount_bubble();
            }
        }
        self.after_change();
    }

    fn mount_toolbar(&self) {
        if self.toolbar.borrow().is_none() {
            let Ok(w) = ui::ToolbarWindow::new() else {
                tracing::error!("cannot create toolbar");
                return;
            };
            let a = app();
            apply_theme!(w, a.theme.get());
            // The layout is fixed for the life of the window; `layout_changed` rebuilds it.
            w.set_vertical(a.settings().floating.toolbar_layout.is_vertical());
            w.on_action(|id| toolbar_action(id.as_str()));
            let weak = w.as_weak();
            w.on_drag_start(move || {
                if let Some(w) = weak.upgrade() {
                    winutil::start_drag(w.window());
                }
            });
            w.window().on_close_requested(|| {
                slint::Timer::single_shot(Duration::ZERO, || app().floating.hide_toolbar());
                slint::CloseRequestResponse::KeepWindowShown
            });
            self.toolbar_placed.set(false);
            *self.toolbar.borrow_mut() = Some(w);
            a.refresh_status();
        }
        self.reveal_toolbar();
    }

    fn reveal_toolbar(&self) {
        if !self.toolbar_wanted.get() || self.fullscreen_hidden.get() {
            return;
        }
        let place = !self.toolbar_placed.get();
        let saved = self.toolbar_pos.get();
        let weak = {
            let guard = self.toolbar.borrow();
            let Some(w) = guard.as_ref() else {
                return;
            };
            let _ = w.show();
            w.as_weak()
        };
        if place {
            let placement = match app().settings().floating.toolbar_layout {
                ToolbarLayout::Horizontal => Placement::TopCenter,
                ToolbarLayout::VerticalRight => Placement::RightCenter,
                ToolbarLayout::VerticalLeft => Placement::LeftCenter,
            };
            if let Some(w) = weak.upgrade() {
                position_window(&w, saved, placement, |p| {
                    app().floating.toolbar_pos.set(Some(p));
                });
                self.toolbar_placed.set(true);
            }
        }
    }

    fn unmount_toolbar(&self) {
        self.toolbar_placed.set(false);
        if let Some(w) = self.toolbar.borrow_mut().take() {
            let _ = w.hide();
            slint::Timer::single_shot(Duration::ZERO, move || drop(w));
        }
    }

    fn mount_bubble(&self) {
        if self.bubble.borrow().is_none() {
            let Ok(w) = ui::BubbleWindow::new() else {
                tracing::error!("cannot create bubble");
                return;
            };
            let a = app();
            apply_theme!(w, a.theme.get());
            w.on_action(|id| bubble_action(id.as_str()));
            let weak = w.as_weak();
            w.on_drag_start(move || {
                if let Some(w) = weak.upgrade() {
                    winutil::start_drag(w.window());
                }
            });
            self.bubble_placed.set(false);
            *self.bubble.borrow_mut() = Some(w);
            a.refresh_status();
        }
        self.reveal_bubble();
    }

    fn reveal_bubble(&self) {
        if !self.bubble_wanted.get() || self.fullscreen_hidden.get() {
            return;
        }
        let place = !self.bubble_placed.get();
        let saved = self.bubble_pos.get();
        let weak = {
            let guard = self.bubble.borrow();
            let Some(w) = guard.as_ref() else {
                return;
            };
            let _ = w.show();
            w.as_weak()
        };
        if place {
            if let Some(w) = weak.upgrade() {
                position_window(&w, saved, Placement::RightEdge, |p| {
                    app().floating.bubble_pos.set(Some(p));
                });
                self.bubble_placed.set(true);
            }
        }
    }

    fn unmount_bubble(&self) {
        self.bubble_placed.set(false);
        if let Some(w) = self.bubble.borrow_mut().take() {
            let _ = w.hide();
            slint::Timer::single_shot(Duration::ZERO, move || drop(w));
        }
    }

    // ── capture & full-screen handling ──

    /// Hides both windows so they are not in the screenshot. Returns what was visible.
    pub fn hide_for_capture(&self) -> (bool, bool) {
        if self.excluded_from_capture.get() {
            return (false, false);
        }
        let toolbar = match self.toolbar.borrow().as_ref() {
            Some(w) if w.window().is_visible() => {
                let _ = w.hide();
                true
            }
            _ => false,
        };
        let bubble = match self.bubble.borrow().as_ref() {
            Some(w) if w.window().is_visible() => {
                let _ = w.hide();
                true
            }
            _ => false,
        };
        (toolbar, bubble)
    }

    pub fn restore_after_capture(&self, toolbar: bool, bubble: bool) {
        if self.fullscreen_hidden.get() {
            return;
        }
        if toolbar && self.toolbar_wanted.get() {
            if let Some(w) = self.toolbar.borrow().as_ref() {
                let _ = w.show();
            }
        }
        if bubble && self.bubble_wanted.get() {
            if let Some(w) = self.bubble.borrow().as_ref() {
                let _ = w.show();
            }
        }
    }

    pub fn save_positions(&self) {
        let (t, b) = (self.toolbar_pos.get(), self.bubble_pos.get());
        let a = app();
        let changed = {
            let s = a.settings();
            s.floating.toolbar_position != t || s.floating.bubble_position != b
        };
        if changed {
            a.update_settings(|s| {
                s.floating.toolbar_position = t;
                s.floating.bubble_position = b;
            });
        }
    }

    fn after_change(&self) {
        let a = app();
        a.tray.set_toolbar_visible(self.toolbar_wanted.get());
        a.tray.set_bubble_visible(self.bubble_wanted.get());
        self.save_positions();
        self.update_poll();
    }

    /// Polls for full-screen apps while a floating window is wanted (Windows only; other
    /// systems have no reliable API for this).
    fn update_poll(&self) {
        let needed = cfg!(windows)
            && (self.toolbar_wanted.get() || self.bubble_wanted.get())
            && app().settings().floating.hide_in_fullscreen;
        let mut poll = self.poll.borrow_mut();
        if !needed {
            if poll.take().is_some() && self.fullscreen_hidden.replace(false) {
                self.reshow();
            }
            return;
        }
        if poll.is_none() {
            let t = slint::Timer::default();
            t.start(slint::TimerMode::Repeated, Duration::from_millis(1000), || {
                app().floating.check_fullscreen();
            });
            *poll = Some(t);
        }
    }

    /// Re-applies the full-screen setting after it changed in Settings.
    pub fn settings_changed(&self) {
        self.update_poll();
    }

    /// The toolbar switched between row and column. The old position belongs to the old
    /// shape, so the toolbar goes back to the default spot of the new layout; an open
    /// toolbar is rebuilt there right away.
    pub fn layout_changed(&self) {
        self.toolbar_pos.set(None);
        if self.toolbar.borrow().is_some() {
            self.unmount_toolbar();
            if self.toolbar_wanted.get() {
                self.mount_toolbar();
            }
        }
        self.save_positions();
    }

    fn check_fullscreen(&self) {
        let a = app();
        if a.overlay.borrow().is_some() {
            return;
        }
        let fs = qrtool_platform::fullscreen::foreground_is_fullscreen();
        if fs && !self.fullscreen_hidden.get() {
            self.fullscreen_hidden.set(true);
            if let Some(w) = self.toolbar.borrow().as_ref() {
                let _ = w.hide();
            }
            if let Some(w) = self.bubble.borrow().as_ref() {
                let _ = w.hide();
            }
        } else if !fs && self.fullscreen_hidden.get() {
            self.fullscreen_hidden.set(false);
            self.reshow();
        }
    }

    fn reshow(&self) {
        self.reveal_toolbar();
        self.reveal_bubble();
    }
}

/// Which floating surface is up. The toolbar and the bubble never share the screen.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum FloatMode {
    Hidden,
    Toolbar,
    Bubble,
}

/// Next surface for a toolbar, bubble, tray or shortcut command.
fn transition(current: FloatMode, command: &str) -> FloatMode {
    match command {
        "show-toolbar" => FloatMode::Toolbar,
        // Close dismisses the floating UI. It must not uncover a bubble.
        "close" | "hide-toolbar" => FloatMode::Hidden,
        "collapse" | "show-bubble" => FloatMode::Bubble,
        "hide-bubble" => match current {
            FloatMode::Toolbar => FloatMode::Toolbar,
            FloatMode::Bubble | FloatMode::Hidden => FloatMode::Hidden,
        },
        "toggle-toolbar" => match current {
            FloatMode::Toolbar => FloatMode::Hidden,
            FloatMode::Bubble | FloatMode::Hidden => FloatMode::Toolbar,
        },
        "toggle-bubble" => match current {
            FloatMode::Bubble => FloatMode::Hidden,
            FloatMode::Toolbar | FloatMode::Hidden => FloatMode::Bubble,
        },
        _ => current,
    }
}

fn toolbar_action(id: &str) {
    tracing::debug!(id, "toolbar action");
    let action = match id {
        "scan-screen" => Action::ScanScreen,
        "scan-region" => Action::ScanRegion,
        "camera" => Action::ScanCamera,
        "clipboard" => Action::ScanClipboard,
        "image" => Action::OpenImage,
        "generate" => Action::Generate(None),
        "history" => Action::History,
        "settings" => Action::Settings,
        // "?": the PDF manual in the browser.
        "help" => Action::Manual,
        // Bubble icon: leave the toolbar and show only the bubble.
        "collapse" => Action::CollapseToBubble,
        // X: dismiss the toolbar. Do not switch back to the bubble.
        "close" => {
            app().floating.hide_toolbar();
            return;
        }
        other => {
            tracing::warn!("unknown toolbar action {other}");
            return;
        }
    };
    dispatch(action);
}

fn bubble_action(id: &str) {
    tracing::debug!(id, "bubble action");
    let action = match id {
        "scan-screen" => Action::ScanScreen,
        "scan-region" => Action::ScanRegion,
        "camera" => Action::ScanCamera,
        "clipboard" => Action::ScanClipboard,
        // The context menu is still closing on this window. Switching on the next
        // turn actually drops the bubble; doing it inline leaves it on screen, and
        // the toolbar's Close then looks like "back to the bubble".
        "toolbar" => {
            slint::Timer::single_shot(Duration::ZERO, || dispatch(Action::ShowToolbar));
            return;
        }
        "hide" => {
            slint::Timer::single_shot(Duration::ZERO, || dispatch(Action::HideBubble));
            return;
        }
        other => {
            tracing::warn!("unknown bubble action {other}");
            return;
        }
    };
    dispatch(action);
}

#[derive(Clone, Copy)]
enum Placement {
    /// Horizontal toolbar.
    TopCenter,
    /// Bubble: right edge, a little below the middle.
    RightEdge,
    /// Vertical toolbar on the right.
    RightCenter,
    /// Vertical toolbar on the left.
    LeftCenter,
}

/// Moves a freshly shown window to its saved position (if still on a monitor) or a
/// default spot, keeps it out of the taskbar and remembers where the user drags it.
fn position_window<C: ComponentHandle + 'static>(
    component: &C,
    saved: Option<(i32, i32)>,
    placement: Placement,
    on_moved: impl Fn((i32, i32)) + 'static,
) {
    winutil::when_created(component, move |c, w| {
        winutil::set_tool_window(w);
        let excluded = winutil::exclude_from_capture(w);
        app().floating.excluded_from_capture.set(excluded);
        // Attach before moving so the clamped position is the one we remember.
        c.window().on_winit_window_event(move |_, event| {
            if let winit::event::WindowEvent::Moved(p) = event {
                on_moved((p.x, p.y));
            }
            EventResult::Propagate
        });
        let pos = saved
            .filter(|&(x, y)| winutil::position_is_visible(w, x, y))
            .or_else(|| default_position(w, placement));
        if let Some((x, y)) = pos {
            let (x, y) = clamp_window_origin(w, x, y);
            w.set_outer_position(winit::dpi::PhysicalPosition::new(x, y));
        }
        // The first size can be the pre-layout default. Re-clamp once the real size
        // is known so the bubble and close buttons stay on the monitor.
        let weak = c.as_weak();
        slint::Timer::single_shot(Duration::from_millis(50), move || {
            let Some(c) = weak.upgrade() else {
                return;
            };
            c.window().with_winit_window(|w| {
                let Ok(pos) = w.outer_position() else {
                    return;
                };
                let (x, y) = clamp_window_origin(w, pos.x, pos.y);
                if x != pos.x || y != pos.y {
                    w.set_outer_position(winit::dpi::PhysicalPosition::new(x, y));
                }
            });
        });
    });
}

/// Pulls a window fully inside the monitor that contains its center.
fn clamp_window_origin(w: &winit::window::Window, x: i32, y: i32) -> (i32, i32) {
    let size = w.outer_size();
    let win_w = size.width as i32;
    let win_h = size.height as i32;
    let cx = x.saturating_add(win_w / 2);
    let cy = y.saturating_add(win_h / 2);
    let mon = w
        .available_monitors()
        .find(|m| {
            let p = m.position();
            let s = m.size();
            cx >= p.x && cy >= p.y && cx < p.x + s.width as i32 && cy < p.y + s.height as i32
        })
        .or_else(|| w.current_monitor());
    let Some(m) = mon else {
        return (x, y);
    };
    let p = m.position();
    let s = m.size();
    clamp_to_rect(x, y, win_w, win_h, p.x, p.y, s.width as i32, s.height as i32)
}

/// Keeps the whole window inside a monitor rectangle. A window wider than the monitor
/// is pinned to the monitor's origin so the leading edge stays reachable.
#[allow(clippy::too_many_arguments)]
fn clamp_to_rect(
    x: i32,
    y: i32,
    win_w: i32,
    win_h: i32,
    mon_x: i32,
    mon_y: i32,
    mon_w: i32,
    mon_h: i32,
) -> (i32, i32) {
    let max_x = mon_x + mon_w - win_w;
    let max_y = mon_y + mon_h - win_h;
    let x = if max_x >= mon_x {
        x.clamp(mon_x, max_x)
    } else {
        mon_x
    };
    let y = if max_y >= mon_y {
        y.clamp(mon_y, max_y)
    } else {
        mon_y
    };
    (x, y)
}

fn default_position(w: &winit::window::Window, placement: Placement) -> Option<(i32, i32)> {
    let m = winutil::monitor_of(w)?;
    let size = w.outer_size();
    let scale = w.scale_factor();
    let margin = (24.0 * scale) as i32;
    Some(match placement {
        Placement::TopCenter => (m.x + (m.width as i32 - size.width as i32) / 2, m.y + margin),
        Placement::RightEdge => (
            m.x + m.width as i32 - size.width as i32 - margin,
            m.y + (m.height as i32) * 3 / 5,
        ),
        Placement::RightCenter => (
            m.x + m.width as i32 - size.width as i32 - margin,
            m.y + (m.height as i32 - size.height as i32) / 2,
        ),
        Placement::LeftCenter => (m.x + margin, m.y + (m.height as i32 - size.height as i32) / 2),
    })
}

#[cfg(test)]
mod tests {
    use super::{clamp_to_rect, transition, FloatMode};

    #[test]
    fn close_never_returns_to_the_bubble() {
        for mode in [FloatMode::Hidden, FloatMode::Toolbar, FloatMode::Bubble] {
            assert_eq!(transition(mode, "close"), FloatMode::Hidden);
            assert_eq!(transition(mode, "hide-toolbar"), FloatMode::Hidden);
        }
    }

    #[test]
    fn collapse_and_toolbar_are_different_modes() {
        assert_eq!(transition(FloatMode::Toolbar, "collapse"), FloatMode::Bubble);
        assert_eq!(transition(FloatMode::Bubble, "show-toolbar"), FloatMode::Toolbar);
        assert_eq!(transition(FloatMode::Toolbar, "show-bubble"), FloatMode::Bubble);
        assert_eq!(transition(FloatMode::Bubble, "hide-bubble"), FloatMode::Hidden);
        assert_eq!(
            transition(FloatMode::Toolbar, "toggle-toolbar"),
            FloatMode::Hidden
        );
        assert_eq!(
            transition(FloatMode::Bubble, "toggle-toolbar"),
            FloatMode::Toolbar
        );
        assert_eq!(transition(FloatMode::Toolbar, "toggle-bubble"), FloatMode::Bubble);
        assert_eq!(transition(FloatMode::Bubble, "toggle-bubble"), FloatMode::Hidden);
    }

    #[test]
    fn toolbar_actions_cover_every_button() {
        let src = include_str!("../ui/floating.slint");
        for id in [
            "scan-screen",
            "scan-region",
            "camera",
            "clipboard",
            "image",
            "generate",
            "history",
            "settings",
            "help",
            "collapse",
            "close",
        ] {
            assert!(
                src.contains(&format!("{{ id: \"{id}\",")),
                "toolbar item {id} missing"
            );
        }
        assert!(
            src.contains("{ id: \"help\", icon: Icons.help,"),
            "help uses the ? icon"
        );
    }

    #[test]
    fn clamp_keeps_the_trailing_buttons_on_screen() {
        // 420px toolbar shoved past the right edge of a 1920px monitor.
        assert_eq!(clamp_to_rect(1800, 40, 420, 80, 0, 0, 1920, 1080), (1500, 40));
        assert_eq!(clamp_to_rect(100, 40, 420, 80, 0, 0, 1920, 1080), (100, 40));
        // Wider than the monitor: keep the leading edge on screen.
        assert_eq!(clamp_to_rect(50, -20, 2000, 80, 0, 0, 1920, 1080), (0, 0));
    }

    #[test]
    fn toolbar_bubble_and_close_buttons_are_distinct() {
        let src = include_str!("../ui/floating.slint");
        let start = src.find("export component ToolbarWindow").unwrap();
        let end = src.find("export component BubbleWindow").unwrap();
        let bar = &src[start..end];
        // Both layouts draw the same item list, so collapse and close stay distinct there.
        assert!(bar.contains("{ id: \"collapse\", icon: Icons.bubble,"));
        assert!(bar.contains("{ id: \"close\", icon: Icons.close,"));
        let collapse = bar.find("id: \"collapse\"").expect("collapse item");
        let close = bar.find("id: \"close\"").expect("close item");
        assert!(collapse < close, "close is the last button");
        assert_eq!(
            bar.matches("root.action(item.id)").count(),
            2,
            "row and column send the item id"
        );
        assert!(!bar.contains("width: layout.preferred-width"));
        assert!(bar.contains("min-width: root.vertical ? 64px : 500px;"));

        let menu = &src[end..];
        for id in [
            "scan-screen",
            "scan-region",
            "camera",
            "clipboard",
            "toolbar",
            "hide",
        ] {
            assert!(
                menu.contains(&format!("root.action(\"{id}\")")),
                "bubble menu missing {id}"
            );
        }
        assert!(menu.contains("width: 48px"), "bubble should be the smaller size");
        let clicked = menu.find("clicked =>").expect("bubble click");
        let clicked_body = &menu[clicked..menu.len().min(clicked + 220)];
        assert!(
            clicked_body.contains("root.action(\"toolbar\")"),
            "a click on the bubble opens the toolbar"
        );
        assert!(
            !clicked_body.contains("scan-screen"),
            "a click on the bubble must not scan"
        );
    }
}
