//! Reads `shared/spec/vectors/patro-summary.json`, the counts the hub generates from the same
//! patro data the tables come from. Its shape is fixed, so reading the `"key": value` lines is
//! enough; the workspace has no JSON crate and these tests must not add one.
#![allow(dead_code, clippy::panic, clippy::unwrap_used)]

use std::collections::BTreeMap;
use std::fs;
use std::path::Path;

fn fields() -> BTreeMap<String, String> {
    let path =
        Path::new(env!("CARGO_MANIFEST_DIR")).join("../../shared/spec/vectors/patro-summary.json");
    let text = fs::read_to_string(&path).unwrap_or_else(|err| {
        panic!(
            "reading {}: {err}; shared/ may predate the summary, re-sync it",
            path.display()
        )
    });
    text.lines()
        .filter_map(|line| {
            let (key, value) = line.trim().trim_end_matches(',').split_once(':')?;
            Some((
                key.trim().trim_matches('"').to_owned(),
                value.trim().trim_matches('"').to_owned(),
            ))
        })
        .collect()
}

fn raw(field: &str) -> String {
    fields()
        .remove(field)
        .unwrap_or_else(|| panic!("patro-summary.json has no {field}"))
}

/// The count of `field`.
pub(crate) fn number(field: &str) -> usize {
    let text = raw(field);
    text.parse()
        .unwrap_or_else(|_| panic!("patro-summary.json: {field} is not a number: {text}"))
}

/// The `"YYYY-MM-DD"` value of `field`, as `(year, month, day)`.
pub(crate) fn day(field: &str) -> (u16, u8, u8) {
    let text = raw(field);
    let mut parts = text.split('-');
    let (Some(year), Some(month), Some(day), None) =
        (parts.next(), parts.next(), parts.next(), parts.next())
    else {
        panic!("patro-summary.json: {field} is not YYYY-MM-DD: {text}");
    };
    let bad = |shown: &str| -> ! { panic!("patro-summary.json: {field} is not a date: {shown}") };
    let year = year.parse::<u16>().unwrap_or_else(|_| bad(&text));
    let month = month.parse::<u8>().unwrap_or_else(|_| bad(&text));
    let day = day.parse::<u8>().unwrap_or_else(|_| bad(&text));
    (year, month, day)
}
