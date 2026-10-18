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
use crate::{AppState, PopupWindow, update_popup, wire_popup};

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
    wire_popup(&popup, &state, &weak);
    update_popup(&state, &weak);
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
fn show_month(popup: &PopupWindow, state: &Rc<RefCell<AppState>>, year: u16, month: Month) {
    state.borrow_mut().viewed_month = (year, month);
    state.borrow_mut().detail = None;
    update_popup(state, &popup.as_weak());
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

    show_month(&popup, &state, 1901, Month::Baishakh);
    assert_eq!(
        element(&popup, "Previous month").accessible_enabled(),
        Some(false)
    );
    click(&popup, "Previous month");
    assert_eq!(
        popup.get_month_title().as_str(),
        "Baishakh 1901 · Apr/May 1844"
    );

    show_month(&popup, &state, 2199, Month::Chaitra);
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
    let (popup, _state, clipboard) = app(NepaliDate::new(2081, 1, 1).expect("date"));
    let label = day_label(&popup, "1");
    click(&popup, &label);
    click(&popup, "Copy BS");
    click(&popup, "Copy AD");
    assert_eq!(
        *clipboard.texts.borrow(),
        vec!["2081/01/01".to_owned(), "2024-04-13".to_owned()]
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

/// Keep the month name model in step with the language helper used above.
#[test]
fn month_names_come_from_the_library() {
    let (popup, _state, _) = app(NepaliDate::new(2081, 1, 1).expect("date"));
    let months = popup.get_month_names();
    assert_eq!(months.row_count(), 12);
    assert_eq!(months.row_data(0).expect("Baishakh").as_str(), "Baishakh");
    assert_eq!(Month::Baishakh.name(Lang::En), "Baishakh");
}
