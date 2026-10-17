# W3: Converter, settings, persistence, autostart, single instance

Requirements: WR-06, WR-07, WR-09 AC1–AC2, WR-01 AC5 (copy), WR-11.
Read first: ADR-0015. Human reads: `docs/learn/11-io-and-platform.md`.

## Tasks

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
`auto-launch` 0.6 (`AutoLaunchBuilder::new().set_app_name(..).set_app_path(..).build()?`, then
`enable()`/`disable()`/`is_enabled()`). App name `mitibar`, path = `std::env::current_exe()`.
macOS: `MacOSLaunchMode::SMAppService` when running from a `.app` bundle (macOS 13+), and the
LaunchAgent mode otherwise (e.g. `cargo run`).
Enable/disable follows `launch_at_login`. At startup, if enabled but the registered path differs
from `current_exe()` (the app was moved), re-register. Log the result; never fail startup
because of autostart.

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
