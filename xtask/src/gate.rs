//! `cargo gate`: the one command that must pass before a commit. The exit code is the verdict.

use std::time::Instant;

use crate::process::cargo;
use crate::{check, test_run};

type Step = fn() -> Result<(), String>;

const STEPS: &[(&str, Step)] = &[
    ("fmt", fmt),
    ("clippy", clippy),
    ("clippy-android", clippy_android),
    ("test", test_run::run),
    ("scenario smoke", scenario_smoke),
    ("repo-check", check::run),
    ("prune", test_run::prune::run),
];

/// The tablet's target, which the gate checks with clippy.
pub const ANDROID_TRIPLE: &str = "aarch64-linux-android";

/// Locally the gate formats the workspace first, so a formatting slip never fails it; under CI
/// (the `CI` variable every CI service sets) it only checks, so unformatted code fails there.
fn fmt() -> Result<(), String> {
    if std::env::var_os("CI").is_none() {
        cargo(&["fmt", "--all"])?;
    }
    cargo(&["fmt", "--all", "--check"])
}

fn clippy() -> Result<(), String> {
    cargo(&[
        "clippy",
        "--workspace",
        "--all-targets",
        "--",
        "-D",
        "warnings",
    ])
}

/// The JNI host compiles only for Android. Checking it needs the rustup target, not the NDK.
fn clippy_android() -> Result<(), String> {
    cargo(&[
        "clippy",
        "--package",
        "openpaint-android",
        "--target",
        ANDROID_TRIPLE,
        "--all-targets",
        "--",
        "-D",
        "warnings",
    ])
}

/// The short harness scenario: the app opens, takes a pen stroke and passes a canvas checkpoint
/// (`tests/scenarios/smoke.json`). Debug build, sharing the test step's compiled crates. Left out
/// on a software GPU (`test_run::software`): its checkpoint is a hash the dev machine's GPU drew.
fn scenario_smoke() -> Result<(), String> {
    if test_run::software::on() {
        println!("  left out on a software GPU: its canvas checkpoint is a hardware GPU's hash");
        return Ok(());
    }
    cargo(&[
        "run",
        "--quiet",
        "--package",
        "openpaint-harness",
        "--",
        "scenario",
        "smoke",
    ])
}

pub fn run() -> Result<(), String> {
    let started = Instant::now();
    for (name, step) in STEPS {
        println!("== gate: {name}");
        if let Err(message) = step() {
            println!(
                "GATE FAILED at {name} after {:.1}s",
                started.elapsed().as_secs_f64()
            );
            return Err(message);
        }
    }
    println!("GATE PASSED in {:.1}s", started.elapsed().as_secs_f64());
    Ok(())
}
