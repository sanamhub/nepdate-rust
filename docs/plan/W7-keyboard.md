# W7: Keyboard completeness

Read first: WR-02 AC2 and WR-10 AC2 in `docs/plan/widget-requirements.md`, ADR-0008 §B,
ADR-0010, ADR-0015, `docs/plan/widget-design.md` §2 and §3, the W2 keyboard task, and the W6
review row in `PROGRESS.md`.

W6 left two keyboard gaps. Return or Space on a Tab-focused year button was never tested, and a
pointer click on a year button did not move focus to it. The focus ring was also inconsistent:
the year box used Slint's accent colour, the day cells and the header Convert button had no ring,
and the standard widgets used Slint's own border. W7 fixes those, then proves the whole Tab path.

No renderer change, no new dependency, and no new user-facing feature. The new
`focused-control` property is a read-only test hook; it draws nothing.

## Focus model

The popup has four focus surfaces. Only one is active at a time.

| Surface | When | Tab order |
|---|---|---|
| Base | always, unless an overlay is open | Convert, Previous month, Next month, Month, Year, Previous year, Next year, Today, then the real day cells row by row |
| Day detail | `detail-visible` | Copy BS, Copy AD |
| Converter | `converter-visible` | Close converter, Convert tab, Days tab, BS → AD, AD → BS, Date to convert. On the Days tab the last two are From BS date and To BS date |
| Settings | `settings-visible` | Close settings, Language, Theme, 24-hour clock, Show seconds, Filter zones, No extra clock, Launch at login, Taskbar text (Windows 11) |

Rules:

- Tab moves forward, Shift+Tab moves backward. The order is the reading order of the controls,
  top to bottom and left to right.
- Opening an overlay makes the base inert. The next Tab enters the overlay at its first
  control, because no base control is in the cycle. Closing the overlay returns focus to the
  header Convert button, the first base control. Slint ignores a `focus()` call made before
  the overlay is attached to the window, so W7 does not promise focus on the opening click.
- The popup root is not a Tab stop. It receives initial focus so Escape works, then the first
  Tab enters the base order. Moving focus to a control inside the popup is not a window focus
  loss and does not hide it.
- The popup hides when winit reports `Focused(false)`. A FocusScope `focus-lost` handler cannot
  do this once a child holds focus, because Slint sends that event only to the focused item. The
  root `FocusScope` keeps a `WindowActivation` fallback for the case where the root itself holds
  focus.
- These are not Tab stops: padding day cells, disabled controls at the 1901 and 2199 edges,
  hidden overlays, static text, weekday headers, scrims, and the settings zone rows. The zone
  rows stay pointer-only; the filter above them is keyboard reachable. See Limits.

## Tasks

### W7-00 Plan rows
Write this file, the roadmap row, and the W7-01 to W7-03 rows in `PROGRESS.md` before code.

AC: the plan, roadmap, and three pending rows are committed before any code change.

### W7-01 Focus order
Add the order above to the code as one testable sequence per surface. The header Convert button,
the month controls, the year controls, and the day cells report their focus through
`focused-control`. The overlay components report their own controls.

AC:
1. `tab_and_shift_tab_follow_the_focus_order` starts at the popup root, presses Tab once per
   stop through the whole base order, and asserts the exact `focused-control` value at every
   step. It then presses Shift+Tab and asserts the reverse sequence. The expected day list is
   read from the populated cells, not hard-coded.
2. `disabled_and_hidden_controls_are_not_focus_stops` checks that Previous month and Previous
   year are skipped at 1901 Baishakh, that Next month and Next year are skipped at 2199
   Chaitra, and that padding cells never appear. It also checks that converter and settings
   controls are absent while their overlays are closed.
3. `overlays_take_and_return_focus` opens the converter, presses Tab once and asserts that
   Close converter has focus, walks its order with Tab and Shift+Tab, closes it with Escape,
   and asserts that focus returns to Convert and the next Tab reaches Previous month. The same
   check covers settings and the day detail.
4. The focused control is reported at every step; a missed or extra stop fails with the
   sequence, not only the final count.

### W7-02 Activation and focus ring
Return and Space activate the focused year button and the focused month arrow. One press is one
step; the key release adds nothing. A disabled edge control is not focusable and its accessible
action stays a no-op.

The focus ring is a `Theme.ink` border. Custom controls draw 2 px. Standard Slint widgets get a
3 px overlay that covers Slint's 2 px outer plus 1 px inner border, so the visible ring is the
token colour in both themes. The ring is drawn on every control in the W7-01 orders, in light
and dark, at 1x and 2x.

AC:
1. `return_and_space_step_once_per_press` focuses Next year and Previous year through Tab and
   drives Return, Space, and a key release; each press moves one year and the release moves
   none. At 1901 and 2199 the disabled button is skipped and the action is a no-op.
2. The same test covers Previous month and Next month, including the 1901 and 2199 edges.
3. `focus_ring_ink_has_at_least_3_to_1_contrast` computes the WCAG ratio between `Theme.ink`
   and `surface`, `surface-raised`, and the Fluent control background in both themes. Every
   ratio is at least 3:1. The measured ratios go in the W7-03 README and the report.
4. A snapshot with Next year focused shows the ring in light and dark at 1x and 2x. The ring
   is not clipped and does not cover the `‹` or `›` glyph.

### W7-03 Focus snapshots
Add eight PNGs under `docs/plan/spike-results/w7/`: Next year focused, light and dark, en and
ne, 1x and 2x. They use the same `--popup --snapshot --today 2081-01-01` path as W2, plus a
snapshot-only `--focus-year-next` flag. Every verdict is "not verified" until a human looks.

AC:
1. The eight images exist, are committed, and the README lists theme, language, scale, viewport,
   and verdict for each.
2. `--focus-year-next` is accepted only with `--popup --snapshot`; the parser rejects it
   otherwise, like `--detail` and `--settings`.
3. Each image is rendered twice and the two SHA-256 values are equal. The README records both
   hashes for every image.
4. The README states the sampled ring and background pixels and their WCAG ratios. The agent
   does not mark any image verified.

## Limits

- Windows x64 only. macOS and Linux focus rings and keyboard paths are unverified.
- The settings zone rows are pointer-only. Making a 598-row list keyboard-navigable is a
  separate task; the filter is keyboard reachable and Escape closes the panel.
- Scrims are pointer-only. The Close buttons and Escape cover the keyboard path.
- The standard widgets' built-in 3 px focus border is hidden under the token overlay. If a
  future Slint version changes that border, the overlay still draws the token ring, but the
  "covers it" claim needs rechecking.
- No renderer, feature, dependency, or packaging change.

## Checkpoint

Commits, the last `cargo xtask ci` line verbatim, test counts, the focus order table, the
contrast ratios, the eight snapshot hashes, and doubtful points.
