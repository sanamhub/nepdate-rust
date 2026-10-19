//! Pure model for the Windows taskbar text (WR-12, ADR-0016). No Win32, no Slint: the line text
//! and the placement maths are unit-tested here, and `src/platform/windows/` only feeds them
//! rectangles and pixels.
// Only the Windows module consumes these items, so they look dead on other platforms.
#![allow(dead_code)]

use nepdate::{LongDate, NepaliDate};

use crate::app::clock;
use crate::app::format;
use crate::app::settings::Settings;

/// The two lines drawn inside the taskbar (WR-12 AC1). Empty when the local date is outside the
/// supported BS range (1901..=2199), which only happens with a wildly wrong system clock.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct TaskbarLines {
    pub line1: String,
    pub line2: String,
}

/// Line 1: weekday, ` | `, local time (WR-05 AC2 format). Line 2: BS long date, ` | `, AD date
/// as `%b %-d, %Y` with the language's digits (the AD month name stays Latin). The weekday and
/// BS date come from the same local date (WR-05 AC4).
pub fn taskbar_lines(now: &jiff::Zoned, s: &Settings) -> TaskbarLines {
    let lang = s.lang();
    let Ok(bs) = NepaliDate::try_from(now.date()) else {
        return TaskbarLines::default();
    };
    TaskbarLines {
        line1: format!(
            "{} | {}",
            bs.weekday().name(lang),
            clock::local_time(now, s)
        ),
        line2: format!(
            "{} | {}",
            bs.long(
                LongDate {
                    weekday: false,
                    year: true,
                    pad: false
                },
                lang
            ),
            format::ad_day_month_year(bs.to_gregorian(), lang),
        ),
    }
}

/// A rectangle in physical pixels. In `TaskbarLayout` every rectangle is in taskbar client
/// coordinates (the child text window's parent), which keeps `place` pure.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Rect {
    pub left: i32,
    pub top: i32,
    pub right: i32,
    pub bottom: i32,
}

/// Which screen edge the taskbar is docked to (`SHAppBarMessage(ABM_GETTASKBARPOS)`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Edge {
    Bottom,
    Top,
    Left,
    Right,
}

/// What `place` needs: the taskbar rectangle, its edge, the Start and Widgets button rectangles
/// (UIA, in client coordinates) and the DPI scale (`dpi / 96`). `start` and `widgets` are `None`
/// when the button is not found.
#[derive(Debug, Clone, PartialEq)]
pub struct TaskbarLayout {
    pub taskbar: Rect,
    pub edge: Edge,
    pub start: Option<Rect>,
    pub widgets: Option<Rect>,
    pub scale: f32,
}

/// Why the text is hidden (WR-12 AC4). `place` never returns `Disabled`; the controller uses it
/// for the `taskbar_text` setting being off.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Hidden {
    NotBottom,
    NoStartButton,
    NoRoom,
    Disabled,
}

/// Positions the text in the taskbar's free area, left of the Start button (WR-12 AC4). The
/// returned rectangle is in taskbar client coordinates. The free area runs from the Widgets
/// button's right edge (when it is shown and left of Start) or the taskbar's left edge, to the
/// Start button's left edge; the text sits 12 px (scaled) from the left, vertically centred, and
/// needs 12 px (scaled) free on its right. A left-aligned taskbar has Start at the far left, so
/// it ends as `NoRoom` with no special case.
// The inset is 12 * scale for a taskbar DPI of at most a few hundred percent, so the f32-to-i32
// cast never truncates anything but the fractional rounding.
#[allow(clippy::cast_possible_truncation)]
pub fn place(layout: &TaskbarLayout, text: (i32, i32)) -> Result<Rect, Hidden> {
    if layout.edge != Edge::Bottom {
        return Err(Hidden::NotBottom);
    }
    let start = layout.start.ok_or(Hidden::NoStartButton)?;
    let left_bound = match layout.widgets {
        Some(widgets) if widgets.right < start.left => widgets.right,
        _ => layout.taskbar.left,
    };
    let inset = (12.0 * layout.scale).round() as i32;
    let (width, height) = text;
    let x = left_bound + inset;
    if x + width + inset > start.left {
        return Err(Hidden::NoRoom);
    }
    let y = layout.taskbar.top + (layout.taskbar.bottom - layout.taskbar.top - height) / 2;
    Ok(Rect {
        left: x,
        top: y,
        right: x + width,
        bottom: y + height,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn kathmandu(ts: &str) -> jiff::Zoned {
        ts.parse().expect("valid fixed Zoned")
    }

    fn layout() -> TaskbarLayout {
        TaskbarLayout {
            taskbar: Rect {
                left: 0,
                top: 0,
                right: 1920,
                bottom: 48,
            },
            edge: Edge::Bottom,
            start: Some(Rect {
                left: 806,
                top: 0,
                right: 851,
                bottom: 48,
            }),
            widgets: None,
            scale: 1.0,
        }
    }

    #[test]
    fn wr12_ac1_lines_en_12h() {
        let now = kathmandu("2026-10-15T13:50:00+05:45[Asia/Kathmandu]");
        let lines = taskbar_lines(&now, &Settings::default());
        assert_eq!(lines.line1, "Thursday | 1:50 PM");
        assert_eq!(lines.line2, "Ashoj 29, 2083 | Oct 15, 2026");
    }

    #[test]
    fn wr12_ac1_lines_en_24h() {
        let now = kathmandu("2026-10-15T13:50:00+05:45[Asia/Kathmandu]");
        let s = Settings {
            clock_24h: true,
            ..Settings::default()
        };
        let lines = taskbar_lines(&now, &s);
        assert_eq!(lines.line1, "Thursday | 13:50");
        assert_eq!(lines.line2, "Ashoj 29, 2083 | Oct 15, 2026");
    }

    #[test]
    fn wr12_ac1_lines_ne() {
        let now = kathmandu("2026-10-15T13:50:00+05:45[Asia/Kathmandu]");
        let s = Settings {
            language: "ne".into(),
            ..Settings::default()
        };
        let lines = taskbar_lines(&now, &s);
        assert_eq!(lines.line1, "बिहिवार | १:५० PM");
        assert_eq!(lines.line2, "असोज २९, २०८३ | Oct १५, २०२६");
    }

    #[test]
    fn wr12_ac1_weekday_and_bs_from_same_date() {
        // The weekday follows the BS date shown next to it, not a shifted zone: at 23:59:30 it is
        // still Thursday (Ashoj 29), at 00:00:30 it is Friday (Ashoj 30).
        let before = kathmandu("2026-10-15T23:59:30+05:45[Asia/Kathmandu]");
        let after = kathmandu("2026-10-16T00:00:30+05:45[Asia/Kathmandu]");
        let b = taskbar_lines(&before, &Settings::default());
        let a = taskbar_lines(&after, &Settings::default());
        assert_eq!(b.line1, "Thursday | 11:59 PM");
        assert_eq!(b.line2, "Ashoj 29, 2083 | Oct 15, 2026");
        assert_eq!(a.line1, "Friday | 12:00 AM");
        assert_eq!(a.line2, "Ashoj 30, 2083 | Oct 16, 2026");
    }

    #[test]
    fn wr12_ac4_place_free_area() {
        let l = layout();
        let r = place(&l, (200, 40)).unwrap();
        assert_eq!(r.left, 12);
        assert_eq!(r.top, 4); // (48 - 40) / 2
        assert_eq!(r.right, 212);
        assert_eq!(r.bottom, 44);
    }

    #[test]
    fn wr12_ac4_place_after_widgets_button() {
        let mut l = layout();
        l.widgets = Some(Rect {
            left: 0,
            top: 0,
            right: 100,
            bottom: 48,
        });
        let r = place(&l, (200, 40)).unwrap();
        assert_eq!(r.left, 112); // 100 + 12
        assert_eq!(r.right, 312);
    }

    #[test]
    fn wr12_ac4_left_aligned_is_no_room() {
        // A left-aligned taskbar has Start at the far left; the text cannot fit before it.
        let mut l = layout();
        l.start = Some(Rect {
            left: 0,
            top: 0,
            right: 45,
            bottom: 48,
        });
        assert_eq!(place(&l, (200, 40)), Err(Hidden::NoRoom));
    }

    #[test]
    fn wr12_ac4_not_bottom() {
        let mut l = layout();
        l.edge = Edge::Top;
        assert_eq!(place(&l, (200, 40)), Err(Hidden::NotBottom));
    }

    #[test]
    fn wr12_ac4_no_start_button() {
        let mut l = layout();
        l.start = None;
        assert_eq!(place(&l, (200, 40)), Err(Hidden::NoStartButton));
    }

    #[test]
    fn wr12_ac4_exact_fit_and_one_px_short() {
        // Exact fit: x + width + inset == start.left is allowed.
        let l = layout();
        // start.left = 806, x = 12, so width = 806 - 12 - 12 = 782 fits exactly.
        assert!(place(&l, (782, 40)).is_ok());
        // One pixel short: width 783 needs 12 + 783 + 12 = 807 > 806.
        assert_eq!(place(&l, (783, 40)), Err(Hidden::NoRoom));
    }

    #[test]
    fn wr12_ac7_scale_150_and_200() {
        let mut l = layout();
        l.scale = 1.5;
        assert_eq!(place(&l, (200, 40)).unwrap().left, 18); // round(12 * 1.5)
        l.scale = 2.0;
        assert_eq!(place(&l, (200, 40)).unwrap().left, 24); // round(12 * 2.0)
    }
}
