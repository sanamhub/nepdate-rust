//! `cargo xtask widget-idle` (W5-01): measure the release widget's idle footprint on Windows.
//!
//! The harness builds `release-widget`, starts it with a temporary settings file
//! (`launch_at_login = false`) through the app's `--settings-file` flag, waits 15 s, then samples
//! private bytes, working set, process CPU, thread count and the all-CPU load every 5 s. It writes
//! `docs/benchmarks/widget/<date>/idle.json`, kills the process, removes the temporary files and
//! checks that no `HKCU\...\Run` value `mitibar` was created.
//!
//! The load rule (W5 plan): a row is LOADED when the all-CPU mean before the row or any sample
//! during it, excluding the measured process's own CPU time, is 15 % or more. A loaded row is
//! reported and gets no verdict.

// The counters are byte counts and 100 ns ticks from Win32. An f64 has 52 bits of mantissa, so a
// value above 4 PB or 142 years would round; that is far outside a five-minute process sample.
#![allow(clippy::cast_precision_loss)]

use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

use anyhow::{Context, Result, bail};

use crate::codegen::workspace_root;

const DEFAULT_MINUTES: u32 = 5;
const WARMUP: Duration = Duration::from_secs(15);
const SAMPLE: Duration = Duration::from_secs(5);
/// ADR-0008 §C: idle RSS at most 40 MB. The harness uses 40 MiB, the unit W4 has reported in.
const RSS_TARGET_BYTES: u64 = 40 * 1024 * 1024;
/// ADR-0008 §C says "about 0 %". Half a percent of one core is the harness's testable edge.
const CPU_TARGET_PERCENT: f64 = 0.5;

/// One verdict cell in the table.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Verdict {
    Pass,
    Miss,
    Loaded,
}

impl Verdict {
    fn label(self) -> &'static str {
        match self {
            Verdict::Pass => "PASS",
            Verdict::Miss => "MISS",
            Verdict::Loaded => "LOADED (no verdict)",
        }
    }
}

/// Parses `--minutes`. The default is 5; a run must be at least one sample long.
fn parse_minutes(value: Option<&str>) -> Result<u32> {
    let minutes = match value {
        None => DEFAULT_MINUTES,
        Some(text) => text
            .parse::<u32>()
            .with_context(|| format!("--minutes must be a whole number, got {text}"))?,
    };
    if minutes == 0 {
        bail!("--minutes must be at least 1");
    }
    Ok(minutes)
}

/// The W5 load rule: the mean before the row or any sample during it is 15.0 % or more.
pub(crate) fn is_loaded(before_mean: f64, during: &[f64]) -> bool {
    before_mean >= 15.0 || during.iter().any(|sample| *sample >= 15.0)
}

/// ADR-0008 §C idle RSS target, applied to the working set (the Windows RSS).
pub(crate) fn rss_verdict(working_set_bytes: u64, loaded: bool) -> Verdict {
    if loaded {
        Verdict::Loaded
    } else if working_set_bytes <= RSS_TARGET_BYTES {
        Verdict::Pass
    } else {
        Verdict::Miss
    }
}

/// ADR-0008 §C idle CPU target ("about 0 %"), tested at most half a percent of one core.
pub(crate) fn cpu_verdict(percent_of_one_core: f64, loaded: bool) -> Verdict {
    if loaded {
        Verdict::Loaded
    } else if percent_of_one_core <= CPU_TARGET_PERCENT {
        Verdict::Pass
    } else {
        Verdict::Miss
    }
}

pub(crate) fn run(minutes: Option<&str>) -> Result<()> {
    let minutes = parse_minutes(minutes)?;
    #[cfg(not(windows))]
    {
        println!("widget-idle: not supported here (Windows only); skipped");
        let _ = minutes;
        Ok(())
    }
    #[cfg(windows)]
    {
        windows_run(minutes)
    }
}

#[cfg(windows)]
fn windows_run(minutes: u32) -> Result<()> {
    let root = workspace_root();
    if running_mitibar()? {
        bail!("a mitibar process is already running; close it before the idle harness");
    }
    if run_value_exists()? {
        bail!("a mitibar Run value already exists; the harness refuses to touch a real login item");
    }

    let target = target_dir(&root);
    let status =
        std::process::Command::new(std::env::var_os("CARGO").unwrap_or_else(|| "cargo".into()))
            .args(["build", "-p", "mitibar", "--profile", "release-widget"])
            .current_dir(&root)
            .status()
            .context("run cargo build for the release-widget profile")?;
    if !status.success() {
        bail!("cargo build -p mitibar --profile release-widget exited with {status}");
    }
    let exe = target.join("release-widget").join("mitibar.exe");
    if !exe.is_file() {
        bail!("{} is missing after the build", exe.display());
    }

    let temp = std::env::temp_dir().join(format!("mitibar-idle-{}", std::process::id()));
    if temp.exists() {
        std::fs::remove_dir_all(&temp).with_context(|| format!("remove {}", temp.display()))?;
    }
    std::fs::create_dir_all(&temp).with_context(|| format!("create {}", temp.display()))?;
    let settings = temp.join("settings.toml");
    std::fs::write(
        &settings,
        "schema = 1\nlanguage = \"en\"\ntheme = \"system\"\nclock_24h = false\n\
         show_seconds = false\nextra_time_zone = \"Asia/Kathmandu\"\n\
         launch_at_login = false\ntaskbar_text = true\n",
    )
    .with_context(|| format!("write {}", settings.display()))?;

    let real_settings = real_settings_path();
    let real_before = real_settings
        .as_ref()
        .and_then(|path| std::fs::read(path).ok());

    let mut child = std::process::Command::new(&exe)
        .arg("--settings-file")
        .arg(&settings)
        .spawn()
        .with_context(|| format!("start {}", exe.display()))?;

    let result = measure(&mut child, minutes);
    let _ = child.kill();
    let _ = child.wait();
    let _ = std::fs::remove_dir_all(&temp);

    // The Run value must still be absent. A real value was refused above, so any value now is new.
    if run_value_exists()? {
        bail!("the app created a HKCU Run value `mitibar`; it must be removed by hand");
    }
    if let Some(path) = &real_settings {
        let after = std::fs::read(path).ok();
        if after != real_before {
            bail!("the real settings file {} changed", path.display());
        }
    }

    let measurement = result?;
    let date = local_date()?;
    let out_dir = root.join("docs/benchmarks/widget").join(&date);
    std::fs::create_dir_all(&out_dir).with_context(|| format!("create {}", out_dir.display()))?;
    let out = out_dir.join("idle.json");
    std::fs::write(&out, serde_json::to_string_pretty(&measurement.to_json())?)
        .with_context(|| format!("write {}", out.display()))?;

    print_lines(&measurement);
    println!("wrote {}", out.display());
    Ok(())
}

#[cfg(windows)]
fn measure(child: &mut std::process::Child, minutes: u32) -> Result<Measurement> {
    let pid = child.id();
    let process = Process::open(pid)?;
    let mut previous = process.snapshot(pid)?;
    let mut warmup = Vec::new();
    // Three 5 s samples: 15 s after start, as the plan says, so fonts and caches are warm.
    for _ in 0..3 {
        std::thread::sleep(SAMPLE);
        if let Some(status) = child.try_wait()? {
            bail!("the app exited during warmup with {status}");
        }
        let now = process.snapshot(pid)?;
        warmup.push(load_between(previous, now));
        previous = now;
    }

    let row_start = Instant::now();
    let start = process.snapshot(pid)?;
    let mut samples = Vec::new();
    let count = minutes * 60 / u32::try_from(SAMPLE.as_secs())?;
    for index in 0..count {
        std::thread::sleep(SAMPLE);
        if let Some(status) = child.try_wait()? {
            bail!("the app exited during the measured row with {status}");
        }
        let now = process.snapshot(pid)?;
        let load = load_between(previous, now);
        samples.push(Point {
            seconds: f64::from(index + 1) * SAMPLE.as_secs_f64(),
            counters: now,
            load_percent: load,
        });
        previous = now;
    }
    let Some(last) = samples.last().copied() else {
        bail!("no samples were taken");
    };
    let wall = row_start.elapsed().as_secs_f64();
    let cpu_seconds =
        last.counters.process_cpu.saturating_sub(start.process_cpu) as f64 / 10_000_000.0;
    let cpu_percent = if wall > 0.0 {
        cpu_seconds / wall * 100.0
    } else {
        0.0
    };
    let before_mean = mean(&warmup);
    let during: Vec<f64> = samples.iter().map(|point| point.load_percent).collect();
    let loaded = is_loaded(before_mean, &during);
    let measurement = Measurement {
        minutes,
        sample_seconds: SAMPLE.as_secs(),
        warmup_seconds: WARMUP.as_secs(),
        before_load: before_mean,
        during_load: mean(&during),
        min_load: during.iter().copied().fold(f64::INFINITY, f64::min),
        max_load: during.iter().copied().fold(f64::NEG_INFINITY, f64::max),
        private_bytes: last.counters.private,
        working_set_bytes: last.counters.working,
        wall_seconds: wall,
        process_cpu_seconds: cpu_seconds,
        cpu_percent,
        threads: last.counters.threads,
        loaded,
        samples,
    };
    Ok(measurement)
}

#[cfg(windows)]
fn print_lines(measurement: &Measurement) {
    let rss = rss_verdict(measurement.working_set_bytes, measurement.loaded);
    let cpu = cpu_verdict(measurement.cpu_percent, measurement.loaded);
    println!(
        "private bytes: {:.2} MB ({} bytes)",
        mb(measurement.private_bytes),
        measurement.private_bytes
    );
    println!(
        "working set (RSS): {:.2} MB ({} bytes), target <= 40 MB: {}",
        mb(measurement.working_set_bytes),
        measurement.working_set_bytes,
        rss.label()
    );
    println!(
        "idle CPU: {:.3} % of one core ({:.3} CPU s in {:.1} s), target about 0 %: {}",
        measurement.cpu_percent,
        measurement.process_cpu_seconds,
        measurement.wall_seconds,
        cpu.label()
    );
    println!("threads: {}", measurement.threads);
    println!(
        "all-CPU load: before {:.1} %, during mean {:.1} % (min {:.1}, max {:.1}); LOADED: {}",
        measurement.before_load,
        measurement.during_load,
        measurement.min_load,
        measurement.max_load,
        measurement.loaded
    );
}

#[cfg(windows)]
fn mb(bytes: u64) -> f64 {
    bytes as f64 / (1024.0 * 1024.0)
}

#[cfg(windows)]
fn mean(values: &[f64]) -> f64 {
    if values.is_empty() {
        return 0.0;
    }
    values.iter().sum::<f64>() / values.len() as f64
}

#[cfg(windows)]
#[derive(Debug, Clone, Copy)]
struct Counters {
    idle: u64,
    kernel: u64,
    user: u64,
    process_cpu: u64,
    private: u64,
    working: u64,
    threads: u32,
}

#[cfg(windows)]
#[derive(Debug, Clone, Copy)]
struct Point {
    seconds: f64,
    counters: Counters,
    load_percent: f64,
}

#[cfg(windows)]
struct Measurement {
    minutes: u32,
    sample_seconds: u64,
    warmup_seconds: u64,
    before_load: f64,
    during_load: f64,
    min_load: f64,
    max_load: f64,
    private_bytes: u64,
    working_set_bytes: u64,
    wall_seconds: f64,
    process_cpu_seconds: f64,
    cpu_percent: f64,
    threads: u32,
    loaded: bool,
    samples: Vec<Point>,
}

#[cfg(windows)]
impl Measurement {
    fn to_json(&self) -> serde_json::Value {
        let samples: Vec<serde_json::Value> = self
            .samples
            .iter()
            .map(|point| {
                serde_json::json!({
                    "seconds": point.seconds,
                    "private_bytes": point.counters.private,
                    "working_set_bytes": point.counters.working,
                    "process_cpu_seconds": point.counters.process_cpu as f64 / 10_000_000.0,
                    "threads": point.counters.threads,
                    "all_cpu_load_percent": point.load_percent,
                })
            })
            .collect();
        serde_json::json!({
            "date": local_date().unwrap_or_default(),
            "machine": std::env::var("COMPUTERNAME").unwrap_or_default(),
            "os": os_name(),
            "arch": std::env::consts::ARCH,
            "profile": "release-widget",
            "command": format!("cargo xtask widget-idle --minutes {}", self.minutes),
            "settings": {
                "language": "en",
                "theme": "system",
                "clock_24h": false,
                "show_seconds": false,
                "extra_time_zone": "Asia/Kathmandu",
                "launch_at_login": false,
                "taskbar_text": true,
            },
            "warmup_seconds": self.warmup_seconds,
            "minutes": self.minutes,
            "sample_seconds": self.sample_seconds,
            "private_bytes": self.private_bytes,
            "working_set_bytes": self.working_set_bytes,
            "process_cpu_seconds": self.process_cpu_seconds,
            "wall_seconds": self.wall_seconds,
            "cpu_percent_of_one_core": self.cpu_percent,
            "threads": self.threads,
            "all_cpu_load": {
                "before_mean_percent": self.before_load,
                "during_mean_percent": self.during_load,
                "min_percent": self.min_load,
                "max_percent": self.max_load,
            },
            "loaded": self.loaded,
            "verdicts": {
                "working_set": rss_verdict(self.working_set_bytes, self.loaded).label(),
                "cpu": cpu_verdict(self.cpu_percent, self.loaded).label(),
            },
            "samples": samples,
        })
    }
}

#[cfg(windows)]
fn load_between(previous: Counters, now: Counters) -> f64 {
    let total = now
        .kernel
        .saturating_add(now.user)
        .saturating_sub(previous.kernel.saturating_add(previous.user));
    let idle = now.idle.saturating_sub(previous.idle);
    let process = now.process_cpu.saturating_sub(previous.process_cpu);
    let busy = total.saturating_sub(idle).saturating_sub(process);
    if total == 0 {
        return 0.0;
    }
    busy as f64 / total as f64 * 100.0
}

#[cfg(windows)]
fn target_dir(root: &Path) -> PathBuf {
    std::env::var_os("CARGO_TARGET_DIR").map_or_else(
        || root.join("target"),
        |dir| {
            let dir = PathBuf::from(dir);
            if dir.is_absolute() {
                dir
            } else {
                root.join(dir)
            }
        },
    )
}

#[cfg(windows)]
fn real_settings_path() -> Option<PathBuf> {
    std::env::var_os("APPDATA").map(|dir| PathBuf::from(dir).join("mitibar").join("settings.toml"))
}

#[cfg(windows)]
fn running_mitibar() -> Result<bool> {
    let output = std::process::Command::new("tasklist")
        .args(["/FI", "IMAGENAME eq mitibar.exe", "/NH"])
        .output()
        .context("run tasklist")?;
    Ok(String::from_utf8_lossy(&output.stdout)
        .to_ascii_lowercase()
        .contains("mitibar.exe"))
}

#[cfg(windows)]
fn run_value_exists() -> Result<bool> {
    let output = std::process::Command::new("reg")
        .args([
            "query",
            r"HKCU\Software\Microsoft\Windows\CurrentVersion\Run",
            "/v",
            "mitibar",
        ])
        .output()
        .context("run reg query for the Run value")?;
    Ok(output.status.success())
}

#[cfg(windows)]
fn local_date() -> Result<String> {
    let output = std::process::Command::new("powershell")
        .args([
            "-NoProfile",
            "-NonInteractive",
            "-Command",
            "Get-Date -Format yyyy-MM-dd",
        ])
        .output()
        .context("run PowerShell for the date")?;
    if !output.status.success() {
        bail!("PowerShell could not read the date");
    }
    Ok(String::from_utf8_lossy(&output.stdout).trim().to_owned())
}

#[cfg(windows)]
fn os_name() -> String {
    let output = std::process::Command::new("powershell")
        .args([
            "-NoProfile",
            "-NonInteractive",
            "-Command",
            "$os = Get-CimInstance Win32_OperatingSystem; \"$($os.Caption) $($os.Version)\"",
        ])
        .output();
    match output {
        Ok(output) if output.status.success() => {
            String::from_utf8_lossy(&output.stdout).trim().to_owned()
        }
        _ => String::from("Windows"),
    }
}

#[cfg(windows)]
mod win {
    #![allow(unsafe_code)]

    use std::mem::size_of;

    use anyhow::{Result, bail};
    use windows::Win32::Foundation::{CloseHandle, FILETIME, HANDLE};
    use windows::Win32::System::Diagnostics::ToolHelp::{
        CreateToolhelp32Snapshot, TH32CS_SNAPTHREAD, THREADENTRY32, Thread32First, Thread32Next,
    };
    use windows::Win32::System::ProcessStatus::{GetProcessMemoryInfo, PROCESS_MEMORY_COUNTERS_EX};
    use windows::Win32::System::Threading::{
        GetProcessTimes, GetSystemTimes, OpenProcess, PROCESS_QUERY_INFORMATION, PROCESS_VM_READ,
    };

    use super::Counters;

    pub(super) struct Process(HANDLE);

    impl Process {
        pub(super) fn open(pid: u32) -> Result<Self> {
            // SAFETY: OpenProcess takes plain values and returns an owned handle; same-user
            // query rights are enough for memory and CPU times.
            let handle =
                unsafe { OpenProcess(PROCESS_QUERY_INFORMATION | PROCESS_VM_READ, false, pid) }?;
            Ok(Self(handle))
        }

        pub(super) fn snapshot(&self, pid: u32) -> Result<Counters> {
            let mut memory = PROCESS_MEMORY_COUNTERS_EX {
                cb: u32::try_from(size_of::<PROCESS_MEMORY_COUNTERS_EX>())?,
                ..PROCESS_MEMORY_COUNTERS_EX::default()
            };
            // SAFETY: `memory` is a live PROCESS_MEMORY_COUNTERS_EX with `cb` set, and the
            // function writes at most `cb` bytes into it.
            unsafe {
                GetProcessMemoryInfo(self.0, std::ptr::from_mut(&mut memory).cast(), memory.cb)?;
            }

            let mut creation = FILETIME::default();
            let mut exit = FILETIME::default();
            let mut kernel_time = FILETIME::default();
            let mut user_time = FILETIME::default();
            // SAFETY: four distinct FILETIME out parameters, all live for the call.
            unsafe {
                GetProcessTimes(
                    self.0,
                    &raw mut creation,
                    &raw mut exit,
                    &raw mut kernel_time,
                    &raw mut user_time,
                )?;
            }

            let mut idle = FILETIME::default();
            let mut kernel = FILETIME::default();
            let mut user = FILETIME::default();
            // SAFETY: three distinct FILETIME out parameters, all live for the call.
            unsafe {
                GetSystemTimes(
                    Some(&raw mut idle),
                    Some(&raw mut kernel),
                    Some(&raw mut user),
                )?;
            }

            Ok(Counters {
                idle: ticks(idle),
                kernel: ticks(kernel),
                user: ticks(user),
                process_cpu: ticks(kernel_time).saturating_add(ticks(user_time)),
                private: u64::try_from(memory.PrivateUsage)?,
                working: u64::try_from(memory.WorkingSetSize)?,
                threads: thread_count(pid)?,
            })
        }
    }

    impl Drop for Process {
        fn drop(&mut self) {
            // SAFETY: the handle is owned by this struct and closed once.
            unsafe {
                let _ = CloseHandle(self.0);
            }
        }
    }

    fn ticks(value: FILETIME) -> u64 {
        (u64::from(value.dwHighDateTime) << 32) | u64::from(value.dwLowDateTime)
    }

    fn thread_count(pid: u32) -> Result<u32> {
        // SAFETY: the snapshot handle is checked by the wrapper and closed below.
        let snapshot = unsafe { CreateToolhelp32Snapshot(TH32CS_SNAPTHREAD, 0) }?;
        let mut entry = THREADENTRY32 {
            dwSize: u32::try_from(size_of::<THREADENTRY32>())?,
            ..THREADENTRY32::default()
        };
        let mut count = 0u32;
        // SAFETY: `entry` has `dwSize` set and is valid for the thread enumeration.
        if unsafe { Thread32First(snapshot, &raw mut entry) }.is_ok() {
            loop {
                if entry.th32OwnerProcessID == pid {
                    count = count.saturating_add(1);
                }
                // SAFETY: same live THREADENTRY32; the loop ends at the last entry.
                if unsafe { Thread32Next(snapshot, &raw mut entry) }.is_err() {
                    break;
                }
            }
        }
        // SAFETY: the snapshot handle is owned here and closed exactly once.
        unsafe {
            let _ = CloseHandle(snapshot);
        }
        if count == 0 {
            bail!("no threads found for process {pid}");
        }
        Ok(count)
    }
}

#[cfg(windows)]
use win::Process;

#[cfg(test)]
mod tests {
    #![allow(clippy::expect_used)]

    use super::*;

    #[test]
    fn the_load_rule_uses_fifteen_percent() {
        assert!(!is_loaded(14.9, &[2.0, 3.0]));
        assert!(is_loaded(15.0, &[2.0, 3.0]));
        assert!(is_loaded(1.0, &[14.9, 15.0, 2.0]));
        assert!(!is_loaded(0.0, &[]));
    }

    #[test]
    fn the_verdict_table_uses_the_adr_targets() {
        assert_eq!(rss_verdict(RSS_TARGET_BYTES, false), Verdict::Pass);
        assert_eq!(rss_verdict(RSS_TARGET_BYTES + 1, false), Verdict::Miss);
        assert_eq!(rss_verdict(RSS_TARGET_BYTES + 1, true), Verdict::Loaded);

        assert_eq!(cpu_verdict(0.5, false), Verdict::Pass);
        assert_eq!(cpu_verdict(0.5001, false), Verdict::Miss);
        assert_eq!(cpu_verdict(9.0, true), Verdict::Loaded);
    }

    #[test]
    fn minutes_defaults_to_five_and_rejects_zero() {
        assert_eq!(parse_minutes(None).expect("default"), 5);
        assert_eq!(parse_minutes(Some("2")).expect("two"), 2);
        assert!(parse_minutes(Some("0")).is_err());
        assert!(parse_minutes(Some("five")).is_err());
    }
}
