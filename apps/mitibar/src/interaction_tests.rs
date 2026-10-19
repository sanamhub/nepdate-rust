//! Headless interaction tests (W3-00a). Slint's testing backend finds controls by their
//! accessible label and invokes the same actions a screen reader would. No test touches the
//! system clipboard: the app gets a fake `Clipboard` port.

use std::cell::RefCell;
use std::rc::Rc;

use i_slint_backend_testing::{AccessibleRole, ElementHandle};
use nepdate::{Lang, Month, NepaliDate};
use slint::{ComponentHandle, Model};

use crate::app::settings::Settings;
use crate::platform::clipboard::Clipboard;
use crate::{AppState, PopupWindow, update_popup, wire_popup, wire_settings};

/// Records every clipboard write, so a test can assert the exact string.
#[derive(Default)]
struct FakeClipboard {
    texts: RefCell<Vec<String>>,
}

impl Clipboard for FakeClipboard {
    fn set_text(&self, text: &str) -> anyhow::Result<()> {
        self.texts.borrow_mut().push(text.to_owned());
        Ok(())
    }
}

/// A popup wired exactly like `main`, on a pinned date.
fn app(today: NepaliDate) -> (PopupWindow, Rc<RefCell<AppState>>, Rc<FakeClipboard>) {
    crate::test_support::init_platform();
    let clipboard = Rc::new(FakeClipboard::default());
    let state = Rc::new(RefCell::new(AppState::new(
        Settings::default(),
        today,
        Some(today),
        clipboard.clone(),
    )));
    let popup = PopupWindow::new().expect("popup component");
    let weak = popup.as_weak();
    state.borrow_mut().popup = weak.clone();
    wire_popup(&popup, &state, &weak);
    update_popup(&state);
    popup.show().expect("show popup");
    popup.invoke_focus();
    (popup, state, clipboard)
}

/// The single element with this accessible label.
fn element(popup: &PopupWindow, label: &str) -> ElementHandle {
    let found: Vec<ElementHandle> = ElementHandle::find_by_accessible_label(popup, label).collect();
    assert_eq!(
        found.len(),
        1,
        "expected one element labelled {label:?}, found {}",
        found.len()
    );
    found.into_iter().next().expect("one element")
}

/// Sets the month without going through the UI, then repaints.
fn show_month(state: &Rc<RefCell<AppState>>, year: u16, month: Month) {
    state.borrow_mut().viewed_month = (year, month);
    state.borrow_mut().detail = None;
    update_popup(state);
}

/// The accessible label of the cell whose BS day is `day` in the shown month.
fn day_label(popup: &PopupWindow, day: &str) -> String {
    let cells = popup.get_cells();
    let cell = cells
        .iter()
        .find(|cell| !cell.is_padding && cell.bs_day.as_str() == day)
        .expect("a real cell for the day");
    cell.accessible_label.to_string()
}

fn click(popup: &PopupWindow, label: &str) {
    element(popup, label).invoke_accessible_default_action();
}

/// One key press and release. Slint handles Tab on the press.
fn press(popup: &PopupWindow, key: slint::platform::Key) {
    for event in [
        slint::platform::WindowEvent::KeyPressed { text: key.into() },
        slint::platform::WindowEvent::KeyReleased { text: key.into() },
    ] {
        popup.window().dispatch_event(event);
    }
    slint::platform::update_timers_and_animations();
}

/// Shift+Tab: press Shift, tap Tab, release Shift, so Slint sees the modifier.
fn press_shift_tab(popup: &PopupWindow) {
    use slint::platform::{Key, WindowEvent};
    popup.window().dispatch_event(WindowEvent::KeyPressed {
        text: Key::Shift.into(),
    });
    press(popup, Key::Tab);
    popup.window().dispatch_event(WindowEvent::KeyReleased {
        text: Key::Shift.into(),
    });
}

fn focused(popup: &PopupWindow) -> String {
    popup.get_focused_control().to_string()
}

/// The base order from the plan, with the real day labels from the populated cells.
fn base_order(popup: &PopupWindow) -> Vec<String> {
    let mut order = [
        "convert",
        "previous month",
        "next month",
        "month",
        "year",
        "previous year",
        "next year",
        "today",
    ]
    .map(str::to_owned)
    .to_vec();
    for cell in popup.get_cells().iter().filter(|cell| !cell.is_padding) {
        order.push(format!("day {}", cell.bs_day));
    }
    order
}

#[test]
fn previous_next_and_today_follow_the_range_edges() {
    let (popup, state, _) = app(NepaliDate::new(2081, 1, 1).expect("date"));
    assert!(popup.get_month_title().starts_with("Baishakh 2081"));
    assert_eq!(
        element(&popup, "Previous month").accessible_enabled(),
        Some(true)
    );
    assert_eq!(
        element(&popup, "Next month").accessible_enabled(),
        Some(true)
    );

    click(&popup, "Next month");
    assert!(popup.get_month_title().starts_with("Jestha 2081"));
    click(&popup, "Previous month");
    assert!(popup.get_month_title().starts_with("Baishakh 2081"));

    show_month(&state, 1901, Month::Baishakh);
    assert_eq!(
        element(&popup, "Previous month").accessible_enabled(),
        Some(false)
    );
    click(&popup, "Previous month");
    assert_eq!(
        popup.get_month_title().as_str(),
        "Baishakh 1901 · Apr/May 1844"
    );

    show_month(&state, 2199, Month::Chaitra);
    assert_eq!(
        element(&popup, "Next month").accessible_enabled(),
        Some(false)
    );
    click(&popup, "Next month");
    assert_eq!(
        popup.get_month_title().as_str(),
        "Chaitra 2199 · Mar/Apr 2143"
    );

    click(&popup, "Today");
    assert!(popup.get_month_title().starts_with("Baishakh 2081"));
}

#[test]
fn month_and_year_pickers_jump() {
    let (popup, _state, _) = app(NepaliDate::new(2081, 1, 1).expect("date"));

    let month = element(&popup, "Month");
    assert_eq!(month.accessible_role(), Some(AccessibleRole::Combobox));
    month.invoke_accessible_expand_action();
    let ashoj = element(&popup, "Ashoj");
    assert_eq!(ashoj.accessible_role(), Some(AccessibleRole::ListItem));
    // The list item has no accessible default action; a real pointer click drives it.
    ashoj.mock_single_click(slint::platform::PointerEventButton::Left);
    assert!(popup.get_month_title().starts_with("Ashoj 2081"));

    let year = element(&popup, "Year");
    assert_eq!(year.accessible_role(), Some(AccessibleRole::Spinbox));
    year.set_accessible_value("2082");
    assert!(popup.get_month_title().starts_with("Ashoj 2082"));
}

#[test]
fn one_wheel_notch_changes_one_month() {
    let (popup, _state, _) = app(NepaliDate::new(2081, 2, 1).expect("date"));
    assert!(popup.get_month_title().starts_with("Jestha 2081"));

    let cell = element(&popup, &day_label(&popup, "1"));
    cell.scroll(0.0, -1.0);
    assert!(popup.get_month_title().starts_with("Baishakh 2081"));

    let cell = element(&popup, &day_label(&popup, "1"));
    cell.scroll(0.0, 1.0);
    assert!(popup.get_month_title().starts_with("Jestha 2081"));
}

#[test]
fn a_day_click_opens_the_detail_and_escape_closes_detail_then_popup() {
    let (popup, _state, _) = app(NepaliDate::new(2081, 1, 1).expect("date"));
    let label = day_label(&popup, "1");
    assert_eq!(
        label, "2081/01/01 2024-04-13 Saturday Panchami",
        "the label is date, weekday and tithi data"
    );
    click(&popup, &label);
    assert!(popup.get_detail_visible());
    assert_eq!(popup.get_detail().bs_long.as_str(), "Baishakh 1, 2081");

    popup
        .window()
        .dispatch_event(slint::platform::WindowEvent::KeyPressed {
            text: slint::platform::Key::Escape.into(),
        });
    assert!(
        !popup.get_detail_visible(),
        "Escape closes the detail first"
    );
    assert!(popup.window().is_visible(), "the popup stays open");

    popup
        .window()
        .dispatch_event(slint::platform::WindowEvent::KeyPressed {
            text: slint::platform::Key::Escape.into(),
        });
    assert!(
        !popup.window().is_visible(),
        "the second Escape hides the popup"
    );
}

#[test]
fn copy_buttons_write_the_exact_strings_to_the_injected_clipboard() {
    let (popup, state, clipboard) = app(NepaliDate::new(2081, 1, 1).expect("date"));
    let label = day_label(&popup, "1");
    click(&popup, &label);
    click(&popup, "Copy BS");
    click(&popup, "Copy AD");
    // The tray copies use the same date and, like the detail buttons, the language digits.
    crate::copy_bs(&state);
    crate::copy_ad(&state);
    assert_eq!(
        *clipboard.texts.borrow(),
        vec![
            "2081/01/01".to_owned(),
            "2024-04-13".to_owned(),
            "2081/01/01".to_owned(),
            "2024-04-13".to_owned()
        ]
    );

    // W5-04: Nepali gets Devanagari digits in both copies, not only Copy BS.
    state.borrow_mut().settings.language = "ne".to_owned();
    let _ = slint::select_bundled_translation("ne");
    crate::copy_bs(&state);
    crate::copy_ad(&state);
    assert_eq!(
        clipboard.texts.borrow()[4..],
        ["२०८१/०१/०१".to_owned(), "२०२४-०४-१३".to_owned()]
    );
}

#[test]
fn the_nine_event_list_moves_when_scrolled() {
    let (popup, _state, _) = app(NepaliDate::new(2081, 6, 1).expect("date"));
    let label = day_label(&popup, "1");
    click(&popup, &label);
    assert_eq!(popup.get_detail().events.row_count(), 9);

    let first = element(&popup, "Vishwakarma Puja");
    let before = first.absolute_position().y;
    let list = element(&popup, "Event list");
    assert_eq!(list.accessible_role(), Some(AccessibleRole::List));
    list.scroll(0.0, -40.0);
    let after = first.absolute_position().y;
    assert!(
        after < before,
        "content-y moved: first event y {before} -> {after}"
    );
}

#[test]
fn every_control_has_a_label_and_a_role() {
    let (popup, _state, _) = app(NepaliDate::new(2081, 1, 1).expect("date"));
    for (label, role) in [
        ("Previous month", AccessibleRole::Button),
        ("Next month", AccessibleRole::Button),
        ("Today", AccessibleRole::Button),
        ("Month", AccessibleRole::Combobox),
        ("Year", AccessibleRole::Spinbox),
    ] {
        let control = element(&popup, label);
        assert_eq!(control.accessible_role(), Some(role), "{label}");
    }
    let cell = element(&popup, &day_label(&popup, "1"));
    assert_eq!(cell.accessible_role(), Some(AccessibleRole::Button));

    click(&popup, &day_label(&popup, "1"));
    assert_eq!(
        element(&popup, "Copy BS").accessible_role(),
        Some(AccessibleRole::Button)
    );
    assert_eq!(
        element(&popup, "Copy AD").accessible_role(),
        Some(AccessibleRole::Button)
    );
    assert_eq!(
        element(&popup, "Close day detail").accessible_role(),
        Some(AccessibleRole::Button)
    );
    assert_eq!(
        element(&popup, "Event list").accessible_role(),
        Some(AccessibleRole::List)
    );
}

/// W3-04 switches language at runtime; Slint must be able to go back to the source language.
#[test]
fn bundled_translation_switches_to_nepali_and_back() {
    crate::test_support::init_platform();
    let popup = PopupWindow::new().expect("popup component");
    let menu = |popup: &PopupWindow| popup.global::<crate::MenuText>().get_settings().to_string();
    slint::select_bundled_translation("ne").expect("Nepali bundle");
    assert_eq!(menu(&popup), "सेटिङ्हरू");
    slint::select_bundled_translation("en").expect("source language");
    assert_eq!(menu(&popup), "Settings");
}

/// Keep the month name model in step with the language helper used above.
#[test]
fn month_names_come_from_the_library() {
    let (popup, _state, _) = app(NepaliDate::new(2081, 1, 1).expect("date"));
    let months = popup.get_month_names();
    assert_eq!(months.row_count(), 12);
    assert_eq!(months.row_data(0).expect("Baishakh").as_str(), "Baishakh");
    assert_eq!(Month::Baishakh.name(Lang::En), "Baishakh");
}

#[test]
fn converter_converts_typed_input_both_ways() {
    let (popup, _state, _) = app(NepaliDate::new(2081, 1, 1).expect("date"));
    element(&popup, "Convert").invoke_accessible_default_action();
    assert!(popup.get_converter_visible());

    element(&popup, "Date to convert").set_accessible_value("2081/02/32");
    let data = popup.get_converter_data();
    assert_eq!(data.short.as_str(), "2024-06-14");
    assert_eq!(data.error.as_str(), "");

    element(&popup, "Date to convert").set_accessible_value("2081/02/33");
    assert_eq!(popup.get_converter_data().error.as_str(), "Invalid date");

    click(&popup, "AD → BS");
    element(&popup, "Date to convert").set_accessible_value("2024-04-13");
    assert_eq!(popup.get_converter_data().short.as_str(), "2081/01/01");

    click(&popup, "Days tab");
    element(&popup, "From BS date").set_accessible_value("2081/01/01");
    element(&popup, "To BS date").set_accessible_value("2081/02/01");
    assert_eq!(popup.get_converter_data().short.as_str(), "31 days");
}

#[test]
fn a_settings_switch_applies_at_once_and_saves_after_300_ms() {
    let (popup, state, _) = app(NepaliDate::new(2081, 1, 1).expect("date"));
    let dir = tempfile::tempdir().expect("tempdir");
    let path = dir.path().join("settings.toml");
    state.borrow_mut().settings_path = Some(path.clone());
    let save_timer = Rc::new(slint::Timer::default());
    wire_settings(&popup, &state, &save_timer);
    crate::refresh_settings(&popup, &state);
    popup.set_settings_visible(true);
    assert!(!state.borrow().settings.clock_24h);

    click(&popup, "24-hour clock");
    assert!(state.borrow().settings.clock_24h, "applied at once");
    assert!(!path.exists(), "not saved before the debounce");

    i_slint_backend_testing::mock_elapsed_time(std::time::Duration::from_millis(350));
    slint::platform::update_timers_and_animations();
    let (saved, warnings) = crate::platform::settings_store::load(&path);
    assert!(warnings.is_empty(), "{warnings:?}");
    assert!(saved.clock_24h, "saved after 300 ms");
}

#[test]
fn the_year_control_shows_language_digits_and_stops_at_the_edges() {
    let (popup, state, _) = app(NepaliDate::new(2081, 1, 1).expect("date"));
    let year = element(&popup, "Year");
    assert_eq!(year.accessible_role(), Some(AccessibleRole::Spinbox));
    assert_eq!(year.accessible_value().as_deref(), Some("2081"));
    assert_eq!(popup.get_year_text().as_str(), "2081");

    state.borrow_mut().settings.language = "ne".to_owned();
    let _ = slint::select_bundled_translation("ne");
    update_popup(&state);
    assert_eq!(popup.get_year_text().as_str(), "२०८१");
    assert_eq!(
        element(&popup, "वर्ष").accessible_value().as_deref(),
        Some("२०८१")
    );

    state.borrow_mut().settings.language = "en".to_owned();
    let _ = slint::select_bundled_translation("en");
    update_popup(&state);

    show_month(&state, 1901, Month::Baishakh);
    element(&popup, "Year").invoke_accessible_decrement_action();
    assert_eq!(popup.get_year(), 1901, "1901 is the lower edge");
    element(&popup, "Year").invoke_accessible_increment_action();
    assert_eq!(popup.get_year(), 1902);

    show_month(&state, 2199, Month::Chaitra);
    element(&popup, "Year").invoke_accessible_increment_action();
    assert_eq!(popup.get_year(), 2199, "2199 is the upper edge");
    element(&popup, "Year").invoke_accessible_decrement_action();
    assert_eq!(popup.get_year(), 2198);
}

#[test]
fn the_visible_year_buttons_step_and_stop_at_the_edges() {
    let (popup, state, _) = app(NepaliDate::new(2081, 1, 1).expect("date"));

    let prev = element(&popup, "Previous year");
    let next = element(&popup, "Next year");
    assert_eq!(prev.accessible_role(), Some(AccessibleRole::Button));
    assert_eq!(next.accessible_role(), Some(AccessibleRole::Button));
    assert!(
        prev.size().width >= 24.0 && prev.size().height >= 24.0,
        "previous-year hit target is {}x{}",
        prev.size().width,
        prev.size().height
    );
    assert!(
        next.size().width >= 24.0 && next.size().height >= 24.0,
        "next-year hit target is {}x{}",
        next.size().width,
        next.size().height
    );

    next.invoke_accessible_default_action();
    assert_eq!(popup.get_year(), 2082);
    prev.invoke_accessible_default_action();
    assert_eq!(popup.get_year(), 2081);

    // A pointer click steps exactly one year (the TouchArea path, not only the accessible
    // action).
    next.mock_single_click(slint::platform::PointerEventButton::Left);
    assert_eq!(popup.get_year(), 2082, "one click is one year");
    assert_eq!(
        focused(&popup),
        "next year",
        "a click moves focus to the button"
    );
    let prev = element(&popup, "Previous year");
    prev.mock_single_click(slint::platform::PointerEventButton::Left);
    assert_eq!(popup.get_year(), 2081, "one click is one year");
    assert_eq!(
        focused(&popup),
        "previous year",
        "a click moves focus to the button"
    );

    // W6-03 AC2: the buttons carry the Devanagari year text and translated labels.
    state.borrow_mut().settings.language = "ne".to_owned();
    let _ = slint::select_bundled_translation("ne");
    update_popup(&state);
    assert_eq!(popup.get_year_text().as_str(), "२०८१");
    element(&popup, "अर्को वर्ष").invoke_accessible_default_action();
    assert_eq!(popup.get_year(), 2082);
    assert_eq!(popup.get_year_text().as_str(), "२०८२");
    element(&popup, "अघिल्लो वर्ष").invoke_accessible_default_action();
    assert_eq!(popup.get_year(), 2081);

    state.borrow_mut().settings.language = "en".to_owned();
    let _ = slint::select_bundled_translation("en");
    update_popup(&state);

    show_month(&state, 1901, Month::Baishakh);
    let prev = element(&popup, "Previous year");
    assert_eq!(prev.accessible_enabled(), Some(false));
    prev.invoke_accessible_default_action();
    assert_eq!(popup.get_year(), 1901, "1901 is the lower edge");

    show_month(&state, 2199, Month::Chaitra);
    let next = element(&popup, "Next year");
    assert_eq!(next.accessible_enabled(), Some(false));
    next.invoke_accessible_default_action();
    assert_eq!(popup.get_year(), 2199, "2199 is the upper edge");

    // Keyboard and wheel still reach the same control (W6-03 AC3).
    show_month(&state, 2081, Month::Baishakh);
    let year = element(&popup, "Year");
    year.mock_single_click(slint::platform::PointerEventButton::Left);
    popup
        .window()
        .dispatch_event(slint::platform::WindowEvent::KeyPressed {
            text: slint::platform::Key::UpArrow.into(),
        });
    assert_eq!(popup.get_year(), 2082);
    let year = element(&popup, "Year");
    year.scroll(0.0, 1.0);
    assert_eq!(popup.get_year(), 2083);
    let year = element(&popup, "Year");
    year.scroll(0.0, -1.0);
    assert_eq!(popup.get_year(), 2082);
}

#[test]
fn tab_and_shift_tab_follow_the_focus_order() {
    let (popup, _state, _) = app(NepaliDate::new(2081, 1, 1).expect("date"));
    let order = base_order(&popup);
    assert_eq!(focused(&popup), "", "the root, not a control");

    let mut forward = Vec::new();
    for _ in 0..order.len() {
        press(&popup, slint::platform::Key::Tab);
        forward.push(focused(&popup));
    }
    assert_eq!(forward, order, "Tab order from the root");
    assert!(
        popup.window().is_visible(),
        "moving focus between controls never hides the popup"
    );

    // From the last day, Shift+Tab retraces the order and wraps from the first control to the
    // last. The root is not a stop, so there is no empty step.
    let mut reverse = Vec::new();
    for _ in 0..order.len() {
        press_shift_tab(&popup);
        reverse.push(focused(&popup));
    }
    assert_eq!(
        reverse,
        order[..order.len() - 1]
            .iter()
            .rev()
            .chain(order[order.len() - 1..].iter())
            .cloned()
            .collect::<Vec<_>>(),
        "Shift+Tab order from the last day"
    );
}

#[test]
fn disabled_and_hidden_controls_are_not_focus_stops() {
    let (popup, state, _) = app(NepaliDate::new(2081, 1, 1).expect("date"));

    // 1901 Baishakh: Previous month and Previous year are disabled.
    show_month(&state, 1901, Month::Baishakh);
    let mut order = Vec::new();
    for _ in 0..6 {
        press(&popup, slint::platform::Key::Tab);
        order.push(focused(&popup));
    }
    assert_eq!(
        order,
        [
            "convert",
            "next month",
            "month",
            "year",
            "next year",
            "today"
        ],
        "1901 skips both previous controls"
    );

    // 2199 Chaitra: Next month and Next year are disabled.
    show_month(&state, 2199, Month::Chaitra);
    popup.invoke_focus();
    let mut order = Vec::new();
    for _ in 0..5 {
        press(&popup, slint::platform::Key::Tab);
        order.push(focused(&popup));
    }
    assert_eq!(
        order,
        [
            "convert",
            "previous month",
            "month",
            "year",
            "previous year"
        ],
        "2199 skips both next controls"
    );

    // A closed overlay is not in the tree at all.
    assert_eq!(
        ElementHandle::find_by_accessible_label(&popup, "Close converter").count(),
        0
    );
    assert_eq!(
        ElementHandle::find_by_accessible_label(&popup, "Close settings").count(),
        0
    );
}

#[test]
fn overlays_take_and_return_focus() {
    let (popup, state, _) = app(NepaliDate::new(2081, 1, 1).expect("date"));

    // The base is inert while the converter is open, so the first Tab enters the overlay.
    click(&popup, "Convert");
    assert!(popup.get_converter_visible());
    assert_eq!(focused(&popup), "", "no base control keeps focus");
    for expected in [
        "converter close",
        "converter convert tab",
        "converter days tab",
        "converter bs to ad",
        "converter ad to bs",
        "converter input",
        "converter close",
    ] {
        press(&popup, slint::platform::Key::Tab);
        assert_eq!(focused(&popup), expected, "converter Tab order");
    }
    for expected in [
        "converter input",
        "converter ad to bs",
        "converter bs to ad",
        "converter days tab",
        "converter convert tab",
        "converter close",
    ] {
        press_shift_tab(&popup);
        assert_eq!(focused(&popup), expected, "converter Shift+Tab order");
    }

    press(&popup, slint::platform::Key::Escape);
    assert!(!popup.get_converter_visible());
    assert_eq!(focused(&popup), "convert", "the base resumes at Convert");
    press(&popup, slint::platform::Key::Tab);
    assert_eq!(
        focused(&popup),
        "previous month",
        "the base order continues"
    );

    // Settings is opened by the tray menu, so the test sets the flag and refreshes the model.
    popup.set_settings_visible(true);
    crate::refresh_settings(&popup, &state);
    popup.invoke_focus();
    for expected in [
        "settings close",
        "settings language",
        "settings theme",
        "settings 24-hour",
        "settings seconds",
        "settings filter",
        "settings no extra clock",
        "settings launch",
        "settings taskbar",
        "settings close",
    ] {
        press(&popup, slint::platform::Key::Tab);
        assert_eq!(focused(&popup), expected, "settings Tab order");
    }
    for expected in [
        "settings taskbar",
        "settings launch",
        "settings no extra clock",
        "settings filter",
        "settings seconds",
        "settings 24-hour",
        "settings theme",
        "settings language",
        "settings close",
    ] {
        press_shift_tab(&popup);
        assert_eq!(focused(&popup), expected, "settings Shift+Tab order");
    }
    press(&popup, slint::platform::Key::Escape);
    assert!(!popup.get_settings_visible());
    assert_eq!(focused(&popup), "convert", "the base resumes at Convert");

    // The day detail holds focus on its two copy buttons.
    popup.invoke_focus();
    let label = day_label(&popup, "1");
    click(&popup, &label);
    assert!(popup.get_detail_visible());
    assert_eq!(focused(&popup), "", "no base control keeps focus");
    for expected in ["detail copy bs", "detail copy ad", "detail copy bs"] {
        press(&popup, slint::platform::Key::Tab);
        assert_eq!(focused(&popup), expected, "detail Tab order");
    }
    press(&popup, slint::platform::Key::Escape);
    assert!(!popup.get_detail_visible());
    assert_eq!(focused(&popup), "convert", "the base resumes at Convert");

    // A reopened converter does not report the control that had focus when it closed.
    click(&popup, "Convert");
    for _ in 0..6 {
        press(&popup, slint::platform::Key::Tab);
    }
    assert_eq!(focused(&popup), "converter input");
    press(&popup, slint::platform::Key::Escape);
    click(&popup, "Convert");
    assert!(popup.get_converter_visible());
    assert_eq!(
        focused(&popup),
        "",
        "nothing in the reopened converter has focus yet"
    );
    press(&popup, slint::platform::Key::Tab);
    assert_eq!(focused(&popup), "converter close");
    assert!(popup.window().is_visible(), "overlays never hide the popup");
}

#[test]
fn return_and_space_step_once_per_press() {
    let (popup, state, _) = app(NepaliDate::new(2081, 1, 1).expect("date"));

    // Year: one press is one year, the release adds nothing.
    popup.invoke_focus_next_year();
    assert_eq!(focused(&popup), "next year");
    press(&popup, slint::platform::Key::Return);
    assert_eq!(popup.get_year(), 2082, "Return adds one year");
    press(&popup, slint::platform::Key::Space);
    assert_eq!(popup.get_year(), 2083, "Space adds one year");
    popup
        .window()
        .dispatch_event(slint::platform::WindowEvent::KeyReleased {
            text: slint::platform::Key::Space.into(),
        });
    assert_eq!(popup.get_year(), 2083, "the release adds nothing");

    // Month: the same two keys on a standard arrow button.
    show_month(&state, 2081, Month::Baishakh);
    popup.invoke_focus();
    press(&popup, slint::platform::Key::Tab);
    press(&popup, slint::platform::Key::Tab);
    assert_eq!(focused(&popup), "previous month");
    press(&popup, slint::platform::Key::Return);
    assert!(
        popup.get_month_title().starts_with("Chaitra 2080"),
        "Return moves one month back: {}",
        popup.get_month_title()
    );
    press(&popup, slint::platform::Key::Space);
    assert!(
        popup.get_month_title().starts_with("Falgun 2080"),
        "Space moves one month back: {}",
        popup.get_month_title()
    );
}

#[test]
fn return_and_space_skip_disabled_edges() {
    // 1901: Previous month and Previous year are skipped.
    let (popup, _state, _) = app(NepaliDate::new(1901, 1, 1).expect("date"));
    press(&popup, slint::platform::Key::Tab);
    press(&popup, slint::platform::Key::Tab);
    assert_eq!(focused(&popup), "next month", "Previous month is skipped");
    press(&popup, slint::platform::Key::Return);
    assert!(popup.get_month_title().starts_with("Jestha 1901"));

    popup.invoke_focus();
    for _ in 0..6 {
        press(&popup, slint::platform::Key::Tab);
    }
    assert_eq!(focused(&popup), "next year");
    press(&popup, slint::platform::Key::Space);
    assert_eq!(popup.get_year(), 1902);

    // 2199: Next month and Next year are skipped.
    let (popup, _state, _) = app(NepaliDate::new(2199, 12, 30).expect("date"));
    press(&popup, slint::platform::Key::Tab);
    press(&popup, slint::platform::Key::Tab);
    assert_eq!(focused(&popup), "previous month");
    press(&popup, slint::platform::Key::Space);
    assert!(popup.get_month_title().starts_with("Falgun 2199"));

    popup.invoke_focus();
    for _ in 0..6 {
        press(&popup, slint::platform::Key::Tab);
    }
    assert_eq!(focused(&popup), "previous year");
    press(&popup, slint::platform::Key::Return);
    assert_eq!(popup.get_year(), 2198);
}

#[test]
fn window_deactivation_hides_the_popup_while_a_control_has_focus() {
    let (popup, _state, _) = app(NepaliDate::new(2081, 1, 1).expect("date"));
    popup.invoke_focus_next_year();
    assert_eq!(focused(&popup), "next year");

    // The winit filter calls this when Focused(false) arrives. A FocusScope focus-lost would
    // reach only the focused button, not the popup root.
    popup.invoke_window_deactivated();
    assert!(
        !popup.window().is_visible(),
        "the popup hides when the window loses activation"
    );
}

#[test]
fn window_active_changed_hides_the_popup_when_the_root_has_focus() {
    let (popup, _state, _) = app(NepaliDate::new(2081, 1, 1).expect("date"));
    assert_eq!(focused(&popup), "", "the root holds focus");

    popup
        .window()
        .dispatch_event(slint::platform::WindowEvent::WindowActiveChanged(false));
    assert!(
        !popup.window().is_visible(),
        "the root FocusScope sees the activation loss"
    );
}

#[test]
fn the_popup_is_created_on_first_open_not_at_startup() {
    crate::test_support::init_platform();
    let today = NepaliDate::new(2081, 1, 1).expect("date");
    let state = Rc::new(RefCell::new(AppState::new(
        Settings::default(),
        today,
        Some(today),
        Rc::new(FakeClipboard::default()),
    )));
    let slot: crate::PopupSlot = Rc::new(RefCell::new(None));
    let popup = crate::ensure_popup(&slot, &state).expect("popup");
    assert!(!popup.window().is_visible(), "created hidden");
    assert!(slot.borrow().is_some(), "the slot owns the strong handle");
    assert!(
        state.borrow().popup.upgrade().is_some(),
        "state has the weak handle"
    );
}

#[test]
fn the_lazy_popup_is_created_once_and_honours_the_theme_flag() {
    crate::test_support::init_platform();
    let today = NepaliDate::new(2081, 1, 1).expect("date");
    let state = Rc::new(RefCell::new(AppState::new(
        Settings::default(),
        today,
        Some(today),
        Rc::new(FakeClipboard::default()),
    )));
    // W5 review: `--theme` reached the eager popup; the lazy one must still apply it.
    state.borrow_mut().theme_override = Some(crate::ThemeChoice::Dark);
    let slot: crate::PopupSlot = Rc::new(RefCell::new(None));
    let first = crate::ensure_popup(&slot, &state).expect("popup");
    assert_eq!(
        first.global::<crate::Theme>().get_choice(),
        crate::ThemeChoice::Dark
    );
    first.set_month_title("kept".into());
    let second = crate::ensure_popup(&slot, &state).expect("popup again");
    assert_eq!(
        second.get_month_title().as_str(),
        "kept",
        "the second open reuses the first component, not a new one"
    );
}

#[test]
fn wr09_ac1_second_launch_opens_the_first_instance_popup() {
    let (popup, state, _) = app(NepaliDate::new(2081, 1, 1).expect("date"));
    popup.hide().expect("hide");
    assert!(!popup.window().is_visible());
    let slot = Rc::new(RefCell::new(Some(popup.clone_strong())));
    crate::install_open_request(&slot, &state);
    crate::run_open_request();
    assert!(
        popup.window().is_visible(),
        "the second-launch wake shows the popup"
    );
}
