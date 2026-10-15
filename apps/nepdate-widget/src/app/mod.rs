//! Application layer: pure Rust, no Slint, no OS calls (ADR-0015). Depends on `nepdate`,
//! `nepdate-patro` and `jiff` only. W1 to W3 fill these modules.

// W1 fills the app layer before main.rs consumes it (W1-05), so the bin target sees the new
// items as unused until then. Removed once main.rs wires the clock.
#![allow(dead_code)]

pub mod clock;
mod converter;
mod holidays;
mod month_view;
pub mod settings;
