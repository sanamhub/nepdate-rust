//! The tithi as a day cell shows it (WR-03 AC4, widget-design §6). Pure Rust.
//!
//! A cell is 48 px wide, and the longest Latin names (`Chaturdashi`, 63 px at the 11 px
//! caption step) do not fit one line. A name that does not fit breaks at a syllable, so the whole
//! name stays readable and no two names share a prefix, which an ellipsis does (`Chatur…` was
//! both `Chaturthi` and `Chaturdashi`). `nepdate-patro` marks a skipped tithi with ` (K)` in
//! Latin and with `*` in Devanagari; the cell shows `*` for both. The tooltip, the day detail and
//! the cell's accessible name keep the full name.

/// One Latin tithi name with its syllable break and whether it fits a cell on one line.
struct Split {
    name: &'static str,
    head: &'static str,
    tail: &'static str,
    /// The plain name fits one line, clear of the cell's 2 px focus ring on both sides.
    fits: bool,
    /// The name with `*` fits one line.
    fits_starred: bool,
}

const fn split(
    name: &'static str,
    head: &'static str,
    tail: &'static str,
    fits: bool,
    fits_starred: bool,
) -> Split {
    Split {
        name,
        head,
        tail,
        fits,
        fits_starred,
    }
}

/// Every Latin base name in `nepdate-patro`'s table. The widths were measured with Noto Sans at
/// 11 px; the interaction tests measure the real layout.
const SPLITS: [Split; 16] = [
    split("Tritiya", "Tri", "tiya", true, true),
    split("Chaturthi", "Chatur", "thi", false, false),
    split("Panchami", "Pan", "chami", false, false),
    split("Shashthi", "Shash", "thi", false, false),
    split("Saptami", "Sap", "tami", true, false),
    split("Ashtami", "Ash", "tami", true, false),
    split("Navami", "Na", "vami", true, false),
    split("Dashami", "Da", "shami", false, false),
    split("Ekadashi", "Eka", "dashi", false, false),
    split("Dwadashi", "Dwa", "dashi", false, false),
    split("Trayodashi", "Trayo", "dashi", false, false),
    split("Chaturdashi", "Chatur", "dashi", false, false),
    split("Purnima", "Pur", "nima", false, false),
    split("Pratipadha", "Prati", "padha", false, false),
    split("Aunsi", "Aun", "si", true, true),
    split("Dwitiya", "Dwi", "tiya", true, false),
];

/// The text of a day cell's tithi line: `name` as `Tithi::name` returns it, with a `\n` where a
/// long Latin name breaks. A Devanagari name is returned as it is; it fits at 12 px.
#[must_use]
pub fn cell_tithi(name: &str) -> String {
    let (base, starred) = match name.strip_suffix(" (K)").or_else(|| name.strip_suffix('*')) {
        Some(base) => (base, true),
        None => (name, false),
    };
    let star = if starred { "*" } else { "" };
    match SPLITS.iter().find(|split| split.name == base) {
        Some(split)
            if !(if starred {
                split.fits_starred
            } else {
                split.fits
            }) =>
        {
            format!("{}-\n{}{star}", split.head, split.tail)
        }
        _ => format!("{base}{star}"),
    }
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeSet;

    use nepdate::Lang;
    use nepdate_patro as patro;

    use super::*;

    /// Every tithi name the data uses, in `lang`.
    fn names(lang: Lang) -> BTreeSet<&'static str> {
        patro::coverage()
            .into_iter()
            .filter_map(|date| patro::info(date).tithi())
            .map(|tithi| tithi.name(lang))
            .collect()
    }

    #[test]
    fn the_data_uses_all_32_names_in_both_scripts() {
        assert_eq!(names(Lang::En).len(), 32);
        assert_eq!(names(Lang::Ne).len(), 32);
    }

    #[test]
    fn a_name_that_fits_stays_on_one_line() {
        assert_eq!(cell_tithi("Saptami"), "Saptami");
        assert_eq!(cell_tithi("Aunsi"), "Aunsi");
        assert_eq!(cell_tithi("Tritiya (K)"), "Tritiya*");
    }

    #[test]
    fn a_long_name_breaks_at_a_syllable_and_keeps_every_letter() {
        assert_eq!(cell_tithi("Chaturdashi"), "Chatur-\ndashi");
        assert_eq!(cell_tithi("Chaturthi"), "Chatur-\nthi");
        assert_eq!(cell_tithi("Pratipadha"), "Prati-\npadha");
    }

    #[test]
    fn the_skipped_mark_is_a_star_after_the_last_syllable() {
        assert_eq!(cell_tithi("Chaturdashi (K)"), "Chatur-\ndashi*");
        assert_eq!(cell_tithi("Saptami (K)"), "Sap-\ntami*");
    }

    #[test]
    fn a_devanagari_name_is_returned_as_it_is() {
        for name in names(Lang::Ne) {
            assert_eq!(cell_tithi(name), name);
        }
    }

    #[test]
    fn every_latin_name_gives_at_most_two_lines_and_loses_no_letter() {
        for name in names(Lang::En) {
            let label = cell_tithi(name);
            assert!(label.lines().count() <= 2, "{name}: {label:?}");
            let letters =
                |text: &str| -> String { text.chars().filter(|c| c.is_alphabetic()).collect() };
            assert_eq!(
                letters(&label),
                letters(name.trim_end_matches(" (K)")),
                "{name}"
            );
            assert_eq!(label.ends_with('*'), name.ends_with("(K)"), "{name}");
        }
    }

    #[test]
    fn the_table_holds_exactly_the_base_names_of_the_data() {
        let bases: BTreeSet<&str> = names(Lang::En)
            .into_iter()
            .map(|name| name.trim_end_matches(" (K)"))
            .collect();
        let table: BTreeSet<&str> = SPLITS.iter().map(|split| split.name).collect();
        assert_eq!(bases, table);
    }

    #[test]
    fn a_break_joins_back_to_its_name() {
        for split in &SPLITS {
            assert_eq!(format!("{}{}", split.head, split.tail), split.name);
        }
    }
}
