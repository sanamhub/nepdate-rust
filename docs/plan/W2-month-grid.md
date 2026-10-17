# W2: Month grid, navigation, day detail, next holiday

**Goal:** the core calendar UI.

Requirements: WR-03 (all AC), WR-04 (all AC), WR-10 keyboard access.
Preconditions (hard): L5 is merged (`nepdate-patro`, `jiff` feature).
Read first: `shared/spec/PATRO.md`, `docs/API.md` (crate `nepdate-patro`), ADR-0015.
Human reads: `docs/learn/10-models-and-callbacks.md`.

### W2-00 Dependencies
In `apps/mitibar/Cargo.toml`: `nepdate = { path = "../../crates/nepdate", features = ["jiff"] }`
and `nepdate-patro = { path = "../../crates/nepdate-patro" }` (ADR-0011, ADR-0015).
AC: `cargo build -p mitibar` succeeds, and `crates/nepdate/Cargo.toml` is unchanged.

## Tasks

### W2-01 Month view model (`src/app/month_view.rs`)
```rust
pub struct MonthView {
    pub year: u16, pub month: Month,
    pub title: String,                     // WR-03 AC5
    pub weekday_headers: [&'static str; 7],
    pub cells: Vec<Cell>,                  // len 35 or 42 (WR-03 AC1)
    pub can_prev: bool, pub can_next: bool,
}
pub struct Cell {
    pub date: Option<NepaliDate>,          // None = padding
    pub bs_day: String, pub ad_day: String, pub ad_month_marker: Option<&'static str>,
    pub is_today: bool, pub is_holiday_tint: bool,  // Saturday or public holiday
    pub tithi: Option<&'static str>, pub has_events: bool,
}
pub fn month_view(year: u16, month: Month, today: NepaliDate, lang: Lang) -> MonthView;
```
AC as tests: `wr03_ac1_rows` (every month 2080–2084 has 35 or 42 cells, and padding count =
first weekday index), `wr03_ac2_ad_month_marker`, `wr03_ac3_saturday_and_holiday_tint`,
`wr03_ac4_no_event_data_outside_coverage` (year 2090 → all tithi `None`, no panic),
`wr03_ac5_title_en_ne`, `wr03_ac6_edges` (1901-01: `can_prev == false`; 2199-12: `can_next == false`).

### W2-02 Grid UI (`ui/month-grid.slint`)
7-column `GridLayout`, with cells as a `for cell in root.cells` repeater over a `[CellData]` model
(generated struct). Weekday header row. The today outline, holiday colour and event dot come from
properties. Mouse wheel → `callback wheel(delta)`. Prev/next/today buttons. Month
`ComboBox` + year `SpinBox` (1901–2199).

### W2-03 Day detail (`ui/day-detail.slint`, `src/app/month_view.rs::day_detail`)
Inline overlay (WR-04 AC1), with Copy BS / Copy AD buttons via `platform::clipboard` (`arboard`).
Escape closes the detail first (WR-04 AC3).

### W2-04 Next holiday (`src/app/holidays.rs`)
`pub fn next_holiday(from: NepaliDate, lang: Lang) -> Option<NextHoliday>` where
`NextHoliday { date: NepaliDate, name: Option<&'static str>, days_away: u32 }`, using
`nepdate_patro::public_holidays()` from `from`, limited to 400 days. `name` is the first event of
that day (None if there are no events). The sentence ("today" / "tomorrow" / "in N days" / "Public
holiday") is built in `.slint` with `@tr`, not in Rust (WR-03 AC8). Tests with fixed dates in 2081.

### W2-05 Keyboard + accessibility (WR-10 AC2)
Day cells are focusable (`FocusScope`), Tab order is grid order, and Enter opens the detail.
PageUp/PageDown/Home/Escape per WR-10 AC2. Every control has an `accessible-label`.
No arrow-key "selected day" state: Tab is enough for v0.1.

### W2-06 Reopen behaviour (WR-02 AC2)
In `AppState`: `viewed_month: (u16, Month)` and `hidden_on: NepaliDate`. On show: if today ≠
`hidden_on`, reset `viewed_month` to today's month.

## Definition of Done
Global DoD. `app/` coverage ≥ 80 % (`cargo llvm-cov -p mitibar` filtered to `src/app`).
Performance: WR-03 AC9. Measure month switching with `log::debug!` timings in a release
build and note the number in the PR.

## Human review checklist
- [ ] Compare `MonthView` (Rust) with `CellData` (Slint). Where is the conversion, and why only there? (ADR-0015)
- [ ] Navigate to 2199 Chaitra and press "next". Nothing should happen, and nothing should crash.
- [ ] Switch language to ne. Are all digits in the grid Devanagari?

## Checkpoint

The implementation agent stops here for review. The reviewer checks:

- [ ] `cargo xtask ci` passes; `app/` coverage ≥ 80 %.
- [ ] The month view model gets patro data only through `nepdate_patro::info` and
      `public_holidays()`; no copy of patro tables in the widget.
- [ ] Month switching time in a release build is in the PR and meets WR-03 AC9.
- [ ] Navigation stops cleanly at the range ends (1901 Baishakh, 2199 Chaitra).
- [ ] Keyboard and accessibility items of WR-10 work; manual checklist lines added.
