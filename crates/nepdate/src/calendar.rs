//! Calendar tables (ADR-0004 §3). Only `PACKED_YEARS` is generated; the three run-time tables
//! below are `static`s whose initialisers are `const fn` calls, so the compiler computes them and
//! stores them in read-only data. Nothing runs at start-up.
//!
//! The builders read only `const` items, never a `static`: a `const fn` may not read a `static`.

use crate::civil::{civil_from_days, days_from_civil};
use crate::generated::calendar::YEARS;
pub(crate) use crate::generated::calendar::{EPOCH_UNIX_DAYS, MAX_YEAR, MIN_YEAR, PACKED_YEARS};

/// Number of supported BS months, 299 years × 12.
pub(crate) const MONTHS: usize = YEARS * 12;
/// Number of AD months that hold a supported day, 1844-04 ..= 2143-04.
pub(crate) const AD_MONTHS: usize = 3589;
/// Serial of BS 2199-12-30, the last supported day (ALGORITHM §4).
pub(crate) const MAX_SERIAL: u32 = 109_211;

/// Days in a BS month. `year` must be in range and `month` in 1..=12.
pub(crate) const fn month_length(year: u16, month: u8) -> u8 {
    // SAFETY-OF-INDEX: callers check MIN_YEAR <= year <= MAX_YEAR first, so the index is
    // 0..YEARS. A wrong caller fails at compile time in const evaluation, or with a panic in
    // a test, never silently.
    #[allow(clippy::indexing_slicing)]
    let packed = PACKED_YEARS[(year - MIN_YEAR) as usize];
    29 + ((packed >> ((month - 1) * 2)) & 0b11) as u8
}

/// `MONTH_INFO[i]` = start serial of BS month `i` (`i = (y - 1901) * 12 + m - 1`) `<< 2`, or'ed
/// with `length - 29`. `MONTH_INFO[MONTHS]` = `109_212 << 2`, so `MONTH_INFO[i + 1] >> 2` is
/// always the start of the next month.
#[cfg_attr(
    not(test),
    expect(dead_code, reason = "serial conversion arrives in L2")
)]
pub(crate) static MONTH_INFO: [u32; MONTHS + 1] = build_month_info();

/// AD date of the first day of BS month `i`, packed `year << 9 | month << 5 | day`.
#[cfg_attr(
    not(test),
    expect(dead_code, reason = "BS to AD conversion arrives in L2")
)]
pub(crate) static BS_MONTH_AD: [u32; MONTHS] = build_bs_month_ad();

/// Serial of the first day of AD month `j` (`j = (y - 1844) * 12 + m - 4`). Entry 0 is -10
/// (1844-04-01 is ten days before the epoch).
#[cfg_attr(
    not(test),
    expect(dead_code, reason = "AD to BS conversion arrives in L2")
)]
pub(crate) static AD_MONTH_START: [i32; AD_MONTHS] = build_ad_month_start();

// SAFETY-OF-INDEX (all three builders): the loop variable runs over exactly the array's length,
// and `i / 12 < YEARS` because `i < MONTHS = YEARS * 12`.
#[allow(clippy::indexing_slicing)]
const fn build_month_info() -> [u32; MONTHS + 1] {
    let mut out = [0u32; MONTHS + 1];
    let mut start = 0u32;
    let mut i = 0;
    // `for` loops aren't allowed in a const fn; `while` is.
    while i < MONTHS {
        let extra = (PACKED_YEARS[i / 12] >> ((i % 12) * 2)) & 0b11;
        out[i] = (start << 2) | extra;
        start += 29 + extra;
        i += 1;
    }
    out[MONTHS] = start << 2;
    out
}

#[allow(
    clippy::indexing_slicing,
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss
)]
const fn build_bs_month_ad() -> [u32; MONTHS] {
    let info = build_month_info();
    let mut out = [0u32; MONTHS];
    let mut i = 0;
    while i < MONTHS {
        let start = (info[i] >> 2) as i64;
        let (y, m, d) = civil_from_days(start + EPOCH_UNIX_DAYS as i64);
        // y is 1844..=2143 and m, d are small, so the casts can't truncate.
        out[i] = ((y as u32) << 9) | ((m as u32) << 5) | d as u32;
        i += 1;
    }
    out
}

#[allow(
    clippy::indexing_slicing,
    clippy::cast_possible_truncation,
    clippy::cast_possible_wrap
)]
const fn build_ad_month_start() -> [i32; AD_MONTHS] {
    let mut out = [0i32; AD_MONTHS];
    let mut j = 0;
    while j < AD_MONTHS {
        let months = j as i64 + 3; // months since 1844-01
        let y = 1844 + months / 12;
        let m = months % 12 + 1;
        // Serials of this span fit easily in i32 (-10 ..= 109_211).
        out[j] = (days_from_civil(y, m, 1) - EPOCH_UNIX_DAYS as i64) as i32;
        j += 1;
    }
    out
}

const _: () = assert!(build_month_info()[MONTHS] >> 2 == MAX_SERIAL + 1);
const _: () = assert!(days_from_civil(1844, 4, 11) == EPOCH_UNIX_DAYS as i64);

#[cfg(test)]
#[allow(clippy::indexing_slicing, clippy::needless_range_loop)]
mod tests {
    use super::*;

    fn start(i: usize) -> u32 {
        MONTH_INFO[i] >> 2
    }

    fn unpack_ad(v: u32) -> (u32, u32, u32) {
        (v >> 9, (v >> 5) & 0b1111, v & 0b1_1111)
    }

    #[test]
    fn month_info_ends_and_first_month() {
        assert_eq!(start(MONTHS), 109_212);
        assert_eq!(MONTH_INFO[0], 0b10);
    }

    #[test]
    fn month_lengths_agree_with_month_info() {
        assert_eq!(month_length(1901, 3), 32);
        assert_eq!(month_length(2081, 12), 31);
        for i in 0..MONTHS {
            let year = MIN_YEAR + u16::try_from(i / 12).unwrap_or(0);
            let month = u8::try_from(i % 12 + 1).unwrap_or(0);
            let from_info = start(i + 1) - start(i);
            assert_eq!(
                from_info,
                u32::from(month_length(year, month)),
                "{year}-{month}"
            );
            assert_eq!(
                MONTH_INFO[i] & 0b11,
                u32::from(month_length(year, month) - 29)
            );
        }
    }

    #[test]
    fn known_odd_year_lengths() {
        assert_eq!(start(187 * 12) - start(186 * 12), 367); // 2087
        assert_eq!(start(190 * 12) - start(189 * 12), 364); // 2090
    }

    #[test]
    fn bs_month_ad() {
        assert_eq!(unpack_ad(BS_MONTH_AD[0]), (1844, 4, 11));
        assert_eq!(unpack_ad(BS_MONTH_AD[(2081 - 1901) * 12]), (2024, 4, 13));
    }

    #[test]
    fn ad_month_start() {
        assert_eq!(AD_MONTH_START[0], -10);
        assert_eq!(AD_MONTH_START[3588], 109_197);
    }

    #[test]
    fn packed_years_first_entry() {
        assert_eq!(PACKED_YEARS[0], 0x0051_1ABA);
    }
}
