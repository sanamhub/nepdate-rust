//! Interaction tests for the Settings window (WR-06, WR-08 AC3, WR-13) and for how it sits with the
//! popup (WR-02 AC2). The app gets fakes for everything the OS would do: the settings store, the
//! global shortcut, the tray, the bar and the login item. Nothing here touches the registry, a file
//! outside a temporary one, the clipboard or a global key.

use std::cell::{Cell, RefCell};
use std::rc::Rc;

use i_slint_backend_testing::{AccessibleRole, ElementHandle};
use nepdate::Lang;
use slint::platform::{Key, WindowEvent};
use slint::{ComponentHandle, Model};

use crate::app::settings::Settings;
use crate::app::shortcut::{Combo, META};
use crate::interaction_tests::{FakeBar, app, bar_anchor, date, settings_of};
use crate::platform::bar::{BarEvent, MenuItem};
use crate::platform::hotkey::{Hotkeys, Refused};
use crate::platform::settings_store::Store;
use crate::platform::tray::{Tray, TrayEvent};
use crate::{AppState, PopupWindow, SettingsWindow, settings_window};

// The fakes.

/// Records every write, and fails them on demand.
#[derive(Default)]
struct FakeStore {
    saved: RefCell<Vec<Settings>>,
    fail: Cell<bool>,
}

impl Store for FakeStore {
    fn save(&self, settings: &Settings) -> anyhow::Result<()> {
        if self.fail.get() {
            anyhow::bail!("the disk is full");
        }
        self.saved.borrow_mut().push(settings.clone());
        Ok(())
    }
}

/// Records every shortcut the app registers. `refuse` makes the OS say another app owns it, and
/// `unavailable` is a Wayland session.
#[derive(Default)]
struct FakeHotkeys {
    registered: RefCell<Vec<Option<Combo>>>,
    refuse: Cell<bool>,
    unavailable: Cell<bool>,
}

impl Hotkeys for FakeHotkeys {
    fn available(&self) -> bool {
        !self.unavailable.get()
    }

    fn set(&self, combo: Option<Combo>) -> Result<(), Refused> {
        self.registered.borrow_mut().push(combo);
        if self.refuse.get() && combo.is_some() {
            return Err(Refused(
                "RegisterHotKey failed: already registered".to_owned(),
            ));
        }
        Ok(())
    }
}

/// Records what the app tells the tray.
#[derive(Default)]
struct FakeTray {
    titles: RefCell<Vec<String>>,
    languages: RefCell<Vec<Lang>>,
    bar_shown: RefCell<Vec<bool>>,
}

impl Tray for FakeTray {
    fn set_title(&self, text: &str) {
        self.titles.borrow_mut().push(text.to_owned());
    }
    fn set_icon_day(&self, _day: &str) {}
    fn set_tooltip(&self, _tooltip: &str) {}
    fn set_dark_taskbar(&self, _dark: bool) {}
    fn set_language(&self, lang: Lang) {
        self.languages.borrow_mut().push(lang);
    }
    fn set_bar_supported(&self, _supported: bool) {}
    fn set_bar_shown(&self, shown: bool) {
        self.bar_shown.borrow_mut().push(shown);
    }
    fn menu_labels(&self) -> crate::platform::bar::MenuLabels {
        crate::platform::bar::MenuLabels::default()
    }
}

/// An app on 2083 Kartik 5 with every port faked and a bar.
struct Fixture {
    popup: PopupWindow,
    state: Rc<RefCell<AppState>>,
    slot: crate::PopupSlot,
    store: Rc<FakeStore>,
    hotkeys: Rc<FakeHotkeys>,
    tray: Rc<FakeTray>,
    bar: Rc<FakeBar>,
    logins: Rc<RefCell<Vec<bool>>>,
}

fn fixture() -> Fixture {
    let (popup, state, _) = app(date(2083, 7, 5));
    let store = Rc::new(FakeStore::default());
    let hotkeys = Rc::new(FakeHotkeys::default());
    let tray = Rc::new(FakeTray::default());
    let bar = Rc::new(FakeBar::default());
    let logins = Rc::new(RefCell::new(Vec::new()));
    {
        let mut s = state.borrow_mut();
        s.store = Some(store.clone());
        s.hotkeys = Some(hotkeys.clone());
        s.tray = Some(tray.clone());
        s.bar = Some(bar.clone());
        s.has_bar = true;
        s.zones = vec![
            "America/New_York".to_owned(),
            "Asia/Kathmandu".to_owned(),
            "Asia/Singapore".to_owned(),
            "Europe/Berlin".to_owned(),
        ];
        let logins = logins.clone();
        s.login_item = Rc::new(move |on| logins.borrow_mut().push(on));
    }
    let slot = Rc::new(RefCell::new(Some(popup.clone_strong())));
    Fixture {
        popup,
        state,
        slot,
        store,
        hotkeys,
        tray,
        bar,
        logins,
    }
}

/// A fixture with the Settings window open.
fn opened() -> (Fixture, SettingsWindow) {
    let f = fixture();
    settings_window::open(&f.state, settings_window::Request::default());
    let window = settings_of(&f.state);
    (f, window)
}

// The helpers.

fn elapse(millis: u64) {
    i_slint_backend_testing::mock_elapsed_time(std::time::Duration::from_millis(millis));
    slint::platform::update_timers_and_animations();
}

/// Lets a change that rebuilt the chips finish: the rows exist only once something walks the tree,
/// as a frame would, and the focus timer runs one tick after that.
fn settle(window: &SettingsWindow) {
    walk(window);
    elapse(10);
    walk(window);
}

fn press(window: &SettingsWindow, key: Key) {
    for event in [
        WindowEvent::KeyPressed { text: key.into() },
        WindowEvent::KeyReleased { text: key.into() },
    ] {
        window.window().dispatch_event(event);
    }
    slint::platform::update_timers_and_animations();
}

/// Presses `modifiers`, taps `key`, then lets the modifiers go.
fn press_with(window: &SettingsWindow, modifiers: &[Key], key: Key) {
    for modifier in modifiers {
        window.window().dispatch_event(WindowEvent::KeyPressed {
            text: (*modifier).into(),
        });
    }
    press(window, key);
    for modifier in modifiers {
        window.window().dispatch_event(WindowEvent::KeyReleased {
            text: (*modifier).into(),
        });
    }
}

/// Presses `modifiers`, types `letter`, then lets the modifiers go.
fn type_with(window: &SettingsWindow, modifiers: &[Key], letter: &str) {
    for modifier in modifiers {
        window.window().dispatch_event(WindowEvent::KeyPressed {
            text: (*modifier).into(),
        });
    }
    for event in [
        WindowEvent::KeyPressed {
            text: letter.into(),
        },
        WindowEvent::KeyReleased {
            text: letter.into(),
        },
    ] {
        window.window().dispatch_event(event);
    }
    for modifier in modifiers {
        window.window().dispatch_event(WindowEvent::KeyReleased {
            text: (*modifier).into(),
        });
    }
    slint::platform::update_timers_and_animations();
}

fn type_text(window: &SettingsWindow, text: &str) {
    for character in text.chars() {
        let text: slint::SharedString = character.to_string().into();
        for event in [
            WindowEvent::KeyPressed { text: text.clone() },
            WindowEvent::KeyReleased { text },
        ] {
            window.window().dispatch_event(event);
        }
    }
    slint::platform::update_timers_and_animations();
}

fn focused(window: &SettingsWindow) -> String {
    window.get_focused_control().to_string()
}

fn press_shift_tab(window: &SettingsWindow) {
    window.window().dispatch_event(WindowEvent::KeyPressed {
        text: Key::Shift.into(),
    });
    press(window, Key::Tab);
    window.window().dispatch_event(WindowEvent::KeyReleased {
        text: Key::Shift.into(),
    });
}

/// Tabs until `name` has focus, with a bound so a missing stop fails and does not hang.
fn tab_to(window: &SettingsWindow, name: &str) {
    for _ in 0..40 {
        press(window, Key::Tab);
        if focused(window) == name {
            return;
        }
    }
    panic!(
        "Tab never reached {name:?}; the last stop was {:?}",
        focused(window)
    );
}

fn elements(window: &SettingsWindow, label: &str) -> Vec<ElementHandle> {
    ElementHandle::find_by_accessible_label(window, label).collect()
}

fn exists(window: &SettingsWindow, label: &str) -> bool {
    !elements(window, label).is_empty()
}

fn element(window: &SettingsWindow, label: &str) -> ElementHandle {
    let found = elements(window, label);
    assert_eq!(
        found.len(),
        1,
        "expected one element labelled {label:?}, found {}",
        found.len()
    );
    found.into_iter().next().expect("one element")
}

fn click(window: &SettingsWindow, label: &str) {
    element(window, label).invoke_accessible_default_action();
    slint::platform::update_timers_and_animations();
}

/// Scrolls to the end, where the Bar and Shortcut sections and About are in view. The testing
/// backend leaves out what the scroll view clips, as a screen reader would not read it either.
fn scroll_to_end(window: &SettingsWindow) {
    window.set_scroll_y(-window.get_scroll_max());
    walk(window);
}

/// Lays the tree out, as a frame would: rows and clipped items exist only once something walks it.
fn walk(window: &SettingsWindow) -> usize {
    ElementHandle::find_by_accessible_label(window, "").count()
}

fn settings(f: &Fixture) -> Settings {
    f.state.borrow().settings.clone()
}

fn top(f: &Fixture) -> Vec<String> {
    settings(f).bar_top
}

fn bottom(f: &Fixture) -> Vec<String> {
    settings(f).bar_bottom
}

fn ids(items: &[&str]) -> Vec<String> {
    items.iter().map(|id| (*id).to_owned()).collect()
}

/// Waits out the 300 ms save debounce.
fn let_the_save_happen() {
    elapse(350);
}

/// Puts the source language back when a test that chose Nepali ends, whichever way it ends: the
/// translation is shared by every window of the thread.
struct English;

impl Drop for English {
    fn drop(&mut self) {
        let _ = slint::select_bundled_translation("en");
    }
}

// WR-06 AC1: its own window.

#[test]
fn wr06_ac1_settings_is_its_own_window_created_on_open_and_dropped_on_close() {
    let f = fixture();
    assert!(
        !settings_window::is_open(&f.state),
        "nothing exists until it is asked for"
    );

    settings_window::open(&f.state, settings_window::Request::default());
    let window = settings_of(&f.state);
    assert!(window.window().is_visible());
    assert_eq!(
        window.get_shown_title().as_str(),
        "MitiBar Settings",
        "the window's title"
    );
    assert!(
        f.popup.window().is_visible(),
        "it never sits on top of the popup: a window of its own"
    );

    // Escape closes it, and the window is dropped a turn later.
    let weak = window.as_weak();
    press(&window, Key::Escape);
    assert!(!settings_window::is_open(&f.state));
    drop(window);
    elapse(10);
    assert!(
        weak.upgrade().is_none(),
        "the window is gone, not hidden and kept"
    );

    // The close button of the frame does the same.
    settings_window::open(&f.state, settings_window::Request::default());
    let window = settings_of(&f.state);
    let weak = window.as_weak();
    window.window().dispatch_event(WindowEvent::CloseRequested);
    assert!(!settings_window::is_open(&f.state));
    drop(window);
    elapse(10);
    assert!(weak.upgrade().is_none());

    // And it comes back new.
    settings_window::open(&f.state, settings_window::Request::default());
    assert!(settings_window::is_open(&f.state));
}

#[test]
fn wr06_ac1_asking_again_brings_the_open_window_forward_instead_of_making_another() {
    let (f, window) = opened();
    window.set_zone_query("kept".into());
    settings_window::open(&f.state, settings_window::Request::default());
    assert_eq!(
        settings_of(&f.state).get_zone_query().as_str(),
        "kept",
        "the same window, not a new one"
    );
}

#[test]
fn wr06_ac1_the_settings_pill_ctrl_comma_the_tray_and_the_bar_menu_all_open_the_window() {
    let f = fixture();
    let close = |f: &Fixture| {
        settings_window::close(&f.state);
        elapse(10);
        assert!(!settings_window::is_open(&f.state));
    };

    // The pill.
    element_of_popup(&f.popup, "Settings").invoke_accessible_default_action();
    assert!(settings_window::is_open(&f.state), "the Settings pill");
    close(&f);

    // Ctrl+Comma.
    for event in [
        WindowEvent::KeyPressed {
            text: Key::Control.into(),
        },
        WindowEvent::KeyPressed { text: ",".into() },
        WindowEvent::KeyReleased { text: ",".into() },
        WindowEvent::KeyReleased {
            text: Key::Control.into(),
        },
    ] {
        f.popup.window().dispatch_event(event);
    }
    assert!(settings_window::is_open(&f.state), "Ctrl+Comma");
    close(&f);

    // The tray menu, with the popup closed: the popup does not open for it.
    f.popup.hide().expect("hide");
    crate::handle_tray_event(TrayEvent::Settings, &f.state, &f.slot);
    assert!(settings_window::is_open(&f.state), "the tray menu");
    assert!(
        !f.popup.window().is_visible(),
        "Settings does not open the popup"
    );
    close(&f);

    // The bar's menu: Settings, and Customize bar.
    for item in [MenuItem::Settings, MenuItem::Customize] {
        crate::handle_bar_event(
            BarEvent::Menu {
                item,
                anchor: bar_anchor(),
            },
            &f.state,
            &f.slot,
        );
        assert!(settings_window::is_open(&f.state), "the bar menu: {item:?}");
        close(&f);
    }
}

fn element_of_popup(popup: &PopupWindow, label: &str) -> ElementHandle {
    let found: Vec<ElementHandle> = ElementHandle::find_by_accessible_label(popup, label).collect();
    assert_eq!(found.len(), 1, "one element labelled {label:?}");
    found.into_iter().next().expect("one element")
}

// WR-02 AC2: focus moving to Settings is not a focus loss.

/// What Windows does when the Settings pill opens the window: the popup is told it lost the
/// focus, then the new window is told it has it.
fn focus_moves_to_settings(f: &Fixture) {
    f.popup.invoke_window_deactivated();
    settings_window::focus_changed(&f.state, true);
}

#[test]
fn wr02_ac2_focus_moving_to_the_settings_window_does_not_hide_the_popup() {
    let f = fixture();
    settings_window::popup_activated(&f.state);
    element_of_popup(&f.popup, "Settings").invoke_accessible_default_action();
    assert!(settings_window::is_open(&f.state));

    f.popup.invoke_window_deactivated();
    assert!(
        f.popup.window().is_visible(),
        "the loss is judged after the other window had its chance"
    );
    settings_window::focus_changed(&f.state, true);
    elapse(200);
    assert!(
        f.popup.window().is_visible(),
        "Settings has the focus, so the popup stays"
    );
}

#[test]
fn wr02_ac2_a_pinned_popup_stays_pinned_while_settings_opens_and_closes() {
    let f = fixture();
    settings_window::popup_activated(&f.state);
    f.popup.set_pinned(true);
    element_of_popup(&f.popup, "Settings").invoke_accessible_default_action();
    let window = settings_of(&f.state);
    focus_moves_to_settings(&f);
    elapse(200);
    assert!(f.popup.window().is_visible());
    assert!(f.popup.get_pinned(), "pinned while Settings has the focus");

    // Clicking another app: nothing has the focus, and the pin still holds.
    settings_window::focus_changed(&f.state, false);
    elapse(200);
    assert!(
        f.popup.window().is_visible(),
        "a pinned popup survives the focus leaving both"
    );
    assert!(f.popup.get_pinned());

    // Closing Settings with Escape hands the focus back to the popup.
    settings_window::focus_changed(&f.state, true);
    press(&window, Key::Escape);
    f.popup.invoke_window_activated();
    elapse(200);
    assert!(f.popup.window().is_visible());
    assert!(f.popup.get_pinned(), "still pinned after Settings closed");
}

#[test]
fn wr02_ac2_closing_settings_while_the_popup_takes_the_focus_back_keeps_the_popup() {
    let (f, window) = opened();
    focus_moves_to_settings(&f);
    press(&window, Key::Escape);
    // The OS tells the closing window, then the popup.
    settings_window::focus_changed(&f.state, false);
    f.popup.invoke_window_activated();
    elapse(200);
    assert!(f.popup.window().is_visible());
}

#[test]
fn wr02_ac2_the_focus_leaving_both_windows_hides_the_popup() {
    let (f, _window) = opened();
    focus_moves_to_settings(&f);
    elapse(200);
    assert!(f.popup.window().is_visible());

    // The user clicks another app: Settings loses the focus and nothing in MitiBar gains it.
    settings_window::focus_changed(&f.state, false);
    assert!(f.popup.window().is_visible(), "not at once");
    elapse(200);
    assert!(
        !f.popup.window().is_visible(),
        "an unpinned popup hides when the user goes elsewhere"
    );
}

#[test]
fn wr02_ac2_the_popup_losing_the_focus_to_another_app_with_settings_open_hides_it() {
    let (f, _window) = opened();
    // The focus goes from the popup to another app, not to Settings.
    f.popup.invoke_window_deactivated();
    elapse(200);
    assert!(!f.popup.window().is_visible());
}

#[test]
fn wr02_ac2_without_a_settings_window_a_focus_loss_hides_the_popup_at_once() {
    let f = fixture();
    f.popup.invoke_window_deactivated();
    assert!(!f.popup.window().is_visible());
}

// WR-06 AC5: every setting applies at once.

#[test]
fn wr06_ac5_the_language_applies_to_the_popup_the_bar_and_the_tray() {
    let (f, window) = opened();
    let _english = English;
    window.invoke_language_picked(1);
    assert_eq!(settings(&f).language, "ne");
    assert_eq!(
        f.tray.languages.borrow().last(),
        Some(&Lang::Ne),
        "the tray icon's digits"
    );
    assert!(
        !f.tray.titles.borrow().is_empty(),
        "the tray title was rewritten"
    );
    let last = f.bar.lines.borrow().last().cloned().expect("bar lines");
    assert!(
        last.bottom
            .chars()
            .any(|c| ('\u{0900}'..='\u{097f}').contains(&c)),
        "the bar speaks Nepali: {:?}",
        last.bottom
    );
    assert!(
        !f.bar.labels.borrow().is_empty(),
        "the bar's menu has new labels"
    );
    assert!(
        f.popup.get_bs().as_str().contains("कार्तिक"),
        "the popup header: {}",
        f.popup.get_bs()
    );
    assert_eq!(
        window.get_data().language_index,
        1,
        "the window follows too"
    );
    window.invoke_language_picked(0);
    assert_eq!(settings(&f).language, "en");
    assert_eq!(f.tray.languages.borrow().last(), Some(&Lang::En));
}

#[test]
fn wr06_ac5_the_theme_applies_to_the_popup_and_the_settings_window() {
    let (f, window) = opened();
    window.invoke_theme_picked(2);
    assert_eq!(settings(&f).theme, "dark");
    assert_eq!(
        f.popup.global::<crate::Theme>().get_choice(),
        crate::ThemeChoice::Dark
    );
    assert_eq!(
        window.global::<crate::Theme>().get_choice(),
        crate::ThemeChoice::Dark
    );
    window.invoke_theme_picked(1);
    assert_eq!(
        f.popup.global::<crate::Theme>().get_choice(),
        crate::ThemeChoice::Light
    );
    assert_eq!(
        window.global::<crate::Theme>().get_choice(),
        crate::ThemeChoice::Light
    );
    window.invoke_theme_picked(0);
    assert_eq!(
        window.global::<crate::Theme>().get_choice(),
        crate::ThemeChoice::System
    );
}

#[test]
fn wr06_ac5_the_24_hour_clock_applies_to_the_popup_and_the_bar() {
    let (f, window) = opened();
    let clock = |f: &Fixture| {
        f.popup
            .get_clocks()
            .row_data(0)
            .expect("the local clock")
            .time
            .to_string()
    };
    assert!(
        clock(&f).contains("AM") || clock(&f).contains("PM"),
        "{}",
        clock(&f)
    );
    window.invoke_clock_changed(true);
    assert!(
        !clock(&f).contains('M'),
        "24-hour in the popup: {}",
        clock(&f)
    );
    let last = f.bar.lines.borrow().last().cloned().expect("lines");
    assert!(
        !last.top.contains("PM") && !last.top.contains("AM"),
        "and on the bar: {}",
        last.top
    );
    assert!(window.get_data().clock_24h);
}

#[test]
fn wr06_ac5_seconds_apply_to_the_popup_and_the_bar_and_rearm_the_tick() {
    let (f, window) = opened();
    let generation = f.state.borrow().timer_generation;
    window.invoke_seconds_changed(true);
    let clock = f
        .popup
        .get_clocks()
        .row_data(0)
        .expect("clock")
        .time
        .to_string();
    assert_eq!(
        clock.matches(':').count(),
        2,
        "seconds in the popup: {clock}"
    );
    let last = f.bar.lines.borrow().last().cloned().expect("lines");
    assert!(
        last.top.matches(':').count() >= 2,
        "and on the bar: {}",
        last.top
    );
    assert_eq!(
        f.state.borrow().timer_generation,
        generation + 1,
        "the tick armed for the next minute is replaced by one for the next second"
    );
}

#[test]
fn wr06_ac5_the_extra_zone_applies_to_the_popup_and_the_bar() {
    let (f, window) = opened();
    // New York, because the extra clock is hidden when it has the offset of the local zone, and
    // no machine that runs the tests is on New York time in both seasons at once.
    window.invoke_zone_picked("America/New_York".into());
    assert_eq!(settings(&f).extra_time_zone, "America/New_York");
    let clocks = f.popup.get_clocks();
    let zones: Vec<String> = clocks
        .iter()
        .map(|line| line.zone_city.to_string())
        .collect();
    assert!(zones.iter().any(|zone| zone.contains("York")), "{zones:?}");
    let last = f.bar.lines.borrow().last().cloned().expect("lines");
    assert!(last.top.contains("York"), "{}", last.top);

    window.invoke_zone_picked("".into());
    assert_eq!(settings(&f).extra_time_zone, "");
    let last = f.bar.lines.borrow().last().cloned().expect("lines");
    assert!(!last.top.contains("York"), "no extra clock: {}", last.top);
}

#[test]
fn wr06_ac5_launch_at_login_reaches_the_login_item() {
    let (f, window) = opened();
    click(&window, "Launch at login");
    assert!(
        !settings(&f).launch_at_login,
        "the default is on, so the click turns it off"
    );
    click(&window, "Launch at login");
    assert!(settings(&f).launch_at_login);
    assert_eq!(*f.logins.borrow(), vec![false, true]);
}

#[test]
fn wr06_ac5_show_bar_and_lock_apply_to_the_bar_and_the_tray() {
    let (f, window) = opened();
    click(&window, "Show bar");
    assert!(!settings(&f).bar);
    assert_eq!(*f.bar.shown.borrow(), vec![false]);
    assert_eq!(
        *f.tray.bar_shown.borrow(),
        vec![false],
        "the tray's Show bar item unchecks"
    );
    click(&window, "Lock position");
    assert!(settings(&f).bar_locked);
    assert_eq!(*f.bar.locked.borrow(), vec![true]);
    assert!(window.get_data().bar_locked, "the switch follows");
}

#[test]
fn wr06_ac5_the_switches_say_what_they_are_and_whether_they_are_on() {
    let (_f, window) = opened();
    for label in [
        "Launch at login",
        "24-hour clock",
        "Show seconds",
        "Show bar",
        "Lock position",
    ] {
        let switch = element(&window, label);
        assert_eq!(
            switch.accessible_role(),
            Some(AccessibleRole::Switch),
            "{label}"
        );
        assert!(
            switch.accessible_checked().is_some(),
            "{label} says whether it is on"
        );
    }
    assert_eq!(
        element(&window, "Launch at login").accessible_checked(),
        Some(true)
    );
    assert_eq!(
        element(&window, "Show seconds").accessible_checked(),
        Some(false)
    );
    for label in ["English", "नेपाली", "System", "Light", "Dark"] {
        assert!(exists(&window, label), "{label}");
    }
    assert_eq!(element(&window, "English").accessible_checked(), Some(true));
}

#[test]
fn wr06_ac5_the_segments_pick_by_click_and_by_arrow_key() {
    let (f, window) = opened();
    click(&window, "नेपाली");
    assert_eq!(settings(&f).language, "ne");
    click(&window, "English");
    assert_eq!(settings(&f).language, "en");

    tab_to(&window, "theme");
    press(&window, Key::RightArrow);
    assert_eq!(settings(&f).theme, "light", "Right moves System to Light");
    press(&window, Key::RightArrow);
    assert_eq!(settings(&f).theme, "dark");
    press(&window, Key::RightArrow);
    assert_eq!(settings(&f).theme, "dark", "and stops at the end");
    press(&window, Key::Home);
    assert_eq!(settings(&f).theme, "system");
}

// WR-06 AC3, AC5: the save.

#[test]
fn wr06_ac5_changes_are_saved_300_ms_after_the_last_one_in_one_write() {
    let (f, window) = opened();
    window.invoke_clock_changed(true);
    elapse(200);
    window.invoke_seconds_changed(true);
    elapse(200);
    assert!(
        f.store.saved.borrow().is_empty(),
        "300 ms after the last change, not the first"
    );
    elapse(150);
    assert_eq!(
        f.store.saved.borrow().len(),
        1,
        "one write for both changes"
    );
    let saved = f.store.saved.borrow().last().cloned().expect("a write");
    assert!(saved.clock_24h && saved.show_seconds);
}

#[test]
fn wr06_ac5_saved_shows_after_the_write_and_clears_after_two_seconds() {
    let (f, window) = opened();
    assert_eq!(window.get_save_status(), 0);
    window.invoke_clock_changed(true);
    assert_eq!(
        window.get_save_status(),
        0,
        "nothing to say before the write"
    );
    let_the_save_happen();
    assert_eq!(f.store.saved.borrow().len(), 1);
    assert_eq!(window.get_save_status(), 1, "Saved");
    elapse(1900);
    assert_eq!(window.get_save_status(), 1, "for 2 s");
    elapse(200);
    assert_eq!(window.get_save_status(), 0);
}

#[test]
fn wr06_ac5_a_failed_write_says_so_until_the_next_good_one() {
    let (f, window) = opened();
    f.store.fail.set(true);
    window.invoke_clock_changed(true);
    let_the_save_happen();
    assert_eq!(window.get_save_status(), 2, "Couldn't save settings");
    assert!(f.store.saved.borrow().is_empty());
    elapse(10_000);
    assert_eq!(window.get_save_status(), 2, "the note stays");

    f.store.fail.set(false);
    window.invoke_clock_changed(false);
    let_the_save_happen();
    assert_eq!(
        window.get_save_status(),
        1,
        "a good write replaces it with Saved"
    );
    elapse(2100);
    assert_eq!(window.get_save_status(), 0);
}

#[test]
fn wr06_ac5_a_failure_after_a_good_write_cancels_the_pending_clear() {
    let (f, window) = opened();
    window.invoke_clock_changed(true);
    let_the_save_happen();
    assert_eq!(window.get_save_status(), 1);
    f.store.fail.set(true);
    window.invoke_clock_changed(false);
    let_the_save_happen();
    assert_eq!(window.get_save_status(), 2);
    elapse(3000);
    assert_eq!(
        window.get_save_status(),
        2,
        "the old 2 s clear does not remove the failure note"
    );
}

#[test]
fn wr06_ac5_the_notes_text_is_the_wording_of_the_requirement() {
    let (f, window) = opened();
    window.invoke_clock_changed(true);
    let_the_save_happen();
    assert!(exists_text(&window, "Saved"));
    f.store.fail.set(true);
    window.invoke_clock_changed(false);
    let_the_save_happen();
    assert!(exists_text(&window, "Couldn't save settings"));
}

/// Whether some element of the window shows `text` as its label or value. The notes are plain
/// text, so this finds them by their text.
fn exists_text(window: &SettingsWindow, text: &str) -> bool {
    ElementHandle::find_by_element_type_name(window, "Text")
        .any(|element| element.accessible_label().as_deref() == Some(text))
}

// WR-06 AC4: the extra clock field.

#[test]
fn wr06_ac4_the_zone_field_shows_the_chosen_zone() {
    let (_f, window) = opened();
    assert_eq!(window.get_zone_query().as_str(), "Asia/Kathmandu");
    assert!(
        !window.get_zone_open(),
        "the list is closed until the field is used"
    );
    assert!(
        !exists(&window, "Europe/Berlin"),
        "and its rows are not in the tree"
    );
}

#[test]
fn wr06_ac4_typing_filters_the_list_by_city_or_region() {
    let (_f, window) = opened();
    tab_to(&window, "zone");
    assert!(window.get_zone_open(), "focus opens the list");
    let rows = |window: &SettingsWindow| -> Vec<String> {
        window
            .get_zone_rows()
            .iter()
            .map(|row| row.to_string())
            .collect()
    };
    assert_eq!(
        rows(&window),
        [
            "",
            "America/New_York",
            "Asia/Kathmandu",
            "Asia/Singapore",
            "Europe/Berlin"
        ],
        "None first, then every zone"
    );
    assert_eq!(
        window.get_zone_highlight(),
        2,
        "the chosen zone is highlighted"
    );

    type_text(&window, "sing");
    assert_eq!(rows(&window), ["Asia/Singapore"], "by city");
    for _ in 0.."sing".len() {
        press(&window, Key::Backspace);
    }
    type_text(&window, "asia");
    assert_eq!(
        rows(&window),
        ["Asia/Kathmandu", "Asia/Singapore"],
        "by region"
    );
    for _ in 0.."asia".len() {
        press(&window, Key::Backspace);
    }
    type_text(&window, "new york");
    assert_eq!(
        rows(&window),
        ["America/New_York"],
        "an underscore reads as a space"
    );
}

#[test]
fn wr06_ac4_up_down_and_enter_pick_a_zone() {
    let (f, window) = opened();
    tab_to(&window, "zone");
    // The highlight starts on Asia/Kathmandu; Down moves to Asia/Singapore.
    press(&window, Key::DownArrow);
    assert_eq!(window.get_zone_highlight(), 3);
    press(&window, Key::DownArrow);
    assert_eq!(window.get_zone_highlight(), 4);
    press(&window, Key::DownArrow);
    assert_eq!(window.get_zone_highlight(), 4, "it stops at the last row");
    press(&window, Key::UpArrow);
    assert_eq!(window.get_zone_highlight(), 3);
    press(&window, Key::Return);
    assert_eq!(settings(&f).extra_time_zone, "Asia/Singapore");
    assert!(!window.get_zone_open(), "Enter closes the list");
    assert_eq!(
        window.get_zone_query().as_str(),
        "Asia/Singapore",
        "and the field shows the pick"
    );
    assert_eq!(focused(&window), "zone", "focus stays on the field");
}

#[test]
fn wr06_ac4_a_typed_filter_is_picked_with_enter() {
    let (f, window) = opened();
    tab_to(&window, "zone");
    type_text(&window, "berl");
    press(&window, Key::Return);
    assert_eq!(settings(&f).extra_time_zone, "Europe/Berlin");
}

#[test]
fn wr06_ac4_none_clears_the_extra_clock() {
    let (f, window) = opened();
    tab_to(&window, "zone");
    press(&window, Key::UpArrow);
    press(&window, Key::UpArrow);
    assert_eq!(window.get_zone_highlight(), 0, "None is the first row");
    press(&window, Key::Return);
    assert_eq!(settings(&f).extra_time_zone, "");
    assert_eq!(window.get_zone_query().as_str(), "None");
    let last = f.bar.lines.borrow().last().cloned().expect("lines");
    assert!(
        !last.top.contains("Kathmandu"),
        "the clock is gone: {}",
        last.top
    );
}

#[test]
fn wr06_ac4_a_click_on_a_row_picks_it_and_a_row_has_an_accessible_name() {
    let (f, window) = opened();
    tab_to(&window, "zone");
    click(&window, "Europe/Berlin");
    assert_eq!(settings(&f).extra_time_zone, "Europe/Berlin");
    tab_to(&window, "zone");
    click(&window, "None");
    assert_eq!(settings(&f).extra_time_zone, "");
}

#[test]
fn wr06_ac4_escape_closes_the_list_without_changing_the_zone_and_a_second_escape_closes_the_window()
{
    let (f, window) = opened();
    tab_to(&window, "zone");
    press(&window, Key::DownArrow);
    press(&window, Key::Escape);
    assert!(!window.get_zone_open(), "the list closes");
    assert_eq!(
        settings(&f).extra_time_zone,
        "Asia/Kathmandu",
        "the zone is unchanged"
    );
    assert_eq!(
        window.get_zone_query().as_str(),
        "Asia/Kathmandu",
        "and the field shows it again"
    );
    assert!(
        settings_window::is_open(&f.state),
        "the first Escape did not close the window"
    );
    press(&window, Key::Escape);
    assert!(!settings_window::is_open(&f.state), "the second one does");
}

// WR-08 AC3: the line editor.

/// The labels of the chips of one line, in order, read from the state.
fn chips(f: &Fixture) -> (Vec<String>, Vec<String>) {
    (top(f), bottom(f))
}

#[test]
fn wr08_ac3_each_line_shows_its_fields_as_chips() {
    let (_f, window) = opened();
    tab_to(&window, "chip bottom 2");
    for label in [
        "Weekday",
        "Time",
        "Extra clock time",
        "BS date",
        "AD date, short",
    ] {
        let chip = element(&window, label);
        assert_eq!(
            chip.accessible_role(),
            Some(AccessibleRole::ListItem),
            "{label}"
        );
    }
    for label in ["Remove Weekday", "Remove Time", "Remove BS date"] {
        assert!(exists(&window, label), "{label}");
    }
    assert!(
        !exists(&window, "Tithi"),
        "a field that is on no line is not a chip"
    );
}

#[test]
fn wr08_ac3_add_field_lists_the_fields_on_neither_line_and_adds_one_to_that_line() {
    let (f, window) = opened();
    scroll_to_end(&window);
    click(&window, "Add field to the top line");
    scroll_to_end(&window);
    for label in [
        "Weekday, long",
        "BS date, short",
        "AD date",
        "Tithi",
        "Holiday",
    ] {
        assert!(exists(&window, label), "{label} is offered");
    }
    assert!(
        !elements(&window, "Time")
            .iter()
            .any(|e| e.accessible_role() == Some(AccessibleRole::Button)),
        "a field in use is not offered"
    );
    click(&window, "Tithi");
    assert_eq!(
        top(&f),
        ids(&["weekday", "time", "zone_time", "tithi"]),
        "at the end of the top line"
    );

    // Once it is on a line, it is no longer offered, on either line.
    scroll_to_end(&window);
    click(&window, "Add field to the bottom line");
    scroll_to_end(&window);
    assert!(
        !elements(&window, "Tithi")
            .iter()
            .any(|e| e.accessible_role() == Some(AccessibleRole::Button)),
        "a field appears at most once"
    );
    click(&window, "Holiday");
    assert_eq!(bottom(&f), ids(&["bs_date", "ad_short", "holiday"]));
    let last = f.bar.lines.borrow().last().cloned().expect("lines");
    assert!(!last.top.is_empty(), "the bar updated on each change");
}

#[test]
fn wr08_ac3_every_field_can_be_added_until_none_is_left() {
    let (f, window) = opened();
    for name in [
        "Weekday, long",
        "BS date, short",
        "AD date",
        "Tithi",
        "Holiday",
    ] {
        tab_to(&window, "add top");
        click(&window, "Add field to the top line");
        click(&window, name);
    }
    // The button is disabled now, so it is no longer a stop; its chips are, and bring it in view.
    tab_to(&window, "chip top 1");
    assert_eq!(top(&f).len() + bottom(&f).len(), 10);
    assert!(
        !element(&window, "Add field to the top line")
            .accessible_enabled()
            .unwrap_or(true),
        "with every field in use there is nothing to add"
    );
}

#[test]
fn wr08_ac3_a_chip_is_removed_with_its_button_and_focus_moves_to_its_neighbour() {
    let (f, window) = opened();
    // The extra clock shows only where its offset differs from the local one, so count the parts
    // of the line rather than look for a time: the machine decides whether there are two or three.
    let parts = |f: &Fixture| {
        f.bar
            .lines
            .borrow()
            .last()
            .map_or(0, |lines| lines.top.split(" · ").count())
    };
    settings_window::refresh(&f.state);
    crate::update_bar(&f.state);
    let before = parts(&f);
    click(&window, "Remove Time");
    assert_eq!(top(&f), ids(&["weekday", "zone_time"]));
    assert!(!exists(&window, "Time"));
    settle(&window);
    assert_eq!(
        focused(&window),
        "chip top 2",
        "the chip that took its place"
    );
    assert!(
        parts(&f) < before,
        "the bar lost the time: {before} parts before"
    );

    // Delete does the same from the keyboard.
    press(&window, Key::Delete);
    settle(&window);
    assert_eq!(top(&f), ids(&["weekday"]));
    assert_eq!(focused(&window), "chip top 1");
}

#[test]
fn wr08_ac3_the_last_chip_cannot_be_removed() {
    let (f, window) = opened();
    {
        let mut s = f.state.borrow_mut();
        s.settings.bar_top = ids(&["time"]);
        s.settings.bar_bottom = Vec::new();
    }
    settings_window::refresh(&f.state);
    let remove = element(&window, "Remove Time");
    assert_eq!(
        remove.accessible_enabled(),
        Some(false),
        "its remove button is disabled"
    );
    remove.invoke_accessible_default_action();
    assert_eq!(top(&f), ids(&["time"]), "and the click does nothing");
    tab_to(&window, "chip top 1");
    press(&window, Key::Delete);
    assert_eq!(top(&f), ids(&["time"]), "Delete does nothing either");

    // Two chips: either can go.
    window.invoke_field_added(1, "tithi".into());
    walk(&window);
    assert_eq!(
        element(&window, "Remove Time").accessible_enabled(),
        Some(true)
    );
}

#[test]
fn wr08_ac3_a_focused_chip_moves_left_and_right_with_its_buttons_and_with_alt_arrows() {
    let (f, window) = opened();
    tab_to(&window, "chip top 1");
    // The arrow buttons show on the focused chip.
    assert_eq!(
        element(&window, "Move Weekday left").accessible_enabled(),
        Some(false),
        "at the left end"
    );
    click(&window, "Move Weekday right");
    settle(&window);
    assert_eq!(top(&f), ids(&["time", "weekday", "zone_time"]));
    assert_eq!(focused(&window), "chip top 2", "the focus follows the chip");

    press_with(&window, &[Key::Alt], Key::RightArrow);
    settle(&window);
    assert_eq!(top(&f), ids(&["time", "zone_time", "weekday"]));
    assert_eq!(focused(&window), "chip top 3");
    press_with(&window, &[Key::Alt], Key::RightArrow);
    settle(&window);
    assert_eq!(
        top(&f),
        ids(&["time", "zone_time", "weekday"]),
        "it stops at the right end"
    );

    press_with(&window, &[Key::Alt], Key::LeftArrow);
    settle(&window);
    assert_eq!(top(&f), ids(&["time", "weekday", "zone_time"]));
    assert_eq!(focused(&window), "chip top 2");
    let last = f.bar.lines.borrow().last().cloned().expect("lines");
    assert!(
        last.top.starts_with(|c: char| c.is_ascii_digit()),
        "the bar follows: {}",
        last.top
    );
}

#[test]
fn wr08_ac3_a_refresh_does_not_take_the_focus_from_a_chip() {
    // The window refreshes after every change and when `Saved` appears and goes. Replacing the
    // chips' models then rebuilt every chip and dropped the focus: the next Alt+Arrow did nothing,
    // and Escape no longer closed the window.
    let (f, window) = opened();
    tab_to(&window, "chip top 2");
    for _ in 0..3 {
        settings_window::refresh(&f.state);
        settle(&window);
    }
    assert_eq!(focused(&window), "chip top 2");
    press_with(&window, &[Key::Alt], Key::RightArrow);
    settle(&window);
    assert_eq!(
        top(&f),
        ids(&["weekday", "zone_time", "time"]),
        "the key reached the chip"
    );

    // A save and its note come and go while the chip has the focus.
    let_the_save_happen();
    elapse(2100);
    walk(&window);
    assert_eq!(window.get_save_status(), 0);
    press_with(&window, &[Key::Alt], Key::LeftArrow);
    settle(&window);
    assert_eq!(
        top(&f),
        ids(&["weekday", "time", "zone_time"]),
        "and still does after the note"
    );
    press(&window, Key::Escape);
    assert!(
        !settings_window::is_open(&f.state),
        "Escape still reaches the window"
    );
}

#[test]
fn wr08_ac3_alt_up_and_alt_down_move_a_chip_to_the_other_line() {
    let (f, window) = opened();
    tab_to(&window, "chip top 3");
    press_with(&window, &[Key::Alt], Key::DownArrow);
    settle(&window);
    assert_eq!(
        chips(&f),
        (
            ids(&["weekday", "time"]),
            ids(&["bs_date", "ad_short", "zone_time"])
        )
    );
    assert_eq!(
        focused(&window),
        "chip bottom 3",
        "the focus follows the chip"
    );

    press_with(&window, &[Key::Alt], Key::DownArrow);
    settle(&window);
    assert_eq!(bottom(&f).len(), 3, "nothing is below the bottom line");

    press_with(&window, &[Key::Alt], Key::UpArrow);
    settle(&window);
    assert_eq!(top(&f), ids(&["weekday", "time", "zone_time"]));
    assert_eq!(focused(&window), "chip top 3");
    press_with(&window, &[Key::Alt], Key::UpArrow);
    settle(&window);
    assert_eq!(top(&f).len(), 3, "nothing is above the top line");
}

#[test]
fn wr08_ac3_a_line_can_be_left_empty_and_the_bar_shows_the_other_alone() {
    let (f, window) = opened();
    for _ in 0..3 {
        tab_to(&window, "chip top 1");
        press_with(&window, &[Key::Alt], Key::DownArrow);
        settle(&window);
    }
    assert_eq!(top(&f), Vec::<String>::new());
    assert_eq!(bottom(&f).len(), 5);
    let last = f.bar.lines.borrow().last().cloned().expect("lines");
    assert!(
        last.top.is_empty(),
        "an empty top line leaves the bottom as the only line"
    );
    assert!(!last.bottom.is_empty());
}

#[test]
fn wr08_ac3_reset_to_default_restores_the_two_lines() {
    let (f, window) = opened();
    click(&window, "Remove Time");
    window.invoke_field_added(0, "tithi".into());
    assert_ne!(top(&f), Settings::default().bar_top);
    scroll_to_end(&window);
    click(&window, "Reset to default");
    assert_eq!(top(&f), Settings::default().bar_top);
    assert_eq!(bottom(&f), Settings::default().bar_bottom);
}

#[test]
fn wr08_ac3_every_line_edit_updates_the_bar_and_is_saved() {
    let (f, window) = opened();
    let before = f.bar.lines.borrow().len();
    click(&window, "Remove Weekday");
    assert!(f.bar.lines.borrow().len() > before, "the bar was told");
    let_the_save_happen();
    let saved = f.store.saved.borrow().last().cloned().expect("a write");
    assert_eq!(saved.bar_top, ids(&["time", "zone_time"]));
}

#[test]
fn wr08_ac3_reset_position_puts_the_bar_back() {
    let (f, window) = opened();
    scroll_to_end(&window);
    click(&window, "Reset position");
    assert_eq!(
        f.bar.positions.borrow().last(),
        Some(&crate::app::bar_place::Saved::default())
    );
}

// WR-13: the shortcut.

#[test]
fn wr13_ac2_the_field_records_the_next_combination_pressed_while_it_has_focus() {
    let (f, window) = opened();
    tab_to(&window, "shortcut");
    type_with(&window, &[Key::Meta, Key::Shift], "k");
    let expected = format!("{META}+Shift+K");
    assert_eq!(settings(&f).shortcut, expected, "recorded");
    assert_eq!(window.get_data().shortcut.as_str(), expected, "and shown");
    assert_eq!(
        f.hotkeys.registered.borrow().last().copied().flatten(),
        Combo::parse(&expected),
        "registered at once"
    );
    assert_eq!(window.get_shortcut_note(), 0);
    let_the_save_happen();
    assert_eq!(
        f.store.saved.borrow().last().expect("a write").shortcut,
        expected
    );

    type_with(&window, &[Key::Control, Key::Shift], "7");
    assert_eq!(
        settings(&f).shortcut,
        "Ctrl+Shift+7",
        "the next press replaces it"
    );
}

#[test]
fn wr13_ac2_a_combination_needs_the_win_key_or_two_other_modifiers() {
    let (f, window) = opened();
    tab_to(&window, "shortcut");
    let before = settings(&f).shortcut;
    for modifiers in [
        &[Key::Control][..],
        &[Key::Alt][..],
        &[Key::Shift][..],
        &[][..],
    ] {
        type_with(&window, modifiers, "k");
        assert_eq!(
            settings(&f).shortcut,
            before,
            "{modifiers:?} alone is refused"
        );
        assert_eq!(window.get_shortcut_note(), 2, "with the note that says why");
    }
    type_with(&window, &[Key::Control, Key::Shift], "k");
    assert_eq!(settings(&f).shortcut, "Ctrl+Shift+K");
    assert_eq!(
        window.get_shortcut_note(),
        0,
        "a good combination clears the note"
    );
    type_with(&window, &[Key::Alt, Key::Shift], "k");
    assert_eq!(settings(&f).shortcut, "Alt+Shift+K");
}

#[test]
fn wr13_ac2_a_key_that_types_a_symbol_is_answered_and_not_recorded() {
    let (f, window) = opened();
    tab_to(&window, "shortcut");
    let before = settings(&f).shortcut;
    // Shift+7 on a US keyboard: the key event carries `&`, not the 7 key.
    type_with(&window, &[Key::Control, Key::Shift], "&");
    assert_eq!(settings(&f).shortcut, before);
    assert_eq!(
        window.get_shortcut_note(),
        5,
        "That key can't be recorded; use a letter or F1 to F12"
    );
    assert_eq!(focused(&window), "shortcut", "the field keeps waiting");
    type_with(&window, &[Key::Control, Key::Shift], "k");
    assert_eq!(settings(&f).shortcut, "Ctrl+Shift+K");
    assert_eq!(window.get_shortcut_note(), 0, "and the note goes");
}

#[test]
fn wr13_ac2_ctrl_alt_is_refused_with_the_altgr_note() {
    let (f, window) = opened();
    tab_to(&window, "shortcut");
    let before = settings(&f).shortcut;
    type_with(&window, &[Key::Control, Key::Alt], "k");
    assert_eq!(settings(&f).shortcut, before);
    assert_eq!(
        window.get_shortcut_note(),
        1,
        "Ctrl+Alt is AltGr on many keyboards"
    );
    type_with(&window, &[Key::Control, Key::Alt, Key::Shift], "k");
    assert_eq!(settings(&f).shortcut, before, "even with a third modifier");
    type_with(&window, &[Key::Meta, Key::Control, Key::Alt], "k");
    assert_eq!(settings(&f).shortcut, before, "or the Win key");
    assert_eq!(
        f.hotkeys.registered.borrow().len(),
        0,
        "a refused combination never reaches the OS"
    );
}

#[test]
fn wr13_ac2_tab_escape_and_a_lone_modifier_are_not_recorded() {
    let (f, window) = opened();
    tab_to(&window, "shortcut");
    let before = settings(&f).shortcut;
    for modifier in [Key::Shift, Key::Control, Key::Alt, Key::Meta] {
        press(&window, modifier);
    }
    assert_eq!(
        settings(&f).shortcut,
        before,
        "a modifier by itself waits for the key"
    );
    assert_eq!(window.get_shortcut_note(), 0);
    press(&window, Key::Tab);
    assert_eq!(focused(&window), "shortcut clear", "Tab still moves on");
    press_shift_tab(&window);
    assert_eq!(focused(&window), "shortcut", "and Shift+Tab back");
    press(&window, Key::Escape);
    assert!(
        !settings_window::is_open(&f.state),
        "Escape still closes the window"
    );
}

#[test]
fn wr13_ac2_clear_removes_the_shortcut_and_reset_restores_the_default() {
    let (f, window) = opened();
    scroll_to_end(&window);
    click(&window, "Clear shortcut");
    assert_eq!(settings(&f).shortcut, "");
    assert_eq!(
        f.hotkeys.registered.borrow().last().copied(),
        Some(None),
        "unregistered"
    );
    assert_eq!(window.get_data().shortcut.as_str(), "");
    let_the_save_happen();
    assert_eq!(f.store.saved.borrow().last().expect("a write").shortcut, "");

    tab_to(&window, "shortcut");
    type_with(&window, &[Key::Control, Key::Shift], "k");
    scroll_to_end(&window);
    click(&window, "Reset shortcut");
    assert_eq!(settings(&f).shortcut, Settings::default().shortcut);
    assert_eq!(
        f.hotkeys.registered.borrow().last().copied().flatten(),
        Some(Combo::DEFAULT)
    );
}

#[test]
fn wr13_ac3_a_combination_another_app_owns_says_so_and_the_rest_keeps_working() {
    let (f, window) = opened();
    f.hotkeys.refuse.set(true);
    tab_to(&window, "shortcut");
    type_with(&window, &[Key::Control, Key::Shift], "k");
    assert_eq!(
        window.get_shortcut_note(),
        3,
        "This shortcut is used by another app"
    );
    assert_eq!(
        settings(&f).shortcut,
        "Ctrl+Shift+K",
        "what was typed stays, so the field shows it"
    );
    // The rest of the app is untouched.
    window.invoke_clock_changed(true);
    assert!(settings(&f).clock_24h);
    assert!(f.popup.window().is_visible());

    // A good one afterwards clears the note.
    f.hotkeys.refuse.set(false);
    type_with(&window, &[Key::Alt, Key::Shift], "k");
    assert_eq!(window.get_shortcut_note(), 0);
}

#[test]
fn wr13_ac3_a_refusal_at_start_is_shown_when_settings_opens() {
    let f = fixture();
    f.hotkeys.refuse.set(true);
    settings_window::apply_shortcut(&f.state);
    settings_window::open(&f.state, settings_window::Request::default());
    assert_eq!(settings_of(&f.state).get_shortcut_note(), 3);
}

#[test]
fn wr13_ac4_on_wayland_the_field_is_disabled_with_its_note() {
    let f = fixture();
    f.hotkeys.unavailable.set(true);
    settings_window::apply_shortcut(&f.state);
    settings_window::open(&f.state, settings_window::Request::default());
    let window = settings_of(&f.state);
    scroll_to_end(&window);
    assert_eq!(
        window.get_shortcut_note(),
        4,
        "Global shortcuts aren't available on Wayland"
    );
    assert!(!window.get_data().shortcut_available);
    assert_eq!(
        element(&window, "Shortcut").accessible_enabled(),
        Some(false)
    );
    assert_eq!(
        element(&window, "Clear shortcut").accessible_enabled(),
        Some(false)
    );
    assert_eq!(
        element(&window, "Reset shortcut").accessible_enabled(),
        Some(false)
    );
    // Nothing was registered, and the field is not a Tab stop.
    assert!(f.hotkeys.registered.borrow().is_empty());
    let mut stops = Vec::new();
    for _ in 0..30 {
        press(&window, Key::Tab);
        stops.push(focused(&window));
    }
    assert!(
        !stops.iter().any(|stop| stop.starts_with("shortcut")),
        "{stops:?}"
    );
}

#[test]
fn wr13_ac1_the_shortcut_toggles_the_popup_by_the_bar_while_the_bar_is_shown() {
    let f = fixture();
    crate::install_shortcut_request(&f.slot, &f.state);
    f.popup.hide().expect("hide");
    crate::run_shortcut_request();
    assert!(f.popup.window().is_visible(), "the shortcut opens it");
    crate::run_shortcut_request();
    assert!(!f.popup.window().is_visible(), "and closes it");

    // With the bar hidden it opens at the tray corner; the fake bar would still have an anchor,
    // so the setting decides.
    f.state.borrow_mut().settings.bar = false;
    crate::run_shortcut_request();
    assert!(f.popup.window().is_visible());
}

// WR-06 AC6: the sections, and About.

#[test]
fn wr06_ac6_the_bar_section_is_left_out_where_there_is_no_bar() {
    let f = fixture();
    f.state.borrow_mut().has_bar = false;
    settings_window::open(&f.state, settings_window::Request::default());
    let window = settings_of(&f.state);
    for label in [
        "Show bar",
        "Lock position",
        "Reset position",
        "Add field to the top line",
    ] {
        assert!(!exists(&window, label), "{label}");
    }
    for label in ["Language", "Launch at login", "24-hour clock", "Shortcut"] {
        assert!(exists(&window, label), "{label}");
    }
}

#[test]
fn wr06_ac6_about_has_the_version_the_license_and_the_notices_as_plain_text() {
    let (_f, window) = opened();
    scroll_to_end(&window);
    assert!(exists_text_containing(
        &window,
        &format!("MitiBar {} · MIT license", env!("CARGO_PKG_VERSION"))
    ));
    for notice in ["Slint", "Noto Sans", "NepDate"] {
        assert!(exists_text_containing(&window, notice), "{notice}");
    }
}

fn exists_text_containing(window: &SettingsWindow, text: &str) -> bool {
    ElementHandle::find_by_element_type_name(window, "Text").any(|element| {
        element
            .accessible_label()
            .is_some_and(|label| label.contains(text))
    })
}

#[test]
fn wr15_ac1_the_window_shows_no_third_party_logo_or_badge() {
    let markup = include_str!("../ui/settings.slint");
    assert!(
        !markup.contains("AboutSlint"),
        "ADR-0020: no Slint widget in the window"
    );
    for file in [
        include_str!("../ui/popup.slint"),
        include_str!("../ui/app.slint"),
        include_str!("../ui/converter.slint"),
        include_str!("../ui/day-detail.slint"),
    ] {
        assert!(!file.contains("AboutSlint"));
    }
}

// WR-10 AC2, W7 and W8-3 AC5: the Tab order, asserted at every stop.

/// The Tab order of the whole window, top to bottom.
fn full_order() -> Vec<&'static str> {
    vec![
        "language",
        "theme",
        "launch",
        "24-hour",
        "seconds",
        "zone",
        "bar",
        "lock",
        "add top",
        "chip top 1",
        "chip top 2",
        "chip top 3",
        "add bottom",
        "chip bottom 1",
        "chip bottom 2",
        "reset lines",
        "reset position",
        "shortcut",
        "shortcut clear",
        "shortcut reset",
    ]
}

#[test]
fn wr10_ac2_the_settings_window_tab_order_is_asserted_at_every_stop() {
    let (_f, window) = opened();
    // Tab walks the order and wraps to the first stop; Shift+Tab walks it back from there.
    let order = full_order();
    for expected in order.iter().copied().chain(["language"]) {
        press(&window, Key::Tab);
        assert_eq!(focused(&window), expected, "Tab order");
    }
    for expected in order.iter().rev() {
        press_shift_tab(&window);
        assert_eq!(focused(&window), *expected, "Shift+Tab order");
    }
}

#[test]
fn wr10_ac2_the_settings_window_has_no_bar_stops_without_a_bar() {
    let f = fixture();
    f.state.borrow_mut().has_bar = false;
    settings_window::open(&f.state, settings_window::Request::default());
    let window = settings_of(&f.state);
    let order = [
        "language",
        "theme",
        "launch",
        "24-hour",
        "seconds",
        "zone",
        "shortcut",
        "shortcut clear",
        "shortcut reset",
    ];
    for expected in order.into_iter().chain(["language"]) {
        press(&window, Key::Tab);
        assert_eq!(focused(&window), expected, "Tab order without a bar");
    }
}

#[test]
fn wr10_ac2_an_open_add_field_list_adds_its_items_to_the_order() {
    let (f, window) = opened();
    tab_to(&window, "add top");
    press(&window, Key::Return);
    assert_eq!(
        focused(&window),
        "add top",
        "Enter opens the list and keeps the focus"
    );
    press(&window, Key::Tab);
    // The five unused fields come next, in the order of the requirement's table.
    for expected in [
        "add item 1",
        "add item 2",
        "add item 3",
        "add item 4",
        "add item 5",
    ] {
        assert_eq!(focused(&window), expected);
        press(&window, Key::Tab);
    }
    assert_eq!(
        focused(&window),
        "chip top 1",
        "then the list ends and the line's chips begin"
    );
    tab_to(&window, "add top");
    press(&window, Key::Escape);
    assert!(
        settings_window::is_open(&f.state),
        "Escape closes the list first"
    );
    assert!(!exists(&window, "Holiday"), "the list is gone");
    press(&window, Key::Escape);
    assert!(
        !settings_window::is_open(&f.state),
        "and the next Escape closes the window"
    );
}

#[test]
fn wr10_ac2_every_control_has_a_name_and_the_right_role() {
    let (_f, window) = opened();
    // Each control is reached by Tab, which scrolls it into view, so the check also covers the
    // ones below the fold.
    for (stop, label, role) in [
        ("language", "Language", AccessibleRole::TabList),
        ("theme", "Theme", AccessibleRole::TabList),
        ("theme", "English", AccessibleRole::Tab),
        ("launch", "Launch at login", AccessibleRole::Switch),
        ("zone", "Extra clock", AccessibleRole::TextInput),
        ("chip top 1", "Remove Weekday", AccessibleRole::Button),
        (
            "add top",
            "Add field to the top line",
            AccessibleRole::Button,
        ),
        (
            "add bottom",
            "Add field to the bottom line",
            AccessibleRole::Button,
        ),
        ("reset lines", "Reset to default", AccessibleRole::Button),
        ("reset position", "Reset position", AccessibleRole::Button),
        ("shortcut", "Shortcut", AccessibleRole::TextInput),
        ("shortcut clear", "Clear shortcut", AccessibleRole::Button),
        ("shortcut reset", "Reset shortcut", AccessibleRole::Button),
    ] {
        tab_to(&window, stop);
        assert_eq!(
            element(&window, label).accessible_role(),
            Some(role),
            "{label}"
        );
    }
}

#[test]
fn wr10_ac2_a_focused_control_scrolls_into_view() {
    let (_f, window) = opened();
    assert!(
        window.get_scroll_y().abs() < f32::EPSILON,
        "it opens at the top"
    );
    tab_to(&window, "shortcut reset");
    let at_the_end = window.get_scroll_y();
    assert!(at_the_end < 0.0, "the last control is brought into view");
    assert!(
        element(&window, "Reset shortcut").size().height > 0.0,
        "and it exists in the tree"
    );
    tab_to(&window, "language");
    assert!(
        window.get_scroll_y() > at_the_end,
        "going back to the first control scrolls back up"
    );
    assert!(exists(&window, "Language"));
}

// The month title (W8-2 review).

#[test]
fn the_month_titles_accessible_name_carries_the_visible_title() {
    let (popup, _state, _) = app(date(2083, 7, 5));
    let by_label = |label: &str| -> bool {
        ElementHandle::find_by_accessible_label(&popup, label).count() == 1
    };
    assert!(
        by_label("Kartik 2083 · Oct/Nov 2026, change view"),
        "the month view"
    );
    assert!(!by_label("Change view"), "the bare name is gone");

    // The year view: the visible title is the year.
    ElementHandle::find_by_accessible_label(&popup, "Kartik 2083 · Oct/Nov 2026, change view")
        .next()
        .expect("the title")
        .invoke_accessible_default_action();
    assert!(by_label("2083, change view"), "the year view");
    ElementHandle::find_by_accessible_label(&popup, "2083, change view")
        .next()
        .expect("the title")
        .invoke_accessible_default_action();
    assert!(
        by_label("2080–2089"),
        "the decade view's title does nothing, so it is only the title"
    );
}

// Every UI string is `@tr(...)` with a Nepali `msgstr` (the W8 ground rules).

/// The text of every `@tr("...")` in `markup`, as written (so `\"` stays `\"`).
fn tr_strings(markup: &str) -> Vec<String> {
    let mut found = Vec::new();
    let mut rest = markup;
    while let Some(start) = rest.find("@tr(\"") {
        let after = &rest[start + 5..];
        let mut end = None;
        let mut escaped = false;
        for (index, c) in after.char_indices() {
            if escaped {
                escaped = false;
            } else if c == '\\' {
                escaped = true;
            } else if c == '"' {
                end = Some(index);
                break;
            }
        }
        let Some(end) = end else {
            break;
        };
        found.push(after[..end].to_owned());
        rest = &after[end..];
    }
    found
}

#[test]
fn every_tr_string_in_the_markup_has_a_nepali_entry_and_no_entry_is_left_over() {
    let markup = [
        include_str!("../ui/app-info.slint"),
        include_str!("../ui/app.slint"),
        include_str!("../ui/controls.slint"),
        include_str!("../ui/converter.slint"),
        include_str!("../ui/day-detail.slint"),
        include_str!("../ui/focus-frame.slint"),
        include_str!("../ui/font-test.slint"),
        include_str!("../ui/menu-text.slint"),
        include_str!("../ui/month-grid.slint"),
        include_str!("../ui/popup.slint"),
        include_str!("../ui/settings.slint"),
        include_str!("../ui/theme.slint"),
        include_str!("../ui/tray-icons.slint"),
        include_str!("../ui/tray.slint"),
    ];
    let po = include_str!("../i18n/ne/LC_MESSAGES/mitibar.po");
    let ids: std::collections::BTreeSet<String> = po
        .lines()
        .filter_map(|line| line.strip_prefix("msgid \""))
        .filter_map(|line| line.strip_suffix('"'))
        .filter(|id| !id.is_empty())
        .map(str::to_owned)
        .collect();
    let used: std::collections::BTreeSet<String> =
        markup.iter().flat_map(|file| tr_strings(file)).collect();
    let missing: Vec<&String> = used.difference(&ids).collect();
    assert!(missing.is_empty(), "no Nepali entry for {missing:?}");
    let left_over: Vec<&String> = ids.difference(&used).collect();
    assert!(left_over.is_empty(), "entries nothing uses: {left_over:?}");
}

#[test]
fn the_settings_window_speaks_nepali_when_the_language_is_nepali() {
    let _english = English;
    let (f, window) = opened();
    window.invoke_language_picked(1);
    assert_eq!(settings(&f).language, "ne");
    for label in ["भाषा", "थिम", "अतिरिक्त घडी", "२४ घण्टाको घडी"]
    {
        assert!(exists(&window, label), "{label}");
    }
    assert_eq!(window.get_shown_title().as_str(), "MitiBar सेटिङ");
    // The chips and their buttons follow.
    assert!(exists(&window, "बार"));
    assert!(exists(&window, "बार हटाउनुहोस्"));
}

// W8-4a: icons and captions sit in the middle of what holds them.

fn centre_x(element: &ElementHandle) -> f32 {
    element.absolute_position().x + element.size().width / 2.0
}

fn centre_y(element: &ElementHandle) -> f32 {
    element.absolute_position().y + element.size().height / 2.0
}

fn right_edge(element: &ElementHandle) -> f32 {
    element.absolute_position().x + element.size().width
}

/// The elements of `type_name` whose top-left corner is inside `outer`.
fn inside(window: &SettingsWindow, outer: &ElementHandle, type_name: &str) -> Vec<ElementHandle> {
    let (position, size) = (outer.absolute_position(), outer.size());
    ElementHandle::find_by_element_type_name(window, type_name)
        .filter(|inner| {
            let at = inner.absolute_position();
            at.x >= position.x
                && at.x < position.x + size.width
                && at.y >= position.y
                && at.y < position.y + size.height
        })
        .collect()
}

/// The `Text` element showing exactly `text`. Only a text that does not opt out of accessibility
/// has its text as its label, so this finds the notes, not the captions of controls.
fn text_element(window: &SettingsWindow, text: &str) -> ElementHandle {
    let found: Vec<ElementHandle> = ElementHandle::find_by_element_type_name(window, "Text")
        .filter(|element| element.accessible_label().as_deref() == Some(text))
        .collect();
    assert_eq!(
        found.len(),
        1,
        "expected one text {text:?}, found {}",
        found.len()
    );
    found.into_iter().next().expect("one text")
}

/// The one `Text` inside `holder`: a chip's name or a segment's caption.
fn caption_in(window: &SettingsWindow, holder: &ElementHandle) -> ElementHandle {
    let mut texts = inside(window, holder, "Text");
    assert!(!texts.is_empty(), "the holder has no text");
    texts.remove(0)
}

#[test]
fn w8_4a_the_saved_note_icon_is_level_with_its_text() {
    let (_f, window) = opened();
    window.set_save_status(1);
    walk(&window);
    let text = text_element(&window, "Saved");
    let icons: Vec<ElementHandle> = ElementHandle::find_by_element_type_name(&window, "LineIcon")
        .filter(|icon| (centre_y(icon) - centre_y(&text)).abs() < 20.0)
        .collect();
    assert_eq!(icons.len(), 1, "the check mark in the header row");
    let dy = centre_y(&icons[0]) - centre_y(&text);
    assert!(
        dy.abs() < 0.01,
        "the check mark is {dy} px off the text's middle"
    );
}

#[test]
fn w8_4a_the_add_field_pill_has_the_same_air_on_both_sides_of_the_ink() {
    let (_f, window) = opened();
    scroll_to_end(&window);
    let pill = element(&window, "Add field to the top line");
    let icons = inside(&window, &pill, "LineIcon");
    let texts = inside(&window, &pill, "Text");
    let (icon, text) = (&icons[0], &texts[0]);
    let left = icon.absolute_position().x - pill.absolute_position().x;
    let right = right_edge(&pill) - right_edge(text);
    assert!(
        (right - left - 2.0).abs() < 0.01,
        "the icon box is {left} px from the edge and the text ends {right} px from the other"
    );
    assert!((centre_y(icon) - centre_y(text)).abs() < 0.01);
    assert!((centre_y(icon) - centre_y(&pill)).abs() < 0.01);
}

#[test]
fn w8_4a_a_chip_centres_its_name_and_its_remove_button() {
    let (_f, window) = opened();
    scroll_to_end(&window);
    let chip = element(&window, "Weekday");
    let name = caption_in(&window, &chip);
    let remove = element(&window, "Remove Weekday");
    for (what, part) in [("name", &name), ("remove button", &remove)] {
        let dy = centre_y(part) - centre_y(&chip);
        assert!(
            dy.abs() < 0.01,
            "the {what} is {dy} px off the chip's middle"
        );
    }
    // 10 px before the name, and 4 px around a 20 px button that holds a 16 px icon with 4 px of
    // air at each side, so the cross is 8 px from the edge.
    let left = name.absolute_position().x - chip.absolute_position().x;
    let right = right_edge(&chip) - right_edge(&remove);
    assert!((left - 10.0).abs() < 0.01, "the name starts {left} px in");
    assert!(
        (right - 4.0).abs() < 0.01,
        "the remove button ends {right} px from the edge"
    );
}

#[test]
fn w8_4a_a_segment_centres_its_text() {
    let (_f, window) = opened();
    for label in ["English", "System", "Dark"] {
        let segment = element(&window, label);
        let text = caption_in(&window, &segment);
        let (dx, dy) = (
            centre_x(&text) - centre_x(&segment),
            centre_y(&text) - centre_y(&segment),
        );
        assert!(
            dx.abs() < 0.01 && dy.abs() < 0.01,
            "{label}: the text is {dx} px right and {dy} px below the segment's middle"
        );
    }
}

#[test]
fn w8_4a_a_row_centres_its_label_and_its_control() {
    let (_f, window) = opened();
    for (label, control) in [
        ("Language", "English"),
        ("Theme", "System"),
        ("Launch at login", "Launch at login"),
    ] {
        let control = element(&window, control);
        // The row's label is the text that ends where the row's controls begin, on the control's
        // row: the nearest text to the left.
        let text = ElementHandle::find_by_element_type_name(&window, "Text")
            .filter(|text| {
                text.absolute_position().x < control.absolute_position().x
                    && (centre_y(text) - centre_y(&control)).abs() < 14.0
            })
            .min_by(|a, b| {
                (centre_y(a) - centre_y(&control))
                    .abs()
                    .total_cmp(&(centre_y(b) - centre_y(&control)).abs())
            })
            .unwrap_or_else(|| panic!("no label for {label}"));
        let dy = centre_y(&control) - centre_y(&text);
        assert!(
            dy.abs() < 0.01,
            "{label}: the control is {dy} px off the label's middle"
        );
    }
}
