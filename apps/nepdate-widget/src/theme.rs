//! Selects the palette side of the `Theme` global from `ui/theme.slint`, which holds the design
//! tokens (docs/plan/widget-design.md §2). The token values and the light/dark switch live in
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
        let design = include_str!("../../../docs/plan/widget-design.md");
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
}
