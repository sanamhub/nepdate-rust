//! Feature `jiff`: conversions between `NepaliDate` and `jiff::civil::Date`, and today's date
//! (ADR-0011, `docs/API.md`).

use jiff::Timestamp;
use jiff::civil::Date;
use jiff::tz::{Offset, TimeZone};

use crate::{Error, NepaliDate};

/// Nepal Standard Time, UTC+05:45, for when the time zone database lacks `Asia/Kathmandu`.
const NEPAL_OFFSET_SECONDS: i32 = 5 * 3600 + 45 * 60;

impl From<NepaliDate> for Date {
    fn from(date: NepaliDate) -> Self {
        let ad = date.to_gregorian();
        // AD 1844-04-11 to 2143-04-15 is inside jiff's years -9999 to 9999, so the fallback
        // is never taken. It exists because the library never panics.
        let parts = (
            i16::try_from(ad.year()),
            i8::try_from(ad.month()),
            i8::try_from(ad.day()),
        );
        match parts {
            (Ok(y), Ok(m), Ok(d)) => Date::new(y, m, d).unwrap_or(Date::MIN),
            _ => Date::MIN,
        }
    }
}

impl TryFrom<Date> for NepaliDate {
    type Error = Error;

    /// [`Error::OutOfRange`] outside AD 1844-04-11 to 2143-04-15.
    fn try_from(date: Date) -> Result<Self, Error> {
        // jiff months are 1 to 12 and days 1 to 31, so both fit a u8.
        let month = u8::try_from(date.month()).map_err(|_| Error::InvalidGregorian)?;
        let day = u8::try_from(date.day()).map_err(|_| Error::InvalidGregorian)?;
        NepaliDate::from_gregorian(i32::from(date.year()), month, day)
    }
}

/// The BS date of the instant `now` in `tz`. Split out of `today_in` so tests can fix the clock.
fn date_at(now: Timestamp, tz: &TimeZone) -> Result<NepaliDate, Error> {
    NepaliDate::try_from(tz.to_datetime(now).date())
}

/// `Asia/Kathmandu` from the time zone database, or a fixed UTC+05:45 zone without it.
fn nepal() -> TimeZone {
    TimeZone::get("Asia/Kathmandu").unwrap_or_else(|_| fixed_nepal())
}

fn fixed_nepal() -> TimeZone {
    // 20,700 seconds is inside jiff's offset range of about ±26 hours, so UTC is never used.
    Offset::from_seconds(NEPAL_OFFSET_SECONDS).map_or(TimeZone::UTC, TimeZone::fixed)
}

impl NepaliDate {
    /// Today's date in Nepal: `Asia/Kathmandu`, or a fixed UTC+05:45 when the time zone
    /// database lacks it. Needs the `jiff` feature.
    ///
    /// # Errors
    ///
    /// [`Error::OutOfRange`] when the system clock is before AD 1844-04-11 or after
    /// 2143-04-15 in Nepal.
    ///
    /// ```
    /// use nepdate::NepaliDate;
    ///
    /// let today = NepaliDate::today()?;
    /// // Midnight may pass between the two calls.
    /// assert!(today.is_today() || today.succ().is_some_and(NepaliDate::is_today));
    /// # Ok::<(), nepdate::Error>(())
    /// ```
    #[must_use = "the clock may be out of range"]
    pub fn today() -> Result<Self, Error> {
        Self::today_in(&nepal())
    }

    /// Today's date in the time zone `tz`. Needs the `jiff` feature.
    ///
    /// # Errors
    ///
    /// [`Error::OutOfRange`] when the system clock is before AD 1844-04-11 or after
    /// 2143-04-15 in `tz`.
    ///
    /// ```
    /// use jiff::tz::TimeZone;
    /// use nepdate::NepaliDate;
    ///
    /// let in_utc = NepaliDate::today_in(&TimeZone::UTC)?;
    /// let in_nepal = NepaliDate::today()?;
    /// // Nepal is 5 h 45 min ahead of UTC: the same day or the next one.
    /// assert!(matches!(in_utc.days_until(in_nepal), 0 | 1));
    /// # Ok::<(), nepdate::Error>(())
    /// ```
    #[must_use = "the clock may be out of range"]
    pub fn today_in(tz: &TimeZone) -> Result<Self, Error> {
        date_at(Timestamp::now(), tz)
    }

    /// Whether `self` is [`NepaliDate::today`]. `false` when the clock is out of range. Needs
    /// the `jiff` feature.
    ///
    /// ```
    /// use nepdate::NepaliDate;
    ///
    /// assert!(!NepaliDate::MIN.is_today());
    /// ```
    #[must_use]
    pub fn is_today(self) -> bool {
        Self::today() == Ok(self)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn at(text: &str) -> Timestamp {
        text.parse().unwrap_or(Timestamp::UNIX_EPOCH)
    }

    #[test]
    fn clock_at_2024_07_30_18_30_utc() {
        // PORT-PLAN phase 4: the same instant is 2081-04-15 in UTC and 2081-04-16 in Nepal.
        let now = at("2024-07-30T18:30:00Z");
        let kathmandu = TimeZone::get("Asia/Kathmandu");
        assert!(kathmandu.is_ok(), "tzdb has Asia/Kathmandu");
        let kathmandu = kathmandu.unwrap_or(TimeZone::UTC);
        assert_eq!(date_at(now, &TimeZone::UTC), NepaliDate::new(2081, 4, 15));
        assert_eq!(date_at(now, &kathmandu), NepaliDate::new(2081, 4, 16));
        assert_eq!(date_at(now, &nepal()), NepaliDate::new(2081, 4, 16));
    }

    #[test]
    fn fixed_offset_fallback_is_nepal_time() {
        // 18:15 UTC is midnight of the next day in Nepal; 18:14 is still the same day.
        let fixed = fixed_nepal();
        assert_eq!(
            date_at(at("2024-07-30T18:15:00Z"), &fixed),
            NepaliDate::new(2081, 4, 16)
        );
        assert_eq!(
            date_at(at("2024-07-30T18:14:59Z"), &fixed),
            NepaliDate::new(2081, 4, 15)
        );
    }

    #[test]
    fn clock_outside_the_supported_range() {
        let early = at("1844-04-10T12:00:00Z");
        assert_eq!(date_at(early, &TimeZone::UTC), Err(Error::OutOfRange));
        let late = at("2143-04-16T12:00:00Z");
        assert_eq!(date_at(late, &TimeZone::UTC), Err(Error::OutOfRange));
    }
}
