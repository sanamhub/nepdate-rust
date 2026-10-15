//! Maps the design tokens (docs/plan/widget-design.md §2) onto the `Theme` global from
//! `ui/theme.slint`. The token values and the light/dark switch live in that `.slint` file; this
//! module only decides which side of the switch is active and flips it once, so every window
//! follows one scheme.
//!
//! The resolved colours are read as `Theme.surface`, `Theme.ink`, and so on in markup, never
//! re-set here.

use slint::ComponentHandle;

use crate::Theme;

/// Which palette set is active.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Scheme {
    Light,
    Dark,
}

impl Scheme {
    fn is_dark(self) -> bool {
        self == Scheme::Dark
    }
}

/// Detects the system light/dark scheme. Falls back to light when the platform signal is
/// missing, so the widget always starts with a readable palette (light is the lokta paper
/// default in the design).
pub fn system_scheme() -> Scheme {
    #[cfg(target_os = "windows")]
    {
        return windows_scheme();
    }
    #[cfg(target_os = "macos")]
    {
        return macos_scheme();
    }
    #[cfg(all(unix, not(target_os = "macos")))]
    {
        return linux_scheme();
    }
    #[allow(unreachable_code)]
    Scheme::Light
}

/// Flips the `Theme.is-dark` switch on `ui`. Called once per window before it is shown.
pub fn apply<'a, T>(ui: &'a T, scheme: Scheme)
where
    T: ComponentHandle,
    Theme<'a>: slint::Global<'a, T>,
{
    ui.global::<Theme<'a>>().set_is_dark(scheme.is_dark());
}

/// `reg.exe query` output for `AppsUseLightTheme` ends in `0x0` (dark) or `0x1` (light).
/// Pure so the parsing is testable without shelling out.
#[cfg(target_os = "windows")]
fn parse_windows_reg(stdout: &str) -> Scheme {
    if stdout.contains("0x0") {
        Scheme::Dark
    } else {
        Scheme::Light
    }
}

#[cfg(target_os = "windows")]
fn windows_scheme() -> Scheme {
    let out = std::process::Command::new("reg")
        .args([
            "query",
            r"HKCU\Software\Microsoft\Windows\CurrentVersion\Themes\Personalize",
            "/v",
            "AppsUseLightTheme",
        ])
        .output();
    match out {
        Ok(out) if out.status.success() => parse_windows_reg(&String::from_utf8_lossy(&out.stdout)),
        _ => Scheme::Light,
    }
}

/// `defaults read -g AppleInterfaceStyle` prints `Dark` for the dark scheme and errors or
/// prints nothing for light.
#[cfg(target_os = "macos")]
fn parse_macos(stdout: &str) -> Scheme {
    if stdout.trim() == "Dark" {
        Scheme::Dark
    } else {
        Scheme::Light
    }
}

#[cfg(target_os = "macos")]
fn macos_scheme() -> Scheme {
    let out = std::process::Command::new("defaults")
        .args(["read", "-g", "AppleInterfaceStyle"])
        .output();
    match out {
        Ok(out) if out.status.success() => parse_macos(&String::from_utf8_lossy(&out.stdout)),
        _ => Scheme::Light,
    }
}

/// `gsettings get org.gnome.desktop.interface color-scheme` prints `'prefer-dark'` for dark and
/// `'default'` or `'prefer-light'` for light.
#[cfg(all(unix, not(target_os = "macos")))]
fn parse_linux_gsettings(stdout: &str) -> Scheme {
    if stdout.contains("dark") {
        Scheme::Dark
    } else {
        Scheme::Light
    }
}

#[cfg(all(unix, not(target_os = "macos")))]
fn linux_scheme() -> Scheme {
    if let Ok(out) = std::process::Command::new("gsettings")
        .args(["get", "org.gnome.desktop.interface", "color-scheme"])
        .output()
        && out.status.success()
    {
        return parse_linux_gsettings(&String::from_utf8_lossy(&out.stdout));
    }
    // gsettings is not always installed; a dark GTK theme is a weaker but common hint.
    if std::env::var("GTK_THEME")
        .map(|t| t.to_ascii_lowercase().contains("dark"))
        .unwrap_or(false)
    {
        return Scheme::Dark;
    }
    Scheme::Light
}

#[cfg(test)]
mod tests {
    use super::*;

    #[cfg(target_os = "windows")]
    #[test]
    fn windows_reg_dark_and_light() {
        assert_eq!(
            parse_windows_reg("AppsUseLightTheme    REG_DWORD    0x0"),
            Scheme::Dark
        );
        assert_eq!(
            parse_windows_reg("AppsUseLightTheme    REG_DWORD    0x1"),
            Scheme::Light
        );
    }

    #[cfg(target_os = "macos")]
    #[test]
    fn macos_dark_and_light() {
        assert_eq!(parse_macos("Dark\n"), Scheme::Dark);
        assert_eq!(parse_macos("\n"), Scheme::Light);
    }

    #[cfg(all(unix, not(target_os = "macos")))]
    #[test]
    fn linux_gsettings_dark_and_light() {
        assert_eq!(parse_linux_gsettings("'prefer-dark'\n"), Scheme::Dark);
        assert_eq!(parse_linux_gsettings("'default'\n"), Scheme::Light);
    }
}
