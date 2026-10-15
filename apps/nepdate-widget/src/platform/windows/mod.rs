//! Windows-only taskbar text (ADR-0016). All `unsafe` Win32 and COM lives in this folder and
//! nowhere else; every block carries a `// SAFETY:` comment and clippy denies undocumented ones.
// W1B-03 wires these modules into `main.rs`; until then the binary does not construct them. Remove
// this allow when wired.
#![allow(dead_code)]
#![allow(unsafe_code)]
#![deny(clippy::undocumented_unsafe_blocks)]

mod controller;
pub mod render;
mod theme;
mod uia;

pub use controller::WindowsTaskbarText;
