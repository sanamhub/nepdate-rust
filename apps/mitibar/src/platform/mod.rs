//! Infrastructure layer: OS integration, thin, behind small traits (ADR-0015). Never imports
//! Slint types except `tray.rs`.

pub mod autostart;
pub mod bar;
pub mod clipboard;
pub mod frame;
pub mod hotkey;
pub mod logging;
pub mod screen;
pub mod settings_store;
pub mod single_instance;
pub mod taskbar_scheme;
pub mod tray;
#[cfg(windows)]
mod windows;
