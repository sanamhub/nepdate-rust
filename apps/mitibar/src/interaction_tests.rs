//! Headless interaction tests. Slint's testing backend finds controls by their
//! accessible label and invokes the same actions a screen reader would. No test touches the
//! system clipboard: the app gets a fake `Clipboard` port.

use std::cell::RefCell;
use std::rc::Rc;

use i_slint_backend_testing::{AccessibleRole, ElementHandle};
use nepdate::{Lang, Month, NepaliDate};
use slint::{ComponentHandle, Model};

use crate::app::bar::BarLines;
use crate::app::bar_place::{Rect, Saved};
use crate::app::settings::Settings;
use crate::app::zoom::Zoom;
use crate::platform::bar::{Anchor, Bar, BarEvent, MenuItem, MenuLabels};
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

fn date(year: u16, month: u8, day: u8) -> NepaliDate {
    NepaliDate::new(year, month, day).expect("valid BS date")
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

/// Whether any element carries this accessible label.
fn exists(popup: &PopupWindow, label: &str) -> bool {
    ElementHandle::find_by_accessible_label(popup, label).count() > 0
}

/// Opens `year`/`month` in the month view without going through the UI, then repaints.
fn show_month(state: &Rc<RefCell<AppState>>, year: u16, month: Month) {
    {
        let mut s = state.borrow_mut();
        s.zoom.jump_to(date(year, month.number(), 1));
        s.detail = None;
    }
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

/// Types `text` one character at a time, as the keyboard would.
fn type_text(popup: &PopupWindow, text: &str) {
    for character in text.chars() {
        let text: slint::SharedString = character.to_string().into();
        for event in [
            slint::platform::WindowEvent::KeyPressed { text: text.clone() },
            slint::platform::WindowEvent::KeyReleased { text },
        ] {
            popup.window().dispatch_event(event);
        }
    }
    slint::platform::update_timers_and_animations();
}

/// A key with Ctrl held: press Control, tap the key, release Control.
fn press_ctrl(popup: &PopupWindow, text: &str) {
    use slint::platform::{Key, WindowEvent};
    popup.window().dispatch_event(WindowEvent::KeyPressed {
        text: Key::Control.into(),
    });
    type_text(popup, text);
    popup.window().dispatch_event(WindowEvent::KeyReleased {
        text: Key::Control.into(),
    });
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

/// Tabs until `name` has focus, with a bound so a missing stop fails and does not hang.
fn tab_to(popup: &PopupWindow, name: &str) {
    for _ in 0..30 {
        press(popup, slint::platform::Key::Tab);
        if focused(popup) == name {
            return;
        }
    }
    panic!(
        "Tab never reached {name:?}; the last stop was {:?}",
        focused(popup)
    );
}

/// Moves the pointer to the middle of `element`.
fn hover(popup: &PopupWindow, element: &ElementHandle) {
    let position = element.absolute_position();
    let size = element.size();
    popup
        .window()
        .dispatch_event(slint::platform::WindowEvent::PointerMoved {
            position: slint::LogicalPosition::new(
                position.x + size.width / 2.0,
                position.y + size.height / 2.0,
            ),
        });
}

/// Moves the pointer to a corner of the window, off every control.
fn leave(popup: &PopupWindow) {
    popup
        .window()
        .dispatch_event(slint::platform::WindowEvent::PointerMoved {
            position: slint::LogicalPosition::new(2.0, 515.0),
        });
}

/// Lets `millis` of the mocked clock pass and runs the timers due in it.
fn elapse(millis: u64) {
    i_slint_backend_testing::mock_elapsed_time(std::time::Duration::from_millis(millis));
    slint::platform::update_timers_and_animations();
}

/// The popup's title row in the month view: `Ashoj 2083 · Sep/Oct 2026`.
fn nav_title(popup: &PopupWindow) -> String {
    popup.get_nav_title().to_string()
}

/// The labels of the year or decade grid's cells.
fn zoom_labels(popup: &PopupWindow) -> Vec<String> {
    popup
        .get_zoom_cells()
        .iter()
        .map(|cell| cell.accessible_label.to_string())
        .collect()
}

/// The base order in visual order. `away` adds the Today pill, which shows only away from today's
/// month; `holiday` adds the holiday line, which needs patro data for the day.
fn base_order(away: bool, holiday: bool) -> Vec<&'static str> {
    let mut order = vec!["convert", "settings", "pin", "previous", "title", "next"];
    if away {
        order.push("today");
    }
    order.push("grid");
    if holiday {
        order.push("upcoming");
    }
    order
}

#[test]
fn previous_next_and_today_follow_the_range_edges() {
    let (popup, state, _) = app(date(2081, 1, 1));
    assert!(popup.get_month_title().starts_with("Baishakh 2081"));
    assert_eq!(
        element(&popup, "Previous month").accessible_enabled(),
        Some(true)
    );
    assert_eq!(
        element(&popup, "Next month").accessible_enabled(),
        Some(true)
    );
    assert!(!popup.get_today_shown(), "today's month has no Today pill");

    click(&popup, "Next month");
    assert!(popup.get_month_title().starts_with("Jestha 2081"));
    assert!(popup.get_today_shown());
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
    assert!(!popup.get_today_shown());
}

#[test]
fn a_step_that_reaches_the_edge_moves_the_focus_to_the_grid() {
    // A control disabled while it has focus keeps its ring, so the focus must leave it.
    let (popup, state, _) = app(date(2081, 1, 1));
    show_month(&state, 1901, Month::Jestha);
    tab_to(&popup, "previous");
    press(&popup, slint::platform::Key::Return);
    assert!(popup.get_month_title().starts_with("Baishakh 1901"));
    assert_eq!(focused(&popup), "grid");

    show_month(&state, 2199, Month::Falgun);
    tab_to(&popup, "next");
    press(&popup, slint::platform::Key::Return);
    assert!(popup.get_month_title().starts_with("Chaitra 2199"));
    assert_eq!(focused(&popup), "grid");
}

#[test]
fn one_wheel_notch_changes_one_month() {
    let (popup, _state, _) = app(date(2081, 2, 1));
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
    let (popup, _state, _) = app(date(2081, 1, 1));
    let label = day_label(&popup, "1");
    assert_eq!(
        label, "2081/01/01 2024-04-13 Saturday Panchami",
        "the label is date, weekday and tithi data"
    );
    click(&popup, &label);
    assert!(popup.get_detail_visible());
    assert_eq!(popup.get_detail().bs_long.as_str(), "Baishakh 1, 2081");

    press(&popup, slint::platform::Key::Escape);
    assert!(
        !popup.get_detail_visible(),
        "Escape closes the detail first"
    );
    assert!(popup.window().is_visible(), "the popup stays open");

    press(&popup, slint::platform::Key::Escape);
    assert!(
        !popup.window().is_visible(),
        "the second Escape hides the popup"
    );
}

#[test]
fn copy_buttons_write_the_exact_strings_to_the_injected_clipboard() {
    let (popup, state, clipboard) = app(date(2081, 1, 1));
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

    // Nepali gets Devanagari digits in both copies, not only Copy BS.
    state.borrow_mut().settings.language = "ne".to_owned();
    let _ = slint::select_bundled_translation("ne");
    crate::copy_bs(&state);
    crate::copy_ad(&state);
    assert_eq!(
        clipboard.texts.borrow()[4..],
        ["२०८१/०१/०१".to_owned(), "२०२४-०४-१३".to_owned()]
    );
    let _ = slint::select_bundled_translation("en");
}

#[test]
fn the_nine_event_list_moves_when_scrolled() {
    let (popup, _state, _) = app(date(2081, 6, 1));
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
    let (popup, state, _) = app(date(2081, 1, 1));
    // Today's pill shows only away from today's month, so look at the month after.
    show_month(&state, 2081, Month::Jestha);
    for label in [
        "Convert",
        "Settings",
        "Pin, not pinned",
        "Previous month",
        "Change view",
        "Next month",
        "Today",
        "Upcoming holidays",
    ] {
        let control = element(&popup, label);
        assert_eq!(
            control.accessible_role(),
            Some(AccessibleRole::Button),
            "{label}"
        );
    }
    let cell = element(&popup, &day_label(&popup, "1"));
    assert_eq!(cell.accessible_role(), Some(AccessibleRole::Button));

    click(&popup, &day_label(&popup, "1"));
    for label in ["Copy BS", "Copy AD", "Close day detail"] {
        assert_eq!(
            element(&popup, label).accessible_role(),
            Some(AccessibleRole::Button),
            "{label}"
        );
    }
    assert_eq!(
        element(&popup, "Event list").accessible_role(),
        Some(AccessibleRole::List)
    );
}

/// Switching language at runtime; Slint must be able to go back to the source language.
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

#[test]
fn converter_converts_typed_input_both_ways() {
    let (popup, _state, _) = app(date(2081, 1, 1));
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
    let (popup, state, _) = app(date(2081, 1, 1));
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

    elapse(350);
    let (saved, warnings) = crate::platform::settings_store::load(&path);
    assert!(warnings.is_empty(), "{warnings:?}");
    assert!(saved.clock_24h, "saved after 300 ms");
}

// WR-03 AC6: the three zoom levels.

#[test]
fn wr03_ac6_the_title_zooms_to_the_year_view_and_the_decade_view() {
    let (popup, state, _) = app(date(2083, 7, 5));
    assert_eq!(popup.get_zoom_mode(), 0);
    assert_eq!(nav_title(&popup), "Kartik 2083 · Oct/Nov 2026");

    click(&popup, "Change view");
    assert_eq!(popup.get_zoom_mode(), 1, "the year view");
    assert_eq!(nav_title(&popup), "2083");
    let labels = zoom_labels(&popup);
    assert_eq!(labels.len(), 12, "a 4 by 3 grid of the year's months");
    assert_eq!(labels[0], "Baishakh 2083");
    assert_eq!(labels[11], "Chaitra 2083");
    let cells = popup.get_zoom_cells();
    let today_cell = cells.row_data(6).expect("Kartik");
    assert!(today_cell.is_today, "today's month is marked");
    assert!(today_cell.is_viewed);
    assert!(!cells.row_data(5).expect("Ashoj").is_today);

    click(&popup, "Change view");
    assert_eq!(popup.get_zoom_mode(), 2, "the decade view");
    assert_eq!(nav_title(&popup), "2080–2089");
    let labels = zoom_labels(&popup);
    assert_eq!(labels.len(), 12);
    assert_eq!(labels[0], "2079", "one year before the decade");
    assert_eq!(labels[11], "2090", "one year after the decade");
    assert!(popup.get_zoom_cells().row_data(4).expect("2083").is_today);
    assert_eq!(
        element(&popup, "Change view").accessible_enabled(),
        Some(false),
        "the decade title does nothing"
    );
    assert_eq!(state.borrow().zoom.zoom(), Zoom::Decade);
}

#[test]
fn wr03_ac6_a_decade_year_opens_its_year_view_and_a_month_opens_the_month_view() {
    let (popup, state, _) = app(date(2083, 7, 5));
    click(&popup, "Change view");
    click(&popup, "Change view");

    click(&popup, "2085");
    assert_eq!(popup.get_zoom_mode(), 1);
    assert_eq!(nav_title(&popup), "2085");
    assert_eq!(
        state.borrow().zoom.viewed(),
        (2085, Month::Kartik),
        "the month stays"
    );

    click(&popup, "Baishakh 2085");
    assert_eq!(popup.get_zoom_mode(), 0);
    assert!(popup.get_month_title().starts_with("Baishakh 2085"));
    assert!(popup.get_today_shown(), "away from today's month");
}

#[test]
fn wr03_ac6_previous_and_next_follow_the_level() {
    let (popup, state, _) = app(date(2083, 7, 5));
    click(&popup, "Change view");
    assert!(exists(&popup, "Previous year"), "named for the year view");
    assert!(!exists(&popup, "Previous month"));
    click(&popup, "Next year");
    assert_eq!(nav_title(&popup), "2084");
    click(&popup, "Previous year");
    click(&popup, "Previous year");
    assert_eq!(nav_title(&popup), "2082");

    click(&popup, "Change view");
    assert!(exists(&popup, "Next decade"), "named for the decade view");
    click(&popup, "Next decade");
    assert_eq!(nav_title(&popup), "2090–2099");
    click(&popup, "Previous decade");
    click(&popup, "Previous decade");
    assert_eq!(nav_title(&popup), "2070–2079");
    assert_eq!(
        state.borrow().zoom.year(),
        2082,
        "paging the decade does not change the year"
    );
}

#[test]
fn wr03_ac6_the_wheel_pages_by_the_level() {
    let (popup, _state, _) = app(date(2083, 7, 5));
    click(&popup, "Change view");
    let cell = element(&popup, "Kartik 2083");
    cell.scroll(0.0, 1.0);
    assert_eq!(nav_title(&popup), "2084", "one notch is one year");
    click(&popup, "Change view");
    let cell = element(&popup, "2084");
    cell.scroll(0.0, 1.0);
    assert_eq!(nav_title(&popup), "2090–2099", "one notch is ten years");
}

#[test]
fn wr03_ac6_the_edges_disable_the_control_at_every_level() {
    // 1901: nothing before it.
    let (popup, _state, _) = app(date(1901, 1, 1));
    assert_eq!(
        element(&popup, "Previous month").accessible_enabled(),
        Some(false)
    );
    click(&popup, "Change view");
    assert_eq!(
        element(&popup, "Previous year").accessible_enabled(),
        Some(false)
    );
    assert_eq!(
        element(&popup, "Next year").accessible_enabled(),
        Some(true)
    );
    click(&popup, "Previous year");
    assert_eq!(nav_title(&popup), "1901", "a disabled control does nothing");
    click(&popup, "Change view");
    assert_eq!(nav_title(&popup), "1900–1909");
    assert_eq!(
        element(&popup, "Previous decade").accessible_enabled(),
        Some(false)
    );
    let cells = popup.get_zoom_cells();
    assert!(
        !cells.row_data(0).expect("1899").enabled,
        "1899 is no choice"
    );
    assert!(
        !cells.row_data(1).expect("1900").enabled,
        "1900 is no choice"
    );
    assert_eq!(cells.row_data(0).expect("1899").label.as_str(), "");
    assert!(cells.row_data(2).expect("1901").enabled);
    assert!(!exists(&popup, "1899"));
    assert!(!exists(&popup, "1900"));
    assert!(exists(&popup, "1901"));

    // 2199: nothing after it.
    let (popup, _state, _) = app(date(2199, 12, 30));
    assert_eq!(
        element(&popup, "Next month").accessible_enabled(),
        Some(false)
    );
    click(&popup, "Change view");
    assert_eq!(
        element(&popup, "Next year").accessible_enabled(),
        Some(false)
    );
    click(&popup, "Change view");
    assert_eq!(nav_title(&popup), "2190–2199");
    assert_eq!(
        element(&popup, "Next decade").accessible_enabled(),
        Some(false)
    );
    let cells = popup.get_zoom_cells();
    assert!(
        !cells.row_data(11).expect("2200").enabled,
        "2200 is no choice"
    );
    assert!(!exists(&popup, "2200"));
    assert!(exists(&popup, "2199"));
}

#[test]
fn wr03_ac6_today_returns_to_todays_month_from_every_level() {
    let (popup, state, _) = app(date(2083, 7, 5));
    for clicks in 0..3 {
        show_month(&state, 2150, Month::Chaitra);
        for _ in 0..clicks {
            click(&popup, "Change view");
        }
        assert_eq!(popup.get_zoom_mode(), clicks);
        assert!(popup.get_today_shown(), "away from today at level {clicks}");
        click(&popup, "Today");
        assert_eq!(popup.get_zoom_mode(), 0, "level {clicks}");
        assert!(popup.get_month_title().starts_with("Kartik 2083"));
        assert!(!popup.get_today_shown());
    }

    // In today's year view the pill still shows: the view is not today's month.
    click(&popup, "Change view");
    assert!(popup.get_today_shown());
}

#[test]
fn wr02_ac2_a_reopen_shows_the_month_view_of_the_last_viewed_month() {
    let (popup, state, _) = app(date(2083, 7, 5));
    click(&popup, "Next month");
    click(&popup, "Change view");
    click(&popup, "Change view");
    assert_eq!(popup.get_zoom_mode(), 2);

    state.borrow_mut().hidden_on = crate::today_local();
    state.borrow_mut().zoom.reopened(crate::today_local());
    update_popup(&state);
    assert_eq!(
        popup.get_zoom_mode(),
        0,
        "a reopen starts in the month view"
    );
    assert_eq!(state.borrow().zoom.viewed(), (2083, Month::Mangsir));
}

// WR-10 AC2: the keys.

#[test]
fn wr10_ac2_the_grid_is_one_tab_stop_and_arrows_move_between_days() {
    let (popup, _state, _) = app(date(2083, 7, 5));
    tab_to(&popup, "grid");
    let cursor = |popup: &PopupWindow| {
        let index = usize::try_from(popup.get_cursor_index()).expect("a cursor");
        popup
            .get_cells()
            .row_data(index)
            .expect("cell")
            .bs_day
            .to_string()
    };
    assert_eq!(cursor(&popup), "5", "the cursor starts on today");

    press(&popup, slint::platform::Key::RightArrow);
    assert_eq!(cursor(&popup), "6");
    assert_eq!(focused(&popup), "grid", "the grid keeps the one stop");
    press(&popup, slint::platform::Key::DownArrow);
    assert_eq!(cursor(&popup), "13");
    press(&popup, slint::platform::Key::UpArrow);
    press(&popup, slint::platform::Key::LeftArrow);
    assert_eq!(cursor(&popup), "5");

    // Past the first day the cursor carries into the previous month.
    for _ in 0..5 {
        press(&popup, slint::platform::Key::LeftArrow);
    }
    assert!(
        popup.get_month_title().starts_with("Ashoj 2083"),
        "carried back"
    );
    assert_eq!(cursor(&popup), "31", "the last day of Ashoj");

    // Tab leaves the grid and Shift+Tab comes back to the same place.
    press(&popup, slint::platform::Key::Tab);
    assert_eq!(focused(&popup), "upcoming");
    press_shift_tab(&popup);
    assert_eq!(focused(&popup), "grid");
    assert_eq!(cursor(&popup), "31");
}

#[test]
fn wr10_ac2_enter_and_space_open_the_focused_days_detail() {
    let (popup, _state, _) = app(date(2083, 7, 5));
    tab_to(&popup, "grid");
    press(&popup, slint::platform::Key::RightArrow);
    press(&popup, slint::platform::Key::Return);
    assert!(popup.get_detail_visible());
    assert_eq!(popup.get_detail().bs_long.as_str(), "Kartik 6, 2083");

    press(&popup, slint::platform::Key::Escape);
    assert!(!popup.get_detail_visible());
    tab_to(&popup, "grid");
    press(&popup, slint::platform::Key::Space);
    assert!(popup.get_detail_visible());
    assert_eq!(
        popup.get_detail().bs_long.as_str(),
        "Kartik 6, 2083",
        "the cursor stayed on the same day"
    );
}

#[test]
fn wr10_ac2_arrows_and_enter_move_between_months_and_years_in_the_zoom_views() {
    let (popup, _state, _) = app(date(2083, 7, 5));
    click(&popup, "Change view");
    tab_to(&popup, "grid");
    assert_eq!(popup.get_cursor_index(), 6, "the cursor starts on Kartik");
    press(&popup, slint::platform::Key::RightArrow);
    press(&popup, slint::platform::Key::DownArrow);
    assert_eq!(popup.get_cursor_index(), 11, "Chaitra");
    press(&popup, slint::platform::Key::RightArrow);
    assert_eq!(nav_title(&popup), "2084", "past Chaitra is the next year");
    assert_eq!(popup.get_cursor_index(), 0);
    press(&popup, slint::platform::Key::Return);
    assert_eq!(popup.get_zoom_mode(), 0, "Enter picks the month");
    assert!(popup.get_month_title().starts_with("Baishakh 2084"));

    click(&popup, "Change view");
    click(&popup, "Change view");
    tab_to(&popup, "grid");
    press(&popup, slint::platform::Key::RightArrow);
    press(&popup, slint::platform::Key::Space);
    assert_eq!(popup.get_zoom_mode(), 1, "Space picks the year");
    assert_eq!(nav_title(&popup), "2085");
}

#[test]
fn wr10_ac2_page_up_and_page_down_page_by_the_level() {
    let (popup, _state, _) = app(date(2083, 7, 5));
    press(&popup, slint::platform::Key::PageDown);
    assert!(popup.get_month_title().starts_with("Mangsir 2083"));
    press(&popup, slint::platform::Key::PageUp);
    press(&popup, slint::platform::Key::PageUp);
    assert!(popup.get_month_title().starts_with("Ashoj 2083"));

    click(&popup, "Change view");
    press(&popup, slint::platform::Key::PageDown);
    assert_eq!(nav_title(&popup), "2084", "a year in the year view");
    click(&popup, "Change view");
    press(&popup, slint::platform::Key::PageDown);
    assert_eq!(
        nav_title(&popup),
        "2090–2099",
        "ten years in the decade view"
    );
    press(&popup, slint::platform::Key::PageUp);
    press(&popup, slint::platform::Key::PageUp);
    assert_eq!(nav_title(&popup), "2070–2079");
}

#[test]
fn wr10_ac2_home_and_t_go_to_today() {
    for key in ["Home", "t", "T"] {
        let (popup, state, _) = app(date(2083, 7, 5));
        show_month(&state, 2090, Month::Jestha);
        click(&popup, "Change view");
        assert_eq!(popup.get_zoom_mode(), 1);
        match key {
            "Home" => press(&popup, slint::platform::Key::Home),
            letter => type_text(&popup, letter),
        }
        assert_eq!(popup.get_zoom_mode(), 0, "{key} returns to the month view");
        assert!(
            popup.get_month_title().starts_with("Kartik 2083"),
            "{key} goes to today"
        );
    }
}

#[test]
fn wr10_ac2_c_opens_the_converter() {
    for key in ["c", "C"] {
        let (popup, _state, _) = app(date(2083, 7, 5));
        type_text(&popup, key);
        assert!(popup.get_converter_visible(), "{key}");
    }
}

#[test]
fn wr10_ac2_ctrl_comma_opens_settings() {
    let (popup, _state, _) = app(date(2083, 7, 5));
    press_ctrl(&popup, ",");
    assert!(popup.get_settings_visible());
    assert_eq!(
        popup.get_settings_data().version.as_str(),
        env!("CARGO_PKG_VERSION")
    );
}

#[test]
fn wr10_ac2_single_letter_keys_do_nothing_in_a_text_field_or_under_an_overlay() {
    // The Go to date field takes its own typing: p, c and t are letters there.
    let (popup, _state, _) = app(date(2083, 7, 5));
    press_ctrl(&popup, "g");
    elapse(5);
    assert_eq!(focused(&popup), "goto");
    type_text(&popup, "pct");
    assert_eq!(popup.get_goto_text().as_str(), "pct");
    assert!(!popup.get_pinned(), "p typed in the field did not pin");
    assert!(
        !popup.get_converter_visible(),
        "c typed in the field did not convert"
    );

    // Under an overlay the base is inert, so the letters and Ctrl+G do nothing.
    let (popup, _state, _) = app(date(2083, 7, 5));
    click(&popup, "Convert");
    popup.invoke_focus();
    type_text(&popup, "p");
    assert!(!popup.get_pinned());
    press_ctrl(&popup, "g");
    assert!(!popup.get_goto_visible());
}

#[test]
fn wr10_ac2_escape_closes_the_open_panel_then_steps_out_of_a_zoom_view_then_closes_the_popup() {
    let (popup, _state, _) = app(date(2083, 7, 5));
    click(&popup, "Change view");
    click(&popup, "Change view");
    assert_eq!(popup.get_zoom_mode(), 2);

    // A panel open over a zoom view closes first: the upcoming list, then the zoom steps.
    click(&popup, "Upcoming holidays");
    assert!(popup.get_upcoming_visible());
    press(&popup, slint::platform::Key::Escape);
    assert!(!popup.get_upcoming_visible(), "the panel closes first");
    assert_eq!(popup.get_zoom_mode(), 2, "the zoom view stays");

    press(&popup, slint::platform::Key::Escape);
    assert_eq!(popup.get_zoom_mode(), 1, "decade to year");
    press(&popup, slint::platform::Key::Escape);
    assert_eq!(popup.get_zoom_mode(), 0, "year to month");
    assert!(popup.window().is_visible());
    press(&popup, slint::platform::Key::Escape);
    assert!(!popup.window().is_visible(), "then the popup closes");
}

#[test]
fn wr10_ac2_every_panel_closes_on_escape_before_the_popup() {
    // Settings, the converter, the day detail and the Go to date field each take one Escape.
    let (popup, _state, _) = app(date(2083, 7, 5));
    popup.set_settings_visible(true);
    popup.invoke_focus();
    press(&popup, slint::platform::Key::Escape);
    assert!(!popup.get_settings_visible());
    assert!(popup.window().is_visible());

    click(&popup, "Convert");
    popup.invoke_focus();
    press(&popup, slint::platform::Key::Escape);
    assert!(!popup.get_converter_visible());
    assert!(popup.window().is_visible());

    press_ctrl(&popup, "g");
    assert!(popup.get_goto_visible());
    press(&popup, slint::platform::Key::Escape);
    assert!(!popup.get_goto_visible(), "the field closes on Escape");
    assert!(popup.window().is_visible());
}

// WR-14 AC1: Pin.

#[test]
fn wr14_ac1_pin_keeps_the_popup_open_on_focus_loss_until_it_is_unpinned() {
    let (popup, _state, _) = app(date(2083, 7, 5));
    assert!(!popup.get_pinned(), "the popup opens unpinned");
    assert!(exists(&popup, "Pin, not pinned"));
    click(&popup, "Pin, not pinned");
    assert!(popup.get_pinned());
    assert!(exists(&popup, "Pin, pinned"), "the name says it is pinned");
    assert!(!exists(&popup, "Pin, not pinned"));

    popup.invoke_window_deactivated();
    assert!(
        popup.window().is_visible(),
        "a pinned popup survives focus loss"
    );
    popup
        .window()
        .dispatch_event(slint::platform::WindowEvent::WindowActiveChanged(false));
    assert!(popup.window().is_visible());

    click(&popup, "Pin, pinned");
    assert!(!popup.get_pinned());
    popup.invoke_window_deactivated();
    assert!(
        !popup.window().is_visible(),
        "unpinned, focus loss hides it"
    );
}

#[test]
fn wr14_ac1_the_p_key_toggles_the_pin() {
    let (popup, _state, _) = app(date(2083, 7, 5));
    type_text(&popup, "p");
    assert!(popup.get_pinned());
    type_text(&popup, "P");
    assert!(!popup.get_pinned());
}

#[test]
fn wr14_ac1_escape_and_a_toggle_close_a_pinned_popup_and_it_reopens_unpinned() {
    let (popup, state, _) = app(date(2083, 7, 5));
    type_text(&popup, "p");
    press(&popup, slint::platform::Key::Escape);
    assert!(!popup.window().is_visible(), "Escape closes a pinned popup");
    assert!(!popup.get_pinned(), "closing unpins it");

    popup.show().expect("show");
    popup.invoke_focus();
    type_text(&popup, "p");
    assert!(popup.get_pinned());
    crate::hide_popup(&state);
    assert!(
        !popup.window().is_visible(),
        "a toggle closes a pinned popup"
    );
    assert!(!popup.get_pinned(), "the toggle unpins it too");
}

// WR-14 AC2: the day tooltip.

#[test]
fn wr14_ac2_a_day_shows_its_tooltip_after_500_ms_and_hides_on_leave() {
    let (popup, _state, _) = app(date(2081, 1, 1));
    let cell = element(&popup, &day_label(&popup, "1"));
    hover(&popup, &cell);
    elapse(499);
    assert!(!popup.get_tip_visible(), "not yet at 499 ms");
    elapse(2);
    assert!(popup.get_tip_visible(), "shown after 500 ms");

    let tip = popup.get_tip();
    assert_eq!(tip.bs_long.as_str(), "Baishakh 1, 2081");
    assert_eq!(tip.ad_long.as_str(), "April 13, 2024");
    assert_eq!(tip.weekday.as_str(), "Saturday");
    assert_eq!(tip.tithi.as_str(), "Panchami");
    assert_eq!(tip.holiday.as_str(), "New Year");
    assert_eq!(tip.events, 3);

    leave(&popup);
    assert!(
        !popup.get_tip_visible(),
        "hidden when the pointer leaves the day"
    );
}

#[test]
fn wr14_ac2_a_key_hides_the_tooltip() {
    let (popup, _state, _) = app(date(2081, 1, 1));
    let cell = element(&popup, &day_label(&popup, "1"));
    hover(&popup, &cell);
    elapse(600);
    assert!(popup.get_tip_visible());
    press(&popup, slint::platform::Key::Shift);
    assert!(!popup.get_tip_visible(), "a key press hides it");

    // It stays hidden while the pointer rests on the same day, and a new visit shows it again.
    elapse(1_000);
    assert!(!popup.get_tip_visible());
    leave(&popup);
    let cell = element(&popup, &day_label(&popup, "2"));
    hover(&popup, &cell);
    elapse(600);
    assert!(popup.get_tip_visible(), "another day shows its own tooltip");
    assert_eq!(popup.get_tip().bs_long.as_str(), "Baishakh 2, 2081");
}

#[test]
fn wr14_ac2_an_ordinary_day_has_no_holiday_or_event_line() {
    let (popup, _state, _) = app(date(2081, 1, 1));
    let cell = element(&popup, &day_label(&popup, "2"));
    hover(&popup, &cell);
    elapse(600);
    let tip = popup.get_tip();
    assert_eq!(tip.holiday.as_str(), "", "no holiday line");
    assert!(
        !tip.tithi.is_empty(),
        "the tithi shows where there is patro data"
    );
}

#[test]
fn wr14_ac2_a_day_outside_the_patro_data_leaves_out_the_tithi_holiday_and_events() {
    let (popup, _state, _) = app(date(2090, 1, 1));
    let cell = element(&popup, &day_label(&popup, "5"));
    hover(&popup, &cell);
    elapse(600);
    let tip = popup.get_tip();
    assert!(!tip.bs_long.is_empty());
    assert!(!tip.weekday.is_empty());
    assert_eq!(tip.tithi.as_str(), "");
    assert_eq!(tip.holiday.as_str(), "");
    assert_eq!(tip.events, 0);
}

#[test]
fn wr14_ac2_the_tooltip_never_covers_the_hovered_day_and_stays_inside_the_popup() {
    let (popup, _state, _) = app(date(2081, 1, 1));
    // Days in the first row, the last row, the left column and the right column.
    for day in ["1", "2", "7", "14", "21", "28", "31"] {
        let label = day_label(&popup, day);
        let cell = element(&popup, &label);
        let (cell_pos, cell_size) = (cell.absolute_position(), cell.size());
        hover(&popup, &cell);
        elapse(600);
        assert!(popup.get_tip_visible(), "day {day}");
        let card = ElementHandle::find_by_element_id(&popup, "PopupWindow::tip-card")
            .next()
            .expect("the tooltip card");
        let (pos, size) = (card.absolute_position(), card.size());
        let overlaps = pos.x < cell_pos.x + cell_size.width
            && cell_pos.x < pos.x + size.width
            && pos.y < cell_pos.y + cell_size.height
            && cell_pos.y < pos.y + size.height;
        assert!(
            !overlaps,
            "day {day}: the card {pos:?} {size:?} covers the cell {cell_pos:?}"
        );
        assert!(
            pos.x >= 0.0
                && pos.y >= 0.0
                && pos.x + size.width <= 360.0
                && pos.y + size.height <= 520.0,
            "day {day}: the card {pos:?} {size:?} leaves the popup"
        );
        leave(&popup);
    }
}

#[test]
fn wr10_ac2_each_header_pills_tooltip_names_its_key() {
    let (popup, _state, _) = app(date(2083, 7, 5));
    for (label, tip) in [
        ("Convert", "Convert (C)"),
        ("Settings", "Settings (Ctrl+,)"),
        ("Pin, not pinned", "Pin (P)"),
    ] {
        hover(&popup, &element(&popup, label));
        elapse(600);
        assert_eq!(popup.get_pill_tip().as_str(), tip, "{label}");
        leave(&popup);
        assert_eq!(popup.get_pill_tip().as_str(), "", "{label} hides on leave");
    }
    click(&popup, "Pin, not pinned");
    leave(&popup);
    hover(&popup, &element(&popup, "Pin, pinned"));
    elapse(600);
    assert_eq!(popup.get_pill_tip().as_str(), "Unpin (P)");
}

// WR-14 AC3: Go to date.

/// Opens the field with Ctrl+G, puts `text` in it and presses Enter.
fn go_to(popup: &PopupWindow, text: &str) {
    press_ctrl(popup, "g");
    elapse(5);
    assert!(popup.get_goto_visible(), "Ctrl+G opens the field");
    element(popup, "Go to date").set_accessible_value(text);
    press(popup, slint::platform::Key::Return);
}

#[test]
fn wr14_ac3_ctrl_g_opens_one_field_in_the_header_with_focus() {
    let (popup, _state, _) = app(date(2083, 7, 5));
    assert!(!popup.get_goto_visible());
    press_ctrl(&popup, "g");
    assert!(popup.get_goto_visible());
    elapse(5);
    assert_eq!(
        focused(&popup),
        "goto",
        "typing goes straight into the field"
    );
    assert_eq!(
        element(&popup, "Go to date").accessible_role(),
        Some(AccessibleRole::TextInput)
    );
}

#[test]
fn wr14_ac3_clicking_the_header_date_opens_the_field() {
    let (popup, _state, _) = app(date(2083, 7, 5));
    click(&popup, "Go to date (Ctrl+G)");
    assert!(
        popup.get_goto_visible(),
        "the date is the mouse way to the field"
    );
    // No key event runs the change trackers here, so the first tick starts the focus timer and
    // the second runs it.
    elapse(5);
    elapse(5);
    assert_eq!(focused(&popup), "goto");
}

#[test]
fn wr14_ac3_the_header_date_names_its_key_after_the_pointer_rests() {
    let (popup, _state, _) = app(date(2083, 7, 5));
    hover(&popup, &element(&popup, "Go to date (Ctrl+G)"));
    elapse(400);
    assert_eq!(popup.get_pill_tip().as_str(), "", "not before 500 ms");
    elapse(200);
    assert_eq!(popup.get_pill_tip().as_str(), "Go to date (Ctrl+G)");
    leave(&popup);
    assert_eq!(popup.get_pill_tip().as_str(), "", "leaving hides it");
}

#[test]
fn wr14_ac3_an_ad_marker_opens_the_bs_date_the_ad_date_falls_on() {
    let (popup, _state, _) = app(date(2083, 7, 5));
    go_to(&popup, "2024-04-13 AD");
    assert_eq!(popup.get_detail().bs_long.as_str(), "Baishakh 1, 2081");
}

#[test]
fn wr14_ac3_a_bs_date_opens_its_month_and_detail() {
    let (popup, state, _) = app(date(2083, 7, 5));
    go_to(&popup, "2081/01/05");
    assert!(!popup.get_goto_visible(), "the field closes on success");
    assert!(popup.get_month_title().starts_with("Baishakh 2081"));
    assert!(popup.get_detail_visible());
    assert_eq!(popup.get_detail().bs_long.as_str(), "Baishakh 5, 2081");
    assert_eq!(state.borrow().zoom.zoom(), Zoom::Month);
}

#[test]
fn wr14_ac3_a_bs_date_with_a_month_name_and_devanagari_digits_opens() {
    let (popup, _state, _) = app(date(2083, 7, 5));
    go_to(&popup, "5 Baishakh 2081");
    assert_eq!(popup.get_detail().bs_long.as_str(), "Baishakh 5, 2081");

    popup.invoke_focus();
    popup
        .window()
        .dispatch_event(slint::platform::WindowEvent::KeyPressed {
            text: slint::platform::Key::Escape.into(),
        });
    go_to(&popup, "२०८२/०२/१५");
    assert_eq!(popup.get_detail().bs_long.as_str(), "Jestha 15, 2082");
}

#[test]
fn wr14_ac3_an_ad_date_opens_the_bs_day_it_falls_on() {
    let (popup, _state, _) = app(date(2083, 7, 5));
    // 1850 is not a BS year, so the text is read as AD.
    let expected = NepaliDate::from_gregorian(1850, 5, 5).expect("AD 1850-05-05");
    go_to(&popup, "1850-05-05");
    assert_eq!(
        popup.get_detail().ad_long.as_str(),
        "May 5, 1850",
        "the opened day is the AD date typed"
    );
    assert!(popup.get_month_title().starts_with(&format!(
        "{} {}",
        expected.month().name(Lang::En),
        expected.year()
    )));
}

#[test]
fn wr14_ac3_the_ambiguous_year_rule_reads_a_valid_bs_date_as_bs() {
    let (popup, _state, _) = app(date(2083, 7, 5));
    // 2026 is a BS year and an AD year; a valid BS date wins.
    go_to(&popup, "2026-10-22");
    assert_eq!(popup.get_detail().bs_long.as_str(), "Magh 22, 2026");
}

#[test]
fn wr14_ac3_errors_follow_the_converter_and_the_field_stays_open() {
    let (popup, _state, _) = app(date(2083, 7, 5));
    press_ctrl(&popup, "g");

    let field = element(&popup, "Go to date");
    field.set_accessible_value("20");
    assert_eq!(popup.get_goto_error().as_str(), "", "too short to judge");
    element(&popup, "Go to date").set_accessible_value("2083/13/45");
    assert_eq!(popup.get_goto_error().as_str(), "Invalid date");
    element(&popup, "Go to date").set_accessible_value("1700-01-01");
    assert_eq!(
        popup.get_goto_error().as_str(),
        "Supported range: 1901–2199 BS (1844–2143 AD)"
    );
    press(&popup, slint::platform::Key::Return);
    assert!(
        popup.get_goto_visible(),
        "Enter on a bad date keeps the field"
    );
    assert!(!popup.get_detail_visible());

    element(&popup, "Go to date").set_accessible_value("2083/07/05");
    assert_eq!(
        popup.get_goto_error().as_str(),
        "",
        "a good date clears the error"
    );

    press(&popup, slint::platform::Key::Escape);
    assert!(!popup.get_goto_visible(), "Escape closes the field");
    assert_eq!(popup.get_goto_error().as_str(), "");
    assert!(popup.window().is_visible(), "and not the popup");
}

#[test]
fn wr14_ac3_the_edges_of_the_range_open() {
    let (popup, _state, _) = app(date(2083, 7, 5));
    go_to(&popup, "1901/01/01");
    assert_eq!(popup.get_detail().bs_long.as_str(), "Baishakh 1, 1901");
    assert_eq!(
        popup.get_cursor_index(),
        i32::from(date(1901, 1, 1).weekday().index_from_sunday())
    );
}

// WR-14 AC4: upcoming holidays.

#[test]
fn wr14_ac4_the_holiday_line_expands_to_the_next_five_holidays() {
    let (popup, _state, _) = app(date(2081, 1, 1));
    assert!(!popup.get_upcoming_visible());
    let line = element(&popup, "Upcoming holidays");
    assert_eq!(line.accessible_expandable(), Some(true));
    assert_eq!(line.accessible_expanded(), Some(false));

    click(&popup, "Upcoming holidays");
    assert!(popup.get_upcoming_visible());
    assert_eq!(
        element(&popup, "Upcoming holidays").accessible_expanded(),
        Some(true)
    );
    let upcoming = popup.get_upcoming();
    assert_eq!(upcoming.row_count(), 5, "five holidays");
    let first = upcoming.row_data(0).expect("first");
    assert_eq!(first.name.as_str(), "New Year");
    assert_eq!(first.date.as_str(), "Baishakh 1, 2081");
    assert_eq!(first.days, 0);
    let second = upcoming.row_data(1).expect("second");
    assert_eq!(second.days, 4);
    assert!(exists(&popup, "New Year, Baishakh 1, 2081, today"));
    assert!(exists(
        &popup,
        &format!("{}, {}, in 4 days", second.name, second.date)
    ));

    click(&popup, "Upcoming holidays");
    assert!(!popup.get_upcoming_visible(), "a second click collapses it");
}

#[test]
fn wr14_ac4_clicking_a_holiday_opens_its_month_and_detail() {
    let (popup, _state, _) = app(date(2081, 1, 1));
    click(&popup, "Upcoming holidays");
    let target = popup.get_upcoming().row_data(4).expect("fifth");
    let label = format!("{}, {}, in {} days", target.name, target.date, target.days);
    click(&popup, &label);
    assert!(!popup.get_upcoming_visible(), "the list collapses");
    assert!(popup.get_detail_visible());
    assert_eq!(popup.get_detail().bs_long.as_str(), target.date.as_str());
    assert_eq!(popup.get_zoom_mode(), 0);
}

#[test]
fn wr14_ac4_escape_collapses_the_list_before_anything_else() {
    let (popup, _state, _) = app(date(2081, 1, 1));
    click(&popup, "Upcoming holidays");
    popup.invoke_focus();
    press(&popup, slint::platform::Key::Escape);
    assert!(!popup.get_upcoming_visible());
    assert!(popup.window().is_visible(), "the popup stays");
}

#[test]
fn wr14_ac4_the_line_and_its_rows_are_keyboard_reachable() {
    let (popup, _state, _) = app(date(2081, 1, 1));
    tab_to(&popup, "upcoming");
    press(&popup, slint::platform::Key::Return);
    assert!(popup.get_upcoming_visible(), "Enter expands");
    press(&popup, slint::platform::Key::Tab);
    assert_eq!(focused(&popup), "upcoming 1", "the rows follow the line");
    press(&popup, slint::platform::Key::Tab);
    assert_eq!(focused(&popup), "upcoming 2");
    press(&popup, slint::platform::Key::Return);
    assert!(popup.get_detail_visible(), "Enter on a row opens its day");
    assert!(!popup.get_upcoming_visible());
}

#[test]
fn wr14_ac4_nothing_shows_outside_the_patro_data() {
    let (popup, _state, _) = app(date(2090, 1, 1));
    assert!(
        !exists(&popup, "Upcoming holidays"),
        "no holiday within the data, so no line to expand"
    );
    assert_eq!(popup.get_upcoming().row_count(), 0);
}

// WR-10 AC2 and W7: the focus order, stop by stop.

#[test]
fn tab_and_shift_tab_follow_the_focus_order() {
    let (popup, _state, _) = app(date(2081, 1, 1));
    let order = base_order(false, true);
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
    // The cycle wraps from the last stop to the first.
    press(&popup, slint::platform::Key::Tab);
    assert_eq!(focused(&popup), "convert", "Tab wraps");

    // Shift+Tab retraces the order from there.
    let mut reverse = Vec::new();
    for _ in 0..order.len() {
        press_shift_tab(&popup);
        reverse.push(focused(&popup));
    }
    let mut expected: Vec<&str> = order.iter().rev().copied().collect();
    expected.rotate_left(0);
    assert_eq!(reverse, expected, "Shift+Tab order");
}

#[test]
fn the_today_pill_is_a_stop_only_away_from_todays_month() {
    let (popup, state, _) = app(date(2081, 1, 1));
    show_month(&state, 2081, Month::Jestha);
    let order = base_order(true, true);
    let mut forward = Vec::new();
    for _ in 0..order.len() {
        press(&popup, slint::platform::Key::Tab);
        forward.push(focused(&popup));
    }
    assert_eq!(forward, order, "Today sits between Next and the grid");

    // The year and decade views keep the pill and the grid as one stop each.
    click(&popup, "Change view");
    popup.invoke_focus();
    let mut year = Vec::new();
    for _ in 0..order.len() {
        press(&popup, slint::platform::Key::Tab);
        year.push(focused(&popup));
    }
    assert_eq!(year, order, "the year view has the same stops");
}

#[test]
fn disabled_and_hidden_controls_are_not_focus_stops() {
    // 1901 Baishakh: Previous is disabled, and there is no patro data for a holiday line.
    let (popup, _state, _) = app(date(1901, 1, 1));
    let mut order = Vec::new();
    for _ in 0..5 {
        press(&popup, slint::platform::Key::Tab);
        order.push(focused(&popup));
    }
    assert_eq!(
        order,
        ["convert", "settings", "pin", "title", "next"],
        "1901 skips Previous"
    );
    press(&popup, slint::platform::Key::Tab);
    assert_eq!(focused(&popup), "grid");
    press(&popup, slint::platform::Key::Tab);
    assert_eq!(
        focused(&popup),
        "convert",
        "the disabled holiday line is not a stop"
    );

    // 2199 Chaitra: Next is disabled.
    let (popup, _state, _) = app(date(2199, 12, 30));
    let mut order = Vec::new();
    for _ in 0..5 {
        press(&popup, slint::platform::Key::Tab);
        order.push(focused(&popup));
    }
    assert_eq!(
        order,
        ["convert", "settings", "pin", "previous", "title"],
        "2199 skips Next"
    );

    // The decade view's title does nothing, so it is not a stop either, and zooming there with
    // the keyboard moves focus on to the grid.
    let (popup, _state, _) = app(date(2083, 7, 5));
    tab_to(&popup, "title");
    press(&popup, slint::platform::Key::Return);
    assert_eq!(popup.get_zoom_mode(), 1);
    assert_eq!(
        focused(&popup),
        "title",
        "the year view keeps the title focused"
    );
    press(&popup, slint::platform::Key::Return);
    assert_eq!(popup.get_zoom_mode(), 2);
    assert_eq!(
        focused(&popup),
        "grid",
        "the disabled title gives focus to the grid"
    );
    popup.invoke_focus();
    let mut order = Vec::new();
    for _ in 0..4 {
        press(&popup, slint::platform::Key::Tab);
        order.push(focused(&popup));
    }
    assert_eq!(order, ["convert", "settings", "pin", "previous"]);
    press(&popup, slint::platform::Key::Tab);
    assert_eq!(focused(&popup), "next", "the title is skipped");

    // A closed overlay is not in the tree at all, and neither is the closed field and list.
    for label in [
        "Close converter",
        "Close settings",
        "Go to date",
        "Close day detail",
    ] {
        assert!(!exists(&popup, label), "{label} is absent while closed");
    }
}

#[test]
fn overlays_take_and_return_focus() {
    let (popup, state, _) = app(date(2081, 1, 1));

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
    assert_eq!(focused(&popup), "settings", "the base order continues");

    // Settings is opened from the header, the tray menu or Ctrl+Comma; the test sets the flag and
    // refreshes the model.
    popup.set_settings_visible(true);
    crate::refresh_settings(&popup, &state);
    popup.invoke_focus();
    // The bar's two controls are in the cycle only where there is a bar (Windows).
    let mut order = vec![
        "settings close",
        "settings language",
        "settings theme",
        "settings 24-hour",
        "settings seconds",
        "settings filter",
        "settings no extra clock",
        "settings launch",
    ];
    if cfg!(windows) {
        order.extend(["settings bar", "settings reset position"]);
    }
    // Tab walks the order and wraps to the first stop; Shift+Tab walks it back from there.
    for expected in order.iter().copied().chain(["settings close"]) {
        press(&popup, slint::platform::Key::Tab);
        assert_eq!(focused(&popup), expected, "settings Tab order");
    }
    for expected in order.iter().rev() {
        press_shift_tab(&popup);
        assert_eq!(focused(&popup), *expected, "settings Shift+Tab order");
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
fn the_settings_pill_opens_settings_and_its_panel_closes_back_to_convert() {
    let (popup, _state, _) = app(date(2081, 1, 1));
    click(&popup, "Settings");
    assert!(popup.get_settings_visible());
    press(&popup, slint::platform::Key::Escape);
    assert!(!popup.get_settings_visible());
    assert_eq!(focused(&popup), "convert");
}

#[test]
fn return_and_space_step_once_per_press() {
    let (popup, _state, _) = app(date(2081, 1, 1));

    // Month: Return and Space on Previous move one month, and the key release adds nothing.
    tab_to(&popup, "previous");
    press(&popup, slint::platform::Key::Return);
    assert!(
        popup.get_month_title().starts_with("Chaitra 2080"),
        "Return moves one month back: {}",
        popup.get_month_title()
    );
    assert_eq!(focused(&popup), "previous", "the pill keeps focus");
    press(&popup, slint::platform::Key::Space);
    assert!(
        popup.get_month_title().starts_with("Falgun 2080"),
        "Space moves one month back: {}",
        popup.get_month_title()
    );
    popup
        .window()
        .dispatch_event(slint::platform::WindowEvent::KeyReleased {
            text: slint::platform::Key::Space.into(),
        });
    assert!(
        popup.get_month_title().starts_with("Falgun 2080"),
        "the release adds nothing"
    );

    // Year: the same keys on Next.
    click(&popup, "Change view");
    popup.invoke_focus();
    tab_to(&popup, "next");
    press(&popup, slint::platform::Key::Return);
    assert_eq!(nav_title(&popup), "2081", "Return adds one year");
    press(&popup, slint::platform::Key::Space);
    assert_eq!(nav_title(&popup), "2082", "Space adds one year");
}

#[test]
fn return_and_space_activate_the_title_and_the_header_pills() {
    let (popup, _state, _) = app(date(2081, 1, 1));
    tab_to(&popup, "title");
    press(&popup, slint::platform::Key::Return);
    assert_eq!(popup.get_zoom_mode(), 1, "Return zooms out");
    press(&popup, slint::platform::Key::Space);
    assert_eq!(popup.get_zoom_mode(), 2, "Space zooms out again");

    let (popup, _state, _) = app(date(2081, 1, 1));
    tab_to(&popup, "pin");
    press(&popup, slint::platform::Key::Space);
    assert!(popup.get_pinned());
    press(&popup, slint::platform::Key::Return);
    assert!(!popup.get_pinned());
}

#[test]
fn return_and_space_skip_disabled_edges() {
    // 1901: Previous is skipped, so Tab from Next's side never lands on it.
    let (popup, _state, _) = app(date(1901, 1, 1));
    tab_to(&popup, "next");
    press(&popup, slint::platform::Key::Return);
    assert!(popup.get_month_title().starts_with("Jestha 1901"));
    assert_eq!(
        element(&popup, "Previous month").accessible_enabled(),
        Some(true),
        "away from the edge it works again"
    );

    // 2199: Next is skipped.
    let (popup, _state, _) = app(date(2199, 12, 30));
    tab_to(&popup, "previous");
    press(&popup, slint::platform::Key::Space);
    assert!(popup.get_month_title().starts_with("Falgun 2199"));
}

#[test]
fn window_deactivation_hides_the_popup_while_a_control_has_focus() {
    let (popup, _state, _) = app(date(2081, 1, 1));
    tab_to(&popup, "next");
    assert_eq!(focused(&popup), "next");

    // The winit filter calls this when Focused(false) arrives. A FocusScope focus-lost would
    // reach only the focused pill, not the popup root.
    popup.invoke_window_deactivated();
    assert!(
        !popup.window().is_visible(),
        "the popup hides when the window loses activation"
    );
}

#[test]
fn a_toggle_hides_an_open_popup_without_panicking() {
    // The bar never takes focus, so a click on it while the popup is open reaches
    // `hide_popup` instead of the focus-loss path. The beta panicked here with "RefCell already
    // borrowed" on every such click.
    let (popup, state, _) = app(date(2081, 1, 1));
    let label = day_label(&popup, "5");
    click(&popup, &label);
    assert!(state.borrow().detail.is_some(), "the detail is open");

    crate::hide_popup(&state);
    assert!(!popup.window().is_visible(), "the toggle hides the popup");
    assert!(
        state.borrow().detail.is_none(),
        "the hide closes the detail"
    );
}

#[test]
fn window_active_changed_hides_the_popup_when_the_root_has_focus() {
    let (popup, _state, _) = app(date(2081, 1, 1));
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
fn wr02_ac5_a_headless_window_keeps_the_square_hairline_frame() {
    // No OS window exists in the testing backend, so nothing rounds it: the markup default is
    // the square frame with its hairline.
    let (popup, _state, _) = app(date(2081, 1, 1));
    assert_eq!(popup.get_frame_style(), crate::FrameStyle::Square);
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
    // `--theme` reached the eager popup; the lazy one must still apply it.
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

// The bar's events and the settings that drive it, against a fake `Bar` port: no window, no
// registry and no real clipboard.

/// Records every call the app makes on the bar.
#[derive(Default)]
struct FakeBar {
    lines: RefCell<Vec<BarLines>>,
    shown: RefCell<Vec<bool>>,
    locked: RefCell<Vec<bool>>,
    positions: RefCell<Vec<Saved>>,
}

impl Bar for FakeBar {
    fn set_lines(&self, lines: &BarLines) {
        self.lines.borrow_mut().push(lines.clone());
    }

    fn set_shown(&self, shown: bool) {
        self.shown.borrow_mut().push(shown);
    }

    fn set_locked(&self, locked: bool) {
        self.locked.borrow_mut().push(locked);
    }

    fn set_position(&self, saved: &Saved) {
        self.positions.borrow_mut().push(saved.clone());
    }

    fn set_menu_labels(&self, _labels: &MenuLabels) {}
}

/// An app with a fake bar, its settings saved to a temporary file, and the settings panel wired.
struct BarApp {
    popup: PopupWindow,
    state: Rc<RefCell<AppState>>,
    bar: Rc<FakeBar>,
    slot: crate::PopupSlot,
    clipboard: Rc<FakeClipboard>,
    settings_path: std::path::PathBuf,
    _dir: tempfile::TempDir,
    _save_timer: Rc<slint::Timer>,
}

fn bar_app() -> BarApp {
    let (popup, state, clipboard) = app(NepaliDate::new(2081, 1, 1).expect("date"));
    let bar = Rc::new(FakeBar::default());
    state.borrow_mut().bar = Some(bar.clone());
    let dir = tempfile::tempdir().expect("tempdir");
    let settings_path = dir.path().join("settings.toml");
    state.borrow_mut().settings_path = Some(settings_path.clone());
    let save_timer = Rc::new(slint::Timer::default());
    wire_settings(&popup, &state, &save_timer);
    let slot = Rc::new(RefCell::new(Some(popup.clone_strong())));
    BarApp {
        popup,
        state,
        bar,
        slot,
        clipboard,
        settings_path,
        _dir: dir,
        _save_timer: save_timer,
    }
}

/// A bar at the bottom of a 1080p screen, centred in a 48 px taskbar.
fn bar_anchor() -> Anchor {
    Anchor {
        bar: Rect::new(500, 1037, 720, 1075),
        work: Rect::new(0, 0, 1920, 1032),
        dpi: 96,
    }
}

fn bar_event(app: &BarApp, event: BarEvent) {
    crate::handle_bar_event(event, &app.state, &app.slot);
}

fn bar_menu(app: &BarApp, item: MenuItem) {
    bar_event(
        app,
        BarEvent::Menu {
            item,
            anchor: bar_anchor(),
        },
    );
}

/// Waits out the 300 ms save debounce, as the event loop would.
fn let_the_save_happen() {
    i_slint_backend_testing::mock_elapsed_time(std::time::Duration::from_millis(350));
    slint::platform::update_timers_and_animations();
}

#[test]
fn wr08_ac6_a_click_on_the_bar_toggles_the_popup() {
    let app = bar_app();
    assert!(app.popup.window().is_visible());
    bar_event(&app, BarEvent::Toggle(bar_anchor()));
    assert!(
        !app.popup.window().is_visible(),
        "a click closes an open popup"
    );
    bar_event(&app, BarEvent::Toggle(bar_anchor()));
    assert!(app.popup.window().is_visible(), "and opens a closed one");
}

#[test]
fn wr08_ac6_a_click_that_deactivated_the_popup_closes_it_and_does_not_reopen_it() {
    // The bar claims the foreground before it sends the toggle. If the popup's deactivation is
    // handled first, the popup has just hidden itself, and the toggle must not reopen it.
    let app = bar_app();
    app.popup.invoke_window_deactivated();
    assert!(!app.popup.window().is_visible(), "the focus loss hid it");
    bar_event(&app, BarEvent::Toggle(bar_anchor()));
    assert!(
        !app.popup.window().is_visible(),
        "the click meant close, so the popup stays closed"
    );

    // Past the guard, a click opens it again.
    app.state.borrow_mut().popup_hidden_at = Some(
        std::time::Instant::now()
            .checked_sub(std::time::Duration::from_secs(1))
            .expect("the clock is past its first second"),
    );
    bar_event(&app, BarEvent::Toggle(bar_anchor()));
    assert!(app.popup.window().is_visible());
}

#[test]
fn wr08_ac6_a_double_click_opens_the_popup_and_never_closes_it() {
    let app = bar_app();
    // The popup was closed: the first click opens it and the second is ignored as a toggle.
    app.popup.hide().expect("hide");
    bar_event(&app, BarEvent::Toggle(bar_anchor()));
    bar_event(&app, BarEvent::Open(bar_anchor()));
    assert!(
        app.popup.window().is_visible(),
        "closed, then double-clicked"
    );

    // The popup was open: the first click closes it and the double-click opens it again.
    bar_event(&app, BarEvent::Toggle(bar_anchor()));
    assert!(!app.popup.window().is_visible());
    bar_event(&app, BarEvent::Open(bar_anchor()));
    assert!(app.popup.window().is_visible(), "open, then double-clicked");

    // Opening an open popup leaves it open, and the menu's Open item behaves the same.
    bar_event(&app, BarEvent::Open(bar_anchor()));
    bar_menu(&app, MenuItem::Open);
    assert!(
        app.popup.window().is_visible(),
        "Open never closes the popup"
    );
}

#[test]
fn wr08_ac6_the_menu_copies_today_in_both_calendars() {
    let app = bar_app();
    bar_menu(&app, MenuItem::CopyBs);
    bar_menu(&app, MenuItem::CopyAd);
    assert_eq!(
        *app.clipboard.texts.borrow(),
        vec!["2081/01/01".to_owned(), "2024-04-13".to_owned()]
    );
}

#[test]
fn wr08_ac6_customize_bar_and_settings_open_settings_at_the_bar() {
    let app = bar_app();
    for item in [MenuItem::Customize, MenuItem::Settings] {
        app.popup.hide().expect("hide");
        app.popup.set_settings_visible(false);
        bar_menu(&app, item);
        assert!(app.popup.window().is_visible(), "{item:?} shows the popup");
        assert!(app.popup.get_settings_visible(), "{item:?} opens Settings");
    }
}

#[test]
fn wr08_ac6_lock_position_is_a_check_item_that_saves() {
    let app = bar_app();
    assert!(!app.state.borrow().settings.bar_locked);
    bar_menu(&app, MenuItem::Lock);
    assert!(app.state.borrow().settings.bar_locked);
    bar_menu(&app, MenuItem::Lock);
    assert!(!app.state.borrow().settings.bar_locked);
    assert_eq!(*app.bar.locked.borrow(), vec![true, false]);

    bar_menu(&app, MenuItem::Lock);
    let_the_save_happen();
    let (saved, warnings) = crate::platform::settings_store::load(&app.settings_path);
    assert!(warnings.is_empty(), "{warnings:?}");
    assert!(saved.bar_locked, "the lock is saved");
}

#[test]
fn wr08_ac6_hide_bar_sets_bar_off_and_the_tray_item_turns_it_back_on() {
    let app = bar_app();
    assert!(app.state.borrow().settings.bar);
    bar_menu(&app, MenuItem::Hide);
    assert!(!app.state.borrow().settings.bar);
    assert_eq!(*app.bar.shown.borrow(), vec![false]);

    // WR-01 AC5: the tray menu's Show bar item.
    crate::handle_tray_event(
        crate::platform::tray::TrayEvent::ShowBar,
        &app.state,
        &app.slot,
    );
    assert!(app.state.borrow().settings.bar);
    assert_eq!(*app.bar.shown.borrow(), vec![false, true]);

    // Show bar toggles: with the bar shown, the same item hides it.
    crate::handle_tray_event(
        crate::platform::tray::TrayEvent::ShowBar,
        &app.state,
        &app.slot,
    );
    assert!(!app.state.borrow().settings.bar);

    let_the_save_happen();
    let (saved, _) = crate::platform::settings_store::load(&app.settings_path);
    assert!(!saved.bar, "Hide bar is saved");
}

#[test]
fn wr08_ac6_the_settings_switch_shows_and_hides_the_bar() {
    let app = bar_app();
    crate::refresh_settings(&app.popup, &app.state);
    app.popup.set_settings_visible(true);
    app.popup.invoke_settings_bar(false);
    assert!(!app.state.borrow().settings.bar);
    assert!(!app.popup.get_settings_data().bar, "the switch follows");
    app.popup.invoke_settings_bar(true);
    assert!(app.state.borrow().settings.bar);
    assert_eq!(*app.bar.shown.borrow(), vec![false, true]);

    // Hide bar from the bar's own menu moves the Settings switch too.
    bar_menu(&app, MenuItem::Hide);
    assert!(!app.popup.get_settings_data().bar);
}

#[test]
fn wr08_ac4_the_end_of_a_drag_is_saved_and_survives_a_restart() {
    let app = bar_app();
    let saved = Saved {
        monitor: r"\\.\DISPLAY2".to_owned(),
        x: 300,
        y: -12,
    };
    bar_event(&app, BarEvent::Moved(saved.clone()));
    assert_eq!(app.state.borrow().settings.bar_position(), saved);
    assert!(!app.settings_path.exists(), "not saved before the debounce");

    let_the_save_happen();
    let (loaded, warnings) = crate::platform::settings_store::load(&app.settings_path);
    assert!(warnings.is_empty(), "{warnings:?}");
    assert_eq!(
        loaded.bar_position(),
        saved,
        "after a restart the bar is where it was left"
    );
}

#[test]
fn wr08_ac5_reset_position_forgets_the_saved_place() {
    let app = bar_app();
    let saved = Saved {
        monitor: r"\\.\DISPLAY2".to_owned(),
        x: 300,
        y: 40,
    };
    bar_event(&app, BarEvent::Moved(saved));
    app.popup.invoke_settings_reset_bar();
    assert_eq!(app.state.borrow().settings.bar_position(), Saved::default());
    assert_eq!(
        app.bar.positions.borrow().last(),
        Some(&Saved::default()),
        "the bar goes back to its default position"
    );

    let_the_save_happen();
    let (loaded, _) = crate::platform::settings_store::load(&app.settings_path);
    assert_eq!(loaded.bar_position(), Saved::default());
}

#[test]
fn wr08_ac2_the_bar_follows_the_clock_settings_at_once() {
    let app = bar_app();
    app.popup.invoke_settings_clock(true);
    let last = app.bar.lines.borrow().last().cloned().expect("lines");
    assert!(
        !last.top.contains("AM") && !last.top.contains("PM"),
        "24-hour time on the bar: {}",
        last.top
    );
    app.popup.invoke_settings_clock(false);
    let last = app.bar.lines.borrow().last().cloned().expect("lines");
    assert!(
        last.top.contains("AM") || last.top.contains("PM"),
        "12-hour time on the bar: {}",
        last.top
    );
}

#[test]
fn the_longest_header_date_is_not_elided_by_the_pills() {
    // "Baishakh 31, 2081" is the longest English date. The header's date box once asked for no
    // width, so the Convert pill took the room and the date showed as "Baishakh ...".
    let (popup, _state, _) = app(date(2081, 1, 31));
    let button = element(&popup, "Go to date (Ctrl+G)");
    assert!(
        button.size().width >= 150.0,
        "the date has room for its text: {} px",
        button.size().width
    );
}
