//! Patro data for `cargo xtask codegen` and `cargo xtask data-status`: reads
//! `shared/data/patro/*.json` and writes the dense per-day tables of ADR-0004 §6 into
//! `crates/nepdate-patro/src/generated/patro.rs`.

// `serde_json::Value[...]` returns `Null` for a missing key or index; it never panics.
#![allow(clippy::indexing_slicing)]

use std::collections::{BTreeMap, HashMap};
use std::fmt::Write as _;
use std::fs;
use std::path::Path;

use anyhow::{Context, Result, bail};
use serde_json::Value;

use crate::codegen::{MIN_YEAR, read_json};

/// A BS date as it appears in the JSON keys.
pub(crate) type Bs = (i64, u8, u8);

/// Month lengths per BS year, from `shared/data/calendar/bs-calendar.json`.
pub(crate) type Lengths = BTreeMap<i64, [u8; 12]>;

/// A `(np, en)` name pair.
type Name = (String, String);

/// What one JSON day says. Days without an entry have none of it.
#[derive(Default)]
pub(crate) struct Day {
    pub(crate) tithi: Option<Name>,
    pub(crate) holiday: bool,
    pub(crate) events: Vec<Name>,
}

/// Every day of every patro file, in date order, plus the source commit for the header.
pub(crate) struct Patro {
    pub(crate) commit: String,
    pub(crate) days: BTreeMap<Bs, Day>,
}

pub(crate) fn read_lengths(root: &Path) -> Result<Lengths> {
    let json = read_json(&root.join("shared/data/calendar/bs-calendar.json"))?;
    let object = json["month_lengths"]
        .as_object()
        .context("month_lengths missing")?;
    let mut lengths = Lengths::new();
    for (year, months) in object {
        let year: i64 = year.parse().with_context(|| format!("year key {year}"))?;
        let mut row = [0u8; 12];
        let months = months.as_array().context("month_lengths row")?;
        if months.len() != 12 {
            bail!("year {year}: {} months, expected 12", months.len());
        }
        for (slot, value) in row.iter_mut().zip(months) {
            *slot = u8::try_from(value.as_u64().unwrap_or(0))?;
        }
        lengths.insert(year, row);
    }
    Ok(lengths)
}

/// Reads `shared/data/patro/*.json` in file-name (= year) order.
pub(crate) fn read(root: &Path) -> Result<Patro> {
    let dir = root.join("shared/data/patro");
    let mut files: Vec<_> = fs::read_dir(&dir)
        .with_context(|| format!("reading {}", dir.display()))?
        .map(|entry| entry.map(|e| e.path()))
        .collect::<Result<_, _>>()?;
    files.retain(|path| path.extension().is_some_and(|ext| ext == "json"));
    files.sort();
    if files.is_empty() {
        bail!("no patro files in {}", dir.display());
    }

    let mut commit = String::new();
    let mut days = BTreeMap::new();
    for path in &files {
        let json = read_json(path)?;
        let shown = path.display();
        if json["schema"].as_i64() != Some(1) {
            bail!("{shown}: expected schema 1");
        }
        let file_commit = json["source"]["commit"]
            .as_str()
            .with_context(|| format!("{shown}: source.commit missing"))?;
        if commit.is_empty() {
            file_commit.clone_into(&mut commit);
        } else if commit != file_commit {
            bail!("{shown}: source.commit {file_commit} differs from {commit}");
        }
        let year = json["year"]
            .as_i64()
            .with_context(|| format!("{shown}: year missing"))?;
        let entries = json["days"]
            .as_object()
            .with_context(|| format!("{shown}: days missing"))?;
        for (key, value) in entries {
            let date = parse_key(key).with_context(|| format!("{shown}: day {key}"))?;
            if date.0 != year {
                bail!("{shown}: day {key} is not in year {year}");
            }
            let day = parse_day(value).with_context(|| format!("{shown}: day {key}"))?;
            if days.insert(date, day).is_some() {
                bail!("{shown}: day {key} appears twice");
            }
        }
    }
    Ok(Patro { commit, days })
}

fn parse_key(key: &str) -> Result<Bs> {
    let mut parts = key.split('-');
    let (Some(y), Some(m), Some(d), None) =
        (parts.next(), parts.next(), parts.next(), parts.next())
    else {
        bail!("expected YYYY-MM-DD");
    };
    Ok((y.parse()?, m.parse()?, d.parse()?))
}

fn parse_name(value: &Value) -> Result<Name> {
    let np = value["np"].as_str().context("np missing")?;
    let en = value["en"].as_str().context("en missing")?;
    if np.is_empty() || en.is_empty() {
        bail!("empty name ({np:?}, {en:?})");
    }
    Ok((np.to_owned(), en.to_owned()))
}

fn parse_day(value: &Value) -> Result<Day> {
    let tithi = match &value["tithi"] {
        Value::Null => None,
        tithi => Some(parse_name(tithi).context("tithi")?),
    };
    let holiday = match &value["holiday"] {
        Value::Null => false,
        Value::Bool(b) => *b,
        other => bail!("holiday is {other}, expected a boolean"),
    };
    let events = match &value["events"] {
        Value::Null => Vec::new(),
        Value::Array(list) => list
            .iter()
            .map(parse_name)
            .collect::<Result<_>>()
            .context("events")?,
        other => bail!("events is {other}, expected an array"),
    };
    Ok(Day {
        tithi,
        holiday,
        events,
    })
}

/// Length of a BS month, or an error when the year isn't in the calendar data.
fn month_length(lengths: &Lengths, year: i64, month: u8) -> Result<u8> {
    let row = lengths
        .get(&year)
        .with_context(|| format!("year {year} is outside the calendar data"))?;
    let index = usize::from(month).checked_sub(1).context("month 0")?;
    row.get(index)
        .copied()
        .with_context(|| format!("month {month} of {year}"))
}

/// ALGORITHM §4: days since BS 1901-01-01. Re-implemented here so xtask doesn't depend on the
/// library it generates tables for.
pub(crate) fn to_serial(lengths: &Lengths, (year, month, day): Bs) -> Result<u32> {
    let length = month_length(lengths, year, month)?;
    if day < 1 || day > length {
        bail!("{year}-{month:02}-{day:02} is not a valid BS date");
    }
    let mut serial = u32::from(day) - 1;
    for y in MIN_YEAR..year {
        serial += lengths
            .get(&y)
            .with_context(|| format!("year {y} missing"))?
            .iter()
            .map(|&len| u32::from(len))
            .sum::<u32>();
    }
    for m in 1..month {
        serial += u32::from(month_length(lengths, year, m)?);
    }
    Ok(serial)
}

/// The day after `date`.
pub(crate) fn next_day(lengths: &Lengths, (year, month, day): Bs) -> Result<Bs> {
    if day < month_length(lengths, year, month)? {
        Ok((year, month, day + 1))
    } else if month < 12 {
        Ok((year, month + 1, 1))
    } else {
        Ok((year + 1, 1, 1))
    }
}

/// Writes `"text"` as a Rust string literal. Unlike `{:?}`, it leaves Devanagari vowel signs
/// readable instead of escaping them as `\u{...}`.
fn literal(out: &mut String, text: &str) -> Result<()> {
    out.push('"');
    for c in text.chars() {
        match c {
            '"' | '\\' => {
                out.push('\\');
                out.push(c);
            }
            c if c.is_control() || matches!(c, '\u{200B}'..='\u{200F}' | '\u{FEFF}') => {
                write!(out, "\\u{{{:X}}}", u32::from(c))?;
            }
            c => out.push(c),
        }
    }
    out.push('"');
    Ok(())
}

/// Index of `name` in `pool`, adding it at the end the first time it is seen.
fn intern(pool: &mut Vec<Name>, index: &mut HashMap<Name, usize>, name: &Name) -> usize {
    *index.entry(name.clone()).or_insert_with(|| {
        pool.push(name.clone());
        pool.len() - 1
    })
}

fn bs(date: Bs) -> String {
    format!("{}-{:02}-{:02}", date.0, date.1, date.2)
}

/// The per-day tables before they are written out, all indexed by `o`.
struct Tables {
    first: Bs,
    last: Bs,
    patro_first: u32,
    /// The BS date of each `o`, for the month comments and error messages.
    dates: Vec<Bs>,
    day_tithi: Vec<Option<usize>>,
    holidays: Vec<usize>,
    day_events: Vec<usize>,
    event_ids: Vec<usize>,
    names: Vec<Name>,
    tithi_names: Vec<Name>,
}

/// Walks every day of the coverage once and builds the tables (ADR-0004 §6).
fn tables(lengths: &Lengths, patro: &Patro) -> Result<Tables> {
    let (Some((&first, _)), Some((&last, _))) =
        (patro.days.first_key_value(), patro.days.last_key_value())
    else {
        bail!("patro data has no days");
    };
    let patro_first = to_serial(lengths, first)?;
    let patro_days = usize::try_from(to_serial(lengths, last)? - patro_first + 1)?;

    let mut t = Tables {
        first,
        last,
        patro_first,
        dates: Vec::with_capacity(patro_days),
        day_tithi: Vec::with_capacity(patro_days),
        holidays: Vec::new(),
        day_events: Vec::with_capacity(patro_days + 1),
        event_ids: Vec::new(),
        names: Vec::new(),
        tithi_names: Vec::new(),
    };
    let mut tithi_index = HashMap::new();
    let mut name_index = HashMap::new();
    let mut date = first;
    for o in 0..patro_days {
        t.dates.push(date);
        t.day_events.push(t.event_ids.len());
        let day = patro.days.get(&date);
        let tithi = day.and_then(|day| day.tithi.as_ref());
        t.day_tithi
            .push(tithi.map(|name| intern(&mut t.tithi_names, &mut tithi_index, name)));
        if day.is_some_and(|day| day.holiday) {
            t.holidays.push(o);
        }
        for event in day.map_or(&[][..], |day| &day.events) {
            t.event_ids
                .push(intern(&mut t.names, &mut name_index, event));
        }
        date = next_day(lengths, date)?;
    }
    t.day_events.push(t.event_ids.len());

    // PATRO §3: tithi data is contiguous, so a missing day inside its range is a data error.
    let from = t.day_tithi.iter().position(Option::is_some);
    let to = t.day_tithi.iter().rposition(Option::is_some);
    if let (Some(from), Some(to)) = (from, to)
        && let Some(gap) = (from..=to).find(|&o| t.day_tithi[o].is_none())
    {
        bail!(
            "tithi range {} ..= {} has a gap: {} has no tithi",
            bs(t.dates[from]),
            bs(t.dates[to]),
            bs(t.dates[gap])
        );
    }
    if t.tithi_names.len() >= usize::from(u8::MAX) {
        bail!("{} tithi names don't fit a u8 index", t.tithi_names.len());
    }
    if t.names.len() > usize::from(u16::MAX) || t.event_ids.len() > usize::from(u16::MAX) {
        bail!(
            "{} names or {} events don't fit a u16",
            t.names.len(),
            t.event_ids.len()
        );
    }
    Ok(t)
}

/// The text of `crates/nepdate-patro/src/generated/patro.rs` (layout: ADR-0004 §6).
pub(crate) fn generate(lengths: &Lengths, patro: &Patro) -> Result<String> {
    let t = tables(lengths, patro)?;
    let (first, last) = (t.first, t.last);
    let mut out = String::new();
    writeln!(
        out,
        "// @generated by `cargo xtask codegen` from shared/data/patro/*.json. DO NOT EDIT.
// Source: https://github.com/RajuPrasai/NepDate @ {} (MIT). See shared/data/NOTICE.
// Layout: ADR-0004 §6. Every per-day table is indexed by `o = serial - PATRO_FIRST`.

/// Serial day (days since BS 1901-01-01) of `COVERAGE_FIRST`.
pub(crate) const PATRO_FIRST: u32 = {};
/// Days from `COVERAGE_FIRST` to `COVERAGE_LAST`, both included.
pub(crate) const PATRO_DAYS: usize = {};
/// `DAY_TITHI` entry of a day without a tithi.
pub(crate) const NO_TITHI: u8 = u8::MAX;
/// First BS day with any data.
pub(crate) const COVERAGE_FIRST: (u16, u8, u8) = ({}, {}, {});
/// Last BS day with any data.
pub(crate) const COVERAGE_LAST: (u16, u8, u8) = ({}, {}, {});
",
        patro.commit,
        t.patro_first,
        t.dates.len(),
        first.0,
        first.1,
        first.2,
        last.0,
        last.1,
        last.2
    )?;

    out.push_str(
        "/// Tithi of day `o`: an index into `TITHI_NAMES`, or `NO_TITHI` (255).
#[rustfmt::skip]
pub(crate) static DAY_TITHI: [u8; PATRO_DAYS] = [
",
    );
    per_month(&mut out, &t.dates, |o| {
        t.day_tithi[o].unwrap_or(usize::from(u8::MAX))
    })?;
    out.push_str("];\n\n");

    let mut bits = vec![0u64; t.dates.len().div_ceil(64)];
    for &o in &t.holidays {
        bits[o / 64] |= 1 << (o % 64);
    }
    out.push_str(
        "/// Bit `o % 64` of word `o / 64` is set when day `o` is a public holiday.
#[rustfmt::skip]
pub(crate) static HOLIDAY_BITS: [u64; PATRO_DAYS.div_ceil(64)] = [
",
    );
    for word in &bits {
        let hex = format!("{word:016X}");
        let groups: Vec<&str> = (0..4).map(|i| &hex[i * 4..i * 4 + 4]).collect();
        writeln!(out, "    0x{},", groups.join("_"))?;
    }
    out.push_str("];\n\n");

    writeln!(
        out,
        "/// `o` of every public holiday, ascending, for `public_holidays()`.
#[rustfmt::skip]
pub(crate) static HOLIDAY_OFFSETS: [u16; {}] = [",
        t.holidays.len()
    )?;
    numbers(&mut out, &t.holidays)?;
    out.push_str("];\n\n");

    out.push_str(
        "/// The events of day `o` are `EVENT_IDS[DAY_EVENTS[o]..DAY_EVENTS[o + 1]]`.
#[rustfmt::skip]
pub(crate) static DAY_EVENTS: [u16; PATRO_DAYS + 1] = [
",
    );
    per_month(&mut out, &t.dates, |o| t.day_events[o])?;
    writeln!(out, "    // end\n    {},", t.event_ids.len())?;
    out.push_str("];\n\n");

    writeln!(
        out,
        "/// Event name of each event: an index into `NAMES`, in file order per day.
#[rustfmt::skip]
pub(crate) static EVENT_IDS: [u16; {}] = [",
        t.event_ids.len()
    )?;
    numbers(&mut out, &t.event_ids)?;
    out.push_str("];\n\n");

    writeln!(
        out,
        "/// Event names `(np, en)`, deduplicated, in first-seen order.
#[rustfmt::skip]
pub(crate) static NAMES: [(&str, &str); {}] = [",
        t.names.len()
    )?;
    pairs(&mut out, &t.names)?;
    out.push_str("];\n\n");

    writeln!(
        out,
        "/// Tithi names `(np, en)`, in first-seen order.
#[rustfmt::skip]
pub(crate) static TITHI_NAMES: [(&str, &str); {}] = [",
        t.tithi_names.len()
    )?;
    pairs(&mut out, &t.tithi_names)?;
    out.push_str("];\n");
    Ok(out)
}

/// One line per month of `dates`, after a `// <year>-<month>` comment.
fn per_month(out: &mut String, dates: &[Bs], value: impl Fn(usize) -> usize) -> Result<()> {
    let mut o = 0;
    while let Some(&(year, month, _)) = dates.get(o) {
        let run: Vec<String> = (o..dates.len())
            .take_while(|&i| dates[i].0 == year && dates[i].1 == month)
            .map(|i| value(i).to_string())
            .collect();
        writeln!(out, "    // {year}-{month:02}\n    {},", run.join(", "))?;
        o += run.len();
    }
    Ok(())
}

/// Sixteen numbers per line.
fn numbers(out: &mut String, values: &[usize]) -> Result<()> {
    for line in values.chunks(16) {
        let shown: Vec<String> = line.iter().map(ToString::to_string).collect();
        writeln!(out, "    {},", shown.join(", "))?;
    }
    Ok(())
}

fn pairs(out: &mut String, values: &[Name]) -> Result<()> {
    for (np, en) in values {
        out.push_str("    (");
        literal(out, np)?;
        out.push_str(", ");
        literal(out, en)?;
        out.push_str("),\n");
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used)]

    use super::*;

    /// Every year 1901 to 2199 with 30-day months: enough for `to_serial` on small inputs.
    fn flat_lengths() -> Lengths {
        (1901..=2199).map(|y| (y, [30; 12])).collect()
    }

    fn tithi_day(name: &str) -> Day {
        Day {
            tithi: Some((name.to_owned(), name.to_owned())),
            ..Day::default()
        }
    }

    #[test]
    fn tithi_gap_names_the_missing_date() {
        let mut days = BTreeMap::new();
        days.insert((2080, 1, 29), tithi_day("A"));
        days.insert((2080, 1, 30), tithi_day("B"));
        // 2080-02-01 is missing.
        days.insert((2080, 2, 2), tithi_day("C"));
        let patro = Patro {
            commit: "test".to_owned(),
            days,
        };
        let err = generate(&flat_lengths(), &patro).unwrap_err().to_string();
        assert!(err.contains("2080-02-01 has no tithi"), "{err}");
    }

    #[test]
    fn contiguous_tithi_and_names_are_deduplicated() {
        let mut days = BTreeMap::new();
        days.insert((2080, 1, 1), tithi_day("A"));
        let mut second = tithi_day("A");
        second.holiday = true;
        second.events = vec![
            ("x".to_owned(), "X".to_owned()),
            ("y".to_owned(), "Y".to_owned()),
        ];
        days.insert((2080, 1, 2), second);
        let mut third = tithi_day("B");
        third.events = vec![("x".to_owned(), "X".to_owned())];
        days.insert((2080, 1, 3), third);
        let patro = Patro {
            commit: "test".to_owned(),
            days,
        };
        let text = generate(&flat_lengths(), &patro).unwrap();
        assert!(text.contains("PATRO_DAYS: usize = 3;"), "{text}");
        assert!(text.contains("    // 2080-01\n    0, 0, 1,\n"), "{text}");
        assert!(
            text.contains("HOLIDAY_OFFSETS: [u16; 1] = [\n    1,\n"),
            "{text}"
        );
        assert!(
            text.contains("    0, 0, 2,\n    // end\n    3,\n"),
            "{text}"
        );
        assert!(
            text.contains("EVENT_IDS: [u16; 3] = [\n    0, 1, 0,\n"),
            "{text}"
        );
        assert!(text.contains("NAMES: [(&str, &str); 2]"), "{text}");
    }

    #[test]
    fn serial_of_the_epoch_and_a_year_later() {
        let lengths = flat_lengths();
        assert_eq!(to_serial(&lengths, (1901, 1, 1)).unwrap(), 0);
        assert_eq!(to_serial(&lengths, (1902, 1, 1)).unwrap(), 360);
        assert!(to_serial(&lengths, (1902, 1, 31)).is_err());
    }

    #[test]
    fn literal_escapes_quotes_and_keeps_devanagari() {
        let mut out = String::new();
        literal(&mut out, "a\"b\\ कुमारी").unwrap();
        assert_eq!(out, "\"a\\\"b\\\\ कुमारी\"");
    }
}
