# MitiBar performance (W4-05)

Measured on 2026-10-18 for MitiBar 0.1.0, against the targets in ADR-0008 §C. The machine is
shared and was busy, so every runtime number carries the all-CPU load beside it.

## Machine and method

| Item | Value |
|---|---|
| Machine | Acer Nitro AN515-58, Intel Core i5-12500H, 12 cores / 16 logical processors, 32 GB RAM |
| OS | Windows 11 Home 10.0.26300 |
| Toolchain | rustc 1.98.1, x86_64-pc-windows-msvc |
| Build | `cargo build -p mitibar --profile release-widget` (`opt-level = "s"`, fat LTO, `panic = "abort"`, stripped) |
| Load sample | `Win32_PerfFormattedData_PerfOS_Processor` `_Total.PercentProcessorTime` |

`LOADED` means the all-CPU average during the row is above 20 % in the W4 table below; the W5
rows use the family rule of 15 % (mean before the row or any sample during it). A loaded row is
reported but **not judged**: a quiet re-run is needed before it can pass or fail. Binary size is
not a timing measurement and is judged even on a loaded machine.

The cold-start number is the `tray ready in ...` line that `main` logs from the start of `main`
to the first `update_tray` call. Each run was a fresh process with the popup closed and settings
`launch_at_login = false`, `taskbar_text = true`, English, 12-hour clock, seconds hidden. The
settings file was temporary and was removed after the run.

Idle RSS is `WorkingSet64` after five minutes with the popup closed. Idle CPU is the process
`TotalProcessorTime` delta divided by wall time, as a percentage of one core (the same unit as
the W1 and W1B checks). Load was sampled 30 times, once every 10 s, during the five minutes.

## Results

| Metric | Target | Measured | All-CPU load | Verdict |
|---|---|---|---|---|
| Windows x64 binary, stripped | ≤ 15 MB (stretch ≤ 8 MB) | 12,519,936 bytes = 12.52 MB = 11.94 MiB | 61.8 % average during the idle window (size is load-independent) | meets 15 MB, misses the 8 MB stretch |
| macOS universal binary | ≤ 15 MB | not measured locally; built only in `release-mitibar.yml` | not measured | not judged |
| Linux x86_64 binary | ≤ 15 MB | not measured locally; built only in `release-mitibar.yml` | not measured | not judged |
| Idle RSS, popup closed, 5 min | ≤ 40 MB | 153.86 MB working set (89.48 MB private) | 61.8 % average, range 19 to 100 % | raw miss, quiet re-run needed (W5) |
| Cold start to tray-ready | ≤ 500 ms | 164.0 ms median of 5 (155.7 to 227.0 ms) | 7 to 99 % per run, about 44 % average of the before/after samples | LOADED, no verdict; the raw numbers are below 500 ms |
| Idle CPU, popup closed, 5 min | ~0 %, wake at most once a minute | 6.87 % of one core (21.328 CPU seconds in 310.4 s) | 61.8 % average, range 19 to 100 % | raw miss, quiet re-run needed (W5) |

### Cold start, all five runs

| Run | Tray-ready | Load before | Load after |
|---|---|---|---|
| 1 | 162.5 ms | 7 % | 26 % |
| 2 | 155.7 ms | 30 % | 60 % |
| 3 | 198.0 ms | 25 % | 44 % |
| 4 | 227.0 ms | 58 % | 99 % |
| 5 | 164.0 ms | 38 % | 53 % |

## What this means

- Binary size meets the 15 MB requirement and misses the 8 MB stretch goal. No feature was added
  or removed to change it.
- The idle RSS (153.86 MB) is far above the 40 MB target, and idle CPU (6.87 % of one core) is
  above the about-0 % target. Both rows are LOADED, so they are not verdicts: the machine was
  between 19 % and 100 % busy, averaging 61.8 %. The numbers are high enough to need a quiet
  re-run before the human accepts 0.1.0; they are reported here rather than hidden.
- Cold start is below 500 ms even on this load. It is still marked LOADED and not judged.
- macOS and Linux were not built or measured on this Windows machine. Their sizes and timings
  come from the release workflow run, which needs a GitHub remote.

## W4 review investigation (2026-10-18)

A quick look for causes, not a measurement that replaces the table above. Same machine and
`release-widget` build (12,519,936 bytes), English, seconds hidden, `launch_at_login = false`,
popup never opened. Each run sampled 15 s after start for about 33 s, so the CPU figure is short
and noisy; the memory figures are what matter here.

| Run | Private | Working set | CPU, % of one core | Threads | All-CPU load |
|---|---|---|---|---|---|
| femtovg (default), taskbar text on | 82.8 MB | 127.5 MB | 1.81 | 21 | 37 % |
| software renderer (`SLINT_BACKEND=winit-software`), taskbar text on | 34.8 MB | 57.8 MB | 0.88 | 17 | 12 % |
| femtovg, `--no-taskbar-text` | 77.0 MB | 113.2 MB | 1.87 | 20 | 23 % |

Findings:

- **The OpenGL renderer is most of the memory.** With femtovg the Intel OpenGL driver
  (`igxelpicd64.dll`, `igdml64.dll` and the shared compiler `igc64.dll`) is loaded at start, even
  though the popup is never shown: the popup's window and GL context exist from startup
  (`PopupWindow::new()` in `main`). The software renderer saves 48 MB private and 70 MB working
  set. ADR-0010 chose femtovg, so changing the renderer or creating the popup lazily is a W5
  decision with numbers, not a review fix.
- **The taskbar text costs about 6 MB private and 14 MB working set** (Direct2D and DirectWrite;
  the D3D11 user-mode driver stays mapped in every run).
- **Something wakes the UI thread while idle.** Even on the software renderer at 12 % load the
  process used 0.88 % of one core. A known waker: the single-instance listener is polled by a
  repeated 250 ms `slint::Timer` (`main.rs`), 4 wakes a second, against WR-05 AC5 ("wake at most
  once a minute"). Other timers and the taskbar window procedure are not yet audited.
- The W4 working set (153.86 MB) is higher than the 127.5 MB here mainly because of the five
  minute window and the heavier load; both runs agree that femtovg is the cause.

Follow-up: W5 (`docs/plan/W5-idle-footprint.md`) measures with a harness, removes the 250 ms
poll, and decides the renderer and popup lifetime with before/after numbers. 0.1.0 is not
releasable until idle RSS and idle CPU are measured on a quiet machine.

## W5-01 idle harness (2026-10-18)

`cargo xtask widget-idle [--minutes N]` (Windows only; macOS and Linux print `not supported
here` and exit 0, counted as skipped, never passed) builds `release-widget` and starts it with
`--settings-file <temp>` and `launch_at_login = false`. The new documented flag makes the run
use that file instead of `%APPDATA%\mitibar\settings.toml`, skip the login item and skip the
single-instance socket. After 15 s of warmup the harness samples private bytes, working set,
process CPU seconds, thread count and the all-CPU load every 5 s, writes
`docs/benchmarks/widget/<date>/idle.json`, kills the process, removes the temporary settings,
checks that `HKCU\Software\Microsoft\Windows\CurrentVersion\Run` still has no `mitibar` value,
and checks that the real settings file is byte-for-byte unchanged.

The load rule is the W5 family rule: a row is `LOADED` when the all-CPU mean before the row, or
any sample during it, excluding the measured process's own CPU time, is 15 % or more. A loaded
row is reported and gets no verdict. RSS is the working set against the 40 MB target; CPU is
judged against "about 0 %" with a testable edge of 0.5 % of one core.

Baseline before W5-03 (1-minute run, 2026-10-18 21:47, the same build as W4):

| Metric | Measured | All-CPU load | Verdict |
|---|---|---|---|
| Private bytes | 82.51 MB (86,515,712 bytes) | before 55.3 %, during mean 53.9 % (19.5 to 70.9) | LOADED, no verdict |
| Working set (RSS) | 131.47 MB (137,859,072 bytes) | same | LOADED, no verdict; raw miss against 40 MB |
| Idle CPU | 1.392 % of one core (0.844 CPU s in 60.6 s) | same | LOADED, no verdict |
| Threads | 16 | same | reported |

The three W5-01 unit tests cover the 14.9 % / 15.0 % / single-15 % rule, the RSS and CPU
verdict table, and the `--minutes` default and zero rejection.

### Final W5-06 run (5 minutes, 2026-10-18 22:47)

The machine was not quiet: the mean before the row was 30.6 % and the mean during it 34.7 %
(5.0 to 72.5). The row is LOADED and gets no verdict, as the family rule says.

| Metric | Measured | All-CPU load | Verdict |
|---|---|---|---|
| Private bytes | 61.66 MB (64,651,264 bytes) | before 30.6 %, during 34.7 % (5.0 to 72.5) | LOADED, no verdict |
| Working set (RSS) | 108.47 MB (113,741,824 bytes) | same | LOADED, no verdict; raw miss against 40 MB |
| Idle CPU | 1.455 % of one core (4.422 CPU s in 303.9 s) | same | LOADED, no verdict; raw miss against about 0 % |
| Threads | 15 | same | reported |

An earlier 5-minute attempt at 22:35 was also LOADED: private 90.78 MB, working set 151.11 MB,
CPU 2.279 % of one core, before 7.4 % and during 20.9 % (5.6 to 64.8). The private figure is
27 MB above the second run, so the first run is reported but not used as a cause. A 3-minute
quiet sample with taskbar text on stayed flat at 64.6 MB private, so the growth did not
reproduce there; the two 5-minute rows differ by load state.

### W5-06 cold starts

Five fresh processes with the temporary settings file, the popup never opened, seconds hidden.
The load is the all-CPU sample immediately before and after the run, the W4-05 method; no sample
was taken inside the startup interval itself. A row is LOADED when either sample is 15 % or more.

| Run | Tray-ready | Load before | Load after | Verdict |
|---|---|---|---|---|
| 1 | 142.9 ms | 48 % | 7 % | LOADED, no verdict |
| 2 | 121.5 ms | 40 % | 20 % | LOADED, no verdict |
| 3 | 92.2 ms | 11 % | 26 % | LOADED, no verdict |
| 4 | 88.7 ms | 2 % | 10 % | before/after quiet, below 500 ms |
| 5 | 87.8 ms | 3 % | 2 % | before/after quiet, below 500 ms |

Median 92.2 ms, against the W4 median of 164.0 ms. The lazy popup removed the window and GL
context creation from the startup path, which is the likely reason; the quiet rows are below
500 ms and the loaded rows get no verdict. The machine never stayed quiet for a 5-minute idle
row during the phase, so W5-06 commits the LOADED numbers and stops; it did not wait or
estimate.

## W5-02 idle wake-ups (2026-10-18)

Before: a repeated 250 ms `slint::Timer` polled the single-instance listener,
`platform::single_instance::take_request()`, so the UI thread woke four times a second forever.
After: the listener thread calls `slint::invoke_from_event_loop(run_open_request)` only when a
second instance writes `open\n`; the closure captures no Slint type and the non-`Send` handler
lives in a UI thread-local.

Waker audit while idle with seconds hidden:

| Waker | Period | Work while idle |
|---|---|---|
| `arm_timer` (`main.rs`) | next minute boundary, single-shot | tray tooltip/icon, hidden popup update, taskbar lines |
| single-instance listener | blocked on `listener.incoming()` | none until a second launch; then one `invoke_from_event_loop` |
| settings save timer | 300 ms after a settings change, single-shot | none unless the user changed a setting |
| taskbar controller (`controller.rs`) | no `SetTimer` | `WM_SETTINGCHANGE`, `TaskbarCreated`, mouse and paint messages only |
| taskbar UIA worker (`uia.rs`) | blocked on `GetMessageW` | none until an Explorer event |
| tray (`tray.rs`) | no timer | OS tray events only |
| snapshot/probe timers | `snapshot` feature only | none in the release build |

The `--wake-probe 120` run (snapshot build, popup never opened, seconds hidden) wrote
`seconds=120` and `wakes=2`: two minute ticks and no second-launch wake, against the AC of at
most 2. The second-launch interaction test hides the popup, installs the handler and runs the
wake path; it shows the popup again. The probe left no process and no `mitibar` Run value.

## W5-03 where the private bytes go (2026-10-18)

The measurements below use the same Windows x64 machine and the `release-widget` build. The
1-minute rows use `cargo xtask widget-idle --minutes 1`; the 20-second rows are a manual
`Get-Process` sample after 20 s. Private bytes and working set do not depend on the all-CPU load;
CPU does, so each row carries its load.

| Run | Private | Working set | CPU, % of one core | Threads | All-CPU load |
|---|---|---|---|---|---|
| femtovg, popup created at start (W5-01 commit, baseline) | 82.80 MB | 131.79 MB | 0.413 | 16 | before 12.6 %, during 14.4 % (max 46.0) |
| femtovg, popup created on first open (W5-03) | 61.79 MB | 112.92 MB | 0.825 | 15 | before 7.3 %, during 7.2 % (3.5 to 11.2) |
| femtovg, popup on first open, `--no-taskbar-text` (20 s) | 58.30 MB | 106.05 MB | not sampled | 16 | not sampled |
| software renderer, popup on first open | 34.68 MB | 60.23 MB | 0.335 | 11 | before 7.8 %, during 6.3 % (3.4 to 8.2) |
| femtovg, popup at start, `--no-taskbar-text` (W4 review) | 77.0 MB | 113.2 MB | 1.87 | 20 | 23 % |
| software renderer, popup at start (W4 review) | 34.8 MB | 57.8 MB | 0.88 | 17 | 12 % |

What each number shows:

- **The popup created at start is a real cause.** Creating it on first open saves 21.01 MB
  private (82.80 to 61.79) and 18.87 MB working set (131.79 to 112.92) on the same machine. It
  is the W5-03 code change. The GL driver is still loaded at start even without the popup, so
  this is not the whole gap.
- **The femtovg renderer is the remaining large cause.** With the popup on first open, the
  software renderer saves another 27.11 MB private (61.79 to 34.68) and 52.69 MB working set
  (112.92 to 60.23), and cuts CPU from 0.825 % to 0.335 %. ADR-0010 chose femtovg; switching
  renderers or changing the ADR-0008 §C target needs a new ADR, so W5-03 did not change either.
- **The taskbar text costs about 3.49 MB private and 6.87 MB working set** in the lazy build
  (61.79 to 58.30, 112.92 to 106.05). Direct2D and DirectWrite resources, not a periodic timer.
- **The bundled fonts are 1,284,988 bytes on disk** (Noto Sans 971,808 and Noto Sans Devanagari
  313,180, both variable). They cannot account for a 20 MB gap by file size; the renderer's font
  and glyph caches are counted in the renderer rows above. W5-03 did not remove a bundled font.
- **The time-zone list is 598 names and 8,504 bytes** by `String::capacity` (test
  `the_time_zone_list_is_small`). It is not a cause.

After the lazy-popup change, the quiet femtovg row is still a raw miss: 112.92 MB working set
against 40 MB, and 0.825 % of one core against "about 0 %". The software row would meet the CPU
target and the 40 MB private figure, but its 60.23 MB working set still misses the RSS target.
The final W5-06 5-minute run was LOADED (30.6 % before, 34.7 % during): 61.66 MB private,
108.47 MB working set, 1.455 % of one core. It gets no verdict, and it does not change the
conclusion that the femtovg working set misses 40 MB and the software renderer still misses it
at 60.23 MB. A renderer or target decision needs a new ADR before code.

### W5 review: ADR-0018

ADR-0018 judges idle memory on private bytes (the process's own commit) and records the working
set without a verdict, because the working set counts shared GPU driver, DWM and font pages. On
the rows above, the software renderer passes (34.68 MB private, 0.335 % of one core) and
femtovg misses (61.79 MB, 0.825 %). The same ADR moves MitiBar to the software renderer, on the
condition that W6-01 shows Devanagari renders correctly with it: every committed image so far
was rendered with femtovg, and the software-renderer rows above measured memory, not text.
`cargo xtask widget-idle` now prints the private-bytes verdict and the working set as recorded
only.
