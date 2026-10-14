//! Infrastructure layer: OS integration, thin, behind small traits (ADR-0015). Never imports
//! Slint types except `tray.rs`. W1 to W3 fill these modules.

mod autostart;
mod clipboard;
mod logging;
mod settings_store;
mod single_instance;
mod tray;
