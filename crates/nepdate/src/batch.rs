//! Ordered batch conversion (NC04 to NC08), behind the `alloc` feature.
//!
//! The C# bulk path uses PLINQ above 500 items and does not promise the input order. These
//! functions always keep the input order. The `useParallel` and `batchSize` arguments of C# do
//! not cross into this API.

use alloc::vec::Vec;
use core::borrow::Borrow;

use crate::{Error, GregorianDate, NepaliDate};

/// Converts a sequence of Gregorian dates to BS dates, in input order (NC04 and NC07).
///
/// A slice, an array or an iterator is accepted; a reference item is borrowed.
///
/// # Errors
///
/// The first conversion that is outside the supported range stops the whole call with
/// [`Error::OutOfRange`]; no partial result is returned.
///
/// ```
/// use nepdate::{to_nepali_dates, GregorianDate, NepaliDate};
///
/// let dates = [
///     GregorianDate::new(1844, 4, 11)?,
///     GregorianDate::new(2024, 7, 30)?,
/// ];
/// let bs = to_nepali_dates(&dates)?;
/// assert_eq!(bs, [NepaliDate::MIN, NepaliDate::new(2081, 4, 15)?]);
/// # Ok::<(), nepdate::Error>(())
/// ```
pub fn to_nepali_dates<I, T>(dates: I) -> Result<Vec<NepaliDate>, Error>
where
    I: IntoIterator<Item = T>,
    T: Borrow<GregorianDate>,
{
    let mut out = Vec::new();
    for date in dates {
        out.push(NepaliDate::try_from(*date.borrow())?);
    }
    Ok(out)
}

/// Converts a sequence of BS dates to Gregorian dates, in input order (NC06 and NC08).
///
/// A slice, an array or an iterator is accepted; a reference item is borrowed.
///
/// # Errors
///
/// This conversion cannot fail for a valid [`NepaliDate`], so the signature keeps the `Result`
/// that matches [`to_nepali_dates`] and the other ports.
///
/// ```
/// use nepdate::{to_english_dates, NepaliDate};
///
/// let dates = [NepaliDate::MIN, NepaliDate::new(2081, 4, 15)?];
/// let ad = to_english_dates(&dates)?;
/// assert_eq!(ad[0].to_string(), "1844-04-11");
/// # Ok::<(), nepdate::Error>(())
/// ```
pub fn to_english_dates<I, T>(dates: I) -> Result<Vec<GregorianDate>, Error>
where
    I: IntoIterator<Item = T>,
    T: Borrow<NepaliDate>,
{
    let mut out = Vec::new();
    for date in dates {
        out.push(date.borrow().to_gregorian());
    }
    Ok(out)
}

/// Parses a sequence of BS date strings and returns their Gregorian dates, in input order
/// (NC05).
///
/// The name is `to_english_dates_from_strings` rather than a second `to_english_dates` because
/// Rust has no function overloading. The result is the same ordered batch.
///
/// # Errors
///
/// The first string that does not parse strictly stops the whole call; no partial result is
/// returned.
///
/// ```
/// use nepdate::to_english_dates_from_strings;
///
/// let ad = to_english_dates_from_strings(["2081-04-15", "1901-01-01"])?;
/// assert_eq!(ad[0].to_string(), "2024-07-30");
/// # Ok::<(), nepdate::Error>(())
/// ```
pub fn to_english_dates_from_strings<I, S>(strings: I) -> Result<Vec<GregorianDate>, Error>
where
    I: IntoIterator<Item = S>,
    S: AsRef<str>,
{
    let mut out = Vec::new();
    for text in strings {
        out.push(text.as_ref().parse::<NepaliDate>()?.to_gregorian());
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    #![allow(clippy::panic, clippy::unwrap_used)]

    use alloc::vec;

    use super::*;

    fn bs(y: u16, m: u8, d: u8) -> NepaliDate {
        NepaliDate::new(y, m, d).unwrap_or(NepaliDate::MIN)
    }

    #[test]
    fn keeps_input_order() {
        let ad = [
            GregorianDate::new(2143, 4, 15).unwrap(),
            GregorianDate::new(1844, 4, 11).unwrap(),
            GregorianDate::new(2024, 7, 30).unwrap(),
        ];
        let dates = to_nepali_dates(ad).unwrap();
        assert_eq!(
            dates,
            vec![NepaliDate::MAX, NepaliDate::MIN, bs(2081, 4, 15)]
        );
        let back = to_english_dates(dates).unwrap();
        assert_eq!(back, ad);
    }

    #[test]
    fn one_bad_item_fails_the_call() {
        let bad = ["2081-04-15", "2081-01-32"];
        assert_eq!(
            to_english_dates_from_strings(bad),
            Err(Error::InvalidDay {
                month_length: 31,
                day: 32
            })
        );
    }
}
