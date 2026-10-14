//! Converts one date between BS and AD and prints both, with the weekday.
//!
//! ```text
//! cargo run -p nepdate --example convert --features jiff -- 2081/04/15
//! cargo run -p nepdate --example convert --features jiff -- --ad 2024-07-30
//! cargo run -p nepdate --example convert --features jiff -- --today
//! ```

use std::env;
use std::process::ExitCode;

use nepdate::{Error, Lang, LongDate, NepaliDate};

const USAGE: &str = "usage: convert <YYYY/MM/DD> | --ad <YYYY-MM-DD> | --today";

fn main() -> ExitCode {
    let args: Vec<String> = env::args().skip(1).collect();
    let args: Vec<&str> = args.iter().map(String::as_str).collect();
    let date = match args.as_slice() {
        ["--today"] => NepaliDate::today(),
        ["--ad", ad] => parse_ad(ad),
        [bs] => NepaliDate::parse_lenient(bs),
        _ => {
            eprintln!("{USAGE}");
            return ExitCode::from(2);
        }
    };
    match date {
        Ok(date) => {
            print(date);
            ExitCode::SUCCESS
        }
        Err(err) => {
            eprintln!("error: {err}");
            ExitCode::FAILURE
        }
    }
}

/// `YYYY-MM-DD` in AD. The library has no AD parser, so the example splits the text itself.
fn parse_ad(text: &str) -> Result<NepaliDate, Error> {
    let mut parts = text.split('-').map(str::parse::<i32>);
    match (parts.next(), parts.next(), parts.next(), parts.next()) {
        (Some(Ok(y)), Some(Ok(m)), Some(Ok(d)), None) => {
            let month = u8::try_from(m).map_err(|_| Error::InvalidGregorian)?;
            let day = u8::try_from(d).map_err(|_| Error::InvalidGregorian)?;
            NepaliDate::from_gregorian(y, month, day)
        }
        _ => Err(Error::InvalidGregorian),
    }
}

fn print(date: NepaliDate) {
    let ad = date.to_gregorian();
    let long = LongDate {
        weekday: true,
        ..LongDate::default()
    };
    println!("BS {date}  {}", date.long(long, Lang::En));
    println!("BS {}", date.long(long, Lang::Ne));
    println!(
        "AD {:04}-{:02}-{:02}  {}",
        ad.year(),
        ad.month(),
        ad.day(),
        ad.weekday().name(Lang::En)
    );
}
