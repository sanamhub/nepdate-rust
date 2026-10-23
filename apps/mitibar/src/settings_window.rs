//! The Settings window (WR-06): created when it is opened and dropped when it is closed. This
//! module opens and places it, wires its callbacks to the app state, saves what changes, and keeps
//! the popup open while focus moves between the two windows (WR-02 AC2).
//!
//! Every handler follows the W8-1 rule: take the state's borrow in its own statement, and let it
//! end before anything that can call back into the app, such as a Slint property or callback.

use std::cell::RefCell;
use std::rc::Rc;
use std::time::Duration;

use slint::{CloseRequestResponse, ComponentHandle};

use crate::app::bar::BarField;
use crate::app::line_editor::{self, At, Line};
use crate::app::shortcut::{self, Combo, Modifiers, Recorded, Refusal};
use crate::ui_bridge::{self, SettingsHint};
use crate::{AppState, SettingsWindow, theme};

/// How long to wait for the other window of the app to take the focus before a focus loss counts.
/// Focus moving from the popup to the Settings window, or back, is not a loss (WR-02 AC2), but the
/// window that loses focus is told before the one that gains it.
const FOCUS_GRACE: Duration = Duration::from_millis(100);

/// How long the `Saved` note shows after a write (WR-06 AC5).
const SAVED_NOTE: Duration = Duration::from_secs(2);

/// How long after the last change the settings are written (WR-06 AC5).
const SAVE_DELAY: Duration = Duration::from_millis(300);

/// The note in the Settings header (WR-06 AC5).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub(crate) enum SaveStatus {
    #[default]
    Idle,
    /// `Saved`, for 2 s after a successful write.
    Saved,
    /// `Couldn't save settings`, until the next successful write.
    Failed,
}

/// The note under the shortcut field (WR-13).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub(crate) enum ShortcutNote {
    #[default]
    Nothing,
    /// Ctrl+Alt is `AltGr` on many keyboards.
    CtrlAlt,
    /// The combination needs the Win key, or two of Ctrl, Alt and Shift.
    TooFewModifiers,
    /// The OS refused it: another app owns the combination.
    InUse,
    /// Wayland has no global shortcuts.
    NoGlobalShortcuts,
    /// The key typed a character the field can't tell a key from, such as `&` from Shift+7.
    KeyNotSupported,
}

impl ShortcutNote {
    /// The code `SettingsWindow::shortcut-note` reads.
    fn code(self) -> i32 {
        match self {
            ShortcutNote::Nothing => 0,
            ShortcutNote::CtrlAlt => 1,
            ShortcutNote::TooFewModifiers => 2,
            ShortcutNote::InUse => 3,
            ShortcutNote::NoGlobalShortcuts => 4,
            ShortcutNote::KeyNotSupported => 5,
        }
    }
}

/// Which part of the window to show first.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub(crate) enum Section {
    #[default]
    Top,
    /// "Customize bar" scrolls to the Bar section (WR-08 AC6).
    Bar,
}

/// What opening Settings asks for.
#[derive(Debug, Clone, Copy, Default)]
pub(crate) struct Request {
    pub section: Section,
    pub hint: SettingsHint,
}

/// Opens the Settings window, or brings the open one forward (WR-06 AC1). A new window is placed
/// beside the popup, or centred.
pub(crate) fn open(state: &Rc<RefCell<AppState>>, request: Request) {
    let existing = state
        .borrow()
        .settings_window
        .as_ref()
        .map(SettingsWindow::clone_strong);
    let created = existing.is_none();
    let Some(window) = existing.or_else(|| create(state)) else {
        return;
    };
    refresh(state);
    if created {
        let popup = state.borrow().popup.upgrade();
        ui_bridge::place_settings(&window, popup.as_ref(), request.hint);
    }
    if let Err(err) = window.show() {
        log::warn!("could not show the Settings window: {err}");
        return;
    }
    if created {
        watch_focus(&window, state);
    }
    if request.section == Section::Bar {
        window.invoke_show_bar_section();
    }
}

/// Builds the window and wires it. It is kept in the state until it closes.
fn create(state: &Rc<RefCell<AppState>>) -> Option<SettingsWindow> {
    let window = match SettingsWindow::new() {
        Ok(window) => window,
        Err(err) => {
            log::warn!("could not create the Settings window: {err}");
            return None;
        }
    };
    ui_bridge::set_app_name(&window);
    let choice = {
        let s = state.borrow();
        s.theme_override
            .unwrap_or_else(|| ui_bridge::theme_choice(&s.settings.theme))
    };
    theme::apply(&window, choice);
    wire(&window, state);
    state.borrow_mut().settings_window = Some(window.clone_strong());
    Some(window)
}

/// The window as `open` builds it, not placed or shown: the `--settings` snapshot renders it.
#[cfg(feature = "snapshot")]
pub(crate) fn create_for_snapshot(state: &Rc<RefCell<AppState>>) -> Option<SettingsWindow> {
    create(state)
}

/// Tells the app when the window gains or loses the OS focus, so the popup can tell a move to
/// Settings from a move away (WR-02 AC2).
fn watch_focus(window: &SettingsWindow, state: &Rc<RefCell<AppState>>) {
    use slint::winit_030::winit::event::WindowEvent as WinitEvent;
    use slint::winit_030::{EventResult, WinitWindowAccessor};

    let state = state.clone();
    window
        .window()
        .on_winit_window_event(move |_window, event| {
            if let WinitEvent::Focused(active) = event {
                focus_changed(&state, *active);
            }
            EventResult::Propagate
        });
}

/// Closes the window and drops it. The drop waits one turn, because this can run inside one of the
/// window's own callbacks.
pub(crate) fn close(state: &Rc<RefCell<AppState>>) {
    let window = state.borrow_mut().settings_window.take();
    state.borrow_mut().settings_active = false;
    let Some(window) = window else {
        return;
    };
    let _ = window.hide();
    slint::Timer::single_shot(Duration::ZERO, move || drop(window));
    settle_focus(state);
}

/// Whether the Settings window exists.
pub(crate) fn is_open(state: &Rc<RefCell<AppState>>) -> bool {
    state.borrow().settings_window.is_some()
}

/// The Settings window gained or lost the OS focus.
pub(crate) fn focus_changed(state: &Rc<RefCell<AppState>>, active: bool) {
    state.borrow_mut().settings_active = active;
    if !active {
        settle_focus(state);
    }
}

/// The popup gained the OS focus.
pub(crate) fn popup_activated(state: &Rc<RefCell<AppState>>) {
    state.borrow_mut().popup_active = true;
}

/// The popup lost the OS focus (WR-02 AC2). With no Settings window it hides at once. With one, the
/// focus may be on its way to it, so the decision waits a moment.
pub(crate) fn popup_deactivated(state: &Rc<RefCell<AppState>>) {
    state.borrow_mut().popup_active = false;
    if is_open(state) {
        settle_focus(state);
    } else {
        dismiss_popup(state);
    }
}

/// After either window lost the focus: if neither has it a moment later, the user went elsewhere
/// and the popup hides, unless it is pinned.
fn settle_focus(state: &Rc<RefCell<AppState>>) {
    let state = state.clone();
    slint::Timer::single_shot(FOCUS_GRACE, move || {
        let (popup_active, settings_active) = {
            let s = state.borrow();
            (s.popup_active, s.settings_active)
        };
        if !popup_active && !settings_active {
            dismiss_popup(&state);
        }
    });
}

/// Hides the open popup unless it is pinned.
fn dismiss_popup(state: &Rc<RefCell<AppState>>) {
    let popup = state.borrow().popup.upgrade();
    if let Some(popup) = popup
        && popup.window().is_visible()
        && !popup.get_pinned()
    {
        popup.invoke_dismiss();
    }
}

/// Repaints the window from the settings, the save status and the shortcut note.
pub(crate) fn refresh(state: &Rc<RefCell<AppState>>) {
    let window = state
        .borrow()
        .settings_window
        .as_ref()
        .map(SettingsWindow::clone_strong);
    let Some(window) = window else {
        return;
    };
    let (settings, has_bar, available, status, note) = {
        let s = state.borrow();
        (
            s.settings.clone(),
            s.has_bar,
            s.hotkeys.as_ref().is_none_or(|hotkeys| hotkeys.available()),
            s.save_status,
            s.shortcut_note,
        )
    };
    ui_bridge::set_settings(&window, &settings, has_bar, available);
    window.set_save_status(match status {
        SaveStatus::Idle => 0,
        SaveStatus::Saved => 1,
        SaveStatus::Failed => 2,
    });
    let note = if available {
        note
    } else {
        ShortcutNote::NoGlobalShortcuts
    };
    window.set_shortcut_note(note.code());
    window.set_meta_key(shortcut::META.into());
}

/// What every setting change does next (WR-06 AC5): repaint everything that shows it, refresh the
/// window and save 300 ms after the last change.
pub(crate) fn applied(state: &Rc<RefCell<AppState>>) {
    crate::update_popup(state);
    crate::update_bar(state);
    crate::update_tray_now(state);
    refresh(state);
    save_later(state);
}

/// Saves `settings.toml` 300 ms after the last change.
pub(crate) fn save_later(state: &Rc<RefCell<AppState>>) {
    let timer = state.borrow().save_timer.clone();
    let state = state.clone();
    timer.start(slint::TimerMode::SingleShot, SAVE_DELAY, move || {
        save_now(&state);
    });
}

/// Writes the settings now and tells the window how it went (WR-06 AC5): `Saved` for 2 s on
/// success, the failure note until the next success otherwise. Does nothing without a store.
pub(crate) fn save_now(state: &Rc<RefCell<AppState>>) {
    let (settings, store, note_timer) = {
        let s = state.borrow();
        (s.settings.clone(), s.store.clone(), s.note_timer.clone())
    };
    let Some(store) = store else {
        return;
    };
    match store.save(&settings) {
        Ok(()) => {
            set_status(state, SaveStatus::Saved);
            let state = state.clone();
            note_timer.start(slint::TimerMode::SingleShot, SAVED_NOTE, move || {
                set_status(&state, SaveStatus::Idle);
            });
        }
        Err(err) => {
            log::warn!("settings: could not save: {err}");
            note_timer.stop();
            set_status(state, SaveStatus::Failed);
        }
    }
}

fn set_status(state: &Rc<RefCell<AppState>>, status: SaveStatus) {
    state.borrow_mut().save_status = status;
    refresh(state);
}

/// Applies the theme setting to the popup and the Settings window.
fn apply_theme(state: &Rc<RefCell<AppState>>) {
    let (choice, popup, window) = {
        let mut s = state.borrow_mut();
        // A `--theme` flag only decides the first paint; a choice made here replaces it.
        s.theme_override = None;
        (
            ui_bridge::theme_choice(&s.settings.theme),
            s.popup.upgrade(),
            s.settings_window.as_ref().map(SettingsWindow::clone_strong),
        )
    };
    if let Some(popup) = popup {
        theme::apply(&popup, choice);
    }
    if let Some(window) = window {
        theme::apply(&window, choice);
    }
}

/// The new language reaches the tray, the bar's menu and everything that shows text.
fn apply_language(state: &Rc<RefCell<AppState>>) {
    let lang = state.borrow().settings.lang();
    let _ = slint::select_bundled_translation(if lang == nepdate::Lang::Ne {
        "ne"
    } else {
        "en"
    });
    let (tray, bar) = {
        let s = state.borrow();
        (s.tray.clone(), s.bar.clone())
    };
    if let Some(tray) = &tray {
        tray.set_language(lang);
    }
    if let (Some(tray), Some(bar)) = (&tray, &bar) {
        bar.set_menu_labels(&tray.menu_labels());
    }
}

/// Registers the shortcut of the settings with the OS and records how it went (WR-13 AC3).
pub(crate) fn apply_shortcut(state: &Rc<RefCell<AppState>>) {
    let (combo, hotkeys) = {
        let s = state.borrow();
        (s.settings.shortcut_combo(), s.hotkeys.clone())
    };
    let note = match hotkeys {
        None => ShortcutNote::Nothing,
        Some(hotkeys) if !hotkeys.available() => ShortcutNote::NoGlobalShortcuts,
        Some(hotkeys) => match hotkeys.set(combo) {
            Ok(()) => ShortcutNote::Nothing,
            Err(refused) => {
                log::warn!("global shortcut: {}", refused.0);
                ShortcutNote::InUse
            }
        },
    };
    state.borrow_mut().shortcut_note = note;
}

/// Makes `text` the shortcut, registers it and saves (WR-13 AC2).
fn set_shortcut(state: &Rc<RefCell<AppState>>, text: String) {
    state.borrow_mut().settings.shortcut = text;
    apply_shortcut(state);
    refresh(state);
    save_later(state);
}

fn line_of(number: i32) -> Line {
    if number == 0 { Line::Top } else { Line::Bottom }
}

fn number_of(line: Line) -> i32 {
    match line {
        Line::Top => 0,
        Line::Bottom => 1,
    }
}

/// Names the chip that takes focus once the lines are rebuilt: `index` -1 is the line's Add field
/// button.
fn focus_chip(state: &Rc<RefCell<AppState>>, line: i32, index: i32) {
    let window = state
        .borrow()
        .settings_window
        .as_ref()
        .map(SettingsWindow::clone_strong);
    if let Some(window) = window {
        window.set_focus_line(line);
        window.set_focus_index(index);
        window.set_focus_pending(true);
    }
}

/// After an edit of the lines: repaint, save, and focus the chip at `at`.
fn lines_edited(state: &Rc<RefCell<AppState>>, at: Option<At>) {
    applied(state);
    if let Some(at) = at {
        focus_chip(
            state,
            number_of(at.line),
            i32::try_from(at.index).unwrap_or(0),
        );
    }
}

/// Wires every control of the window to the app state (WR-06 AC5): each change is applied at once
/// and saved after 300 ms.
fn wire(window: &SettingsWindow, state: &Rc<RefCell<AppState>>) {
    wire_general(window, state);
    wire_zone(window, state);
    wire_bar(window, state);
    wire_lines(window, state);
    wire_shortcut(window, state);
    window.on_closed({
        let state = state.clone();
        move || close(&state)
    });
    window.window().on_close_requested({
        let state = state.clone();
        move || {
            close(&state);
            CloseRequestResponse::HideWindow
        }
    });
}

/// General and Clock.
fn wire_general(window: &SettingsWindow, state: &Rc<RefCell<AppState>>) {
    window.on_language_picked({
        let state = state.clone();
        move |index| {
            let language = if index == 1 { "ne" } else { "en" };
            language.clone_into(&mut state.borrow_mut().settings.language);
            apply_language(&state);
            applied(&state);
        }
    });
    window.on_theme_picked({
        let state = state.clone();
        move |index| {
            let theme = match index {
                1 => "light",
                2 => "dark",
                _ => "system",
            };
            theme.clone_into(&mut state.borrow_mut().settings.theme);
            apply_theme(&state);
            applied(&state);
        }
    });
    window.on_launch_changed({
        let state = state.clone();
        move |value| {
            state.borrow_mut().settings.launch_at_login = value;
            let login = state.borrow().login_item.clone();
            login(value);
            applied(&state);
        }
    });
    window.on_clock_changed({
        let state = state.clone();
        move |value| {
            state.borrow_mut().settings.clock_24h = value;
            applied(&state);
        }
    });
    window.on_seconds_changed({
        let state = state.clone();
        move |value| {
            state.borrow_mut().settings.show_seconds = value;
            // The tick is armed for the next minute; seconds need one every second.
            crate::rearm_timer(&state);
            applied(&state);
        }
    });
}

/// The extra clock: the drop-down, filtering, picking and None (WR-06 AC4).
fn wire_zone(window: &SettingsWindow, state: &Rc<RefCell<AppState>>) {
    window.on_zone_opened({
        let state = state.clone();
        move || show_zone_rows(&state, "")
    });
    window.on_zone_edited({
        let state = state.clone();
        move |text| show_zone_rows(&state, text.as_str())
    });
    window.on_zone_picked({
        let state = state.clone();
        move |zone| {
            state.borrow_mut().settings.extra_time_zone = zone.to_string();
            applied(&state);
        }
    });
}

/// Fills the drop-down for what is typed.
fn show_zone_rows(state: &Rc<RefCell<AppState>>, query: &str) {
    let window = state
        .borrow()
        .settings_window
        .as_ref()
        .map(SettingsWindow::clone_strong);
    let Some(window) = window else {
        return;
    };
    let (rows, highlight) = {
        let s = state.borrow();
        ui_bridge::zone_rows(&s.zones, query, &s.settings.extra_time_zone)
    };
    ui_bridge::set_zone_rows(&window, &rows, highlight);
}

/// Show bar, Lock position and Reset position.
fn wire_bar(window: &SettingsWindow, state: &Rc<RefCell<AppState>>) {
    window.on_bar_changed({
        let state = state.clone();
        move |value| crate::set_bar_shown(&state, value)
    });
    window.on_lock_changed({
        let state = state.clone();
        move |value| crate::set_bar_locked(&state, value)
    });
    window.on_reset_position({
        let state = state.clone();
        move || crate::reset_bar_position(&state)
    });
}

/// The line editor (WR-08 AC3): the chips, Add field and Reset to default.
fn wire_lines(window: &SettingsWindow, state: &Rc<RefCell<AppState>>) {
    window.on_chip_removed({
        let state = state.clone();
        move |line, index| {
            let (line, Ok(index)) = (line_of(line), usize::try_from(index)) else {
                return;
            };
            if !line_editor::remove(&mut state.borrow_mut().settings, line, index) {
                return;
            }
            // Focus the chip that took its place, or the line's Add field when none is left.
            let remaining = match line {
                Line::Top => state.borrow().settings.bar_top.len(),
                Line::Bottom => state.borrow().settings.bar_bottom.len(),
            };
            applied(&state);
            let next = remaining
                .checked_sub(1)
                .map_or(-1, |last| i32::try_from(index.min(last)).unwrap_or(0));
            focus_chip(&state, number_of(line), next);
        }
    });
    window.on_chip_moved({
        let state = state.clone();
        move |line, index, delta| {
            let (line, Ok(index)) = (line_of(line), usize::try_from(index)) else {
                return;
            };
            let at = line_editor::move_within(&mut state.borrow_mut().settings, line, index, delta);
            if at.is_some() {
                lines_edited(&state, at);
            }
        }
    });
    window.on_chip_moved_line({
        let state = state.clone();
        move |line, index, up| {
            let (line, Ok(index)) = (line_of(line), usize::try_from(index)) else {
                return;
            };
            let at = line_editor::move_across(&mut state.borrow_mut().settings, line, index, up);
            if at.is_some() {
                lines_edited(&state, at);
            }
        }
    });
    window.on_field_added({
        let state = state.clone();
        move |line, id| {
            let Some(field) = BarField::from_id(id.as_str()) else {
                return;
            };
            let at = line_editor::add(&mut state.borrow_mut().settings, line_of(line), field);
            if at.is_some() {
                lines_edited(&state, at);
            }
        }
    });
    window.on_lines_reset({
        let state = state.clone();
        move || {
            line_editor::reset(&mut state.borrow_mut().settings);
            applied(&state);
        }
    });
}

/// The shortcut field, Clear and Reset (WR-13 AC2).
fn wire_shortcut(window: &SettingsWindow, state: &Rc<RefCell<AppState>>) {
    window.on_shortcut_key({
        let state = state.clone();
        move |text, meta, ctrl, alt, shift| {
            let mods = Modifiers {
                meta,
                ctrl,
                alt,
                shift,
            };
            match shortcut::record(mods, ui_bridge::pressed(text.as_str())) {
                Recorded::Ignored => false,
                Recorded::Accepted(combo) => {
                    set_shortcut(&state, combo.to_string());
                    true
                }
                Recorded::Refused(refusal) => {
                    state.borrow_mut().shortcut_note = match refusal {
                        Refusal::CtrlAlt => ShortcutNote::CtrlAlt,
                        Refusal::TooFewModifiers => ShortcutNote::TooFewModifiers,
                        Refusal::KeyNotSupported => ShortcutNote::KeyNotSupported,
                    };
                    refresh(&state);
                    true
                }
            }
        }
    });
    window.on_shortcut_cleared({
        let state = state.clone();
        move || set_shortcut(&state, String::new())
    });
    window.on_shortcut_reset({
        let state = state.clone();
        move || set_shortcut(&state, Combo::DEFAULT.to_string())
    });
}
