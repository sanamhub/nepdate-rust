#![doc = include_str!("../README.md")]
#![no_std]
#![deny(missing_docs)]
#![forbid(unsafe_code)]
#![cfg_attr(docsrs, feature(doc_cfg))]

#[cfg(feature = "std")]
extern crate std;

#[cfg(feature = "alloc")]
extern crate alloc;

mod generated {
    pub(crate) mod calendar;
}

mod arith;
#[cfg(feature = "alloc")]
mod batch;
mod calendar;
#[cfg(feature = "chrono")]
mod chrono_impl;
mod civil;
mod date;
mod error;
mod fiscal;
mod format;
mod gregorian;
#[cfg(feature = "jiff")]
mod jiff_impl;
mod names;
mod parse;
mod range;
#[cfg(feature = "serde")]
mod serde_impl;

pub use crate::arith::{DateDiff, MonthOverflow};
#[cfg(feature = "alloc")]
pub use crate::batch::{to_english_dates, to_english_dates_from_strings, to_nepali_dates};
pub use crate::date::NepaliDate;
pub use crate::error::{Error, ParseErrorKind};
pub use crate::fiscal::{FiscalLabel, FiscalYear, Quarter};
pub use crate::format::{
    DateOrder, Digits, LongDate, LongFormat, PatternFormat, Separator, ShortFormat,
};
pub use crate::gregorian::GregorianDate;
pub use crate::names::{Lang, Month, Weekday};
pub use crate::range::{DatesWithInterval, Days, NepaliDateRange, RangeFormat};
