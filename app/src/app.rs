//! Application state and the central dispatcher.
//!
//! Everything UI-related runs on the Slint event loop thread. State lives in a
//! thread-local `Rc<App>`; background threads hand results back with
//! `slint::invoke_from_event_loop`.

use std::cell::{Cell, Ref, RefCell};
use std::rc::Rc;
use std::sync::OnceLock;
use std::time::Duration;

use anyhow::Context as _;
use qrtool_core::settings::Settings;
use qrtool_platform::{autostart, ipc, paths, shell_menu};

use crate::theme::{apply_theme, ThemeState};
use crate::{floating, generator, hotkeys, i18n, overlay, preview, settings_ui, tray, ui, updater, welcome};

thread_local! {
    static APP: RefCell<Option<Rc<App>>> = const { RefCell::new(None) };
}

/// The running application. Panics if called before [`run`] set it up.
pub fn app() -> Rc<App> {
    APP.with(|a| a.borrow().clone())
        .expect("application not initialised")
}

/// Things the user can ask for, from the tray, hotkeys, toolbar, bubble or IPC.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Action {
    ScanScreen,
    ScanRegion,
    ScanCamera,
    ScanClipboard,
    OpenImage,
    /// Scan a picture the user already chose (right-click menu or `scan-file`).
    OpenImageFile(std::path::PathBuf),
    Generate(Option<String>),
    ToggleToolbar,
    ShowToolbar,
    ToggleBubble,
    CollapseToBubble,
    HideBubble,
    Settings,
    History,
    /// Open the PDF user manual in the browser.
    Manual,
    CheckUpdates,
    Quit,
}

impl From<ipc::Command> for Action {
    fn from(cmd: ipc::Command) -> Self {
        match cmd {
            ipc::Command::Activate => Action::ShowToolbar,
            ipc::Command::ScanScreen => Action::ScanScreen,
            ipc::Command::ScanRegion => Action::ScanRegion,
            ipc::Command::ScanCamera => Action::ScanCamera,
            ipc::Command::ScanClipboard => Action::ScanClipboard,
            ipc::Command::ScanFile { path } => Action::OpenImageFile(std::path::PathBuf::from(path)),
            ipc::Command::Toolbar => Action::ToggleToolbar,
            ipc::Command::ShowToolbar => Action::ShowToolbar,
            ipc::Command::Bubble => Action::ToggleBubble,
            ipc::Command::Generate { text } => Action::Generate(text),
            ipc::Command::Settings => Action::Settings,
            ipc::Command::Quit => Action::Quit,
        }
    }
}

/// Camera availability, discovered in the background.
#[derive(Debug, Clone, Default)]
pub struct CameraInfo {
    pub probed: bool,
    pub names: Vec<String>,
}

pub struct App {
    settings: RefCell<Settings>,
    pub theme: Cell<ThemeState>,
    pub tray: ui::AppTray,
    pub hotkeys: RefCell<hotkeys::HotkeyService>,
    pub cameras: RefCell<CameraInfo>,

    pub overlay: RefCell<Option<overlay::Session>>,
    pub preview: RefCell<Option<preview::Preview>>,
    pub floating: floating::Floating,
    pub settings_win: RefCell<Option<settings_ui::SettingsUi>>,
    pub generator: RefCell<Option<generator::Generator>>,
    pub welcome: RefCell<Option<ui::WelcomeWindow>>,
    pub messages: RefCell<Vec<ui::MessageWindow>>,

    timers: RefCell<Vec<slint::Timer>>,
}

impl App {
    pub fn settings(&self) -> Ref<'_, Settings> {
        self.settings.borrow()
    }

    /// Changes settings and saves them.
    pub fn update_settings(&self, f: impl FnOnce(&mut Settings)) {
        let snapshot = {
            let mut s = self.settings.borrow_mut();
            f(&mut s);
            s.sanitize();
            s.clone()
        };
        if let Err(e) = snapshot.save(&paths::settings_file()) {
            tracing::error!("cannot save settings: {e}");
        }
    }

    pub fn camera_available(&self) -> bool {
        if !cfg!(feature = "camera") {
            return false;
        }
        let c = self.cameras.borrow();
        // Until probing finished, assume there is one so the buttons are not greyed out.
        !c.probed || !c.names.is_empty()
    }

    /// Re-reads the system theme and pushes it into every open window.
    pub fn refresh_theme(&self) {
        let state = ThemeState::detect(self.settings().theme);
        if state == self.theme.get() {
            return;
        }
        self.theme.set(state);
        self.apply_theme_everywhere();
    }

    pub fn apply_theme_everywhere(&self) {
        let t = self.theme.get();
        apply_theme!(self.tray, t);
        if let Some(o) = self.overlay.borrow().as_ref() {
            o.apply_theme(t);
        }
        if let Some(p) = self.preview.borrow().as_ref() {
            apply_theme!(p.ui, t);
        }
        self.floating.apply_theme(t);
        if let Some(s) = self.settings_win.borrow().as_ref() {
            apply_theme!(s.ui, t);
        }
        if let Some(g) = self.generator.borrow().as_ref() {
            apply_theme!(g.ui, t);
        }
        if let Some(w) = self.welcome.borrow().as_ref() {
            apply_theme!(w, t);
        }
        for m in self.messages.borrow().iter() {
            apply_theme!(m, t);
        }
    }

    /// Updates everything that shows camera availability or hotkeys.
    pub fn refresh_status(&self) {
        let cam = self.camera_available();
        let (hk_screen, hk_region, hk_camera) = {
            let s = self.settings();
            (
                hotkeys::display(&s.hotkeys.scan_screen),
                hotkeys::display(&s.hotkeys.scan_region),
                hotkeys::display(&s.hotkeys.scan_camera),
            )
        };
        let supported = hotkeys::supported();
        let shown = |v: &str| if supported { v.to_string() } else { String::new() };
        self.tray.set_camera_available(cam);
        self.tray.set_hotkey_screen(shown(&hk_screen).into());
        self.tray.set_hotkey_camera(shown(&hk_camera).into());
        self.floating
            .refresh_status(cam, &shown(&hk_screen), &shown(&hk_region), &shown(&hk_camera));
        if let Some(s) = self.settings_win.borrow().as_ref() {
            s.refresh_cameras();
        }
    }

    /// Texts that Rust needs (dialog titles, …), translated.
    pub fn tr(&self, key: &str) -> String {
        self.tray.global::<ui::Strings>().invoke_t(key.into()).to_string()
    }
}

/// Runs one action.
pub fn dispatch(action: Action) {
    if matches!(action, Action::OpenImageFile(_)) {
        // The path is a local file name, not a code. Keep it out of the log.
        tracing::debug!("dispatch open-image-file");
    } else {
        tracing::debug!(?action, "dispatch");
    }
    let a = app();
    match action {
        Action::ScanScreen => overlay::start_screen(),
        Action::ScanRegion => overlay::start_region(),
        Action::ScanCamera => {
            // The camera window does not replace the toolbar; the shortcut should
            // leave the full toolbar on screen as well.
            a.floating.show_toolbar();
            preview::open_camera();
        }
        Action::ScanClipboard => preview::scan_clipboard(),
        Action::OpenImage => preview::pick_image(),
        Action::OpenImageFile(path) => preview::open_image_file(path),
        Action::Generate(text) => generator::open(text),
        Action::ToggleToolbar => a.floating.toggle_toolbar(),
        Action::ShowToolbar => a.floating.show_toolbar(),
        Action::ToggleBubble => a.floating.toggle_bubble(),
        Action::CollapseToBubble => a.floating.collapse_to_bubble(),
        Action::HideBubble => a.floating.hide_bubble(),
        Action::Settings => settings_ui::open(None),
        Action::History => settings_ui::open(Some(4)),
        Action::Manual => open_manual(),
        Action::CheckUpdates => updater::check(true),
        Action::Quit => quit(),
    }
}

/// Opens the PDF manual on GitHub in the language of the UI (a new browser tab).
fn open_manual() {
    let language = i18n::effective_language(&app().settings().language);
    let url = qrtool_core::app_info::manual_url(language);
    match qrtool_core::url_safety::analyze_url(&url) {
        Ok(report) => {
            if let Err(e) = qrtool_platform::open::open_web(&report) {
                crate::message::show("open-failed", &e.to_string(), 3);
            }
        }
        Err(e) => tracing::warn!("manual link rejected: {e}"),
    }
}

/// Writes or removes the Windows right-click menu so it matches settings and language.
pub fn sync_shell_menu() {
    let a = app();
    let enabled = a.settings().shell_menu;
    if !enabled {
        if !shell_menu::remove() {
            tracing::warn!("could not remove the right-click menu");
        }
        return;
    }
    let labels = shell_menu::Labels {
        scan_screen: a.tr("shell-scan-screen"),
        scan_region: a.tr("shell-scan-region"),
        scan_camera: a.tr("shell-scan-camera"),
        toolbar: a.tr("shell-toolbar"),
        open_image: a.tr("shell-open-image"),
    };
    if !shell_menu::install(&labels) {
        tracing::warn!("could not add the right-click menu");
    }
}

/// Saves state and leaves the event loop.
pub fn quit() {
    let a = app();
    a.floating.save_positions();
    if let Some(p) = a.preview.borrow_mut().take() {
        p.close_silently();
    }
    a.overlay.borrow_mut().take();
    let _ = slint::quit_event_loop();
}

/// UTC offset of the local time zone, read once at start-up while single-threaded.
static LOCAL_OFFSET: OnceLock<time::UtcOffset> = OnceLock::new();

pub fn init_local_offset() {
    let off = time::UtcOffset::current_local_offset().unwrap_or(time::UtcOffset::UTC);
    let _ = LOCAL_OFFSET.set(off);
}

pub fn local_offset() -> time::UtcOffset {
    LOCAL_OFFSET.get().copied().unwrap_or(time::UtcOffset::UTC)
}

fn select_backend(renderer: &str) -> anyhow::Result<()> {
    #[allow(unused_mut)]
    let mut selector = slint::BackendSelector::new().backend_name("winit".into());
    // Windows: the software renderer is the default. With OpenGL the full-screen overlay
    // stopped repainting on some GPU drivers and took ~0.8 s to appear; the software
    // renderer shows it in ~0.1 s and also uses less memory. `renderer = "femtovg"` in
    // settings.toml switches back.
    let renderer = if renderer.is_empty() && cfg!(windows) {
        "software"
    } else {
        renderer
    };
    if !renderer.is_empty() {
        tracing::info!("renderer: {renderer}");
        selector = selector.renderer_name(renderer.into());
    }
    #[cfg(target_os = "macos")]
    {
        use slint::winit_030::winit::event_loop::EventLoop;
        use slint::winit_030::winit::platform::macos::{ActivationPolicy, EventLoopBuilderExtMacOS};
        use slint::winit_030::SlintEvent;
        // A menu-bar app: no Dock icon, no app menu.
        let mut builder = EventLoop::<SlintEvent>::with_user_event();
        builder.with_activation_policy(ActivationPolicy::Accessory);
        selector = selector.with_winit_event_loop_builder(builder);
    }
    if selector.select().is_ok() {
        return Ok(());
    }
    // No OpenGL (VMs, remote desktops): fall back to the software renderer.
    tracing::warn!("default renderer unavailable, using the software renderer");
    slint::BackendSelector::new()
        .backend_name("winit".into())
        .renderer_name("software".into())
        .select()
        .context("cannot initialise the window system")
}

/// Starts the UI and runs until the user quits.
pub fn run(autostart_launch: bool, command: Option<ipc::Command>) -> anyhow::Result<()> {
    let settings = Settings::load(&paths::settings_file());
    // Write back once so migrations (see `Settings::sanitize`) are stored.
    if paths::settings_file().exists() {
        if let Err(e) = settings.save(&paths::settings_file()) {
            tracing::warn!("cannot save settings: {e}");
        }
    }
    select_backend(&settings.renderer)?;

    let theme = ThemeState::detect(settings.theme);

    // The tray is the first component; translations can be selected after it exists.
    let tray = ui::AppTray::new().context("cannot create the tray icon")?;
    i18n::apply(&settings.language);

    let app = Rc::new(App {
        settings: RefCell::new(settings),
        theme: Cell::new(theme),
        tray,
        hotkeys: RefCell::new(hotkeys::HotkeyService::new()),
        cameras: RefCell::new(CameraInfo::default()),
        overlay: RefCell::new(None),
        preview: RefCell::new(None),
        floating: floating::Floating::default(),
        settings_win: RefCell::new(None),
        generator: RefCell::new(None),
        welcome: RefCell::new(None),
        messages: RefCell::new(Vec::new()),
        timers: RefCell::new(Vec::new()),
    });
    APP.with(|a| *a.borrow_mut() = Some(app.clone()));

    tray::init(&app);
    app.apply_theme_everywhere();
    hotkeys::init();
    start_ipc_server();
    sync_shell_menu();
    probe_cameras();
    start_timers(&app);
    floating::init();

    let first_run = !app.settings().first_run_completed;
    if first_run {
        // Autostart is on by default; the welcome screen tells the user and offers to
        // turn it off.
        let wanted = app.settings().autostart;
        autostart::set_enabled(wanted);
        welcome::open();
    } else {
        // Keep the OS entry in sync (e.g. after moving the program).
        let wanted = app.settings().autostart;
        if autostart::is_enabled() != wanted {
            autostart::set_enabled(wanted);
        }
        // The bubble is the collapsed toolbar, so startup shows at most one of them.
        let bubble = app.settings().floating.bubble_at_startup;
        if bubble {
            app.floating.show_bubble();
        } else if !autostart_launch && command.is_none() {
            // Started by hand with no bubble: show the toolbar so the user sees it is running.
            app.floating.show_toolbar();
        }
    }

    if let Some(cmd) = command {
        let action = Action::from(cmd);
        // Let the event loop start (and windows get created) first.
        slint::Timer::single_shot(Duration::from_millis(150), move || dispatch(action));
    }

    if app.settings().updates.check_automatically {
        // Wait a little after login so we do not slow the desktop down.
        let delay = if autostart_launch { 90 } else { 20 };
        slint::Timer::single_shot(Duration::from_secs(delay), || updater::check(false));
    }

    slint::run_event_loop_until_quit().context("event loop failed")?;

    tracing::info!("exiting");
    app.hotkeys.borrow_mut().unregister_all();
    app.timers.borrow_mut().clear();
    APP.with(|a| a.borrow_mut().take());
    Ok(())
}

fn start_ipc_server() {
    let res = ipc::start_server(|cmd| {
        let _ = slint::invoke_from_event_loop(move || dispatch(Action::from(cmd)));
    });
    if let Err(e) = res {
        tracing::warn!("single-instance server unavailable: {e}");
    }
}

/// Lists cameras in the background (it can take a moment on some systems).
pub fn probe_cameras() {
    #[cfg(feature = "camera")]
    std::thread::spawn(|| {
        let names = match qrtool_camera::list_cameras() {
            Ok(list) => list.into_iter().map(|c| c.name).collect(),
            Err(e) => {
                tracing::info!("no cameras: {e}");
                Vec::new()
            }
        };
        let _ = slint::invoke_from_event_loop(move || {
            let a = app();
            *a.cameras.borrow_mut() = CameraInfo { probed: true, names };
            a.refresh_status();
        });
    });
    #[cfg(not(feature = "camera"))]
    {
        let a = app();
        *a.cameras.borrow_mut() = CameraInfo {
            probed: true,
            names: Vec::new(),
        };
    }
}

fn start_timers(app_rc: &Rc<App>) {
    // Follow theme / accent changes of the system. Reading them is cheap.
    let theme_timer = slint::Timer::default();
    theme_timer.start(slint::TimerMode::Repeated, Duration::from_secs(3), || {
        app().refresh_theme();
    });
    app_rc.timers.borrow_mut().push(theme_timer);

    // Heartbeat in the debug log: shows whether the UI thread is alive when diagnosing.
    let heartbeat = slint::Timer::default();
    heartbeat.start(slint::TimerMode::Repeated, Duration::from_secs(30), || {
        tracing::debug!("ui alive");
    });
    app_rc.timers.borrow_mut().push(heartbeat);

    // Automatic update check once a day while running.
    let update_timer = slint::Timer::default();
    update_timer.start(slint::TimerMode::Repeated, Duration::from_secs(24 * 3600), || {
        if app().settings().updates.check_automatically {
            updater::check(false);
        }
    });
    app_rc.timers.borrow_mut().push(update_timer);
}
