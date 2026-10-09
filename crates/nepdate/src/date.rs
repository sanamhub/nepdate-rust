//! `NepaliDate`, a valid BS date in the supported range (ADR-0004 §2).

use core::fmt;

use crate::calendar::{MAX_YEAR, MIN_YEAR, month_length};
use crate::{Error, Month};

/// A valid Bikram Sambat date from 1901-01-01 to 2199-12-30.
///
/// Four bytes, `Copy`, ordered by date. Every value is valid: the only ways to build one check
/// the calendar table.
///
/// ```
/// use nepdate::{Month, NepaliDate};
///
/// let date = NepaliDate::new(2081, 4, 15)?;
/// assert_eq!(date.month(), Month::Shrawan);
/// assert_eq!(date.to_string(), "2081/04/15");
/// # Ok::<(), nepdate::Error>(())
/// ```
///
/// The fields are private, so a date can't be built without the checks:
///
/// ```compile_fail
/// let date = nepdate::NepaliDate { year: 2081, month: nepdate::Month::Shrawan, day: 40 };
/// ```
// Field order is year, month, day so the derived `Ord` is date order.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct NepaliDate {
    year: u16,
    month: Month,
    day: u8,
}

impl NepaliDate {
    /// The first supported date, BS 1901-01-01 (AD 1844-04-11).
    ///
    /// ```
    /// assert_eq!(nepdate::NepaliDate::MIN.to_string(), "1901/01/01");
    /// ```
    pub const MIN: NepaliDate = NepaliDate::new_unchecked(1901, Month::Baishakh, 1);

    /// The last supported date, BS 2199-12-30 (AD 2143-04-15).
    ///
    /// ```
    /// assert_eq!(nepdate::NepaliDate::MAX.to_string(), "2199/12/30");
    /// ```
    pub const MAX: NepaliDate = NepaliDate::new_unchecked(2199, Month::Chaitra, 30);

    /// A date from year, month and day, checked against the calendar table.
    ///
    /// # Errors
    ///
    /// Checked in this order (ALGORITHM §3): [`Error::OutOfRange`] for a year outside 1901 to
    /// 2199, [`Error::InvalidMonth`], then [`Error::InvalidDay`].
    ///
    /// ```
    /// use nepdate::{Error, NepaliDate};
    ///
    /// assert!(NepaliDate::new(2081, 4, 32).is_ok());
    /// assert_eq!(
    ///     NepaliDate::new(2081, 4, 33),
    ///     Err(Error::InvalidDay { month_length: 32, day: 33 })
    /// );
    ///
    /// // const fn, so a date can be a constant:
    /// const NEW_YEAR: NepaliDate = match NepaliDate::new(2081, 1, 1) {
    ///     Ok(d) => d,
    ///     Err(_) => panic!(),
    /// };
    /// assert_eq!(NEW_YEAR.day(), 1);
    /// ```
    pub const fn new(year: u16, month: u8, day: u8) -> Result<Self, Error> {
        if year < MIN_YEAR || year > MAX_YEAR {
            return Err(Error::OutOfRange);
        }
        let Some(month_value) = Month::from_number(month) else {
            return Err(Error::InvalidMonth(month));
        };
        let len = month_length(year, month);
        if day < 1 || day > len {
            return Err(Error::InvalidDay {
                month_length: len,
                day,
            });
        }
        Ok(Self {
            year,
            month: month_value,
            day,
        })
    }

    /// A date the caller has already checked. Only for constants and proven values.
    pub(crate) const fn new_unchecked(year: u16, month: Month, day: u8) -> Self {
        Self { year, month, day }
    }

    /// The BS year, 1901 to 2199.
    ///
    /// ```
    /// assert_eq!(nepdate::NepaliDate::new(2081, 4, 15)?.year(), 2081);
    /// # Ok::<(), nepdate::Error>(())
    /// ```
    #[must_use]
    pub const fn year(self) -> u16 {
        self.year
    }

    /// The month.
    ///
    /// ```
    /// use nepdate::{Month, NepaliDate};
    ///
    /// assert_eq!(NepaliDate::new(2081, 4, 15)?.month(), Month::Shrawan);
    /// # Ok::<(), nepdate::Error>(())
    /// ```
    #[must_use]
    pub const fn month(self) -> Month {
        self.month
    }

    /// The month number, 1 to 12.
    ///
    /// ```
    /// assert_eq!(nepdate::NepaliDate::new(2081, 4, 15)?.month_number(), 4);
    /// # Ok::<(), nepdate::Error>(())
    /// ```
    #[must_use]
    pub const fn month_number(self) -> u8 {
        self.month.number()
    }

    /// The day of the month, 1 to 32.
    ///
    /// ```
    /// assert_eq!(nepdate::NepaliDate::new(2081, 4, 15)?.day(), 15);
    /// # Ok::<(), nepdate::Error>(())
    /// ```
    #[must_use]
    pub const fn day(self) -> u8 {
        self.day
    }

    /// Days in this date's month, 29 to 32.
    ///
    /// ```
    /// assert_eq!(nepdate::NepaliDate::new(2081, 4, 1)?.month_length(), 32);
    /// # Ok::<(), nepdate::Error>(())
    /// ```
    #[must_use]
    pub fn month_length(self) -> u8 {
        month_length(self.year, self.month.number())
    }

    /// The first day of this date's month.
    ///
    /// ```
    /// let d = nepdate::NepaliDate::new(2081, 4, 15)?;
    /// assert_eq!(d.first_day_of_month().to_string(), "2081/04/01");
    /// # Ok::<(), nepdate::Error>(())
    /// ```
    #[must_use]
    pub fn first_day_of_month(self) -> Self {
        Self::new_unchecked(self.year, self.month, 1)
    }

    /// The last day of this date's month.
    ///
    /// ```
    /// let d = nepdate::NepaliDate::new(2081, 4, 15)?;
    /// assert_eq!(d.last_day_of_month().to_string(), "2081/04/32");
    /// # Ok::<(), nepdate::Error>(())
    /// ```
    #[must_use]
    pub fn last_day_of_month(self) -> Self {
        Self::new_unchecked(self.year, self.month, self.month_length())
    }
}

impl fmt::Display for NepaliDate {
    /// `YYYY/MM/DD`, zero padded, ASCII digits (FORMATTING §2).
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{:04}/{:02}/{:02}",
            self.year,
            self.month.number(),
            self.day
        )
    }
}

impl fmt::Debug for NepaliDate {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "NepaliDate({:04}-{:02}-{:02})",
            self.year,
            self.month.number(),
            self.day
        )
    }
}

#[cfg(test)]
mod tests {
    extern crate std;
    use super::*;
    use core::mem::size_of;
    use std::format;

    #[test]
    fn error_precedence() {
        assert_eq!(NepaliDate::new(1900, 13, 40), Err(Error::OutOfRange));
        assert_eq!(NepaliDate::new(2080, 13, 40), Err(Error::InvalidMonth(13)));
        assert_eq!(
            NepaliDate::new(2080, 1, 32),
            Err(Error::InvalidDay {
                month_length: 31,
                day: 32
            })
        );
        assert!(NepaliDate::new(2081, 4, 32).is_ok());
        assert_eq!(
            NepaliDate::new(2081, 4, 33),
            Err(Error::InvalidDay {
                month_length: 32,
                day: 33
            })
        );
        assert_eq!(NepaliDate::new(2081, 0, 1), Err(Error::InvalidMonth(0)));
        assert_eq!(NepaliDate::new(2200, 1, 1), Err(Error::OutOfRange));
        assert_eq!(
            NepaliDate::new(2081, 1, 0),
            Err(Error::InvalidDay {
                month_length: 31,
                day: 0
            })
        );
    }

    #[test]
    fn sizes() {
        assert_eq!(size_of::<NepaliDate>(), 4);
        assert_eq!(size_of::<Option<NepaliDate>>(), 4);
        assert!(size_of::<Result<NepaliDate, Error>>() <= 4);
    }

    #[test]
    fn ordering() {
        let a = NepaliDate::new(2080, 12, 30);
        let b = NepaliDate::new(2081, 1, 1);
        assert!(matches!((a, b), (Ok(a), Ok(b)) if a < b));
        assert!(NepaliDate::MIN < NepaliDate::MAX);
    }

    #[test]
    fn const_construction() {
        const D: Result<NepaliDate, Error> = NepaliDate::new(2081, 4, 15);
        assert!(matches!(D, Ok(d) if d.day() == 15));
    }

    #[test]
    fn min_max_are_valid() {
        assert_eq!(NepaliDate::new(1901, 1, 1), Ok(NepaliDate::MIN));
        assert_eq!(NepaliDate::new(2199, 12, 30), Ok(NepaliDate::MAX));
        assert_eq!(NepaliDate::MAX.month_length(), 30);
    }

    #[test]
    fn display_and_debug() {
        let d = NepaliDate::new_unchecked(2081, Month::Shrawan, 5);
        assert_eq!(format!("{d}"), "2081/04/05");
        assert_eq!(format!("{d:?}"), "NepaliDate(2081-04-05)");
    }

    #[test]
    fn every_valid_day_builds_and_every_invalid_day_fails() {
        for year in MIN_YEAR..=MAX_YEAR {
            for month in 1..=12u8 {
                let len = month_length(year, month);
                for day in 1..=len {
                    assert!(NepaliDate::new(year, month, day).is_ok());
                }
                assert_eq!(
                    NepaliDate::new(year, month, len + 1),
                    Err(Error::InvalidDay {
                        month_length: len,
                        day: len + 1
                    })
                );
            }
        }
    }
}
