//! Windows-only mini bar (ADR-0019). All `unsafe` Win32 and COM lives in this folder and nowhere
//! else; every block carries a `// SAFETY:` comment and clippy denies undocumented ones.
#![allow(unsafe_code)]
#![deny(clippy::undocumented_unsafe_blocks)]

mod bar;
mod foreground;
pub(crate) mod frame;
pub(crate) mod monitors;
pub mod render;
pub(crate) mod theme;

pub use bar::WindowsBar;
