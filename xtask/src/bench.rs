//! `cargo xtask bench`: runs the `alloc` and `agree` binaries and the criterion bench,
//! one at a time, each pinned to logical CPU 2 (affinity mask `0x4`, BENCHMARK §1).

use std::process::Command;

use anyhow::{Result, bail};

/// The three commands, in order. `cargo bench` runs criterion with the bench crate's release
/// profile (`lto = "fat"`, `codegen-units = 1`, what ADR-0004 measured with).
const COMMANDS: [&[&str]; 3] = [
    &["run", "-p", "nepdate-bench", "--release", "--bin", "alloc"],
    &["run", "-p", "nepdate-bench", "--release", "--bin", "agree"],
    &["bench", "-p", "nepdate-bench"],
];

pub(crate) fn run() -> Result<()> {
    for args in COMMANDS {
        run_pinned("cargo", args)?;
    }
    Ok(())
}

/// Runs `program args...` pinned to logical CPU 2 and waits for it, printing the exact command.
///
/// On Windows this is `cmd /c start /b /affinity 4 /wait <command>`; `/b` keeps the output in
/// this console (without it `start` opens a new window), `/affinity 4` is mask `0x4` = CPU 2,
/// and `/wait` blocks until it finishes. On Linux it is `taskset -c 2 <command>`.
fn run_pinned(program: &str, args: &[&str]) -> Result<()> {
    let joined = args.join(" ");
    let status = if cfg!(windows) {
        let full = format!("start /b /affinity 4 /wait {program} {joined}");
        println!("$ {full}");
        Command::new("cmd").args(["/c", &full]).status()?
    } else {
        let full = format!("taskset -c 2 {program} {joined}");
        println!("$ {full}");
        Command::new("taskset")
            .arg("-c")
            .arg("2")
            .arg(program)
            .args(args)
            .status()?
    };
    if !status.success() {
        bail!("`{program} {joined}` (pinned to CPU 2) exited with {status}");
    }
    Ok(())
}
