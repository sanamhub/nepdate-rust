//! Compares each competitor's B1 (BS to AD) and B2 (AD to BS) results on the 64 dates with
//! `nepdate` and prints one line per competitor (L6-02 AC3). An input a competitor rejects
//! counts as a difference, so a fast wrong answer is never listed as agreeing.

#![allow(clippy::unwrap_used, clippy::expect_used)]
// Every indexed array is `[T; 64]` and the index is `0..64`, so no `get` is needed.
#![allow(clippy::indexing_slicing)]

use chrono::Datelike;

use bikram::bikram::Bikram;
use nepdate_bench::inputs::INPUTS;

fn verdict(differ: usize) -> String {
    if differ == 0 {
        "yes".to_string()
    } else {
        format!("no ({differ} of 64 differ)")
    }
}

fn main() {
    let inputs = &*INPUTS;
    let bs_hyphen = inputs.bs_hyphen();
    let ad_hyphen = inputs.ad_hyphen();

    println!(
        "nepdate: yes (its vector tests check the 64 against shared/data; competitors are \
         compared to nepdate below)"
    );

    // npdatetime: `new(y, m, d).to_gregorian()` and `from_gregorian(y, m, d)`, both `Result`.
    let (mut b1, mut b2) = (0, 0);
    for i in 0..64 {
        let (y, m, d) = inputs.parts[i];
        match npdatetime::NepaliDate::new(i32::from(y), m, d).and_then(|n| n.to_gregorian()) {
            Ok(ad) if ad == inputs.ad[i] => {}
            _ => b1 += 1,
        }
        let (ay, am, ad) = inputs.ad[i];
        match npdatetime::NepaliDate::from_gregorian(ay, am, ad) {
            Ok(n) if (n.year, n.month, n.day) == (i32::from(y), m, d) => {}
            _ => b2 += 1,
        }
    }
    println!("npdatetime: B1 {}, B2 {}", verdict(b1), verdict(b2));

    // adtobs: `from_bs_ymd(y, m, d).to_gregorian()` and `from_gregorian_ymd(y, m, d)`.
    let (mut b1, mut b2) = (0, 0);
    for i in 0..64 {
        let (y, m, d) = inputs.parts[i];
        match adtobs::NepaliDate::from_bs_ymd(i32::from(y), u32::from(m), u32::from(d))
            .map(|n| n.to_gregorian())
        {
            Ok(ad)
                if (ad.year(), ad.month(), ad.day())
                    == (
                        inputs.ad[i].0,
                        u32::from(inputs.ad[i].1),
                        u32::from(inputs.ad[i].2),
                    ) => {}
            _ => b1 += 1,
        }
        let (ay, am, ad) = inputs.ad[i];
        match adtobs::NepaliDate::from_gregorian_ymd(ay, u32::from(am), u32::from(ad)) {
            Ok(n) if (n.year(), n.month().number(), n.day()) == (i32::from(y), m, d) => {}
            _ => b2 += 1,
        }
    }
    println!("adtobs: B1 {}, B2 {}", verdict(b1), verdict(b2));

    // nepali-date-converter: string API `bs_to_ad("YYYY-MM-DD")` / `ad_to_bs("YYYY-MM-DD")`.
    let (mut b1, mut b2) = (0, 0);
    for i in 0..64 {
        let (y, m, d) = inputs.parts[i];
        match nepali_date_converter::bs_to_ad(&bs_hyphen[i]) {
            Some(ad)
                if (ad.year, ad.month, ad.day)
                    == (
                        inputs.ad[i].0,
                        u32::from(inputs.ad[i].1),
                        u32::from(inputs.ad[i].2),
                    ) => {}
            _ => b1 += 1,
        }
        match nepali_date_converter::ad_to_bs(&ad_hyphen[i]) {
            Some(n) if (n.year, n.month, n.day) == (i32::from(y), u32::from(m), u32::from(d)) => {}
            _ => b2 += 1,
        }
    }
    println!(
        "nepali-date-converter: B1 {}, B2 {}",
        verdict(b1),
        verdict(b2)
    );

    // nepdate-cli (lib `bikram`): `to_gregorian(y, m, d)` and `from_gregorian` plus getters.
    let (mut b1, mut b2) = (0, 0);
    for i in 0..64 {
        let (y, m, d) = inputs.parts[i];
        let ad = Bikram::new().to_gregorian(i32::from(y), i32::from(m), i32::from(d));
        if ad
            != (
                inputs.ad[i].0,
                i32::from(inputs.ad[i].1),
                i32::from(inputs.ad[i].2),
            )
        {
            b1 += 1;
        }
        let (ay, am, ad) = inputs.ad[i];
        let mut b = Bikram::new();
        b.from_gregorian(ay, i32::from(am), i32::from(ad));
        if (b.get_year(), b.get_month(), b.get_day()) != (i32::from(y), i32::from(m), i32::from(d))
        {
            b2 += 1;
        }
    }
    println!("nepdate-cli: B1 {}, B2 {}", verdict(b1), verdict(b2));
}
