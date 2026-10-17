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
#[cfg(feature = "snapshot")]
use std::path::PathBuf;
use std::rc::Rc;

use nepdate::{DateOrder, Lang, LongDate, Month, NepaliDate, Separator};
use slint::{ComponentHandle, Weak};

use crate::app::settings::Settings;
use crate::app::taskbar::taskbar_lines;
use crate::app::{APP_NAME, clock, holidays, month_view};
use crate::platform::taskbar::{MenuItem, TaskbarEvent, TaskbarText};
use crate::platform::tray::{SlintTray, Tray, TrayEvent};

slint::include_modules!();

/// The shared state (ADR-0015): settings, the viewed month and the popup's last hide.
struct AppState {
    settings: Settings,
    /// When focus loss last hid the popup; see `handle_tray_event`.
    popup_hidden_at: Option<std::time::Instant>,
    /// The month the grid shows (WR-03 AC6).
    viewed_month: (u16, Month),
    /// The local BS date when the popup last hid; a different date on reopen resets the month
    /// (WR-02 AC2, W2-06).
    hidden_on: NepaliDate,
    /// The open day detail, if any (WR-04); cleared on hide and navigation.
    detail: Option<month_view::DayDetail>,
}

/// A tray click within this long after focus loss hid the popup was the click that took the focus
/// away, so it means "close", not "open again".
const REOPEN_GUARD: std::time::Duration = std::time::Duration::from_millis(400);

/// Command-line flags. `theme` and `lang` override the settings, for snapshot determinism.
#[derive(Debug, Default)]
struct Cli {
    font_test: bool,
    popup: bool,
    #[cfg(feature = "snapshot")]
    snapshot: Option<PathBuf>,
    theme: Option<ThemeChoice>,
    lang: Option<Lang>,
    no_taskbar_text: bool,
    #[cfg(all(windows, feature = "snapshot"))]
    taskbar_snapshot: Option<PathBuf>,
    #[cfg(all(windows, feature = "snapshot"))]
    taskbar_theme: Option<bool>, // Some(true) = dark taskbar
    #[cfg(all(windows, feature = "snapshot"))]
    taskbar_scale: Option<f32>,
}

impl Cli {
    fn parse(args: impl Iterator<Item = String>) -> anyhow::Result<Self> {
        let mut cli = Cli::default();
        let mut args = args.peekable();
        while let Some(arg) = args.next() {
            match arg.as_str() {
                "--font-test" => cli.font_test = true,
                "--popup" => cli.popup = true,
                "--no-taskbar-text" => cli.no_taskbar_text = true,
                #[cfg(feature = "snapshot")]
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
                #[cfg(all(windows, feature = "snapshot"))]
                "--taskbar-snapshot" => {
                    let path = args
                        .next()
                        .ok_or_else(|| anyhow::anyhow!("--taskbar-snapshot needs a path"))?;
                    cli.taskbar_snapshot = Some(PathBuf::from(path));
                }
                #[cfg(all(windows, feature = "snapshot"))]
                "--taskbar-theme" => {
                    let value = args
                        .next()
                        .ok_or_else(|| anyhow::anyhow!("--taskbar-theme needs light or dark"))?;
                    cli.taskbar_theme = Some(match value.as_str() {
                        "light" => false,
                        "dark" => true,
                        other => {
                            anyhow::bail!("--taskbar-theme must be light or dark, got {other}")
                        }
                    });
                }
                #[cfg(all(windows, feature = "snapshot"))]
                "--scale" => {
                    let value = args
                        .next()
                        .ok_or_else(|| anyhow::anyhow!("--scale needs 1, 1.5 or 2"))?;
                    cli.taskbar_scale = Some(match value.as_str() {
                        "1" => 1.0,
                        "1.5" => 1.5,
                        "2" => 2.0,
                        other => anyhow::bail!("--scale must be 1, 1.5 or 2, got {other}"),
                    });
                }
                other => anyhow::bail!("unknown argument: {other}"),
            }
        }
        #[cfg(feature = "snapshot")]
        {
            if cli.snapshot.is_some() && !cli.font_test && !cli.popup {
                anyhow::bail!("--snapshot needs --font-test or --popup");
            }
            if cli.popup && cli.snapshot.is_none() {
                anyhow::bail!("--popup needs --snapshot");
            }
        }
        if cli.font_test && cli.popup {
            anyhow::bail!("--font-test and --popup are exclusive");
        }
        Ok(cli)
    }
}

fn main() -> anyhow::Result<()> {
    // Best-effort: the app still runs if the file logger can't start; the panic hook is
    // installed regardless (W1-06).
    let _ = platform::logging::init();

    let cli = Cli::parse(std::env::args().skip(1))?;

    // W0 Devanagari spike window (unchanged).
    if cli.font_test {
        let ui = FontTestWindow::new()?;
        theme::apply(&ui, cli.theme.unwrap_or(ThemeChoice::System));
        #[cfg(feature = "snapshot")]
        if let Some(path) = cli.snapshot {
            return snapshot(&ui, &path);
        }
        ui.run()?;
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

    // Taskbar snapshot (W1B-04 test tool): render the text and composite it over a taskbar
    // strip, exactly as the controller sends it to `UpdateLayeredWindow`.
    #[cfg(all(windows, feature = "snapshot"))]
    if let Some(path) = cli.taskbar_snapshot.clone() {
        let now = jiff::Zoned::now();
        let lines = taskbar_lines(&now, &settings);
        let dark = cli
            .taskbar_theme
            .unwrap_or_else(|| platform::taskbar_scheme::taskbar_is_dark().unwrap_or(false));
        return write_taskbar_snapshot(&lines, cli.taskbar_scale.unwrap_or(1.0), dark, &path);
    }

    // Keep the popup out of the Windows taskbar (W1-04). The hook runs before winit creates the
    // window, so it has to be selected before any window is constructed.
    #[cfg(windows)]
    {
        use slint::winit_030::winit::platform::windows::WindowAttributesExtWindows;
        slint::BackendSelector::new()
            .with_winit_window_attributes_hook(|attrs| attrs.with_skip_taskbar(true))
            .select()?;
    }

    let today = today_local();
    let state = Rc::new(RefCell::new(AppState {
        settings,
        popup_hidden_at: None,
        viewed_month: (today.year(), today.month()),
        hidden_on: today,
        detail: None,
    }));
    // The `--no-taskbar-text` flag turns WR-12 off for this run (W3 builds the settings UI).
    if cli.no_taskbar_text {
        state.borrow_mut().settings.taskbar_text = false;
    }
    let popup = PopupWindow::new()?;
    // The popup title (Alt+Tab) and every later window title share this one value (ADR-0017).
    ui_bridge::set_app_name(&popup);
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
    #[cfg(feature = "snapshot")]
    if cli.popup {
        let path = cli
            .snapshot
            .clone()
            .ok_or_else(|| anyhow::anyhow!("--popup needs --snapshot"))?;
        update_popup(&state, &popup.as_weak());
        return snapshot(&popup, &path);
    }

    let popup_weak = popup.as_weak();
    wire_popup(&popup, &state, &popup_weak);

    // Tray (W1-03): WR-01 events delivered back here.
    let tray = Rc::new(SlintTray::new(lang, {
        let state = state.clone();
        let popup_weak = popup_weak.clone();
        move |event| handle_tray_event(event, &state, &popup_weak)
    })?);

    // Taskbar text (W1B-03): WR-12 on Windows 11, a no-op elsewhere. Its events run the same
    // actions as the tray menu; its scheme callback keeps the tray icon ink in step.
    let taskbar: Rc<dyn TaskbarText> = Rc::from(platform::taskbar::create(
        {
            let state = state.clone();
            let popup_weak = popup_weak.clone();
            move |event| handle_taskbar_event(event, &state, &popup_weak)
        },
        {
            let tray = tray.clone();
            move |dark| tray.set_dark_taskbar(dark)
        },
    ));
    taskbar.set_menu_labels(&ui_bridge::menu_labels(&popup));
    taskbar.set_enabled(state.borrow().settings.taskbar_text);
    // First taskbar lines, then the timer updates them each tick (WR-12 AC1).
    {
        let s = state.borrow();
        let now = jiff::Zoned::now();
        taskbar.set_lines(&taskbar_lines(&now, &s.settings));
    }

    // First paint of the tray, then the once-per-minute (or per-second) timer (W1-05).
    update_tray(&state, &tray);
    arm_timer(state.clone(), popup_weak.clone(), tray, taskbar);

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
            let just_hidden = state
                .borrow()
                .popup_hidden_at
                .is_some_and(|at| at.elapsed() < REOPEN_GUARD);
            if popup.window().is_visible() {
                record_hidden(state);
                let _ = popup.hide();
            } else if !just_hidden {
                prepare_show(state);
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

/// Handles a WR-12 taskbar event. `Toggle` opens or hides the popup above the text; `Menu` runs
/// the same actions as the tray menu (ADR-0016 point 7).
fn handle_taskbar_event(
    event: TaskbarEvent,
    state: &Rc<RefCell<AppState>>,
    popup_weak: &Weak<PopupWindow>,
) {
    match event {
        TaskbarEvent::Toggle { anchor } => {
            let Some(popup) = popup_weak.upgrade() else {
                return;
            };
            if popup.window().is_visible() {
                record_hidden(state);
                let _ = popup.hide();
            } else {
                prepare_show(state);
                update_popup(state, popup_weak);
                let _ =
                    ui_bridge::show_popup_at(&popup, ui_bridge::PopupAnchor::Above(anchor.into()));
            }
        }
        TaskbarEvent::Menu(item) => {
            let tray_event = match item {
                MenuItem::Open => TrayEvent::Open,
                MenuItem::CopyBs => TrayEvent::CopyBs,
                MenuItem::CopyAd => TrayEvent::CopyAd,
                MenuItem::Settings => TrayEvent::Settings,
                MenuItem::Quit => TrayEvent::Quit,
            };
            handle_tray_event(tray_event, state, popup_weak);
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

/// Refreshes the popup header, clocks and month grid from the local date and viewed month.
fn update_popup(state: &Rc<RefCell<AppState>>, popup_weak: &Weak<PopupWindow>) {
    let Some(popup) = popup_weak.upgrade() else {
        return;
    };
    let s = state.borrow();
    let now = jiff::Zoned::now();
    let lang = s.settings.lang();
    let header = clock::header(&now, lang);
    let lines = clock::clock_lines(&now, &s.settings);
    ui_bridge::set_header(&popup, &header);
    ui_bridge::set_clocks(&popup, &lines);

    let today = NepaliDate::try_from(now.date()).unwrap_or(NepaliDate::MIN);
    let (year, month) = s.viewed_month;
    let started = std::time::Instant::now();
    let view = month_view::month_view(year, month, today, lang);
    log::debug!("month view {} built in {:?}", view.title, started.elapsed());
    ui_bridge::set_month(&popup, &view, lang);
    let holiday = holidays::next_holiday(today, lang);
    ui_bridge::set_next_holiday(&popup, holiday.as_ref());
    ui_bridge::set_detail(&popup, s.detail.as_ref());
}

/// The local BS date, or `MIN` if the system clock is outside 1901..=2199.
fn today_local() -> NepaliDate {
    let now = jiff::Zoned::now();
    NepaliDate::try_from(now.date()).unwrap_or(NepaliDate::MIN)
}

/// Records that the popup hid on today's BS date, so a reopen on a later date resets the month
/// (WR-02 AC2, W2-06). Only the focus-loss path sets `popup_hidden_at`, which guards the reopen.
fn record_hidden(state: &Rc<RefCell<AppState>>) {
    let today = today_local();
    let mut state = state.borrow_mut();
    state.hidden_on = today;
    state.detail = None;
}

/// Prepares the popup for a show: keep the viewed month while the date is unchanged, otherwise
/// jump to today's month (WR-02 AC2, W2-06), and start with the detail closed.
fn prepare_show(state: &Rc<RefCell<AppState>>) {
    let today = today_local();
    let mut state = state.borrow_mut();
    state.viewed_month = month_view::month_on_show(state.viewed_month, state.hidden_on, today);
    state.hidden_on = today;
    state.detail = None;
}

/// Sets the viewed month and repaints the popup.
fn set_month(
    state: &Rc<RefCell<AppState>>,
    popup_weak: &Weak<PopupWindow>,
    year: u16,
    month: Month,
) {
    state.borrow_mut().viewed_month = (year, month);
    state.borrow_mut().detail = None;
    update_popup(state, popup_weak);
}

/// Moves the viewed month by `delta`, stopping at the range edges (WR-03 AC6).
fn change_month(state: &Rc<RefCell<AppState>>, popup_weak: &Weak<PopupWindow>, delta: i32) {
    let (year, month) = state.borrow().viewed_month;
    let Some((year, month)) = month_view::step_month(year, month, delta) else {
        return;
    };
    set_month(state, popup_weak, year, month);
}

/// Wires the popup's hide notification (WR-02 AC2) and month navigation (W2-02). Kept out of
/// `main` so the composition root stays readable.
fn wire_popup(popup: &PopupWindow, state: &Rc<RefCell<AppState>>, popup_weak: &Weak<PopupWindow>) {
    popup.on_hidden({
        let state = state.clone();
        move || {
            let mut state = state.borrow_mut();
            state.popup_hidden_at = Some(std::time::Instant::now());
            state.hidden_on = today_local();
            state.detail = None;
        }
    });

    // W2-03: a day opens the inline detail; Escape or the scrim clears it. The copy buttons use
    // the same clipboard path as the tray.
    popup.on_day_clicked({
        let state = state.clone();
        let popup_weak = popup_weak.clone();
        move |index| open_detail(&state, &popup_weak, index)
    });
    popup.on_detail_closed({
        let state = state.clone();
        let popup_weak = popup_weak.clone();
        move || clear_detail(&state, &popup_weak)
    });
    popup.on_copy_detail_bs({
        let state = state.clone();
        move || copy_detail(&state, true)
    });
    popup.on_copy_detail_ad({
        let state = state.clone();
        move || copy_detail(&state, false)
    });

    popup.on_prev_month({
        let state = state.clone();
        let popup_weak = popup_weak.clone();
        move || change_month(&state, &popup_weak, -1)
    });
    popup.on_next_month({
        let state = state.clone();
        let popup_weak = popup_weak.clone();
        move || change_month(&state, &popup_weak, 1)
    });
    popup.on_today({
        let state = state.clone();
        let popup_weak = popup_weak.clone();
        move || {
            let today = today_local();
            set_month(&state, &popup_weak, today.year(), today.month());
        }
    });
    popup.on_month_picked({
        let state = state.clone();
        let popup_weak = popup_weak.clone();
        move |index| {
            let Some(month) = u8::try_from(index + 1)
                .ok()
                .and_then(|number| Month::try_from(number).ok())
            else {
                return;
            };
            let year = state.borrow().viewed_month.0;
            set_month(&state, &popup_weak, year, month);
        }
    });
    popup.on_year_picked({
        let state = state.clone();
        let popup_weak = popup_weak.clone();
        move |year| {
            let Ok(year) = u16::try_from(year) else {
                return;
            };
            if !(1901..=2199).contains(&year) {
                return;
            }
            let month = state.borrow().viewed_month.1;
            set_month(&state, &popup_weak, year, month);
        }
    });
    popup.on_wheel({
        let state = state.clone();
        let popup_weak = popup_weak.clone();
        move |delta| {
            if delta == 0 {
                return;
            }
            change_month(&state, &popup_weak, if delta > 0 { 1 } else { -1 });
        }
    });
}

/// Opens the detail for a grid cell, if it is a real day (W2-03).
fn open_detail(state: &Rc<RefCell<AppState>>, popup_weak: &Weak<PopupWindow>, index: i32) {
    let (year, month, lang) = {
        let state = state.borrow();
        (
            state.viewed_month.0,
            state.viewed_month.1,
            state.settings.lang(),
        )
    };
    let Some(date) = month_view::date_at(year, month, index) else {
        return;
    };
    state.borrow_mut().detail = Some(month_view::day_detail(date, lang));
    update_popup(state, popup_weak);
}

/// Clears the detail without changing the month (WR-04 AC3).
fn clear_detail(state: &Rc<RefCell<AppState>>, popup_weak: &Weak<PopupWindow>) {
    state.borrow_mut().detail = None;
    if let Some(popup) = popup_weak.upgrade() {
        ui_bridge::set_detail(&popup, None);
    }
}

/// Copies the detail's BS or AD date (WR-04 AC2).
fn copy_detail(state: &Rc<RefCell<AppState>>, bs: bool) {
    let state = state.borrow();
    let Some(detail) = state.detail.as_ref() else {
        return;
    };
    let text = if bs { &detail.copy_bs } else { &detail.copy_ad };
    let _ = platform::clipboard::set_text(text);
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
    // Re-read each tick, so a light/dark switch reaches the icon within a minute; W1B reacts to
    // WM_SETTINGCHANGE at once. Elsewhere the markup follows Slint's palette.
    if let Some(dark) = platform::taskbar_scheme::taskbar_is_dark() {
        tray.set_dark_taskbar(dark);
    }
    tray.set_tooltip(&format!(
        "{APP_NAME}\n{} · {}",
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

/// One tick of the W1 timer: tray, popup, taskbar text, then re-arms (WR-05 AC5/AC6).
fn update_tick(
    state: &Rc<RefCell<AppState>>,
    popup_weak: &Weak<PopupWindow>,
    tray: &Rc<SlintTray>,
    taskbar: &Rc<dyn TaskbarText>,
) {
    update_tray(state, tray);
    update_popup(state, popup_weak);
    let s = state.borrow();
    let now = jiff::Zoned::now();
    taskbar.set_lines(&taskbar_lines(&now, &s.settings));
}

/// Schedules the next single-shot tick. The closure re-arms itself, so the process sleeps about
/// 60 s between wakes when seconds are hidden (WR-05 AC5).
fn arm_timer(
    state: Rc<RefCell<AppState>>,
    popup_weak: Weak<PopupWindow>,
    tray: Rc<SlintTray>,
    taskbar: Rc<dyn TaskbarText>,
) {
    let now = jiff::Zoned::now();
    let show_seconds = state.borrow().settings.show_seconds;
    let delay = clock::next_wakeup(&now, show_seconds);
    slint::Timer::single_shot(delay, move || {
        update_tick(&state, &popup_weak, &tray, &taskbar);
        arm_timer(state, popup_weak, tray, taskbar);
    });
}

/// Renders a window and writes it as a PNG (W0 tool, reused for the popup). Uses Slint's
/// `Window::take_snapshot` and the `image` crate; the window is sized on the first event-loop
/// turn, so the snapshot runs from a timer once `Window::size()` is non-zero.
#[cfg(feature = "snapshot")]
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

/// Renders the taskbar text at `scale` and composites it over a taskbar-coloured strip
/// (`#F3F3F3` light, `#202020` dark, 48 px scaled high), exactly as the controller sends the text
/// to `UpdateLayeredWindow` (W1B-04).
// The strip height and inset are 12 or 48 times a scale of at most a few, so the f32-to-u32 cast
// only drops the fractional rounding.
#[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
#[cfg(all(windows, feature = "snapshot"))]
fn write_taskbar_snapshot(
    lines: &crate::app::taskbar::TaskbarLines,
    scale: f32,
    dark: bool,
    path: &std::path::Path,
) -> anyhow::Result<()> {
    use crate::platform::taskbar::{render_taskbar_text, taskbar_ink};

    let rendered = render_taskbar_text(lines, scale, taskbar_ink(dark))?;
    let strip = (48.0 * scale).round() as u32;
    let inset = (12.0 * scale).round() as u32;
    let width = rendered.width + inset * 2;
    let (blue, green, red) = if dark {
        (0x20u8, 0x20u8, 0x20u8)
    } else {
        (0xF3u8, 0xF3u8, 0xF3u8)
    };
    let mut out = vec![0u8; (width * strip * 4) as usize];
    for pixel in out.as_chunks_mut::<4>().0 {
        pixel[0] = blue;
        pixel[1] = green;
        pixel[2] = red;
        pixel[3] = 255;
    }
    let y_off = (strip - rendered.height) / 2;
    for y in 0..rendered.height {
        for x in 0..rendered.width {
            let src = ((y * rendered.width + x) * 4) as usize;
            let dst = (((y + y_off) * width + x + inset) * 4) as usize;
            // Premultiplied source over opaque background: out = src + bg * (1 - a). The sum is
            // always 0..=255, so the u8 cast only drops the fractional rounding.
            let alpha = u32::from(rendered.pixels[src + 3]);
            for channel in 0..3 {
                let s = u32::from(rendered.pixels[src + channel]);
                let b = u32::from(out[dst + channel]);
                out[dst + channel] = (s + b * (255 - alpha) / 255) as u8;
            }
        }
    }
    let img = image::RgbaImage::from_raw(width, strip, out).ok_or_else(|| {
        anyhow::anyhow!("taskbar snapshot buffer is {width}x{strip} but the byte count is wrong")
    })?;
    img.save_with_format(path, image::ImageFormat::Png)?;
    println!("wrote {}", path.display());
    Ok(())
}

/// Encodes the window's RGBA8 snapshot buffer as a PNG at `path`.
#[cfg(feature = "snapshot")]
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
