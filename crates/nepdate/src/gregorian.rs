//! `GregorianDate`, a minimal AD date so the core needs no date dependency.

use core::fmt;

use crate::civil::{days_from_civil, greg_month_length};
use crate::{Error, Weekday};

/// A valid proleptic Gregorian (AD) date, for any `i32` year.
///
/// ```
/// use nepdate::{GregorianDate, NepaliDate};
///
/// let ad = GregorianDate::new(2024, 4, 13)?;
/// assert_eq!(NepaliDate::try_from(ad)?.to_string(), "2081/01/01");
/// assert_eq!(ad.to_string(), "2024-04-13");
/// # Ok::<(), nepdate::Error>(())
/// ```
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Debug)]
pub struct GregorianDate {
    year: i32,
    month: u8,
    day: u8,
}

impl GregorianDate {
    /// A Gregorian date, checked for month 1 to 12 and the month's length (leap years included).
    ///
    /// # Errors
    ///
    /// [`Error::InvalidGregorian`] when the date doesn't exist, for example 2023-02-29.
    ///
    /// ```
    /// use nepdate::{Error, GregorianDate};
    ///
    /// assert!(GregorianDate::new(2024, 2, 29).is_ok());
    /// assert_eq!(GregorianDate::new(2023, 2, 29), Err(Error::InvalidGregorian));
    /// ```
    #[must_use = "the date may be invalid"]
    pub fn new(year: i32, month: u8, day: u8) -> Result<Self, Error> {
        if !(1..=12).contains(&month) || day < 1 || day > greg_month_length(i64::from(year), month)
        {
            return Err(Error::InvalidGregorian);
        }
        Ok(Self { year, month, day })
    }

    /// A date the caller has already proven valid.
    pub(crate) const fn new_unchecked(year: i32, month: u8, day: u8) -> Self {
        Self { year, month, day }
    }

    /// The year (proleptic: 0 is 1 BC).
    ///
    /// ```
    /// assert_eq!(nepdate::GregorianDate::new(2024, 4, 13)?.year(), 2024);
    /// # Ok::<(), nepdate::Error>(())
    /// ```
    #[must_use]
    pub const fn year(self) -> i32 {
        self.year
    }

    /// The month, 1 to 12.
    ///
    /// ```
    /// assert_eq!(nepdate::GregorianDate::new(2024, 4, 13)?.month(), 4);
    /// # Ok::<(), nepdate::Error>(())
    /// ```
    #[must_use]
    pub const fn month(self) -> u8 {
        self.month
    }

    /// The day of the month, 1 to 31.
    ///
    /// ```
    /// assert_eq!(nepdate::GregorianDate::new(2024, 4, 13)?.day(), 13);
    /// # Ok::<(), nepdate::Error>(())
    /// ```
    #[must_use]
    pub const fn day(self) -> u8 {
        self.day
    }

    /// The day of the week. Not a hot path: it uses ALGORITHM §5 directly.
    ///
    /// ```
    /// use nepdate::{GregorianDate, Weekday};
    ///
    /// assert_eq!(GregorianDate::new(1970, 1, 1)?.weekday(), Weekday::Thursday);
    /// # Ok::<(), nepdate::Error>(())
    /// ```
    #[must_use]
    #[allow(
        clippy::indexing_slicing,
        clippy::cast_possible_truncation,
        clippy::cast_sign_loss
    )]
    pub fn weekday(self) -> Weekday {
        let days = days_from_civil(
            i64::from(self.year),
            i64::from(self.month),
            i64::from(self.day),
        );
        // 1970-01-01 was a Thursday (index 4); rem_euclid keeps the result in 0..7.
        Weekday::ALL[(days + 4).rem_euclid(7) as usize]
    }
}

impl fmt::Display for GregorianDate {
    /// `YYYY-MM-DD`.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:04}-{:02}-{:02}", self.year, self.month, self.day)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn validity() {
        assert_eq!(
            GregorianDate::new(2023, 2, 29),
            Err(Error::InvalidGregorian)
        );
        assert!(GregorianDate::new(2024, 2, 29).is_ok());
        assert_eq!(
            GregorianDate::new(1900, 2, 29),
            Err(Error::InvalidGregorian)
        );
        assert!(GregorianDate::new(2000, 2, 29).is_ok());
        assert_eq!(
            GregorianDate::new(2024, 13, 1),
            Err(Error::InvalidGregorian)
        );
        assert_eq!(GregorianDate::new(2024, 0, 1), Err(Error::InvalidGregorian));
        assert_eq!(
            GregorianDate::new(2024, 4, 31),
            Err(Error::InvalidGregorian)
        );
        assert_eq!(GregorianDate::new(2024, 4, 0), Err(Error::InvalidGregorian));
    }

    #[test]
    fn weekdays() {
        let thursday = GregorianDate::new(1970, 1, 1).map(GregorianDate::weekday);
        assert_eq!(thursday, Ok(Weekday::Thursday));
        let friday = GregorianDate::new(-1, 1, 1).map(GregorianDate::weekday);
        assert_eq!(friday, Ok(Weekday::Friday));
    }
}
