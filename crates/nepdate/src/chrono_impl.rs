//! Feature `chrono`: conversions between `NepaliDate` and `chrono::NaiveDate` (ADR-0005).

use chrono::{Datelike, NaiveDate};

use crate::{Error, NepaliDate};

#[cfg_attr(docsrs, doc(cfg(feature = "chrono")))]
impl From<NepaliDate> for NaiveDate {
    fn from(date: NepaliDate) -> Self {
        let ad = date.to_gregorian();
        // AD 1844-04-11 to 2143-04-15 is inside chrono's range, so the fallback is never
        // taken. It exists because the library never panics.
        NaiveDate::from_ymd_opt(ad.year(), u32::from(ad.month()), u32::from(ad.day()))
            .unwrap_or(NaiveDate::MIN)
    }
}

#[cfg_attr(docsrs, doc(cfg(feature = "chrono")))]
impl TryFrom<NaiveDate> for NepaliDate {
    type Error = Error;

    /// [`Error::OutOfRange`] outside AD 1844-04-11 to 2143-04-15.
    fn try_from(date: NaiveDate) -> Result<Self, Error> {
        // chrono months are 1 to 12 and days 1 to 31, so both fit a u8.
        let month = u8::try_from(date.month()).map_err(|_| Error::InvalidGregorian)?;
        let day = u8::try_from(date.day()).map_err(|_| Error::InvalidGregorian)?;
        NepaliDate::from_gregorian(date.year(), month, day)
    }
}
