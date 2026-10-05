//! Single-instance handling and command forwarding.
//!
//! The first instance listens on a per-user local socket (a named pipe on Windows, a Unix
//! domain socket elsewhere). Later invocations such as `qrcode-tool scan-screen` connect,
//! send one command and exit. This is also how users on Wayland can bind the app to their
//! desktop's own keyboard shortcuts.

use std::io::{BufRead, BufReader, Write};
use std::thread::JoinHandle;

use interprocess::local_socket::{prelude::*, GenericNamespaced, ListenerOptions, Name, Stream};
use serde::{Deserialize, Serialize};

/// Commands that can be sent to the running instance.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "cmd", rename_all = "kebab-case")]
pub enum Command {
    /// Bring up the app (shows the toolbar).
    Activate,
    ScanScreen,
    /// Freeze the screen and wait for a dragged region. Distinct from [`Command::ScanScreen`].
    ScanRegion,
    ScanCamera,
    ScanClipboard,
    /// Scan one image file and show it in the preview window.
    ScanFile {
        path: String,
    },
    Toolbar,
    /// Show the floating toolbar (does not hide it when it is already visible).
    ShowToolbar,
    Bubble,
    Generate {
        text: Option<String>,
    },
    Settings,
    Quit,
}

fn socket_name() -> std::io::Result<Name<'static>> {
    let user = std::env::var("USERNAME")
        .or_else(|_| std::env::var("USER"))
        .unwrap_or_else(|_| "user".into());
    let user: String = user
        .chars()
        .filter(|c| c.is_ascii_alphanumeric() || *c == '-' || *c == '_')
        .take(32)
        .collect();
    format!("{}-{}.sock", qrtool_core::app_info::BIN_NAME, user)
        .to_ns_name::<GenericNamespaced>()
        .map(|n| n.into_owned())
}

/// Tries to deliver `cmd` to an already running instance. Returns `true` if one answered.
pub fn send_to_running(cmd: &Command) -> bool {
    let Ok(name) = socket_name() else {
        return false;
    };
    let Ok(stream) = Stream::connect(name) else {
        return false;
    };
    let mut stream = BufReader::new(stream);
    let Ok(mut line) = serde_json::to_string(cmd) else {
        return false;
    };
    line.push('\n');
    if stream.get_mut().write_all(line.as_bytes()).is_err() {
        return false;
    }
    let mut reply = String::new();
    stream.read_line(&mut reply).is_ok() && reply.trim() == "ok"
}

/// Starts listening for commands. `on_command` is called on the listener thread.
///
/// Returns an error if the socket cannot be created (e.g. another instance owns it).
pub fn start_server(on_command: impl Fn(Command) + Send + 'static) -> std::io::Result<JoinHandle<()>> {
    let name = socket_name()?;
    let listener = ListenerOptions::new()
        .name(name)
        .try_overwrite(true)
        .create_sync()?;
    std::thread::Builder::new()
        .name("ipc-server".into())
        .spawn(move || {
            for conn in listener.incoming().filter_map(Result::ok) {
                let mut conn = BufReader::new(conn);
                let mut line = String::new();
                if conn.read_line(&mut line).is_err() || line.len() > 64 * 1024 {
                    continue;
                }
                let reply = match serde_json::from_str::<Command>(line.trim()) {
                    Ok(cmd) => {
                        on_command(cmd);
                        "ok\n"
                    }
                    Err(_) => "error\n",
                };
                let _ = conn.get_mut().write_all(reply.as_bytes());
            }
        })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn command_json_roundtrip() {
        let cmds = [
            Command::ScanScreen,
            Command::ScanRegion,
            Command::ShowToolbar,
            Command::ScanFile {
                path: r"C:\Pictures\code.png".into(),
            },
            Command::Generate {
                text: Some("https://example.com".into()),
            },
            Command::Quit,
        ];
        for c in cmds {
            let s = serde_json::to_string(&c).unwrap();
            assert_eq!(serde_json::from_str::<Command>(&s).unwrap(), c);
        }
        assert_eq!(
            serde_json::to_string(&Command::ScanScreen).unwrap(),
            r#"{"cmd":"scan-screen"}"#
        );
        assert_eq!(
            serde_json::to_string(&Command::ScanRegion).unwrap(),
            r#"{"cmd":"scan-region"}"#
        );
        assert_eq!(
            serde_json::to_string(&Command::ShowToolbar).unwrap(),
            r#"{"cmd":"show-toolbar"}"#
        );
        assert_eq!(
            serde_json::to_string(&Command::ScanFile {
                path: r"C:\Pictures\code.png".into(),
            })
            .unwrap(),
            r#"{"cmd":"scan-file","path":"C:\\Pictures\\code.png"}"#
        );
    }
}
