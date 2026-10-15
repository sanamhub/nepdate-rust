//! Composition root: wires platform + app + ui, starts the event loop (ADR-0015). W1 adds the
//! tray, the popup and the once-per-minute timer.

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

use std::cell::RefCell;
use std::path::PathBuf;
use std::rc::Rc;

use nepdate::{DateOrder, Lang, LongDate, NepaliDate, Separator};
use slint::{ComponentHandle, Weak};

use crate::app::clock;
use crate::app::settings::Settings;
use crate::platform::tray::{SlintTray, Tray, TrayEvent};

slint::include_modules!();

/// The shared state (ADR-0015): settings now, the viewed month and selection from W2 on.
struct AppState {
    settings: Settings,
}

/// Command-line flags. `theme` and `lang` override the settings, for snapshot determinism.
#[derive(Debug, Default)]
struct Cli {
    font_test: bool,
    popup: bool,
    snapshot: Option<PathBuf>,
    theme: Option<ThemeChoice>,
    lang: Option<Lang>,
}

impl Cli {
    fn parse(args: impl Iterator<Item = String>) -> anyhow::Result<Self> {
        let mut cli = Cli::default();
        let mut args = args.peekable();
        while let Some(arg) = args.next() {
            match arg.as_str() {
                "--font-test" => cli.font_test = true,
                "--popup" => cli.popup = true,
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
                    cli.theme = Some(match value.as_str() {
                        "light" => ThemeChoice::Light,
                        "dark" => ThemeChoice::Dark,
                        "system" => ThemeChoice::System,
                        other => {
                            anyhow::bail!("--theme must be light, dark or system, got {other}")
                        }
                    });
                }
                "--lang" => {
                    let value = args
                        .next()
                        .ok_or_else(|| anyhow::anyhow!("--lang needs en or ne"))?;
                    cli.lang = Some(match value.as_str() {
                        "en" => Lang::En,
                        "ne" => Lang::Ne,
                        other => anyhow::bail!("--lang must be en or ne, got {other}"),
                    });
                }
                other => anyhow::bail!("unknown argument: {other}"),
            }
        }
        if cli.snapshot.is_some() && !cli.font_test && !cli.popup {
            anyhow::bail!("--snapshot needs --font-test or --popup");
        }
        if cli.popup && cli.snapshot.is_none() {
            anyhow::bail!("--popup needs --snapshot");
        }
        if cli.font_test && cli.popup {
            anyhow::bail!("--font-test and --popup are exclusive");
        }
        Ok(cli)
    }
}

fn main() -> anyhow::Result<()> {
    let cli = Cli::parse(std::env::args().skip(1))?;

    // W0 Devanagari spike window (unchanged).
    if cli.font_test {
        let ui = FontTestWindow::new()?;
        theme::apply(&ui, cli.theme.unwrap_or(ThemeChoice::System));
        match cli.snapshot {
            Some(path) => snapshot(&ui, &path)?,
            None => ui.run()?,
        }
        return Ok(());
    }

    let mut settings = Settings::default();
    if let Some(lang) = cli.lang {
        settings.language = match lang {
            Lang::Ne => String::from("ne"),
            Lang::En => String::from("en"),
        };
    }
    let lang = settings.lang();

    // Keep the popup out of the Windows taskbar (W1-04). The hook runs before winit creates the
    // window, so it has to be selected before any window is constructed.
    #[cfg(windows)]
    {
        use slint::winit_030::winit::platform::windows::WindowAttributesExtWindows;
        slint::BackendSelector::new()
            .with_winit_window_attributes_hook(|attrs| attrs.with_skip_taskbar(true))
            .select()?;
    }

    let state = Rc::new(RefCell::new(AppState { settings }));
    let popup = PopupWindow::new()?;
    theme::apply(
        &popup,
        cli.theme
            .unwrap_or_else(|| ui_bridge::theme_choice(&state.borrow().settings.theme)),
    );

    // Switch the language after the first component exists so the runtime context is up
    // (ADR-0008 A.3). English is the source language, so only Nepali has a bundle; the `@tr`
    // bindings update live.
    if lang == Lang::Ne {
        slint::select_bundled_translation("ne")?;
    }

    // Snapshot the popup (W1 test tool): fill it with today's header and clocks, then render.
    if cli.popup {
        let path = cli
            .snapshot
            .clone()
            .ok_or_else(|| anyhow::anyhow!("--popup needs --snapshot"))?;
        update_popup(&state, &popup.as_weak());
        return snapshot(&popup, &path);
    }

    let popup_weak = popup.as_weak();

    // Tray (W1-03): WR-01 events delivered back here.
    let tray = Rc::new(SlintTray::new(lang, {
        let state = state.clone();
        let popup_weak = popup_weak.clone();
        move |event| handle_tray_event(event, &state, &popup_weak)
    })?);

    // First paint of the tray, then the once-per-minute (or per-second) timer (W1-05).
    update_tray(&state, &tray);
    arm_timer(state.clone(), popup_weak.clone(), tray);

    slint::run_event_loop()?;
    Ok(())
}

/// Handles a WR-01 tray event. `Settings` opens no panel in W1 (W3 builds it).
fn handle_tray_event(
    event: TrayEvent,
    state: &Rc<RefCell<AppState>>,
    popup_weak: &Weak<PopupWindow>,
) {
    match event {
        TrayEvent::Open => {
            let Some(popup) = popup_weak.upgrade() else {
                return;
            };
            if popup.window().is_visible() {
                let _ = popup.hide();
            } else {
                update_popup(state, popup_weak);
                let _ = ui_bridge::show_popup_at(&popup, ui_bridge::PopupAnchor::TrayCorner);
            }
        }
        TrayEvent::CopyBs => copy_bs(&state.borrow().settings),
        TrayEvent::CopyAd => copy_ad(),
        TrayEvent::Settings => {}
        TrayEvent::Quit => {
            let _ = slint::quit_event_loop();
        }
    }
}

/// Writes today's BS date as `YYYY/MM/DD` to the clipboard (WR-01 AC5).
fn copy_bs(s: &Settings) {
    let now = jiff::Zoned::now();
    if let Ok(bs) = NepaliDate::try_from(now.date()) {
        let text = bs
            .short(DateOrder::Ymd, Separator::Slash, true, s.lang())
            .to_string();
        let _ = platform::clipboard::set_text(&text);
    }
}

/// Writes today's AD date as `YYYY-MM-DD` to the clipboard (WR-01 AC5).
fn copy_ad() {
    let now = jiff::Zoned::now();
    if let Ok(bs) = NepaliDate::try_from(now.date()) {
        let _ = platform::clipboard::set_text(&bs.to_gregorian().to_string());
    }
}

/// Refreshes the popup header and clock lines from the local date and time.
fn update_popup(state: &Rc<RefCell<AppState>>, popup_weak: &Weak<PopupWindow>) {
    let Some(popup) = popup_weak.upgrade() else {
        return;
    };
    let s = state.borrow();
    let now = jiff::Zoned::now();
    let header = clock::header(&now, s.settings.lang());
    let lines = clock::clock_lines(&now, &s.settings);
    ui_bridge::set_header(&popup, &header);
    ui_bridge::set_clocks(&popup, &lines);
}

/// Refreshes the tray title (macOS), day icon (Windows/Linux) and tooltip.
fn update_tray(state: &Rc<RefCell<AppState>>, tray: &Rc<SlintTray>) {
    let s = state.borrow();
    let now = jiff::Zoned::now();
    let lang = s.settings.lang();
    let Ok(bs) = NepaliDate::try_from(now.date()) else {
        return;
    };
    tray.set_title(&format!(
        "{} {}",
        lang.digits(u32::from(bs.day()), 1),
        bs.month().short_name(lang)
    ));
    tray.set_icon_day(&bs.day().to_string());
    tray.set_tooltip(&format!(
        "{} · {}",
        bs.long(
            LongDate {
                weekday: false,
                year: true,
                pad: false
            },
            lang
        ),
        bs.to_gregorian(),
    ));
}

/// One tick of the W1 timer: tray, then popup, then re-arms for the next boundary (WR-05 AC5/AC6).
fn update_tick(
    state: &Rc<RefCell<AppState>>,
    popup_weak: &Weak<PopupWindow>,
    tray: &Rc<SlintTray>,
) {
    update_tray(state, tray);
    update_popup(state, popup_weak);
}

/// Schedules the next single-shot tick. The closure re-arms itself, so the process sleeps about
/// 60 s between wakes when seconds are hidden (WR-05 AC5).
fn arm_timer(state: Rc<RefCell<AppState>>, popup_weak: Weak<PopupWindow>, tray: Rc<SlintTray>) {
    let now = jiff::Zoned::now();
    let show_seconds = state.borrow().settings.show_seconds;
    let delay = clock::next_wakeup(&now, show_seconds);
    slint::Timer::single_shot(delay, move || {
        update_tick(&state, &popup_weak, &tray);
        arm_timer(state, popup_weak, tray);
    });
}

/// Renders a window and writes it as a PNG (W0 tool, reused for the popup). Uses Slint's
/// `Window::take_snapshot` and the `image` crate; the window is sized on the first event-loop
/// turn, so the snapshot runs from a timer once `Window::size()` is non-zero.
fn snapshot<C: ComponentHandle + 'static>(ui: &C, path: &std::path::Path) -> anyhow::Result<()> {
    use std::cell::Cell;

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
                    *result.borrow_mut() =
                        Some(Err(anyhow::anyhow!("window closed before the snapshot")));
                    let _ = slint::quit_event_loop();
                    return;
                };
                let size = ui.window().size();
                if size.width == 0 || size.height == 0 {
                    if attempt >= 20 {
                        *result.borrow_mut() =
                            Some(Err(anyhow::anyhow!("window never got a size")));
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
fn write_snapshot<C: ComponentHandle>(ui: &C, path: &std::path::Path) -> anyhow::Result<()> {
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
