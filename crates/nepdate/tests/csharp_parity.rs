//! Parity with C# `NepDate` 2.0.7 on `shared/spec/vectors/csharp-golden/dates.tsv` (L2-07).
//! Conversion columns here; L3 and L4 add the arithmetic and formatting columns.
#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing,
    clippy::many_single_char_names
)]

mod common;

use nepdate::Lang;

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
        // L3: fy_start, fy_end, quarter_start, quarter_end.
    }
}
