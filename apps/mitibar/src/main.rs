//! Composition root: wires the platform, app and UI layers and starts the event loop
//! (ADR-0015).

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
#[cfg(test)]
mod interaction_tests;
mod platform;
#[cfg(test)]
pub(crate) mod test_support;
mod theme;
mod ui_bridge;

use std::cell::RefCell;
use std::path::PathBuf;
use std::rc::Rc;

use nepdate::{DateOrder, Lang, LongDate, Month, NepaliDate, Separator};
use slint::{ComponentHandle, Weak};

use crate::app::bar::bar_lines;
use crate::app::bar_place::Saved;
use crate::app::converter::{self, Direction};
use crate::app::settings::Settings;
use crate::app::{APP_NAME, clock, holidays, month_view};
use crate::platform::bar::{Anchor, Bar, BarEvent, MenuItem};
use crate::platform::clipboard::{Clipboard, SystemClipboard};
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
    /// (WR-02 AC2).
    hidden_on: NepaliDate,
    /// The open day detail, if any (WR-04); cleared on hide and navigation.
    detail: Option<month_view::DayDetail>,
    /// The fixed local date for a snapshot (`--today`) or a test; `None` in the normal app.
    today_override: Option<NepaliDate>,
    /// The clipboard port. Tests inject a fake so they never touch the real clipboard.
    clipboard: Rc<dyn Clipboard>,
    /// Every IANA zone, for the settings filter.
    zones: Vec<String>,
    /// The live tray and bar, so a settings change can reach them.
    tray: Option<Rc<SlintTray>>,
    bar: Option<Rc<dyn Bar>>,
    /// Where the settings panel saves: the per-user file in the app, a temporary file in
    /// a test, `None` in a snapshot run or without a config directory.
    settings_path: Option<PathBuf>,
    /// The popup once it exists. It is created on the first open, not at startup, so the
    /// window and its renderer are not created while the popup has never been shown.
    popup: Weak<PopupWindow>,
    /// The one settings debounce timer, created once and shared by the popup.
    save_timer: Rc<slint::Timer>,
    /// The `--theme` flag, applied when the popup is created. A lazy popup must still honour it.
    theme_override: Option<ThemeChoice>,
}

/// The strong popup handle. `AppState` keeps only a `Weak`, so the popup's own callbacks (which
/// capture `AppState`) do not keep the popup alive through the state. The tray and bar
/// closures do hold this slot and the state, and `AppState` holds the tray, so a reference cycle
/// exists; it lives exactly as long as the process, which never drops these objects before exit.
type PopupSlot = Rc<RefCell<Option<PopupWindow>>>;

impl AppState {
    fn new(
        settings: Settings,
        today: NepaliDate,
        today_override: Option<NepaliDate>,
        clipboard: Rc<dyn Clipboard>,
    ) -> Self {
        Self {
            settings,
            popup_hidden_at: None,
            viewed_month: (today.year(), today.month()),
            hidden_on: today,
            detail: None,
            today_override,
            clipboard,
            zones: Vec::new(),
            tray: None,
            bar: None,
            settings_path: None,
            popup: Weak::default(),
            save_timer: Rc::new(slint::Timer::default()),
            theme_override: None,
        }
    }
}

/// A tray click within this long after focus loss hid the popup was the click that took the focus
/// away, so it means "close", not "open again".
const REOPEN_GUARD: std::time::Duration = std::time::Duration::from_millis(400);

thread_local! {
    /// The UI-thread action a second launch runs. A thread-local holds the non-`Send` `Rc`, so
    /// the listener thread can wake the loop with a closure that captures nothing.
    static OPEN_REQUEST: RefCell<Option<Rc<dyn Fn()>>> = const { RefCell::new(None) };
}

/// Installs the second-launch handler on the UI thread.
pub(crate) fn install_open_request(slot: &PopupSlot, state: &Rc<RefCell<AppState>>) {
    let slot = slot.clone();
    let state = state.clone();
    let handler: Rc<dyn Fn()> = Rc::new(move || {
        open_popup(&slot, &state, ui_bridge::PopupAnchor::TrayCorner);
    });
    OPEN_REQUEST.with(|slot| *slot.borrow_mut() = Some(handler));
}

/// Runs the installed second-launch handler on the UI thread.
pub(crate) fn run_open_request() {
    note_wake();
    // Clone the handler out before calling it: the handler borrows state and may run a callback
    // that reaches this thread-local again.
    let handler = OPEN_REQUEST.with(|slot| slot.borrow().clone());
    if let Some(handler) = handler {
        handler();
    }
}

/// Called on the listener thread when a second instance connects.
fn wake_ui() {
    if let Err(err) = slint::invoke_from_event_loop(run_open_request) {
        log::warn!("single instance: could not wake the UI thread: {err}");
    }
}

/// The idle-wake counter, present only in a `snapshot` build.
#[cfg(feature = "snapshot")]
static IDLE_WAKES: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);

/// Counts one UI wake: a timer tick or a second-launch open. A no-op in the shipped build.
fn note_wake() {
    #[cfg(feature = "snapshot")]
    {
        IDLE_WAKES.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    }
}

#[cfg(feature = "snapshot")]
fn idle_wakes() -> u64 {
    IDLE_WAKES.load(std::sync::atomic::Ordering::Relaxed)
}

/// Creates the popup the first time it is needed. Until then `PopupWindow::new` has not
/// run, so no window or renderer exists for a popup nobody has seen.
fn ensure_popup(slot: &PopupSlot, state: &Rc<RefCell<AppState>>) -> Option<PopupWindow> {
    if let Some(popup) = slot.borrow().as_ref() {
        return Some(popup.clone_strong());
    }
    let popup = match PopupWindow::new() {
        Ok(popup) => popup,
        Err(err) => {
            log::warn!("could not create the popup: {err}");
            return None;
        }
    };
    ui_bridge::set_app_name(&popup);
    let choice = {
        let s = state.borrow();
        s.theme_override
            .unwrap_or_else(|| ui_bridge::theme_choice(&s.settings.theme))
    };
    theme::apply(&popup, choice);
    let weak = popup.as_weak();
    state.borrow_mut().popup = weak.clone();
    wire_popup(&popup, state, &weak);
    let save_timer = state.borrow().save_timer.clone();
    wire_settings(&popup, state, &save_timer);
    *slot.borrow_mut() = Some(popup.clone_strong());
    Some(popup)
}

/// Shows the popup, creating it first on the first open.
fn open_popup(slot: &PopupSlot, state: &Rc<RefCell<AppState>>, anchor: ui_bridge::PopupAnchor) {
    let Some(popup) = ensure_popup(slot, state) else {
        return;
    };
    prepare_show(state);
    update_popup(state);
    if let Err(err) = ui_bridge::show_popup_at(&popup, anchor) {
        log::warn!("could not show the popup: {err}");
    }
}

/// Whether the popup exists and is visible.
fn popup_visible(state: &Rc<RefCell<AppState>>) -> bool {
    state
        .borrow()
        .popup
        .upgrade()
        .is_some_and(|popup| popup.window().is_visible())
}

/// Hides the popup if it exists, recording the hide for the reopen guard.
fn hide_popup(state: &Rc<RefCell<AppState>>) {
    // Upgrade in its own statement: an `if let` keeps the borrow alive for the whole block,
    // and `record_hidden` needs `borrow_mut`.
    let popup = state.borrow().popup.upgrade();
    if let Some(popup) = popup {
        record_hidden(state);
        let _ = popup.hide();
    }
}

/// Command-line flags. `theme` and `lang` override the settings, for snapshot determinism.
#[derive(Debug, Default)]
// The flags are independent switches; a state machine would obscure the parser.
#[allow(clippy::struct_excessive_bools)]
struct Cli {
    font_test: bool,
    popup: bool,
    #[cfg(feature = "snapshot")]
    snapshot: Option<PathBuf>,
    /// A fixed BS date for the popup snapshot; never accepted by the normal app.
    #[cfg(feature = "snapshot")]
    today: Option<NepaliDate>,
    /// A BS date whose day-detail card the snapshot opens.
    #[cfg(feature = "snapshot")]
    detail: Option<NepaliDate>,
    /// Opens the converter overlay in a snapshot.
    #[cfg(feature = "snapshot")]
    converter: bool,
    /// Opens the settings panel in a snapshot.
    #[cfg(feature = "snapshot")]
    settings_panel: bool,
    /// Focuses Next year for a focus-ring snapshot.
    #[cfg(feature = "snapshot")]
    focus_year_next: bool,
    /// Run for this many seconds, then write the UI-wake count and quit. A test tool,
    /// available only with the `snapshot` feature.
    #[cfg(feature = "snapshot")]
    wake_probe: Option<u64>,
    /// Where `--wake-probe` writes its count.
    #[cfg(feature = "snapshot")]
    wake_out: Option<PathBuf>,
    theme: Option<ThemeChoice>,
    lang: Option<Lang>,
    /// Hides the bar for this run, whatever `bar` says in the settings file.
    no_bar: bool,
    /// Use this settings file instead of the per-user one, and do not touch the login item
    /// or the single-instance socket. It is how the idle harness starts a measured process.
    settings_file: Option<PathBuf>,
    #[cfg(all(windows, feature = "snapshot"))]
    bar_snapshot: Option<PathBuf>,
    #[cfg(all(windows, feature = "snapshot"))]
    bar_theme: Option<bool>, // Some(true) = dark taskbar
    #[cfg(all(windows, feature = "snapshot"))]
    bar_scale: Option<f32>,
}

/// Reads one `--today` or `--detail` value. Both flags are snapshot tools, so this helper is too.
#[cfg(feature = "snapshot")]
fn parse_bs_date(
    args: &mut impl Iterator<Item = String>,
    flag: &str,
) -> anyhow::Result<NepaliDate> {
    let value = args
        .next()
        .ok_or_else(|| anyhow::anyhow!("{flag} needs a BS date YYYY-MM-DD"))?;
    value
        .parse()
        .map_err(|err| anyhow::anyhow!("{flag} must be a BS date YYYY-MM-DD: {err}"))
}

impl Cli {
    // One arm per flag is the clearest form; splitting it hides the flag table.
    #[allow(clippy::too_many_lines)]
    fn parse(args: impl Iterator<Item = String>) -> anyhow::Result<Self> {
        let mut cli = Cli::default();
        let mut args = args.peekable();
        while let Some(arg) = args.next() {
            match arg.as_str() {
                "--font-test" => cli.font_test = true,
                "--popup" => cli.popup = true,
                "--no-bar" => cli.no_bar = true,
                "--settings-file" => {
                    let path = args
                        .next()
                        .ok_or_else(|| anyhow::anyhow!("--settings-file needs a path"))?;
                    cli.settings_file = Some(PathBuf::from(path));
                }
                #[cfg(feature = "snapshot")]
                "--snapshot" => {
                    let path = args
                        .next()
                        .ok_or_else(|| anyhow::anyhow!("--snapshot needs a path"))?;
                    cli.snapshot = Some(PathBuf::from(path));
                }
                #[cfg(feature = "snapshot")]
                "--today" => cli.today = Some(parse_bs_date(&mut args, "--today")?),
                #[cfg(feature = "snapshot")]
                "--detail" => cli.detail = Some(parse_bs_date(&mut args, "--detail")?),
                #[cfg(feature = "snapshot")]
                "--converter" => cli.converter = true,
                #[cfg(feature = "snapshot")]
                "--settings" => cli.settings_panel = true,
                #[cfg(feature = "snapshot")]
                "--focus-year-next" => cli.focus_year_next = true,
                #[cfg(feature = "snapshot")]
                "--wake-probe" => {
                    let value = args
                        .next()
                        .ok_or_else(|| anyhow::anyhow!("--wake-probe needs a number of seconds"))?;
                    cli.wake_probe = Some(
                        value
                            .parse()
                            .map_err(|_| anyhow::anyhow!("--wake-probe needs whole seconds"))?,
                    );
                }
                #[cfg(feature = "snapshot")]
                "--wake-out" => {
                    let path = args
                        .next()
                        .ok_or_else(|| anyhow::anyhow!("--wake-out needs a path"))?;
                    cli.wake_out = Some(PathBuf::from(path));
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
                "--bar-snapshot" => {
                    let path = args
                        .next()
                        .ok_or_else(|| anyhow::anyhow!("--bar-snapshot needs a path"))?;
                    cli.bar_snapshot = Some(PathBuf::from(path));
                }
                #[cfg(all(windows, feature = "snapshot"))]
                "--bar-theme" => {
                    let value = args
                        .next()
                        .ok_or_else(|| anyhow::anyhow!("--bar-theme needs light or dark"))?;
                    cli.bar_theme = Some(match value.as_str() {
                        "light" => false,
                        "dark" => true,
                        other => {
                            anyhow::bail!("--bar-theme must be light or dark, got {other}")
                        }
                    });
                }
                #[cfg(all(windows, feature = "snapshot"))]
                "--scale" => {
                    let value = args
                        .next()
                        .ok_or_else(|| anyhow::anyhow!("--scale needs 1, 1.5 or 2"))?;
                    cli.bar_scale = Some(match value.as_str() {
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
            // Both flags are snapshot tools only: the normal app must never pin its date.
            if cli.today.is_some() && !cli.popup {
                anyhow::bail!("--today needs --popup --snapshot");
            }
            if cli.detail.is_some() && !cli.popup {
                anyhow::bail!("--detail needs --popup --snapshot");
            }
            if cli.converter && !cli.popup {
                anyhow::bail!("--converter needs --popup --snapshot");
            }
            if cli.settings_panel && !cli.popup {
                anyhow::bail!("--settings needs --popup --snapshot");
            }
            if cli.focus_year_next && !cli.popup {
                anyhow::bail!("--focus-year-next needs --popup --snapshot");
            }
        }
        #[cfg(feature = "snapshot")]
        {
            if cli.wake_probe.is_some() != cli.wake_out.is_some() {
                anyhow::bail!("--wake-probe and --wake-out go together");
            }
            if cli.wake_probe.is_some() && cli.settings_file.is_none() {
                anyhow::bail!(
                    "--wake-probe needs --settings-file, so it never touches the user's file"
                );
            }
            if cli.wake_probe.is_some() && (cli.popup || cli.font_test || cli.snapshot.is_some()) {
                anyhow::bail!("--wake-probe does not combine with a snapshot window");
            }
        }
        if cli.font_test && cli.popup {
            anyhow::bail!("--font-test and --popup are exclusive");
        }
        Ok(cli)
    }

    /// Whether this run is a snapshot tool, which must not touch the user's login item or
    /// settings file.
    fn is_snapshot_tool(&self) -> bool {
        // A settings-file run is isolated like a snapshot tool: no login item, no socket,
        // and no read or write of the real settings file unless the flag points at it.
        if self.settings_file.is_some() {
            return true;
        }
        #[cfg(feature = "snapshot")]
        if self.popup || self.wake_probe.is_some() {
            return true;
        }
        #[cfg(all(windows, feature = "snapshot"))]
        if self.bar_snapshot.is_some() {
            return true;
        }
        false
    }
}

/// The settings for this run: the per-user file, or the defaults for a snapshot tool so a
/// snapshot image neither reads nor writes `settings.toml`.
fn load_settings(cli: &Cli) -> (Settings, Vec<String>) {
    // The idle harness points at its own file, so the real `%APPDATA%\mitibar` is never
    // read or written. A missing file is created there, which is also what the harness expects.
    if let Some(path) = &cli.settings_file {
        return platform::settings_store::load(path);
    }
    #[cfg(feature = "snapshot")]
    if cli.popup {
        return (Settings::default(), Vec::new());
    }
    #[cfg(all(windows, feature = "snapshot"))]
    if cli.bar_snapshot.is_some() {
        return (Settings::default(), Vec::new());
    }
    let _ = cli;
    platform::settings_store::load_default()
}

// The composition root reads top to bottom: parse, settings, single instance, windows, tray,
// timers. Splitting it would scatter the startup order.
#[allow(clippy::too_many_lines)]
fn main() -> anyhow::Result<()> {
    // The start of the process, so the "tray ready" line measures cold start.
    let startup = std::time::Instant::now();
    // Best-effort: the app still runs if the file logger can't start; the panic hook is
    // installed regardless.
    let _ = platform::logging::init();

    let cli = Cli::parse(std::env::args().skip(1))?;

    // The debug font-test window.
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

    // WR-09 AC1: a second launch signals this one and exits 0 before touching settings, the
    // login item or the UI. A snapshot run never claims the socket, so a snapshot image can be
    // rendered while the app is open.
    // If the socket can't be created at all, the app still starts, without the guard.
    let mut primary = None;
    if !cli.is_snapshot_tool() {
        match platform::single_instance::claim() {
            Ok(platform::single_instance::Claim::Secondary) => return Ok(()),
            Ok(platform::single_instance::Claim::Primary(listener)) => primary = Some(listener),
            Err(err) => log::warn!("single instance: {err}; running without the guard"),
        }
    }

    // WR-06: load the per-user file (or write the defaults), then let the snapshot flags
    // override language and theme for this run only. A snapshot run uses the defaults and never
    // touches the user's settings file.
    let (mut settings, settings_warnings) = load_settings(&cli);
    for warning in settings_warnings {
        log::warn!("settings: {warning}");
    }
    if let Some(lang) = cli.lang {
        settings.language = match lang {
            Lang::Ne => String::from("ne"),
            Lang::En => String::from("en"),
        };
    }
    // The `--no-bar` flag hides the bar for this run.
    if cli.no_bar {
        settings.bar = false;
    }
    // WR-09 AC2: match the login item to the setting. A snapshot never touches it.
    if !cli.is_snapshot_tool() {
        platform::autostart::apply_on_start(settings.launch_at_login);
    }
    let lang = settings.lang();

    // Bar snapshot: render the bar and composite it over a taskbar strip, exactly as the window
    // sends it to `UpdateLayeredWindow`.
    #[cfg(all(windows, feature = "snapshot"))]
    if let Some(path) = cli.bar_snapshot.clone() {
        let lines = snapshot_bar_lines(&settings);
        let dark = cli
            .bar_theme
            .unwrap_or_else(|| platform::taskbar_scheme::taskbar_is_dark().unwrap_or(false));
        return write_bar_snapshot(&lines, cli.bar_scale.unwrap_or(1.0), dark, &path);
    }

    // Keep the popup out of the Windows taskbar. The hook runs before winit creates the
    // window, so it has to be selected before any window is constructed.
    #[cfg(windows)]
    {
        use slint::winit_030::winit::platform::windows::WindowAttributesExtWindows;
        slint::BackendSelector::new()
            .with_winit_window_attributes_hook(|attrs| attrs.with_skip_taskbar(true))
            .select()?;
    }

    #[cfg(feature = "snapshot")]
    let today = cli.today.unwrap_or_else(today_local);
    #[cfg(not(feature = "snapshot"))]
    let today = today_local();
    #[cfg(feature = "snapshot")]
    let today_override = cli.today;
    #[cfg(not(feature = "snapshot"))]
    let today_override = None;
    let state = Rc::new(RefCell::new(AppState::new(
        settings,
        today,
        today_override,
        Rc::new(SystemClipboard),
    )));
    state.borrow_mut().zones = ui_bridge::all_time_zones();
    state.borrow_mut().theme_override = cli.theme;
    if let Some(path) = &cli.settings_file {
        state.borrow_mut().settings_path = Some(path.clone());
    } else if !cli.is_snapshot_tool() {
        state.borrow_mut().settings_path = platform::settings_store::default_path();
    }
    // The popup is created on the first open, not here. The slot holds the one strong
    // handle; `AppState` keeps only a weak one, so the component's callbacks cannot form a cycle.
    let slot: PopupSlot = Rc::new(RefCell::new(None));

    // The snapshot tool needs a window now; the normal app creates it later.
    #[cfg(feature = "snapshot")]
    if cli.popup {
        let popup = PopupWindow::new()?;
        ui_bridge::set_app_name(&popup);
        theme::apply(
            &popup,
            cli.theme
                .unwrap_or_else(|| ui_bridge::theme_choice(&state.borrow().settings.theme)),
        );
        state.borrow_mut().popup = popup.as_weak();
        if lang == Lang::Ne {
            slint::select_bundled_translation("ne")?;
        }
        return snapshot_popup(&cli, &state, &popup, lang);
    }

    // Tray: WR-01 events delivered back here.
    let tray = Rc::new(SlintTray::new(lang, {
        let state = state.clone();
        let slot = slot.clone();
        move |event| handle_tray_event(event, &state, &slot)
    })?);

    // Switch the language after the first component exists so the runtime context is up
    // (ADR-0008 A.3). English is the source language, so only Nepali has a bundle; the `@tr`
    // bindings update live.
    if lang == Lang::Ne {
        slint::select_bundled_translation("ne")?;
    }

    // The bar: WR-08 on Windows, a no-op elsewhere. Its events run the same actions as the tray
    // menu; its scheme callback keeps the tray icon ink in step.
    let bar: Rc<dyn Bar> = Rc::from(platform::bar::create(
        {
            let state = state.clone();
            let slot = slot.clone();
            move |event| handle_bar_event(event, &state, &slot)
        },
        {
            let tray = tray.clone();
            move |dark| tray.set_dark_taskbar(dark)
        },
    ));
    // The labels come from the tray component, which exists before the popup.
    bar.set_menu_labels(&tray.menu_labels());
    // First lines, then the timer updates them each tick (WR-08 AC9). The bar is shown last, so
    // it appears fully configured.
    {
        let s = state.borrow();
        bar.set_locked(s.settings.bar_locked);
        bar.set_position(&s.settings.bar_position());
        bar.set_lines(&bar_lines(&jiff::Zoned::now(), &s.settings));
        bar.set_shown(s.settings.bar);
        tray.set_bar_supported(cfg!(windows));
        tray.set_bar_shown(s.settings.bar);
    }

    // The settings panel needs the live tray and bar for language and bar changes. The debounce
    // timer lives in `AppState`, so a lazily created popup shares it.
    state.borrow_mut().tray = Some(tray.clone());
    state.borrow_mut().bar = Some(bar.clone());

    // First paint of the tray, then the once-per-minute (or per-second) timer.
    update_tray(&state, &tray);
    // The benchmark in docs/benchmarks-widget.md reads this line for cold start.
    log::info!("tray ready in {:?}", startup.elapsed());

    // WR-09 AC1: the listener thread wakes the event loop with `invoke_from_event_loop`; the UI
    // thread then runs the same open path as before. No polling timer remains.
    if let Some(listener) = primary {
        install_open_request(&slot, &state);
        platform::single_instance::start(listener, wake_ui);
    }

    arm_timer(state.clone(), tray, bar);

    // Count UI wakes in a headless window, write the count and quit. Snapshot tool only.
    #[cfg(feature = "snapshot")]
    if let Some(seconds) = cli.wake_probe {
        let out = cli
            .wake_out
            .clone()
            .ok_or_else(|| anyhow::anyhow!("--wake-probe needs --wake-out"))?;
        slint::Timer::single_shot(std::time::Duration::from_secs(seconds), move || {
            let text = format!("seconds={seconds}\nwakes={}\n", idle_wakes());
            if let Err(err) = std::fs::write(&out, text) {
                log::warn!("wake probe: could not write {}: {err}", out.display());
            }
            let _ = slint::quit_event_loop();
        });
    }

    slint::run_event_loop()?;
    // A change made in the last 300 ms before Quit is still waiting for the debounce timer.
    let save_timer = state.borrow().save_timer.clone();
    if save_timer.running() {
        save_settings(&state);
    }
    Ok(())
}

/// Renders one popup snapshot. `--today` and `--detail` are snapshot-only; the normal
/// path never reaches this function, and the parser rejects both without `--popup`.
#[cfg(feature = "snapshot")]
fn snapshot_popup(
    cli: &Cli,
    state: &Rc<RefCell<AppState>>,
    popup: &PopupWindow,
    lang: Lang,
) -> anyhow::Result<()> {
    let path = cli
        .snapshot
        .clone()
        .ok_or_else(|| anyhow::anyhow!("--popup needs --snapshot"))?;
    if let Some(detail) = cli.detail {
        state.borrow_mut().detail = Some(month_view::day_detail(detail, lang));
    }
    update_popup(state);
    if cli.converter {
        let today = today_of(&state.borrow());
        ui_bridge::reset_converter(popup, today, lang);
        popup.set_converter_input("2081/01/01".into());
        let outcome = converter::convert("2081/01/01", Direction::BsToAd, lang);
        ui_bridge::set_converter(popup, &outcome);
        popup.set_converter_visible(true);
    }
    if cli.focus_year_next {
        popup.invoke_focus_next_year();
    }
    if cli.settings_panel {
        if let Some(theme) = cli.theme {
            match theme {
                ThemeChoice::Light => "light",
                ThemeChoice::Dark => "dark",
                ThemeChoice::System => "system",
            }
            .clone_into(&mut state.borrow_mut().settings.theme);
        }
        refresh_settings(popup, state);
        popup.set_settings_visible(true);
    }
    snapshot(popup, &path)
}

/// Handles a WR-01 tray event.
fn handle_tray_event(event: TrayEvent, state: &Rc<RefCell<AppState>>, slot: &PopupSlot) {
    match event {
        TrayEvent::Open => {
            let just_hidden = state
                .borrow()
                .popup_hidden_at
                .is_some_and(|at| at.elapsed() < REOPEN_GUARD);
            if popup_visible(state) {
                hide_popup(state);
            } else if !just_hidden {
                open_popup(slot, state, ui_bridge::PopupAnchor::TrayCorner);
            }
        }
        TrayEvent::CopyBs => copy_bs(state),
        TrayEvent::CopyAd => copy_ad(state),
        TrayEvent::ShowBar => {
            let shown = !state.borrow().settings.bar;
            set_bar_shown(state, shown);
        }
        TrayEvent::Settings => open_settings(state, slot, ui_bridge::PopupAnchor::TrayCorner),
        TrayEvent::Quit => {
            let _ = slint::quit_event_loop();
        }
    }
}

/// Handles a WR-08 bar event (ADR-0019 points 5 and 8). A click toggles the popup, a double-click
/// opens it and never closes it, a drag's end is saved, and the right-click menu runs the same
/// actions as the tray menu, with the popup opening at the bar.
fn handle_bar_event(event: BarEvent, state: &Rc<RefCell<AppState>>, slot: &PopupSlot) {
    match event {
        BarEvent::Toggle(anchor) => {
            // The bar claims the foreground before this runs, which deactivates an open popup. If
            // that deactivation is handled first, the popup is already hidden here, and the click
            // meant "close": the same guard as the tray keeps it from reopening.
            let just_hidden = state
                .borrow()
                .popup_hidden_at
                .is_some_and(|at| at.elapsed() < REOPEN_GUARD);
            if popup_visible(state) {
                hide_popup(state);
            } else if !just_hidden {
                open_popup(slot, state, ui_bridge::PopupAnchor::Bar(anchor));
            }
        }
        BarEvent::Open(anchor) => open_popup_if_hidden(slot, state, anchor),
        BarEvent::Moved(saved) => {
            state.borrow_mut().settings.set_bar_position(&saved);
            save_later(state);
        }
        BarEvent::Menu { item, anchor } => match item {
            MenuItem::Open => open_popup_if_hidden(slot, state, anchor),
            MenuItem::CopyBs => copy_bs(state),
            MenuItem::CopyAd => copy_ad(state),
            // The Bar section of Settings is W8-3; until then Settings holds the bar's switches.
            MenuItem::Customize | MenuItem::Settings => {
                open_settings(state, slot, ui_bridge::PopupAnchor::Bar(anchor));
            }
            MenuItem::Lock => {
                let locked = !state.borrow().settings.bar_locked;
                state.borrow_mut().settings.bar_locked = locked;
                let bar = state.borrow().bar.clone();
                if let Some(bar) = bar {
                    bar.set_locked(locked);
                }
                save_later(state);
            }
            MenuItem::Hide => set_bar_shown(state, false),
            MenuItem::Quit => {
                let _ = slint::quit_event_loop();
            }
        },
    }
}

/// Opens the popup at the bar unless it is already open: a double-click and the menu's Open item
/// leave an open popup open.
fn open_popup_if_hidden(slot: &PopupSlot, state: &Rc<RefCell<AppState>>, anchor: Anchor) {
    if !popup_visible(state) {
        open_popup(slot, state, ui_bridge::PopupAnchor::Bar(anchor));
    }
}

/// Shows or hides the bar (the `bar` setting): the bar itself, the tray's Show bar check mark and
/// the Settings switch follow, and the setting is saved (WR-01 AC5, WR-08 AC6).
fn set_bar_shown(state: &Rc<RefCell<AppState>>, shown: bool) {
    state.borrow_mut().settings.bar = shown;
    let (bar, tray, popup) = {
        let s = state.borrow();
        (s.bar.clone(), s.tray.clone(), s.popup.upgrade())
    };
    if let Some(bar) = bar {
        bar.set_shown(shown);
    }
    if let Some(tray) = tray {
        tray.set_bar_shown(shown);
    }
    if let Some(popup) = popup {
        refresh_settings(&popup, state);
    }
    save_later(state);
}

/// Puts the bar back at its default position and forgets the saved one (WR-08 AC5).
fn reset_bar_position(state: &Rc<RefCell<AppState>>) {
    state
        .borrow_mut()
        .settings
        .set_bar_position(&Saved::default());
    let bar = state.borrow().bar.clone();
    if let Some(bar) = bar {
        bar.set_position(&Saved::default());
    }
    save_later(state);
}

/// Repaints the bar's lines from the clock and settings. A no-op when nothing changed.
fn update_bar(state: &Rc<RefCell<AppState>>) {
    let bar = state.borrow().bar.clone();
    if let Some(bar) = bar {
        let lines = bar_lines(&jiff::Zoned::now(), &state.borrow().settings);
        bar.set_lines(&lines);
    }
}

/// Writes today's BS date as `YYYY/MM/DD` to the clipboard (WR-01 AC5).
fn copy_bs(state: &Rc<RefCell<AppState>>) {
    let state = state.borrow();
    let bs = today_of(&state);
    let text = bs
        .short(
            DateOrder::Ymd,
            Separator::Slash,
            true,
            state.settings.lang(),
        )
        .to_string();
    let _ = state.clipboard.set_text(&text);
}

/// Writes today's AD date as `YYYY-MM-DD` to the clipboard, in the language's digits like Copy
/// BS, so the clipboard is exactly what the header shows (WR-01 AC5).
fn copy_ad(state: &Rc<RefCell<AppState>>) {
    let state = state.borrow();
    let text = app::format::ad_short(today_of(&state).to_gregorian(), state.settings.lang());
    let _ = state.clipboard.set_text(&text);
}

/// Refreshes the popup header, clocks and month grid from the local date and viewed month.
fn update_popup(state: &Rc<RefCell<AppState>>) {
    let weak = state.borrow().popup.clone();
    let Some(popup) = weak.upgrade() else {
        return;
    };
    let s = state.borrow();
    let now = popup_now(&s);
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

/// The date the app calls today: the pinned date in a snapshot or test, otherwise the local date.
fn today_of(s: &AppState) -> NepaliDate {
    s.today_override.unwrap_or_else(today_local)
}

/// The local BS date, or `MIN` if the system clock is outside 1901..=2199.
fn today_local() -> NepaliDate {
    let now = jiff::Zoned::now();
    NepaliDate::try_from(now.date()).unwrap_or(NepaliDate::MIN)
}

/// The local time the popup renders. A snapshot can pin the date with `--today` and the clock to
/// noon, so re-rendering the same image does not change it.
fn popup_now(s: &AppState) -> jiff::Zoned {
    let now = jiff::Zoned::now();
    let Some(today) = s.today_override else {
        return now;
    };
    let ad: jiff::civil::Date = today.into();
    // Noon is a valid local time in nearly every zone, so a pinned date still builds a Zoned.
    let noon = jiff::civil::Time::new(12, 0, 0, 0).unwrap_or(now.time());
    now.with().date(ad).time(noon).build().unwrap_or(now)
}

/// Records that the popup hid on today's BS date, so a reopen on a later date resets the month
/// (WR-02 AC2). Only the focus-loss path sets `popup_hidden_at`, which guards the reopen.
fn record_hidden(state: &Rc<RefCell<AppState>>) {
    let today = today_local();
    let mut state = state.borrow_mut();
    state.hidden_on = today;
    state.detail = None;
}

/// Prepares the popup for a show: keep the viewed month while the date is unchanged, otherwise
/// jump to today's month (WR-02 AC2), and start with the detail closed.
fn prepare_show(state: &Rc<RefCell<AppState>>) {
    let today = today_local();
    let mut state = state.borrow_mut();
    state.viewed_month = month_view::month_on_show(state.viewed_month, state.hidden_on, today);
    state.hidden_on = today;
    state.detail = None;
}

/// Sets the viewed month and repaints the popup.
fn set_month(state: &Rc<RefCell<AppState>>, year: u16, month: Month) {
    state.borrow_mut().viewed_month = (year, month);
    state.borrow_mut().detail = None;
    update_popup(state);
}

/// Moves the viewed month by `delta`, stopping at the range edges (WR-03 AC6).
fn change_month(state: &Rc<RefCell<AppState>>, delta: i32) {
    let (year, month) = state.borrow().viewed_month;
    let Some((year, month)) = month_view::step_month(year, month, delta) else {
        return;
    };
    set_month(state, year, month);
}

/// Wires the popup's hide notification (WR-02 AC2) and month navigation. Kept out of
/// `main` so the composition root stays readable.
fn wire_popup(popup: &PopupWindow, state: &Rc<RefCell<AppState>>, popup_weak: &Weak<PopupWindow>) {
    popup.on_hidden({
        let state = state.clone();
        let popup_weak = popup_weak.clone();
        move || {
            if let Some(popup) = popup_weak.upgrade() {
                popup.set_converter_visible(false);
                popup.set_settings_visible(false);
            }
            let mut state = state.borrow_mut();
            state.popup_hidden_at = Some(std::time::Instant::now());
            state.hidden_on = today_local();
            state.detail = None;
        }
    });

    // A day opens the inline detail; Escape or the scrim clears it. The copy buttons use
    // the same clipboard path as the tray.
    popup.on_day_clicked({
        let state = state.clone();
        move |index| open_detail(&state, index)
    });
    popup.on_detail_closed({
        let state = state.clone();
        move || clear_detail(&state)
    });
    popup.on_copy_detail_bs({
        let state = state.clone();
        move || copy_detail(&state, true)
    });
    popup.on_copy_detail_ad({
        let state = state.clone();
        move || copy_detail(&state, false)
    });

    wire_converter(popup, state, popup_weak);

    popup.on_prev_month({
        let state = state.clone();
        move || change_month(&state, -1)
    });
    popup.on_next_month({
        let state = state.clone();
        move || change_month(&state, 1)
    });
    popup.on_today({
        let state = state.clone();
        move || {
            let today = today_of(&state.borrow());
            set_month(&state, today.year(), today.month());
        }
    });
    popup.on_month_picked({
        let state = state.clone();
        move |index| {
            let Some(month) = u8::try_from(index + 1)
                .ok()
                .and_then(|number| Month::try_from(number).ok())
            else {
                return;
            };
            let year = state.borrow().viewed_month.0;
            set_month(&state, year, month);
        }
    });
    popup.on_year_picked({
        let state = state.clone();
        move |year| {
            let Ok(year) = u16::try_from(year) else {
                return;
            };
            if !(1901..=2199).contains(&year) {
                return;
            }
            let month = state.borrow().viewed_month.1;
            set_month(&state, year, month);
        }
    });
    popup.on_wheel({
        let state = state.clone();
        move |delta| {
            if delta == 0 {
                return;
            }
            change_month(&state, if delta > 0 { 1 } else { -1 });
        }
    });
}

/// The converter overlay. Opening it closes the day detail, and the Days tab starts with
/// today on both sides.
fn wire_converter(
    popup: &PopupWindow,
    state: &Rc<RefCell<AppState>>,
    popup_weak: &Weak<PopupWindow>,
) {
    popup.on_converter_opened({
        let state = state.clone();
        let popup_weak = popup_weak.clone();
        move || {
            let (today, lang) = {
                let state = state.borrow();
                (today_of(&state), state.settings.lang())
            };
            state.borrow_mut().detail = None;
            if let Some(popup) = popup_weak.upgrade() {
                ui_bridge::set_detail(&popup, None);
                ui_bridge::reset_converter(&popup, today, lang);
            }
        }
    });
    popup.on_converter_edited({
        let state = state.clone();
        let popup_weak = popup_weak.clone();
        move |input, direction| {
            let lang = state.borrow().settings.lang();
            let direction = if direction == 0 {
                Direction::BsToAd
            } else {
                Direction::AdToBs
            };
            let outcome = converter::convert(input.as_str(), direction, lang);
            if let Some(popup) = popup_weak.upgrade() {
                ui_bridge::set_converter(&popup, &outcome);
            }
        }
    });
    popup.on_converter_days_edited({
        let state = state.clone();
        let popup_weak = popup_weak.clone();
        move |from, to| {
            let lang = state.borrow().settings.lang();
            let outcome = converter::days_between(from.as_str(), to.as_str(), lang);
            if let Some(popup) = popup_weak.upgrade() {
                ui_bridge::set_converter(&popup, &outcome);
            }
        }
    });
}

/// The settings panel. Each change is applied at once and saved 300 ms later. One
/// callback per key keeps the mapping visible.
#[allow(clippy::too_many_lines)]
fn wire_settings(
    popup: &PopupWindow,
    state: &Rc<RefCell<AppState>>,
    save_timer: &Rc<slint::Timer>,
) {
    popup.on_settings_language({
        let state = state.clone();
        let save_timer = save_timer.clone();
        let popup = popup.as_weak();
        move |index| {
            let language = if index == 1 { "ne" } else { "en" };
            language.clone_into(&mut state.borrow_mut().settings.language);
            let _ = slint::select_bundled_translation(language);
            if let Some(popup) = popup.upgrade() {
                update_popup(&state);
                let (tray, bar) = {
                    let s = state.borrow();
                    (s.tray.clone(), s.bar.clone())
                };
                if let Some(tray) = &tray {
                    update_tray(&state, tray);
                }
                if let (Some(bar), Some(tray)) = (&bar, &tray) {
                    bar.set_menu_labels(&tray.menu_labels());
                }
                update_bar(&state);
                refresh_settings(&popup, &state);
            }
            schedule_save(&save_timer, &state);
        }
    });
    popup.on_settings_theme({
        let state = state.clone();
        let save_timer = save_timer.clone();
        let popup_weak = popup.as_weak();
        move |index| {
            let theme = match index {
                1 => "light",
                2 => "dark",
                _ => "system",
            };
            theme.clone_into(&mut state.borrow_mut().settings.theme);
            if let Some(popup) = popup_weak.upgrade() {
                theme::apply(&popup, ui_bridge::theme_choice(theme));
            }
            schedule_save(&save_timer, &state);
        }
    });
    popup.on_settings_clock({
        let state = state.clone();
        let save_timer = save_timer.clone();
        move |value| {
            state.borrow_mut().settings.clock_24h = value;
            update_popup(&state);
            update_bar(&state);
            schedule_save(&save_timer, &state);
        }
    });
    popup.on_settings_seconds({
        let state = state.clone();
        let save_timer = save_timer.clone();
        move |value| {
            state.borrow_mut().settings.show_seconds = value;
            update_popup(&state);
            update_bar(&state);
            schedule_save(&save_timer, &state);
        }
    });
    popup.on_settings_zone({
        let state = state.clone();
        let save_timer = save_timer.clone();
        let popup = popup.as_weak();
        move |zone| {
            state.borrow_mut().settings.extra_time_zone = zone.to_string();
            update_popup(&state);
            update_bar(&state);
            if let Some(popup) = popup.upgrade() {
                refresh_settings(&popup, &state);
            }
            schedule_save(&save_timer, &state);
        }
    });
    popup.on_settings_launch({
        let state = state.clone();
        let save_timer = save_timer.clone();
        move |value| {
            state.borrow_mut().settings.launch_at_login = value;
            platform::autostart::apply(value);
            schedule_save(&save_timer, &state);
        }
    });
    popup.on_settings_bar({
        let state = state.clone();
        move |value| set_bar_shown(&state, value)
    });
    popup.on_settings_reset_bar({
        let state = state.clone();
        move || reset_bar_position(&state)
    });
    popup.on_settings_filter_changed({
        let state = state.clone();
        let popup = popup.as_weak();
        move |filter| {
            if let Some(popup) = popup.upgrade() {
                let s = state.borrow();
                let zones = ui_bridge::filter_time_zones(&s.zones, filter.as_str());
                ui_bridge::set_settings(&popup, &s.settings, &zones);
            }
        }
    });
    popup.on_settings_closed(|| {});
}

/// Repaints the settings panel from `AppState`, keeping the current zone filter.
fn refresh_settings(popup: &PopupWindow, state: &Rc<RefCell<AppState>>) {
    let filter = popup.get_settings_filter().to_string();
    let s = state.borrow();
    let zones = ui_bridge::filter_time_zones(&s.zones, &filter);
    ui_bridge::set_settings(popup, &s.settings, &zones);
}

/// Saves `settings.toml` 300 ms after the last change.
fn schedule_save(timer: &Rc<slint::Timer>, state: &Rc<RefCell<AppState>>) {
    let state = state.clone();
    timer.start(
        slint::TimerMode::SingleShot,
        std::time::Duration::from_millis(300),
        move || save_settings(&state),
    );
}

/// `schedule_save` for a change that did not come from the settings panel, so it has no timer in
/// hand: the one shared timer lives in `AppState`.
fn save_later(state: &Rc<RefCell<AppState>>) {
    let timer = state.borrow().save_timer.clone();
    schedule_save(&timer, state);
}

/// Writes the current settings to `AppState::settings_path`, if there is one.
fn save_settings(state: &Rc<RefCell<AppState>>) {
    let (settings, path) = {
        let s = state.borrow();
        (s.settings.clone(), s.settings_path.clone())
    };
    let Some(path) = path else {
        return;
    };
    if let Err(err) = platform::settings_store::save(&path, &settings) {
        log::warn!("settings: could not save: {err}");
    }
}

/// Opens the settings panel, showing the popup at `anchor` first if it was hidden.
fn open_settings(state: &Rc<RefCell<AppState>>, slot: &PopupSlot, anchor: ui_bridge::PopupAnchor) {
    let Some(popup) = ensure_popup(slot, state) else {
        return;
    };
    if !popup.window().is_visible() {
        prepare_show(state);
        update_popup(state);
        let _ = ui_bridge::show_popup_at(&popup, anchor);
    }
    popup.set_settings_filter("".into());
    refresh_settings(&popup, state);
    popup.set_settings_visible(true);
    popup.invoke_focus();
}

/// Opens the detail for a grid cell, if it is a real day.
fn open_detail(state: &Rc<RefCell<AppState>>, index: i32) {
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
    update_popup(state);
}

/// Clears the detail without changing the month (WR-04 AC3).
fn clear_detail(state: &Rc<RefCell<AppState>>) {
    state.borrow_mut().detail = None;
    let popup = state.borrow().popup.upgrade();
    if let Some(popup) = popup {
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
    let _ = state.clipboard.set_text(text);
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
    // Re-read each tick, so a light/dark switch reaches the icon within a minute; the Windows
    // taskbar reacts to WM_SETTINGCHANGE at once. Elsewhere the markup follows Slint's palette.
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
        app::format::ad_short(bs.to_gregorian(), lang),
    ));
}

/// One tick of the timer: tray, popup, bar, then re-arms (WR-05 AC5/AC6). This is the only timer
/// that runs at idle; the bar adds none (WR-08 AC9).
fn update_tick(state: &Rc<RefCell<AppState>>, tray: &Rc<SlintTray>, bar: &Rc<dyn Bar>) {
    note_wake();
    update_tray(state, tray);
    update_popup(state);
    // The lines are built before the bar redraws, so no state borrow is held across its Win32
    // calls.
    let lines = bar_lines(&jiff::Zoned::now(), &state.borrow().settings);
    bar.set_lines(&lines);
}

/// Schedules the next single-shot tick. The closure re-arms itself, so the process sleeps about
/// 60 s between wakes when seconds are hidden (WR-05 AC5).
fn arm_timer(state: Rc<RefCell<AppState>>, tray: Rc<SlintTray>, bar: Rc<dyn Bar>) {
    let now = jiff::Zoned::now();
    let show_seconds = state.borrow().settings.show_seconds;
    let delay = clock::next_wakeup(&now, show_seconds);
    slint::Timer::single_shot(delay, move || {
        update_tick(&state, &tray, &bar);
        arm_timer(state, tray, bar);
    });
}

/// Renders a window and writes it as a PNG (test tool, reused for the popup). Uses Slint's
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

/// The lines a bar snapshot shows: the moment of the WR-08 AC2 table (Thursday, 2083 Kartik 5,
/// 6:31 PM in Kathmandu) with Singapore as the extra clock, so the image never changes with the
/// clock and shows the defaults. `--lang` picks the language through `settings`.
#[cfg(all(windows, feature = "snapshot"))]
fn snapshot_bar_lines(settings: &Settings) -> crate::app::bar::BarLines {
    let now: jiff::Zoned = "2026-10-22T18:31:00+05:45[Asia/Kathmandu]"
        .parse()
        .expect("a fixed valid timestamp");
    let settings = Settings {
        extra_time_zone: "Asia/Singapore".to_owned(),
        ..settings.clone()
    };
    bar_lines(&now, &settings)
}

/// Renders the bar at `scale` and composites it over a taskbar-coloured strip (`#F3F3F3` light,
/// `#202020` dark, 48 px scaled high), exactly as the window sends it to `UpdateLayeredWindow`.
// The strip height and inset are 12 or 48 times a scale of at most a few, so the f32-to-u32 cast
// only drops the fractional rounding.
#[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
#[cfg(all(windows, feature = "snapshot"))]
fn write_bar_snapshot(
    lines: &crate::app::bar::BarLines,
    scale: f32,
    dark: bool,
    path: &std::path::Path,
) -> anyhow::Result<()> {
    use crate::platform::bar::{bar_palette, render_bar};

    // The renderer caches its DirectWrite and Direct2D objects per thread. Render on a worker,
    // as the bar does, so that cache is released when the worker ends; on the main thread it would
    // be released during process exit, after DirectWrite has shut down.
    let rendered = std::thread::scope(|scope| {
        scope
            .spawn(|| render_bar(lines, scale, bar_palette(dark), false))
            .join()
    })
    .map_err(|_| anyhow::anyhow!("the bar render thread panicked"))??;
    let strip = (48.0 * scale).round() as u32;
    let inset = (12.0 * scale).round() as u32;
    let width = rendered.width + inset * 2;
    let background: u8 = if dark { 0x20 } else { 0xF3 };
    // RGBA bytes: an opaque, grey strip.
    let mut out = vec![background; (width * strip * 4) as usize];
    for pixel in out.as_chunks_mut::<4>().0 {
        pixel[3] = 255;
    }
    let y_off = (strip - rendered.height) / 2;
    for y in 0..rendered.height {
        for x in 0..rendered.width {
            let src = ((y * rendered.width + x) * 4) as usize;
            let dst = (((y + y_off) * width + x + inset) * 4) as usize;
            // The renderer's pixels are premultiplied BGRA; the PNG is RGBA. Premultiplied source
            // over an opaque background: out = src + bg * (1 - a). The sum is always 0..=255, so
            // the u8 cast only drops the fractional rounding.
            let alpha = u32::from(rendered.pixels[src + 3]);
            for (channel, source) in [(0, 2), (1, 1), (2, 0)] {
                let s = u32::from(rendered.pixels[src + source]);
                let b = u32::from(out[dst + channel]);
                out[dst + channel] = (s + b * (255 - alpha) / 255) as u8;
            }
        }
    }
    let img = image::RgbaImage::from_raw(width, strip, out).ok_or_else(|| {
        anyhow::anyhow!("bar snapshot buffer is {width}x{strip} but the byte count is wrong")
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn settings_file_flag_is_parsed_and_isolates_the_run() {
        let cli = Cli::parse(
            ["--settings-file", "C:\\temp\\mitibar-settings.toml"]
                .into_iter()
                .map(str::to_owned),
        )
        .expect("parse");
        assert_eq!(
            cli.settings_file.as_deref(),
            Some(std::path::Path::new("C:\\temp\\mitibar-settings.toml"))
        );
        assert!(cli.is_snapshot_tool());
    }

    #[cfg(feature = "snapshot")]
    #[test]
    fn focus_year_flag_needs_a_popup_snapshot() {
        let cli = Cli::parse(
            ["--popup", "--snapshot", "focus.png", "--focus-year-next"]
                .into_iter()
                .map(str::to_owned),
        )
        .expect("parse");
        assert!(cli.focus_year_next);

        assert!(Cli::parse(["--focus-year-next"].into_iter().map(str::to_owned)).is_err());
        assert!(
            Cli::parse(
                ["--snapshot", "focus.png", "--focus-year-next"]
                    .into_iter()
                    .map(str::to_owned)
            )
            .is_err()
        );
    }
}
