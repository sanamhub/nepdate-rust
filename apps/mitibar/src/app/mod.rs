//! Application layer: pure Rust, no Slint, no OS calls (ADR-0015). Depends on `nepdate`,
//! `nepdate-patro` and `jiff` only. W1 to W3 fill these modules.

/// The display name wherever a person reads it (ADR-0017). The one Rust copy: the tray tooltip
/// uses it directly, and `ui_bridge::set_app_name` fills the Slint global from it.
pub const APP_NAME: &str = "MitiBar";

pub mod clock;
mod converter;
mod holidays;
pub mod month_view;
pub mod settings;
pub mod taskbar;
