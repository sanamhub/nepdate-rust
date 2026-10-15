//! Infrastructure layer: OS integration, thin, behind small traits (ADR-0015). Never imports
//! Slint types except `tray.rs`. W1 to W3 fill these modules.

mod autostart;
pub mod clipboard;
pub mod logging;
mod settings_store;
mod single_instance;
pub mod taskbar_scheme;
pub mod tray;
