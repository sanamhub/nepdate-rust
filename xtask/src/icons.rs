//! `cargo xtask icons` (W1-03): pre-renders the tray day-number icons, 1 to 32 in the en and ne
//! digit sets, as 32x32 PNGs under `apps/nepdate-widget/ui/tray/`. One SVG template is rasterised
//! per day with the bundled Noto fonts through `resvg`; the shipped binary does no rasterising.

use anyhow::{Context, Result};

const SIZE: u32 = 32;
const DAYS: u32 = 32;
const FONT_EN: &str = "apps/nepdate-widget/ui/fonts/NotoSans-Variable.ttf";
const FONT_NE: &str = "apps/nepdate-widget/ui/fonts/NotoSansDevanagari-Variable.ttf";
const OUT_DIR: &str = "apps/nepdate-widget/ui/tray";

pub(crate) fn icons() -> Result<()> {
    let mut fontdb = fontdb::Database::new();
    fontdb.load_font_file(FONT_EN).context("load Noto Sans")?;
    fontdb
        .load_font_file(FONT_NE)
        .context("load Noto Sans Devanagari")?;

    for lang in ["en", "ne"] {
        let family = if lang == "en" {
            "Noto Sans"
        } else {
            "Noto Sans Devanagari"
        };
        let dir = format!("{OUT_DIR}/{lang}");
        std::fs::create_dir_all(&dir).with_context(|| format!("create {dir}"))?;
        for day in 1..=DAYS {
            let svg = template(family, &day_string(day, lang));
            let png = render(&svg, &fontdb).with_context(|| format!("render {lang} day {day}"))?;
            let path = format!("{dir}/{day}.png");
            std::fs::write(&path, png).with_context(|| format!("write {path}"))?;
        }
    }
    println!("wrote {DAYS} icons per language under {OUT_DIR}");
    Ok(())
}

/// The day number as ASCII digits (en) or Devanagari digits (ne).
fn day_string(day: u32, lang: &str) -> String {
    if lang == "en" {
        return day.to_string();
    }
    day.to_string()
        .chars()
        .map(|c| {
            let digit = c.to_digit(10).unwrap_or(0);
            char::from_u32(0x0966 + digit).unwrap_or('0')
        })
        .collect()
}

/// The one SVG template, with the font family and the day number substituted (W1-03).
/// White ink on a transparent background, the convention for notification-area icons.
fn template(font_family: &str, day: &str) -> String {
    format!(
        "<svg xmlns=\"http://www.w3.org/2000/svg\" width=\"{SIZE}\" height=\"{SIZE}\" viewBox=\"0 0 {SIZE} {SIZE}\">\
         <text x=\"16\" y=\"23\" font-family=\"{font_family}\" font-size=\"20\" text-anchor=\"middle\" fill=\"#FFFFFF\">{day}</text>\
         </svg>"
    )
}

/// Rasterises `svg` at `SIZE` x `SIZE` and returns the PNG bytes.
fn render(svg: &str, fontdb: &fontdb::Database) -> Result<Vec<u8>> {
    let options = resvg::usvg::Options {
        fontdb: std::sync::Arc::new(fontdb.clone()),
        ..resvg::usvg::Options::default()
    };
    let tree = resvg::usvg::Tree::from_str(svg, &options).context("parse SVG")?;
    let mut pixmap = resvg::tiny_skia::Pixmap::new(SIZE, SIZE).context("allocate pixmap")?;
    resvg::render(
        &tree,
        resvg::tiny_skia::Transform::identity(),
        &mut pixmap.as_mut(),
    );
    pixmap.encode_png().context("encode PNG")
}
