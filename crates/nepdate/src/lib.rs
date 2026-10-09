//! Bikram Sambat (Nepali) calendar. See `shared/spec/` in the repository for the algorithm.
#![no_std]
#![forbid(unsafe_code)]
#![cfg_attr(docsrs, feature(doc_cfg))]

#[cfg(feature = "std")]
extern crate std;

mod generated {
    pub(crate) mod calendar;
}

mod calendar;
mod civil;
mod date;
mod error;
mod names;

pub use crate::date::NepaliDate;
pub use crate::error::{Error, ParseErrorKind};
pub use crate::names::{Lang, Month, Weekday};
