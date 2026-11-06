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

/// A cargo command for the same toolchain that runs xtask, in the shell's environment: without
/// the variables cargo set for xtask's own run (its package's name, version and manifest), which
/// a build script (ring's) depends on. With them every gate built ring again and relinked the
/// harness, which a `cargo scenarios` running beside it holds open (S106).
pub fn cargo_command() -> Command {
    let mut command = Command::new(std::env::var("CARGO").unwrap_or_else(|_| "cargo".to_string()));
    for (key, _) in std::env::vars_os() {
        if key.to_str().is_some_and(set_by_cargo_run) {
            command.env_remove(key);
        }
    }
    command
}

/// The variables `cargo run` sets for the program it runs.
fn set_by_cargo_run(key: &str) -> bool {
    key.starts_with("CARGO_PKG_")
        || matches!(
            key,
            "CARGO_MANIFEST_DIR"
                | "CARGO_MANIFEST_PATH"
                | "CARGO_CRATE_NAME"
                | "CARGO_BIN_NAME"
                | "CARGO_PRIMARY_PACKAGE"
        )
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

/// Every file git tracks and every new file it would track (untracked and not ignored), as
/// workspace-relative paths with forward slashes, each once. A new file counts before it is
/// staged, so a gate run before `git add` sees it.
pub fn tracked_files() -> Result<Vec<String>, String> {
    let output = Command::new("git")
        .args([
            "ls-files",
            "-z",
            "--cached",
            "--others",
            "--exclude-standard",
        ])
        .current_dir(root())
        .output()
        .map_err(|error| format!("cannot run git ls-files: {error}"))?;
    if !output.status.success() {
        return Err("git ls-files failed; repo-check needs a git repository".to_string());
    }
    let listing = String::from_utf8_lossy(&output.stdout);
    let mut files: Vec<String> = listing
        .split('\0')
        .filter(|path| !path.is_empty())
        .map(|path| path.replace('\\', "/"))
        .collect();
    files.sort_unstable();
    files.dedup();
    Ok(files)
}
