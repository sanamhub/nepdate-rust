//! Every row of `shared/spec/vectors/parse.tsv`, both parsers (L4-05, PARSING).
#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing
)]

mod common;

use nepdate::{Error, NepaliDate, ParseErrorKind};

/// A cell is a date `YYYY-MM-DD` or `ERR:<Kind>`.
fn expected(cell: &str) -> Result<NepaliDate, &str> {
    match cell.strip_prefix("ERR:") {
        Some(kind) => Err(kind),
        None => Ok(common::bs(cell)),
    }
}

/// The spec's name for an error kind.
fn kind_name(e: Error) -> &'static str {
    match e {
        Error::OutOfRange => "OutOfRange",
        Error::InvalidMonth(_) => "InvalidMonth",
        Error::InvalidDay { .. } => "InvalidDay",
        Error::InvalidGregorian => "InvalidGregorian",
        Error::Parse(ParseErrorKind::InvalidCharacter) => "InvalidCharacter",
        Error::Parse(ParseErrorKind::WrongGroupCount) => "WrongGroupCount",
        Error::Parse(ParseErrorKind::NumberTooLong) => "NumberTooLong",
        Error::Parse(ParseErrorKind::Ambiguous) => "Ambiguous",
        Error::Parse(ParseErrorKind::Unrecognized) => "Unrecognized",
        _ => "unknown",
    }
}

#[test]
fn every_row_both_columns() {
    let text = common::read_shared("spec/vectors/parse.tsv");
    let mut count = 0;
    // Not `common::rows`: the empty input is a real row, and an input may start with spaces.
    for line in text.lines().skip(1) {
        let cols: Vec<&str> = line.split('\t').collect();
        assert_eq!(cols.len(), 3, "{line:?}");
        let input = cols[0];
        let strict = input.parse::<NepaliDate>().map_err(kind_name);
        let lenient = NepaliDate::parse_lenient(input).map_err(kind_name);
        assert_eq!(strict, expected(cols[1]), "strict {input:?}");
        assert_eq!(lenient, expected(cols[2]), "lenient {input:?}");
        count += 1;
    }
    assert_eq!(count, 59);
}
