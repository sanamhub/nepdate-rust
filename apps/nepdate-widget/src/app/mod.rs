//! Application layer: pure Rust, no Slint, no OS calls (ADR-0015). Depends on `nepdate`,
//! `nepdate-patro` and `jiff` only. W1 to W3 fill these modules.

pub mod clock;
mod converter;
mod holidays;
mod month_view;
pub mod settings;
