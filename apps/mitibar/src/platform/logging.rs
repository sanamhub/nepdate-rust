//! Logging to `dirs::data_local_dir()/mitibar/mitibar.log`, size-rotated at 1 MB
//! keeping 2 files (WR-09 AC4), plus the panic hook that logs message and location (WR-09 AC5).

use std::path::Path;

use anyhow::Context;
use simplelog::{Config, LevelFilter, WriteLogger};

const LOG_FILE: &str = "mitibar.log";
const MAX_BYTES: u64 = 1_000_000;

/// Installs the panic hook and the file logger. Called once at startup; best-effort, so a
/// logging failure does not stop the app (the panic hook still runs, logging is just a no-op
/// without the logger).
pub fn init() -> anyhow::Result<()> {
    install_panic_hook();

    let dir = dirs::data_local_dir()
        .unwrap_or_else(|| std::path::PathBuf::from("."))
        .join("mitibar");
    std::fs::create_dir_all(&dir).with_context(|| format!("create {}", dir.display()))?;
    rotate_if_needed(&dir);

    let path = dir.join(LOG_FILE);
    let file = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(&path)
        .with_context(|| format!("open {}", path.display()))?;
    WriteLogger::init(LevelFilter::Info, Config::default(), file).context("init logger")?;
    log::info!("mitibar logging to {}", path.display());
    Ok(())
}

/// If the current log is over 1 MB, moves it to `<LOG_FILE>.1`, overwriting the previous `.1`,
/// so two files are kept (WR-09 AC4).
fn rotate_if_needed(dir: &Path) {
    let log = dir.join(LOG_FILE);
    let too_big = log.metadata().is_ok_and(|m| m.len() > MAX_BYTES);
    if too_big {
        let rotated = dir.join(format!("{LOG_FILE}.1"));
        let _ = std::fs::remove_file(&rotated);
        let _ = std::fs::rename(&log, &rotated);
    }
}

/// Logs the panic message and location before the process exits (WR-09 AC5).
fn install_panic_hook() {
    std::panic::set_hook(Box::new(|info| {
        let location = info.location().map(ToString::to_string).unwrap_or_default();
        let payload = if let Some(s) = info.payload().downcast_ref::<&str>() {
            *s
        } else if let Some(s) = info.payload().downcast_ref::<String>() {
            s.as_str()
        } else {
            "non-string panic payload"
        };
        log::error!("panic at {location}: {payload}");
    }));
}
