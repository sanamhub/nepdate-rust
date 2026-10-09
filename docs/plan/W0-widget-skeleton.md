# W0: Widget skeleton and Devanagari go/no-go spike

**Goal:** a Slint app in the workspace that opens a window with bundled fonts, and proves
Devanagari renders correctly on Windows, macOS and Linux **before** any feature work.

Preconditions (hard): L2 is merged.
Read first: ADR-0008, ADR-0010, ADR-0015, `docs/plan/widget-requirements.md`.
Human reads: `docs/learn/08-slint-basics.md`.

## Tasks

### W0-01 Crate skeleton
- Add `apps/nepdate-widget` to workspace `members` (not `default-members`).
- `apps/nepdate-widget/Cargo.toml`: `publish = false`, `rust-version = "1.92"` (Slint MSRV),
  deps: `slint = "1.18"`, `nepdate = { path = "../../crates/nepdate" }`,
  `jiff = "0.2"`, `anyhow = "1"`; build-dep `slint-build = "1.18"`. W0 needs only L2, so no
  `nepdate` features yet: today's BS date is `NepaliDate::from_gregorian(..)` of
  `jiff::Zoned::now().date()`, converting jiff's `i16`/`i8` fields with `i32::from` and
  `u8::try_from` (no `as` casts). W2-00 adds the `jiff` feature and `nepdate-patro`.
- `build.rs`: `slint_build::compile("ui/app.slint").expect("slint compile");` (allowed: build scripts may panic).
- Module layout exactly per ADR-0015, with empty modules containing only a `//!` doc line.
- `main.rs`: `slint::include_modules!();` then show `AppWindow` with the text of today's BS
  date in both languages.
- Windows: `#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]` so there's no
  console window in release builds.

### W0-02 Fonts
- Download (human approves the download) **Noto Sans** and **Noto Sans Devanagari** (variable TTF, OFL-1.1)
  from the official `notofonts` GitHub releases into `apps/nepdate-widget/ui/fonts/`, with
  their `OFL.txt`. Record the source URL and version in `ui/fonts/README.md`.
- In `app.slint`: `import "fonts/NotoSans-Variable.ttf"; import "fonts/NotoSansDevanagari-Variable.ttf";`
  and set `default-font-family: "Noto Sans";`. Rename the downloaded files to
  `NotoSans-Variable.ttf` / `NotoSansDevanagari-Variable.ttf` (brackets in filenames are
  unverified with `import`). Devanagari text should fall back to Noto Sans
  Devanagari. If fallback doesn't happen, set `font-family` explicitly on Nepali text.

### W0-03 Devanagari spike (go/no-go)
- A debug-only window `ui/font-test.slint` showing every line of
  `docs/plan/devanagari-test-strings.md` at 14 px and 24 px, run with `cargo run -p nepdate-widget -- --font-test`.
- The human runs it on Windows, macOS and Linux (X11 or Wayland) and saves screenshots to
  `docs/plan/spike-results/<os>.png`.
- Pass criteria: conjuncts are joined (क्ष, ज्ञ, श्र, द्ध), reph sits above (र्क, धर्म), the
  pre-base matra ि is placed **before** its consonant (कि), no tofu boxes, digits ०–९ correct.
- Fail on any OS → switch to the Skia renderer feature (`slint = { features = ["renderer-skia"] }`)
  and retest. Still failing → **stop**, and the human writes a new ADR (Tauri fallback, ADR-0010).

### W0-04 Widget CI
Extend `.github/workflows/ci.yml`: a `widget` job on the 3 OSes running
`cargo clippy -p nepdate-widget -- -D warnings` and `cargo test -p nepdate-widget`.
Linux apt deps: `libxkbcommon-dev libwayland-dev libfontconfig1-dev libdbus-1-dev`.
Update `deny.toml`: allow `LicenseRef-Slint-Royalty-free-2.0` (ADR-0013) **only** for Slint
crates (`[[licenses.exceptions]]`). Create `apps/nepdate-widget/CHANGELOG.md` with an empty
`## [Unreleased]`.

## Definition of Done
Global DoD + spike results committed and marked PASS/FAIL per OS in
`docs/plan/spike-results/README.md`.

## Human review checklist
- [ ] Look at the 3 screenshots side by side. Does कि look right (the vowel sign drawn first)?
- [ ] Open `build.rs`. When does it run? (learn/08 §2)
- [ ] Check the release binary size: `cargo build -p nepdate-widget --release` → note it down.

## Checkpoint

The implementation agent stops here for review. The reviewer checks:

- [ ] `cargo xtask ci` passes and the widget CI job is green on all 3 OSes.
- [ ] The Devanagari spike result is recorded PASS or FAIL per OS with screenshots; on FAIL the
      agent stopped and reported (go/no-go).
- [ ] Fonts are bundled with their licence files; `deny.toml` allows the Slint licence only for
      Slint crates.
- [ ] The widget depends on `nepdate` by path and is `publish = false`.
