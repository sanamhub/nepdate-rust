//! The one error type of the crate (ADR-0007).

use core::fmt;

/// Why a date couldn't be built, converted or parsed.
///
/// ```
/// use nepdate::{Error, NepaliDate};
///
/// assert_eq!(NepaliDate::new(2081, 13, 1), Err(Error::InvalidMonth(13)));
/// assert_eq!(
///     Error::InvalidMonth(13).to_string(),
///     "invalid month 13: expected 1 to 12"
/// );
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum Error {
    /// The date is outside BS 1901-01-01 to 2199-12-30 (AD 1844-04-11 to 2143-04-15).
    OutOfRange,
    /// The month isn't 1 to 12.
    InvalidMonth(u8),
    /// The day isn't 1 to the length of the month.
    InvalidDay {
        /// Days in the month that was asked for.
        month_length: u8,
        /// The day that was asked for.
        day: u8,
    },
    /// The Gregorian date doesn't exist, for example 2023-02-29.
    InvalidGregorian,
    /// The text isn't a date in an accepted form.
    Parse(ParseErrorKind),
}

/// Why a text couldn't be parsed (PARSING).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum ParseErrorKind {
    /// A character that isn't a digit or a separator.
    InvalidCharacter,
    /// Not exactly three groups of digits.
    WrongGroupCount,
    /// A number with more than 4 digits.
    NumberTooLong,
    /// The order of year, month and day can't be decided.
    Ambiguous,
    /// No accepted form matches.
    Unrecognized,
}

impl fmt::Display for ParseErrorKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Self::InvalidCharacter => "invalid character",
            Self::WrongGroupCount => "expected year, month and day",
            Self::NumberTooLong => "number too long",
            Self::Ambiguous => "ambiguous date",
            Self::Unrecognized => "unrecognized date format",
        })
    }
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::OutOfRange => {
                f.write_str("date out of supported range (BS 1901-01-01 to 2199-12-30)")
            }
            Self::InvalidMonth(m) => write!(f, "invalid month {m}: expected 1 to 12"),
            Self::InvalidDay { month_length, day } => {
                write!(f, "invalid day {day}: month has {month_length} days")
            }
            Self::InvalidGregorian => f.write_str("invalid Gregorian date"),
            Self::Parse(kind) => write!(f, "could not parse date: {kind}"),
        }
    }
}

#[cfg(feature = "std")]
impl std::error::Error for Error {}

#[cfg(test)]
mod tests {
    extern crate std;
    use super::*;
    use std::string::ToString;

    #[test]
    fn messages() {
        assert_eq!(
            Error::OutOfRange.to_string(),
            "date out of supported range (BS 1901-01-01 to 2199-12-30)"
        );
        assert_eq!(
            Error::InvalidMonth(13).to_string(),
            "invalid month 13: expected 1 to 12"
        );
        assert_eq!(
            Error::InvalidDay {
                month_length: 31,
                day: 32
            }
            .to_string(),
            "invalid day 32: month has 31 days"
        );
        assert_eq!(
            Error::InvalidGregorian.to_string(),
            "invalid Gregorian date"
        );
        let parse = [
            (ParseErrorKind::InvalidCharacter, "invalid character"),
            (
                ParseErrorKind::WrongGroupCount,
                "expected year, month and day",
            ),
            (ParseErrorKind::NumberTooLong, "number too long"),
            (ParseErrorKind::Ambiguous, "ambiguous date"),
            (ParseErrorKind::Unrecognized, "unrecognized date format"),
        ];
        for (kind, text) in parse {
            assert_eq!(
                Error::Parse(kind).to_string(),
                std::format!("could not parse date: {text}")
            );
        }
    }
}
