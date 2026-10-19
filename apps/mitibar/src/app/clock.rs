//! Clock text for the local and extra time zone (WR-05), plus the popup header (WR-02 AC3).
//! Pure: takes a fixed `jiff::Zoned` so every function is testable without a clock (ADR-0015).

use std::fmt::Write as _;

use nepdate::{Lang, LongDate, NepaliDate};

use crate::app::format;
use crate::app::settings::Settings;

/// One clock line for the popup. `zone_city` is `None` for the local line; the UI shows the
/// `@tr("Local")` label there. For the extra zone it is the city part of the IANA id, shown as
/// `{city} {offset}`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ClockLine {
    pub zone_city: Option<String>,
    pub time: String,
    pub offset: String,
}

/// The popup header: the BS long date, its weekday (the same date, WR-05 AC4) and the AD date.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Header {
    pub bs: String,
    pub weekday: String,
    pub ad: String,
}

/// One or two clock lines (WR-05 AC1): local always, the extra zone only when its UTC offset
/// differs from the local offset at this instant.
pub fn clock_lines(now: &jiff::Zoned, s: &Settings) -> Vec<ClockLine> {
    let lang = s.lang();
    let mut lines = vec![ClockLine {
        zone_city: None,
        time: format_time(now.time(), s.clock_24h, s.show_seconds, lang),
        offset: String::new(),
    }];
    if s.extra_time_zone.is_empty() {
        return lines;
    }
    let Ok(zone) = jiff::tz::TimeZone::get(&s.extra_time_zone) else {
        // An unknown zone id is a settings error (WR-06 AC2); show the local line only.
        return lines;
    };
    let extra = now.with_time_zone(zone);
    if extra.offset() != now.offset() {
        lines.push(ClockLine {
            zone_city: Some(city_of(&s.extra_time_zone)),
            time: format_time(extra.time(), s.clock_24h, s.show_seconds, lang),
            offset: extra.offset().to_string(),
        });
    }
    lines
}

/// The local time string for `now`, per WR-05 AC2. The taskbar text (WR-12 AC1) reuses this so
/// the two surfaces never format the time differently.
pub fn local_time(now: &jiff::Zoned, s: &Settings) -> String {
    format_time(now.time(), s.clock_24h, s.show_seconds, s.lang())
}

/// The time until the next wake-up: the next second boundary when seconds are shown, the next
/// minute boundary otherwise, both plus 50 ms (WR-05 AC5). The 50 ms makes the wake land just
/// after the boundary so the clock reads the new minute or second.
pub fn next_wakeup(now: &jiff::Zoned, show_seconds: bool) -> std::time::Duration {
    let sec = now.time().second(); // 0..=59
    let nanos = now.timestamp().subsec_nanosecond(); // 0..1_000_000_000
    if show_seconds {
        // nanos is 0..1_000_000_000, so the subtraction never underflows and the u64 never fails.
        let until_second = u64::try_from(1_000_000_000 - nanos).unwrap_or(0);
        std::time::Duration::from_nanos(until_second) + std::time::Duration::from_millis(50)
    } else {
        let secs = u64::try_from(60 - sec).unwrap_or(60); // 1..=60
        std::time::Duration::from_secs(secs).saturating_sub(std::time::Duration::from_nanos(
            u64::try_from(nanos).unwrap_or(0),
        )) + std::time::Duration::from_millis(50)
    }
}

/// The header for the local date in `now` (WR-02 AC3, WR-05 AC4). The BS date and the weekday
/// both come from `now.date()`. Empty when the local date is outside the supported BS range
/// (1901..=2199), which only happens with a wildly wrong system clock.
pub fn header(now: &jiff::Zoned, lang: Lang) -> Header {
    let Ok(bs) = NepaliDate::try_from(now.date()) else {
        return Header::default();
    };
    Header {
        bs: bs
            .long(
                LongDate {
                    weekday: false,
                    year: true,
                    pad: false,
                },
                lang,
            )
            .to_string(),
        weekday: bs.weekday().name(lang).to_owned(),
        ad: format::ad_short(bs.to_gregorian(), lang),
    }
}

/// The city part of an IANA id (`America/Argentina/Buenos_Aires` -> `Buenos Aires`), WR-05 AC3.
fn city_of(zone_id: &str) -> String {
    zone_id
        .rsplit('/')
        .next()
        .unwrap_or(zone_id)
        .replace('_', " ")
}

/// Formats a time per WR-05 AC2: 12 h `2:05 PM` / 24 h `14:05`, optional `:SS`, digits localised
/// with the `nepdate` `Digits` formatter. The AM/PM label stays Latin in both languages.
fn format_time(time: jiff::civil::Time, clock_24h: bool, show_seconds: bool, lang: Lang) -> String {
    let mut out = String::new();
    if clock_24h {
        push_digits(&mut out, time.hour(), 2, lang);
        out.push(':');
        push_digits(&mut out, time.minute(), 2, lang);
        if show_seconds {
            out.push(':');
            push_digits(&mut out, time.second(), 2, lang);
        }
    } else {
        let (hour, am_pm) = to_12h(time.hour());
        push_digits(&mut out, hour, 1, lang);
        out.push(':');
        push_digits(&mut out, time.minute(), 2, lang);
        if show_seconds {
            out.push(':');
            push_digits(&mut out, time.second(), 2, lang);
        }
        out.push(' ');
        out.push_str(am_pm);
    }
    out
}

/// 24 h hour -> (12 h hour, AM/PM).
fn to_12h(hour: i8) -> (i8, &'static str) {
    match hour {
        0 => (12, "AM"),
        1..=11 => (hour, "AM"),
        12 => (12, "PM"),
        _ => (hour - 12, "PM"), // 13..=23
    }
}

/// Writes `n` with `width` digits (leading zeros) into `out`, in `lang`'s digit set.
fn push_digits(out: &mut String, n: i8, width: u8, lang: Lang) {
    // Hour/minute/second are 0..=59 and the 12 h hour is 1..=12, so the conversion never fails.
    let value = u32::try_from(n).unwrap_or(0);
    let _ = write!(out, "{}", lang.digits(value, width));
}

#[cfg(test)]
mod tests {
    use super::*;

    fn kathmandu(ts: &str) -> jiff::Zoned {
        ts.parse().expect("valid fixed Zoned")
    }

    #[test]
    fn wr05_ac1_hides_extra_zone_with_same_offset() {
        let now = kathmandu("2026-09-24T12:00:00+05:45[Asia/Kathmandu]");
        // Same zone as local: same offset, hidden.
        let same = Settings {
            extra_time_zone: "Asia/Kathmandu".into(),
            ..Settings::default()
        };
        let lines = clock_lines(&now, &same);
        assert_eq!(lines.len(), 1);
        assert_eq!(lines[0].zone_city, None);

        // Kolkata is +05:30, not +05:45: shown as a second line.
        let other = Settings {
            extra_time_zone: "Asia/Kolkata".into(),
            ..Settings::default()
        };
        let lines = clock_lines(&now, &other);
        assert_eq!(lines.len(), 2);
        assert_eq!(lines[1].zone_city.as_deref(), Some("Kolkata"));
        assert_eq!(lines[1].offset, "+05:30");
    }

    #[test]
    fn wr05_ac2_12h_and_24h_and_seconds() {
        let now = kathmandu("2026-09-24T14:05:07+05:45[Asia/Kathmandu]");
        let cases = [
            (
                Settings {
                    clock_24h: false,
                    show_seconds: false,
                    ..Settings::default()
                },
                "2:05 PM",
            ),
            (
                Settings {
                    clock_24h: true,
                    show_seconds: false,
                    ..Settings::default()
                },
                "14:05",
            ),
            (
                Settings {
                    clock_24h: false,
                    show_seconds: true,
                    ..Settings::default()
                },
                "2:05:07 PM",
            ),
            (
                Settings {
                    clock_24h: true,
                    show_seconds: true,
                    ..Settings::default()
                },
                "14:05:07",
            ),
        ];
        for (s, expected) in cases {
            assert_eq!(clock_lines(&now, &s)[0].time, expected, "{s:?}");
        }
        // Nepali digits, 12 h, no seconds.
        let ne = Settings {
            language: "ne".into(),
            ..Settings::default()
        };
        assert_eq!(clock_lines(&now, &ne)[0].time, "२:०५ PM");
    }

    #[test]
    fn wr05_ac3_label_is_city_with_offset() {
        let now = kathmandu("2026-09-24T12:00:00+05:30[Asia/Kolkata]");
        let s = Settings {
            extra_time_zone: "Asia/Kathmandu".into(),
            ..Settings::default()
        };
        let lines = clock_lines(&now, &s);
        assert_eq!(lines[1].zone_city.as_deref(), Some("Kathmandu"));
        assert_eq!(lines[1].offset, "+05:45");

        let ba = Settings {
            extra_time_zone: "America/Argentina/Buenos_Aires".into(),
            ..Settings::default()
        };
        let lines = clock_lines(&now, &ba);
        assert_eq!(lines[1].zone_city.as_deref(), Some("Buenos Aires"));
    }

    #[test]
    fn wr05_ac4_weekday_matches_bs_date() {
        // 2026-09-24 is Ashoj 8, 2083, a Thursday (devanagari-test-strings.md lines 10 and 11).
        let now = kathmandu("2026-09-24T23:59:30+05:45[Asia/Kathmandu]");
        let h = header(&now, Lang::En);
        assert_eq!(h.bs, "Ashoj 8, 2083");
        assert_eq!(h.weekday, "Thursday");
        assert_eq!(h.ad, "2026-09-24");
        // The AD date follows the language's digit set (widget-design §4); the separator stays
        // ASCII so the string is still a date.
        let ne = header(&now, Lang::Ne);
        assert_eq!(ne.bs, "असोज ८, २०८३");
        assert_eq!(ne.weekday, "बिहिवार");
        assert_eq!(ne.ad, "२०२६-०९-२४");
        // The weekday string is the weekday of the BS date, not of a shifted zone.
        assert_eq!(
            h.weekday,
            NepaliDate::try_from(now.date())
                .unwrap()
                .weekday()
                .name(Lang::En)
        );
    }

    #[test]
    fn wr05_ac5_wakeup_is_next_minute_plus_50ms() {
        let now = kathmandu("2026-09-24T23:59:30+05:45[Asia/Kathmandu]");
        assert_eq!(
            next_wakeup(&now, false),
            std::time::Duration::from_millis(30_050)
        );
        // Seconds shown: wake at the next second boundary (+50 ms).
        assert_eq!(
            next_wakeup(&now, true),
            std::time::Duration::from_millis(1050)
        );
    }
}
