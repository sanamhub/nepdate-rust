//! Windows-only taskbar text (ADR-0016). All `unsafe` Win32 and COM lives in this folder and
//! nowhere else; every block carries a `// SAFETY:` comment and clippy denies undocumented ones.
#![allow(unsafe_code)]
#![deny(clippy::undocumented_unsafe_blocks)]

mod controller;
pub mod render;
pub(crate) mod theme;
mod uia;

pub use controller::WindowsTaskbarText;
