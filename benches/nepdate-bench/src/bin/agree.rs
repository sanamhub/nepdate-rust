//! Compares each competitor's B1 (BS to AD) and B2 (AD to BS) results on the 64 dates with
//! `nepdate` and prints one line per competitor, followed by every differing date
//! with both answers, so a reader can check each one against the source data. An input a
//! competitor rejects counts as a difference, so a fast wrong answer is never listed as
//! agreeing; rejected inputs are listed as "rejected", not as wrong answers.

#![allow(clippy::unwrap_used, clippy::expect_used)]
// Every indexed array is `[T; 64]` and the index is `0..64`, so no `get` is needed.
#![allow(clippy::indexing_slicing)]

use chrono::Datelike;

use bikram::bikram::Bikram;
use nepdate_bench::inputs::INPUTS;

/// A date as (year, month, day), the shape every comparison is made in.
type Ymd = (i32, u32, u32);

/// One competitor in one direction: for each differing date, the input, nepdate's answer and
/// the competitor's answer, or `None` when it rejected the input.
#[derive(Default)]
struct Tally {
    differ: Vec<(Ymd, Ymd, Option<Ymd>)>,
}

impl Tally {
    fn check(&mut self, input: Ymd, expected: Ymd, got: Option<Ymd>) {
        if got != Some(expected) {
            self.differ.push((input, expected, got));
        }
    }

    fn verdict(&self) -> String {
        let rejected = self.differ.iter().filter(|d| d.2.is_none()).count();
        match (self.differ.len(), rejected) {
            (0, _) => "yes".to_string(),
            (n, 0) => format!("no ({n} of 64 differ)"),
            (n, r) => format!("no ({n} of 64 differ, {r} of them rejected)"),
        }
    }
}

fn ymd((y, m, d): Ymd) -> String {
    format!("{y:04}-{m:02}-{d:02}")
}

fn report(name: &str, b1: &Tally, b2: &Tally) {
    println!("{name}: B1 {}, B2 {}", b1.verdict(), b2.verdict());
    for (label, tally, from, to) in [("B1", b1, "BS", "AD"), ("B2", b2, "AD", "BS")] {
        for &(input, expected, got) in &tally.differ {
            let got = got.map_or_else(|| "rejected".to_string(), ymd);
            println!(
                "  {label} {from} {}: nepdate {to} {}, {name} {got}",
                ymd(input),
                ymd(expected)
            );
        }
    }
}

fn main() {
    let inputs = &*INPUTS;
    let bs_hyphen = inputs.bs_hyphen();
    let ad_hyphen = inputs.ad_hyphen();
    let bs: Vec<Ymd> = inputs
        .parts
        .iter()
        .map(|&(y, m, d)| (i32::from(y), u32::from(m), u32::from(d)))
        .collect();
    let ad: Vec<Ymd> = inputs
        .ad
        .iter()
        .map(|&(y, m, d)| (y, u32::from(m), u32::from(d)))
        .collect();

    println!(
        "nepdate: yes (its vector tests check the 64 against shared/data; competitors are \
         compared to nepdate below)"
    );

    // npdatetime: `new(y, m, d).to_gregorian()` and `from_gregorian(y, m, d)`, both `Result`.
    let (mut b1, mut b2) = (Tally::default(), Tally::default());
    for i in 0..64 {
        let (y, m, d) = inputs.parts[i];
        let got = npdatetime::NepaliDate::new(i32::from(y), m, d)
            .and_then(|n| n.to_gregorian())
            .ok()
            .map(|(gy, gm, gd)| (gy, u32::from(gm), u32::from(gd)));
        b1.check(bs[i], ad[i], got);
        let (ay, am, ad_day) = inputs.ad[i];
        let got = npdatetime::NepaliDate::from_gregorian(ay, am, ad_day)
            .ok()
            .map(|n| (n.year, u32::from(n.month), u32::from(n.day)));
        b2.check(ad[i], bs[i], got);
    }
    report("npdatetime", &b1, &b2);

    // adtobs: `from_bs_ymd(y, m, d).to_gregorian()` and `from_gregorian_ymd(y, m, d)`.
    let (mut b1, mut b2) = (Tally::default(), Tally::default());
    for i in 0..64 {
        let (y, m, d) = bs[i];
        let got = adtobs::NepaliDate::from_bs_ymd(y, m, d)
            .ok()
            .map(|n| n.to_gregorian())
            .map(|g| (g.year(), g.month(), g.day()));
        b1.check(bs[i], ad[i], got);
        let (ay, am, ad_day) = ad[i];
        let got = adtobs::NepaliDate::from_gregorian_ymd(ay, am, ad_day)
            .ok()
            .map(|n| (n.year(), u32::from(n.month().number()), u32::from(n.day())));
        b2.check(ad[i], bs[i], got);
    }
    report("adtobs", &b1, &b2);

    // nepali-date-converter: string API `bs_to_ad("YYYY-MM-DD")` / `ad_to_bs("YYYY-MM-DD")`.
    let (mut b1, mut b2) = (Tally::default(), Tally::default());
    for i in 0..64 {
        let got = nepali_date_converter::bs_to_ad(&bs_hyphen[i]).map(|g| (g.year, g.month, g.day));
        b1.check(bs[i], ad[i], got);
        let got = nepali_date_converter::ad_to_bs(&ad_hyphen[i]).map(|n| (n.year, n.month, n.day));
        b2.check(ad[i], bs[i], got);
    }
    report("nepali-date-converter", &b1, &b2);

    // nepdate-cli (lib `bikram`): `to_gregorian(y, m, d)` and `from_gregorian` plus getters.
    // It has no error path, so every answer is compared.
    let (mut b1, mut b2) = (Tally::default(), Tally::default());
    for i in 0..64 {
        let (y, m, d) = inputs.parts[i];
        let (gy, gm, gd) = Bikram::new().to_gregorian(i32::from(y), i32::from(m), i32::from(d));
        let got = u32::try_from(gm).ok().zip(u32::try_from(gd).ok());
        b1.check(bs[i], ad[i], got.map(|(gm, gd)| (gy, gm, gd)));
        let (ay, am, ad_day) = inputs.ad[i];
        let mut b = Bikram::new();
        b.from_gregorian(ay, i32::from(am), i32::from(ad_day));
        let got = u32::try_from(b.get_month())
            .ok()
            .zip(u32::try_from(b.get_day()).ok());
        b2.check(ad[i], bs[i], got.map(|(m, d)| (b.get_year(), m, d)));
    }
    report("nepdate-cli", &b1, &b2);
}
