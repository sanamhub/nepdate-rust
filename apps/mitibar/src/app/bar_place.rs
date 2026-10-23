//! Where the bar goes (WR-08 AC1, AC4, AC5, AC8; ADR-0019 points 4 to 8). Pure functions over
//! rectangles, so the default position, clamping, snapping and the saved-position restore are
//! tested without a desktop. Rectangles and points are physical pixels in the virtual screen; a
//! saved position is logical pixels from its monitor's top-left, so it survives a DPI change.

// The Windows module is the only caller, so these look dead on other platforms.
#![cfg_attr(not(windows), allow(dead_code))]

/// How far the bar sits from the edge it is placed against, in logical px (ADR-0019 point 7).
const INSET: i32 = 12;
/// How close to the taskbar's centre line a drop snaps to it, in logical px (ADR-0019 point 5).
const SNAP: i32 = 8;

/// A rectangle in physical pixels.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Rect {
    pub left: i32,
    pub top: i32,
    pub right: i32,
    pub bottom: i32,
}

impl Rect {
    pub const fn new(left: i32, top: i32, right: i32, bottom: i32) -> Self {
        Self {
            left,
            top,
            right,
            bottom,
        }
    }

    pub const fn height(self) -> i32 {
        self.bottom - self.top
    }

    fn contains(self, point: Point) -> bool {
        point.x >= self.left && point.x < self.right && point.y >= self.top && point.y < self.bottom
    }

    /// The rectangle of `size` whose top-left is `at`.
    pub const fn at(at: Point, size: Size) -> Self {
        Self::new(at.x, at.y, at.x + size.width, at.y + size.height)
    }

    /// The squared distance from `point` to the nearest point of this rectangle (0 inside).
    fn distance_squared(self, point: Point) -> i64 {
        let dx = i64::from((self.left - point.x).max(point.x - (self.right - 1)).max(0));
        let dy = i64::from((self.top - point.y).max(point.y - (self.bottom - 1)).max(0));
        dx * dx + dy * dy
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Point {
    pub x: i32,
    pub y: i32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Size {
    pub width: i32,
    pub height: i32,
}

/// One monitor. `work` is the area the taskbar leaves free; the bar floats over the taskbar, so it
/// is placed against `bounds`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Monitor {
    /// The device name, such as `\\.\DISPLAY1`. It is what a saved position names.
    pub name: String,
    pub bounds: Rect,
    pub work: Rect,
    pub dpi: u32,
    pub primary: bool,
}

/// What the default position needs to know about the shell (ADR-0019 point 7).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Shell {
    /// A Windows 11 taskbar with centred icons, which leaves its left end empty.
    pub centred: bool,
    /// `TrayNotifyWnd`, the notification area, in screen coordinates.
    pub tray_notify: Option<Rect>,
}

/// A saved position (WR-06 `bar_monitor`, `bar_x`, `bar_y`): the monitor's device name and the
/// bar's top-left in logical px from that monitor's top-left. An empty name means "the default
/// position".
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Saved {
    pub monitor: String,
    pub x: i32,
    pub y: i32,
}

/// Which screen edge a taskbar is docked to.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Edge {
    Bottom,
    Top,
    Left,
    Right,
}

/// Logical px to physical px at `dpi`, rounded to the nearest.
pub fn to_physical(logical: i32, dpi: u32) -> i32 {
    round_div(i64::from(logical) * i64::from(dpi), 96)
}

/// Physical px to logical px at `dpi`, rounded to the nearest.
pub fn to_logical(physical: i32, dpi: u32) -> i32 {
    round_div(i64::from(physical) * 96, i64::from(dpi.max(1)))
}

/// `numerator / denominator` rounded half up, also for a negative numerator.
fn round_div(numerator: i64, denominator: i64) -> i32 {
    let rounded = (2 * numerator + denominator).div_euclid(2 * denominator);
    i32::try_from(rounded).unwrap_or(if rounded < 0 { i32::MIN } else { i32::MAX })
}

/// The taskbar on `monitor`, read from the gap between the monitor and its work area. `None` when
/// there is no gap: an auto-hide taskbar takes no room.
pub fn taskbar_of(monitor: &Monitor) -> Option<(Edge, Rect)> {
    let (b, w) = (monitor.bounds, monitor.work);
    if w.bottom < b.bottom {
        Some((Edge::Bottom, Rect::new(b.left, w.bottom, b.right, b.bottom)))
    } else if w.top > b.top {
        Some((Edge::Top, Rect::new(b.left, b.top, b.right, w.top)))
    } else if w.left > b.left {
        Some((Edge::Left, Rect::new(b.left, b.top, w.left, b.bottom)))
    } else if w.right < b.right {
        Some((Edge::Right, Rect::new(w.right, b.top, b.right, b.bottom)))
    } else {
        None
    }
}

/// The bar's top-left when no position is saved (ADR-0019 point 7), on the primary monitor. On a
/// horizontal taskbar the bar is centred vertically in it. A centred Windows 11 taskbar leaves its
/// left end empty, so the bar sits `INSET` from the taskbar's left edge; otherwise its right edge
/// sits `INSET` left of the notification area. With a vertical or hidden taskbar, or no
/// notification area to measure from, the bar sits `INSET` in from the bottom-right corner of the
/// work area.
pub fn default_position(primary: &Monitor, shell: &Shell, size: Size) -> Point {
    let inset = to_physical(INSET, primary.dpi);
    let corner = Point {
        x: primary.work.right - inset - size.width,
        y: primary.work.bottom - inset - size.height,
    };
    let point = match taskbar_of(primary) {
        Some((Edge::Bottom | Edge::Top, taskbar)) => {
            let y = taskbar.top + (taskbar.height() - size.height) / 2;
            match (shell.centred, shell.tray_notify) {
                (true, _) => Point {
                    x: taskbar.left + inset,
                    y,
                },
                (false, Some(tray)) => Point {
                    x: tray.left - inset - size.width,
                    y,
                },
                (false, None) => corner,
            }
        }
        _ => corner,
    };
    clamp(point, size, primary.bounds)
}

/// Moves `point` so a bar of `size` lies fully inside `bounds`. A bar larger than `bounds` keeps
/// its top-left corner on screen.
pub fn clamp(point: Point, size: Size, bounds: Rect) -> Point {
    Point {
        x: point.x.min(bounds.right - size.width).max(bounds.left),
        y: point.y.min(bounds.bottom - size.height).max(bounds.top),
    }
}

/// The monitor a saved position is drawn on: the one it names, else the primary monitor (first
/// run, a "Reset position", or a monitor that is gone). `None` only with no monitors at all.
pub fn target_monitor<'a>(saved: &Saved, monitors: &'a [Monitor]) -> Option<&'a Monitor> {
    monitors
        .iter()
        .find(|monitor| !saved.monitor.is_empty() && monitor.name == saved.monitor)
        .or_else(|| monitors.iter().find(|monitor| monitor.primary))
        .or_else(|| monitors.first())
}

/// The bar's top-left on `monitor` (WR-08 AC5, AC8): the saved position if it is for this monitor,
/// moved fully on-screen, otherwise the default position.
pub fn position(saved: &Saved, monitor: &Monitor, shell: &Shell, size: Size) -> Point {
    if saved.monitor.is_empty() || saved.monitor != monitor.name {
        return default_position(monitor, shell, size);
    }
    let point = Point {
        x: monitor.bounds.left + to_physical(saved.x, monitor.dpi),
        y: monitor.bounds.top + to_physical(saved.y, monitor.dpi),
    };
    clamp(point, size, monitor.bounds)
}

/// The monitor under `point`, or the nearest one when the point is in a gap between monitors.
pub fn monitor_at(monitors: &[Monitor], point: Point) -> Option<&Monitor> {
    monitors
        .iter()
        .find(|monitor| monitor.bounds.contains(point))
        .or_else(|| {
            monitors
                .iter()
                .min_by_key(|monitor| monitor.bounds.distance_squared(point))
        })
}

/// Where a drag puts the bar: the top-left the pointer asks for, moved fully onto the monitor under
/// the pointer (ADR-0019 point 5). Returns the monitor too, so the caller can scale the bar to it.
pub fn drag_to(
    wanted: Point,
    pointer: Point,
    size: Size,
    monitors: &[Monitor],
) -> Option<(Point, &Monitor)> {
    let monitor = monitor_at(monitors, pointer)?;
    Some((clamp(wanted, size, monitor.bounds), monitor))
}

/// A drop within `SNAP` logical px of the taskbar's centre line lands on it (ADR-0019 point 5): the
/// vertical centre of a horizontal taskbar, the horizontal centre of a vertical one. Nothing snaps
/// on a monitor with no taskbar.
pub fn snap(point: Point, size: Size, monitor: &Monitor) -> Point {
    let Some((edge, taskbar)) = taskbar_of(monitor) else {
        return point;
    };
    let reach = to_physical(SNAP, monitor.dpi);
    let snapped = match edge {
        Edge::Bottom | Edge::Top => {
            let line = i32::midpoint(taskbar.top, taskbar.bottom);
            let centre = point.y + size.height / 2;
            if (centre - line).abs() <= reach {
                Point {
                    x: point.x,
                    y: line - size.height / 2,
                }
            } else {
                point
            }
        }
        Edge::Left | Edge::Right => {
            let line = i32::midpoint(taskbar.left, taskbar.right);
            let centre = point.x + size.width / 2;
            if (centre - line).abs() <= reach {
                Point {
                    x: line - size.width / 2,
                    y: point.y,
                }
            } else {
                point
            }
        }
    };
    clamp(snapped, size, monitor.bounds)
}

/// The position to save for a bar whose top-left is `point` on `monitor` (WR-06).
pub fn to_saved(point: Point, monitor: &Monitor) -> Saved {
    Saved {
        monitor: monitor.name.clone(),
        x: to_logical(point.x - monitor.bounds.left, monitor.dpi),
        y: to_logical(point.y - monitor.bounds.top, monitor.dpi),
    }
}

/// Where the popup opens for a click on the bar (WR-02 AC1, ADR-0019 point 8): above the bar when
/// there is room, below it otherwise, left-aligned with it, and fully inside `work`.
pub fn popup_position(bar: Rect, popup: Size, work: Rect) -> Point {
    let above = bar.top - popup.height;
    let y = if above >= work.top { above } else { bar.bottom };
    Point {
        x: bar.left.min(work.right - popup.width).max(work.left),
        y: y.min(work.bottom - popup.height).max(work.top),
    }
}

/// Whether a window covers a whole monitor, which is what the full-screen check looks for
/// (ADR-0019 point 4).
pub fn covers_monitor(window: Rect, monitor: Rect) -> bool {
    window.left <= monitor.left
        && window.top <= monitor.top
        && window.right >= monitor.right
        && window.bottom >= monitor.bottom
}

#[cfg(test)]
mod tests {
    use super::*;

    const fn size(width: i32, height: i32) -> Size {
        Size { width, height }
    }

    fn monitor(name: &str, bounds: Rect, work: Rect, dpi: u32, primary: bool) -> Monitor {
        Monitor {
            name: name.to_owned(),
            bounds,
            work,
            dpi,
            primary,
        }
    }

    /// 1920x1080 at 100 %, a 40 px bottom taskbar (the Windows 10 default).
    fn windows_10() -> (Monitor, Shell) {
        (
            monitor(
                r"\\.\DISPLAY1",
                Rect::new(0, 0, 1920, 1080),
                Rect::new(0, 0, 1920, 1040),
                96,
                true,
            ),
            Shell {
                centred: false,
                tray_notify: Some(Rect::new(1656, 1040, 1920, 1080)),
            },
        )
    }

    /// 1920x1080 at 100 %, a 48 px bottom taskbar with centred icons (the Windows 11 default).
    fn windows_11_centred() -> (Monitor, Shell) {
        (
            monitor(
                r"\\.\DISPLAY1",
                Rect::new(0, 0, 1920, 1080),
                Rect::new(0, 0, 1920, 1032),
                96,
                true,
            ),
            Shell {
                centred: true,
                tray_notify: Some(Rect::new(1624, 1032, 1920, 1080)),
            },
        )
    }

    // The bar for these fixtures: 220 x 38 px, two lines at 100 %.
    const BAR: Size = size(220, 38);

    #[test]
    fn rounding_is_to_the_nearest_for_both_signs() {
        assert_eq!(to_physical(12, 96), 12);
        assert_eq!(to_physical(12, 144), 18);
        assert_eq!(to_physical(12, 192), 24);
        assert_eq!(to_physical(5, 144), 8, "7.5 rounds up");
        assert_eq!(to_logical(18, 144), 12);
        assert_eq!(to_logical(8, 144), 5, "5.33 rounds down");
        assert_eq!(to_logical(-18, 144), -12);
        assert_eq!(to_physical(-5, 144), -7, "-7.5 rounds up, towards zero");
    }

    #[test]
    fn a_saved_offset_is_stable_through_physical_and_back_at_every_scale() {
        for dpi in [96, 120, 144, 168, 192] {
            for logical in -300..=300 {
                assert_eq!(
                    to_logical(to_physical(logical, dpi), dpi),
                    logical,
                    "{logical} logical px at {dpi} dpi"
                );
            }
        }
    }

    #[test]
    fn the_taskbar_is_read_from_the_gap_between_the_monitor_and_its_work_area() {
        let (m, _) = windows_10();
        assert_eq!(
            taskbar_of(&m),
            Some((Edge::Bottom, Rect::new(0, 1040, 1920, 1080)))
        );
        let top = monitor(
            "t",
            Rect::new(0, 0, 1920, 1080),
            Rect::new(0, 40, 1920, 1080),
            96,
            true,
        );
        assert_eq!(
            taskbar_of(&top),
            Some((Edge::Top, Rect::new(0, 0, 1920, 40)))
        );
        let left = monitor(
            "l",
            Rect::new(0, 0, 1920, 1080),
            Rect::new(60, 0, 1920, 1080),
            96,
            true,
        );
        assert_eq!(
            taskbar_of(&left),
            Some((Edge::Left, Rect::new(0, 0, 60, 1080)))
        );
        let right = monitor(
            "r",
            Rect::new(0, 0, 1920, 1080),
            Rect::new(0, 0, 1860, 1080),
            96,
            true,
        );
        assert_eq!(
            taskbar_of(&right),
            Some((Edge::Right, Rect::new(1860, 0, 1920, 1080)))
        );
        let hidden = monitor(
            "h",
            Rect::new(0, 0, 1920, 1080),
            Rect::new(0, 0, 1920, 1080),
            96,
            true,
        );
        assert_eq!(taskbar_of(&hidden), None);
    }

    #[test]
    fn wr08_ac5_default_position_on_windows_10_sits_left_of_the_notification_area() {
        let (m, shell) = windows_10();
        // Right edge 12 px left of TrayNotifyWnd (x = 1656), centred in the 40 px taskbar.
        let at = default_position(&m, &shell, BAR);
        assert_eq!(
            at,
            Point {
                x: 1656 - 12 - 220,
                y: 1040 + 1
            }
        );
    }

    #[test]
    fn wr08_ac5_default_position_on_a_centred_windows_11_taskbar_is_12_px_from_its_left_edge() {
        let (m, shell) = windows_11_centred();
        let at = default_position(&m, &shell, BAR);
        assert_eq!(at, Point { x: 12, y: 1032 + 5 });
    }

    #[test]
    fn wr08_ac5_default_position_on_a_left_aligned_windows_11_taskbar_sits_left_of_the_notification_area()
     {
        let (m, mut shell) = windows_11_centred();
        shell.centred = false;
        let at = default_position(&m, &shell, BAR);
        assert_eq!(
            at,
            Point {
                x: 1624 - 12 - 220,
                y: 1032 + 5
            }
        );
    }

    #[test]
    fn wr08_ac5_default_position_on_a_top_taskbar_is_centred_in_it() {
        let m = monitor(
            "top",
            Rect::new(0, 0, 1920, 1080),
            Rect::new(0, 40, 1920, 1080),
            96,
            true,
        );
        let shell = Shell {
            centred: false,
            tray_notify: Some(Rect::new(1656, 0, 1920, 40)),
        };
        assert_eq!(
            default_position(&m, &shell, BAR),
            Point {
                x: 1656 - 12 - 220,
                y: 1
            }
        );
    }

    #[test]
    fn wr08_ac5_default_position_with_a_vertical_taskbar_is_above_the_work_area_corner() {
        for (work, expected) in [
            // Left taskbar: the work area starts at x = 60.
            (
                Rect::new(60, 0, 1920, 1080),
                Point {
                    x: 1920 - 12 - 220,
                    y: 1080 - 12 - 38,
                },
            ),
            // Right taskbar: the work area ends at x = 1860.
            (
                Rect::new(0, 0, 1860, 1080),
                Point {
                    x: 1860 - 12 - 220,
                    y: 1080 - 12 - 38,
                },
            ),
        ] {
            let m = monitor("v", Rect::new(0, 0, 1920, 1080), work, 96, true);
            let shell = Shell {
                centred: true,
                tray_notify: Some(Rect::new(0, 1000, 60, 1080)),
            };
            assert_eq!(default_position(&m, &shell, BAR), expected);
        }
    }

    #[test]
    fn wr08_ac5_default_position_with_a_hidden_taskbar_is_above_the_work_area_corner() {
        let m = monitor(
            "h",
            Rect::new(0, 0, 1920, 1080),
            Rect::new(0, 0, 1920, 1080),
            96,
            true,
        );
        let shell = Shell {
            centred: true,
            tray_notify: None,
        };
        assert_eq!(
            default_position(&m, &shell, BAR),
            Point {
                x: 1920 - 12 - 220,
                y: 1080 - 12 - 38
            }
        );
    }

    #[test]
    fn wr08_ac5_a_bottom_taskbar_with_no_notification_area_falls_back_to_the_corner() {
        let (m, mut shell) = windows_10();
        shell.tray_notify = None;
        assert_eq!(
            default_position(&m, &shell, BAR),
            Point {
                x: 1920 - 12 - 220,
                y: 1040 - 12 - 38
            }
        );
    }

    #[test]
    fn wr08_ac8_default_position_scales_the_inset_at_150_and_200_percent() {
        // The same Windows 11 layout at 3840x2160: 150 % has a 72 px taskbar, 200 % a 96 px one.
        for (dpi, taskbar, inset, bar) in
            [(144, 72, 18, size(330, 57)), (192, 96, 24, size(440, 76))]
        {
            let m = monitor(
                "hi",
                Rect::new(0, 0, 3840, 2160),
                Rect::new(0, 0, 3840, 2160 - taskbar),
                dpi,
                true,
            );
            let shell = Shell {
                centred: true,
                tray_notify: None,
            };
            let at = default_position(&m, &shell, bar);
            assert_eq!(at.x, inset, "{dpi} dpi");
            assert_eq!(
                at.y,
                2160 - taskbar + (taskbar - bar.height) / 2,
                "{dpi} dpi"
            );
        }
    }

    #[test]
    fn wr08_ac8_default_position_stays_on_screen_when_the_bar_is_wider_than_the_room() {
        let (m, shell) = windows_10();
        // Wider than the room left of the notification area: moved fully on-screen, not off the left.
        let at = default_position(&m, &shell, size(1800, 38));
        assert_eq!(at.x, 0);
        let at = default_position(&m, &shell, size(2400, 38));
        assert_eq!(
            at.x, 0,
            "a bar wider than the monitor keeps its left edge on screen"
        );
    }

    #[test]
    fn clamp_keeps_the_bar_fully_on_the_monitor() {
        let bounds = Rect::new(0, 0, 1920, 1080);
        assert_eq!(
            clamp(Point { x: 100, y: 100 }, BAR, bounds),
            Point { x: 100, y: 100 }
        );
        assert_eq!(
            clamp(Point { x: -50, y: -9 }, BAR, bounds),
            Point { x: 0, y: 0 }
        );
        assert_eq!(
            clamp(Point { x: 1900, y: 1070 }, BAR, bounds),
            Point {
                x: 1920 - 220,
                y: 1080 - 38
            }
        );
    }

    fn two_monitors() -> Vec<Monitor> {
        // A 1920x1080 secondary to the left of the primary, so its coordinates are negative.
        vec![
            monitor(
                r"\\.\DISPLAY1",
                Rect::new(0, 0, 1920, 1080),
                Rect::new(0, 0, 1920, 1032),
                96,
                true,
            ),
            monitor(
                r"\\.\DISPLAY2",
                Rect::new(-1920, 0, 0, 1080),
                Rect::new(-1920, 0, 0, 1040),
                96,
                false,
            ),
        ]
    }

    fn centred_shell() -> Shell {
        Shell {
            centred: true,
            tray_notify: Some(Rect::new(1624, 1032, 1920, 1080)),
        }
    }

    #[test]
    fn wr08_ac5_no_saved_monitor_means_the_default_position_on_the_primary() {
        let monitors = two_monitors();
        let saved = Saved::default();
        let target = target_monitor(&saved, &monitors).expect("a monitor");
        assert_eq!(target.name, r"\\.\DISPLAY1");
        assert_eq!(
            position(&saved, target, &centred_shell(), BAR),
            Point { x: 12, y: 1037 }
        );
    }

    #[test]
    fn wr08_ac4_a_saved_position_is_restored_on_its_own_monitor() {
        let monitors = two_monitors();
        let saved = Saved {
            monitor: r"\\.\DISPLAY2".to_owned(),
            x: 300,
            y: 200,
        };
        let target = target_monitor(&saved, &monitors).expect("a monitor");
        assert_eq!(target.name, r"\\.\DISPLAY2");
        assert_eq!(
            position(&saved, target, &centred_shell(), BAR),
            Point {
                x: -1920 + 300,
                y: 200
            }
        );
    }

    #[test]
    fn wr08_ac5_a_removed_monitor_sends_the_bar_to_the_default_position_on_the_primary() {
        let mut monitors = two_monitors();
        monitors.remove(1);
        let saved = Saved {
            monitor: r"\\.\DISPLAY2".to_owned(),
            x: 300,
            y: 200,
        };
        let target = target_monitor(&saved, &monitors).expect("a monitor");
        assert_eq!(target.name, r"\\.\DISPLAY1");
        assert_eq!(
            position(&saved, target, &centred_shell(), BAR),
            Point { x: 12, y: 1037 },
            "the saved offset is for another monitor, so it is not used here"
        );
    }

    #[test]
    fn wr08_ac8_a_saved_position_that_no_longer_fits_is_moved_fully_on_screen() {
        // The monitor shrank from 4K to 1080p: the saved offset is now past its right edge.
        let monitors = two_monitors();
        let saved = Saved {
            monitor: r"\\.\DISPLAY1".to_owned(),
            x: 3500,
            y: 2000,
        };
        let target = target_monitor(&saved, &monitors).expect("a monitor");
        assert_eq!(
            position(&saved, target, &centred_shell(), BAR),
            Point {
                x: 1920 - 220,
                y: 1080 - 38
            }
        );
    }

    #[test]
    fn wr08_ac8_a_saved_position_follows_its_monitor_to_a_new_scale() {
        // 100 % saved at (300, 200) logical; the monitor is now at 150 %: the offset scales.
        let mut monitors = two_monitors();
        monitors[0].dpi = 144;
        monitors[0].bounds = Rect::new(0, 0, 2880, 1620);
        monitors[0].work = Rect::new(0, 0, 2880, 1548);
        let saved = Saved {
            monitor: r"\\.\DISPLAY1".to_owned(),
            x: 300,
            y: 200,
        };
        let target = target_monitor(&saved, &monitors).expect("a monitor");
        assert_eq!(
            position(&saved, target, &centred_shell(), size(330, 57)),
            Point { x: 450, y: 300 }
        );
    }

    #[test]
    fn target_monitor_falls_back_to_the_first_monitor_when_none_is_primary() {
        let mut monitors = two_monitors();
        monitors[0].primary = false;
        assert_eq!(
            target_monitor(&Saved::default(), &monitors).map(|m| m.name.as_str()),
            Some(r"\\.\DISPLAY1")
        );
        assert!(target_monitor(&Saved::default(), &[]).is_none());
    }

    #[test]
    fn saving_and_restoring_gives_the_same_place_at_each_scale() {
        for (dpi, bounds) in [
            (96, Rect::new(0, 0, 1920, 1080)),
            (144, Rect::new(0, 0, 2880, 1620)),
            (192, Rect::new(0, 0, 3840, 2160)),
        ] {
            let m = monitor("m", bounds, bounds, dpi, true);
            let bar = size(to_physical(220, dpi), to_physical(38, dpi));
            let at = Point {
                x: to_physical(417, dpi),
                y: to_physical(233, dpi),
            };
            let saved = to_saved(at, &m);
            assert_eq!((saved.x, saved.y), (417, 233), "{dpi} dpi");
            let shell = Shell {
                centred: true,
                tray_notify: None,
            };
            assert_eq!(position(&saved, &m, &shell, bar), at, "{dpi} dpi");
        }
    }

    #[test]
    fn monitor_at_finds_the_monitor_under_a_point_or_the_nearest() {
        let monitors = two_monitors();
        let name = |point| monitor_at(&monitors, point).map(|m| m.name.as_str());
        assert_eq!(name(Point { x: 10, y: 10 }), Some(r"\\.\DISPLAY1"));
        assert_eq!(name(Point { x: -10, y: 10 }), Some(r"\\.\DISPLAY2"));
        assert_eq!(name(Point { x: -1920, y: 0 }), Some(r"\\.\DISPLAY2"));
        // Past every edge: the nearest monitor.
        assert_eq!(name(Point { x: 5000, y: 5000 }), Some(r"\\.\DISPLAY1"));
        assert_eq!(name(Point { x: -5000, y: -5 }), Some(r"\\.\DISPLAY2"));
        assert!(monitor_at(&[], Point { x: 0, y: 0 }).is_none());
    }

    #[test]
    fn wr08_ac4_a_drag_keeps_the_bar_fully_on_the_monitor_under_the_pointer() {
        let monitors = two_monitors();
        // The pointer is on the left monitor but the bar was dragged half past its right edge:
        // it is pulled back inside that monitor, not left straddling the two.
        let (at, m) = drag_to(
            Point { x: -100, y: 500 },
            Point { x: -50, y: 510 },
            BAR,
            &monitors,
        )
        .expect("a monitor");
        assert_eq!(m.name, r"\\.\DISPLAY2");
        assert_eq!(at, Point { x: -220, y: 500 });
        // The pointer crosses to the primary monitor: the bar follows it there.
        let (at, m) = drag_to(
            Point { x: -5, y: 500 },
            Point { x: 40, y: 510 },
            BAR,
            &monitors,
        )
        .expect("a monitor");
        assert_eq!(m.name, r"\\.\DISPLAY1");
        assert_eq!(at, Point { x: 0, y: 500 });
    }

    #[test]
    fn wr08_ac4_a_drop_within_8_logical_px_of_the_taskbar_centre_snaps_to_it() {
        let (m, _) = windows_10(); // taskbar 1040..1080, centre line y = 1060
        // Bar centre at 1060 + 8 = 1068: top = 1068 - 19 = 1049.
        let snapped = snap(Point { x: 700, y: 1049 }, BAR, &m);
        assert_eq!(
            snapped,
            Point {
                x: 700,
                y: 1060 - 19
            }
        );
        // Bar centre at 1060 - 8.
        assert_eq!(snap(Point { x: 700, y: 1033 }, BAR, &m).y, 1060 - 19);
        // 9 px off the centre line: left where it was dropped.
        assert_eq!(snap(Point { x: 700, y: 1032 }, BAR, &m).y, 1032);
    }

    #[test]
    fn wr08_ac4_the_snap_distance_is_8_logical_px_at_150_and_200_percent() {
        for (dpi, reach, taskbar_height) in [(144, 12, 72), (192, 16, 96)] {
            let m = monitor(
                "hi",
                Rect::new(0, 0, 3840, 2160),
                Rect::new(0, 0, 3840, 2160 - taskbar_height),
                dpi,
                true,
            );
            let (_, taskbar) = taskbar_of(&m).expect("a taskbar");
            let line = i32::midpoint(taskbar.top, taskbar.bottom);
            let bar = size(300, 40);
            let at = |off: i32| Point {
                x: 100,
                y: line - bar.height / 2 + off,
            };
            assert_eq!(
                snap(at(reach), bar, &m).y,
                line - bar.height / 2,
                "{dpi} dpi, at the limit"
            );
            assert_eq!(
                snap(at(reach + 1), bar, &m).y,
                at(reach + 1).y,
                "{dpi} dpi, one past"
            );
        }
    }

    #[test]
    fn wr08_ac4_a_vertical_taskbar_snaps_on_its_horizontal_centre() {
        let m = monitor(
            "left",
            Rect::new(0, 0, 1920, 1080),
            Rect::new(60, 0, 1920, 1080), // taskbar x 0..60, centre line x = 30
            96,
            true,
        );
        // Bar centre x = 20 + 110 = 130: far off.
        assert_eq!(
            snap(Point { x: 20, y: 400 }, BAR, &m),
            Point { x: 20, y: 400 }
        );
        // Bar centre x = 30 + 8 = 38: top-left x = 38 - 110 = -72, clamped back onto the monitor.
        assert_eq!(
            snap(Point { x: -72, y: 400 }, BAR, &m),
            Point { x: 0, y: 400 }
        );
    }

    #[test]
    fn wr08_ac4_nothing_snaps_with_a_hidden_taskbar() {
        let m = monitor(
            "h",
            Rect::new(0, 0, 1920, 1080),
            Rect::new(0, 0, 1920, 1080),
            96,
            true,
        );
        assert_eq!(
            snap(Point { x: 10, y: 1040 }, BAR, &m),
            Point { x: 10, y: 1040 }
        );
    }

    #[test]
    fn wr08_ac4_dropping_on_a_second_monitors_taskbar_snaps_to_that_taskbar() {
        let monitors = two_monitors();
        let second = &monitors[1]; // taskbar y 1040..1080
        let snapped = snap(Point { x: -900, y: 1045 }, BAR, second);
        assert_eq!(
            snapped,
            Point {
                x: -900,
                y: 1060 - 19
            }
        );
    }

    #[test]
    fn wr02_ac1_the_popup_opens_above_the_bar_when_there_is_room() {
        let work = Rect::new(0, 0, 1920, 1032);
        let bar = Rect::new(500, 1037, 720, 1075);
        // Above the bar, but never over the taskbar: its bottom edge stops at the work area.
        assert_eq!(
            popup_position(bar, size(360, 520), work),
            Point {
                x: 500,
                y: 1032 - 520
            }
        );
        // A bar floating mid-desktop: directly above it.
        let floating = Rect::new(500, 700, 720, 738);
        assert_eq!(
            popup_position(floating, size(360, 520), work),
            Point {
                x: 500,
                y: 700 - 520
            }
        );
    }

    #[test]
    fn wr02_ac1_the_popup_opens_below_the_bar_when_there_is_no_room_above() {
        let work = Rect::new(0, 40, 1920, 1080);
        let bar = Rect::new(500, 1, 720, 39); // in a top taskbar
        assert_eq!(
            popup_position(bar, size(360, 520), work),
            Point { x: 500, y: 40 },
            "below the bar, and not under the taskbar"
        );
        let near_top = Rect::new(500, 300, 720, 338);
        assert_eq!(
            popup_position(near_top, size(360, 520), Rect::new(0, 0, 1920, 1080)),
            Point { x: 500, y: 338 }
        );
    }

    #[test]
    fn wr02_ac1_the_popup_is_left_aligned_with_the_bar_and_fully_on_screen() {
        let work = Rect::new(0, 0, 1920, 1032);
        // Near the right edge: pulled left until it fits.
        assert_eq!(
            popup_position(Rect::new(1800, 700, 1900, 738), size(360, 520), work).x,
            1920 - 360
        );
        // Off the left edge: pushed right.
        assert_eq!(
            popup_position(Rect::new(-30, 700, 70, 738), size(360, 520), work).x,
            0
        );
        // On a secondary monitor with negative coordinates.
        let second = Rect::new(-1920, 0, 0, 1040);
        assert_eq!(
            popup_position(Rect::new(-700, 900, -480, 938), size(360, 520), second),
            Point { x: -700, y: 380 }
        );
        // Taller than the work area: the top stays on screen.
        assert_eq!(
            popup_position(Rect::new(10, 200, 100, 238), size(360, 2000), work).y,
            0
        );
    }

    #[test]
    fn wr08_ac1_a_window_covering_the_whole_monitor_counts_as_full_screen() {
        let monitor = Rect::new(0, 0, 1920, 1080);
        assert!(covers_monitor(Rect::new(0, 0, 1920, 1080), monitor));
        assert!(
            covers_monitor(Rect::new(-8, -8, 1928, 1088), monitor),
            "a maximised window overhangs by its border"
        );
        assert!(
            !covers_monitor(Rect::new(0, 0, 1920, 1032), monitor),
            "a maximised window leaves the taskbar"
        );
        assert!(!covers_monitor(Rect::new(1, 0, 1920, 1080), monitor));
        assert!(!covers_monitor(Rect::new(100, 100, 900, 700), monitor));
    }
}
