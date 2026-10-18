//! Per-user launch at login (WR-09 AC2, W3-05). Windows writes the quoted executable path to
//! the `HKCU\...\Run` value `mitibar` itself; `auto-launch` 0.6 registers a `LaunchAgent` or
//! `SMAppService` on macOS and an `XDG` autostart file on Linux. No admin rights.
//!
//! The tests use a fake `LoginItem`; they never create a registry value, `LaunchAgent` or desktop
//! file. The real registration is behind the same trait.

use std::path::Path;

use crate::app::APP_NAME;

/// The small part of a login item the app needs. Tests provide a fake.
pub trait LoginItem {
    /// Whether an entry currently exists.
    ///
    /// # Errors
    ///
    /// Returns the platform error when the entry cannot be read.
    fn is_enabled(&self) -> anyhow::Result<bool>;
    /// Creates or replaces the entry with the current executable.
    ///
    /// # Errors
    ///
    /// Returns the platform error when the entry cannot be written.
    fn enable(&self) -> anyhow::Result<()>;
    /// Removes the entry. Missing is not an error.
    ///
    /// # Errors
    ///
    /// Returns the platform error when the entry cannot be removed.
    fn disable(&self) -> anyhow::Result<()>;
}

/// Makes the login item match `enabled` (WR-09 AC2). Enabling always writes the entry, so a
/// moved executable is re-registered; `auto-launch` 0.6 exposes no getter for the path.
///
/// # Errors
///
/// Returns the first platform error.
pub fn sync(enabled: bool, item: &dyn LoginItem) -> anyhow::Result<()> {
    if enabled {
        // Always write, so a moved executable re-registers. `auto-launch` 0.6 exposes no getter
        // for the registered path.
        item.enable()?;
    } else if item.is_enabled()? {
        item.disable()?;
    }
    Ok(())
}

/// Applies `enabled` to the OS login item. Startup never fails because of this (W3-05): the
/// error is logged and the app continues.
pub fn apply(enabled: bool) {
    let Some(exe) = std::env::current_exe().ok() else {
        log::warn!("autostart: cannot find the current executable");
        return;
    };
    let item = platform_item(&exe);
    if let Err(err) = sync(enabled, item.as_ref()) {
        log::warn!("autostart: {err}");
    } else {
        log::debug!("autostart: launch at login is {enabled}");
    }
}

/// The platform login item for `exe`.
///
/// Windows writes the `HKCU` `Run` value itself instead of using `auto-launch`, which writes the
/// path unquoted (a path with spaces is then an unquoted-path entry, CWE-428) and also sets the
/// Task Manager `StartupApproved` value on every enable, which would undo a user's "Disable" in
/// Task Manager at each start.
#[cfg(target_os = "windows")]
fn platform_item(exe: &Path) -> Box<dyn LoginItem> {
    Box::new(RunKeyItem {
        command: run_command(exe),
    })
}

/// The `Run` value data: the executable path in double quotes.
#[cfg(any(target_os = "windows", test))]
fn run_command(exe: &Path) -> String {
    format!("\"{}\"", exe.display())
}

/// `HKCU\Software\Microsoft\Windows\CurrentVersion\Run`, value `APP_NAME`.
#[cfg(target_os = "windows")]
struct RunKeyItem {
    command: String,
}

#[cfg(target_os = "windows")]
const RUN_KEY: &str = r"Software\Microsoft\Windows\CurrentVersion\Run";

#[cfg(target_os = "windows")]
fn is_not_found(err: &windows_core::Error) -> bool {
    // ERROR_FILE_NOT_FOUND, for a missing key or value.
    err.code() == windows_core::HRESULT::from_win32(2)
}

#[cfg(target_os = "windows")]
impl LoginItem for RunKeyItem {
    fn is_enabled(&self) -> anyhow::Result<bool> {
        match windows_registry::CURRENT_USER
            .open(RUN_KEY)
            .and_then(|key| key.get_string(APP_NAME))
        {
            Ok(_) => Ok(true),
            Err(err) if is_not_found(&err) => Ok(false),
            Err(err) => Err(err.into()),
        }
    }

    fn enable(&self) -> anyhow::Result<()> {
        windows_registry::CURRENT_USER
            .create(RUN_KEY)?
            .set_string(APP_NAME, &self.command)?;
        Ok(())
    }

    fn disable(&self) -> anyhow::Result<()> {
        match windows_registry::CURRENT_USER
            .options()
            .write()
            .open(RUN_KEY)
            .and_then(|key| key.remove_value(APP_NAME))
        {
            Ok(()) => Ok(()),
            Err(err) if is_not_found(&err) => Ok(()),
            Err(err) => Err(err.into()),
        }
    }
}

#[cfg(target_os = "macos")]
fn platform_item(exe: &Path) -> Box<dyn LoginItem> {
    use auto_launch::{AutoLaunch, MacOSLaunchMode};

    let path = exe.to_string_lossy();
    // A bundled .app can use SMAppService (macOS 13+); `cargo run` uses a LaunchAgent.
    let mode = if path.contains(".app/Contents/MacOS/") {
        MacOSLaunchMode::SMAppService
    } else {
        MacOSLaunchMode::LaunchAgent
    };
    let item = AutoLaunch::new(
        APP_NAME,
        path.as_ref(),
        mode,
        &[] as &[&str],
        &[] as &[&str],
        "",
    );
    Box::new(AutoItem(item))
}

#[cfg(all(unix, not(target_os = "macos")))]
fn platform_item(exe: &Path) -> Box<dyn LoginItem> {
    use auto_launch::{AutoLaunch, LinuxLaunchMode};

    let path = exe.to_string_lossy();
    let item = AutoLaunch::new(
        APP_NAME,
        path.as_ref(),
        LinuxLaunchMode::XdgAutostart,
        &[] as &[&str],
    );
    Box::new(AutoItem(item))
}

/// The `auto-launch` wrapper (macOS and Linux).
#[cfg(not(target_os = "windows"))]
struct AutoItem(auto_launch::AutoLaunch);

#[cfg(not(target_os = "windows"))]
impl LoginItem for AutoItem {
    fn is_enabled(&self) -> anyhow::Result<bool> {
        Ok(self.0.is_enabled()?)
    }

    fn enable(&self) -> anyhow::Result<()> {
        self.0.enable()?;
        Ok(())
    }

    fn disable(&self) -> anyhow::Result<()> {
        self.0.disable()?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use std::cell::Cell;

    use super::*;

    #[derive(Default)]
    struct FakeItem {
        enabled: Cell<bool>,
        enables: Cell<u32>,
        disables: Cell<u32>,
    }

    impl LoginItem for FakeItem {
        fn is_enabled(&self) -> anyhow::Result<bool> {
            Ok(self.enabled.get())
        }

        fn enable(&self) -> anyhow::Result<()> {
            self.enabled.set(true);
            self.enables.set(self.enables.get() + 1);
            Ok(())
        }

        fn disable(&self) -> anyhow::Result<()> {
            self.enabled.set(false);
            self.disables.set(self.disables.get() + 1);
            Ok(())
        }
    }

    #[test]
    fn wr09_ac2_sync_follows_the_setting_and_refreshes_the_path() {
        let item = FakeItem::default();
        sync(true, &item).expect("enable");
        assert!(item.enabled.get());
        assert_eq!(item.enables.get(), 1);
        sync(true, &item).expect("still enabled");
        assert_eq!(item.enables.get(), 2, "the path is written again on start");
        sync(false, &item).expect("disable");
        assert!(!item.enabled.get());
        assert_eq!(item.disables.get(), 1);
        sync(false, &item).expect("still disabled");
        assert_eq!(item.disables.get(), 1, "no second remove");
    }

    #[test]
    fn wr09_ac2_run_value_quotes_a_path_with_spaces() {
        let exe = Path::new(r"C:\Program Files\MitiBar\mitibar.exe");
        assert_eq!(
            run_command(exe),
            r#""C:\Program Files\MitiBar\mitibar.exe""#
        );
    }

    #[test]
    fn wr09_ac2_sync_starts_from_an_existing_entry() {
        let item = FakeItem::default();
        item.enabled.set(true);
        sync(false, &item).expect("disable");
        assert!(!item.enabled.get());
        assert_eq!(item.enables.get(), 0);
    }
}
