//! Composition root: wires the layers together and starts the Slint event loop (ADR-0015).
//! W0 only shows today's BS date in both languages; the app and platform modules stay empty
//! until W1 to W3.

#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]
// Slint's `include_modules!` generated component code has pub items without docs, and uses
// unwrap, expect, panic and direct indexing. A binary has no public API, so these lints are
// allowed crate-wide rather than failing the workspace gate on generated code.
#![allow(
    missing_docs,
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing
)]

mod app;
mod platform;
mod ui_bridge;

use nepdate::{Lang, NepaliDate};

slint::include_modules!();

/// Today's BS date as two strings, one per language. Built from the local calendar date, so it
/// matches WR-05 AC4 from the start.
struct Today {
    en: String,
    ne: String,
}

/// `jiff::Zoned::now().date()` gives the local date; `NepaliDate::from_gregorian` turns it into
/// the BS date. jiff's `i16`/`i8` fields widen with `i32::from` and `u8::try_from`, no `as`.
fn today() -> anyhow::Result<Today> {
    let date = jiff::Zoned::now().date();
    let bs = NepaliDate::from_gregorian(
        i32::from(date.year()),
        u8::try_from(date.month())?,
        u8::try_from(date.day())?,
    )?;
    Ok(Today {
        en: bs.to_string(),
        ne: format!(
            "{}/{}/{}",
            Lang::Ne.digits(u32::from(bs.year()), 4),
            Lang::Ne.digits(u32::from(bs.month_number()), 2),
            Lang::Ne.digits(u32::from(bs.day()), 2)
        ),
    })
}

fn main() -> anyhow::Result<()> {
    let today = today()?;
    let ui = AppWindow::new()?;
    ui.set_bs_en(today.en.into());
    ui.set_bs_ne(today.ne.into());
    ui.run()?;
    Ok(())
}
