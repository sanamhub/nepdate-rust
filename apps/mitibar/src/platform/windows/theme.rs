//! The bar's colours (WR-08 AC7, widget-design §2 and §5). The bar follows the **system** theme,
//! which is what the taskbar follows; the scheme itself comes from `platform::taskbar_scheme`
//! (windows-registry, no process spawn). This module only holds the colours, kept equal to the
//! design table by the test below.

/// The tokens the bar uses, as straight `[r, g, b]`.
pub struct Palette {
    pub surface: [u8; 3],
    pub surface_raised: [u8; 3],
    pub ink: [u8; 3],
    pub ink_muted: [u8; 3],
    pub hairline: [u8; 3],
}

/// The bar's surface is `surface` at 90 % opacity (widget-design §5).
pub const SURFACE_ALPHA: f32 = 0.9;

pub const LIGHT: Palette = Palette {
    surface: [0xFD, 0xFB, 0xF8],
    surface_raised: [0xFF, 0xFF, 0xFF],
    ink: [0x21, 0x1C, 0x17],
    ink_muted: [0x6E, 0x65, 0x5C],
    hairline: [0xE7, 0xE0, 0xD6],
};

pub const DARK: Palette = Palette {
    surface: [0x16, 0x12, 0x0E],
    surface_raised: [0x20, 0x1B, 0x16],
    ink: [0xF1, 0xEC, 0xE4],
    ink_muted: [0x9B, 0x92, 0x88],
    hairline: [0x2C, 0x26, 0x20],
};

/// The palette for `dark` (true = dark taskbar).
pub fn palette(dark: bool) -> &'static Palette {
    if dark { &DARK } else { &LIGHT }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The constants equal the token rows of `docs/design/widget-design.md` §2, the same style as
    /// the theme drift test. Changing one constant makes it fail.
    #[test]
    fn wr08_ac7_palette_matches_design() {
        let design = include_str!("../../../../../docs/design/widget-design.md");
        let unquote = |s: &str| {
            s.strip_prefix('`')
                .and_then(|s| s.strip_suffix('`'))
                .map(str::to_owned)
        };
        let hex = |c: [u8; 3]| format!("#{:02X}{:02X}{:02X}", c[0], c[1], c[2]);
        let mut checked = Vec::new();
        for line in design.lines() {
            let cells: Vec<&str> = line.split('|').map(str::trim).collect();
            let [_, token, light, dark, ..] = cells.as_slice() else {
                continue;
            };
            let (Some(token), Some(light), Some(dark)) =
                (unquote(token), unquote(light), unquote(dark))
            else {
                continue;
            };
            let expected = match token.as_str() {
                "surface" => (LIGHT.surface, DARK.surface),
                "surface-raised" => (LIGHT.surface_raised, DARK.surface_raised),
                "ink" => (LIGHT.ink, DARK.ink),
                "ink-muted" => (LIGHT.ink_muted, DARK.ink_muted),
                "hairline" => (LIGHT.hairline, DARK.hairline),
                _ => continue,
            };
            assert_eq!(hex(expected.0), light, "{token} light");
            assert_eq!(hex(expected.1), dark, "{token} dark");
            checked.push(token);
        }
        assert_eq!(checked.len(), 5, "design table lacks a token: {checked:?}");
    }

    #[test]
    fn wr08_ac7_the_scheme_picks_the_palette() {
        assert_eq!(palette(false).ink, LIGHT.ink);
        assert_eq!(palette(true).ink, DARK.ink);
    }
}
