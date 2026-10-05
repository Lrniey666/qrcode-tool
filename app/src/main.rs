//! QRcode tool — scan QR codes on your computer as easily as on your phone.
//!
//! The binary is both the tray application and a small command-line tool:
//! `qrcode-tool scan-screen` forwards a command to the running instance (useful for
//! Wayland desktops where global shortcuts must be bound in the system settings), and
//! `qrcode-tool decode <files>` decodes images without starting the UI.

// Release builds on Windows are GUI applications (no console window).
#![cfg_attr(all(windows, not(debug_assertions)), windows_subsystem = "windows")]
// The thread-locals are already `const`; clippy misreports them on the GNU toolchain, whose
// std uses OS-based TLS.
#![cfg_attr(
    all(windows, target_env = "gnu"),
    allow(clippy::missing_const_for_thread_local)
)]

mod app;
mod cli;
mod detail;
mod floating;
mod generator;
mod hold;
mod hotkeys;
mod i18n;
mod logging;
mod message;
mod overlay;
mod preview;
mod scan;
mod settings_ui;
mod theme;
mod tray;
mod updater;
mod welcome;
mod winutil;

/// Generated Slint components.
#[allow(clippy::all, clippy::pedantic)]
pub mod ui {
    slint::include_modules!();
}

use std::process::ExitCode;

use clap::Parser;
use qrtool_platform::ipc;

fn main() -> ExitCode {
    // Velopack must run first: during install/update/uninstall it handles its own
    // command-line arguments and may exit the process.
    updater::startup_hook();

    // The local time offset must be read while the process is still single-threaded.
    app::init_local_offset();

    #[cfg(windows)]
    winutil::attach_parent_console();

    let cli = cli::Cli::parse();

    if let Some(cli::Cmd::Decode { files, json }) = &cli.command {
        return cli::decode_files(files, *json);
    }

    let command = cli.command.as_ref().map(cli::Cmd::to_ipc);

    // Single instance: hand the command to the running copy and leave.
    let forwarded = command.clone().unwrap_or(ipc::Command::Activate);
    if ipc::send_to_running(&forwarded) {
        return ExitCode::SUCCESS;
    }
    if matches!(command, Some(ipc::Command::Quit)) {
        // Nothing to quit.
        return ExitCode::SUCCESS;
    }

    let _log_guard = logging::init();
    tracing::info!(version = qrtool_core::app_info::VERSION, "starting");

    match app::run(cli.autostart, command) {
        Ok(()) => ExitCode::SUCCESS,
        Err(err) => {
            tracing::error!("fatal: {err:#}");
            eprintln!("qrcode-tool: {err:#}");
            ExitCode::FAILURE
        }
    }
}
