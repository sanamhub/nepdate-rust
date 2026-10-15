//! The taskbar's light or dark scheme, which picks the tray icon's ink (W1-03) and, in W1B, the
//! taskbar text colour (ADR-0016 point 5). Windows keeps it apart from the app scheme:
//! `SystemUsesLightTheme` for the taskbar, `AppsUseLightTheme` for apps. It is read through the
//! registry API, never by spawning a process.

/// `Some(true)` for a dark taskbar, `Some(false)` for a light one, `None` where the OS has no
/// separate taskbar scheme (the tray then follows Slint's palette).
#[cfg(windows)]
#[allow(
    clippy::unnecessary_wraps,
    reason = "same signature as the non-Windows version, which returns None"
)]
pub fn taskbar_is_dark() -> Option<bool> {
    let value = windows_registry::CURRENT_USER
        .open(r"Software\Microsoft\Windows\CurrentVersion\Themes\Personalize")
        .and_then(|key| key.get_u32("SystemUsesLightTheme"))
        .ok();
    Some(is_dark(value))
}

#[cfg(not(windows))]
pub fn taskbar_is_dark() -> Option<bool> {
    None
}

/// `SystemUsesLightTheme` is 1 for light and 0 for dark; a missing value means dark, the Windows 10
/// default taskbar (ADR-0016 point 5).
#[cfg_attr(not(windows), allow(dead_code))]
fn is_dark(system_uses_light_theme: Option<u32>) -> bool {
    system_uses_light_theme != Some(1)
}

#[cfg(test)]
mod tests {
    use super::is_dark;

    #[test]
    fn registry_value_to_scheme() {
        assert!(!is_dark(Some(1)));
        assert!(is_dark(Some(0)));
        assert!(is_dark(None));
    }
}
