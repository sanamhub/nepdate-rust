//! The mini bar's text (WR-08 AC2, ADR-0019 point 9). Pure: the settings and a `Zoned` go in, two
//! line strings come out. The Win32 module only draws what this returns.

use nepdate::{Lang, LongDate, NepaliDate};

use crate::app::settings::Settings;
use crate::app::{clock, format, holidays};

/// Between two fields on a line. The renderer draws it in `ink-muted` (widget-design §5).
pub const SEPARATOR: &str = " · ";

/// One thing a bar line can show (WR-08 AC2). The ids are the values of `bar_top` and
/// `bar_bottom` in the settings file.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BarField {
    Weekday,
    WeekdayLong,
    Time,
    ZoneTime,
    BsDate,
    BsShort,
    AdDate,
    AdShort,
    Tithi,
    Holiday,
}

impl BarField {
    /// Every field, in the order of the WR-08 AC2 table.
    pub const ALL: [BarField; 10] = [
        BarField::Weekday,
        BarField::WeekdayLong,
        BarField::Time,
        BarField::ZoneTime,
        BarField::BsDate,
        BarField::BsShort,
        BarField::AdDate,
        BarField::AdShort,
        BarField::Tithi,
        BarField::Holiday,
    ];

    /// The id used in the settings file.
    pub const fn id(self) -> &'static str {
        match self {
            BarField::Weekday => "weekday",
            BarField::WeekdayLong => "weekday_long",
            BarField::Time => "time",
            BarField::ZoneTime => "zone_time",
            BarField::BsDate => "bs_date",
            BarField::BsShort => "bs_short",
            BarField::AdDate => "ad_date",
            BarField::AdShort => "ad_short",
            BarField::Tithi => "tithi",
            BarField::Holiday => "holiday",
        }
    }

    /// The field with this id, or `None` for an unknown one.
    pub fn from_id(id: &str) -> Option<BarField> {
        BarField::ALL.into_iter().find(|field| field.id() == id)
    }
}

/// The top line of a fresh install (WR-08 AC2).
pub const DEFAULT_TOP: &[BarField] = &[BarField::Weekday, BarField::Time, BarField::ZoneTime];
/// The bottom line of a fresh install (WR-08 AC2).
pub const DEFAULT_BOTTOM: &[BarField] = &[BarField::BsDate, BarField::AdShort];

/// The settings-file ids of `fields`.
pub fn ids(fields: &[BarField]) -> Vec<String> {
    fields.iter().map(|field| field.id().to_owned()).collect()
}

/// The two lines the bar draws. A line with nothing to show is an empty string.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct BarLines {
    pub top: String,
    pub bottom: String,
}

impl BarLines {
    /// Whether there is nothing to draw.
    #[cfg_attr(not(windows), allow(dead_code))]
    pub fn is_empty(&self) -> bool {
        self.top.is_empty() && self.bottom.is_empty()
    }
}

/// What a field needs: the clock, the settings and the local BS date. The weekday, the BS date and
/// the AD date all come from the same local date (WR-05 AC4).
struct Day<'a> {
    now: &'a jiff::Zoned,
    settings: &'a Settings,
    lang: Lang,
    /// `None` when the local date is outside 1901..=2199, which only happens with a wildly wrong
    /// system clock; the date fields are then empty and the time fields still work.
    bs: Option<NepaliDate>,
}

/// The bar's two lines for `now` (WR-08 AC2): each line is its fields in order, joined with
/// ` · `, with an empty field left out together with its separator.
pub fn bar_lines(now: &jiff::Zoned, s: &Settings) -> BarLines {
    let day = Day {
        now,
        settings: s,
        lang: s.lang(),
        bs: NepaliDate::try_from(now.date()).ok(),
    };
    BarLines {
        top: line(&s.bar_top, &day),
        bottom: line(&s.bar_bottom, &day),
    }
}

fn line(ids: &[String], day: &Day<'_>) -> String {
    let texts: Vec<String> = ids
        .iter()
        .filter_map(|id| BarField::from_id(id))
        .map(|field| field_text(field, day))
        .filter(|text| !text.is_empty())
        .collect();
    texts.join(SEPARATOR)
}

/// The text of one field, or an empty string when it has nothing to show.
fn field_text(field: BarField, day: &Day<'_>) -> String {
    let lang = day.lang;
    match field {
        BarField::Time => clock::local_time(day.now, day.settings),
        BarField::ZoneTime => clock::clock_lines(day.now, day.settings)
            .into_iter()
            .find_map(|line| line.zone_city.map(|city| format!("{city} {}", line.time)))
            .unwrap_or_default(),
        _ => day
            .bs
            .map_or_else(String::new, |bs| date_text(field, bs, lang)),
    }
}

/// The fields that read from the BS date.
fn date_text(field: BarField, bs: NepaliDate, lang: Lang) -> String {
    match field {
        BarField::Weekday => bs.weekday().short_name(lang).to_owned(),
        BarField::WeekdayLong => bs.weekday().name(lang).to_owned(),
        BarField::BsDate => bs_long(bs, true, lang),
        BarField::BsShort => bs_long(bs, false, lang),
        BarField::AdDate => format::ad_day_month_year(bs.to_gregorian(), lang),
        BarField::AdShort => format::ad_month_day(bs.to_gregorian(), lang),
        BarField::Tithi => nepdate_patro::info(bs)
            .tithi()
            .map_or_else(String::new, |tithi| tithi.name(lang).to_owned()),
        BarField::Holiday => {
            holidays::holiday_name(bs, lang).map_or_else(String::new, str::to_owned)
        }
        BarField::Time | BarField::ZoneTime => String::new(),
    }
}

fn bs_long(bs: NepaliDate, year: bool, lang: Lang) -> String {
    bs.long(
        LongDate {
            weekday: false,
            year,
            pad: false,
        },
        lang,
    )
    .to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 2083 Kartik 5 is 2026-10-22, a Thursday (the WR-08 AC2 table). Local time is Kathmandu.
    fn thursday() -> jiff::Zoned {
        "2026-10-22T18:31:00+05:45[Asia/Kathmandu]"
            .parse()
            .expect("fixed Zoned")
    }

    fn settings(language: &str, top: &[BarField], bottom: &[BarField]) -> Settings {
        Settings {
            language: language.to_owned(),
            extra_time_zone: "Asia/Singapore".to_owned(),
            bar_top: ids(top),
            bar_bottom: ids(bottom),
            ..Settings::default()
        }
    }

    /// Every field of the WR-08 AC2 table, in en and ne, one at a time.
    #[test]
    fn wr08_ac2_every_field_in_en_and_ne() {
        let now = thursday();
        let cases = [
            (BarField::Weekday, "Thu", "बिहि"),
            (BarField::WeekdayLong, "Thursday", "बिहिवार"),
            (BarField::Time, "6:31 PM", "६:३१ PM"),
            (BarField::ZoneTime, "Singapore 8:46 PM", "Singapore ८:४६ PM"),
            (BarField::BsDate, "Kartik 5, 2083", "कार्तिक ५, २०८३"),
            (BarField::BsShort, "Kartik 5", "कार्तिक ५"),
            (BarField::AdDate, "Oct 22, 2026", "Oct २२, २०२६"),
            (BarField::AdShort, "Oct 22", "Oct २२"),
            (BarField::Tithi, "Ekadashi", "एकादशी"),
        ];
        for (field, en, ne) in cases {
            for (language, expected) in [("en", en), ("ne", ne)] {
                let s = settings(language, &[field], &[]);
                assert_eq!(
                    bar_lines(&now, &s).top,
                    expected,
                    "{} in {language}",
                    field.id()
                );
            }
        }
    }

    #[test]
    fn wr08_ac2_the_holiday_field_names_todays_public_holiday() {
        // 2081 Baishakh 1 is the New Year, a public holiday; the day after is not one.
        let holiday: jiff::Zoned = "2024-04-13T09:00:00+05:45[Asia/Kathmandu]"
            .parse()
            .expect("fixed Zoned");
        let ordinary: jiff::Zoned = "2024-04-14T09:00:00+05:45[Asia/Kathmandu]"
            .parse()
            .expect("fixed Zoned");
        let en = settings("en", &[BarField::Holiday], &[]);
        let ne = settings("ne", &[BarField::Holiday], &[]);
        assert_eq!(bar_lines(&holiday, &en).top, "New Year");
        assert_eq!(bar_lines(&holiday, &ne).top, "नयाँ वर्ष");
        assert_eq!(bar_lines(&ordinary, &en).top, "");
    }

    #[test]
    fn wr08_ac2_the_defaults_string() {
        let s = settings("en", DEFAULT_TOP, DEFAULT_BOTTOM);
        let lines = bar_lines(&thursday(), &s);
        assert_eq!(lines.top, "Thu · 6:31 PM · Singapore 8:46 PM");
        assert_eq!(lines.bottom, "Kartik 5, 2083 · Oct 22");
        assert_eq!(Settings::default().bar_top, ids(DEFAULT_TOP));
        assert_eq!(Settings::default().bar_bottom, ids(DEFAULT_BOTTOM));
    }

    #[test]
    fn wr08_ac2_the_defaults_string_in_nepali() {
        let s = settings("ne", DEFAULT_TOP, DEFAULT_BOTTOM);
        let lines = bar_lines(&thursday(), &s);
        assert_eq!(lines.top, "बिहि · ६:३१ PM · Singapore ८:४६ PM");
        assert_eq!(lines.bottom, "कार्तिक ५, २०८३ · Oct २२");
    }

    #[test]
    fn wr08_ac2_an_empty_field_is_left_out_with_its_separator() {
        // The extra zone has the local offset, so `zone_time` is empty (WR-05 AC1).
        let mut s = settings("en", DEFAULT_TOP, DEFAULT_BOTTOM);
        s.extra_time_zone = "Asia/Kathmandu".to_owned();
        assert_eq!(bar_lines(&thursday(), &s).top, "Thu · 6:31 PM");

        // No extra clock at all.
        s.extra_time_zone = String::new();
        assert_eq!(bar_lines(&thursday(), &s).top, "Thu · 6:31 PM");

        // An empty field in the middle and at the start.
        let outside: jiff::Zoned = "2070-01-01T09:00:00+05:45[Asia/Kathmandu]"
            .parse()
            .expect("fixed Zoned");
        let s = settings(
            "en",
            &[BarField::Tithi, BarField::Weekday, BarField::Holiday],
            &[BarField::Weekday, BarField::Tithi, BarField::AdShort],
        );
        let lines = bar_lines(&outside, &s);
        assert_eq!(
            lines.top, "Wed",
            "tithi and holiday are empty outside the patro data"
        );
        assert_eq!(lines.bottom, "Wed · Jan 1");
    }

    #[test]
    fn wr08_ac2_a_line_of_only_empty_fields_is_empty() {
        let s = settings(
            "en",
            &[BarField::Holiday, BarField::Tithi],
            &[BarField::Holiday],
        );
        let outside: jiff::Zoned = "2070-01-01T09:00:00+05:45[Asia/Kathmandu]"
            .parse()
            .expect("fixed Zoned");
        let lines = bar_lines(&outside, &s);
        assert_eq!(lines, BarLines::default());
        assert!(lines.is_empty());
    }

    #[test]
    fn wr08_ac2_top_empty_leaves_only_the_bottom_line() {
        let s = settings("en", &[], DEFAULT_BOTTOM);
        let lines = bar_lines(&thursday(), &s);
        assert_eq!(lines.top, "");
        assert_eq!(lines.bottom, "Kartik 5, 2083 · Oct 22");
        assert!(!lines.is_empty());
    }

    #[test]
    fn wr08_ac2_both_lines_are_built_from_their_own_fields() {
        let s = settings(
            "en",
            &[BarField::BsShort, BarField::Time],
            &[BarField::WeekdayLong, BarField::AdDate],
        );
        let lines = bar_lines(&thursday(), &s);
        assert_eq!(lines.top, "Kartik 5 · 6:31 PM");
        assert_eq!(lines.bottom, "Thursday · Oct 22, 2026");
    }

    #[test]
    fn wr05_ac4_the_weekday_follows_the_bs_date_shown_next_to_it() {
        // At 23:59 it is still Thursday Kartik 5; a minute later it is Friday Kartik 6.
        let s = settings("en", &[BarField::Weekday], &[BarField::BsShort]);
        let before: jiff::Zoned = "2026-10-22T23:59:30+05:45[Asia/Kathmandu]"
            .parse()
            .expect("fixed Zoned");
        let after: jiff::Zoned = "2026-10-23T00:00:30+05:45[Asia/Kathmandu]"
            .parse()
            .expect("fixed Zoned");
        let b = bar_lines(&before, &s);
        let a = bar_lines(&after, &s);
        assert_eq!((b.top.as_str(), b.bottom.as_str()), ("Thu", "Kartik 5"));
        assert_eq!((a.top.as_str(), a.bottom.as_str()), ("Fri", "Kartik 6"));
    }

    #[test]
    fn wr08_ac2_the_time_follows_the_clock_settings() {
        let mut s = settings("en", &[BarField::Time, BarField::ZoneTime], &[]);
        s.clock_24h = true;
        s.show_seconds = true;
        assert_eq!(
            bar_lines(&thursday(), &s).top,
            "18:31:00 · Singapore 20:46:00"
        );
    }

    #[test]
    fn a_date_outside_the_range_leaves_the_date_fields_empty_and_keeps_the_time() {
        let far: jiff::Zoned = "1800-01-01T09:00:00+00:00[UTC]"
            .parse()
            .expect("fixed Zoned");
        let s = settings(
            "en",
            &[BarField::Weekday, BarField::Time],
            &[BarField::BsDate],
        );
        let lines = bar_lines(&far, &s);
        assert_eq!(lines.top, "9:00 AM");
        assert_eq!(lines.bottom, "");
    }

    #[test]
    fn every_field_id_round_trips_and_an_unknown_id_is_none() {
        for field in BarField::ALL {
            assert_eq!(BarField::from_id(field.id()), Some(field));
        }
        assert_eq!(BarField::from_id("sunrise"), None);
        assert_eq!(BarField::from_id(""), None);
        let ids: Vec<&str> = BarField::ALL.iter().map(|field| field.id()).collect();
        let mut unique = ids.clone();
        unique.sort_unstable();
        unique.dedup();
        assert_eq!(unique.len(), ids.len(), "ids are unique");
    }
}
