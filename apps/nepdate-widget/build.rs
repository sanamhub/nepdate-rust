//! Compiles the `.slint` files into Rust before the crate compiles (learn/08 §2).
//! A typo in the markup is a compile error here, with the file and line.
//!
//! Bundles the Nepali translations from `i18n/ne/LC_MESSAGES/nepdate-widget.po` (ADR-0008 A.3).
//! The default translation context is disabled so `@tr("...")` uses the plain `msgid`, matching
//! the `.po` file written by hand.

// Build scripts may panic (W0-01); a bad `.slint` file should fail the build loudly.
#![allow(clippy::expect_used, clippy::unwrap_used)]

fn main() {
    let config = slint_build::CompilerConfiguration::new()
        .with_bundled_translations("i18n")
        .with_default_translation_context(slint_build::DefaultTranslationContext::None);
    // app.slint is the single entry; it imports the other windows, so one compile call emits
    // every component (a second call would overwrite the first's generated file).
    slint_build::compile_with_config("ui/app.slint", config).expect("slint compile");
}
