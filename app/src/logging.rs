//! Logging to a small rotating file in the platform log directory.
//!
//! Logs never contain decoded content, only events and errors. Set `RUST_LOG=debug`
//! for more detail.

use tracing_appender::non_blocking::WorkerGuard;
use tracing_appender::rolling::{RollingFileAppender, Rotation};
use tracing_subscriber::{fmt, prelude::*, EnvFilter};

/// Initialises logging. Keep the returned guard alive until the program ends.
pub fn init() -> Option<WorkerGuard> {
    let filter = EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info"));
    let dir = qrtool_platform::paths::log_dir();
    let file = std::fs::create_dir_all(&dir).ok().and_then(|_| {
        RollingFileAppender::builder()
            .rotation(Rotation::DAILY)
            .max_log_files(5)
            .filename_prefix("qrcode-tool")
            .filename_suffix("log")
            .build(&dir)
            .ok()
    });

    let stderr_layer = cfg!(debug_assertions).then(|| fmt::layer().with_writer(std::io::stderr));
    match file {
        Some(appender) => {
            let (writer, guard) = tracing_appender::non_blocking(appender);
            let _ = tracing_subscriber::registry()
                .with(filter)
                .with(fmt::layer().with_ansi(false).with_writer(writer))
                .with(stderr_layer)
                .try_init();
            Some(guard)
        }
        None => {
            let _ = tracing_subscriber::registry()
                .with(filter)
                .with(fmt::layer().with_writer(std::io::stderr))
                .try_init();
            None
        }
    }
}
