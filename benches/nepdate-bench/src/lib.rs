//! Shared pieces for the `nepdate-bench` crate: the 64 benchmark inputs of
//! `shared/spec/BENCHMARK.md` §2 and a stack buffer for the allocation-free formatters.
//!
//! This crate is `publish = false`, so it and its competitor dependencies never reach a
//! published crate (ADR-0014). `cargo xtask bench` runs its `alloc` and `agree` binaries and
//! the `compare` criterion bench.

// The inputs are known-valid, so the harness may unwrap/expect; the libraries never do.
#![allow(clippy::unwrap_used, clippy::expect_used)]

pub mod inputs;

use std::fmt;
use std::hint::black_box;

use nepdate::{Lang, NepaliDate, NepaliDateRange};

/// B7: everything [`nepdate_patro::info`] says about one day, every name read in both languages.
/// The returned byte count only exists so the loop result is consumed; it is not meaningful.
#[must_use]
pub fn read_day(date: NepaliDate) -> usize {
    let info = nepdate_patro::info(black_box(date));
    let mut bytes = 0;
    if let Some(tithi) = info.tithi() {
        bytes += tithi.name(Lang::En).len() + tithi.name(Lang::Ne).len();
    }
    bytes += usize::from(info.is_public_holiday());
    for event in info.events() {
        bytes += event.name(Lang::En).len() + event.name(Lang::Ne).len();
    }
    black_box(bytes)
}

/// B8: a month grid, the same [`read_day`] for every day of the month `date` is in.
///
/// # Panics
///
/// Never for the benchmark inputs: every [`NepaliDate`] is in a valid month, so
/// [`NepaliDateRange::month`] always returns `Ok`.
#[must_use]
pub fn read_month(date: NepaliDate) -> usize {
    let mut bytes = 0;
    for day in NepaliDateRange::month(date.year(), date.month_number())
        .expect("a valid date is in a valid month")
        .days()
    {
        bytes += read_day(day);
    }
    black_box(bytes)
}

/// A reused stack buffer for B5: 128 bytes hold the longest Nepali long date with weekday.
/// Nothing is checked past the end, so a write that overflows fails the same way the library's
/// own `write_to` callers do.
pub struct StackBuf {
    bytes: [u8; 128],
    len: usize,
}

impl StackBuf {
    /// An empty buffer, ready for a formatter to write into.
    #[must_use]
    pub fn new() -> Self {
        Self {
            bytes: [0; 128],
            len: 0,
        }
    }

    /// Empties the buffer so the next write starts from the beginning.
    pub fn clear(&mut self) {
        self.len = 0;
    }

    /// The bytes written so far.
    #[must_use]
    pub fn as_bytes(&self) -> &[u8] {
        // `len` never exceeds the capacity: `write_str` rejects an overflowing write.
        self.bytes.get(..self.len).unwrap_or(&[])
    }
}

impl Default for StackBuf {
    fn default() -> Self {
        Self::new()
    }
}

impl fmt::Write for StackBuf {
    fn write_str(&mut self, s: &str) -> fmt::Result {
        let end = self.len + s.len();
        let dest = self.bytes.get_mut(self.len..end).ok_or(fmt::Error)?;
        dest.copy_from_slice(s.as_bytes());
        self.len = end;
        Ok(())
    }
}
