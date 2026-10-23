//! Application layer: pure Rust, no Slint, no OS calls (ADR-0015). Depends on `nepdate`,
//! `nepdate-patro` and `jiff` only.

/// The display name wherever a person reads it (ADR-0017). The one Rust copy: the tray tooltip
/// uses it directly, and `ui_bridge::set_app_name` fills the Slint global from it.
pub const APP_NAME: &str = "MitiBar";

pub mod bar;
pub mod bar_place;
pub mod bar_pointer;
pub mod clock;
pub mod converter;
pub mod format;
pub mod goto;
pub mod holidays;
pub mod line_editor;
pub mod month_view;
pub mod settings;
pub mod settings_place;
pub mod shortcut;
pub mod tithi_label;
pub mod zoom;
