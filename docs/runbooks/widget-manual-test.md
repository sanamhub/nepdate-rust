# Widget manual test checklist

Copy this file per release/OS and tick. OS: ______ Version: ______ Tester: ______ Date: ______

## Tray (WR-01)
- [ ] AC1 Icon/title visible after start
- [ ] AC2 Shows today's BS day (Devanagari when ne)
- [ ] AC3 Tooltip shows BS long + AD date
- [ ] AC4 Click toggles popup (macOS: menu → Open)
- [ ] AC5 Menu order correct; Copy BS/AD puts the right text on the clipboard
- [ ] AC6 (Linux GNOME without the AppIndicator extension) popup opens at start; app keeps running

## Popup (WR-02)
- [ ] AC1 Opens next to the tray, fully on-screen (test with the taskbar on each screen edge on Windows)
- [ ] AC2 Hides on focus loss and on Escape; reopening keeps the viewed month, resets after a date change
- [ ] AC4 Light/dark/system themes; Devanagari renders per `docs/plan/devanagari-test-strings.md`

## Month grid (WR-03) / Day detail (WR-04)
- [ ] Grid shape, AD numbers, today outline, Saturday/holiday tint
- [ ] Tithi + event dots in 2081; nothing shown (no error) in 2090
- [ ] Prev/next/today/pickers/wheel; disabled at 1901-01 and 2199-12
- [ ] Next-holiday line
- [ ] Day detail content + copy buttons; Escape order

## Clock (WR-05)
- [ ] Local + Kathmandu shown; Kathmandu hidden when the machine is set to Asia/Kathmandu
- [ ] 12h/24h/seconds; Nepali digits
- [ ] Midnight rollover (set the clock to 23:59:30) updates within 61 s
- [ ] Sleep/resume: correct within 61 s

## Settings / Converter / Lifecycle / Privacy (WR-06, 07, 09, 11)
- [ ] Every setting applies immediately and persists across restart
- [ ] Corrupt settings file → app starts with defaults, `.broken-*` file created
- [ ] Converter BS→AD, AD→BS, Devanagari input, range message, days-between
- [ ] No network connections while running (firewall / Resource Monitor / `lsof -i`)
- [ ] Second launch opens the first instance's popup; exit code 0
- [ ] Launch at login works after reboot; disabling removes the entry
- [ ] Quit leaves no process

## Accessibility (WR-10)
- [ ] Full keyboard operation; screen reader (Narrator / VoiceOver / Orca) reads buttons
