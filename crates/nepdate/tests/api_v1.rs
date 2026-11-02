//! Conformance with the API v1 vectors in `shared/spec/vectors`.
//!
//! These tests load every file that covers a Rust row: `year-length.tsv`, `day-diff.tsv`,
//! `ymd-int.tsv`, `bulk.tsv`, `serialization.tsv`, `format-matrix.tsv`, `ranges.tsv` and the
//! four `range-*` files. `#` lines are skipped. No row is skipped: the new files record the
//! port's expected value in rows whose note is a D number, including D-01, D-07, D-11, D-15 and
//! D-16.
#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing,
    clippy::many_single_char_names
)]

mod common;

use nepdate::{
    DateOrder, Error, FiscalYear, GregorianDate, Lang, NepaliDate, NepaliDateRange, Separator,
    to_english_dates, to_english_dates_from_strings, to_nepali_dates,
};

fn iso(date: NepaliDate) -> String {
    date.short(DateOrder::Ymd, Separator::Dash, true, Lang::En)
        .to_string()
}

fn iso_ad(date: GregorianDate) -> String {
    date.to_string()
}

/// A BS date written `YYYY-MM-DD`, or `None` when the text is not a valid supported date.
fn try_date(text: &str) -> Option<NepaliDate> {
    let mut parts = text.split('-');
    let year = parts.next()?.parse::<u16>().ok()?;
    let month = parts.next()?.parse::<u8>().ok()?;
    let day = parts.next()?.parse::<u8>().ok()?;
    if parts.next().is_some() {
        return None;
    }
    NepaliDate::new(year, month, day).ok()
}

/// An AD date written `YYYY-MM-DD`.
fn ad(text: &str) -> GregorianDate {
    let (y, m, d) = common::ymd(text);
    GregorianDate::new(y, m, d).unwrap_or_else(|e| panic!("{text}: {e}"))
}

/// A range cell `start..end`, or `None` for `none` and for an invalid range.
fn try_range(text: &str) -> Option<NepaliDateRange> {
    let (start, end) = text.split_once("..")?;
    NepaliDateRange::new(try_date(start)?, try_date(end)?)
}

fn range(text: &str) -> NepaliDateRange {
    try_range(text).unwrap_or_else(|| panic!("not a range: {text}"))
}

fn date_list(text: &str) -> Vec<String> {
    if text == "none" {
        Vec::new()
    } else {
        text.split(';').map(str::to_owned).collect()
    }
}

fn iso_list(dates: impl IntoIterator<Item = NepaliDate>) -> String {
    let rows: Vec<String> = dates.into_iter().map(iso).collect();
    if rows.is_empty() {
        "none".to_owned()
    } else {
        rows.join(";")
    }
}

fn iso_range_list(ranges: &[NepaliDateRange]) -> String {
    if ranges.is_empty() {
        return "none".to_owned();
    }
    ranges
        .iter()
        .map(|r| format!("{}..{}", iso(r.start()), iso(r.end())))
        .collect::<Vec<_>>()
        .join(";")
}

fn order(text: &str) -> DateOrder {
    match text {
        "Ymd" => DateOrder::Ymd,
        "Ydm" => DateOrder::Ydm,
        "Myd" => DateOrder::Myd,
        "Mdy" => DateOrder::Mdy,
        "Dym" => DateOrder::Dym,
        "Dmy" => DateOrder::Dmy,
        other => panic!("unknown order {other}"),
    }
}

fn separator(text: &str) -> Separator {
    match text {
        "Slash" => Separator::Slash,
        "Backslash" => Separator::Backslash,
        "Dot" => Separator::Dot,
        "Underscore" => Separator::Underscore,
        "Dash" => Separator::Dash,
        "Space" => Separator::Space,
        other => panic!("unknown separator {other}"),
    }
}

fn lang(text: &str) -> Lang {
    match text {
        "en" => Lang::En,
        "ne" => Lang::Ne,
        other => panic!("unknown language {other}"),
    }
}

#[test]
fn year_length_rows() {
    let rows = common::rows("spec/vectors/year-length.tsv", '\t');
    assert_eq!(rows.len(), 299);
    for row in &rows {
        let year = row[0].parse::<u16>().unwrap();
        let expected = row[1].parse::<u16>().unwrap();
        let date = NepaliDate::new(year, 1, 1).unwrap();
        assert_eq!(date.year_length(), expected, "year {year}");
    }
}

#[test]
fn day_diff_rows() {
    let rows = common::rows("spec/vectors/csharp-golden/day-diff.tsv", '\t');
    assert_eq!(rows.len(), 16);
    for row in &rows {
        let a = common::bs(&row[0]);
        let b = common::bs(&row[1]);
        let until = row[2].parse::<i32>().unwrap();
        let minus = row[3].parse::<i32>().unwrap();
        assert_eq!(a.days_until(b), until, "days_until {} {}", row[0], row[1]);
        assert_eq!((a - b).total_days, minus, "operator {} {}", row[0], row[1]);
        assert_eq!(a.diff(b).total_days, until, "diff {} {}", row[0], row[1]);
    }
}

#[test]
fn ymd_int_rows() {
    let rows = common::rows("spec/vectors/csharp-golden/ymd-int.tsv", '\t');
    assert_eq!(rows.len(), 14);
    for row in &rows {
        match row[0].as_str() {
            "date" => {
                let date = common::bs(&row[1]);
                let n = row[2].parse::<i32>().unwrap();
                assert_eq!(date.to_ymd_int(), n, "to_ymd_int {}", row[1]);
                assert_eq!(NepaliDate::from_ymd_int(n), Ok(date), "from_ymd_int {n}");
            }
            "int" => {
                let n = row[2].parse::<i32>().unwrap();
                let actual = NepaliDate::from_ymd_int(n);
                if row[3] == "ERR" {
                    assert!(actual.is_err(), "from_ymd_int {n}: {actual:?}");
                } else {
                    assert_eq!(actual, Ok(common::bs(&row[3])), "from_ymd_int {n}");
                }
            }
            other => panic!("unknown kind {other}"),
        }
    }
}

#[test]
fn bulk_rows() {
    let rows = common::rows("spec/vectors/csharp-golden/bulk.tsv", '\t');
    assert_eq!(rows.len(), 6);
    for row in &rows {
        let input: Vec<&str> = row[1].split(';').collect();
        let actual: Result<Vec<String>, Error> = match row[0].as_str() {
            "to_nepali_dates" | "batch_to_nepali_dates" => {
                let dates: Vec<GregorianDate> = input.iter().map(|s| ad(s)).collect();
                to_nepali_dates(dates).map(|v| v.into_iter().map(iso).collect())
            }
            "to_english_dates_from_strings" => to_english_dates_from_strings(input.clone())
                .map(|v| v.into_iter().map(iso_ad).collect()),
            "to_english_dates_from_dates" | "batch_to_english_dates" => {
                let dates: Vec<NepaliDate> = input.iter().map(|s| common::bs(s)).collect();
                to_english_dates(dates).map(|v| v.into_iter().map(iso_ad).collect())
            }
            other => panic!("unknown operation {other}"),
        };
        match (row[2].as_str(), actual) {
            ("ERR", Err(_)) => {}
            ("ERR", Ok(value)) => panic!("expected an error, got {value:?}"),
            (expected, Ok(value)) => {
                assert_eq!(value, date_list(expected), "{}", row[0]);
            }
            (expected, Err(error)) => panic!("{}: {error}; expected {expected}", row[0]),
        }
    }
}

#[cfg(feature = "serde")]
#[test]
fn serialization_rows() {
    let rows = common::rows("spec/vectors/csharp-golden/serialization.tsv", '\t');
    assert_eq!(rows.len(), 11);
    for row in &rows {
        match row[0].as_str() {
            "write" => {
                let date = common::bs(&row[1]);
                assert_eq!(
                    serde_json::to_string(&date).unwrap(),
                    row[2],
                    "write {}",
                    row[1]
                );
            }
            "read" => {
                let actual = serde_json::from_str::<NepaliDate>(&row[1]);
                if row[3] == "ERR" {
                    assert!(actual.is_err(), "read {}: {actual:?}", row[1]);
                } else {
                    assert_eq!(actual.unwrap(), common::bs(&row[3]), "read {}", row[1]);
                }
            }
            "roundtrip" => {
                let date = common::bs(&row[1]);
                let text = serde_json::to_string(&date).unwrap();
                assert_eq!(text, row[2], "roundtrip write {}", row[1]);
                assert_eq!(
                    serde_json::from_str::<NepaliDate>(&text).unwrap(),
                    date,
                    "roundtrip read {}",
                    row[1]
                );
            }
            other => panic!("unknown kind {other}"),
        }
    }
}

struct CappedBuffer {
    bytes: Vec<u8>,
    capacity: usize,
}

impl CappedBuffer {
    fn new(capacity: usize) -> Self {
        Self {
            bytes: Vec::new(),
            capacity,
        }
    }

    fn text(&self) -> String {
        String::from_utf8(self.bytes.clone()).unwrap()
    }
}

impl std::fmt::Write for CappedBuffer {
    fn write_str(&mut self, s: &str) -> std::fmt::Result {
        if self.bytes.len() + s.len() > self.capacity {
            return Err(std::fmt::Error);
        }
        self.bytes.extend_from_slice(s.as_bytes());
        Ok(())
    }
}

#[test]
fn format_matrix_rows() {
    let rows = common::rows("spec/vectors/csharp-golden/format-matrix.tsv", '\t');
    assert_eq!(rows.len(), 102);
    for row in &rows {
        let d = common::bs(&row[1]);
        let l = lang(&row[5]);
        match row[0].as_str() {
            "short" => {
                let actual = d
                    .short(order(&row[2]), separator(&row[3]), row[4] == "true", l)
                    .to_string();
                assert_eq!(actual, row[8], "short {} {}", row[1], row[2]);
            }
            "try_format" => {
                let capacity = row[7].parse::<usize>().unwrap();
                let mut buf = CappedBuffer::new(capacity);
                let result = if row[6] == "default" {
                    d.short(DateOrder::Ymd, Separator::Slash, true, l)
                        .write_to(&mut buf)
                } else {
                    d.format_pattern(&row[6], l).write_to(&mut buf)
                };
                if row[8] == "ERR" {
                    assert!(
                        result.is_err(),
                        "try_format {} {} {capacity}",
                        row[1],
                        row[6]
                    );
                } else {
                    assert_eq!(result, Ok(()), "try_format {} {}", row[1], row[6]);
                    assert_eq!(buf.text(), row[8], "try_format {} {}", row[1], row[6]);
                }
            }
            other => panic!("unknown kind {other}"),
        }
    }
}

#[test]
fn range_construct_and_queries() {
    let rows = common::rows("spec/vectors/csharp-golden/ranges.tsv", '\t');
    assert_eq!(rows.len(), 54);
    for row in &rows {
        let start = try_date(&row[0]);
        let end = try_date(&row[1]);
        let built = start.zip(end).and_then(|(a, b)| NepaliDateRange::new(a, b));
        match row[2].as_str() {
            "construct" => {
                if row[4] == "ERR" {
                    assert!(built.is_none(), "construct {} {}", row[0], row[1]);
                } else {
                    assert_eq!(built.map(|r| iso_range_list(&[r])), Some(row[4].clone()));
                }
            }
            "length" => {
                assert_eq!(
                    built.map(NepaliDateRange::len).map(|n| n.to_string()),
                    Some(row[4].clone())
                );
            }
            "contains_date" => {
                let r = built.unwrap_or_else(|| panic!("bad range {} {}", row[0], row[1]));
                let d = common::bs(&row[3]);
                assert_eq!(
                    r.contains(d).to_string(),
                    row[4],
                    "contains_date {}",
                    row[0]
                );
            }
            "contains_range" => {
                let r = built.unwrap_or_else(|| panic!("bad range {} {}", row[0], row[1]));
                let other = range(&row[3]);
                assert_eq!(
                    r.contains_range(other).to_string(),
                    row[4],
                    "contains_range {}",
                    row[0]
                );
            }
            "overlaps" => {
                let r = built.unwrap_or_else(|| panic!("bad range {} {}", row[0], row[1]));
                let other = range(&row[3]);
                assert_eq!(r.overlaps(other).to_string(), row[4], "overlaps {}", row[0]);
            }
            "is_adjacent_to" => {
                let r = built.unwrap_or_else(|| panic!("bad range {} {}", row[0], row[1]));
                let other = range(&row[3]);
                assert_eq!(
                    r.is_adjacent_to(other).to_string(),
                    row[4],
                    "is_adjacent_to {}",
                    row[0]
                );
            }
            "intersect" => {
                let r = built.unwrap_or_else(|| panic!("bad range {} {}", row[0], row[1]));
                let other = range(&row[3]);
                let actual = r
                    .intersect(other)
                    .map_or_else(|| "none".to_owned(), |x| iso_range_list(&[x]));
                assert_eq!(actual, row[4], "intersect {}", row[0]);
            }
            "union" => {
                let r = built.unwrap_or_else(|| panic!("bad range {} {}", row[0], row[1]));
                let other = range(&row[3]);
                assert_eq!(
                    iso_range_list(&[r.union(other)]),
                    row[4],
                    "union {}",
                    row[0]
                );
            }
            "except" => {
                let r = built.unwrap_or_else(|| panic!("bad range {} {}", row[0], row[1]));
                let other = range(&row[3]);
                assert_eq!(
                    iso_range_list(&r.except(other)),
                    row[4],
                    "except {}",
                    row[0]
                );
            }
            "equals" => {
                let r = built.unwrap_or_else(|| panic!("bad range {} {}", row[0], row[1]));
                let other = range(&row[3]);
                assert_eq!((r == other).to_string(), row[4], "equals {}", row[0]);
            }
            "to_string" => {
                let r = built.unwrap_or_else(|| panic!("bad range {} {}", row[0], row[1]));
                assert_eq!(r.to_string(), row[4], "to_string {}", row[0]);
            }
            "enumerate" => {
                let r = built.unwrap_or_else(|| panic!("bad range {} {}", row[0], row[1]));
                assert_eq!(iso_list(r), row[4], "enumerate {}", row[0]);
            }
            other => panic!("unknown operation {other}"),
        }
    }
}

#[test]
fn range_factory_rows() {
    let rows = common::rows("spec/vectors/csharp-golden/range-factories.tsv", '\t');
    assert_eq!(rows.len(), 27);
    for row in &rows {
        let mut args = row[1].split(',');
        let actual: Option<NepaliDateRange> = match row[0].as_str() {
            "single_day" => try_date(&row[1]).map(NepaliDateRange::single),
            "from_day_count" => {
                let start = try_date(args.next().unwrap_or(""));
                let days = args.next().and_then(|s| s.parse::<u32>().ok());
                start
                    .zip(days)
                    .and_then(|(s, n)| NepaliDateRange::from_day_count(s, n).ok())
            }
            "for_month" => {
                let y = args.next().and_then(|s| s.parse::<u16>().ok());
                let m = args.next().and_then(|s| s.parse::<u8>().ok());
                y.zip(m)
                    .and_then(|(y, m)| NepaliDateRange::month(y, m).ok())
            }
            "for_calendar_year" => row[1]
                .parse::<u16>()
                .ok()
                .and_then(|y| NepaliDateRange::year(y).ok()),
            "for_fiscal_year" => row[1]
                .parse::<u16>()
                .ok()
                .and_then(|y| FiscalYear::new(y).range().ok()),
            "construct" => {
                let a = try_date(args.next().unwrap_or(""));
                let b = try_date(args.next().unwrap_or(""));
                a.zip(b).and_then(|(a, b)| NepaliDateRange::new(a, b))
            }
            other => panic!("unknown factory {other}"),
        };
        if row[2] == "ERR" {
            assert!(actual.is_none(), "{} {:?}", row[0], row[1]);
        } else {
            assert_eq!(
                actual.map(|r| iso_range_list(&[r])).as_deref(),
                Some(row[2].as_str()),
                "{} {:?}",
                row[0],
                row[1]
            );
        }
    }
}

#[test]
fn range_split_rows() {
    let rows = common::rows("spec/vectors/csharp-golden/range-splits.tsv", '\t');
    assert_eq!(rows.len(), 9);
    for row in &rows {
        let r = NepaliDateRange::new(common::bs(&row[0]), common::bs(&row[1])).unwrap();
        let actual = match row[2].as_str() {
            "split_by_month" => r.split_by_month(),
            "split_by_fiscal_quarter" => r.split_by_fiscal_quarter(),
            other => panic!("unknown operation {other}"),
        };
        assert_eq!(iso_range_list(&actual), row[3], "{} {}", row[0], row[2]);
    }
}

#[test]
fn range_day_rows() {
    let rows = common::rows("spec/vectors/csharp-golden/range-days.tsv", '\t');
    assert_eq!(rows.len(), 18);
    for row in &rows {
        let r = NepaliDateRange::new(common::bs(&row[0]), common::bs(&row[1])).unwrap();
        let actual: Result<String, Error> = match row[2].as_str() {
            "dates_with_interval" => {
                let interval = row[3].parse::<u32>().unwrap_or(0);
                r.dates_with_interval(interval).map(iso_list)
            }
            "working_days" => Ok(iso_list(r.working_days(row[3] == "true"))),
            "weekend_days" => Ok(iso_list(r.weekend_days(row[3] == "true"))),
            "enumerate" => Ok(iso_list(r)),
            other => panic!("unknown operation {other}"),
        };
        if row[4] == "ERR" {
            assert!(actual.is_err(), "{} {}", row[0], row[2]);
        } else {
            assert_eq!(
                actual.as_deref(),
                Ok(row[4].as_str()),
                "{} {}",
                row[0],
                row[2]
            );
        }
    }
}

#[test]
fn range_format_rows() {
    let rows = common::rows("spec/vectors/csharp-golden/range-format.tsv", '\t');
    assert_eq!(rows.len(), 18);
    for row in &rows {
        let built = NepaliDateRange::new(common::bs(&row[0]), common::bs(&row[1]));
        if row[6] == "ERR" {
            assert!(built.is_none(), "format {} {}", row[0], row[1]);
            continue;
        }
        let r = built.unwrap();
        let actual = match row[2].as_str() {
            "default" => r.to_string(),
            "short" => r
                .to_string_with(order(&row[3]), separator(&row[4]), lang(&row[5]))
                .to_string(),
            other => panic!("unknown kind {other}"),
        };
        assert_eq!(actual, row[6], "{} {} {}", row[0], row[1], row[2]);
    }
}
