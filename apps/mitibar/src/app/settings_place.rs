//! Where the Settings window opens (WR-06 AC1). A pure function over rectangles in physical
//! pixels: beside the popup on the side with more room, top edges aligned and 8 logical px apart;
//! centred on the monitor when neither side has room or the popup is closed.
//!
//! The gap and the alignment are between what is drawn. Windows gives a framed window an invisible
//! resize border, so the window's own rectangle reaches past its visible edge; `Insets` carries
//! that, and the position returned is the window's, not the visible rectangle's.

use crate::app::bar_place::{Point, Rect, Size, clamp, to_physical};

/// The space between the popup and the Settings window, in logical px (WR-06 AC1).
const GAP: i32 = 8;

/// How far a window's own rectangle reaches past its visible one on each side, in physical px.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Insets {
    pub left: i32,
    pub top: i32,
    pub right: i32,
    pub bottom: i32,
}

/// The visible size and the invisible insets of a framed window whose client area is `client`,
/// from the system metrics at the window's DPI: the caption height and the width of the sizing
/// frame with its padded border (`SM_CYCAPTION`, and `SM_CXSIZEFRAME` plus `SM_CXPADDEDBORDER`).
/// Windows 10 and 11 draw the frame one pixel inside its sizing border, and leave out the top.
pub fn framed(client: Size, caption: i32, frame: i32) -> (Size, Insets) {
    let invisible = (frame - 1).max(0);
    let visible = Size {
        width: client.width + 2 * (frame - invisible),
        height: client.height + caption + frame + (frame - invisible),
    };
    (
        visible,
        Insets {
            left: invisible,
            top: 0,
            right: invisible,
            bottom: invisible,
        },
    )
}

/// Where the top-left of a Settings window goes. `visible` is the size of what is drawn and
/// `insets` the invisible border around it. `popup` is the popup's rectangle when it is open;
/// `work` is the work area of the popup's monitor, or of the monitor to centre on when there is
/// no popup. `dpi` scales the gap.
pub fn position(popup: Option<Rect>, visible: Size, insets: Insets, work: Rect, dpi: u32) -> Point {
    let centred = Point {
        x: work.left + (work.right - work.left - visible.width) / 2,
        y: work.top + (work.bottom - work.top - visible.height) / 2,
    };
    let at = |shown: Point| {
        // `clamp` keeps what is drawn on the screen, then the window's own corner is further out.
        let shown = clamp(shown, visible, work);
        Point {
            x: shown.x - insets.left,
            y: shown.y - insets.top,
        }
    };
    let Some(popup) = popup else {
        return at(centred);
    };
    let gap = to_physical(GAP, dpi);
    let room_right = work.right - popup.right - gap;
    let room_left = popup.left - work.left - gap;
    // The roomier side; the right one on a tie.
    let (room, x) = if room_right >= room_left {
        (room_right, popup.right + gap)
    } else {
        (room_left, popup.left - gap - visible.width)
    };
    if room < visible.width {
        return at(centred);
    }
    at(Point { x, y: popup.top })
}

#[cfg(test)]
mod tests {
    use super::*;

    const WORK: Rect = Rect::new(0, 0, 1920, 1032);
    /// 420 by 560 logical px at 100 %, with no frame around it.
    const SIZE: Size = Size {
        width: 420,
        height: 560,
    };
    const NONE: Insets = Insets {
        left: 0,
        top: 0,
        right: 0,
        bottom: 0,
    };

    /// A 360 by 520 popup whose top-left is at `(x, y)`.
    fn popup_at(x: i32, y: i32) -> Rect {
        Rect::at(
            Point { x, y },
            Size {
                width: 360,
                height: 520,
            },
        )
    }

    #[test]
    fn wr06_ac1_beside_the_popup_on_the_right_when_that_side_has_more_room() {
        // A popup at the left of a wide screen: the right side has 1512 px.
        let at = position(Some(popup_at(40, 100)), SIZE, NONE, WORK, 96);
        assert_eq!(
            at,
            Point { x: 400 + 8, y: 100 },
            "8 px right of the popup, tops aligned"
        );
    }

    #[test]
    fn wr06_ac1_beside_the_popup_on_the_left_when_that_side_has_more_room() {
        // The tray corner: the popup is at the bottom-right, so the left side is the roomy one.
        let at = position(Some(popup_at(1548, 500)), SIZE, NONE, WORK, 96);
        assert_eq!(
            at,
            Point {
                x: 1548 - 8 - 420,
                y: 472
            },
            "8 px left of the popup, pushed up to stay on the screen"
        );
        assert_eq!(
            at.y + SIZE.height,
            WORK.bottom,
            "the bottom edge sits on the work area"
        );
    }

    #[test]
    fn wr06_ac1_the_top_edges_align_when_the_window_fits_below_the_popups_top() {
        let at = position(Some(popup_at(40, 300)), SIZE, NONE, WORK, 96);
        assert_eq!(at.y, 300);
    }

    #[test]
    fn wr06_ac1_a_tie_goes_to_the_right() {
        // 1920 wide, a 360 px popup in the middle: 780 px of room on both sides.
        let at = position(Some(popup_at(780, 100)), SIZE, NONE, WORK, 96);
        assert_eq!(at.x, 1140 + 8);
    }

    #[test]
    fn wr06_ac1_centred_when_neither_side_has_room() {
        // A 1000 px wide screen: 320 px each side of a centred popup, too little for 420 px.
        let work = Rect::new(0, 0, 1000, 800);
        let at = position(Some(popup_at(320, 100)), SIZE, NONE, work, 96);
        assert_eq!(
            at,
            Point {
                x: (1000 - 420) / 2,
                y: (800 - 560) / 2
            }
        );
    }

    #[test]
    fn wr06_ac1_the_roomier_side_must_also_fit_the_window() {
        // 332 px on the right and 292 on the left: the right side is roomier, still under 420 px.
        let work = Rect::new(0, 0, 1000, 800);
        let at = position(Some(popup_at(300, 100)), SIZE, NONE, work, 96);
        assert_eq!(
            at,
            Point { x: 290, y: 120 },
            "centred, not squeezed beside the popup"
        );
    }

    #[test]
    fn wr06_ac1_centred_on_the_monitor_when_the_popup_is_closed() {
        let at = position(None, SIZE, NONE, WORK, 96);
        assert_eq!(
            at,
            Point {
                x: (1920 - 420) / 2,
                y: (1032 - 560) / 2
            }
        );
    }

    #[test]
    fn wr06_ac1_centring_follows_a_monitor_that_is_not_at_the_origin() {
        let work = Rect::new(-1920, -200, 0, 832);
        let at = position(None, SIZE, NONE, work, 96);
        assert_eq!(
            at,
            Point {
                x: -1920 + (1920 - 420) / 2,
                y: -200 + (1032 - 560) / 2
            }
        );
    }

    #[test]
    fn wr06_ac1_the_gap_and_size_scale_with_the_monitor() {
        // 200 %: the window is 840 by 1120 physical px, the gap 16.
        let size = Size {
            width: 840,
            height: 1120,
        };
        let work = Rect::new(0, 0, 3840, 2064);
        let popup = Some(Rect::new(80, 200, 800, 1240));
        let at = position(popup, size, NONE, work, 192);
        assert_eq!(
            at,
            Point {
                x: 800 + 16,
                y: 200
            }
        );
    }

    #[test]
    fn a_window_taller_than_the_work_area_starts_at_its_top() {
        let work = Rect::new(0, 0, 1920, 500);
        let at = position(Some(popup_at(40, 0)), SIZE, NONE, work, 96);
        assert_eq!(at.y, 0);
    }

    /// What Windows 11 reports at 100 %: a 4 px sizing frame and a 4 px padded border, a 23 px
    /// caption, and a window whose outer rectangle is 436 by 599 around a 420 by 560 client area,
    /// drawn 422 by 592 (measured with `DWMWA_EXTENDED_FRAME_BOUNDS`).
    #[test]
    fn framed_matches_what_windows_11_measured_at_100_percent() {
        let (visible, insets) = framed(SIZE, 23, 8);
        assert_eq!(
            visible,
            Size {
                width: 422,
                height: 592
            }
        );
        assert_eq!(
            insets,
            Insets {
                left: 7,
                top: 0,
                right: 7,
                bottom: 7
            }
        );
        assert_eq!(
            visible.width + insets.left + insets.right,
            436,
            "the outer width"
        );
        assert_eq!(visible.height + insets.bottom, 599, "the outer height");
    }

    #[test]
    fn wr06_ac1_the_gap_is_between_what_is_drawn_not_between_the_window_rectangles() {
        let (visible, insets) = framed(SIZE, 23, 8);
        // The popup is at the right of the screen, so Settings goes to its left.
        let popup = popup_at(1500, 100);
        let at = position(Some(popup), visible, insets, WORK, 96);
        let drawn_right = at.x + insets.left + visible.width;
        assert_eq!(popup.left - drawn_right, 8, "8 px between the drawn edges");
        assert_eq!(at.y + insets.top, popup.top, "the tops line up");

        // And to the right of a popup at the left.
        let popup = popup_at(40, 100);
        let at = position(Some(popup), visible, insets, WORK, 96);
        assert_eq!(at.x + insets.left - popup.right, 8);
    }

    #[test]
    fn a_framed_window_is_drawn_whole_on_the_screen_when_it_is_pushed_up() {
        let (visible, insets) = framed(SIZE, 23, 8);
        let at = position(Some(popup_at(1500, 700)), visible, insets, WORK, 96);
        assert_eq!(
            at.y + insets.top + visible.height,
            WORK.bottom,
            "its drawn bottom edge sits on the work area"
        );
    }

    #[test]
    fn a_framed_window_is_centred_by_what_is_drawn() {
        let (visible, insets) = framed(SIZE, 23, 8);
        let at = position(None, visible, insets, WORK, 96);
        let left = at.x + insets.left;
        let right = WORK.right - (at.x + insets.left + visible.width);
        assert!((left - right).abs() <= 1, "{left} and {right}");
    }
}
