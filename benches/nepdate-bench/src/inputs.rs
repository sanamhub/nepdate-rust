//! The 64 inputs of `shared/spec/BENCHMARK.md` §2, built once and shared by the criterion
//! benches (`benches/compare.rs`) and the `alloc` and `agree` binaries.
//!
//! The 64 dates are serial `s0 + i * ((s1 - s0) / 63)` for `i` in `0..64`, where `s0` is the
//! serial of BS 2000-01-01 and `s1` of 2090-12-30; the inputs for B7 and B8 are the same formula
//! over the patro coverage (`PATRO.md` §2). Building them is not timed.

#![allow(clippy::unwrap_used, clippy::expect_used)]

use std::sync::LazyLock;

use nepdate::NepaliDate;

/// Serial of BS 2000-01-01 (ALGORITHM §4): 36,161 days after 1901-01-01.
const S0: i32 = 36_161;

/// Everything the operations need, built once.
pub struct Inputs {
    /// The 64 BS dates of BENCHMARK §2, starting at 2000-01-01.
    pub bs: [NepaliDate; 64],
    /// The (year, month, day) parts of [`Inputs::bs`], for B1.
    pub parts: [(u16, u8, u8); 64],
    /// The AD equivalents of [`Inputs::bs`], for B2.
    pub ad: [(i32, u8, u8); 64],
    /// [`Inputs::bs`] formatted `YYYY/MM/DD` (ASCII), for B6.
    pub text: [String; 64],
    /// The 64 dates moved into the patro coverage, for B7 and B8.
    pub patro: [NepaliDate; 64],
}

/// Built once on first use, so the timed loops never pay for it.
pub static INPUTS: LazyLock<Inputs> = LazyLock::new(Inputs::build);

impl Inputs {
    fn build() -> Self {
        // BENCHMARK §2: serial s0 + i * ((s1 - s0) / 63), s0 = 2000-01-01, s1 = 2090-12-30.
        let s1 = NepaliDate::MIN.days_until(NepaliDate::new(2090, 12, 30).unwrap());
        let step = (s1 - S0) / 63;
        let bs = std::array::from_fn(|i| {
            NepaliDate::MIN
                .checked_add_days(S0 + step * i32::try_from(i).unwrap())
                .unwrap()
        });
        assert_eq!(bs[0].to_string(), "2000/01/01");
        // Integer division means the last date is at most `s1`, not necessarily exactly it.
        assert!(NepaliDate::MIN.days_until(bs[63]) <= s1);

        let parts = bs.map(|d| (d.year(), d.month_number(), d.day()));
        let ad = bs.map(|d| {
            let g = d.to_gregorian();
            (g.year(), g.month(), g.day())
        });
        let text = bs.map(|d| d.to_string());

        let coverage = nepdate_patro::coverage();
        let step = coverage.start().days_until(coverage.end()) / 63;
        let patro = std::array::from_fn(|i| {
            coverage
                .start()
                .checked_add_days(i32::try_from(i).unwrap() * step)
                .unwrap()
        });
        assert_eq!(patro[0], coverage.start());
        assert!(
            coverage.start().days_until(patro[63]) <= coverage.start().days_until(coverage.end())
        );

        Self {
            bs,
            parts,
            ad,
            text,
            patro,
        }
    }

    /// `YYYY-MM-DD` hyphen strings, for `nepali-date-converter` (its only API).
    #[must_use]
    pub fn bs_hyphen(&self) -> [String; 64] {
        self.parts.map(|(y, m, d)| format!("{y:04}-{m:02}-{d:02}"))
    }

    /// `YYYY-MM-DD` hyphen strings of the AD equivalents, for `nepali-date-converter`.
    #[must_use]
    pub fn ad_hyphen(&self) -> [String; 64] {
        self.ad.map(|(y, m, d)| format!("{y:04}-{m:02}-{d:02}"))
    }
}
