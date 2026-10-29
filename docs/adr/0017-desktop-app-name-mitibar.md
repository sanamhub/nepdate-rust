> Moved to [sanamhub/mitibar](https://github.com/sanamhub/mitibar) as ADR-0006. The text is kept here unchanged.

# ADR-0017: The desktop app is called MitiBar

- **Status:** accepted
- **Date:** 2026-10-17
- **Approved by:** Sanam
- **Amends:** ADR-0008 (the app's name), ADR-0012 (the widget's tag name and artefact names),
  ADR-0015 (the crate path)

## Context

The desktop app was called "NepDate Widget" and lived in `apps/nepdate-widget`. Raju
Prasai's C# app, which this one is a smaller alternative to, is called NepDateWidget. Sharing the
name invites confusion in search results, in winget and on Product Hunt, where his app is listed as
"NepDate Widget". The libraries keep the NepDate name: they are ports of his library and say so.

The maintainer wanted a name with its own personality that works for Nepali and English speakers.

## Decision

1. The display name is **MitiBar**, written with a capital M and a capital B, in every place a
   person reads it: window and tray titles, tooltips, menus, the README, release notes, the
   installer and the winget package name. *Miti* is Nepali for date. Nepali weekday names end in
   *-bar* (Sombar, Mangalbar), so MitiBar reads like an eighth day of the week, and it lives in
   the taskbar. Tagline: "the eighth day of the week".
2. Every identifier is lower case **`mitibar`**:
   - crate and binary `mitibar` (`mitibar.exe` on Windows), in `apps/mitibar/`;
   - log folder `dirs::data_local_dir()/mitibar/`, log file `mitibar.log`;
   - settings folder `dirs::config_dir()/mitibar/`;
   - gettext domain `mitibar` (`i18n/ne/LC_MESSAGES/mitibar.po`);
   - window classes `mitibar-taskbar-controller` and `mitibar-taskbar-text`;
   - release tag `mitibar-vX.Y.Z` (ADR-0012, amended to this name), `xtask release-check mitibar`.
3. The repository stays `nepdate-rust`. The library crates, `nepdate` and `nepdate-patro`, and
   their tags (`nepdate-vX.Y.Z`) don't change.
4. In the Nepali UI the display name stays `MitiBar` in Latin script, as a brand. The tagline is
   translated.
5. Nothing to migrate: no release has shipped, and settings were not persisted before the
   rename. The old
   log folder `nepdate-widget/` is left alone; it only ever held development logs.

## Consequences

- One rename change touched the crate path, every user-visible string, CI, xtask and docs.
  Accepted ADRs keep the old name in their text; this ADR is the record of the change.
- Later work uses `apps/mitibar` and `mitibar`.
- The name was free on 2026-10-17 on crates.io, npm, PyPI, winget-pkgs and as a GitHub user name.
  The only near match was a dormant GitHub repository, `aksharlabs/NepaliMitibar` (0 stars, last
  pushed January 2023).
