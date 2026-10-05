//! Window helpers on top of Slint's winit backend: per-monitor full-screen overlays,
//! tool windows that stay out of the taskbar, and dragging borderless windows.

use slint::winit_030::{winit, WinitWindowAccessor};
use slint::ComponentHandle;

/// Where a screenshot came from, in physical pixels of the virtual desktop.
#[derive(Debug, Clone, Copy)]
pub struct MonitorRect {
    pub x: i32,
    pub y: i32,
    pub width: u32,
    pub height: u32,
}

/// Runs `f` with the winit window once it exists (windows are created lazily on `show()`).
pub fn when_created<C: ComponentHandle + 'static>(
    component: &C,
    f: impl FnOnce(&C, &winit::window::Window) + 'static,
) {
    let weak = component.as_weak();
    let _ = slint::spawn_local(async move {
        let Some(c) = weak.upgrade() else {
            return;
        };
        if let Ok(w) = c.window().winit_window().await {
            f(&c, &w);
        }
    });
}

/// Puts a window on `rect` before it is shown so it is created on the right monitor
/// (and therefore with the right scale factor).
pub fn place_on(window: &slint::Window, rect: MonitorRect) {
    window.set_position(slint::PhysicalPosition::new(rect.x, rect.y));
    window.set_size(slint::PhysicalSize::new(rect.width, rect.height));
}

/// Makes a shown window cover the monitor at `rect` (borderless full screen, above the
/// taskbar / menu bar) and takes the keyboard focus if `focus` is set.
pub fn make_overlay(w: &winit::window::Window, rect: MonitorRect, focus: bool) {
    tracing::debug!(?rect, focus, "make_overlay");
    set_tool_window(w);
    // No text input here; keeping the IME off stops it from eating keys like Esc.
    w.set_ime_allowed(false);

    #[cfg(target_os = "macos")]
    {
        use winit::platform::macos::WindowExtMacOS;
        w.set_outer_position(winit::dpi::PhysicalPosition::new(rect.x, rect.y));
        // "Simple" full screen: no new Space and no animation.
        w.set_simple_fullscreen(true);
    }
    #[cfg(windows)]
    {
        // A borderless, topmost window that exactly covers the monitor is enough on
        // Windows and avoids the taskbar/COM work winit does for real full screen.
        w.set_outer_position(winit::dpi::PhysicalPosition::new(rect.x, rect.y));
        let _ = w.request_inner_size(winit::dpi::PhysicalSize::new(rect.width, rect.height));
    }
    #[cfg(not(any(target_os = "macos", windows)))]
    {
        let monitor = w
            .available_monitors()
            .find(|m| {
                let p = m.position();
                p.x == rect.x && p.y == rect.y
            })
            .or_else(|| {
                let cx = rect.x + rect.width as i32 / 2;
                let cy = rect.y + rect.height as i32 / 2;
                w.available_monitors().find(|m| {
                    let p = m.position();
                    let s = m.size();
                    cx >= p.x && cy >= p.y && cx < p.x + s.width as i32 && cy < p.y + s.height as i32
                })
            });
        if monitor.is_some() {
            w.set_fullscreen(Some(winit::window::Fullscreen::Borderless(monitor)));
        } else {
            w.set_outer_position(winit::dpi::PhysicalPosition::new(rect.x, rect.y));
            let _ = w.request_inner_size(winit::dpi::PhysicalSize::new(rect.width, rect.height));
        }
    }
    if focus {
        bring_to_front(w);
    }
    tracing::debug!("make_overlay done");
}

/// Logs focus and keyboard events of a window at debug level (diagnostics).
pub fn trace_input_events(window: &slint::Window) {
    window.on_winit_window_event(|_, event| {
        use winit::event::WindowEvent;
        match event {
            WindowEvent::Focused(f) => tracing::debug!(focused = f, "winit focus"),
            WindowEvent::KeyboardInput { event, .. } => {
                tracing::debug!(key = ?event.logical_key, state = ?event.state, "winit key")
            }
            WindowEvent::Occluded(o) => tracing::debug!(occluded = o, "winit occluded"),
            WindowEvent::MouseInput { state, button, .. } => {
                tracing::debug!(?state, ?button, "winit mouse")
            }
            _ => {}
        }
        slint::winit_030::EventResult::Propagate
    });
}

/// Gives a window the keyboard focus. On Windows the foreground lock usually prevents a
/// background process from doing that, so the input queues are attached briefly (the
/// documented way for an app that was just activated by a hotkey or tray click).
pub fn bring_to_front(w: &winit::window::Window) {
    #[cfg(windows)]
    {
        use windows_sys::Win32::System::Threading::{AttachThreadInput, GetCurrentThreadId};
        use windows_sys::Win32::UI::Input::KeyboardAndMouse::{SetActiveWindow, SetFocus};
        use windows_sys::Win32::UI::WindowsAndMessaging::{
            BringWindowToTop, GetForegroundWindow, GetWindowThreadProcessId, SetForegroundWindow,
        };
        if let Some(hwnd) = hwnd_of(w) {
            // SAFETY: plain Win32 calls on a live window owned by this thread.
            unsafe {
                let fg = GetForegroundWindow();
                let fg_thread = GetWindowThreadProcessId(fg, std::ptr::null_mut());
                let me = GetCurrentThreadId();
                let attach = fg_thread != 0 && fg_thread != me;
                if attach {
                    AttachThreadInput(me, fg_thread, 1);
                }
                BringWindowToTop(hwnd);
                SetForegroundWindow(hwnd);
                SetActiveWindow(hwnd);
                SetFocus(hwnd);
                if attach {
                    AttachThreadInput(me, fg_thread, 0);
                }
                if GetForegroundWindow() != hwnd {
                    // Still locked: a synthetic Alt press lifts the foreground lock (the
                    // same technique AutoHotkey and PowerToys use), then try again.
                    alt_tap();
                    SetForegroundWindow(hwnd);
                    SetFocus(hwnd);
                }
                tracing::debug!(focused = GetForegroundWindow() == hwnd, "bring_to_front");
            }
            return;
        }
    }
    w.focus_window();
}

/// Presses and releases Alt (no visible effect when followed by a focus change).
#[cfg(windows)]
unsafe fn alt_tap() {
    use windows_sys::Win32::UI::Input::KeyboardAndMouse::{
        SendInput, INPUT, INPUT_0, INPUT_KEYBOARD, KEYBDINPUT, KEYEVENTF_EXTENDEDKEY, KEYEVENTF_KEYUP,
        VK_MENU,
    };
    let key = |flags| INPUT {
        r#type: INPUT_KEYBOARD,
        Anonymous: INPUT_0 {
            ki: KEYBDINPUT {
                wVk: VK_MENU,
                wScan: 0,
                dwFlags: flags,
                time: 0,
                dwExtraInfo: 0,
            },
        },
    };
    let inputs = [
        key(KEYEVENTF_EXTENDEDKEY),
        key(KEYEVENTF_EXTENDEDKEY | KEYEVENTF_KEYUP),
    ];
    SendInput(
        inputs.len() as u32,
        inputs.as_ptr(),
        std::mem::size_of::<INPUT>() as i32,
    );
}

/// Keeps a window out of screenshots (Windows 10 2004+), so floating windows do not have
/// to be hidden before the screen is captured.
pub fn exclude_from_capture(w: &winit::window::Window) -> bool {
    #[cfg(windows)]
    {
        use windows_sys::Win32::UI::WindowsAndMessaging::{SetWindowDisplayAffinity, WDA_EXCLUDEFROMCAPTURE};
        if let Some(hwnd) = hwnd_of(w) {
            // SAFETY: live window owned by this thread.
            return unsafe { SetWindowDisplayAffinity(hwnd, WDA_EXCLUDEFROMCAPTURE) } != 0;
        }
        false
    }
    #[cfg(not(windows))]
    {
        let _ = w;
        false
    }
}

#[cfg(windows)]
fn hwnd_of(w: &winit::window::Window) -> Option<windows_sys::Win32::Foundation::HWND> {
    use winit::raw_window_handle::{HasWindowHandle, RawWindowHandle};
    match w.window_handle().ok()?.as_raw() {
        RawWindowHandle::Win32(h) => Some(h.hwnd.get() as windows_sys::Win32::Foundation::HWND),
        _ => None,
    }
}

/// Keeps a window out of the taskbar and Alt+Tab (Windows). Other platforms: no-op —
/// on macOS the whole app runs as an accessory app without a Dock icon.
pub fn set_tool_window(w: &winit::window::Window) {
    #[cfg(windows)]
    {
        use windows_sys::Win32::UI::WindowsAndMessaging::{
            GetWindowLongPtrW, SetWindowLongPtrW, SetWindowPos, GWL_EXSTYLE, SWP_FRAMECHANGED,
            SWP_NOACTIVATE, SWP_NOMOVE, SWP_NOSIZE, SWP_NOZORDER, WS_EX_APPWINDOW, WS_EX_TOOLWINDOW,
        };
        if let Some(hwnd) = hwnd_of(w) {
            // SAFETY: `hwnd` is a live window owned by this thread.
            unsafe {
                let style = GetWindowLongPtrW(hwnd, GWL_EXSTYLE);
                let new = (style | WS_EX_TOOLWINDOW as isize) & !(WS_EX_APPWINDOW as isize);
                if new != style {
                    SetWindowLongPtrW(hwnd, GWL_EXSTYLE, new);
                    SetWindowPos(
                        hwnd,
                        std::ptr::null_mut(),
                        0,
                        0,
                        0,
                        0,
                        SWP_NOMOVE | SWP_NOSIZE | SWP_NOZORDER | SWP_NOACTIVATE | SWP_FRAMECHANGED,
                    );
                }
            }
        }
    }
    #[cfg(not(windows))]
    let _ = w;
}

/// Starts moving a borderless window with the mouse (call on mouse-down).
pub fn start_drag(window: &slint::Window) {
    window.with_winit_window(|w| {
        let _ = w.drag_window();
    });
}

/// The monitor that contains most of the window, as a rectangle in physical pixels.
pub fn monitor_of(w: &winit::window::Window) -> Option<MonitorRect> {
    let m = w.current_monitor().or_else(|| w.primary_monitor())?;
    let p = m.position();
    let s = m.size();
    Some(MonitorRect {
        x: p.x,
        y: p.y,
        width: s.width,
        height: s.height,
    })
}

/// Centres a normal window on the primary monitor once it exists. Window managers that
/// place windows themselves (most Linux desktops) may override this.
pub fn center_when_created<C: ComponentHandle + 'static>(component: &C) {
    when_created(component, |_, w| {
        let Some(m) = w.primary_monitor().or_else(|| w.current_monitor()) else {
            return;
        };
        let p = m.position();
        let s = m.size();
        let size = w.outer_size();
        let x = p.x + (s.width as i32 - size.width as i32) / 2;
        let y = p.y + (s.height as i32 - size.height as i32) / 3;
        w.set_outer_position(winit::dpi::PhysicalPosition::new(x.max(p.x), y.max(p.y)));
        bring_to_front(w);
    });
}

/// Whether a saved position is still on one of the connected monitors.
pub fn position_is_visible(w: &winit::window::Window, x: i32, y: i32) -> bool {
    w.available_monitors().any(|m| {
        let p = m.position();
        let s = m.size();
        x >= p.x - 16 && y >= p.y - 16 && x < p.x + s.width as i32 - 24 && y < p.y + s.height as i32 - 24
    })
}

/// Lets `qrcode-tool --help` and `qrcode-tool decode` print to the terminal they were
/// started from, even though release builds are GUI applications.
#[cfg(windows)]
pub fn attach_parent_console() {
    use windows_sys::Win32::System::Console::{AttachConsole, ATTACH_PARENT_PROCESS};
    if std::env::args_os().len() > 1 {
        // SAFETY: plain Win32 call without pointers.
        unsafe {
            AttachConsole(ATTACH_PARENT_PROCESS);
        }
    }
}
