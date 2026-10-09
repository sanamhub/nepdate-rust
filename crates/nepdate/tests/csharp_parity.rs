//! Parity with C# `NepDate` 2.0.7 on `shared/spec/vectors/csharp-golden/dates.tsv` (L2-07).
//! Conversion columns (L2-07), arithmetic files and fiscal columns (L3-05); L4 adds formatting.
//! No arithmetic row is skipped: no PARITY deviation applies to integer arithmetic.
#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing,
    clippy::many_single_char_names
)]

mod common;

use nepdate::{Error, Lang, MonthOverflow, NepaliDate, NepaliDateRange};

#[test]
fn dates_conversion_columns() {
    let rows = common::rows("spec/vectors/csharp-golden/dates.tsv", '\t');
    assert_eq!(rows.len(), 1018);
    for row in &rows {
        let date = common::bs(&row[0]);
        assert_eq!(date.to_gregorian().to_string(), row[1], "ad of {}", row[0]);
        assert_eq!(
            date.weekday().name(Lang::En),
            row[2],
            "weekday of {}",
            row[0]
        );
        assert_eq!(
            date.day_of_year().to_string(),
            row[3],
            "day_of_year of {}",
            row[0]
        );
        assert_eq!(
            date.month_length().to_string(),
            row[4],
            "month_length of {}",
            row[0]
        );
        assert_eq!(date.to_string(), row[5], "to_string of {}", row[0]);
        // L4: unicode, long_en, long_en_weekday, long_ne, long_ne_weekday, dmy_dash_nopad.
    }
}

/// `ERR` in a golden file means C# threw; here it is `Err(Error::OutOfRange)`.
fn expect(result: Result<NepaliDate, Error>, cell: &str, what: &str) {
    if cell == "ERR" {
        assert_eq!(result, Err(Error::OutOfRange), "{what}");
    } else {
        assert_eq!(result, Ok(common::bs(cell)), "{what}");
    }
}

#[test]
fn add_days_all_rows() {
    let rows = common::rows("spec/vectors/csharp-golden/add-days.tsv", '\t');
    assert_eq!(rows.len(), 9162);
    for row in &rows {
        let date = common::bs(&row[0]);
        let days: i32 = row[1].parse().unwrap();
        expect(
            date.checked_add_days(days),
            &row[2],
            &format!("{} + {days} days", row[0]),
        );
    }
}

#[test]
fn add_months_all_rows_both_columns() {
    let rows = common::rows("spec/vectors/csharp-golden/add-months.tsv", '\t');
    assert_eq!(rows.len(), 10180);
    for row in &rows {
        let date = common::bs(&row[0]);
        let months: i32 = row[1].parse().unwrap();
        let what = format!("{} + {months} months", row[0]);
        expect(
            date.checked_add_months(months, MonthOverflow::Clamp),
            &row[2],
            &format!("{what}, clamp"),
        );
        expect(
            date.checked_add_months(months, MonthOverflow::Overflow),
            &row[3],
            &format!("{what}, overflow"),
        );
    }
}

#[test]
fn dates_fiscal_columns() {
    let rows = common::rows("spec/vectors/csharp-golden/dates.tsv", '\t');
    assert_eq!(rows.len(), 1018);
    for row in &rows {
        let date = common::bs(&row[0]);
        let fy = date.fiscal_year();
        let quarter = fy.quarter_range(date.quarter());
        expect(fy.start(), &row[12], &format!("fy_start of {}", row[0]));
        expect(fy.end(), &row[13], &format!("fy_end of {}", row[0]));
        expect(
            quarter.map(NepaliDateRange::start),
            &row[14],
            &format!("quarter_start of {}", row[0]),
        );
        expect(
            quarter.map(NepaliDateRange::end),
            &row[15],
            &format!("quarter_end of {}", row[0]),
        );
    }
}
