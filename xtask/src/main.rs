//! Repository tasks: `cargo xtask ci` runs every quality gate, `cargo xtask coverage` the
//! coverage gate (L6-03), `cargo xtask codegen` writes the generated tables (ADR-0006), `cargo xtask catalog-check <phase>` checks the C# test catalogue
//! (L2-08), `cargo xtask data-status` reports how far the patro data reaches (L5-01b) and
//! `cargo xtask check-core-has-no-patro` keeps patro data out of the core (L5-06),
//! `cargo xtask release-check` and `cargo xtask changelog` prepare a release (L6-04). Arguments
//! are read from `std::env::args()` to keep this tiny.

mod bench;
mod catalog;
mod codegen;
mod core_check;
mod data_status;
mod icons;
mod patro;
mod release;

use std::process::{Command, ExitCode};

use anyhow::{Result, bail};

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let args: Vec<&str> = args.iter().map(String::as_str).collect();
    let result = match args.as_slice() {
        ["ci"] => ci(),
        ["coverage"] => run(&COVERAGE),
        ["bench"] => bench::run(),
        ["codegen"] => codegen::run(false),
        ["codegen", "--check"] => codegen::run(true),
        ["catalog-check", phase] => catalog::run(phase, false),
        ["catalog-check", phase, "--list"] => catalog::run(phase, true),
        ["data-status"] => data_status::run(None),
        ["data-status", "--today", today] => data_status::run(Some(today)),
        ["check-core-has-no-patro"] => core_check::run(),
        ["icons"] => icons::icons(false),
        ["icons", "--check"] => icons::icons(true),
        ["release-check", package, version] => release::run_check(package, version),
        ["changelog", package, version] => release::run_changelog(package, version),
        _ => {
            eprintln!(
                "usage: cargo xtask <ci | coverage | bench | codegen [--check] | catalog-check <phase> [--list] | \
                 data-status [--today YYYY-MM-DD] | check-core-has-no-patro | icons [--check] | \
                 release-check <package> <version> | changelog <package> <version>>"
            );
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
            "not binary_id(nepdate::no_alloc) and not binary_id(nepdate-patro::no_alloc)",
        ],
    ),
    step("cargo", &["test", "-p", "nepdate", "--test", "no_alloc"]),
    step(
        "cargo",
        &["test", "-p", "nepdate-patro", "--test", "no_alloc"],
    ),
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
    step("cargo", &["xtask", "icons", "--check"]),
    Step {
        program: "cargo",
        args: &["deny", "check"],
        env: &[],
        optional: true,
    },
    step("cargo", &["xtask", "catalog-check", "1"]),
    step("cargo", &["xtask", "catalog-check", "2"]),
    step("cargo", &["xtask", "catalog-check", "3"]),
    step("cargo", &["xtask", "catalog-check", "4"]),
    // L5-06: each feature combination on its own, since `cargo hack` isn't in the budget.
    step(
        "cargo",
        &[
            "build",
            "-p",
            "nepdate",
            "--no-default-features",
            "--features",
            "serde",
        ],
    ),
    step(
        "cargo",
        &["test", "-p", "nepdate", "--features", "jiff,chrono,serde"],
    ),
    step("cargo", &["test", "-p", "nepdate"]),
    step("cargo", &["test", "-p", "nepdate-patro"]),
    step(
        "cargo",
        &[
            "build",
            "-p",
            "nepdate-patro",
            "--target",
            "wasm32-unknown-unknown",
        ],
    ),
    step("cargo", &["xtask", "check-core-has-no-patro"]),
];

/// L6-03: line coverage of both libraries, at least 90 % (ADR-0014). Not a `ci` step, because
/// it needs `cargo-llvm-cov` and the `llvm-tools-preview` component; CI runs it as its own job.
const COVERAGE: Step = step(
    "cargo",
    &[
        "llvm-cov",
        "-p",
        "nepdate",
        "-p",
        "nepdate-patro",
        "--all-features",
        "--fail-under-lines",
        "90",
    ],
);

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
