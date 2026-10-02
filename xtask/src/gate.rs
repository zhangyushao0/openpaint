//! `cargo gate`: the one command that must pass before a commit. The exit code is the verdict.

use std::time::Instant;

use crate::check;
use crate::process::cargo;

type Step = fn() -> Result<(), String>;

const STEPS: &[(&str, Step)] = &[
    ("fmt", fmt),
    ("clippy", clippy),
    ("clippy-android", clippy_android),
    ("test", test),
    ("repo-check", check::run),
];

fn fmt() -> Result<(), String> {
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
        "aarch64-linux-android",
        "--all-targets",
        "--",
        "-D",
        "warnings",
    ])
}

fn test() -> Result<(), String> {
    cargo(&["test", "--workspace"])
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
