//! Bikram Sambat (Nepali) calendar. See `shared/spec/` in the repository for the algorithm.
#![no_std]
#![forbid(unsafe_code)]
#![cfg_attr(docsrs, feature(doc_cfg))]

#[cfg(feature = "std")]
extern crate std;

mod generated {
    pub(crate) mod calendar;
}

mod arith;
mod calendar;
mod civil;
mod date;
mod error;
mod fiscal;
mod gregorian;
mod names;
mod range;

pub use crate::arith::{DateDiff, MonthOverflow};
pub use crate::date::NepaliDate;
pub use crate::error::{Error, ParseErrorKind};
pub use crate::fiscal::{FiscalLabel, FiscalYear, Quarter};
pub use crate::gregorian::GregorianDate;
// `Digits` belongs with the formatters (docs/API.md); it lives in names.rs until L4-02 adds
// format.rs, because FiscalLabel needs it first (L3-03).
pub use crate::names::{Digits, Lang, Month, Weekday};
pub use crate::range::{Days, NepaliDateRange};
