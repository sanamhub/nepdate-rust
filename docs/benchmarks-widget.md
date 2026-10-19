# MitiBar performance

Measured on one Windows x64 machine on 2026-10-19: Acer Nitro AN515-58, Intel Core i5-12500H,
32 GB RAM, Windows 11 Home 10.0.26300. Build: `cargo build -p mitibar --profile release-widget`
(`opt-level = "s"`, fat LTO, `panic = "abort"`, stripped). The app used Slint's software
renderer, with the popup closed, the Windows 11 taskbar text on, English, a 12-hour clock and
seconds hidden. Settings came from a temporary file with `launch_at_login = false`. The idle and
cold-start rows predate keyboard navigation, which does not run while the popup is closed; the
binary size is from the current build.

## Method

`cargo xtask widget-idle --minutes 5` builds the app, starts it with that temporary settings file,
waits 15 s, then samples private bytes, working set, process CPU time, thread count and total CPU
load every 5 s for five minutes. It writes the raw row, stops the process, removes the settings file,
and checks that no `mitibar` login item or real settings file was touched.

A row is `LOADED` when the mean total CPU load before the run, or any sample during it, is 15 % or
more. The measured process's own CPU is subtracted from the load. A LOADED row is reported and gets
no verdict.

Cold start is the `tray ready in ...` line, from the start of `main` to the first tray update.
Each run is a fresh process with the same settings. The load is sampled before the run, during the
startup interval and after it. A run is LOADED when any of those samples is 15 % or more.

ADR-0018 judges idle memory on private bytes and records the working set without a verdict. The
CPU edge is 0.5 % of one core. Binary size is load-independent.

## Results

| Metric | Target | Measured | Verdict |
|---|---|---|---|
| Windows x64 binary, stripped | ≤ 15 MB (stretch ≤ 8 MB) | 12,145,152 bytes (11.58 MiB) | passes 15 MB, misses the 8 MB stretch |
| Idle private bytes, popup closed, 5 min | ≤ 40 MB | 34.35 MB (36,016,128 bytes) | LOADED, no verdict |
| Idle working set, popup closed, 5 min | recorded only | 73.59 MB (77,164,544 bytes) | not judged |
| Idle CPU, popup closed, 5 min | about 0 %, edge 0.5 % of one core | 1.217 % (3.703 CPU s in 304.2 s) | LOADED, no verdict; above the edge |
| Cold start to tray-ready | ≤ 500 ms | 68.0 ms median of 5 (62.4 to 80.4 ms) | LOADED, no verdict |

The idle row is LOADED: mean load before was 47.9 %, during was 31.2 % (6.5 to 99.9). One
5-second sample reached 99.9 %. The private figure is below 40 MB and the CPU figure is above the
0.5 % edge, but neither gets a verdict.

### Cold starts

| Run | Tray-ready | Poll saw | Load before | Load during startup | Load after |
|---|---|---|---|---|---|
| 1 | 80.4 ms | 159.5 ms | 6.6 % | 23.8 % | 19.7 % |
| 2 | 68.0 ms | 107.0 ms | 15.9 % | 21.8 % | 10.6 % |
| 3 | 62.4 ms | 91.0 ms | 6.9 % | 19.4 % | 12.7 % |
| 4 | 63.4 ms | 105.8 ms | 10.8 % | 32.4 % | 17.0 % |
| 5 | 74.0 ms | 108.3 ms | 7.9 % | 27.8 % | 19.4 % |

Every startup sample is above 15 %, so every row is LOADED. The raw durations are below 500 ms,
but they are not a verdict. The poll saw the log line 29 to 79 ms after the process wrote it.

## What this means

MitiBar meets the 15 MB binary target and misses the 8 MB stretch goal. The idle private bytes are
below 40 MB, and the idle CPU is above the 0.5 % edge, but the machine was too busy for either
number to count. Cold start is below 500 ms on every run, also on a busy machine.

0.1.0 is not releasable until a quiet 5-minute row passes private bytes and CPU, and the cold
starts have quiet samples. The macOS and Linux binaries are built in the release workflow and are
not measured here.
