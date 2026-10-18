//! TOML load and save, atomic write, `dirs` paths (WR-06, W3-03). A missing file is written with
//! the defaults; a file that does not parse is renamed to `settings.toml.broken-<unix-ts>` and
//! the defaults are used (WR-06 AC2). Writes go to a temp file in the same folder and are then
//! renamed over the target (WR-06 AC3, learn/11 §2).

use std::fs;
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

/// Saves the per-user settings, or does nothing when there is no config directory. The settings
/// UI wires this in W3-04.
///
/// # Errors
///
/// Returns the IO or TOML error when the file cannot be written.
#[allow(dead_code)]
pub fn save_default(settings: &Settings) -> anyhow::Result<()> {
    let path =
        default_path().ok_or_else(|| anyhow::anyhow!("no config directory for settings.toml"))?;
    save(&path, settings)
}

/// Loads `path`. A missing file is created with the defaults. A parse failure renames the file
/// and falls back to the defaults. Every warning is returned for the log.
#[must_use]
pub fn load(path: &Path) -> (Settings, Vec<String>) {
    let mut warnings = Vec::new();
    match fs::read_to_string(path) {
        Ok(text) => match toml::from_str::<Settings>(&text) {
            Ok(mut settings) => {
                warnings.extend(settings.validate());
                (settings, warnings)
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

/// Writes `settings` atomically: the same folder's `settings.toml.tmp`, then a rename.
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
    fs::write(&temp, text)?;
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
            taskbar_text: false,
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
    fn wr06_ac2_missing_file_writes_defaults() {
        let dir = tempdir().expect("tempdir");
        let path = dir.path().join("settings.toml");
        let (loaded, warnings) = load(&path);
        assert_eq!(loaded, Settings::default());
        assert!(warnings.is_empty(), "{warnings:?}");
        assert!(path.exists(), "the defaults were written");
        let text = fs::read_to_string(&path).expect("read");
        assert!(text.contains("schema = 1"), "{text}");
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
