//! Settings struct, defaults and validation, no file IO (WR-06). W1 holds the plain struct and
//! the WR-06 defaults; W3 adds serde, file IO and validation.

use nepdate::Lang;
use serde::{Deserialize, Serialize};

/// Per-user settings (WR-06). The `language` and `theme` values stay strings so the TOML file
/// reads and writes them as written. The four bools are the four bool keys of WR-06, so the lint
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
    pub taskbar_text: bool,
}

impl Settings {
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
        warnings
    }
}

impl Default for Settings {
    fn default() -> Self {
        Settings {
            schema: 1,
            language: "en".to_owned(),
            theme: "system".to_owned(),
            clock_24h: false,
            show_seconds: false,
            extra_time_zone: "Asia/Kathmandu".to_owned(),
            launch_at_login: true,
            taskbar_text: true,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn defaults_match_wr06() {
        let s = Settings::default();
        assert_eq!(s.schema, 1);
        assert_eq!(s.language, "en");
        assert_eq!(s.theme, "system");
        assert!(!s.clock_24h);
        assert!(!s.show_seconds);
        assert_eq!(s.extra_time_zone, "Asia/Kathmandu");
        assert!(s.launch_at_login);
        assert!(s.taskbar_text);
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
            ..Settings::default()
        };
        let warnings = s.validate();
        assert_eq!(warnings.len(), 4, "{warnings:?}");
        assert_eq!(s, Settings::default());
    }

    #[test]
    fn wr06_ac2_validate_keeps_valid_values_and_an_empty_zone() {
        let mut s = Settings {
            schema: 1,
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
    fn wr06_ac2_serde_defaults_missing_keys_and_ignores_unknown() {
        let parsed: Settings =
            toml::from_str("language = \"ne\"\nunknown_key = 42\n").expect("toml");
        assert_eq!(parsed.language, "ne");
        assert_eq!(parsed.theme, "system");
        assert_eq!(parsed.schema, 1);
        assert!(parsed.launch_at_login);
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
