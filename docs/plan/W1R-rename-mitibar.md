# W1R: Rename the desktop app to MitiBar

Read first: ADR-0017 (the decision and every name), ADR-0012 (tags), ADR-0015 (crate layout).

Starts after W1B is merged into `main`. No behaviour changes in this phase: the same tests, the
same snapshots apart from the name, the same CI steps. `cargo xtask ci` passes at every commit.

Display name `MitiBar` (capital M, capital B) wherever a person reads it. Identifier `mitibar`
(lower case) everywhere else. Never `Mitibar`, `MITIBAR` or `miti-bar`.

## Tasks

### W1R-01 Move the crate
- `git mv apps/nepdate-widget apps/mitibar`, so history follows the files.
- `apps/mitibar/Cargo.toml`: `name = "mitibar"`. The binary takes the crate name; check that
  `target/debug/mitibar.exe` is built.
- Root `Cargo.toml` workspace members, `Cargo.lock`, `.github/workflows/ci.yml` (every
  `-p nepdate-widget` and path), `xtask/src/main.rs` (the `--features snapshot` step),
  `xtask/src/icons.rs` (the four paths), `xtask/src/release.rs`:
  - the package `mitibar` at `apps/mitibar`;
  - tag prefix `mitibar` (ADR-0017 §2), so the tag is `mitibar-vX.Y.Z`;
  - its tests, and the error message `expected nepdate or mitibar`.
- `xtask icons --check` and `codegen --check` still pass, which proves the generated files moved
  with their generators.

AC: `cargo xtask ci` passes. `git log --follow apps/mitibar/src/main.rs` shows the W0 history.

### W1R-02 Identifiers inside the app
- Logging: folder `mitibar`, file `mitibar.log`, the start-up line `mitibar logging to ...`, and
  the module doc comment.
- Gettext: `git mv` the `.po` file to `i18n/ne/LC_MESSAGES/mitibar.po`; `build.rs` and
  `app/settings.rs` follow. The `.po` header's project field says `mitibar`.
- Windows classes in `platform/windows/controller.rs`: `mitibar-taskbar-controller` and
  `mitibar-taskbar-text`. The UIA worker thread name: `mitibar-taskbar-uia`.
- Nothing else: there is no persisted setting yet (W3), so there is nothing to migrate.

AC: run the release build once; the log appears at `%LOCALAPPDATA%\mitibar\mitibar.log`. Report
the path and its first line.

### W1R-03 What the user reads
- `git grep -n "NepDate"` inside `apps/mitibar/ui` and `apps/mitibar/src`. For each hit, list in
  the report whether it names the app (change to `MitiBar`) or the library or calendar (keep).
- The app name appears at least in the tray tooltip's first line, the popup window's `title`
  (the taskbar and Alt+Tab show it) and the right-click menu's quit item if it names the app. Use
  one `pub const APP_NAME: &str = "MitiBar";` in `app/` and one Slint global property fed from it,
  not string literals in several files.
- Nepali UI: the name stays `MitiBar` in Latin script (ADR-0017 §4); no `.po` entry for it.
- Regenerate the W1 popup and tray snapshots and the W1B taskbar snapshots that show a title or
  tooltip text. Mark them "not verified"; the reviewer looks.

AC: a unit test asserts `APP_NAME == "MitiBar"` and that the Slint property equals it (one place
to change, one test that sees both).

### W1R-04 Docs
- `README.md`: the app section is titled `MitiBar`, with the tagline from ADR-0017 and one
  sentence on the name (miti, the -bar of weekday names, the taskbar).
- `apps/mitibar/CHANGELOG.md` heading, `docs/runbooks/release-widget.md` (`git mv` to
  `release-mitibar.md`, tag `mitibar-vX.Y.Z`), `SECURITY.md`, `docs/competitors.md`,
  `docs/legal/README.md`, `docs/learn/*`, `CLAUDE.md`, `AGENTS.md`.
- Plans for future work: `widget-requirements.md`, `widget-design.md`, `W2-month-grid.md`,
  `W3-converter-settings-lifecycle.md` (settings folder `dirs::config_dir()/mitibar/`),
  `W4-package-and-release.md` (installer and winget names `MitiBar`, id `sanamhub.MitiBar`),
  `00-ROADMAP.md`.
- Leave as they are: Accepted ADRs 0001 to 0016 (ADR-0017 records the change), the
  history rows in `PROGRESS.md`, the finished phase plans (`W0`, `W1`, `W1B`) and
  `docs/plan/spike-results/`. They describe what happened under the old name.
- References to Raju Prasai's app stay "NepDateWidget": it is his app's name.

AC: `git grep -nE "nepdate-widget|nepdate_widget|NepDate Widget"` prints only lines in the
files listed under "Leave as they are", plus lines that name the C# app. List any others in the
report with a reason.

## Checkpoint

Add `| W1R checkpoint | ready for review |` to `PROGRESS.md`. Report:
- commits;
- the `xtask ci` line;
- the W1R-01 `git log --follow` output (first and last line);
- the log path and its first line;
- the W1R-03 table of `NepDate` hits with keep or change;
- the W1R-04 grep output with reasons;
- the paths of the regenerated snapshots, marked "not verified".
