//! Parity with C# `NepDate` 2.0.7 on `shared/spec/vectors/csharp-golden/dates.tsv`.
//! Conversion columns, arithmetic files and fiscal columns, text columns,
//! `format-pattern.tsv` and the strict column of the C# `parse.tsv`.
//! No arithmetic row is skipped: no PARITY deviation applies to integer arithmetic. The only
//! skipped rows are D-04 (`MMM` for Ashad and Ashoj) and D-05 (Devanagari digits in strict
//! parsing), each behind a commented filter.
#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing,
    clippy::many_single_char_names
)]

mod common;

use nepdate::{
    DateOrder, Error, Lang, LongDate, MonthOverflow, NepaliDate, NepaliDateRange, Separator,
};

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

#[test]
fn dates_text_columns() {
    let rows = common::rows("spec/vectors/csharp-golden/dates.tsv", '\t');
    let weekday = LongDate {
        weekday: true,
        ..LongDate::default()
    };
    for row in &rows {
        let d = common::bs(&row[0]);
        let columns = [
            (
                d.short(DateOrder::Ymd, Separator::Slash, true, Lang::Ne)
                    .to_string(),
                "unicode",
            ),
            (d.long(LongDate::default(), Lang::En).to_string(), "long_en"),
            (d.long(weekday, Lang::En).to_string(), "long_en_weekday"),
            (d.long(LongDate::default(), Lang::Ne).to_string(), "long_ne"),
            (d.long(weekday, Lang::Ne).to_string(), "long_ne_weekday"),
            (
                d.short(DateOrder::Dmy, Separator::Dash, false, Lang::En)
                    .to_string(),
                "dmy_dash_nopad",
            ),
        ];
        for (i, (actual, name)) in columns.iter().enumerate() {
            assert_eq!(actual, &row[6 + i], "{name} of {}", row[0]);
        }
    }
}

/// Whether `pattern` has a run of exactly three `M`s, the token D-04 changes.
fn has_mmm(pattern: &str) -> bool {
    pattern.split(|c| c != 'M').any(|run| run.len() == 3)
}

#[test]
fn format_pattern_rows() {
    let rows = common::rows("spec/vectors/csharp-golden/format-pattern.tsv", '\t');
    assert_eq!(rows.len(), 451);
    let mut skipped = 0;
    for row in &rows {
        let d = common::bs(&row[0]);
        // D-04 (ADR-0009): MMM for Ashad/Ashoj differs on purpose
        if has_mmm(&row[1]) && matches!(d.month_number(), 3 | 6) {
            skipped += 1;
            continue;
        }
        let actual = d.format_pattern(&row[1], Lang::En).to_string();
        assert_eq!(actual, row[2], "{} with {:?}", row[0], row[1]);
    }
    assert!(skipped > 0 && skipped < 20, "{skipped} D-04 rows");
}

#[test]
fn csharp_parse_strict_column() {
    let text = common::read_shared("spec/vectors/csharp-golden/parse.tsv");
    let mut checked = 0;
    for line in text.lines().skip(1) {
        let cols: Vec<&str> = line.split('\t').collect();
        let (input, strict) = (cols[0], cols[1]);
        // D-05 (ADR-0009): strict parsing accepts Devanagari digits, which C# rejects
        if input
            .chars()
            .any(|c| ('\u{0966}'..='\u{096F}').contains(&c))
        {
            continue;
        }
        let actual = input.parse::<NepaliDate>();
        if strict == "ERR" {
            assert!(actual.is_err(), "{input:?}: {actual:?}");
        } else {
            assert_eq!(actual, Ok(common::bs(strict)), "{input:?}");
        }
        checked += 1;
    }
    assert_eq!(checked, 40);
}
