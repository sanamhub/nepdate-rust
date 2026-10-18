//! Infrastructure layer: OS integration, thin, behind small traits (ADR-0015). Never imports
//! Slint types except `tray.rs`. W1 to W3 fill these modules.

pub mod autostart;
pub mod clipboard;
pub mod logging;
pub mod settings_store;
pub mod single_instance;
pub mod taskbar;
pub mod taskbar_scheme;
pub mod tray;
#[cfg(windows)]
mod windows;
