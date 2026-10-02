//! Running cargo and git from the workspace root.

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
    let cargo = std::env::var("CARGO").unwrap_or_else(|_| "cargo".to_string());
    let started = Instant::now();
    let status = Command::new(cargo)
        .args(args)
        .current_dir(root())
        .status()
        .map_err(|error| format!("cannot start cargo {}: {error}", args.join(" ")))?;
    println!(
        "  cargo {} took {:.1}s",
        args.join(" "),
        started.elapsed().as_secs_f64()
    );
    if status.success() {
        Ok(())
    } else {
        Err(format!("cargo {} failed", args.join(" ")))
    }
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
