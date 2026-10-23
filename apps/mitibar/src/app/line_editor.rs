//! The bar line editor's operations (WR-08 AC3). Pure: each one changes the two field lists of the
//! settings and says where the chip ended up, so the UI can keep the focus on it.

use crate::app::bar::{BarField, DEFAULT_BOTTOM, DEFAULT_TOP, ids};
use crate::app::settings::Settings;

/// One of the bar's two lines.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Line {
    Top,
    Bottom,
}

impl Line {
    /// The line a chip leaves and the one it reaches with Alt+Up (`true`) or Alt+Down.
    const fn across(up: bool) -> (Line, Line) {
        if up {
            (Line::Bottom, Line::Top)
        } else {
            (Line::Top, Line::Bottom)
        }
    }
}

/// Where a chip is after a change.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct At {
    pub line: Line,
    pub index: usize,
}

fn list(settings: &Settings, line: Line) -> &Vec<String> {
    match line {
        Line::Top => &settings.bar_top,
        Line::Bottom => &settings.bar_bottom,
    }
}

fn list_mut(settings: &mut Settings, line: Line) -> &mut Vec<String> {
    match line {
        Line::Top => &mut settings.bar_top,
        Line::Bottom => &mut settings.bar_bottom,
    }
}

/// The fields on neither line, in the order of the WR-08 AC2 table: what "Add field" offers.
pub fn unused(settings: &Settings) -> Vec<BarField> {
    BarField::ALL
        .into_iter()
        .filter(|field| {
            !settings
                .bar_top
                .iter()
                .chain(&settings.bar_bottom)
                .any(|id| id == field.id())
        })
        .collect()
}

/// Whether `line` has only one chip and the other line has none. Its remove button is disabled,
/// so at least one field stays.
pub fn is_last_chip(settings: &Settings) -> bool {
    settings.bar_top.len() + settings.bar_bottom.len() <= 1
}

/// Adds `field` at the end of `line`. Returns where it went, or `None` when the field is already
/// on a line: a field appears at most once.
pub fn add(settings: &mut Settings, line: Line, field: BarField) -> Option<At> {
    if !unused(settings).contains(&field) {
        return None;
    }
    let fields = list_mut(settings, line);
    fields.push(field.id().to_owned());
    Some(At {
        line,
        index: fields.len() - 1,
    })
}

/// Removes the chip at `index`. Returns `false` and changes nothing when it is the last chip or
/// there is no such chip.
pub fn remove(settings: &mut Settings, line: Line, index: usize) -> bool {
    if is_last_chip(settings) || index >= list(settings, line).len() {
        return false;
    }
    list_mut(settings, line).remove(index);
    true
}

/// Moves the chip one place left (`delta` -1) or right (1) on its line. Returns where it is now,
/// or `None` at the end of the line.
pub fn move_within(settings: &mut Settings, line: Line, index: usize, delta: i32) -> Option<At> {
    let fields = list_mut(settings, line);
    let target = match delta {
        -1 => index.checked_sub(1)?,
        1 => index + 1,
        _ => return None,
    };
    if index >= fields.len() || target >= fields.len() {
        return None;
    }
    fields.swap(index, target);
    Some(At {
        line,
        index: target,
    })
}

/// Moves the chip to the other line: Alt+Up takes a bottom chip to the top line, Alt+Down a top
/// chip to the bottom one. It lands at the same place, or at the end of a shorter line. Returns
/// where it is now, or `None` when it is already on the line it would move to.
pub fn move_across(settings: &mut Settings, line: Line, index: usize, up: bool) -> Option<At> {
    let (from, to) = Line::across(up);
    if line != from || index >= list(settings, from).len() {
        return None;
    }
    let field = list_mut(settings, from).remove(index);
    let target = list_mut(settings, to);
    let at = index.min(target.len());
    target.insert(at, field);
    Some(At {
        line: to,
        index: at,
    })
}

/// "Reset to default": the two lines of WR-08 AC2.
pub fn reset(settings: &mut Settings) {
    settings.bar_top = ids(DEFAULT_TOP);
    settings.bar_bottom = ids(DEFAULT_BOTTOM);
}

#[cfg(test)]
mod tests {
    use super::*;

    fn lines(settings: &Settings) -> (Vec<&str>, Vec<&str>) {
        (
            settings.bar_top.iter().map(String::as_str).collect(),
            settings.bar_bottom.iter().map(String::as_str).collect(),
        )
    }

    fn with(top: &[&str], bottom: &[&str]) -> Settings {
        Settings {
            bar_top: top.iter().map(|id| (*id).to_owned()).collect(),
            bar_bottom: bottom.iter().map(|id| (*id).to_owned()).collect(),
            ..Settings::default()
        }
    }

    #[test]
    fn the_defaults_leave_five_fields_to_add() {
        let settings = Settings::default();
        let offered: Vec<&str> = unused(&settings).iter().map(|field| field.id()).collect();
        assert_eq!(
            offered,
            ["weekday_long", "bs_short", "ad_date", "tithi", "holiday"]
        );
    }

    #[test]
    fn a_field_can_be_added_once_to_either_line() {
        let mut settings = Settings::default();
        assert_eq!(
            add(&mut settings, Line::Top, BarField::Tithi),
            Some(At {
                line: Line::Top,
                index: 3
            })
        );
        assert_eq!(
            add(&mut settings, Line::Bottom, BarField::Tithi),
            None,
            "it is on the top line now"
        );
        assert_eq!(
            add(&mut settings, Line::Bottom, BarField::Time),
            None,
            "a field in use is not offered"
        );
        assert_eq!(
            add(&mut settings, Line::Bottom, BarField::Holiday),
            Some(At {
                line: Line::Bottom,
                index: 2
            })
        );
        assert_eq!(
            lines(&settings),
            (
                vec!["weekday", "time", "zone_time", "tithi"],
                vec!["bs_date", "ad_short", "holiday"]
            )
        );
    }

    #[test]
    fn every_field_in_use_leaves_nothing_to_add() {
        let mut settings = Settings::default();
        for field in unused(&settings) {
            assert!(add(&mut settings, Line::Bottom, field).is_some());
        }
        assert!(unused(&settings).is_empty());
        assert_eq!(settings.bar_bottom.len() + settings.bar_top.len(), 10);
    }

    #[test]
    fn a_chip_is_removed_from_its_line() {
        let mut settings = Settings::default();
        assert!(remove(&mut settings, Line::Top, 1));
        assert_eq!(
            lines(&settings),
            (vec!["weekday", "zone_time"], vec!["bs_date", "ad_short"])
        );
        assert!(!remove(&mut settings, Line::Top, 5), "no such chip");
        assert!(unused(&settings).iter().any(|field| field.id() == "time"));
    }

    #[test]
    fn wr08_ac3_the_last_chip_cannot_be_removed() {
        let mut settings = with(&["time"], &[]);
        assert!(is_last_chip(&settings));
        assert!(!remove(&mut settings, Line::Top, 0));
        assert_eq!(lines(&settings), (vec!["time"], vec![]));

        // One chip on each line is not the last one.
        let mut two = with(&["time"], &["bs_date"]);
        assert!(!is_last_chip(&two));
        assert!(remove(&mut two, Line::Top, 0));
        assert_eq!(lines(&two), (vec![], vec!["bs_date"]));
        assert!(is_last_chip(&two));
        assert!(!remove(&mut two, Line::Bottom, 0));
    }

    #[test]
    fn a_chip_moves_one_place_along_its_line_and_stops_at_the_ends() {
        let mut settings = Settings::default();
        assert_eq!(
            move_within(&mut settings, Line::Top, 0, 1),
            Some(At {
                line: Line::Top,
                index: 1
            })
        );
        assert_eq!(settings.bar_top, ["time", "weekday", "zone_time"]);
        assert_eq!(
            move_within(&mut settings, Line::Top, 2, -1),
            Some(At {
                line: Line::Top,
                index: 1
            })
        );
        assert_eq!(settings.bar_top, ["time", "zone_time", "weekday"]);
        assert_eq!(
            move_within(&mut settings, Line::Top, 0, -1),
            None,
            "left end"
        );
        assert_eq!(
            move_within(&mut settings, Line::Top, 2, 1),
            None,
            "right end"
        );
        assert_eq!(
            move_within(&mut settings, Line::Top, 9, 1),
            None,
            "no such chip"
        );
        assert_eq!(
            move_within(&mut settings, Line::Top, 0, 2),
            None,
            "one place at a time"
        );
        assert_eq!(settings.bar_top, ["time", "zone_time", "weekday"]);
    }

    #[test]
    fn a_chip_moves_to_the_other_line_at_the_same_place_or_the_end() {
        let mut settings = Settings::default();
        // Alt+Down on the top line's last chip: the bottom line has two, so it goes to the end.
        assert_eq!(
            move_across(&mut settings, Line::Top, 2, false),
            Some(At {
                line: Line::Bottom,
                index: 2
            })
        );
        assert_eq!(
            lines(&settings),
            (
                vec!["weekday", "time"],
                vec!["bs_date", "ad_short", "zone_time"]
            )
        );
        // Alt+Up on the bottom line's first chip: it takes the same place above.
        assert_eq!(
            move_across(&mut settings, Line::Bottom, 0, true),
            Some(At {
                line: Line::Top,
                index: 0
            })
        );
        assert_eq!(
            lines(&settings),
            (
                vec!["bs_date", "weekday", "time"],
                vec!["ad_short", "zone_time"]
            )
        );
    }

    #[test]
    fn a_chip_cannot_move_off_the_top_or_bottom_line() {
        let mut settings = Settings::default();
        assert_eq!(move_across(&mut settings, Line::Top, 0, true), None);
        assert_eq!(move_across(&mut settings, Line::Bottom, 0, false), None);
        assert_eq!(move_across(&mut settings, Line::Top, 7, false), None);
        assert_eq!(settings, Settings::default());
    }

    #[test]
    fn moving_the_last_chip_of_a_line_leaves_that_line_empty() {
        let mut settings = with(&["time"], &["bs_date"]);
        assert!(move_across(&mut settings, Line::Top, 0, false).is_some());
        assert_eq!(lines(&settings), (vec![], vec!["time", "bs_date"]));
    }

    #[test]
    fn reset_restores_the_wr08_ac2_defaults() {
        let mut settings = with(&["tithi"], &["holiday", "ad_date"]);
        reset(&mut settings);
        assert_eq!(settings.bar_top, Settings::default().bar_top);
        assert_eq!(settings.bar_bottom, Settings::default().bar_bottom);
    }
}
