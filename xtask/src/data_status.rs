//! `cargo xtask data-status [--today YYYY-MM-DD]` (L5-01b): when each kind of patro data ends,
//! and whether a release still has half a year of tithi and holidays left (W4-00 gate).

use std::time::{SystemTime, UNIX_EPOCH};

use anyhow::{Context, Result, bail};

use crate::codegen::{EPOCH_UNIX_DAYS, days_from_civil, workspace_root};
use crate::patro::{Bs, Day, Lengths, read, read_lengths, to_serial};

/// Tithi or holiday data must reach at least this many days past today.
const MIN_DAYS_LEFT: i64 = 183;

/// A data kind: its name, whether a day has it, and whether the release gate checks it.
type Kind = (&'static str, fn(&Day) -> bool, bool);

pub(crate) fn run(today: Option<&str>) -> Result<()> {
    let today = match today {
        Some(text) => parse_ad(text)?,
        None => utc_today()?,
    };
    let root = workspace_root();
    let lengths = read_lengths(&root)?;
    let patro = read(&root)?;
    let kinds: [Kind; 3] = [
        ("tithi", |d| d.tithi.is_some(), true),
        ("holidays", |d| d.holiday, true),
        ("events", |d| !d.events.is_empty(), false),
    ];
    println!("today (AD): {}", ad(today));
    let mut short = Vec::new();
    for (kind, has, gated) in kinds {
        let mut dates = patro
            .days
            .iter()
            .filter(|(_, day)| has(day))
            .map(|(&d, _)| d);
        let first = dates.next().with_context(|| format!("no {kind} data"))?;
        let last = dates.next_back().unwrap_or(first);
        let last_ad = ad_days(&lengths, last)?;
        let left = last_ad - today;
        println!(
            "{kind}: BS {} ..= {} (AD {} ..= {}), {left} days left",
            bs(first),
            bs(last),
            ad(ad_days(&lengths, first)?),
            ad(last_ad)
        );
        if gated && left < MIN_DAYS_LEFT {
            short.push(format!("{kind} ({left} days)"));
        }
    }
    if !short.is_empty() {
        bail!(
            "data-status: less than {MIN_DAYS_LEFT} days of data left for {}",
            short.join(", ")
        );
    }
    Ok(())
}

/// Days since 1970-01-01 of a BS date.
fn ad_days(lengths: &Lengths, date: Bs) -> Result<i64> {
    Ok(EPOCH_UNIX_DAYS + i64::from(to_serial(lengths, date)?))
}

fn utc_today() -> Result<i64> {
    let secs = SystemTime::now().duration_since(UNIX_EPOCH)?.as_secs();
    Ok(i64::try_from(secs / 86_400)?)
}

fn parse_ad(text: &str) -> Result<i64> {
    let parts: Vec<i64> = text
        .split('-')
        .map(str::parse)
        .collect::<Result<_, _>>()
        .with_context(|| format!("--today {text}: expected YYYY-MM-DD"))?;
    let [y, m, d] = parts[..] else {
        bail!("--today {text}: expected YYYY-MM-DD");
    };
    let days = days_from_civil(y, m, d);
    if ad(days) != format!("{y:04}-{m:02}-{d:02}") {
        bail!("--today {text}: not a real date");
    }
    Ok(days)
}

fn bs((y, m, d): Bs) -> String {
    format!("{y}-{m:02}-{d:02}")
}

/// `YYYY-MM-DD` of a day count since 1970-01-01 (ALGORITHM §5, Hinnant's `civil_from_days`).
fn ad(days: i64) -> String {
    let z = days + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z.rem_euclid(146_097);
    let yoe = (doe - doe / 1_460 + doe / 36_524 - doe / 146_096) / 365;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    let y = yoe + era * 400 + i64::from(m <= 2);
    format!("{y:04}-{m:02}-{d:02}")
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used)]

    use super::*;

    #[test]
    fn ad_round_trips() {
        for text in ["1970-01-01", "1844-04-11", "2027-04-13", "2000-02-29"] {
            assert_eq!(ad(parse_ad(text).unwrap()), text);
        }
        assert!(parse_ad("2026-02-30").is_err());
        assert!(parse_ad("2026-10").is_err());
    }
}
