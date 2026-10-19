# ADR-0015: Widget internal architecture

- **Status:** accepted
- **Date:** 2026-09-24
- **Approved by:** Sanam

## Context

The project follows Clean Architecture (Domain → Application → Infrastructure →
Presentation). The widget is small (target < 3k lines of Rust + `.slint`). Separate crates
per layer would be ceremony. Layer boundaries still matter for testability, because the UI
and the OS integration can't be unit-tested easily.

## Decision

One binary crate, `apps/mitibar`, with **modules as layers** and a strict
dependency direction:

```
apps/mitibar/
├── build.rs              # slint_build::compile_with_config(.., with_bundled_translations("i18n"))
├── i18n/ne/LC_MESSAGES/mitibar.po   # Nepali translations of @tr() strings
├── ui/                   # PRESENTATION (Slint markup): app.slint, popup.slint, month-grid.slint,
│   └── fonts/            #   day-detail.slint, converter.slint, settings.slint
└── src/
    ├── main.rs           # composition root: wires platform + app + ui, starts event loop
    ├── app/              # APPLICATION (pure Rust, no Slint, no OS calls), unit-tested
    │   ├── month_view.rs #   builds MonthView { title, cells[..42], ... } from nepdate + nepdate-patro
    │   ├── clock.rs      #   clock text for local + extra zone (takes a jiff::Zoned)
    │   ├── converter.rs  #   converter input -> result text / error
    │   ├── holidays.rs   #   next-holiday search
    │   └── settings.rs   #   Settings struct, defaults, validation (no file IO)
    ├── platform/         # INFRASTRUCTURE (OS integration), thin, behind small traits
    │   ├── tray.rs       #   tray icon + menu (Slint SystemTrayIcon or tray-icon fallback)
    │   ├── autostart.rs  #   auto-launch
    │   ├── single_instance.rs
    │   ├── settings_store.rs # TOML load/save, atomic write, `dirs` paths
    │   ├── clipboard.rs
    │   └── logging.rs
    └── ui_bridge.rs      # maps app/* view models <-> Slint generated types & callbacks
```

Rules:

- The **domain** is the `nepdate` and `nepdate-patro` library crates. `app/` depends on
  those two and `jiff` only. It never imports `slint` or `platform`.
- `platform/` never imports `slint` types except `tray.rs` (it may need the event loop).
- `ui_bridge.rs` is the only place that converts between `app` types and Slint types.
- Time is injected: `app` functions take `now: &jiff::Zoned` instead of calling
  `Zoned::now()`, so they are testable.
- State: a single `AppState` (settings + current view month + selection) owned by the
  main thread inside `Rc<RefCell<AppState>>`; Slint callbacks borrow it briefly. No
  global mutable statics.
- Timers: one `slint::Timer`, re-armed for the next minute boundary (or next second when
  seconds are shown).

Dependencies (the widget may use these without a new ADR): `slint`, `slint-build`,
`nepdate` and `nepdate-patro` (path), `jiff`, `serde` + `toml`, `dirs`, `auto-launch`, `interprocess`,
`arboard` (feature `wayland-data-control`), `log` + `simplelog`, `anyhow`,
`tray-icon` + `muda` (fallback only, ADR-0010); dev-only: `tempfile`.
`xtask` may use `resvg` for icon generation.
Tray day-number icons are **pre-rendered** by `cargo xtask icons` (32 days × 2 digit sets) and
embedded with `@image-url`, so there's no runtime rasteriser or image crate.

## Consequences

- Most logic is unit-testable without a display (≥ 80 % coverage target on `app/`).
- Swapping Slint for Tauri later would touch `ui/`, `ui_bridge.rs` and `main.rs` only.
