//! Layout tests for the popup (W8-4a). They read the boxes the layout gives icons and captions, so
//! an icon that drifts off the middle of its pill fails. The testing backend measures text with
//! the bundled fonts, like the app.

use i_slint_backend_testing::ElementHandle;

use crate::PopupWindow;
use crate::interaction_tests::{app, date};

/// A laid-out box in window coordinates.
#[derive(Debug, Clone, Copy)]
struct Rect {
    x: f32,
    y: f32,
    w: f32,
    h: f32,
}

impl Rect {
    fn of(element: &ElementHandle) -> Self {
        let position = element.absolute_position();
        let size = element.size();
        Rect {
            x: position.x,
            y: position.y,
            w: size.width,
            h: size.height,
        }
    }

    fn right(self) -> f32 {
        self.x + self.w
    }

    fn bottom(self) -> f32 {
        self.y + self.h
    }

    /// Whether the top-left corner of `part` is in this box.
    fn holds(self, part: Rect) -> bool {
        part.x >= self.x && part.x < self.right() && part.y >= self.y && part.y < self.bottom()
    }
}

// Icons and captions.

/// The boxes of the `LineIcon`s and the texts that sit inside `container`.
fn parts_in(popup: &PopupWindow, container: Rect) -> (Vec<Rect>, Vec<Rect>) {
    let inside = |type_name: &str| {
        ElementHandle::find_by_element_type_name(popup, type_name)
            .map(|element| Rect::of(&element))
            .filter(|rect| container.holds(*rect))
            .collect::<Vec<_>>()
    };
    (inside("LineIcon"), inside("Text"))
}

fn pill(popup: &PopupWindow, label: &str) -> Rect {
    let found = ElementHandle::find_by_accessible_label(popup, label)
        .next()
        .unwrap_or_else(|| panic!("no element labelled {label:?}"));
    Rect::of(&found)
}

#[test]
fn w8_4a_the_icon_of_an_icon_only_pill_is_in_the_middle() {
    let (popup, _state, _) = app(date(2083, 7, 5));
    for label in [
        "Settings",
        "Pin, not pinned",
        "Previous month",
        "Next month",
    ] {
        let pill = pill(&popup, label);
        let (icons, _) = parts_in(&popup, pill);
        assert_eq!(icons.len(), 1, "{label}");
        let icon = icons[0];
        let dx = icon.x + icon.w / 2.0 - (pill.x + pill.w / 2.0);
        let dy = icon.y + icon.h / 2.0 - (pill.y + pill.h / 2.0);
        assert!(
            dx.abs() < 0.01 && dy.abs() < 0.01,
            "{label}: the icon is {dx} px right and {dy} px below the middle"
        );
    }
}

#[test]
fn w8_4a_a_pill_with_an_icon_and_text_has_the_same_air_on_both_sides_of_the_ink() {
    let (popup, _state, _) = app(date(2083, 7, 5));
    let pill = pill(&popup, "Convert");
    let (icons, texts) = parts_in(&popup, pill);
    let (icon, text) = (icons[0], texts[0]);
    // The icon's drawing leaves 2 px of air inside its 16 px box, so its box starts 2 px closer
    // to the edge than the text's end.
    let left = icon.x - pill.x;
    let right = pill.right() - text.right();
    assert!(
        (right - left - 2.0).abs() < 0.01,
        "the icon box is {left} px from the edge and the text ends {right} px from the other"
    );
    let gap = text.x - icon.right();
    assert!((gap - 6.0).abs() < 0.01, "icon and text are {gap} px apart");
    let dy = icon.y + icon.h / 2.0 - (text.y + text.h / 2.0);
    assert!(dy.abs() < 0.01, "the icon is {dy} px off the text's middle");
}

#[test]
fn w8_4a_the_holiday_line_chevron_is_level_with_its_text() {
    let (popup, _state, _) = app(date(2083, 7, 5));
    let line = pill(&popup, "Upcoming holidays");
    let (icons, texts) = parts_in(&popup, line);
    let (icon, text) = (icons[0], texts[0]);
    let dy = icon.y + icon.h / 2.0 - (text.y + text.h / 2.0);
    assert!(
        dy.abs() < 0.01,
        "the chevron is {dy} px off the text's middle"
    );
    let dline = icon.y + icon.h / 2.0 - (line.y + line.h / 2.0);
    assert!(
        dline.abs() < 0.01,
        "the chevron is {dline} px off the line's middle"
    );
}
