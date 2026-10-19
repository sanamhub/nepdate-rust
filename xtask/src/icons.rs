//! `cargo xtask icons [--check]` (W1-03, W4-02): renders the tray day-number icons and the app
//! icon. The tray icons are 32x32 PNGs under `apps/mitibar/ui/tray/<lang>/<scheme>/` for each
//! digit set (en, ne) and taskbar scheme (light, dark), plus `ui/tray-icons.slint`, the image
//! arrays the tray indexes. The app icon comes from `assets/icon.svg` as PNG sizes 16 to 1024,
//! an `.ico` and an `.icns` under `apps/mitibar/icons/` for `cargo-packager`.
//!
//! One SVG template per tray icon is rasterised with the bundled Noto fonts through `resvg`; the
//! shipped binary does no rasterising. `--check` renders everything in memory and fails if any
//! committed file differs, so the icons and the arrays can't drift from this code.

use std::path::Path;

use anyhow::{Context, Result, bail};

use crate::codegen::workspace_root;

const SIZE: u32 = 32;
const DAYS: u32 = 32;
const FONT_EN: &str = "apps/mitibar/ui/fonts/NotoSans-Variable.ttf";
const FONT_NE: &str = "apps/mitibar/ui/fonts/NotoSansDevanagari-Variable.ttf";
const OUT_DIR: &str = "apps/mitibar/ui/tray";
const SLINT_OUT: &str = "apps/mitibar/ui/tray-icons.slint";

/// The app icon source and the generated files (W4-02).
const APP_SVG: &str = "assets/icon.svg";
const APP_OUT: &str = "apps/mitibar/icons";
const APP_PNG_SIZES: [u32; 9] = [16, 24, 32, 48, 64, 128, 256, 512, 1024];
/// The sizes inside the Windows `.ico` (multi-image, PNG compressed).
const ICO_SIZES: [u32; 7] = [16, 24, 32, 48, 64, 128, 256];
/// The sizes inside the macOS `.icns`, with their `OSType` tags.
const ICNS_SIZES: [(u32, &[u8; 4]); 7] = [
    (16, b"icp4"),
    (32, b"icp5"),
    (64, b"icp6"),
    (128, b"ic07"),
    (256, b"ic08"),
    (512, b"ic09"),
    (1024, b"ic10"),
];

/// Ink per taskbar scheme: the `ink` token of docs/design/widget-design.md for that scheme, so the
/// digits contrast with a light or a dark taskbar the way the popup text contrasts with its
/// surface.
const SCHEMES: [(&str, &str); 2] = [("light", "#211C17"), ("dark", "#F1ECE4")];
const LANGS: [(&str, &str); 2] = [("en", "Noto Sans"), ("ne", "Noto Sans Devanagari")];

pub(crate) fn icons(check: bool) -> Result<()> {
    let root = workspace_root();
    let files = generated(&root)?;

    if check {
        let stale: Vec<&str> = files
            .iter()
            .filter(|(path, bytes)| {
                std::fs::read(root.join(path)).ok().as_deref() != Some(bytes.as_slice())
            })
            .map(|(path, _)| path.as_str())
            .collect();
        if let Some(first) = stale.first() {
            bail!(
                "{} icon file(s) differ from `cargo xtask icons`, first: {first}",
                stale.len()
            );
        }
        println!("icons --check: {} files match", files.len());
        return Ok(());
    }
    for (path, bytes) in &files {
        let path = root.join(path);
        if let Some(dir) = path.parent() {
            std::fs::create_dir_all(dir).with_context(|| format!("create {}", dir.display()))?;
        }
        std::fs::write(&path, bytes).with_context(|| format!("write {}", path.display()))?;
    }
    println!(
        "wrote {} files under {OUT_DIR}, {APP_OUT} and {SLINT_OUT}",
        files.len()
    );
    Ok(())
}

/// Every generated icon file, relative to `root`. Pure apart from reading `APP_SVG` and the
/// bundled fonts, so `--check` and the freshness test both compare the same bytes.
fn generated(root: &Path) -> Result<Vec<(String, Vec<u8>)>> {
    let mut fontdb = fontdb::Database::new();
    fontdb
        .load_font_file(root.join(FONT_EN))
        .context("load Noto Sans")?;
    fontdb
        .load_font_file(root.join(FONT_NE))
        .context("load Noto Sans Devanagari")?;

    let mut files: Vec<(String, Vec<u8>)> = Vec::new();
    for (lang, family) in LANGS {
        for (scheme, ink) in SCHEMES {
            for day in 1..=DAYS {
                let svg = template(family, ink, &day_string(day, lang));
                let png = render(&svg, SIZE, &fontdb)
                    .with_context(|| format!("render {lang} {scheme} day {day}"))?;
                files.push((format!("{OUT_DIR}/{lang}/{scheme}/{day}.png"), png));
            }
        }
    }
    files.push((SLINT_OUT.to_owned(), slint_arrays().into_bytes()));

    let app_svg =
        std::fs::read_to_string(root.join(APP_SVG)).with_context(|| format!("read {APP_SVG}"))?;
    let mut app_pngs: Vec<(u32, Vec<u8>)> = Vec::new();
    for size in APP_PNG_SIZES {
        let png =
            render(&app_svg, size, &fontdb).with_context(|| format!("render app icon {size}"))?;
        files.push((format!("{APP_OUT}/{size}x{size}.png"), png.clone()));
        app_pngs.push((size, png));
    }
    let ico = ico(&app_pngs)?;
    files.push((format!("{APP_OUT}/icon.ico"), ico));
    let icns = icns(&app_pngs)?;
    files.push((format!("{APP_OUT}/icon.icns"), icns));

    Ok(files)
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

/// The one tray SVG template. The digits fill most of the 32 px square, because Windows shows
/// the icon at 16 px at 100 % scale; a stroke in the fill colour thickens the regular weight
/// (resvg uses the variable fonts' default instance) so the strokes survive that downscale.
fn template(font_family: &str, ink: &str, day: &str) -> String {
    format!(
        "<svg xmlns=\"http://www.w3.org/2000/svg\" width=\"{SIZE}\" height=\"{SIZE}\" viewBox=\"0 0 {SIZE} {SIZE}\">\
         <text x=\"16\" y=\"25\" font-family=\"{font_family}\" font-size=\"24\" text-anchor=\"middle\" \
         fill=\"{ink}\" stroke=\"{ink}\" stroke-width=\"1.4\" stroke-linejoin=\"round\">{day}</text>\
         </svg>"
    )
}

/// `ui/tray-icons.slint`: one global with an image array per language and taskbar scheme.
fn slint_arrays() -> String {
    use std::fmt::Write as _;

    let mut out = String::from(
        "// Generated by `cargo xtask icons`; do not edit. `cargo xtask ci` checks it.\n\
         export global TrayIcons {\n",
    );
    for (lang, _) in LANGS {
        for (scheme, _) in SCHEMES {
            let _ = writeln!(out, "    out property <[image]> {lang}-{scheme}: [");
            for day in 1..=DAYS {
                let comma = if day == DAYS { "" } else { "," };
                let _ = writeln!(
                    out,
                    "        @image-url(\"tray/{lang}/{scheme}/{day}.png\"){comma}"
                );
            }
            out.push_str("    ];\n");
        }
    }
    out.push_str("}\n");
    out
}

/// Rasterises `svg` at `size` x `size` and returns the PNG bytes.
fn render(svg: &str, size: u32, fontdb: &fontdb::Database) -> Result<Vec<u8>> {
    let options = resvg::usvg::Options {
        fontdb: std::sync::Arc::new(fontdb.clone()),
        ..resvg::usvg::Options::default()
    };
    let tree = resvg::usvg::Tree::from_str(svg, &options).context("parse SVG")?;
    let mut pixmap = resvg::tiny_skia::Pixmap::new(size, size).context("allocate pixmap")?;
    // The tray template is 32x32, but the app icon is 1024x1024: scale the drawing to fit the
    // requested pixmap instead of cropping it at the top-left.
    let source = tree.size().width();
    let scale = if source > 0.0 {
        f32::from(u16::try_from(size).unwrap_or(1)) / source
    } else {
        1.0
    };
    resvg::render(
        &tree,
        resvg::tiny_skia::Transform::from_scale(scale, scale),
        &mut pixmap.as_mut(),
    );
    pixmap.encode_png().context("encode PNG")
}

/// A Windows `.ico` with PNG-compressed images. Vista and later read PNG entries; a 256 px
/// entry stores width and height as 0.
fn ico(pngs: &[(u32, Vec<u8>)]) -> Result<Vec<u8>> {
    let images: Vec<(u32, &[u8])> = ICO_SIZES
        .iter()
        .map(|size| {
            pngs.iter()
                .find(|(candidate, _)| candidate == size)
                .map(|(_, png)| (*size, png.as_slice()))
                .with_context(|| format!("missing {size} px icon"))
        })
        .collect::<Result<_>>()?;
    let mut out = Vec::new();
    out.extend_from_slice(&0u16.to_le_bytes()); // reserved
    out.extend_from_slice(&1u16.to_le_bytes()); // type: icon
    let count = u16::try_from(images.len()).context("icon count")?;
    out.extend_from_slice(&count.to_le_bytes());
    let header = 6 + 16 * images.len();
    let mut offset = u32::try_from(header).context("icon header")?;
    for (size, png) in &images {
        let byte = u8::try_from(size % 256).context("icon size")?;
        out.push(byte); // width, 0 means 256
        out.push(byte); // height
        out.push(0); // palette size
        out.push(0); // reserved
        out.extend_from_slice(&1u16.to_le_bytes()); // colour planes
        out.extend_from_slice(&32u16.to_le_bytes()); // bits per pixel
        out.extend_from_slice(
            &u32::try_from(png.len())
                .context("icon length")?
                .to_le_bytes(),
        );
        out.extend_from_slice(&offset.to_le_bytes());
        offset += u32::try_from(png.len()).context("icon offset")?;
    }
    for (_, png) in &images {
        out.extend_from_slice(png);
    }
    Ok(out)
}

/// A macOS `.icns` with PNG entries. The container is the 8-byte header followed by one
/// big-endian tag, length and PNG payload per image.
fn icns(pngs: &[(u32, Vec<u8>)]) -> Result<Vec<u8>> {
    let mut body = Vec::new();
    for (size, tag) in ICNS_SIZES {
        let png = pngs
            .iter()
            .find(|(candidate, _)| *candidate == size)
            .map(|(_, png)| png)
            .with_context(|| format!("missing {size} px icon"))?;
        let length = 8 + u32::try_from(png.len()).context("icns entry length")?;
        body.extend_from_slice(tag);
        body.extend_from_slice(&length.to_be_bytes());
        body.extend_from_slice(png);
    }
    let total = 8 + u32::try_from(body.len()).context("icns length")?;
    let mut out = Vec::new();
    out.extend_from_slice(b"icns");
    out.extend_from_slice(&total.to_be_bytes());
    out.extend_from_slice(&body);
    Ok(out)
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used, clippy::expect_used, clippy::indexing_slicing)]

    use super::*;

    #[test]
    fn committed_icons_match_a_fresh_run() {
        let root = workspace_root();
        for (path, bytes) in generated(&root).expect("generate icons") {
            let committed = std::fs::read(root.join(&path))
                .with_context(|| format!("read committed {path}"))
                .expect("every generated file is committed");
            assert_eq!(committed, bytes, "{path} is stale; run `cargo xtask icons`");
        }
    }

    #[test]
    fn ico_and_icns_have_the_expected_containers() {
        let pngs: Vec<(u32, Vec<u8>)> = APP_PNG_SIZES
            .iter()
            .map(|size| (*size, vec![0xAB; usize::try_from(*size).unwrap_or(0)]))
            .collect();
        let ico = ico(&pngs).expect("ico");
        assert_eq!(&ico[..4], &[0, 0, 1, 0]);
        assert_eq!(u16::from_le_bytes([ico[4], ico[5]]), 7);
        let icns = icns(&pngs).expect("icns");
        assert_eq!(&icns[..4], b"icns");
        assert_eq!(
            u32::from_be_bytes([icns[4], icns[5], icns[6], icns[7]]) as usize,
            icns.len()
        );
    }
}
