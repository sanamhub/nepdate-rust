//! Compiles the `.slint` files into Rust before the crate compiles.
//! A typo in the markup is a compile error here, with the file and line.
//!
//! Bundles the Nepali translations from `i18n/ne/LC_MESSAGES/mitibar.po` (ADR-0008 A.3).
//! The default translation context is disabled so `@tr("...")` uses the plain `msgid`, matching
//! the `.po` file written by hand.
//!
//! On Windows, embeds the application manifest (ADR-0016): Windows 8 to 11 `supportedOS`,
//! per-monitor-v2 DPI awareness and the UTF-8 active code page. The layered child window needs
//! the `supportedOS` entry.

// Build scripts may panic; a bad `.slint` file should fail the build loudly.
#![allow(clippy::expect_used, clippy::unwrap_used)]

fn main() {
    // Slint element ids and accessible metadata are in debug builds only. The interaction tests
    // find elements by accessible label, which needs the metadata; release keeps its smaller
    // generated code.
    let debug_info = std::env::var("PROFILE").as_deref() == Ok("debug");
    let config = slint_build::CompilerConfiguration::new()
        .with_bundled_translations("i18n")
        .with_default_translation_context(slint_build::DefaultTranslationContext::None)
        .with_debug_info(debug_info);
    // app.slint is the single entry; it imports the other windows, so one compile call emits
    // every component (a second call would overwrite the first's generated file).
    slint_build::compile_with_config("ui/app.slint", config).expect("slint compile");

    if std::env::var_os("CARGO_CFG_WINDOWS").is_some() {
        use embed_manifest::manifest::SupportedOS::{Windows8, Windows10};
        use embed_manifest::{embed_manifest, new_manifest};

        // `Windows10` covers 10 and 11 (embed-manifest models them as one entry). The defaults
        // already select per-monitor-v2 DPI and the UTF-8 code page (ADR-0016).
        let manifest = new_manifest("mitibar").supported_os(Windows8..=Windows10);
        embed_manifest(manifest).expect("embed manifest");
    }
    println!("cargo:rerun-if-changed=build.rs");
    println!("cargo:rerun-if-env-changed=PROFILE");
}
