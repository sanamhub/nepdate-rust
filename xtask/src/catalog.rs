//! `cargo xtask catalog-check <phase> [--list]`: proves every row of the C# test catalogue
//! (`shared/spec/vectors/csharp-tests/catalog.tsv`) for one PORT-PLAN phase has a Rust test or a
//! deviation comment.

use std::collections::BTreeSet;
use std::fs;
use std::path::Path;

use anyhow::{Context, Result, bail};

use crate::codegen::workspace_root;

const CATALOG: &str = "shared/spec/vectors/csharp-tests/catalog.tsv";
const PARITY: &str = "shared/spec/PARITY.md";

/// One catalogue row we care about: its `id` and the test name made from it.
struct Row {
    id: String,
    name: String,
}

pub(crate) fn run(phase: &str, list: bool) -> Result<()> {
    let root = workspace_root();
    let rows = rows_for_phase(&root, phase)?;
    if rows.is_empty() {
        bail!("no catalogue rows have phase `{phase}`");
    }
    if list {
        for row in &rows {
            println!("{}", row.name);
        }
        return Ok(());
    }

    let deviations = parity_ids(&root)?;
    let sources = test_sources(&root)?;
    let missing: Vec<&Row> = rows
        .iter()
        .filter(|row| !is_covered(row, &sources, &deviations))
        .collect();
    if missing.is_empty() {
        println!("catalog-check {phase}: all {} rows covered", rows.len());
        return Ok(());
    }
    for row in &missing {
        println!(
            "missing: {} (want `fn {}(` or `// cs:{} D-xx`)",
            row.id, row.name, row.id
        );
    }
    bail!(
        "catalog-check {phase}: {} of {} rows have no test",
        missing.len(),
        rows.len()
    )
}

fn rows_for_phase(root: &Path, phase: &str) -> Result<Vec<Row>> {
    let path = root.join(CATALOG);
    let text = fs::read_to_string(&path).with_context(|| format!("reading {}", path.display()))?;
    // Line 1 is a `#` comment and line 2 the header (id, file, method, data, asserts, phase).
    let mut rows = Vec::new();
    for line in text.lines().filter(|l| !l.starts_with('#')).skip(1) {
        let fields: Vec<&str> = line.split('\t').collect();
        let (Some(id), Some(row_phase)) = (fields.first(), fields.get(5)) else {
            bail!("{CATALOG}: row with fewer than 6 columns: {line}");
        };
        if *row_phase == phase {
            rows.push(Row {
                id: (*id).to_owned(),
                name: test_name(id),
            });
        }
    }
    Ok(rows)
}

/// The deviation IDs (`D-01` ...) listed in the PARITY table.
fn parity_ids(root: &Path) -> Result<BTreeSet<String>> {
    let path = root.join(PARITY);
    let text = fs::read_to_string(&path).with_context(|| format!("reading {}", path.display()))?;
    Ok(text
        .lines()
        .filter_map(|line| line.strip_prefix("| D-"))
        .filter_map(|rest| rest.split_whitespace().next())
        .map(|digits| format!("D-{digits}"))
        .collect())
}

/// Contents of every `crates/*/tests/*.rs` (top level only: `tests/common/` holds helpers).
fn test_sources(root: &Path) -> Result<Vec<String>> {
    let mut sources = Vec::new();
    for krate in fs::read_dir(root.join("crates"))? {
        let tests = krate?.path().join("tests");
        if !tests.is_dir() {
            continue;
        }
        for entry in fs::read_dir(&tests)? {
            let path = entry?.path();
            if path.extension().is_some_and(|ext| ext == "rs") {
                let text = fs::read_to_string(&path)
                    .with_context(|| format!("reading {}", path.display()))?;
                sources.push(text);
            }
        }
    }
    Ok(sources)
}

fn is_covered(row: &Row, sources: &[String], deviations: &BTreeSet<String>) -> bool {
    let test = format!("fn {}(", row.name);
    // The trailing space keeps `Foo#1` from matching a comment for `Foo#10`.
    let comment = format!("// cs:{} ", row.id);
    sources.iter().any(|text| {
        text.contains(&test)
            || text.lines().any(|line| {
                line.trim_start()
                    .strip_prefix(&comment)
                    .and_then(|rest| rest.split_whitespace().next())
                    .is_some_and(|id| deviations.contains(id))
            })
    })
}

/// The Rust test name for a catalogue id: `.` becomes `__`, `#` becomes `_`, CamelCase
/// becomes `snake_case` and anything else outside `[a-z0-9_]` becomes `_`.
fn test_name(id: &str) -> String {
    let chars: Vec<char> = id.chars().collect();
    let mut name = String::from("cs_");
    for (i, &c) in chars.iter().enumerate() {
        let prev = i.checked_sub(1).and_then(|p| chars.get(p)).copied();
        let next = chars.get(i + 1).copied();
        match c {
            '.' => name.push_str("__"),
            'A'..='Z' => {
                // A word starts after a lower-case letter or digit ("addDays"), or at the last
                // capital of an acronym ("ViaIComparable" gives "via_i_comparable").
                let after_word = prev.is_some_and(|p| p.is_ascii_lowercase() || p.is_ascii_digit());
                let ends_acronym = prev.is_some_and(|p| p.is_ascii_uppercase())
                    && next.is_some_and(|n| n.is_ascii_lowercase());
                if after_word || ends_acronym {
                    name.push('_');
                }
                name.push(c.to_ascii_lowercase());
            }
            'a'..='z' | '0'..='9' | '_' => name.push(c),
            // `#` (the theory row number) and anything else.
            _ => name.push('_'),
        }
    }
    name
}

#[cfg(test)]
mod tests {
    use super::test_name;

    #[test]
    fn name_rule_example_from_l2_08() {
        assert_eq!(
            test_name("NepaliDateTests.AddDays_Positive#3"),
            "cs_nepali_date_tests__add_days_positive_3"
        );
    }

    #[test]
    fn name_rule_theory_row() {
        assert_eq!(
            test_name("DictionaryIntegrityTests.KnownReferenceDates_ConvertCorrectly#4"),
            "cs_dictionary_integrity_tests__known_reference_dates_convert_correctly_4"
        );
    }

    #[test]
    fn name_rule_acronym_and_digits() {
        assert_eq!(
            test_name("NepaliDateComparableTests.ArraySort_ViaIComparable_OrdersCorrectly"),
            "cs_nepali_date_comparable_tests__array_sort_via_i_comparable_orders_correctly"
        );
        assert_eq!(
            test_name("T.DayOfYear_FirstDayOfMonth4_EqualsSum_Of_Months1To3_Plus1"),
            "cs_t__day_of_year_first_day_of_month4_equals_sum_of_months1_to3_plus1"
        );
    }
}
