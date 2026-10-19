//! The two `ink` colours for the taskbar text (ADR-0016 point 5, WR-12 AC2). The taskbar scheme
//! itself comes from `platform::taskbar_scheme` (windows-registry, no process spawn); this module
//! only holds the colours, kept equal to the design table by the test below.

/// The `ink` token for a light taskbar (`#211C17`), premultiplied alpha 1.
pub const INK_LIGHT: [u8; 4] = [0x21, 0x1C, 0x17, 0xFF];
/// The `ink` token for a dark taskbar (`#F1ECE4`), premultiplied alpha 1.
pub const INK_DARK: [u8; 4] = [0xF1, 0xEC, 0xE4, 0xFF];

/// The ink for `dark` (true = dark taskbar).
pub fn ink(dark: bool) -> [u8; 4] {
    if dark { INK_DARK } else { INK_LIGHT }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The two constants equal the `ink` row of `docs/design/widget-design.md` §2, the same style as
    /// the theme drift test. Changing one constant makes it fail.
    #[test]
    fn wr12_ac2_ink_matches_design() {
        let design = include_str!("../../../../../docs/design/widget-design.md");
        let unquote = |s: &str| {
            s.strip_prefix('`')
                .and_then(|s| s.strip_suffix('`'))
                .map(str::to_owned)
        };
        let mut found = None;
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
            if token == "ink" {
                found = Some((light, dark));
            }
        }
        let (light, dark) = found.expect("design table lacks `ink`");
        let hex = |c: [u8; 4]| format!("#{:02X}{:02X}{:02X}", c[0], c[1], c[2]);
        assert_eq!(hex(INK_LIGHT), light);
        assert_eq!(hex(INK_DARK), dark);
    }
}
