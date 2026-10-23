//! Settings struct, defaults and validation, no file IO (WR-06). The settings file itself lives
//! in the platform layer.

use nepdate::Lang;
use serde::{Deserialize, Serialize};

use crate::app::bar::{BarField, DEFAULT_BOTTOM, DEFAULT_TOP, ids};
use crate::app::bar_place::Saved;
use crate::app::shortcut::Combo;

/// The schema this build reads and writes (WR-06).
const SCHEMA: u32 = 2;

/// Per-user settings (WR-06). The `language` and `theme` values stay strings so the TOML file
/// reads and writes them as written. The five bools are the bool keys of WR-06, so the lint
/// is allowed rather than re-shaping the config file.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
#[allow(clippy::struct_excessive_bools)]
pub struct Settings {
    pub schema: u32,
    pub language: String,
    pub theme: String,
    pub clock_24h: bool,
    pub show_seconds: bool,
    pub extra_time_zone: String,
    pub launch_at_login: bool,
    /// Whether the floating bar is shown (WR-08, Windows only; ignored elsewhere).
    pub bar: bool,
    pub bar_locked: bool,
    /// Field ids of the bar's two lines (WR-08 AC2).
    pub bar_top: Vec<String>,
    pub bar_bottom: Vec<String>,
    /// The monitor the bar was left on, by device name; empty means the default position.
    pub bar_monitor: String,
    /// The bar's top-left, in logical px from that monitor's top-left.
    pub bar_x: i32,
    pub bar_y: i32,
    /// The global shortcut as text, such as `Win+Shift+N`; empty means none (WR-13).
    pub shortcut: String,
}

/// What `Settings::from_toml` read, and whether the file needs writing back.
#[derive(Debug)]
pub struct Loaded {
    pub settings: Settings,
    pub warnings: Vec<String>,
    /// The file was an older schema, so the caller should write it back as the current one.
    pub migrated: bool,
}

/// The keys of schema 1 that schema 2 replaced. Read loosely, so a wrong type is a warning and
/// not a parse failure of the whole file.
#[derive(Debug, Default, Deserialize)]
struct SchemaOne {
    taskbar_text: Option<toml::Value>,
}

impl Settings {
    /// Parses a settings file. A schema 1 file is migrated: `taskbar_text = false` becomes
    /// `bar = false`, the new keys take their defaults, and `migrated` is set (WR-06).
    ///
    /// # Errors
    ///
    /// Returns the TOML error when the text does not parse.
    pub fn from_toml(text: &str) -> Result<Loaded, toml::de::Error> {
        let mut settings: Settings = toml::from_str(text)?;
        let mut warnings = Vec::new();
        let migrated = settings.schema == 1;
        if migrated {
            let legacy: SchemaOne = toml::from_str(text)?;
            match legacy.taskbar_text {
                Some(toml::Value::Boolean(on)) => settings.bar = on,
                Some(other) => warnings.push(format!(
                    "taskbar_text {other} is not a bool; using the default for bar"
                )),
                None => {}
            }
            settings.schema = SCHEMA;
        }
        warnings.extend(settings.validate());
        Ok(Loaded {
            settings,
            warnings,
            migrated,
        })
    }

    /// The saved bar position (`bar_monitor`, `bar_x`, `bar_y`); the default one when none is.
    pub fn bar_position(&self) -> Saved {
        Saved {
            monitor: self.bar_monitor.clone(),
            x: self.bar_x,
            y: self.bar_y,
        }
    }

    /// Stores a bar position. `Saved::default()` is the default position (WR-08 AC5).
    pub fn set_bar_position(&mut self, saved: &Saved) {
        self.bar_monitor.clone_from(&saved.monitor);
        self.bar_x = saved.x;
        self.bar_y = saved.y;
    }

    /// The global shortcut, or `None` when the setting is empty or not a valid combination.
    pub fn shortcut_combo(&self) -> Option<Combo> {
        Combo::parse(&self.shortcut).filter(|combo| combo.check().is_ok())
    }

    /// The active language. Anything other than `"ne"` is English (WR-06 AC2 fallback).
    pub fn lang(&self) -> Lang {
        match self.language.as_str() {
            "ne" => Lang::Ne,
            _ => Lang::En,
        }
    }

    /// Replaces every invalid value with that key's default and returns the warnings to log
    /// (WR-06 AC2). A missing key never reaches here: serde's `#[serde(default)]` fills it.
    /// An unknown IANA zone becomes `Asia/Kathmandu`; an empty zone stays empty, which means
    /// "no extra clock".
    ///
    /// The two bar lines are checked key by key for unknown field ids, then together: a field
    /// used twice, or both lines empty, is a combination the editor can't produce, so both keys
    /// take their defaults.
    pub fn validate(&mut self) -> Vec<String> {
        let defaults = Settings::default();
        let mut warnings = Vec::new();
        if self.schema != defaults.schema {
            warnings.push(format!(
                "schema {} is not {}; using {}",
                self.schema, defaults.schema, defaults.schema
            ));
            self.schema = defaults.schema;
        }
        if !matches!(self.language.as_str(), "en" | "ne") {
            warnings.push(format!(
                "language {:?} is not en or ne; using en",
                self.language
            ));
            self.language = defaults.language;
        }
        if !matches!(self.theme.as_str(), "system" | "light" | "dark") {
            warnings.push(format!(
                "theme {:?} is not system, light or dark; using system",
                self.theme
            ));
            self.theme = defaults.theme;
        }
        if !self.extra_time_zone.is_empty()
            && jiff::tz::TimeZone::get(&self.extra_time_zone).is_err()
        {
            warnings.push(format!(
                "extra_time_zone {:?} is not an IANA zone; using {}",
                self.extra_time_zone, defaults.extra_time_zone
            ));
            self.extra_time_zone = defaults.extra_time_zone;
        }
        for (key, line, default) in [
            ("bar_top", &mut self.bar_top, defaults.bar_top.clone()),
            (
                "bar_bottom",
                &mut self.bar_bottom,
                defaults.bar_bottom.clone(),
            ),
        ] {
            if let Some(unknown) = line.iter().find(|id| BarField::from_id(id).is_none()) {
                warnings.push(format!(
                    "{key} has the unknown field {unknown:?}; using the default"
                ));
                *line = default;
            }
        }
        if !self.shortcut.is_empty() && self.shortcut_combo().is_none() {
            warnings.push(format!(
                "shortcut {:?} is not a valid combination; using {}",
                self.shortcut, defaults.shortcut
            ));
            self.shortcut.clone_from(&defaults.shortcut);
        }
        if lines_conflict(&self.bar_top, &self.bar_bottom) {
            warnings.push(
                "bar_top and bar_bottom repeat a field or are both empty; using the defaults"
                    .to_owned(),
            );
            self.bar_top = defaults.bar_top;
            self.bar_bottom = defaults.bar_bottom;
        }
        warnings
    }
}

/// Whether the two lines together repeat a field or hold none.
fn lines_conflict(top: &[String], bottom: &[String]) -> bool {
    let all: Vec<&String> = top.iter().chain(bottom).collect();
    all.is_empty()
        || all
            .iter()
            .enumerate()
            .any(|(index, id)| all.iter().take(index).any(|earlier| earlier == id))
}

impl Default for Settings {
    fn default() -> Self {
        Settings {
            schema: SCHEMA,
            language: "en".to_owned(),
            theme: "system".to_owned(),
            clock_24h: false,
            show_seconds: false,
            extra_time_zone: "Asia/Kathmandu".to_owned(),
            launch_at_login: true,
            bar: true,
            bar_locked: false,
            bar_top: ids(DEFAULT_TOP),
            bar_bottom: ids(DEFAULT_BOTTOM),
            bar_monitor: String::new(),
            bar_x: 0,
            bar_y: 0,
            shortcut: Combo::DEFAULT.to_string(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn strings(ids: &[&str]) -> Vec<String> {
        ids.iter().map(|id| (*id).to_owned()).collect()
    }

    #[test]
    fn defaults_match_wr06() {
        let s = Settings::default();
        assert_eq!(s.schema, 2);
        assert_eq!(s.language, "en");
        assert_eq!(s.theme, "system");
        assert!(!s.clock_24h);
        assert!(!s.show_seconds);
        assert_eq!(s.extra_time_zone, "Asia/Kathmandu");
        assert!(s.launch_at_login);
        assert!(s.bar);
        assert!(!s.bar_locked);
        assert_eq!(s.bar_top, strings(&["weekday", "time", "zone_time"]));
        assert_eq!(s.bar_bottom, strings(&["bs_date", "ad_short"]));
        assert_eq!(s.bar_monitor, "");
        assert_eq!((s.bar_x, s.bar_y), (0, 0));
        assert_eq!(
            s.shortcut,
            format!("{}+Shift+N", crate::app::shortcut::META)
        );
    }

    #[test]
    fn the_bar_position_is_stored_and_reset_through_saved() {
        let mut s = Settings::default();
        assert_eq!(s.bar_position(), Saved::default());
        let saved = Saved {
            monitor: r"\\.\DISPLAY2".to_owned(),
            x: -30,
            y: 8,
        };
        s.set_bar_position(&saved);
        assert_eq!(s.bar_position(), saved);
        assert_eq!(
            (s.bar_monitor.as_str(), s.bar_x, s.bar_y),
            (r"\\.\DISPLAY2", -30, 8)
        );
        s.set_bar_position(&Saved::default());
        assert_eq!(s.bar_position(), Saved::default(), "Reset position");
    }

    #[test]
    fn lang_falls_back_to_en_for_unknown() {
        assert_eq!(Settings::default().lang(), Lang::En);
        let ne = Settings {
            language: "ne".to_owned(),
            ..Settings::default()
        };
        assert_eq!(ne.lang(), Lang::Ne);
        let bogus = Settings {
            language: "fr".to_owned(),
            ..Settings::default()
        };
        assert_eq!(bogus.lang(), Lang::En);
    }

    #[test]
    fn wr06_ac2_validate_replaces_invalid_values() {
        let mut s = Settings {
            schema: 7,
            language: "fr".to_owned(),
            theme: "sepia".to_owned(),
            extra_time_zone: "Mars/Olympus".to_owned(),
            bar_top: strings(&["weekday", "moon_phase"]),
            ..Settings::default()
        };
        let warnings = s.validate();
        assert_eq!(warnings.len(), 5, "{warnings:?}");
        assert_eq!(s, Settings::default());
    }

    #[test]
    fn wr06_ac2_a_shortcut_that_is_not_valid_takes_the_default() {
        for bad in [
            "Ctrl+Alt+N",
            "N",
            "Ctrl+N",
            "Win+Hyper+N",
            "Win+",
            "Win+F13",
        ] {
            let mut s = Settings {
                shortcut: bad.to_owned(),
                ..Settings::default()
            };
            let warnings = s.validate();
            assert_eq!(warnings.len(), 1, "{bad}: {warnings:?}");
            assert!(warnings[0].contains("shortcut"), "{warnings:?}");
            assert_eq!(s.shortcut, Settings::default().shortcut, "{bad}");
        }
    }

    #[test]
    fn wr06_ac2_an_empty_shortcut_is_none_and_a_valid_one_is_kept() {
        let mut none = Settings {
            shortcut: String::new(),
            ..Settings::default()
        };
        assert!(none.validate().is_empty());
        assert_eq!(none.shortcut_combo(), None);

        let mut chosen = Settings {
            shortcut: "Ctrl+Shift+K".to_owned(),
            ..Settings::default()
        };
        assert!(chosen.validate().is_empty());
        assert_eq!(chosen.shortcut, "Ctrl+Shift+K");
        assert_eq!(
            chosen.shortcut_combo().map(|combo| combo.to_string()),
            Some("Ctrl+Shift+K".to_owned())
        );
    }

    #[test]
    fn wr06_ac2_validate_keeps_valid_values_and_an_empty_zone() {
        let mut s = Settings {
            schema: 2,
            language: "ne".to_owned(),
            theme: "dark".to_owned(),
            extra_time_zone: String::new(),
            ..Settings::default()
        };
        assert!(s.validate().is_empty());
        assert_eq!(s.language, "ne");
        assert_eq!(s.theme, "dark");
        assert_eq!(s.extra_time_zone, "");
    }

    #[test]
    fn wr06_ac2_a_bar_line_with_an_unknown_field_takes_that_keys_default() {
        let mut s = Settings {
            bar_bottom: strings(&["bs_date", "sunrise"]),
            ..Settings::default()
        };
        let warnings = s.validate();
        assert_eq!(warnings.len(), 1, "{warnings:?}");
        assert!(warnings[0].contains("bar_bottom"), "{warnings:?}");
        assert_eq!(s.bar_bottom, strings(&["bs_date", "ad_short"]));
        assert_eq!(s.bar_top, strings(&["weekday", "time", "zone_time"]));
    }

    #[test]
    fn wr06_ac2_a_repeated_field_resets_both_bar_lines() {
        let mut s = Settings {
            bar_top: strings(&["time"]),
            bar_bottom: strings(&["bs_date", "time"]),
            ..Settings::default()
        };
        assert_eq!(s.validate().len(), 1);
        assert_eq!(s.bar_top, Settings::default().bar_top);
        assert_eq!(s.bar_bottom, Settings::default().bar_bottom);
    }

    #[test]
    fn wr06_ac2_both_bar_lines_empty_reset_to_the_defaults_but_one_empty_line_is_kept() {
        let mut both = Settings {
            bar_top: Vec::new(),
            bar_bottom: Vec::new(),
            ..Settings::default()
        };
        assert_eq!(both.validate().len(), 1);
        assert_eq!(both.bar_top, Settings::default().bar_top);

        let mut one = Settings {
            bar_top: Vec::new(),
            ..Settings::default()
        };
        assert!(one.validate().is_empty());
        assert!(one.bar_top.is_empty());
    }

    #[test]
    fn wr06_ac2_serde_defaults_missing_keys_and_ignores_unknown() {
        let loaded = Settings::from_toml("language = \"ne\"\nunknown_key = 42\n").expect("toml");
        let parsed = loaded.settings;
        assert_eq!(parsed.language, "ne");
        assert_eq!(parsed.theme, "system");
        assert_eq!(parsed.schema, 2);
        assert!(parsed.launch_at_login);
        assert!(parsed.bar);
        assert!(!loaded.migrated);
        assert!(loaded.warnings.is_empty(), "{:?}", loaded.warnings);
    }

    #[test]
    fn wr06_schema_1_with_the_taskbar_text_off_turns_the_bar_off() {
        let loaded = Settings::from_toml("schema = 1\ntaskbar_text = false\n").expect("toml");
        assert!(loaded.migrated);
        assert_eq!(loaded.settings.schema, 2);
        assert!(!loaded.settings.bar);
        assert!(loaded.warnings.is_empty(), "{:?}", loaded.warnings);
    }

    #[test]
    fn wr06_schema_1_with_the_taskbar_text_on_keeps_the_bar_on_and_defaults_the_rest() {
        let loaded = Settings::from_toml(
            "schema = 1\nlanguage = \"ne\"\nclock_24h = true\ntaskbar_text = true\n",
        )
        .expect("toml");
        assert!(loaded.migrated);
        let s = loaded.settings;
        assert!(s.bar);
        assert_eq!(s.language, "ne");
        assert!(s.clock_24h);
        assert_eq!(s.bar_top, Settings::default().bar_top);
        assert_eq!(s.bar_monitor, "");
    }

    #[test]
    fn wr06_schema_1_with_a_bad_taskbar_text_warns_and_keeps_the_default() {
        let loaded = Settings::from_toml("schema = 1\ntaskbar_text = \"yes\"\n").expect("toml");
        assert!(loaded.migrated);
        assert!(loaded.settings.bar);
        assert_eq!(loaded.warnings.len(), 1, "{:?}", loaded.warnings);
        assert!(loaded.warnings[0].contains("taskbar_text"));
    }

    #[test]
    fn wr06_schema_2_ignores_a_leftover_taskbar_text() {
        let loaded = Settings::from_toml("schema = 2\ntaskbar_text = false\n").expect("toml");
        assert!(!loaded.migrated);
        assert!(loaded.settings.bar, "only a schema 1 file is migrated");
    }

    #[test]
    fn wr06_a_schema_from_the_future_warns_and_is_read_as_the_current_one() {
        let loaded = Settings::from_toml("schema = 9\n").expect("toml");
        assert!(!loaded.migrated);
        assert_eq!(loaded.settings.schema, 2);
        assert_eq!(loaded.warnings.len(), 1, "{:?}", loaded.warnings);
    }

    #[test]
    fn wr06_the_bar_keys_round_trip_through_toml() {
        let s = Settings {
            bar: false,
            bar_locked: true,
            bar_top: strings(&["tithi"]),
            bar_bottom: strings(&["holiday", "ad_date"]),
            bar_monitor: r"\\.\DISPLAY2".to_owned(),
            bar_x: -40,
            bar_y: 12,
            shortcut: "Alt+Shift+F5".to_owned(),
            ..Settings::default()
        };
        let text = toml::to_string_pretty(&s).expect("serialize");
        let loaded = Settings::from_toml(&text).expect("parse");
        assert_eq!(loaded.settings, s);
        assert!(loaded.warnings.is_empty(), "{:?}", loaded.warnings);
    }

    /// Every `msgstr` in the Nepali translation file must be non-empty, except the header
    /// block's own `msgstr ""`. An empty translation would silently fall back to English
    /// (ADR-0008 A.3), so this test fails the build of a half-filled `.po` file.
    #[test]
    fn po_file_has_no_empty_translations() {
        let po = include_str!("../../i18n/ne/LC_MESSAGES/mitibar.po");
        let mut header_seen = false;
        for entry in po.split("\n\n") {
            let Some(msgstr) = entry.lines().find(|l| l.starts_with("msgstr ")) else {
                continue;
            };
            let value = msgstr.trim_start_matches("msgstr ").trim_matches('"');
            if value.is_empty() {
                assert!(!header_seen, "empty translation in entry:\n{entry}");
                header_seen = true;
            }
        }
        assert!(header_seen, "no header block in the .po file");
    }
}
