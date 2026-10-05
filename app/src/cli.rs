//! Command-line interface.

use std::path::PathBuf;
use std::process::ExitCode;

use clap::{Parser, Subcommand};
use qrtool_core::payload::classify;
use qrtool_decode::{scan_image, DecodeOptions};
use qrtool_platform::ipc;

#[derive(Debug, Parser)]
#[command(
    name = "qrcode-tool",
    version,
    about = "Scan QR codes on your computer as easily as on your phone.",
    long_about = "Scan QR codes on your computer as easily as on your phone.\n\n\
        Without a command the app starts in the system tray (or, if it is already running, \
        shows its toolbar). Commands are forwarded to the running instance, so you can bind \
        them to keyboard shortcuts in your desktop settings."
)]
pub struct Cli {
    /// Started by the operating system at login: stay hidden in the tray.
    #[arg(long, hide = true)]
    pub autostart: bool,

    #[command(subcommand)]
    pub command: Option<Cmd>,
}

#[derive(Debug, Clone, Subcommand)]
pub enum Cmd {
    /// Scan the whole screen (same as the global shortcut).
    ScanScreen,
    /// Freeze the screen and drag a box around one code.
    ScanRegion,
    /// Scan with the webcam.
    ScanCamera,
    /// Scan an image on the clipboard.
    ScanClipboard,
    /// Scan an image file in the preview window.
    ScanFile {
        /// Image file (PNG, JPEG, BMP, GIF, WebP).
        path: PathBuf,
    },
    /// Show or hide the floating toolbar.
    Toolbar,
    /// Show the floating toolbar.
    ShowToolbar,
    /// Show or hide the floating bubble.
    Bubble,
    /// Create a QR code.
    Generate {
        /// Text or link to encode.
        text: Option<String>,
    },
    /// Open the settings.
    Settings,
    /// Quit the running instance.
    Quit,
    /// Decode image files and print the results (does not start the UI).
    Decode {
        /// Image files (PNG, JPEG, BMP, GIF, WebP).
        #[arg(required = true)]
        files: Vec<PathBuf>,
        /// Print JSON instead of plain text.
        #[arg(long)]
        json: bool,
    },
}

impl Cmd {
    pub fn to_ipc(&self) -> ipc::Command {
        match self {
            Cmd::ScanScreen => ipc::Command::ScanScreen,
            Cmd::ScanRegion => ipc::Command::ScanRegion,
            Cmd::ScanCamera => ipc::Command::ScanCamera,
            Cmd::ScanClipboard => ipc::Command::ScanClipboard,
            Cmd::ScanFile { path } => ipc::Command::ScanFile {
                path: path.to_str().unwrap_or("").to_string(),
            },
            Cmd::Toolbar => ipc::Command::Toolbar,
            Cmd::ShowToolbar => ipc::Command::ShowToolbar,
            Cmd::Bubble => ipc::Command::Bubble,
            Cmd::Generate { text } => ipc::Command::Generate { text: text.clone() },
            Cmd::Settings => ipc::Command::Settings,
            Cmd::Quit => ipc::Command::Quit,
            // Never forwarded; handled in `main`.
            Cmd::Decode { .. } => ipc::Command::Activate,
        }
    }
}

/// `qrcode-tool decode`: prints every code found in the given files.
/// Exit code 0 if at least one code was found, 1 if none, 2 on errors.
pub fn decode_files(files: &[PathBuf], json: bool) -> ExitCode {
    let mut found = 0usize;
    let mut failed = false;
    let mut out = Vec::new();
    for path in files {
        let img = match image::open(path) {
            Ok(img) => img.to_rgba8(),
            Err(err) => {
                eprintln!("{}: {err}", path.display());
                failed = true;
                continue;
            }
        };
        for det in scan_image(&img, DecodeOptions::default()) {
            found += 1;
            let payload = classify(&det.text, Some(&det.bytes));
            if json {
                out.push(serde_json::json!({
                    "file": path.display().to_string(),
                    "format": det.format,
                    "kind": payload.kind().id(),
                    "text": det.text,
                    "position": det.quad.iter().map(|p| [p.x, p.y]).collect::<Vec<_>>(),
                }));
            } else if files.len() > 1 {
                println!("{}: {}", path.display(), det.text);
            } else {
                println!("{}", det.text);
            }
        }
    }
    if json {
        println!(
            "{}",
            serde_json::to_string_pretty(&out).unwrap_or_else(|_| "[]".into())
        );
    }
    if failed {
        ExitCode::from(2)
    } else if found == 0 {
        ExitCode::from(1)
    } else {
        ExitCode::SUCCESS
    }
}
