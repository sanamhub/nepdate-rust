//! Every supported day, checked against an independent copy of ALGORITHM §4 and §5.
#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing,
    clippy::many_single_char_names
)]

mod common;

use nepdate::{GregorianDate, NepaliDate};

/// ALGORITHM §2, §4 and §5 copied line by line, over month lengths read from the JSON with a
/// small hand-written reader, so nothing here shares code or tables with the library.
#[allow(
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    clippy::cast_possible_wrap,
    clippy::wrong_self_convention
)] // the spec's names and integer types, copied as written
mod reference {
    pub(crate) const MIN_YEAR: i64 = 1901;
    pub(crate) const EPOCH_UNIX_DAYS: i64 = -45_920;

    pub(crate) struct Calendar {
        /// `LEN[y - MIN_YEAR][m - 1]`.
        len: Vec<[i64; 12]>,
        /// `YEAR_START[i]`, with one extra entry for the end.
        year_start: Vec<i64>,
    }

    impl Calendar {
        /// Reads `"YYYY": [l1, ..., l12]` lines from `bs-calendar.json`.
        pub(crate) fn from_json(text: &str) -> Self {
            let mut len = Vec::new();
            for line in text.lines() {
                let line = line.trim();
                let Some((key, rest)) = line.split_once(':') else {
                    continue;
                };
                let key = key.trim_matches('"');
                if key.len() != 4 || !key.bytes().all(|b| b.is_ascii_digit()) {
                    continue;
                }
                let inside = rest.trim().trim_end_matches(',').trim_matches(['[', ']']);
                let values: Vec<i64> = inside
                    .split(',')
                    .map(|v| v.trim().parse().unwrap())
                    .collect();
                len.push(<[i64; 12]>::try_from(values).unwrap());
            }
            assert_eq!(len.len(), 299);
            let mut year_start = vec![0];
            for year in &len {
                year_start.push(year_start.last().unwrap() + year.iter().sum::<i64>());
            }
            Self { len, year_start }
        }

        pub(crate) fn month_length(&self, y: i64, m: i64) -> i64 {
            self.len[(y - MIN_YEAR) as usize][(m - 1) as usize]
        }

        pub(crate) fn to_serial(&self, y: i64, m: i64, d: i64) -> i64 {
            let mut s = self.year_start[(y - MIN_YEAR) as usize];
            for k in 1..m {
                s += self.month_length(y, k);
            }
            s + (d - 1)
        }

        pub(crate) fn from_serial(&self, s: i64) -> (i64, i64, i64) {
            // Largest i with YEAR_START[i] <= s, over YEAR_START[0..YEARS].
            let i = self.year_start[..299].partition_point(|&start| start <= s) - 1;
            let y = MIN_YEAR + i as i64;
            let mut r = s - self.year_start[i];
            let mut m = 1;
            while r >= self.month_length(y, m) {
                r -= self.month_length(y, m);
                m += 1;
            }
            (y, m, r + 1)
        }
    }

    pub(crate) fn civil_from_days(z: i64) -> (i64, i64, i64) {
        let z = z + 719_468;
        let era = z / 146_097;
        let doe = z - era * 146_097;
        let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
        let y = yoe + era * 400;
        let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
        let mp = (5 * doy + 2) / 153;
        let d = doy - (153 * mp + 2) / 5 + 1;
        let m = if mp < 10 { mp + 3 } else { mp - 9 };
        let y = if m <= 2 { y + 1 } else { y };
        (y, m, d)
    }
}

fn parts(d: NepaliDate) -> (i64, i64, i64) {
    (
        i64::from(d.year()),
        i64::from(d.month_number()),
        i64::from(d.day()),
    )
}

fn ad_parts(g: GregorianDate) -> (i64, i64, i64) {
    (
        i64::from(g.year()),
        i64::from(g.month()),
        i64::from(g.day()),
    )
}

#[test]
fn every_serial() {
    let cal =
        reference::Calendar::from_json(&common::read_shared("data/calendar/bs-calendar.json"));
    let mut date = NepaliDate::MIN;
    let mut previous_ad: Option<GregorianDate> = None;
    for s in 0..=109_211i64 {
        // The library walks with succ(), which is from_serial(to_serial() + 1).
        assert_eq!(parts(date), cal.from_serial(s), "from_serial({s})");
        let (y, m, d) = parts(date);
        assert_eq!(cal.to_serial(y, m, d), s, "to_serial of serial {s}");
        assert_eq!(i64::from(NepaliDate::MIN.days_until(date)), s);

        let ad = date.to_gregorian();
        assert_eq!(
            ad_parts(ad),
            reference::civil_from_days(s + reference::EPOCH_UNIX_DAYS),
            "AD of {s}"
        );
        assert_eq!(
            NepaliDate::try_from(ad),
            Ok(date),
            "AD back to BS for serial {s}"
        );
        // The previous serial's AD date is the day before this one, so AD days are consecutive.
        if let Some(prev) = previous_ad {
            let day_before = reference::civil_from_days(s - 1 + reference::EPOCH_UNIX_DAYS);
            assert_eq!(
                ad_parts(prev),
                day_before,
                "AD days not consecutive at serial {s}"
            );
            assert!(prev < ad);
        }
        previous_ad = Some(ad);
        match date.succ() {
            Some(next) => {
                assert_eq!(next.pred(), Some(date));
                date = next;
            }
            None => assert_eq!(s, 109_211),
        }
    }
    assert_eq!(date, NepaliDate::MAX);
}
