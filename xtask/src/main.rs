//! Repository tasks: `cargo xtask ci` runs every quality gate, `cargo xtask codegen` writes the
//! generated tables (ADR-0006). Arguments are read from `std::env::args()` to keep this tiny.

mod codegen;

use std::process::{Command, ExitCode};

use anyhow::{Result, bail};

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let args: Vec<&str> = args.iter().map(String::as_str).collect();
    let result = match args.as_slice() {
        ["ci"] => ci(),
        ["codegen"] => codegen::run(false),
        ["codegen", "--check"] => codegen::run(true),
        _ => {
            eprintln!("usage: cargo xtask <ci | codegen [--check]>");
            return ExitCode::from(2);
        }
    };
    match result {
        Ok(()) => ExitCode::SUCCESS,
        Err(err) => {
            eprintln!("{err:#}");
            ExitCode::FAILURE
        }
    }
}

/// One CI step: a program, its arguments and environment variables to set.
struct Step {
    program: &'static str,
    args: &'static [&'static str],
    env: &'static [(&'static str, &'static str)],
    /// Skip with a warning when the program isn't installed (only for optional tools).
    optional: bool,
}

const fn step(program: &'static str, args: &'static [&'static str]) -> Step {
    Step {
        program,
        args,
        env: &[],
        optional: false,
    }
}

const STEPS: &[Step] = &[
    step("cargo", &["fmt", "--all", "--check"]),
    step(
        "cargo",
        &[
            "clippy",
            "--workspace",
            "--all-targets",
            "--all-features",
            "--",
            "-D",
            "warnings",
        ],
    ),
    // `no_alloc` has no test harness, so nextest can't list it; it runs as its own step below.
    step(
        "cargo",
        &[
            "nextest",
            "run",
            "--workspace",
            "--all-features",
            "--no-tests=pass",
            "-E",
            "not binary_id(nepdate::no_alloc)",
        ],
    ),
    step("cargo", &["test", "-p", "nepdate", "--test", "no_alloc"]),
    step("cargo", &["test", "--doc", "--workspace", "--all-features"]),
    Step {
        program: "cargo",
        args: &["doc", "--workspace", "--no-deps", "--all-features"],
        env: &[("RUSTDOCFLAGS", "-D warnings")],
        optional: false,
    },
    step(
        "cargo",
        &["build", "-p", "nepdate", "--no-default-features"],
    ),
    step(
        "cargo",
        &[
            "build",
            "-p",
            "nepdate",
            "--no-default-features",
            "--target",
            "wasm32-unknown-unknown",
        ],
    ),
    step("cargo", &["xtask", "codegen", "--check"]),
    Step {
        program: "cargo",
        args: &["deny", "check"],
        env: &[],
        optional: true,
    },
];

fn ci() -> Result<()> {
    for (index, step) in STEPS.iter().enumerate() {
        let number = index + 1;
        let shown = format!("{} {}", step.program, step.args.join(" "));
        if step.optional && !subcommand_installed(step.args) {
            println!("xtask ci: step {number} skipped, `{shown}` is not installed");
            continue;
        }
        if let Err(err) = run(step) {
            println!("xtask ci: FAILED at step {number} ({shown})");
            return Err(err);
        }
    }
    println!("xtask ci: all {} steps passed", STEPS.len());
    Ok(())
}

/// Whether `cargo <first arg>` exists, by asking it for its version.
fn subcommand_installed(args: &[&str]) -> bool {
    let Some(sub) = args.first() else {
        return false;
    };
    Command::new("cargo")
        .args([sub, "--version"])
        .output()
        .is_ok_and(|out| out.status.success())
}

fn run(step: &Step) -> Result<()> {
    println!("$ {} {}", step.program, step.args.join(" "));
    let mut command = Command::new(step.program);
    command.args(step.args);
    for (key, value) in step.env {
        command.env(key, value);
    }
    let status = command.status()?;
    if !status.success() {
        bail!(
            "`{} {}` exited with {status}",
            step.program,
            step.args.join(" ")
        );
    }
    Ok(())
}
