# W3: Converter, settings, persistence, autostart, single instance

Requirements: WR-06, WR-07, WR-09 AC1–AC2, WR-01 AC5 (copy), WR-11.
Read first: ADR-0015. Human reads: `docs/learn/11-io-and-platform.md`.

## Tasks

### W3-00a Interaction tests (before W3-01)
Slint's testing backend (`i-slint-backend-testing`, pinned to the `slint` version in
`Cargo.lock`) drives the popup headlessly. Elements are found by accessible label and their
accessible actions are invoked. The tests cover:

- previous, next and Today, including disabled previous at 1901-01 and disabled next at 2199-12;
- a month picker jump and a year picker jump;
- one wheel notch changing the month;
- a day click opening the day detail;
- Escape closing the detail first, then the popup;
- Copy BS and Copy AD writing the exact strings through an injected clipboard port (the tests
  never touch the system clipboard);
- the nine-event list scrolling (the content moves).

Every control has an `accessible-label` and a role. The interaction test module is
`src/interaction_tests.rs`; the test platform initializes once per thread so `cargo test` and
`cargo nextest` both work. A control that the testing backend cannot drive is named in the
report, with the reason, and its manual checklist line stays unticked.

### W3-00b English tithi width in cells
At 1x the English tithi in a cell elides to four letters ("Sapt…"). The fix is the 11 px Latin
`caption` step of widget-design §3, allowed for Latin by §4, while Devanagari stays at 12 px or
larger. The line keeps the W2 review's "full width" wording, so the Latin caption uses the
whole cell instead of the 4 px inset of line 1. The today cell is the exception: it keeps the
4 px inset so its 2 px ring stays 2 px clear of the glyphs. No abbreviation is invented: the
library has no short tithi form. `ui_bridge.rs` passes the size from the language, like
`ad-day-size`. The W2 plan's "tithi 12 px" becomes "full cell width (4 px inset on today), 11 px
Latin, 12 px Devanagari". Before and after PNGs go in `docs/plan/spike-results/w3/`. If a name
still elides at 11 px, the report says which; no smaller font is used.

### W3-00c Tithi "*" marker
Some Nepali tithi cells show a trailing `*`. Find whether it comes from the hub data, the
library or the widget, and report what it means. The display changes only if a specification or
library document defines the marker; otherwise it stays as the data has it.

### W3-01 Converter model (`src/app/converter.rs`)
```rust
pub enum Direction { BsToAd, AdToBs }
pub enum ConvertOutcome { Empty, Ok { short: String, long: String }, Error(String) }
pub fn convert(input: &str, dir: Direction, lang: Lang) -> ConvertOutcome;       // WR-07 AC1–AC3
pub fn days_between(a: &str, b: &str, lang: Lang) -> ConvertOutcome;            // WR-07 AC4 (uses NepaliDate::diff)
```
BS input uses `NepaliDate::parse_lenient`. AD input is strict `YYYY-MM-DD` / `YYYY/MM/DD`
(ASCII or Devanagari digits), parsed in this module with a small function (reuse the
separator rules from PARSING §1; the digit mapping is `nepdate::Lang` knowledge, so don't
duplicate it: add a `pub fn` to the library only if the spec is updated first. Otherwise map
digits locally with a 10-entry table).
Tests: `wr07_ac2_error_only_after_8_chars_or_2_separators`, `wr07_ac3_range_message`, etc.

### W3-02 Converter UI (`ui/converter.slint`)
Tab/segmented control inside the popup: "Convert" | "Days". Live update on `edited`.

### W3-03 Settings persistence (`src/platform/settings_store.rs`)
- `serde` + `toml`; `#[serde(default)]` on the struct so missing keys use defaults and
  unknown keys are ignored (WR-06 AC2).
- `validate(&mut Settings) -> Vec<String>`, in `app/settings.rs` (pure): replaces invalid values
  with defaults and returns warnings to log. An unknown IANA zone becomes `"Asia/Kathmandu"`.
- Parse failure → rename to `settings.toml.broken-<unix-ts>`, use defaults.
- Atomic save: write `settings.toml.tmp` in the same dir, `fs::rename` over the target
  (on Windows `rename` replaces the file since Rust 1.5 using `MoveFileExW` with REPLACE_EXISTING).
- Tests with a `tempfile` dir: round-trip, broken file, unknown key, invalid language.

### W3-04 Settings UI (`ui/settings.slint`)
Controls for every WR-06 key. Theme: set `Palette.color-scheme` (`light`/`dark`), or leave it
at the system default for "system". The time zone picker is a filterable list (WR-06 AC4). Changes
apply immediately (WR-06 AC1) and save debounced by 300 ms.
About section: version, data coverage (`nepdate_patro::coverage()`), NepDate MIT credit,
Slint attribution (`AboutSlint`, required by the Royalty-free license, ADR-0013), font licenses.

### W3-05 Autostart (`src/platform/autostart.rs`)
`auto-launch` 0.6 has no builder; it is `AutoLaunch::new` with a platform mode:
`WindowsEnableMode::CurrentUser`, `LinuxLaunchMode::XdgAutostart`, or `MacOSLaunchMode::SMAppService`
inside a `.app` bundle and `LaunchAgent` otherwise (e.g. `cargo run`). Then
`enable()`/`disable()`/`is_enabled()`. App name `mitibar`, path = `std::env::current_exe()`.
Enable/disable follows `launch_at_login`. Enabling writes the entry on every start, which also
refreshes the path when the app was moved; `auto-launch` 0.6 exposes no getter for the registered
path. Log the result; never fail startup because of autostart.

### W3-06 Single instance (`src/platform/single_instance.rs`)
A local socket (`interprocess` 2.x: `ListenerOptions::new().name(n).create_sync()`,
`Stream::connect(n)`). Name `mitibar-<username-hash>.sock` via `GenericNamespaced` when
`GenericNamespaced::is_supported()` (Windows, Linux). Otherwise (macOS) use `GenericFilePath` in the
data-local dir. The
first instance listens on a background thread and forwards "open" to the UI thread with
`slint::invoke_from_event_loop`. A second instance connects, sends `open\n`, and exits 0
(WR-09 AC1). If the connection fails but the socket exists (stale), take over.

## Definition of Done
Global DoD + milestone M4 (runs on all 3 OSes from `cargo run -p mitibar --release`).

## Human review checklist
- [ ] Corrupt `settings.toml` by hand (delete a quote). Start the app. What happens? Check the log.
- [ ] Start the app twice. Does the second launch open the first one's popup?
- [ ] Read `settings_store.rs`: why write to a temp file and rename? (learn/11 §2)

## Checkpoint

The implementation agent stops here for review. The reviewer checks:

- [ ] `cargo xtask ci` passes; `app/` coverage ≥ 80 %.
- [ ] The converter uses `NepaliDate::parse_lenient`, `from_gregorian` and `to_gregorian`, and
      shows `Error` kinds as translated messages.
- [ ] Settings are written to a temp file and renamed; a corrupt file falls back to defaults and
      logs a warning.
- [ ] Autostart and single instance work on all 3 OSes (manual checklist ticked); a stale socket
      is taken over.
- [ ] Milestone M4: `cargo run -p mitibar --release` works on Windows, macOS and Linux.
