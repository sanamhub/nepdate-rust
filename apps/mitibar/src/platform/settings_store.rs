//! TOML load and save, atomic write, `dirs` paths (WR-06). A missing file is written with
//! the defaults; a file that does not parse is renamed to `settings.toml.broken-<unix-ts>` and
//! the defaults are used (WR-06 AC2). Writes go to a temp file in the same folder and are then
//! renamed over the target (WR-06 AC3).

use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use crate::app::settings::Settings;

/// `dirs::config_dir()/mitibar/settings.toml` (WR-06).
#[must_use]
pub fn default_path() -> Option<PathBuf> {
    dirs::config_dir().map(|dir| dir.join("mitibar").join("settings.toml"))
}

/// Loads the per-user settings, or the defaults when there is no config directory.
#[must_use]
pub fn load_default() -> (Settings, Vec<String>) {
    let Some(path) = default_path() else {
        return (
            Settings::default(),
            vec!["no config directory; using the default settings".to_owned()],
        );
    };
    load(&path)
}

/// Loads `path`. A missing file is created with the defaults. A parse failure renames the file
/// and falls back to the defaults. Every warning is returned for the log.
#[must_use]
pub fn load(path: &Path) -> (Settings, Vec<String>) {
    let mut warnings = Vec::new();
    match fs::read_to_string(path) {
        Ok(text) => match Settings::from_toml(&text) {
            Ok(loaded) => {
                warnings.extend(loaded.warnings);
                if loaded.migrated {
                    // An older schema is rewritten as the current one, so the next start reads it
                    // as such (WR-06). The migrated values are used either way.
                    match save(path, &loaded.settings) {
                        Ok(()) => warnings.push(format!(
                            "rewrote settings.toml as schema {}",
                            loaded.settings.schema
                        )),
                        Err(write_err) => warnings.push(format!(
                            "could not rewrite settings.toml as schema {}: {write_err}",
                            loaded.settings.schema
                        )),
                    }
                }
                (loaded.settings, warnings)
            }
            Err(err) => {
                warnings.push(format!("settings.toml did not parse: {err}"));
                let broken = broken_path(path);
                match fs::rename(path, &broken) {
                    Ok(()) => {
                        warnings.push(format!("moved the broken file to {}", broken.display()));
                    }
                    Err(rename_err) => warnings.push(format!(
                        "could not move the broken file to {}: {rename_err}",
                        broken.display()
                    )),
                }
                (Settings::default(), warnings)
            }
        },
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => {
            let settings = Settings::default();
            if let Err(write_err) = save(path, &settings) {
                warnings.push(format!("could not write the default settings: {write_err}"));
            }
            (settings, warnings)
        }
        Err(err) => {
            warnings.push(format!("could not read settings.toml: {err}"));
            (Settings::default(), warnings)
        }
    }
}

/// Writes `settings` atomically: the same folder's `settings.toml.tmp`, flushed to disk, then a
/// rename over the target (`MoveFileExW` with replace on Windows). Without the flush, a power
/// loss after the rename can leave an empty file on some file systems.
///
/// # Errors
///
/// Returns the IO or TOML error when the temporary file or the rename fails.
pub fn save(path: &Path, settings: &Settings) -> anyhow::Result<()> {
    if let Some(dir) = path.parent() {
        fs::create_dir_all(dir)?;
    }
    let text = toml::to_string_pretty(settings)?;
    let temp = path.with_extension("toml.tmp");
    {
        let mut file = fs::File::create(&temp)?;
        file.write_all(text.as_bytes())?;
        file.sync_all()?;
    }
    fs::rename(&temp, path)?;
    Ok(())
}

/// `settings.toml.broken-<unix-ts>` next to the original file.
fn broken_path(path: &Path) -> PathBuf {
    let stamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |elapsed| elapsed.as_secs());
    let name = path.file_name().map_or_else(
        || "settings.toml".into(),
        |name| name.to_string_lossy().into_owned(),
    );
    path.with_file_name(format!("{name}.broken-{stamp}"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn wr06_ac2_round_trip() {
        let dir = tempdir().expect("tempdir");
        let path = dir.path().join("settings.toml");
        let mut settings = Settings {
            language: "ne".to_owned(),
            theme: "dark".to_owned(),
            clock_24h: true,
            show_seconds: true,
            extra_time_zone: "Europe/Berlin".to_owned(),
            launch_at_login: false,
            bar: false,
            bar_locked: true,
            bar_top: vec!["tithi".to_owned()],
            bar_bottom: vec!["bs_short".to_owned(), "holiday".to_owned()],
            bar_monitor: r"\\.\DISPLAY2".to_owned(),
            bar_x: -12,
            bar_y: 40,
            ..Settings::default()
        };
        save(&path, &settings).expect("save");
        let (loaded, warnings) = load(&path);
        assert!(warnings.is_empty(), "{warnings:?}");
        settings.schema = loaded.schema;
        assert_eq!(loaded, settings);
        assert!(
            !path.with_extension("toml.tmp").exists(),
            "temp file removed"
        );
    }

    #[test]
    fn wr06_ac3_save_replaces_an_existing_file() {
        let dir = tempdir().expect("tempdir");
        let path = dir.path().join("settings.toml");
        save(&path, &Settings::default()).expect("first save");
        let dark = Settings {
            theme: "dark".to_owned(),
            ..Settings::default()
        };
        save(&path, &dark).expect("save over the existing file");
        assert_eq!(load(&path).0.theme, "dark");
    }

    #[test]
    fn wr06_ac2_missing_file_writes_defaults() {
        let dir = tempdir().expect("tempdir");
        let path = dir.path().join("settings.toml");
        let (loaded, warnings) = load(&path);
        assert_eq!(loaded, Settings::default());
        assert!(warnings.is_empty(), "{warnings:?}");
        assert!(path.exists(), "the defaults were written");
        let text = fs::read_to_string(&path).expect("read");
        assert!(text.contains("schema = 2"), "{text}");
        assert!(text.contains("bar = true"), "{text}");
        assert!(text.contains("bar_top"), "{text}");
        assert!(text.contains("zone_time"), "{text}");
    }

    #[test]
    fn wr06_schema_1_with_the_taskbar_text_off_is_read_as_bar_off_and_rewritten() {
        let dir = tempdir().expect("tempdir");
        let path = dir.path().join("settings.toml");
        fs::write(
            &path,
            "schema = 1\nlanguage = \"ne\"\ntheme = \"dark\"\nclock_24h = false\n\
             show_seconds = false\nextra_time_zone = \"Asia/Kathmandu\"\n\
             launch_at_login = false\ntaskbar_text = false\n",
        )
        .expect("write");
        let (loaded, warnings) = load(&path);
        assert!(!loaded.bar, "taskbar_text = false became bar = false");
        assert_eq!(loaded.schema, 2);
        assert_eq!(loaded.language, "ne");
        assert_eq!(loaded.theme, "dark");
        assert!(!loaded.launch_at_login);
        assert_eq!(loaded.bar_top, Settings::default().bar_top);
        assert_eq!(warnings.len(), 1, "{warnings:?}");
        assert!(warnings[0].contains("schema 2"), "{warnings:?}");

        let text = fs::read_to_string(&path).expect("read");
        assert!(
            text.contains("schema = 2"),
            "rewritten as schema 2:\n{text}"
        );
        assert!(
            !text.contains("taskbar_text"),
            "the old key is gone:\n{text}"
        );
        assert!(text.contains("bar = false"), "{text}");
        // The next start reads the rewritten file as it is, with nothing to migrate.
        let (again, warnings) = load(&path);
        assert_eq!(again, loaded);
        assert!(warnings.is_empty(), "{warnings:?}");
    }

    #[test]
    fn wr06_schema_1_with_the_taskbar_text_on_keeps_the_bar_on() {
        let dir = tempdir().expect("tempdir");
        let path = dir.path().join("settings.toml");
        fs::write(&path, "schema = 1\ntaskbar_text = true\n").expect("write");
        let (loaded, _) = load(&path);
        assert!(loaded.bar);
        assert!(
            fs::read_to_string(&path)
                .expect("read")
                .contains("schema = 2")
        );
    }

    #[test]
    fn wr06_a_schema_1_file_that_cannot_be_rewritten_still_loads_migrated() {
        let dir = tempdir().expect("tempdir");
        let path = dir.path().join("settings.toml");
        fs::write(&path, "schema = 1\ntaskbar_text = false\n").expect("write");
        // The temp file's name is taken by a directory, so the rewrite fails.
        fs::create_dir(path.with_extension("toml.tmp")).expect("block the temp file");
        let (loaded, warnings) = load(&path);
        assert!(!loaded.bar);
        assert_eq!(loaded.schema, 2);
        assert!(
            warnings.iter().any(|w| w.contains("could not rewrite")),
            "{warnings:?}"
        );
    }

    #[test]
    fn wr06_ac2_invalid_bar_values_fall_back_with_a_warning_each() {
        let dir = tempdir().expect("tempdir");
        let path = dir.path().join("settings.toml");
        fs::write(
            &path,
            "schema = 2\nbar_top = [\"weekday\", \"sunrise\"]\nbar_bottom = []\nbar_locked = true\n\
             unknown_key = 1\n",
        )
        .expect("write");
        let (loaded, warnings) = load(&path);
        assert_eq!(
            loaded.bar_top,
            Settings::default().bar_top,
            "unknown field id"
        );
        assert!(loaded.bar_bottom.is_empty(), "one empty line is allowed");
        assert!(loaded.bar_locked, "valid keys are kept");
        assert_eq!(warnings.len(), 1, "{warnings:?}");
        assert!(warnings[0].contains("bar_top"), "{warnings:?}");
    }

    #[test]
    fn wr06_ac2_unknown_keys_in_a_schema_2_file_are_ignored_without_a_warning() {
        let dir = tempdir().expect("tempdir");
        let path = dir.path().join("settings.toml");
        fs::write(
            &path,
            "schema = 2\nbar = false\nshortcut = \"Win+Shift+N\"\n",
        )
        .expect("write");
        let (loaded, warnings) = load(&path);
        assert!(!loaded.bar);
        assert!(warnings.is_empty(), "{warnings:?}");
    }

    #[test]
    fn wr06_ac2_broken_file_is_renamed() {
        let dir = tempdir().expect("tempdir");
        let path = dir.path().join("settings.toml");
        fs::write(&path, "language = \"ne\n").expect("write");
        let (loaded, warnings) = load(&path);
        assert_eq!(loaded, Settings::default());
        assert!(!path.exists(), "the broken file moved away");
        let broken: Vec<_> = fs::read_dir(dir.path())
            .expect("read dir")
            .filter_map(Result::ok)
            .filter(|entry| {
                entry
                    .file_name()
                    .to_string_lossy()
                    .starts_with("settings.toml.broken-")
            })
            .collect();
        assert_eq!(broken.len(), 1, "one broken file");
        assert!(
            warnings
                .iter()
                .any(|warning| warning.contains("did not parse")),
            "{warnings:?}"
        );
    }

    #[test]
    fn wr06_ac2_unknown_key_is_ignored_and_invalid_language_warns() {
        let dir = tempdir().expect("tempdir");
        let path = dir.path().join("settings.toml");
        fs::write(
            &path,
            "language = \"fr\"\ntheme = \"dark\"\nunknown_key = 42\n",
        )
        .expect("write");
        let (loaded, warnings) = load(&path);
        assert_eq!(loaded.language, "en");
        assert_eq!(loaded.theme, "dark");
        assert_eq!(warnings.len(), 1, "{warnings:?}");
        assert!(warnings[0].contains("language"), "{warnings:?}");
    }

    #[test]
    fn wr06_ac2_unknown_zone_falls_back() {
        let dir = tempdir().expect("tempdir");
        let path = dir.path().join("settings.toml");
        fs::write(&path, "extra_time_zone = \"Mars/Olympus\"\n").expect("write");
        let (loaded, warnings) = load(&path);
        assert_eq!(loaded.extra_time_zone, "Asia/Kathmandu");
        assert_eq!(warnings.len(), 1, "{warnings:?}");
    }
}
