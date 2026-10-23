# MitiBar manual test

Copy this file per release and OS. Record the OS and version, the tester and the date:

```text
OS:                   Version:
Tester:               Date:
Package:              Install source:
```

Install the release candidate from its package, not from a development build. Use a clean user
account or a clean VM, and keep a firewall log or a packet capture open for the privacy line.

## Tray (WR-01)

- [ ] Icon or menu-bar title appears after start.
- [ ] Windows and Linux show today's BS day; Nepali uses Devanagari digits.
- [ ] Tooltip shows `MitiBar` and the BS and AD dates.
- [ ] Left click opens the popup; a second click closes it. On macOS, Open is the first menu item.
- [ ] Menu order is Open, Copy today (BS), Copy today (AD), Settings, Quit, in the current language.
- [ ] Copy BS and Copy AD put the exact strings on the clipboard.
- [ ] Linux without a tray (GNOME without the AppIndicator extension): the popup opens at start
      and the app keeps running.

## Popup (WR-02)

- [ ] Opens at the tray corner, fully on screen.
- [ ] Alt-tab away hides it; Escape hides it.
- [ ] Reopening keeps the viewed month. After the date changes, it opens on today's month.
- [ ] Light, dark and system themes, and every control stays readable.
- [ ] Text quality at 100 % and 200 %: no tofu, clipped or misplaced glyph. Check conjuncts
      क्ष ज्ञ श्र, reph र्क and every matra.

## Month grid and day detail (WR-03, WR-04)

- [ ] One BS month, Sunday first, 5 or 6 rows, blank padding cells.
- [ ] BS day and AD day; the AD month appears on the first AD day of the month.
- [ ] Today's outline, Saturday colour and public-holiday colour.
- [ ] Tithi and event dots where data exists; nothing and no error outside coverage.
- [ ] Previous and next month, Today, the month and year pickers and the mouse wheel.
- [ ] Both range edges: the matching control is disabled, with no crash.
- [ ] Next-holiday line, or no line when none is found.
- [ ] Clicking a day opens the detail with BS and AD long dates, weekday, tithi, holiday and
      events.
- [ ] Copy BS and Copy AD in the detail.
- [ ] Escape and a click outside close the detail first, then the popup.

## Clock (WR-05)

- [ ] Local time plus the extra zone. The extra zone is hidden when the offsets are equal.
- [ ] 12-hour and 24-hour formats, with and without seconds.
- [ ] Nepali digits in the header, clock, grid, detail and both copy strings.
- [ ] Set the clock to 23:59:30: the date rolls over within 61 s.
- [ ] Sleep and resume: the display is correct within 61 s.

## Settings and converter (WR-06, WR-07)

- [ ] Every setting applies immediately and survives a restart.
- [ ] A corrupt settings file starts with defaults and leaves a `.broken-*` file.
- [ ] The zone filter narrows the list as text is typed.
- [ ] BS to AD and AD to BS, including Devanagari digits.
- [ ] Invalid and out-of-range input shows the range message.
- [ ] Days-between on the Days tab.
- [ ] Type in the converter with an IME active.

## Lifecycle and privacy (WR-09, WR-11)

- [ ] A second launch opens the first instance's popup and exits 0; one process remains.
- [ ] Launch at login after a reboot; turning it off removes the entry.
- [ ] Quit leaves no process.
- [ ] No network connections while running.
- [ ] Logs contain no user names or home-folder paths.

## Accessibility (WR-10)

- [ ] Tab and Shift+Tab follow the visual order and skip disabled controls.
- [ ] Return and Space activate the focused button once per press. At 1901 and 2199 the edge
      controls do nothing.
- [ ] Every focused control shows the ink focus ring in light and dark, at 100 % and 200 %.
- [ ] A pointer click on a year button leaves focus on it.
- [ ] Narrator, VoiceOver and Orca read every button, day cell and field.

## Floating bar (WR-08, Windows)

- [ ] First run: the bar sits at its default position on the primary monitor, fully on-screen.
- [ ] Light and dark theme, including a live switch.
- [ ] Drag it: it follows the pointer, stays on one monitor and snaps to the taskbar's centre
      line within 8 px. Quit and start again: it is where it was left.
- [ ] Lock position: a drag does nothing and a click still toggles the popup.
- [ ] Click toggles the popup beside the bar; double-click leaves it open; right-click menu items
      all work in English and Nepali; Hide bar and the tray's Show bar agree.
- [ ] Settings line editor: add, remove, move and move across lines; the bar updates each time.
- [ ] Restart Explorer: the bar stays or returns. Scale 100 %, 150 % and 200 %.

## Global shortcut (WR-13)

- [ ] Win+Shift+N (Cmd+Shift+N, Super+Shift+N on X11) toggles the popup from another app.
- [ ] Record Ctrl+Shift+K, then a combination another app owns: the in-use note shows.
- [ ] Ctrl+Alt+K is refused with the AltGr note; Clear and Reset work.
- [ ] Wayland: the field is disabled with its note.

## Settings window and popup helpers (WR-06, WR-14)

- [ ] The Settings pill, Ctrl+Comma, the tray and the bar menu open one Settings window beside
      the popup; the popup stays open; a pinned popup stays pinned when focus leaves.
- [ ] Saved shows for 2 s after a change; a read-only settings file shows the failure note.
- [ ] Pin, the day tooltip, Go to date (Ctrl+G, with and without `AD`) and the next holidays list.
## Known gaps

- A real alt-tab and a real tray click are not automated. The unit tests call the hide path
  directly.
- IME typing in the converter is untested.
- Text quality with the software renderer is verified on Windows x64 only; macOS and Linux have
  not been run.
- The 3-OS checklist has not been completed for 0.1.0.
