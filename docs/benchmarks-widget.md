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

`LOADED` means the all-CPU average during the row is above 20 %. A loaded row is reported but
**not judged**: a quiet re-run is needed before it can pass or fail. Binary size is not a timing
measurement and is judged even on a loaded machine.

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
