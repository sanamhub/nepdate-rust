> Moved to [sanamhub/mitibar](https://github.com/sanamhub/mitibar) as ADR-0002. The text is kept here unchanged.

# ADR-0010: GUI framework for the widget: Slint

- **Status:** accepted (license option: see ADR-0013)
- **Date:** 2026-09-24
- **Approved by:** Sanam

## Context

Requirements (ADR-0008, `docs/design/widget-requirements.md`): tray / menu-bar icon,
small borderless popup, month grid, **correct Devanagari rendering** (conjuncts क्ष ज्ञ
श्र, reph र्क, matras), Windows + macOS + Linux (X11 and Wayland), small binary, low
idle RAM, fast start, accessible, and learnable by a Rust beginner.

Research snapshot (2026-09-24, versions from crates.io):

| Option | Version | Tray | Devanagari shaping | Size / RAM (est.) | A11y | Beginner fit |
|---|---|---|---|---|---|---|
| **Slint** | 1.18.1 | built-in `SystemTrayIcon` (since 1.17) | yes (Parley/HarfRust on femtovg; Skia also shapes) | 3–8 MB / 15–40 MB | AccessKit | declarative `.slint` markup + live preview; Rust only for logic |
| Tauri | 2.11.6 | built-in | excellent (browser engine) | 3–10 MB / 30–100+ MB (webview) | best | needs HTML/CSS/JS + IPC; Tauri 3 alpha already out |
| iced | 0.14.0 | none (add `tray-icon`) | yes (cosmic-text) | 8–17 MB / 50–100 MB | none | Elm architecture, pre-1.0 churn |
| egui/eframe | 0.36.2 | via `tray-icon` | **risky** for Indic scripts (open issues) | 4–18 MB | AccessKit | easiest, but disqualified by shaping risk |
| Dioxus desktop | 0.7.10 | yes | webview | webview RAM | browser | 0.8 API churn |
| Makepad, gpui, Xilem, Floem | – | no | varies | – | limited | too immature / host-app-specific |

## Decision

- Use **Slint 1.18.x** (`slint = "1.18"`, `slint-build` for compiling `.slint` files).
- Renderer: **femtovg** (default). If Devanagari quality issues appear in the font test,
  switch to the Skia renderer (feature flag only, no code changes).
- Windowing backend: Slint's **winit** backend.
- Tray: Slint's `SystemTrayIcon`. **Fallback** if a blocker is hit (e.g. Linux menu
  bug slint-ui/slint#13624 still open): `tray-icon` 0.25 (ksni backend on Linux) +
  `muda` 0.20, driven from the Slint event loop. The switch is contained in
  `apps/mitibar/src/platform/tray.rs` (one module behind a small trait).
- Fonts: bundle **Noto Sans Devanagari** and **Noto Sans** (both OFL-1.1) via
  `import "fonts/...ttf"` in `.slint`. Do not rely on system fonts.
- The first widget milestone includes a **font test with an exit criterion**: render the test
  strings in `docs/design/devanagari-test-strings.md` on all 3 OSes, screenshot, and compare.
  If Slint fails on any OS and Skia does not fix it, stop and raise it with the maintainer;
  the runner-up is **Tauri 2** (new ADR required).

## Consequences

- UI lives in `apps/mitibar/ui/*.slint`; Rust code stays small and testable.
- MSRV is set by Slint (1.92 at 1.18.1); the workspace pins the dev toolchain separately.
- Wayland limits (no client-side positioning or always-on-top) apply to every toolkit;
  handled in WR-02/WR-08 acceptance criteria.
- Slint is used under its Royalty-free 2.0 license (ADR-0013).
