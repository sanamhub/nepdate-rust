//! Prints one BS month as a grid, like the Unix `cal` command.
//!
//! ```text
//! cargo run -p nepdate --example month_calendar -- 2081 4
//! cargo run -p nepdate --example month_calendar -- 2081 4 ne
//! ```

use std::env;
use std::fmt::Write as _;
use std::process::ExitCode;

use nepdate::{Lang, NepaliDateRange, Weekday};

const USAGE: &str = "usage: month_calendar <year> <month 1-12> [en|ne]";

fn main() -> ExitCode {
    let args: Vec<String> = env::args().skip(1).collect();
    let args: Vec<&str> = args.iter().map(String::as_str).collect();
    let (year, month, lang) = match args.as_slice() {
        [y, m] | [y, m, "en"] => (y, m, Lang::En),
        [y, m, "ne"] => (y, m, Lang::Ne),
        _ => {
            eprintln!("{USAGE}");
            return ExitCode::from(2);
        }
    };
    let (Ok(year), Ok(month)) = (year.parse::<u16>(), month.parse::<u8>()) else {
        eprintln!("{USAGE}");
        return ExitCode::from(2);
    };
    match NepaliDateRange::month(year, month) {
        Ok(range) => {
            print!("{}", grid(range, lang));
            ExitCode::SUCCESS
        }
        Err(err) => {
            eprintln!("error: {err}");
            ExitCode::FAILURE
        }
    }
}

/// The month as text: a title line, a weekday header, then one line per week, Sunday first.
fn grid(range: NepaliDateRange, lang: Lang) -> String {
    let first = range.start();
    let mut out = format!(
        "{} {}\n",
        first.month().name(lang),
        lang.digits(u32::from(first.year()), 4)
    );
    let header: Vec<String> = Weekday::ALL
        .iter()
        // A leading space keeps Nepali names apart: their display width isn't their char count.
        .map(|w| format!(" {:>3}", w.short_name(lang)))
        .collect();
    out.push_str(&header.concat());
    out.push('\n');

    let blanks = usize::from(first.weekday().index_from_sunday());
    out.push_str(&"    ".repeat(blanks));
    let mut column = blanks;
    for day in range.days() {
        // Writing to a String can't fail.
        let _ = write!(
            out,
            "{:>4}",
            lang.digits(u32::from(day.day()), 1).to_string()
        );
        column += 1;
        if column == 7 {
            out.push('\n');
            column = 0;
        }
    }
    if column != 0 {
        out.push('\n');
    }
    out
}
