//! Running cargo, git and other tools from the workspace root.

use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::Instant;

/// The workspace root: the parent of the xtask crate directory.
pub fn root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("xtask lives one level below the workspace root")
        .to_path_buf()
}

/// Runs one cargo command with inherited output and prints how long it took.
pub fn cargo(args: &[&str]) -> Result<(), String> {
    run(cargo_command().args(args))
}

/// A cargo command for the same toolchain that runs xtask.
pub fn cargo_command() -> Command {
    Command::new(std::env::var("CARGO").unwrap_or_else(|_| "cargo".to_string()))
}

/// Runs `command` from the workspace root with inherited output and prints how long it took.
pub fn run(command: &mut Command) -> Result<(), String> {
    let line = describe(command);
    let started = Instant::now();
    let status = command
        .current_dir(root())
        .status()
        .map_err(|error| format!("cannot start {line}: {error}"))?;
    println!("  {line} took {:.1}s", started.elapsed().as_secs_f64());
    if status.success() {
        Ok(())
    } else {
        Err(format!("{line} failed"))
    }
}

/// Program name and arguments, without directories, for progress and error lines.
fn describe(command: &Command) -> String {
    let program = Path::new(command.get_program())
        .file_stem()
        .map(|stem| stem.to_string_lossy().into_owned())
        .unwrap_or_default();
    let args: Vec<String> = command
        .get_args()
        .map(|arg| arg.to_string_lossy().into_owned())
        .collect();
    format!("{program} {}", args.join(" "))
}

/// Every file git tracks, as workspace-relative paths with forward slashes.
pub fn tracked_files() -> Result<Vec<String>, String> {
    let output = Command::new("git")
        .args(["ls-files", "-z"])
        .current_dir(root())
        .output()
        .map_err(|error| format!("cannot run git ls-files: {error}"))?;
    if !output.status.success() {
        return Err("git ls-files failed; repo-check needs a git repository".to_string());
    }
    let listing = String::from_utf8_lossy(&output.stdout);
    Ok(listing
        .split('\0')
        .filter(|path| !path.is_empty())
        .map(|path| path.replace('\\', "/"))
        .collect())
}
