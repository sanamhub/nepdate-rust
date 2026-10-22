//! Per-user launch at login (WR-09 AC2). Windows writes the quoted executable path to
//! the `HKCU\...\Run` value `mitibar` itself; `auto-launch` 0.6 registers a `LaunchAgent` or
//! `SMAppService` on macOS and an `XDG` autostart file on Linux. No admin rights.
//!
//! Two entry points, because the rules differ. `apply_on_start` runs when the app starts with the
//! setting on: it writes the item only when it is missing or points at a file that no longer
//! exists, so a portable copy never takes over from an installed one. `apply` runs when the user
//! flips the switch: turning it on always points the item at the running copy.
//!
//! The tests use a fake `LoginItem`; they never create a registry value, `LaunchAgent` or desktop
//! file. The real registration is behind the same trait.

use std::path::{Path, PathBuf};

use crate::app::APP_NAME;

/// Where a login item points.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Target {
    /// There is no login item.
    Missing,
    /// The item runs this file.
    #[cfg_attr(not(windows), allow(dead_code))]
    Path(PathBuf),
    /// The item exists, but the platform can't say what it runs (`auto-launch` 0.6 has no getter
    /// for the path on macOS and Linux).
    #[cfg_attr(windows, allow(dead_code))]
    Unknown,
}

/// The small part of a login item the app needs. Tests provide a fake.
pub trait LoginItem {
    /// What the item points at, if it exists.
    ///
    /// # Errors
    ///
    /// Returns the platform error when the entry cannot be read.
    fn target(&self) -> anyhow::Result<Target>;
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

/// Makes the login item match `enabled` when the user flips the setting (WR-09 AC2). Enabling
/// always writes the entry, so it points at the copy that is running.
///
/// # Errors
///
/// Returns the first platform error.
pub fn sync(enabled: bool, item: &dyn LoginItem) -> anyhow::Result<()> {
    if enabled { item.enable() } else { remove(item) }
}

/// Makes the login item match the setting when the app starts (WR-09 AC2). With the setting on,
/// the item is written only when it is missing or points at a file that no longer exists
/// (`exists`). One that points at an existing file is left alone, whichever copy that is, so a
/// portable copy never replaces an item that points at an installed copy. Where the platform can't
/// say what the item runs (`Target::Unknown`), it is written, as before.
///
/// # Errors
///
/// Returns the first platform error.
pub fn sync_on_start(
    enabled: bool,
    item: &dyn LoginItem,
    exists: &dyn Fn(&Path) -> bool,
) -> anyhow::Result<()> {
    if !enabled {
        return remove(item);
    }
    match item.target()? {
        Target::Path(path) if exists(&path) => Ok(()),
        Target::Missing | Target::Path(_) | Target::Unknown => item.enable(),
    }
}

fn remove(item: &dyn LoginItem) -> anyhow::Result<()> {
    if item.target()? != Target::Missing {
        item.disable()?;
    }
    Ok(())
}

/// Applies the setting when the user flips it. Never fails the app: the error is logged.
pub fn apply(enabled: bool) {
    run(enabled, |item| sync(enabled, item));
}

/// Applies the setting at startup (see `sync_on_start`). Never fails the app: the error is logged.
pub fn apply_on_start(enabled: bool) {
    run(enabled, |item| {
        sync_on_start(enabled, item, &|path| path.exists())
    });
}

fn run(enabled: bool, action: impl FnOnce(&dyn LoginItem) -> anyhow::Result<()>) {
    let Some(exe) = std::env::current_exe().ok() else {
        log::warn!("autostart: cannot find the current executable");
        return;
    };
    let item = platform_item(&exe);
    if let Err(err) = action(item.as_ref()) {
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

/// The executable a `Run` value starts: the quoted path, or the whole value when it has no quotes.
#[cfg(any(target_os = "windows", test))]
fn command_path(command: &str) -> PathBuf {
    let command = command.trim();
    match command.strip_prefix('"') {
        Some(rest) => PathBuf::from(rest.split('"').next().unwrap_or(rest)),
        None => PathBuf::from(command),
    }
}

/// `HKCU\Software\Microsoft\Windows\CurrentVersion\Run`, value `APP_NAME`.
#[cfg(target_os = "windows")]
struct RunKeyItem {
    command: String,
}

#[cfg(target_os = "windows")]
const RUN_KEY: &str = r"Software\Microsoft\Windows\CurrentVersion\Run";

#[cfg(target_os = "windows")]
fn is_not_found(err: &windows::core::Error) -> bool {
    // ERROR_FILE_NOT_FOUND, for a missing key or value.
    err.code() == windows::core::HRESULT::from_win32(2)
}

#[cfg(target_os = "windows")]
impl LoginItem for RunKeyItem {
    fn target(&self) -> anyhow::Result<Target> {
        match windows_registry::CURRENT_USER
            .open(RUN_KEY)
            .and_then(|key| key.get_string(APP_NAME))
        {
            Ok(command) => Ok(Target::Path(command_path(&command))),
            Err(err) if is_not_found(&err) => Ok(Target::Missing),
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
    fn target(&self) -> anyhow::Result<Target> {
        Ok(if self.0.is_enabled()? {
            Target::Unknown
        } else {
            Target::Missing
        })
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
    use std::cell::{Cell, RefCell};

    use super::*;

    /// A fake registry value: what it holds, and how often it was written or removed.
    #[derive(Default)]
    struct FakeItem {
        value: RefCell<Option<Target>>,
        /// The file `enable` writes: the running copy.
        writes_as: PathBuf,
        enables: Cell<u32>,
        disables: Cell<u32>,
    }

    impl FakeItem {
        fn running(exe: &str) -> Self {
            Self {
                writes_as: PathBuf::from(exe),
                ..Self::default()
            }
        }

        fn holding(exe: &str, target: Target) -> Self {
            let item = Self::running(exe);
            *item.value.borrow_mut() = Some(target);
            item
        }

        fn target_now(&self) -> Target {
            self.value.borrow().clone().unwrap_or(Target::Missing)
        }
    }

    impl LoginItem for FakeItem {
        fn target(&self) -> anyhow::Result<Target> {
            Ok(self.target_now())
        }

        fn enable(&self) -> anyhow::Result<()> {
            *self.value.borrow_mut() = Some(Target::Path(self.writes_as.clone()));
            self.enables.set(self.enables.get() + 1);
            Ok(())
        }

        fn disable(&self) -> anyhow::Result<()> {
            *self.value.borrow_mut() = None;
            self.disables.set(self.disables.get() + 1);
            Ok(())
        }
    }

    const INSTALLED: &str = r"C:\Users\sam\AppData\Local\Programs\MitiBar\MitiBar.exe";
    const PORTABLE: &str = r"C:\Users\sam\Downloads\MitiBar\MitiBar.exe";

    fn path(text: &str) -> Target {
        Target::Path(PathBuf::from(text))
    }

    /// Every path in `present` exists; nothing else does.
    fn files(present: &'static [&'static str]) -> impl Fn(&Path) -> bool {
        move |path| present.iter().any(|file| Path::new(file) == path)
    }

    #[test]
    fn wr09_ac2_a_missing_item_is_written_on_start() {
        let item = FakeItem::running(INSTALLED);
        sync_on_start(true, &item, &files(&[INSTALLED])).expect("start");
        assert_eq!(item.enables.get(), 1);
        assert_eq!(item.target_now(), path(INSTALLED));
    }

    #[test]
    fn wr09_ac2_an_item_pointing_at_a_file_that_is_gone_is_written_on_start() {
        // The installed copy was uninstalled; the item still points at it.
        let item = FakeItem::holding(PORTABLE, path(INSTALLED));
        sync_on_start(true, &item, &files(&[PORTABLE])).expect("start");
        assert_eq!(item.enables.get(), 1, "a stale item is repaired");
        assert_eq!(item.target_now(), path(PORTABLE));
    }

    #[test]
    fn wr09_ac2_an_item_pointing_at_an_existing_file_is_left_alone_on_start() {
        let item = FakeItem::holding(INSTALLED, path(INSTALLED));
        sync_on_start(true, &item, &files(&[INSTALLED])).expect("start");
        assert_eq!(item.enables.get(), 0, "no write when nothing is wrong");
        assert_eq!(item.disables.get(), 0);
    }

    #[test]
    fn wr09_ac2_a_portable_copy_never_replaces_an_item_for_an_existing_installed_copy() {
        let item = FakeItem::holding(PORTABLE, path(INSTALLED));
        sync_on_start(true, &item, &files(&[INSTALLED, PORTABLE])).expect("start");
        assert_eq!(item.enables.get(), 0);
        assert_eq!(item.target_now(), path(INSTALLED));
    }

    #[test]
    fn wr09_ac2_a_portable_copy_writes_when_the_installed_copy_is_gone_or_never_was() {
        let stale = FakeItem::holding(PORTABLE, path(INSTALLED));
        sync_on_start(true, &stale, &files(&[PORTABLE])).expect("start");
        assert_eq!(stale.target_now(), path(PORTABLE));

        let missing = FakeItem::running(PORTABLE);
        sync_on_start(true, &missing, &files(&[PORTABLE])).expect("start");
        assert_eq!(missing.target_now(), path(PORTABLE));
    }

    #[test]
    fn wr09_ac2_an_installed_copy_leaves_an_item_for_an_existing_portable_copy_alone() {
        // The rule is about missing and stale items only; the user's Settings switch is how an
        // installed copy takes the item over.
        let item = FakeItem::holding(INSTALLED, path(PORTABLE));
        sync_on_start(true, &item, &files(&[INSTALLED, PORTABLE])).expect("start");
        assert_eq!(item.enables.get(), 0);
        assert_eq!(item.target_now(), path(PORTABLE));
    }

    #[test]
    fn wr09_ac2_an_item_whose_target_the_platform_cannot_read_is_written_on_start() {
        let item = FakeItem::holding(INSTALLED, Target::Unknown);
        sync_on_start(true, &item, &files(&[])).expect("start");
        assert_eq!(item.enables.get(), 1);
    }

    #[test]
    fn wr09_ac2_with_the_setting_off_the_item_is_removed_on_start_and_never_written() {
        let item = FakeItem::holding(INSTALLED, path(INSTALLED));
        sync_on_start(false, &item, &files(&[INSTALLED])).expect("start");
        assert_eq!(item.target_now(), Target::Missing);
        assert_eq!(item.disables.get(), 1);
        sync_on_start(false, &item, &files(&[INSTALLED])).expect("start again");
        assert_eq!(item.disables.get(), 1, "no second remove");
        assert_eq!(item.enables.get(), 0);
    }

    #[test]
    fn wr09_ac2_turning_the_setting_on_always_points_the_item_at_the_running_copy() {
        // The item points at an existing installed copy; the portable copy's user turns the
        // switch on, which is an explicit choice, so the item moves.
        let item = FakeItem::holding(PORTABLE, path(INSTALLED));
        sync(true, &item).expect("turn on");
        assert_eq!(item.target_now(), path(PORTABLE));
        assert_eq!(item.enables.get(), 1);
        // And once more with the item already right: written again, so a moved copy re-registers.
        sync(true, &item).expect("turn on again");
        assert_eq!(item.enables.get(), 2);
    }

    #[test]
    fn wr09_ac2_turning_the_setting_off_removes_the_item_once() {
        let item = FakeItem::holding(INSTALLED, path(INSTALLED));
        sync(false, &item).expect("turn off");
        assert_eq!(item.target_now(), Target::Missing);
        assert_eq!(item.disables.get(), 1);
        sync(false, &item).expect("still off");
        assert_eq!(item.disables.get(), 1, "no second remove");
        assert_eq!(item.enables.get(), 0);
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
    fn wr09_ac2_the_run_value_is_read_back_as_a_path() {
        let exe = Path::new(r"C:\Program Files\MitiBar\mitibar.exe");
        assert_eq!(command_path(&run_command(exe)), exe);
        // An unquoted value, such as one written by an older build or by hand.
        assert_eq!(
            command_path(r"C:\MitiBar\mitibar.exe"),
            Path::new(r"C:\MitiBar\mitibar.exe")
        );
        assert_eq!(
            command_path(r#"  "C:\MitiBar\mitibar.exe" --flag"#),
            Path::new(r"C:\MitiBar\mitibar.exe")
        );
    }
}
