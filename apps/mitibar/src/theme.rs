//! Selects the palette side of the `Theme` global from `ui/theme.slint`, which holds the design
//! tokens (docs/design/widget-design.md §2). The token values and the light/dark switch live in
//! that `.slint` file; this module only passes the user's choice in.
//!
//! The system scheme is not detected here. With `ThemeChoice::System` the markup reads Slint's
//! `Palette.color-scheme`, which the winit backend fills from the OS and updates live. Shelling
//! out (`reg`, `defaults`, `gsettings`) would cost a process spawn at startup and, from a
//! `windows_subsystem = "windows"` binary, flash a console window.

use slint::ComponentHandle;

use crate::{Theme, ThemeChoice};

/// Sets `Theme.choice` on `ui`. Called once per window before it is shown.
pub fn apply<'a, T>(ui: &'a T, choice: ThemeChoice)
where
    T: ComponentHandle,
    Theme<'a>: slint::Global<'a, T>,
{
    ui.global::<Theme<'a>>().set_choice(choice);
}

#[cfg(test)]
mod tests {
    /// Every `-light` / `-dark` token in `ui/theme.slint` equals its cell in the design table, and
    /// every table row is in the markup, so the two can't drift apart.
    #[test]
    fn tokens_match_design_table() {
        let design = include_str!("../../../docs/design/widget-design.md");
        let markup = include_str!("../ui/theme.slint");

        let unquote = |s: &str| {
            s.strip_prefix('`')
                .and_then(|s| s.strip_suffix('`'))
                .map(str::to_owned)
        };
        let mut rows = 0;
        for line in design.lines() {
            let cells: Vec<&str> = line.split('|').map(str::trim).collect();
            // | `token` | `#light` | `#dark` | use |
            let [_, token, light, dark, ..] = cells.as_slice() else {
                continue;
            };
            let (Some(token), Some(light), Some(dark)) =
                (unquote(token), unquote(light), unquote(dark))
            else {
                continue;
            };
            if !light.starts_with('#') {
                continue;
            }
            rows += 1;
            for (side, value) in [("light", &light), ("dark", &dark)] {
                let decl = format!("property <color> {token}-{side}: {value};");
                assert!(markup.contains(&decl), "theme.slint lacks `{decl}`");
            }
        }
        assert_eq!(rows, 8, "design table rows");
        let declared = markup.matches("property <color> ").count();
        assert_eq!(
            declared,
            8 * 3,
            "8 light + 8 dark + 8 resolved colour properties"
        );
    }

    /// The focus ring uses `ink`. WCAG 2.1 wants at least 3:1 for a non-text indicator;
    /// measure it against the two surfaces and the Fluent control background (white at 70 % in
    /// light, white at 6 % in dark, both over the surface).
    #[test]
    fn focus_ring_ink_has_at_least_3_to_1_contrast() {
        let design = include_str!("../../../docs/design/widget-design.md");
        let mut values = std::collections::HashMap::new();
        for line in design.lines() {
            let cells: Vec<&str> = line.split('|').map(str::trim).collect();
            let [_, token, light, dark, ..] = cells.as_slice() else {
                continue;
            };
            let unquote = |s: &str| {
                s.strip_prefix('`')
                    .and_then(|s| s.strip_suffix('`'))
                    .map(str::to_owned)
            };
            let (Some(token), Some(light), Some(dark)) =
                (unquote(token), unquote(light), unquote(dark))
            else {
                continue;
            };
            if !light.starts_with('#') {
                continue;
            }
            values.insert(format!("{token}-light"), (light, dark.clone()));
        }

        let color = |token: &str, side: &str| {
            let (light, dark) = values
                .get(&format!("{token}-light"))
                .unwrap_or_else(|| panic!("design table lacks {token}"));
            if side == "light" { light } else { dark }
        };
        let ratio = |foreground: &str, background: &str| {
            let foreground = luminance(foreground);
            let background = luminance(background);
            let (lighter, darker) = if foreground > background {
                (foreground, background)
            } else {
                (background, foreground)
            };
            (lighter + 0.05) / (darker + 0.05)
        };

        for side in ["light", "dark"] {
            let ink = color("ink", side);
            let surface = color("surface", side);
            for (name, background) in [
                ("surface", surface.clone()),
                ("surface-raised", color("surface-raised", side).clone()),
                (
                    "control-background",
                    // Fluent control-background is white at alpha 0xB3 in light and 0x0F in
                    // dark, composited over the surface.
                    composite_over(
                        "#FFFFFF",
                        if side == "light" { 0xB3 } else { 0x0F },
                        surface,
                    ),
                ),
            ] {
                let measured = ratio(ink, &background);
                assert!(
                    measured >= 3.0,
                    "{side} ink on {name} is {measured:.2}:1, below 3:1"
                );
                println!("{side} ink on {name}: {measured:.2}:1");
            }
        }
    }

    fn luminance(hex: &str) -> f64 {
        let value = u32::from_str_radix(hex.trim_start_matches('#'), 16).expect("hex colour");
        let channel = |shift: u32| {
            let value = f64::from((value >> shift) & 0xFF) / 255.0;
            if value <= 0.040_45 {
                value / 12.92
            } else {
                ((value + 0.055) / 1.055).powf(2.4)
            }
        };
        0.2126 * channel(16) + 0.7152 * channel(8) + 0.0722 * channel(0)
    }

    fn composite_over(foreground: &str, alpha: u32, background: &str) -> String {
        use std::fmt::Write as _;

        let channel = |hex: &str, shift: u32| {
            (u32::from_str_radix(hex.trim_start_matches('#'), 16).expect("hex colour") >> shift)
                & 0xFF
        };
        let mut mixed = String::from("#");
        for shift in [16, 8, 0] {
            let value = (alpha * channel(foreground, shift)
                + (255 - alpha) * channel(background, shift)
                + 127)
                / 255;
            write!(mixed, "{value:02X}").expect("writing to a String");
        }
        mixed
    }
}
