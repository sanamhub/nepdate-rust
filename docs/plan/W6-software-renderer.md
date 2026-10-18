# W6: MitiBar on the software renderer

Read first: ADR-0018 (the decision and its condition), ADR-0010 (why femtovg was chosen),
ADR-0008 §C, `docs/plan/devanagari-test-strings.md`, `docs/benchmarks-widget.md` (W5 rows and
"W5 review: ADR-0018"), the W5 review row in PROGRESS.md.

W5 measured femtovg at 61.79 MB private and 0.825 % of one core idle, and the software renderer
at 34.68 MB and 0.335 %. ADR-0018 moves MitiBar to the software renderer if Devanagari renders
correctly with it. W6 runs that check first, then switches, then measures. No new feature.

## Load rule

Unchanged: a row is LOADED when the all-CPU mean before the row, or any sample during it,
excluding the measured process's own CPU time, is 15 % or more. A LOADED row is committed with
its numbers and gets no verdict. Do not wait for a quiet window and do not estimate.

## Tasks

### W6-00 Plan rows
PROGRESS rows for W6-01 to W6-05, each with its AC copied from here before code.

### W6-01 Devanagari exit check (stop point)
Render every string in `docs/plan/devanagari-test-strings.md` and every W2/W3 snapshot with the
software renderer (`SLINT_BACKEND=winit-software` or the backend selector) at 1x and 2x, light
and dark, English and Nepali, next to the committed femtovg images.

AC:
1. A side-by-side sheet per image (software left, femtovg right) is committed under
   `docs/plan/spike-results/w6/`, made by a script in the repo, not by hand.
2. Each conjunct (क्ष, ज्ञ, श्र), reph (र्क) and matra in the test strings is listed with
   "same", "differs, acceptable" or "wrong", with the reason.
3. If any is "wrong" and no font or Slint feature setting fixes it, stop the phase and report.
   ADR-0018 is then void and nothing in W6-02 starts.

### W6-02 Switch the renderer
Only after W6-01 passes. Select the software renderer on Windows, macOS and Linux. Drop femtovg
from the dependency (`slint` with `default-features = false` and the features MitiBar needs),
unless that is a large change; then keep it behind a non-default feature and say why here.

AC:
1. `cargo tree -p mitibar -e features` shows no femtovg or OpenGL crate in the default build
   (or the documented feature gate).
2. A test or xtask check fails if the default feature set brings femtovg back (negative control).
3. Release binary size is reported before and after.
4. The W2/W3 snapshots are re-rendered with the shipped renderer; each verdict stays
   "not verified" until a human looks.

### W6-03 Visible year buttons
The year control gets visible decrement and increment buttons beside the year text, for pointer
users without a wheel and touch on Windows tablets.

AC:
1. Each button has an accessible label ("Previous year", "Next year", translated), is disabled
   at 1901 and 2199, and has a 24 px or larger hit target.
2. The interaction test drives both buttons, the edges and the Devanagari year text.
3. Keyboard, wheel and accessibility actions keep working.

### W6-04 Quiet idle row and cold starts
`cargo xtask widget-idle --minutes 5` and 5 cold starts on the software-renderer build. Commit
the JSON and the table rows with their load, LOADED or not.

AC:
1. Private bytes and CPU have a verdict against ADR-0018 on a quiet row, or the row is LOADED
   and says so.
2. Cold start samples load during startup, not only before and after, or the table says it
   does not.

### W6-05 Runbook and checklist
`docs/runbooks/release-mitibar.md` step 4 and the known-gaps table reflect W6. The 3-OS manual
checklist gains a line for text quality on the software renderer at 100 % and 200 % scale.

## Checkpoint
Commits, the final `xtask ci` line, the W6-01 sheet and per-glyph list, `cargo tree` evidence,
size before and after, every idle and cold-start line with its load, the year-button test,
doubtful points.
