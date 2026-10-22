//! What a mouse press on the bar means (WR-08 AC4, AC6; ADR-0019 points 5 and 8): a click, a
//! double-click or a drag. A state machine over screen points, so the rules are tested without a
//! window. The Win32 module feeds it the button and move messages and acts on what it returns.

// The Windows module is the only caller, so this looks dead on other platforms.
#![cfg_attr(not(windows), allow(dead_code))]

use crate::app::bar_place::Point;

/// What the window should do after a pointer event.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Action {
    None,
    /// A press and release without movement: toggle the popup.
    Click,
    /// The second press of a double-click: open the popup and leave it open. The release that
    /// follows is swallowed, so it is not a click.
    DoubleClick,
    /// The bar follows the pointer: put its top-left at this point.
    DragTo(Point),
    /// The button went up after a drag: snap and save the position.
    DragEnd,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum State {
    Idle,
    /// The button is down and the pointer has not left the drag rectangle (or the bar is locked).
    Pressed {
        start: Point,
        grab: Point,
        moved: bool,
    },
    Dragging {
        grab: Point,
    },
    /// After a double-click press: the matching release is not a click.
    DoubleDown,
}

/// The pointer state of the bar. `locked` is `bar_locked`: a locked bar never drags, but a
/// click still toggles the popup.
#[derive(Debug, Clone, Copy)]
pub struct Pointer {
    state: State,
    locked: bool,
    /// `SM_CXDRAG` and `SM_CYDRAG`: the width and height of the rectangle around the press that
    /// the pointer has to leave before the press becomes a drag.
    drag: (i32, i32),
}

impl Pointer {
    pub const fn new(drag: (i32, i32)) -> Self {
        Self {
            state: State::Idle,
            locked: false,
            drag,
        }
    }

    pub fn set_locked(&mut self, locked: bool) {
        self.locked = locked;
    }

    /// Whether a button is down: the window holds the mouse capture exactly then.
    pub fn is_pressed(&self) -> bool {
        self.state != State::Idle
    }

    /// The first press of a click. `pointer` is the cursor and `window` the bar's top-left, both
    /// in screen coordinates.
    pub fn down(&mut self, pointer: Point, window: Point) -> Action {
        self.state = State::Pressed {
            start: pointer,
            grab: Point {
                x: pointer.x - window.x,
                y: pointer.y - window.y,
            },
            moved: false,
        };
        Action::None
    }

    /// The press `CS_DBLCLKS` reports as a double-click instead of a second `down`.
    pub fn double_down(&mut self) -> Action {
        self.state = State::DoubleDown;
        Action::DoubleClick
    }

    pub fn moved(&mut self, pointer: Point) -> Action {
        match self.state {
            State::Pressed { start, grab, .. } => {
                let outside = (pointer.x - start.x).abs() > self.drag.0 / 2
                    || (pointer.y - start.y).abs() > self.drag.1 / 2;
                if !outside {
                    return Action::None;
                }
                if self.locked {
                    // A locked bar ignores the drag, and the release is no click either.
                    self.state = State::Pressed {
                        start,
                        grab,
                        moved: true,
                    };
                    return Action::None;
                }
                self.state = State::Dragging { grab };
                Action::DragTo(Point {
                    x: pointer.x - grab.x,
                    y: pointer.y - grab.y,
                })
            }
            State::Dragging { grab } => Action::DragTo(Point {
                x: pointer.x - grab.x,
                y: pointer.y - grab.y,
            }),
            State::Idle | State::DoubleDown => Action::None,
        }
    }

    pub fn up(&mut self) -> Action {
        let state = std::mem::replace(&mut self.state, State::Idle);
        match state {
            State::Pressed { moved: false, .. } => Action::Click,
            State::Dragging { .. } => Action::DragEnd,
            State::Pressed { moved: true, .. } | State::Idle | State::DoubleDown => Action::None,
        }
    }

    /// The window lost the mouse capture while a button was down (another window took it, or
    /// Alt+Tab). A drag in progress ends where it is; a press is dropped.
    pub fn capture_lost(&mut self) -> Action {
        match std::mem::replace(&mut self.state, State::Idle) {
            State::Dragging { .. } => Action::DragEnd,
            _ => Action::None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const fn point(x: i32, y: i32) -> Point {
        Point { x, y }
    }

    /// `SM_CXDRAG` and `SM_CYDRAG` are 4 on a default Windows install.
    fn pointer() -> Pointer {
        Pointer::new((4, 4))
    }

    #[test]
    fn wr08_ac6_a_press_and_release_without_movement_is_a_click() {
        let mut p = pointer();
        assert_eq!(p.down(point(100, 100), point(60, 90)), Action::None);
        assert!(p.is_pressed());
        assert_eq!(p.up(), Action::Click);
        assert!(!p.is_pressed());
    }

    #[test]
    fn wr08_ac4_movement_inside_the_drag_rectangle_is_still_a_click() {
        let mut p = pointer();
        p.down(point(100, 100), point(60, 90));
        assert_eq!(p.moved(point(102, 98)), Action::None);
        assert_eq!(p.moved(point(98, 102)), Action::None);
        assert_eq!(p.up(), Action::Click);
    }

    #[test]
    fn wr08_ac4_past_the_drag_distance_the_bar_follows_the_pointer() {
        let mut p = pointer();
        // The press is 40 px right of and 10 px below the bar's top-left.
        p.down(point(100, 100), point(60, 90));
        assert_eq!(p.moved(point(103, 100)), Action::DragTo(point(63, 90)));
        assert_eq!(p.moved(point(180, 140)), Action::DragTo(point(140, 130)));
        // Back near the start: still dragging, no click.
        assert_eq!(p.moved(point(100, 100)), Action::DragTo(point(60, 90)));
        assert_eq!(p.up(), Action::DragEnd);
        assert!(!p.is_pressed());
    }

    #[test]
    fn wr08_ac4_a_drag_that_started_on_one_axis_follows_on_both() {
        let mut p = pointer();
        p.down(point(100, 100), point(60, 90));
        assert_eq!(p.moved(point(100, 106)), Action::DragTo(point(60, 96)));
        assert_eq!(p.moved(point(130, 106)), Action::DragTo(point(90, 96)));
    }

    #[test]
    fn wr08_ac4_a_locked_bar_does_not_drag_and_the_release_is_not_a_click() {
        let mut p = pointer();
        p.set_locked(true);
        p.down(point(100, 100), point(60, 90));
        assert_eq!(p.moved(point(180, 140)), Action::None);
        assert_eq!(p.moved(point(300, 300)), Action::None);
        assert_eq!(p.up(), Action::None);
    }

    #[test]
    fn wr08_ac4_a_locked_bar_still_takes_a_click() {
        let mut p = pointer();
        p.set_locked(true);
        p.down(point(100, 100), point(60, 90));
        assert_eq!(p.up(), Action::Click);
    }

    #[test]
    fn wr08_ac4_locking_during_a_drag_does_not_cut_it_short() {
        let mut p = pointer();
        p.down(point(100, 100), point(60, 90));
        p.moved(point(150, 100));
        p.set_locked(true);
        assert_eq!(p.moved(point(160, 100)), Action::DragTo(point(120, 90)));
        assert_eq!(p.up(), Action::DragEnd);
    }

    #[test]
    fn wr08_ac6_a_double_click_opens_and_its_release_is_not_a_click() {
        let mut p = pointer();
        // First click: a toggle.
        p.down(point(100, 100), point(60, 90));
        assert_eq!(p.up(), Action::Click);
        // Second press, reported by the window class as a double-click.
        assert_eq!(p.double_down(), Action::DoubleClick);
        assert!(
            p.is_pressed(),
            "the window keeps the capture until the release"
        );
        assert_eq!(
            p.moved(point(140, 140)),
            Action::None,
            "no drag out of a double-click"
        );
        assert_eq!(
            p.up(),
            Action::None,
            "the second click never toggles the popup"
        );
        assert!(!p.is_pressed());
    }

    #[test]
    fn wr08_ac6_a_click_after_a_double_click_is_a_click_again() {
        let mut p = pointer();
        p.double_down();
        p.up();
        p.down(point(100, 100), point(60, 90));
        assert_eq!(p.up(), Action::Click);
    }

    #[test]
    fn a_move_or_release_with_no_button_down_does_nothing() {
        let mut p = pointer();
        assert_eq!(p.moved(point(5, 5)), Action::None);
        assert_eq!(p.up(), Action::None);
        assert_eq!(p.capture_lost(), Action::None);
    }

    #[test]
    fn losing_the_capture_ends_a_drag_in_place_and_drops_a_press() {
        let mut p = pointer();
        p.down(point(100, 100), point(60, 90));
        p.moved(point(150, 100));
        assert_eq!(p.capture_lost(), Action::DragEnd);
        assert_eq!(p.up(), Action::None, "the release after it is nothing");

        p.down(point(100, 100), point(60, 90));
        assert_eq!(p.capture_lost(), Action::None);
        assert!(!p.is_pressed());
        assert_eq!(p.up(), Action::None);
    }

    #[test]
    fn the_drag_rectangle_follows_the_system_metrics() {
        let mut wide = Pointer::new((16, 16));
        wide.down(point(100, 100), point(60, 90));
        assert_eq!(wide.moved(point(106, 100)), Action::None, "inside 16 px");
        assert!(matches!(wide.moved(point(109, 100)), Action::DragTo(_)));
    }
}
