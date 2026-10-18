# W5: MitiBar idle footprint and 0.1.0 readiness

Read first: ADR-0008 §C (targets), ADR-0010 (Slint, femtovg), ADR-0012 (packaging),
ADR-0015, `docs/benchmarks-widget.md` (W4 numbers and the W4 review investigation),
`docs/runbooks/release-mitibar.md` (Known gaps before 1.0).

W4 left 0.1.0 not releasable: idle RSS (153.86 MB working set, 89.48 MB private, target 40 MB)
and idle CPU (6.87 % of one core, target about 0 %) are raw misses, measured on a loaded machine.
W5 measures them with a repeatable harness, removes the causes it can prove, and closes the
other W4 review items. No new feature.

## Load rule

The family rule: a row is **LOADED** when the all-CPU mean before the row, or any sample during
it, excluding the measured process's own CPU time, is 15 % or more. A loaded row is reported
with its numbers and gets no verdict. W4 used 20 %; W5 uses 15 %.

## Tasks

### W5-00 Plan rows
PROGRESS rows for W5-01 to W5-06, each with its AC copied from here before code.

### W5-01 Idle measurement harness
`cargo xtask widget-idle [--minutes N]` (Windows first; macOS and Linux print "not supported
here" and exit 0, counted as skipped, never passed). It builds `release-widget`, starts it with
a temporary settings file (`launch_at_login = false`) through a new documented
`--settings-file <path>` flag, waits 15 s, then samples for N minutes (default 5): private bytes,
working set, process CPU seconds, thread count and the all-CPU load once every 5 s. It prints one
line per metric with LOADED or a verdict against ADR-0008 §C, writes
`docs/benchmarks/widget/<date>/idle.json`, kills the process, removes the temporary files, and
checks that no `HKCU\...\Run` value `mitibar` was created.

AC: unit tests for the LOADED rule (14.9 % quiet, 15.0 % loaded, a single 15 % sample loaded)
and for the verdict table; a run on this machine with its JSON committed, LOADED or not.

### W5-02 Idle wake-ups
With seconds hidden the app must wake at most once a minute (WR-05 AC5). Known waker from the
W4 review: the single-instance listener is polled by a 250 ms repeated `slint::Timer`
(`main.rs`, WR-09 AC1), so the UI thread wakes 4 times a second forever. Replace it with a
wake from the listener thread (`slint::invoke_from_event_loop`). Audit every other timer and
the taskbar-text window procedure for periodic work while idle and record what each does.

AC: an idle-wake counter (test-only or behind the `snapshot` feature) shows at most 2 UI-thread
wakes in a 2-minute headless run with seconds hidden; the second-launch interaction test still
opens the first instance's popup.

### W5-03 Idle memory
The W4 review found the femtovg OpenGL driver loaded at start for a popup that is never
shown: 82.8 MB private with femtovg, 34.8 MB with the software renderer, same build.
Find where the private bytes go before changing anything, with numbers in
`docs/benchmarks-widget.md`: femtovg versus the software renderer (`SLINT_BACKEND`), the popup
window created at start versus on first open, the bundled variable fonts, the time-zone list,
the taskbar-text Direct2D resources. Then remove what the numbers show, one commit per cause,
each with its before/after line. A renderer change (ADR-0010) or a target change (ADR-0008 §C)
needs a new ADR and stops the phase for review first.

AC: private bytes and working set before and after each change on the same machine and load
state; the final W5-01 run.

### W5-04 Year control digits
The month header's year `SpinBox` shows ASCII `2083` in Nepali (W4 review, visible in
`docs/plan/spike-results/w2/month-grid-light-ne-1x.png`): Slint's `SpinBox` has no formatter.
Replace it with a control that shows `lang.digits(year)` and keeps the 1901 to 2199 range,
keyboard steps, mouse wheel and the accessible label "Year" with the localized value.

AC: interaction test in both languages (Devanagari value in `ne`, ASCII in `en`), range edges
disabled, re-rendered W2 snapshots marked not verified.

### W5-05 Release workflow gaps
From the runbook's "Known gaps before 1.0", fix the two that are code, both still gated off:
sign the release binary before packaging on Windows (the installer and the zip then carry a
signed `mitibar.exe`), and on macOS sign the `.app` before the DMG is built, then notarize and
staple the DMG. Add `aarch64-pc-windows-msvc` to the Windows build (cross-compiled on
`windows-latest`) per the ADR-0012 table. The static test in `xtask/src/workflow.rs` checks each,
with a negative control.

AC: `release_workflow_matches_the_plan` and the new negative controls pass; actionlint clean;
the runbook table updated. The workflow still cannot run without a remote; say so.

### W5-06 Quiet re-run
Run W5-01 for 5 minutes and the W4-05 cold-start method 5 times when the machine is quiet. If it
never is during the phase, commit the LOADED numbers and stop; do not wait or estimate.

## Checkpoint

The implementation agent stops here for review. The reviewer checks:

- [ ] `cargo xtask ci` passes.
- [ ] Each memory or CPU change has a before/after line on the same load state.
- [ ] No renderer or target change without a new ADR.
- [ ] The Known gaps table in the release runbook matches what is left.
- [ ] Nothing was tagged, pushed or published.
