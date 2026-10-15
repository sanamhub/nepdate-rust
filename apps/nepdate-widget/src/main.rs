//! Composition root: wires the layers together and starts the Slint event loop (ADR-0015).
//! W0 shows today's BS date in both languages and hosts the Devanagari spike window; the app
//! and platform modules stay empty until W1 to W3.

#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]
// Slint's `include_modules!` generated component code has pub items without docs, and uses
// unwrap, expect, panic and direct indexing. A binary has no public API, so these lints are
// allowed crate-wide rather than failing the workspace gate on generated code.
#![allow(
    missing_docs,
    unreachable_pub,
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing
)]

mod app;
mod platform;
mod theme;
mod ui_bridge;

use std::path::PathBuf;

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

#[derive(Debug)]
struct Cli {
    font_test: bool,
    snapshot: Option<PathBuf>,
    theme: ThemeChoice,
}

impl Cli {
    fn parse(args: impl Iterator<Item = String>) -> anyhow::Result<Self> {
        let mut cli = Cli {
            font_test: false,
            snapshot: None,
            theme: ThemeChoice::System,
        };
        let mut args = args.peekable();
        while let Some(arg) = args.next() {
            match arg.as_str() {
                "--font-test" => cli.font_test = true,
                "--snapshot" => {
                    let path = args
                        .next()
                        .ok_or_else(|| anyhow::anyhow!("--snapshot needs a path"))?;
                    cli.snapshot = Some(PathBuf::from(path));
                }
                "--theme" => {
                    let value = args
                        .next()
                        .ok_or_else(|| anyhow::anyhow!("--theme needs light, dark or system"))?;
                    cli.theme = match value.as_str() {
                        "light" => ThemeChoice::Light,
                        "dark" => ThemeChoice::Dark,
                        "system" => ThemeChoice::System,
                        other => {
                            anyhow::bail!("--theme must be light, dark or system, got {other}")
                        }
                    };
                }
                other => anyhow::bail!("unknown argument: {other}"),
            }
        }
        if cli.snapshot.is_some() && !cli.font_test {
            anyhow::bail!("--snapshot needs --font-test");
        }
        Ok(cli)
    }
}

/// Renders the font-test window and writes it as a PNG without a manual screen capture. Uses
/// Slint's `Window::take_snapshot` API (an RGBA8 buffer), then encodes with the `image` crate.
///
/// The window gets its real size only after the first event-loop turn, so the snapshot is taken
/// from a timer callback once `Window::size()` is non-zero, with a bounded retry and a clear
/// error if the size never arrives.
fn snapshot_font_test(ui: &FontTestWindow, path: &std::path::Path) -> anyhow::Result<()> {
    use std::cell::{Cell, RefCell};
    use std::rc::Rc;

    ui.show()?;
    let weak = ui.as_weak();
    let path = path.to_path_buf();
    let result: Rc<RefCell<Option<anyhow::Result<()>>>> = Rc::new(RefCell::new(None));
    let attempts: Rc<Cell<u32>> = Rc::new(Cell::new(0));

    let timer = slint::Timer::default();
    timer.start(
        slint::TimerMode::Repeated,
        std::time::Duration::from_millis(50),
        {
            let result = result.clone();
            let attempts = attempts.clone();
            move || {
                let attempt = attempts.get() + 1;
                attempts.set(attempt);
                let Some(ui) = weak.upgrade() else {
                    *result.borrow_mut() = Some(Err(anyhow::anyhow!(
                        "font-test window closed before the snapshot"
                    )));
                    let _ = slint::quit_event_loop();
                    return;
                };
                let size = ui.window().size();
                if size.width == 0 || size.height == 0 {
                    if attempt >= 20 {
                        *result.borrow_mut() =
                            Some(Err(anyhow::anyhow!("font-test window never got a size")));
                        let _ = slint::quit_event_loop();
                    }
                    return;
                }
                *result.borrow_mut() = Some(write_snapshot(&ui, &path));
                let _ = slint::quit_event_loop();
            }
        },
    );

    ui.run()?;
    drop(timer);

    result
        .borrow_mut()
        .take()
        .unwrap_or_else(|| Err(anyhow::anyhow!("snapshot timer never fired")))
}

/// Encodes the window's RGBA8 snapshot buffer as a PNG at `path`.
fn write_snapshot(ui: &FontTestWindow, path: &std::path::Path) -> anyhow::Result<()> {
    let buffer = ui.window().take_snapshot()?;
    let (width, height) = (buffer.width(), buffer.height());
    let img =
        image::RgbaImage::from_raw(width, height, buffer.as_bytes().to_vec()).ok_or_else(|| {
            anyhow::anyhow!("snapshot buffer is {width}x{height} but the byte count is wrong")
        })?;
    img.save_with_format(path, image::ImageFormat::Png)?;
    println!("wrote {}", path.display());
    Ok(())
}

fn main() -> anyhow::Result<()> {
    let cli = Cli::parse(std::env::args().skip(1))?;
    if cli.font_test {
        let ui = FontTestWindow::new()?;
        theme::apply(&ui, cli.theme);
        match cli.snapshot {
            Some(path) => snapshot_font_test(&ui, &path)?,
            None => ui.run()?,
        }
    } else {
        let today = today()?;
        let ui = AppWindow::new()?;
        theme::apply(&ui, cli.theme);
        ui.set_bs_en(today.en.into());
        ui.set_bs_ne(today.ne.into());
        ui.run()?;
    }
    Ok(())
}
